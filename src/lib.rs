use nom::{
    bytes::complete::{tag, take},
    combinator::{map, map_res},
    error,
    multi::{length_data, many0},
    number::complete::{le_u16, le_u32},
    sequence::tuple,
};

/// The supported .wpilog version (major, minor) for this parser.
pub(crate) const SUPPORTED_VERSION: u16 = 0x0100;
/// The magic number for the .wpilog file format.
pub(crate) const WPILOG_MAGIC: &[u8] = b"WPILOG";

// Types

type I<'a> = &'a [u8];
type IResult<'a, O> = nom::IResult<I<'a>, O, ParsingError<'a>>;

// Errors

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ParsingError<'a> {
    #[error("Not a DataLog file!")]
    InvalidMagic,
    #[error("Invalid control record type!")]
    InvalidControlRecordType,
    #[error("Unknown type: \"{0}\"")]
    UnknownType(&'a str),
    #[error("Unsupported DataLog version: 0x{0:x}")]
    UnsupportedVersion(u16),
    #[error("\"{caller}\" requested {expected} bytes of data but was given only {given} ({:?}) bytes.", given.to_le_bytes())]
    DataTooShortForRequestedLength {
        caller: &'a str,
        expected: usize,
        given: usize,
    },
    #[error("Error while parsing utf8 string: {0}")]
    Utf8Error(std::str::Utf8Error),
    #[error("Nom Error: {0:?}")]
    Nom(error::ErrorKind),
    #[error("(ctx: {0}, err: {1:?})")]
    Context(&'static str, Box<Self>),
    #[error("This is not a valid error.")]
    Placeholder,
}

impl<'a> error::ParseError<I<'a>> for ParsingError<'a> {
    fn from_error_kind(_: I<'a>, kind: error::ErrorKind) -> Self {
        Self::Nom(kind)
    }

    fn append(_: I<'a>, _: error::ErrorKind, other: Self) -> Self {
        other
    }
}

impl<'a> error::ContextError<I<'a>> for ParsingError<'a> {
    fn add_context(_: I<'a>, ctx: &'static str, other: Self) -> Self {
        Self::Context(ctx, Box::new(other))
    }
}

impl<'a> error::FromExternalError<I<'a>, std::str::Utf8Error> for ParsingError<'a> {
    fn from_external_error(_: I<'a>, _: error::ErrorKind, e: std::str::Utf8Error) -> Self {
        Self::Utf8Error(e)
    }
}

impl<'a> error::FromExternalError<I<'a>, ParsingError<'a>> for ParsingError<'a> {
    fn from_external_error(_: I<'a>, _: error::ErrorKind, e: ParsingError<'a>) -> Self {
        e
    }
}

impl<'a> nom::ErrorConvert<ParsingError<'a>> for error::Error<(I<'a>, usize)> {
    fn convert(self) -> ParsingError<'a> {
        ParsingError::Placeholder
    }
}

// Structs

#[derive(Debug, Clone, PartialEq)]
pub struct DataLog<'a> {
    pub metadata: &'a str,
    pub records: Vec<Record<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record<'a> {
    entry_id: u32,
    timestamp: u64,
    payload: RecordPayload<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecordPayload<'a> {
    Start {
        target_entry_id: u32,
        name: &'a str,
        ty: EntryType<'a>,
        metadata: &'a str,
    },
    Finish {
        target_entry_id: u32,
    },
    Value(&'a [u8]),
    Metadata {
        target_entry_id: u32,
        metadata: &'a str,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryType<'a> {
    Raw,
    Boolean,
    Int64,
    Float,
    Double,
    String,
    Array(Box<EntryType<'a>>),
    Unknown(&'a str),
}

impl<'a> From<&'a str> for EntryType<'a> {
    fn from(value: &'a str) -> Self {
        if value.is_empty() {
            return EntryType::Unknown(value);
        }

        if value.ends_with("[]") {
            return Self::Array(Box::new(value[..value.len() - 2].into()));
        }

        match value {
            "raw" => Self::Raw,
            "boolean" => Self::Boolean,
            "int64" => Self::Int64,
            "float" => Self::Float,
            "double" => Self::Double,
            "string" => Self::String,
            _ => return EntryType::Unknown(value),
        }
    }
}

impl<'a> DataLog<'a> {
    pub fn parse_from_bytes(data: I<'a>) -> Result<Self, nom::Err<ParsingError<'a>>> {
        map(
            tuple((
                error::context("header", parse_header),
                error::context("records", many0(error::context("record", parse_record))),
            )),
            |(metadata, records)| Self { metadata, records },
        )(data)
        .map(|(_, log)| log)
    }
}

// Parsers

fn ensure<'a, O>(
    mut f: impl FnMut(I<'a>) -> IResult<'a, O>,
    condition: impl Fn(&O) -> Result<(), ParsingError<'a>>,
) -> impl FnMut(I<'a>) -> IResult<'a, O> {
    move |data: I<'a>| {
        f(data).and_then(|(i, o)| {
            condition(&o)
                .and_then(|_| Ok((i, o)))
                .map_err(|e| nom::Err::Error(e))
        })
    }
}

fn ensure_success<'a, O>(
    mut f: impl FnMut(I<'a>) -> IResult<'a, O>,
    error: ParsingError<'a>,
) -> impl FnMut(I<'a>) -> IResult<'a, O> {
    move |data: I<'a>| f(data).map_err(|_| nom::Err::Error(error.clone()))
}

fn length_prefixed_string<'a>(data: I<'a>) -> IResult<'a, &'a str> {
    map_res(
        length_data(ensure(le_u32, move |&n| {
            let len = data.len();

            if n as usize > len {
                Err(ParsingError::DataTooShortForRequestedLength {
                    caller: "length-prefixed string",
                    expected: n as usize,
                    given: len,
                })
            } else {
                Ok(())
            }
        })),
        |s: &[u8]| std::str::from_utf8(s),
    )(data)
}

fn parse_header<'a>(data: I<'a>) -> IResult<'a, &'a str> {
    let (data, _) = ensure_success(tag(WPILOG_MAGIC), ParsingError::InvalidMagic)(data)?;
    let (data, _) = ensure(le_u16, |&v| {
        (v == SUPPORTED_VERSION)
            .then(|| ())
            .ok_or(ParsingError::UnsupportedVersion(v))
    })(data)?;

    length_prefixed_string(data)
}

pub fn parse_record<'a>(data: I<'a>) -> IResult<'a, Record<'a>> {
    fn u32(data: &[u8], n: u8) -> (I<'_>, u32) {
        let (data, bytes) = read_bytes::<4>(data, n as usize + 1);
        (data, u32::from_le_bytes(bytes))
    }

    fn u64(data: &[u8], n: u8) -> (I<'_>, u64) {
        let (data, bytes) = read_bytes::<8>(data, n as usize + 1);
        (data, u64::from_le_bytes(bytes))
    }

    let (data, lengths) = take(1usize)(data)?;
    let lengths = lengths[0];

    let (data, entry_id) = u32(data, lengths & 0b11);
    let (data, payload_size) = u32(data, (lengths & 0b11 << 2) >> 2);
    let (data, timestamp) = u64(data, (lengths & 0b111 << 4) >> 4);

    let (data, payload) = parse_record_payload(data, entry_id, payload_size as usize)?;

    Ok((
        data,
        Record {
            entry_id,
            timestamp,
            payload,
        },
    ))
}

fn parse_record_payload<'a>(
    data: I<'a>,
    entry_id: u32,
    payload_size: usize,
) -> IResult<'a, RecordPayload<'a>> {
    if entry_id != 0 {
        let (data, payload) = take(payload_size)(data)?;
        return Ok((data, RecordPayload::Value(payload)));
    }

    let (data, ty) = take(1usize)(data)?;

    let (data, target_entry_id) = le_u32(data)?;

    match ty[0] {
        0 => {
            // Start
            map_res(
                tuple((
                    length_prefixed_string,
                    length_prefixed_string,
                    length_prefixed_string,
                )),
                |(name, ty, metadata)| {
                    Ok::<_, ParsingError<'a>>(RecordPayload::Start {
                        target_entry_id,
                        name,
                        ty: EntryType::from(ty),
                        metadata,
                    })
                },
            )(data)
        }
        1 => {
            // Finish
            Ok((data, RecordPayload::Finish { target_entry_id }))
        }
        2 => {
            // Metadata
            map(length_prefixed_string, |metadata| RecordPayload::Metadata {
                target_entry_id,
                metadata,
            })(data)
        }
        _ => Err(nom::Err::Error(ParsingError::InvalidControlRecordType)),
    }
}

// Util

fn read_bytes<'a, const N: usize>(data: I<'a>, n: usize) -> (I<'a>, [u8; N]) {
    assert!(n <= N);
    assert!(data.len() >= n);

    let mut buf = [0u8; N];
    for i in 0..n {
        buf[i] = data[i];
    }

    (&data[n..], buf)
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: [u8; 0x10] = [
        0x57, 0x50, 0x49, 0x4C, 0x4F, 0x47, // Magic = "WPILOG"
        0x00, 0x01, // Version = 0x0100
        0x04, 0x00, 0x00, 0x00, // Metadata Length = 4
        0x74, 0x65, 0x73, 0x74, // Metadata = "test"
    ];

    const INT_RECORD: [u8; 0x0E] = [
        0x20, // Timestamp Length = 3, Payload Size Length = 1, Entry Id Length = 1
        0x01, // Entry Id = 1
        0x08, // Payload Size = 8
        0x40, 0x42, 0x0F, // Timestamp = 1_000_000
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Payload = [u8; 8]
    ];

    const START_RECORD: [u8; 0x20] = [
        0x20, // Timestamp Length = 3, Payload Size Length = 1, Entry Id Length = 1
        0x00, // Entry Id = 0
        0x1A, // Payload Size = 26
        0x40, 0x42, 0x0F, // Timestamp = 1_000_000
        0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, // Payload = [u8; 26]
        0x00, 0x00, 0x74, 0x65, 0x73, 0x74, 0x05, //
        0x00, 0x00, 0x00, 0x69, 0x6E, 0x74, 0x36, //
        0x34, 0x00, 0x00, 0x00, 0x00,
    ];

    const FINISH_RECORD: [u8; 0x0B] = [
        0x20, // Timestamp Length = 3, Payload Size Length = 1, Entry Id Length = 1
        0x00, // Entry Id = 0
        0x05, // Payload Size = 5
        0x40, 0x42, 0x0F, // Timestamp = 1_000_000
        0x01, 0x01, 0x00, 0x00, 0x00, // Payload = [u8; 5]
    ];

    const METADATA_RECORD: [u8; 0x1E] = [
        0x20, // Timestamp Length = 3, Payload Size Length = 1, Entry Id Length = 1
        0x00, // Entry Id = 0
        0x18, // Payload Size = 24
        0x40, 0x42, 0x0F, // Timestamp = 1_000_000
        0x02, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, // Payload = [u8; 24]
        0x00, 0x7B, 0x22, 0x73, 0x6F, 0x75, 0x72, 0x63, //
        0x65, 0x22, 0x3A, 0x22, 0x4E, 0x54, 0x22, 0x7D, //
    ];

    #[test]
    fn test_parse_header() {
        assert_eq!(parse_header(&HEADER), Ok((&[] as &[u8], "test")));
    }

    #[test]
    fn test_parse_payload_value() {
        assert_eq!(
            parse_record_payload(&INT_RECORD[6..], 0x01, 0x08),
            Ok((&[] as &[u8], RecordPayload::Value(&INT_RECORD[6..])))
        )
    }

    #[test]
    fn test_parse_payload_start() {
        assert_eq!(
            parse_record_payload(&START_RECORD[6..], 0x00, 0x1A),
            Ok((
                &[] as &[u8],
                RecordPayload::Start {
                    target_entry_id: 1,
                    name: "test",
                    ty: EntryType::Int64,
                    metadata: ""
                }
            ))
        )
    }

    #[test]
    fn test_parse_payload_finish() {
        assert_eq!(
            parse_record_payload(&FINISH_RECORD[6..], 0x00, 0x05),
            Ok((&[] as &[u8], RecordPayload::Finish { target_entry_id: 1 }))
        )
    }

    #[test]
    fn test_parse_payload_metadata() {
        assert_eq!(
            parse_record_payload(&METADATA_RECORD[6..], 0x00, 0x18),
            Ok((
                &[] as &[u8],
                RecordPayload::Metadata {
                    target_entry_id: 1,
                    metadata: "{\"source\":\"NT\"}"
                }
            ))
        )
    }

    #[test]
    fn test_parse_entry_type() {
        for (input, expected) in [
            ("", EntryType::Unknown("")),
            ("raw", EntryType::Raw),
            ("raw[]", EntryType::Array(Box::new(EntryType::Raw))),
            (
                "unknown[][]",
                EntryType::Array(Box::new(EntryType::Array(Box::new(EntryType::Unknown(
                    "unknown",
                ))))),
            ),
            ("[]", EntryType::Array(Box::new(EntryType::Unknown("")))),
        ] {
            assert_eq!(EntryType::from(input), expected);
        }
    }

    #[test]
    fn test_parse_record() {
        assert_eq!(
            parse_record(&START_RECORD),
            Ok((
                &[] as &[u8],
                Record {
                    entry_id: 0,
                    timestamp: 1_000_000,
                    payload: RecordPayload::Start {
                        target_entry_id: 1,
                        name: "test",
                        ty: EntryType::Int64,
                        metadata: ""
                    }
                }
            ))
        )
    }

    #[test]
    fn test_parse_log() {
        let mut data: Vec<u8> = vec![];

        for (i, seg) in ([
            &HEADER as &[u8],
            &START_RECORD,
            &INT_RECORD,
            &METADATA_RECORD,
            &FINISH_RECORD,
        ])
        .into_iter()
        .enumerate()
        {
            data.extend_from_slice(seg);

            assert_eq!(
                DataLog::parse_from_bytes(&data).map(|log| log.records.len()),
                Ok(i)
            );
        }
    }
}

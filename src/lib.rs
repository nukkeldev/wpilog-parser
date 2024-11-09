// Entries in the log are started using a Start control record

// A Finish control record may be used to indicate no further
// records with that entry ID will follow.

// Following a Finish control record, an entry ID may be reused by
// another Start control record

// Multiple Start control records for a single entry ID without an
// intervening Finish control record have unspecified behavior.

// All values are stored in little endian order.

pub const MAGIC: &[u8] = b"WPILOG";
pub const SUPPORTED_VERSION: (u8, u8) = (1, 0);

pub mod read_only {
    use core::str;
    use log::debug;
    use std::fmt::Debug;

    use thiserror::Error;

    use crate::{MAGIC, SUPPORTED_VERSION};

    // Parse

    #[derive(Error, Debug, Clone)]
    pub enum ParseError {
        #[error("Expected file to start with b\"WPILOG\"!")]
        InvalidMagic,
        #[error("File is empty.")]
        EmptyFile,
        #[error("Expected version {}.{}, but got version {}.{}", crate::SUPPORTED_VERSION.0, crate::SUPPORTED_VERSION.1, .0, .1)]
        UnsupportedVersion(u8, u8),
        #[error("Variable sized type's data did not match it's expected length.")]
        VariableSizedTypeTooShort,
        #[error("Failed to parse utf8.")]
        InvalidUtf8(#[from] std::str::Utf8Error),
    }

    pub trait Parse<'a>: Sized {
        fn parse(data: &'a [u8]) -> Result<(usize, Self), ParseError>;
    }

    // Zero-Copy

    #[derive(Clone, PartialEq)]
    pub struct ZCVU32LE<'a>([&'a u8; 4]);

    impl<'a> ZCVU32LE<'a> {
        fn parse(data: &'a [u8]) -> Result<(usize, Self), ParseError> {
            Ok((
                data.len().min(4),
                Self(
                    [
                        data.get(0).unwrap_or(&0),
                        data.get(1).unwrap_or(&0),
                        data.get(2).unwrap_or(&0),
                        data.get(3).unwrap_or(&0),
                    ]
                    .try_into()
                    .expect("Invalid length; This should be unreachable."),
                ),
            ))
        }

        fn get(&self) -> u32 {
            u32::from_le_bytes([*self.0[0], *self.0[1], *self.0[2], *self.0[3]])
        }
    }

    impl<'a> Debug for ZCVU32LE<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.get())
        }
    }

    #[derive(Clone, PartialEq)]
    pub struct ZCVU64LE<'a>([&'a u8; 8]);

    impl<'a> ZCVU64LE<'a> {
        fn parse(data: &'a [u8]) -> Result<(usize, Self), ParseError> {
            Ok((
                data.len().min(8),
                Self(
                    [
                        data.get(0).unwrap_or(&0),
                        data.get(1).unwrap_or(&0),
                        data.get(2).unwrap_or(&0),
                        data.get(3).unwrap_or(&0),
                        data.get(4).unwrap_or(&0),
                        data.get(5).unwrap_or(&0),
                        data.get(6).unwrap_or(&0),
                        data.get(7).unwrap_or(&0),
                    ]
                    .try_into()
                    .expect("Invalid length; This should be unreachable."),
                ),
            ))
        }

        fn get(&self) -> u64 {
            u64::from_le_bytes([
                *self.0[0], *self.0[1], *self.0[2], *self.0[3], *self.0[4], *self.0[5], *self.0[6],
                *self.0[7],
            ])
        }
    }

    impl<'a> Debug for ZCVU64LE<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.get())
        }
    }

    #[derive(Clone, PartialEq)]
    pub struct ZeroCopyString<'a>(&'a [u8]);

    impl<'a> Debug for ZeroCopyString<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", &self.get().unwrap())
        }
    }

    impl<'a> Parse<'a> for ZeroCopyString<'a> {
        fn parse(data: &'a [u8]) -> Result<(usize, Self), ParseError> {
            let (bytes_read, length) = ZCVU32LE::parse(data)?;
            let len = length.get() as usize;

            if data.len() - bytes_read < len {
                return Err(ParseError::VariableSizedTypeTooShort);
            }

            let str = &data[bytes_read..bytes_read + len];

            Ok((bytes_read + len, Self(str)))
        }
    }

    impl<'a> ZeroCopyString<'a> {
        fn get(&self) -> Result<&'a str, ParseError> {
            str::from_utf8(self.0).map_err(|e| e.into())
        }
    }

    // Header

    #[derive(Debug, Clone, PartialEq)]
    pub struct Header<'a> {
        major_version: &'a u8,
        minor_version: &'a u8,
        extra_string: ZeroCopyString<'a>,
    }

    impl<'a> Header<'a> {
        fn parse(data: &'a [u8]) -> Result<(usize, Self), ParseError> {
            if !data.starts_with(MAGIC) {
                return Err(ParseError::InvalidMagic);
            }

            let mut bytes_read = MAGIC.len();

            let minor_version = &data[bytes_read];
            bytes_read += 1;
            let major_version = &data[bytes_read];
            bytes_read += 1;

            debug!("WPILog Version {major_version}.{minor_version}");

            if (*major_version, *minor_version) != SUPPORTED_VERSION {
                return Err(ParseError::UnsupportedVersion(
                    *major_version,
                    *minor_version,
                ));
            }

            let (br, extra_string) = ZeroCopyString::parse(&data[bytes_read..])?;
            bytes_read += br;

            Ok((
                bytes_read,
                Self {
                    major_version,
                    minor_version,
                    extra_string,
                },
            ))
        }
    }

    // Record

    #[derive(Clone, PartialEq)]
    pub struct RecordHeaderLengths<'a>(&'a u8);

    impl<'a> RecordHeaderLengths<'a> {
        fn entry_id_length(&self) -> usize {
            ((self.0 & 0b11) + 1) as usize
        }

        fn payload_size_length(&self) -> usize {
            (((self.0 & (0b11 << 2)) >> 2) + 1) as usize
        }

        fn timestamp_length(&self) -> usize {
            (((self.0 & (0b111 << 4)) >> 4) + 1) as usize
        }
    }

    impl<'a> Debug for RecordHeaderLengths<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("RecordHeaderLengths")
                .field("entry_id_length", &self.entry_id_length())
                .field("payload_size_length", &self.payload_size_length())
                .field("timestamp_length", &self.timestamp_length())
                .finish()
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Record<'a> {
        pub lengths: RecordHeaderLengths<'a>,
        pub entry_id: ZCVU32LE<'a>,
        pub payload_size: ZCVU32LE<'a>,
        pub timestamp: ZCVU64LE<'a>,
        pub payload: &'a [u8],
    }

    impl<'a> Record<'a> {
        fn parse(data: &'a [u8]) -> Result<(usize, Self), ParseError> {
            let lengths = RecordHeaderLengths(&data[0]);
            let mut bytes_read = 1;

            let (br, entry_id) =
                ZCVU32LE::parse(&data[bytes_read..bytes_read + lengths.entry_id_length()])?;
            bytes_read += br;
            let (br, payload_size) =
                ZCVU32LE::parse(&data[bytes_read..bytes_read + lengths.payload_size_length()])?;
            bytes_read += br;
            let (br, timestamp) =
                ZCVU64LE::parse(&data[bytes_read..bytes_read + lengths.timestamp_length()])?;
            bytes_read += br;

            let payload = &data[bytes_read..bytes_read + payload_size.get() as usize];
            bytes_read += payload_size.get() as usize;

            Ok((
                bytes_read,
                Self {
                    lengths,
                    entry_id,
                    payload_size,
                    timestamp,
                    payload,
                },
            ))
        }
    }

    // WPILog

    #[derive(Debug, Clone, PartialEq)]
    pub struct WPILog<'a> {
        pub header: Header<'a>,
        pub records: Vec<Record<'a>>,
    }

    impl<'a> WPILog<'a> {
        fn parse(data: &'a [u8]) -> Result<Self, ParseError> {
            if data.is_empty() {
                return Err(ParseError::EmptyFile);
            }

            let (mut bytes_read, header) = Header::parse(data)?;

            let mut records = vec![];
            while bytes_read < data.len() {
                let (br, record) = Record::parse(&data[bytes_read..])?;
                bytes_read += br;

                records.push(record);
            }

            Ok(Self { header, records })
        }
    }

    #[cfg(test)]
    mod tests {
        use std::{fs::File, time::Instant};

        use memmap2::Mmap;

        use super::*;

        const SPEC_EXAMPLE_HEADER: &[u8] = &[
            0x57, 0x50, 0x49, 0x4c, 0x4f, 0x47, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];
        const SPEC_EXAMPLE_HEADER_PARSED: Header = Header {
            major_version: &1,
            minor_version: &0,
            extra_string: ZeroCopyString(b""),
        };

        const CORRECT_HEADER: &[u8] =
            const_str::concat_bytes!(b"WPILOG", [0, 1], [13, 0, 0, 0], b"Hello, World!");
        const CORRECT_HEADER_PARSED: Header = Header {
            major_version: &1,
            minor_version: &0,
            extra_string: ZeroCopyString(b"Hello, World!"),
        };

        const NO_MAGIC_HEADER: &[u8] =
            const_str::concat_bytes!([0, 1], [13, 0, 0, 0], b"Hello, World!");
        const WRONG_VERSION_HEADER: &[u8] =
            const_str::concat_bytes!(b"WPILOG", [1, 1], [13, 0, 0, 0], b"Hello, World!");
        const INVALID_STRING_HEADER: &[u8] =
            const_str::concat_bytes!(b"WPILOG", [0, 1], [13, 0, 0, 0], b"Hello!");

        #[test]
        fn parse_header() {
            let _ = pretty_env_logger::try_init_timed();

            assert!(Header::parse(SPEC_EXAMPLE_HEADER).unwrap().1 == SPEC_EXAMPLE_HEADER_PARSED);
            assert!(Header::parse(CORRECT_HEADER).unwrap().1 == CORRECT_HEADER_PARSED);
            assert!(Header::parse(NO_MAGIC_HEADER).is_err());
            assert!(Header::parse(WRONG_VERSION_HEADER).is_err());
            assert!(Header::parse(INVALID_STRING_HEADER).is_err());
        }

        const SPEC_EXAMPLE_RECORD: &[u8] = &[
            0x20, 0x01, 0x08, 0x40, 0x42, 0x0f, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        const SPEC_EXAMPLE_LOG: &[u8] =
            const_str::concat_bytes!(SPEC_EXAMPLE_HEADER, SPEC_EXAMPLE_RECORD,);
        const CORRECT_LOG: &[u8] = const_str::concat_bytes!(CORRECT_HEADER, 0b0_000_00_00);

        #[test]
        fn parse_log() {
            let _ = pretty_env_logger::try_init_timed();

            assert!(
                dbg!(WPILog::parse(SPEC_EXAMPLE_LOG).unwrap())
                    == WPILog {
                        header: SPEC_EXAMPLE_HEADER_PARSED,
                        records: vec![Record {
                            lengths: RecordHeaderLengths(&0b00100000),
                            entry_id: ZCVU32LE([&1, &0, &0, &0]),
                            payload_size: ZCVU32LE([&8, &0, &0, &0]),
                            timestamp: ZCVU64LE([&0x40, &0x42, &0x0f, &0, &0, &0, &0, &0]),
                            payload: &[3, 0, 0, 0, 0, 0, 0, 0]
                        }],
                    }
            );
        }

        #[test]
        fn real_log() {
            let _ = pretty_env_logger::try_init_timed();

            let mut start = Instant::now();
            let file = File::open("Log_24-04-06_13-28-45_e5.wpilog").unwrap();
            let mmap = unsafe { Mmap::map(&file).unwrap() };
            let mut end = start.elapsed();
            debug!("Loaded file in {end:?}");

            start = Instant::now();
            let log = WPILog::parse(&mmap[..]).unwrap();
            end = start.elapsed();
            let rate = end / log.records.len() as u32;
            debug!(
                "Parsed {} records in {end:?} @ ~{rate:?} per record / ~{:.0} records per second",
                log.records.len(),
                (log.records.len() as f32 / rate.as_secs_f32())
            );
            // debug!("First 100 records: {:#?}", &log.records[..100])
        }
    }
}

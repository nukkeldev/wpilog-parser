use std::collections::HashMap;

use crate::{
    errors::WPILogParseError,
    parsing::{
        u32, u32_len_prefix_utf8_string_unchecked, variable_length_u32, variable_length_u64,
    },
    MINIMUM_WPILOG_SIZE, SUPPORTED_VERSION, WPILOG_MAGIC,
};

#[derive(Debug, Clone)]
pub struct WPILog<'a> {
    metadata: &'a str, // Arbitrary metadata
                       // entries: HashMap<&'a str, Entry<'a>>, // Name correlated entries
}

impl<'a> WPILog<'a> {
    /// Verifies the header of the .wpilog, checking the magic and version.
    /// Returns the unread data and the "extra header string" if verified.
    fn verify_header(data: &'a [u8]) -> Result<(&'a [u8], &'a str), WPILogParseError> {
        debug_assert!(
            data.len() >= MINIMUM_WPILOG_SIZE,
            "{}",
            WPILogParseError::TooShort
        );

        {
            if &data[0..6] != WPILOG_MAGIC {
                return Err(WPILogParseError::InvalidMagic);
            }

            let version = [data[7], data[6]]; // LE swap
            if version != SUPPORTED_VERSION {
                return Err(WPILogParseError::UnsupportedVersion(version));
            }
        }

        let (data, metadata) = u32_len_prefix_utf8_string_unchecked(&data[8..]);

        Ok((data, metadata))
    }

    pub fn parse(data: &'a [u8]) -> Result<Self, WPILogParseError> {
        let (mut data, metadata) = Self::verify_header(data)?;

        let mut entries: HashMap<&'a str, Vec<&[u8]>> = HashMap::new();
        let mut correlation_table: HashMap<u32, HashMap<u64, &'a str>> = HashMap::new();

        // First-Pass
        while !data.is_empty() {
            let entry_id;
            let payload_size;
            let timestamp;

            let entry_id_length = data[0] & 0b11;
            let payload_size_length = (data[0] & 0b11 << 2) >> 2;
            let timestamp_length = (data[0] & 0b111 << 4) >> 4;
            data = &data[1..];

            (data, entry_id) = variable_length_u32(data, entry_id_length as usize + 1);
            (data, payload_size) = variable_length_u32(data, payload_size_length as usize + 1);
            (data, timestamp) = variable_length_u64(data, timestamp_length as usize + 1);

            debug_assert!(payload_size != 0);

            let payload = &data[..payload_size as usize];
            data = &data[payload_size as usize..];

            if entry_id != 0 {
                continue;
            }

            debug_assert!(payload_size >= 5);

            let control_record_type = payload[0];
            let (_, target_entry_id) = u32(&payload[1..5]);

            match control_record_type {
                0 => {
                    let (_, entry_name) = u32_len_prefix_utf8_string_unchecked(&payload[5..]);
                    correlation_table
                        .entry(target_entry_id)
                        .and_modify(|c| {
                            c.insert(timestamp, entry_name);
                        })
                        .or_insert(HashMap::new());
                }
                _ => {}
            }
        }

        println!("Correlation Table: {correlation_table:#?}");

        Ok(Self { metadata })
    }
}

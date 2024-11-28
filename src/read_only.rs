use std::{
    collections::{hash_map::Keys, HashMap},
    ops::Index,
};

use crate::{
    debug, error,
    errors::WPILogParseError,
    parsing::{
        u32, u32_len_prefix_utf8_string_unchecked, variable_length_u32, variable_length_u64,
    },
    trace, tracing, warn, MINIMUM_WPILOG_SIZE, SUPPORTED_VERSION, WPILOG_MAGIC,
};

// TYPES

pub type Timestamp = u64;

// WPILOG

#[derive(Clone)]
pub struct WPILog<'a> {
    /// File header metadata
    pub(crate) metadata: &'a str,
    /// Name correlated entries
    pub(crate) entries: HashMap<&'a str, Entry>,
}

impl<'a> WPILog<'a> {
    // Parsing

    /// Verifies the header of the .wpilog, checking the magic and version.
    /// Increments the `bytes_read` by the parsed bytes.
    /// Returns the "extra header string" if verified.
    fn verify_header(data: &'a [u8], bytes_read: &mut usize) -> Result<&'a str, WPILogParseError> {
        debug_assert!(
            data.len() >= MINIMUM_WPILOG_SIZE,
            "{}",
            WPILogParseError::TooShort
        );

        {
            if &data[0..6] != WPILOG_MAGIC {
                return Err(WPILogParseError::InvalidMagic);
            }
            *bytes_read += 6;

            let version = [data[7], data[6]]; // LE swap
            if version != SUPPORTED_VERSION {
                return Err(WPILogParseError::UnsupportedVersion(version));
            }
            *bytes_read += 2;
        }

        let metadata = u32_len_prefix_utf8_string_unchecked(&data[*bytes_read..], bytes_read);

        Ok(metadata)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(data_len = data.len())))]
    pub fn parse(data: &'a [u8]) -> Result<Self, WPILogParseError> {
        let len = data.len();
        let mut bytes_read = 0;

        let metadata = Self::verify_header(data, &mut bytes_read)?;

        debug!("Verified header: {metadata:?}");

        let mut id_name: HashMap<u32, &str> = HashMap::new();
        let mut entries: HashMap<&str, Entry> = HashMap::new();

        tracing! {
            let mut now = std::time::Instant::now();
            let mut record_count = 0;
            let mut entry_count = 0;
        }

        while bytes_read < len {
            tracing! {
                let span = tracing::span!(
                    tracing::Level::INFO,
                    "record",
                    idx = record_count,
                    offset = bytes_read
                );
                let _guard = span.enter();
            }

            let entry_id_length = (data[bytes_read] & 0b11) as usize + 1;
            let payload_size_length = ((data[bytes_read] & 0b11 << 2) >> 2) as usize + 1;
            let timestamp_length = ((data[bytes_read] & 0b111 << 4) >> 4) as usize + 1;
            bytes_read += 1;

            trace!(entry_id_length, payload_size_length, timestamp_length);

            let entry_id = variable_length_u32(&data[bytes_read..], entry_id_length);
            bytes_read += entry_id_length;
            let payload_size =
                variable_length_u32(&data[bytes_read..], payload_size_length) as usize;
            bytes_read += payload_size_length;
            let timestamp = variable_length_u64(&data[bytes_read..], timestamp_length);
            bytes_read += timestamp_length;

            trace!(entry_id, payload_size, timestamp);

            tracing! {
                record_count += 1;
            }

            if entry_id != 0 {
                entries
                    .get_mut(&id_name[&entry_id])
                    .unwrap()
                    .add_value(timestamp, bytes_read);
                bytes_read += payload_size;

                continue;
            }

            tracing! {
                entry_count += 1;
            }

            let control_record_type = data[bytes_read];
            bytes_read += 1;

            let target_entry_id = u32(&data[bytes_read..bytes_read + 4]);
            bytes_read += 4;

            if control_record_type == 0 {
                if id_name.contains_key(&target_entry_id) {
                    unimplemented!("This parser does not support entry_id rebindings.");
                }

                let name = u32_len_prefix_utf8_string_unchecked(&data[bytes_read..], &mut 0);
                id_name.insert(target_entry_id, name);
                entries.insert(name, Entry::new(bytes_read));
            }

            bytes_read += payload_size - 5;
        }

        tracing! {
            let elapsed = now.elapsed();

            debug!(
                ?elapsed,
                entry_count,
                record_count,
                time_per_record = ?elapsed.div_f32(record_count as f32),
                throughput = (record_count as f32) / elapsed.as_secs_f32()
            );

            now = std::time::Instant::now();
        }

        // Rarely will records be dirastically out of order, so sorting should be fairly cheap.
        entries.values_mut().for_each(|v| v.sort_by_timestamp());

        debug!(second_pass = ?now.elapsed());

        let log = Self { metadata, entries };

        trace!(entry_names = ?log.get_entry_names());

        Ok(log)
    }

    // Getters

    pub fn get_entry_names(&self) -> Keys<'_, &str, Entry> {
        self.entries.keys()
    }
}

impl<'a> Index<&str> for WPILog<'a> {
    type Output = Entry;

    fn index(&self, index: &str) -> &Self::Output {
        &self.entries[index]
    }
}

// ENTRY

#[derive(Debug, Clone)]
pub struct Entry {
    decl_offset: usize,
    values: Vec<(Timestamp, usize)>, // TODO: All data types should be parseable without the payload_size iirc
}

impl Entry {
    fn new(decl_offset: usize) -> Self {
        Self {
            decl_offset,
            values: vec![],
        }
    }

    fn add_value(&mut self, timestamp: Timestamp, value_offset: usize) {
        self.values.push((timestamp, value_offset));
    }

    fn sort_by_timestamp(&mut self) {
        self.values.sort_by_key(|(t, _)| *t);
    }
}

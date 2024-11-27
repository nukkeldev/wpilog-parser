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
        let len = data.len();

        let (mut data, metadata) = Self::verify_header(data)?;

        let mut entries: HashMap<&'a str, Vec<(u64, &'a [u8])>> = HashMap::new();
        let mut id_entries: HashMap<u32, Vec<(u64, &'a [u8])>> = HashMap::new();
        let mut correlation_table: HashMap<u32, EntryIdNameBinding<'a>> = HashMap::new();

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

            let payload = &data[..payload_size as usize];
            data = &data[payload_size as usize..];

            if entry_id != 0 {
                id_entries
                    .entry(entry_id)
                    .or_default()
                    .push((timestamp, payload));

                continue;
            }

            let control_record_type = payload[0];
            let (_, target_entry_id) = u32(&payload[1..5]);

            if control_record_type == 0 {
                let (_, entry_name) = u32_len_prefix_utf8_string_unchecked(&payload[5..]);
                correlation_table
                    .entry(target_entry_id)
                    .and_modify(|b| b.add_binding(timestamp, entry_name))
                    .or_insert(EntryIdNameBinding::new(timestamp, entry_name));
            }
        }

        for (id, e) in id_entries {
            for entry in e {
                let entry_name = correlation_table.get_mut(&id).unwrap().get_binding(entry.0);
                if let Some(name) = entry_name {
                    entries.entry(name).or_default().push(entry);
                } else {
                    println!("{id} {:x}", len - data.len());
                }
            }
        }

        Ok(Self { metadata })
    }
}

#[derive(Debug, Clone)]
struct EntryIdNameBinding<'a> {
    last_idx: usize,
    bindings: Vec<(u64, &'a str)>,
}

impl<'a> EntryIdNameBinding<'a> {
    pub fn new(timestamp: u64, name: &'a str) -> Self {
        Self {
            last_idx: 0,
            bindings: vec![(timestamp, name)],
        }
    }

    pub fn add_binding(&mut self, timestamp: u64, name: &'a str) {
        if self.bindings[0].0 > timestamp {
            self.bindings.insert(0, (timestamp, name));
        }

        for i in 1..self.bindings.len() - 1 {
            if timestamp > self.bindings[i].0 {
                self.bindings.insert(i + 1, (timestamp, name));
                return;
            }
        }
        self.bindings.push((timestamp, name));
    }

    pub fn get_binding(&mut self, timestamp: u64) -> Option<&'a str> {
        if timestamp < self.bindings[0].0 {
            println!("{self:?} {timestamp}");
            return None;
        }

        for i in 0..self.bindings.len() {
            if timestamp >= self.bindings[(i + self.last_idx) % self.bindings.len()].0 {
                self.last_idx = (i + self.last_idx) % self.bindings.len();
                return Some(self.bindings[0].1);
            }
        }

        unreachable!("wut")
    }
}

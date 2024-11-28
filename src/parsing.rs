use std::mem::size_of;

use crate::{error, extract_from_slice, trace};

pub(crate) fn from_utf8_release_unchecked<'a>(data: &'a [u8]) -> &'a str {
    #[cfg(debug_assertions)]
    {
        std::str::from_utf8(data).expect("Invalid UTF8 String")
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        std::str::from_utf8_unchecked(data)
    }
}

/// Parses a &str from little-endian `u32` length-prefixed utf8 string.
/// - Bounds checking and UTF8 validation only in debug mode.
///
/// Returns unread bytes and the &str.
#[cfg_attr(feature = "tracing", tracing::instrument(skip(data), fields(data_len = data.len())))]
pub(crate) fn u32_len_prefix_utf8_string_unchecked<'a>(
    data: &'a [u8],
    bytes_read: &mut usize,
) -> &'a str {
    const U32_SIZE: usize = size_of::<u32>();

    #[cfg(debug_assertions)]
    {
        if data.len() < U32_SIZE {
            error!("data too short to parse a u32 length-prefixed string");
            // Already at the end of the data regardless, parsing farther will lead to corrupt results (non-recoverable)
            panic!("invalid data");
        }
    }
    let len = u32::from_le_bytes(extract_from_slice!(data, 0, 1, 2, 3)) as usize;

    trace!(
        string_len = len,
        raw_length_bytes = ?extract_from_slice!(data, 0, 1, 2, 3)
    );

    #[cfg(debug_assertions)]
    {
        if data.len() < U32_SIZE + len {
            error!("data too short to parse the length of the string");
            panic!("invalid data");
        }
    }
    let str = from_utf8_release_unchecked(&data[U32_SIZE..U32_SIZE + len]);

    trace!(str);

    *bytes_read += U32_SIZE + len;

    str
}

/// Parses a little-endian u32.
/// - `data` must be at least 4 bytes.
///
/// Returns unread bytes and the u32.
pub(crate) fn u32<'a>(data: &'a [u8]) -> u32 {
    // The lack of bounds checking requires the data to be at least 4 bytes long.
    debug_assert!(data.len() >= 4);

    u32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

/// Parses a little-endian u32 with a given length in bytes.
/// - `data` must be at least 4 bytes (see impl).
///
/// Returns unread bytes and the u32.
pub(crate) fn variable_length_u32<'a>(data: &'a [u8], len: usize) -> u32 {
    // The lack of bounds checking requires the data to be at least 4 bytes long.
    debug_assert!(data.len() >= size_of::<u32>());

    u32::from_le_bytes([
        // For each possible byte of the u32, we check whether it's index is in bounds for
        // the given length. If it is not, we zero it out. The wrapping_sub is to do an
        // underflow that will get us either 0 or 255. For that reason, the condition is
        // negated as well (!(# < len) => (# >= len)).
        data[0] & ((0 >= len) as u8).wrapping_sub(1),
        data[1] & ((1 >= len) as u8).wrapping_sub(1),
        data[2] & ((2 >= len) as u8).wrapping_sub(1),
        data[3] & ((3 >= len) as u8).wrapping_sub(1),
    ])
}

/// Parses a little-endian u64 with a given length in bytes.
/// - `data` must be at least 8 bytes (see impl).
///
/// Returns unread bytes and the u64.
pub(crate) fn variable_length_u64<'a>(data: &'a [u8], len: usize) -> u64 {
    // The lack of bounds checking requires the data to be at least 8 bytes long.
    debug_assert!(data.len() >= size_of::<u64>());

    u64::from_le_bytes([
        // See comment in `variable_length_u32` for explanation.
        data[0] & ((0 >= len) as u8).wrapping_sub(1),
        data[1] & ((1 >= len) as u8).wrapping_sub(1),
        data[2] & ((2 >= len) as u8).wrapping_sub(1),
        data[3] & ((3 >= len) as u8).wrapping_sub(1),
        data[4] & ((4 >= len) as u8).wrapping_sub(1),
        data[5] & ((5 >= len) as u8).wrapping_sub(1),
        data[6] & ((6 >= len) as u8).wrapping_sub(1),
        data[7] & ((7 >= len) as u8).wrapping_sub(1),
    ])
}

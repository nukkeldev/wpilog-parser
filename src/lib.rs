pub mod errors;
pub mod read_only;

mod macros;
mod parsing;

/// The supported .wpilog version (major, minor) for this parser.
pub(crate) const SUPPORTED_VERSION: [u8; 2] = [1, 0];
/// The magic number for the .wpilog file format.
pub(crate) const WPILOG_MAGIC: &[u8] = b"WPILOG";
/// The minimum byte count for a empty (Header + no metadata) .wpilog file.
pub(crate) const MINIMUM_WPILOG_SIZE: usize = 6 + 2 + 4;

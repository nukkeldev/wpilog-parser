use thiserror::Error;

use crate::SUPPORTED_VERSION;

#[derive(Debug, Clone, Error)]
pub enum WPILogParseError {
    #[error("Data does not start with the proper magic")]
    InvalidMagic,
    #[error("Data is too short to be a .wpilog file")]
    TooShort,
    #[error("Unsupported version '{0:?}', expected '{SUPPORTED_VERSION:?}'")]
    UnsupportedVersion([u8; 2]),
}

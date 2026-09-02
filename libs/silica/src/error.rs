use thiserror::Error;

#[derive(Error, Debug)]
pub enum SilicaError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plist error: {0}")]
    PlistError(#[from] plist::Error),
    #[error("Zip error: {0}")]
    ZipError(#[from] zip::result::ZipError),
    #[error("Ns archive error: {0}")]
    NsArchiveError(#[from] crate::ns_archive::error::NsArchiveError),
    #[error("Invalid values in file")]
    InvalidValue,
    #[error("{resource} exceeds the {limit}-byte limit (actual: {actual} bytes)")]
    ResourceLimitExceeded {
        resource: &'static str,
        limit: u64,
        actual: u64,
    },
    #[error("Unknown decoding error")]
    #[allow(dead_code)]
    Unknown,
}

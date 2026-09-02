use crate::error::SilicaError;
use std::io::Read;

pub const MAX_PROCREATE_ARCHIVE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const MAX_DOCUMENT_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_QUICKLOOK_PNG_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) fn read_bounded(
    reader: impl Read,
    declared_size: u64,
    resource: &'static str,
    limit: u64,
) -> Result<Vec<u8>, SilicaError> {
    if declared_size > limit {
        return Err(SilicaError::ResourceLimitExceeded {
            resource,
            limit,
            actual: declared_size,
        });
    }

    let capacity =
        usize::try_from(declared_size).map_err(|_| SilicaError::ResourceLimitExceeded {
            resource,
            limit: usize::MAX as u64,
            actual: declared_size,
        })?;
    let mut bytes = Vec::with_capacity(capacity);
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(SilicaError::ResourceLimitExceeded {
            resource,
            limit,
            actual,
        });
    }
    Ok(bytes)
}

use crate::error::SilicaError;
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

pub const MAX_PROCREATE_ARCHIVE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const MAX_DOCUMENT_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_QUICKLOOK_PNG_BYTES: u64 = 64 * 1024 * 1024;
const MAX_INITIAL_READ_CAPACITY: u64 = 16 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub fn read_procreate_archive(path: impl AsRef<Path>) -> Result<Vec<u8>, SilicaError> {
    let file = std::fs::File::open(path)?;
    let declared_size = file.metadata()?.len();
    read_bounded(
        file,
        declared_size,
        "Procreate archive",
        MAX_PROCREATE_ARCHIVE_BYTES,
    )
}

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

    // A valid multi-gigabyte document should grow incrementally instead of reserving its full
    // declared size before the OS has demonstrated that the process can read it.
    let initial_capacity = declared_size.min(MAX_INITIAL_READ_CAPACITY);
    let capacity =
        usize::try_from(initial_capacity).map_err(|_| SilicaError::ResourceLimitExceeded {
            resource,
            limit: usize::MAX as u64,
            actual: initial_capacity,
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

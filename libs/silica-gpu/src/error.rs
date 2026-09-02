use thiserror::Error;

#[derive(Error, Debug)]
pub enum SilicaError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("silica error: {0}")]
    Silica(#[from] silica::error::SilicaError),
    #[error("Ns archive error: {0}")]
    NsArchiveError(#[from] silica::ns_archive::error::NsArchiveError),
    #[error("Zip error: {0}")]
    ZipError(#[from] zip::result::ZipError),
    #[error("LZO error: {0}")]
    LzoError(#[from] lzokay::Error),
    #[error("LZ4 error: {0}")]
    Lz4Error(#[from] lz4_flex::block::DecompressError),
    #[error("Corrupted format")]
    CorruptedFormat,
    #[error("invalid canvas size {width}x{height}; device limit is {max_dimension}")]
    InvalidCanvasSize {
        width: u32,
        height: u32,
        max_dimension: u32,
    },
    #[error("invalid tile size {tile_size}; device limit is {max_dimension}")]
    InvalidTileSize { tile_size: u32, max_dimension: u32 },
    #[error("atlas requires {required_layers} layers; device limit is {max_layers}")]
    AtlasCapacityExceeded {
        required_layers: u32,
        max_layers: u32,
    },
    #[error("encoded tile chunk exceeds the {limit}-byte limit (actual: {actual} bytes)")]
    ChunkTooLarge { limit: u64, actual: u64 },
    #[error("tile coordinate ({col}, {row}) is outside the {cols}x{rows} canvas tiling")]
    ChunkCoordinateOutOfBounds {
        col: u32,
        row: u32,
        cols: u32,
        rows: u32,
    },
    #[error("decoded tile length mismatch: expected {expected} bytes, got {actual}")]
    DecodedChunkLengthMismatch { expected: usize, actual: usize },
    #[error("tile upload exceeds the allocated atlas texture")]
    ChunkUploadOutOfBounds,
    #[error("hierarchy identity {0:?} is not present in the GPU document")]
    HierarchyNotFound(silica::HierarchyId),
    #[error("hierarchy identity {0:?} does not support clipping")]
    HierarchyDoesNotSupportClipping(silica::HierarchyId),
    #[error("hierarchy identity {0:?} does not support blend modes")]
    HierarchyDoesNotSupportBlendMode(silica::HierarchyId),
    #[error("hierarchy identity {0:?} does not support opacity")]
    HierarchyDoesNotSupportOpacity(silica::HierarchyId),
}

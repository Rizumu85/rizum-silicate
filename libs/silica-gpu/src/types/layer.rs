use crate::{error::SilicaError, params::LoadParams};
#[cfg(not(target_arch = "wasm32"))]
use rayon::{iter::IntoParallelRefIterator, prelude::ParallelIterator};
use std::{io::Read, num::NonZeroU32};

#[derive(Debug, Clone, PartialEq)]
pub struct SilicaChunk {
    pub col: u32,
    pub row: u32,
    pub atlas_index: NonZeroU32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SilicaImageData {
    pub chunks: Vec<SilicaChunk>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SilicaLayer {
    info: silica::SilicaLayer,
    pub image: SilicaImageData,
    pub mask: Option<Box<SilicaLayer>>,
    pub id: u32,
}

impl std::ops::Deref for SilicaLayer {
    type Target = silica::SilicaLayer;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl std::ops::DerefMut for SilicaLayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.info
    }
}

impl SilicaLayer {
    const RGBA_CHANNEL_COUNT: usize = 4;
    const MAX_ENCODED_CHUNK_BYTES: u64 = 64 * 1024 * 1024;

    fn parse_chunk_str(chunk_str: &str) -> Result<(u32, u32), SilicaError> {
        let tilde_index = chunk_str
            .find('~')
            .ok_or_else(|| SilicaError::CorruptedFormat)?;
        let col = chunk_str[..tilde_index]
            .parse::<u32>()
            .map_err(|_| SilicaError::CorruptedFormat)?;
        let row = chunk_str[tilde_index + 1..]
            .parse::<u32>()
            .map_err(|_| SilicaError::CorruptedFormat)?;

        Ok((col, row))
    }

    pub(crate) fn load(
        mut info: silica::SilicaLayer,
        params: &LoadParams<'_>,
        is_mask: bool,
    ) -> Result<SilicaLayer, SilicaError> {
        let chunks = {
            #[cfg(not(target_arch = "wasm32"))]
            let iter = params.file_names.par_iter();
            #[cfg(target_arch = "wasm32")]
            let iter = params.file_names.iter();
            iter
        }
        .filter_map(|path| {
            path.strip_prefix(info.uuid.as_str())
                .and_then(|relative| relative.strip_prefix('/'))
                .map(|relative| (*path, relative))
        })
        .map(|(path, relative)| -> Result<SilicaChunk, SilicaError> {
            let mut archive = params.archive.clone();

            let (chunk_str, is_lz4) = if let Some(stem) = relative.strip_suffix(".lz4") {
                (stem, true)
            } else if let Some(stem) = relative.strip_suffix(".chunk") {
                (stem, false)
            } else {
                return Err(SilicaError::CorruptedFormat);
            };
            if chunk_str.contains('/') {
                return Err(SilicaError::CorruptedFormat);
            }
            let (col, row) = Self::parse_chunk_str(chunk_str)?;
            if col >= params.tiling.cols || row >= params.tiling.rows {
                return Err(SilicaError::ChunkCoordinateOutOfBounds {
                    col,
                    row,
                    cols: params.tiling.cols,
                    rows: params.tiling.rows,
                });
            }

            let tile_extent = params.tiling.tile_extent(col, row);

            let mut chunk = archive.by_name(path)?;
            if chunk.size() > Self::MAX_ENCODED_CHUNK_BYTES {
                return Err(SilicaError::ChunkTooLarge {
                    limit: Self::MAX_ENCODED_CHUNK_BYTES,
                    actual: chunk.size(),
                });
            }

            let mut buf = Vec::with_capacity(chunk.size() as usize);
            chunk
                .by_ref()
                .take(Self::MAX_ENCODED_CHUNK_BYTES + 1)
                .read_to_end(&mut buf)?;
            if buf.len() as u64 > Self::MAX_ENCODED_CHUNK_BYTES {
                return Err(SilicaError::ChunkTooLarge {
                    limit: Self::MAX_ENCODED_CHUNK_BYTES,
                    actual: buf.len() as u64,
                });
            }

            let pixel_count = (tile_extent.width as usize)
                .checked_mul(tile_extent.height as usize)
                .ok_or(SilicaError::CorruptedFormat)?;
            let data_len = pixel_count
                .checked_mul(Self::RGBA_CHANNEL_COUNT)
                .ok_or(SilicaError::CorruptedFormat)?;

            // Try RGBA first (4 channels), but fall back to grayscale (1 channel) for masks
            let decompress_len = if is_mask { pixel_count } else { data_len };

            let data = if is_lz4 {
                let mut data = Vec::with_capacity(decompress_len);
                lz4::decompress(buf.as_slice(), &mut data, decompress_len)?;
                data
            } else {
                let mut data = vec![0; decompress_len];
                let actual = lzokay::decompress::decompress(buf.as_slice(), &mut data)?;
                if actual != decompress_len {
                    return Err(SilicaError::DecodedChunkLengthMismatch {
                        expected: decompress_len,
                        actual,
                    });
                }
                data
            };

            let data = if is_mask {
                // Expand grayscale mask to RGBA by replicating the single channel into R, G, B and setting A to the same value
                let mut rgba = Vec::with_capacity(data_len);
                rgba.extend(
                    data.into_iter()
                        .flat_map(|value| [value; Self::RGBA_CHANNEL_COUNT]),
                );
                rgba
            } else {
                data
            };

            if data.len() != data_len {
                return Err(SilicaError::DecodedChunkLengthMismatch {
                    expected: data_len,
                    actual: data.len(),
                });
            }

            let atlas_index =
                NonZeroU32::new(params.allocate_chunk_id()).ok_or(SilicaError::CorruptedFormat)?;

            let origin = params.tiling.atlas_origin(atlas_index.get());

            Self::replace_from_bytes(
                params.queue,
                params.atlas_texture,
                &data,
                origin,
                tile_extent,
            )?;
            Ok(SilicaChunk {
                col,
                row,
                atlas_index,
            })
        })
        .collect::<Result<Vec<SilicaChunk>, _>>()?;

        Ok(SilicaLayer {
            image: SilicaImageData { chunks },
            mask: info
                .mask
                .take()
                .map(|mask| Self::load(*mask, params, true).map(Box::new))
                .transpose()?,
            info,
            id: params.allocate_layer_id(),
        })
    }

    /// Replace a section of the texture with raw RGBA data.
    ///
    /// ### Note
    /// The position `x` and `y` and size `width` and `height` data
    /// should strictly fit within the texture boundaries.
    fn replace_from_bytes(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        data: &[u8],
        origin: wgpu::Origin3d,
        size: wgpu::Extent3d,
    ) -> Result<(), SilicaError> {
        let texture_size = texture.size();
        let end_x = origin.x.checked_add(size.width);
        let end_y = origin.y.checked_add(size.height);
        if end_x.is_none_or(|end| end > texture_size.width)
            || end_y.is_none_or(|end| end > texture_size.height)
            || origin.z >= texture_size.depth_or_array_layers
        {
            return Err(SilicaError::ChunkUploadOutOfBounds);
        }
        queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual pixel data
            data,
            // The layout of the texture
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.width),
                rows_per_image: Some(size.height),
            },
            size,
        );
        Ok(())
    }
}

mod lz4 {
    use std::fmt::Debug;
    use std::io;

    use lz4_flex::frame::Error;

    const BLOCK_MAGIC_COMPRESSED: [u8; 4] = [0x62, 0x76, 0x34, 0x31];
    const BLOCK_MAGIC_UNCOMPRESSED: [u8; 4] = [0x62, 0x76, 0x34, 0x2d];
    const BLOCK_MAGIC_END: [u8; 4] = [0x62, 0x76, 0x34, 0x24];

    #[derive(Debug)]
    pub(crate) enum BlockInfo {
        Compressed(u32, u32),
        Uncompressed(u32),
        EndMark,
    }

    impl BlockInfo {
        fn read_bytes<'b>(r: &mut &'b [u8]) -> io::Result<&'b [u8; 4]> {
            let Some((bytes, rest)) = r.split_first_chunk::<4>() else {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Unexpected end of file",
                ));
            };
            *r = rest;
            Ok(bytes)
        }

        fn read_len(r: &[u8; 4]) -> io::Result<u32> {
            Ok(u32::from_le_bytes(*r))
        }

        pub(crate) fn read(mut r: &[u8]) -> Result<Self, Error> {
            match *Self::read_bytes(&mut r)? {
                BLOCK_MAGIC_COMPRESSED => {
                    // A compressed block header consists of the octets
                    // 0x62, 0x76, 0x34, and 0x31, followed by:

                    // the size in bytes of the decoded (plaintext) data
                    let decoded_len = Self::read_len(Self::read_bytes(&mut r)?)?;
                    // the size (in bytes) of the encoded data stored
                    let encoded_len = Self::read_len(Self::read_bytes(&mut r)?)?;
                    // both size fields as (possibly unaligned) 32-bit little-endian values

                    Ok(BlockInfo::Compressed(encoded_len, decoded_len))
                }
                BLOCK_MAGIC_UNCOMPRESSED => {
                    // An uncompressed block header consists of the octets
                    // 0x62, 0x76, 0x34, and 0x2d, followed by:

                    // the size in bytes of the decoded (plaintext) data
                    let decoded_len = Self::read_len(Self::read_bytes(&mut r)?)?;
                    // the size (in bytes) of the encoded data stored
                    let encoded_len = Self::read_len(Self::read_bytes(&mut r)?)?;

                    if decoded_len != encoded_len {
                        return Err(Error::BlockTooBig);
                    }

                    Ok(BlockInfo::Uncompressed(decoded_len))
                }
                BLOCK_MAGIC_END => Ok(BlockInfo::EndMark),
                _ => Err(Error::WrongMagicNumber),
            }
        }

        pub(crate) fn encoding_bytes(&self) -> usize {
            match self {
                BlockInfo::Compressed(_, _) | BlockInfo::Uncompressed(_) => 12,
                BlockInfo::EndMark => 4,
            }
        }
    }

    struct ChainDecoder<'a> {
        /// The underlying reader.
        src: &'a [u8],
        /// The decompressed bytes buffer. Bytes are decompressed from src to dst
        /// before being passed back to the caller.
        dst: &'a mut Vec<u8>,
        expected_len: usize,
    }

    impl<'a> ChainDecoder<'a> {
        /// Creates a new Decoder for the specified reader.
        fn new(src: &'a [u8], dst: &'a mut Vec<u8>, expected_len: usize) -> ChainDecoder<'a> {
            ChainDecoder {
                src,
                dst,
                expected_len,
            }
        }

        fn read_raw<'b>(r: &mut &'b [u8], len: usize) -> io::Result<&'b [u8]> {
            let Some((src, rest)) = r.split_at_checked(len) else {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Unexpected end of file",
                ));
            };
            *r = rest;
            Ok(src)
        }

        fn read_block(&mut self) -> io::Result<bool> {
            // Read and decompress block
            let block_info = BlockInfo::read(&self.src)?;
            self.src = &self.src[block_info.encoding_bytes()..];

            match block_info {
                BlockInfo::Uncompressed(len) => {
                    let len = len as usize;
                    if self.dst.len().saturating_add(len) > self.expected_len {
                        return Err(Error::BlockTooBig.into());
                    }

                    let src = Self::read_raw(&mut self.src, len)?;

                    self.dst.extend_from_slice(src);
                }
                BlockInfo::Compressed(len, block_size) => {
                    let len = len as usize;
                    let block_size = block_size as usize;

                    if len > block_size
                        || self.dst.len().saturating_add(block_size) > self.expected_len
                    {
                        return Err(Error::BlockTooBig.into());
                    }

                    let src = Self::read_raw(&mut self.src, len)?;

                    // Independent blocks OR linked blocks with only prefix data
                    let dst_end = self.dst.len();
                    self.dst.resize(dst_end + block_size, 0);
                    let (prev, dst) = self.dst.split_at_mut(dst_end);
                    debug_assert_eq!(dst.len(), block_size);
                    let decomp_size = lz4_flex::block::decompress_into_with_dict(src, dst, prev)
                        .map_err(Error::DecompressionError)?;

                    if decomp_size != block_size {
                        return Err(Error::ContentLengthError {
                            expected: block_size as u64,
                            actual: decomp_size as u64,
                        }
                        .into());
                    }

                    debug_assert_eq!(block_size, decomp_size);
                }

                BlockInfo::EndMark => {
                    return Ok(false);
                }
            }

            Ok(true)
        }

        fn decode(mut self) -> io::Result<()> {
            loop {
                match self.read_block() {
                    Ok(false) => return Ok(()),
                    Ok(true) => continue,
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }
    }

    pub fn decompress(src: &[u8], dst: &mut Vec<u8>, expected_len: usize) -> io::Result<()> {
        ChainDecoder::new(src, dst, expected_len).decode()
    }
}

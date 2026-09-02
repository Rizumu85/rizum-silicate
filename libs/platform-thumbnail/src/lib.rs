use std::{
    io::{Cursor, Read, Seek},
    path::Path,
};

use silica::quicklook::{
    QuickLookPngPath, extract_quicklook_png, extract_quicklook_png_from_reader,
};

pub const DEFAULT_THUMBNAIL_MAX_DIMENSION: u32 = 2048;
const MAX_DECODED_SOURCE_DIMENSION: u32 = 16_384;
const MAX_DECODE_ALLOC_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformThumbnailPng {
    pub source: PlatformThumbnailSource,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformThumbnailRgba {
    pub source: PlatformThumbnailSource,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformThumbnailSource {
    QuickLookPreview,
    QuickLookThumbnail,
}

#[derive(Debug)]
pub enum PlatformThumbnailError {
    Read(std::io::Error),
    Archive(silica::error::SilicaError),
    Decode(image::ImageError),
}

pub fn load_platform_thumbnail_png(
    path: impl AsRef<Path>,
) -> Result<Option<PlatformThumbnailPng>, PlatformThumbnailError> {
    let file = std::fs::File::open(path).map_err(PlatformThumbnailError::Read)?;
    load_platform_thumbnail_png_from_reader(file)
}

pub fn load_platform_thumbnail_png_from_archive_bytes(
    bytes: &[u8],
) -> Result<Option<PlatformThumbnailPng>, PlatformThumbnailError> {
    let Some(image) = extract_quicklook_png(bytes).map_err(PlatformThumbnailError::Archive)? else {
        return Ok(None);
    };

    Ok(Some(PlatformThumbnailPng {
        source: platform_thumbnail_source(image.path),
        bytes: image.bytes,
    }))
}

pub fn load_platform_thumbnail_png_from_reader(
    reader: impl Read + Seek,
) -> Result<Option<PlatformThumbnailPng>, PlatformThumbnailError> {
    let Some(image) =
        extract_quicklook_png_from_reader(reader).map_err(PlatformThumbnailError::Archive)?
    else {
        return Ok(None);
    };

    Ok(Some(PlatformThumbnailPng {
        source: platform_thumbnail_source(image.path),
        bytes: image.bytes,
    }))
}

pub fn load_platform_thumbnail_rgba(
    path: impl AsRef<Path>,
) -> Result<Option<PlatformThumbnailRgba>, PlatformThumbnailError> {
    load_platform_thumbnail_rgba_at_size(path, DEFAULT_THUMBNAIL_MAX_DIMENSION)
}

pub fn load_platform_thumbnail_rgba_at_size(
    path: impl AsRef<Path>,
    max_dimension: u32,
) -> Result<Option<PlatformThumbnailRgba>, PlatformThumbnailError> {
    let Some(png) = load_platform_thumbnail_png(path)? else {
        return Ok(None);
    };
    decode_platform_thumbnail_rgba(png, max_dimension).map(Some)
}

pub fn load_platform_thumbnail_rgba_from_archive_bytes(
    bytes: &[u8],
) -> Result<Option<PlatformThumbnailRgba>, PlatformThumbnailError> {
    load_platform_thumbnail_rgba_from_archive_bytes_at_size(bytes, DEFAULT_THUMBNAIL_MAX_DIMENSION)
}

pub fn load_platform_thumbnail_rgba_from_archive_bytes_at_size(
    bytes: &[u8],
    max_dimension: u32,
) -> Result<Option<PlatformThumbnailRgba>, PlatformThumbnailError> {
    let Some(png) = load_platform_thumbnail_png_from_archive_bytes(bytes)? else {
        return Ok(None);
    };
    decode_platform_thumbnail_rgba(png, max_dimension).map(Some)
}

pub fn decode_platform_thumbnail_rgba(
    png: PlatformThumbnailPng,
    max_dimension: u32,
) -> Result<PlatformThumbnailRgba, PlatformThumbnailError> {
    let mut reader =
        image::ImageReader::with_format(Cursor::new(&png.bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DECODED_SOURCE_DIMENSION);
    limits.max_image_height = Some(MAX_DECODED_SOURCE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC_BYTES);
    reader.limits(limits);
    let image = reader.decode().map_err(PlatformThumbnailError::Decode)?;
    let max_dimension = max_dimension.clamp(1, DEFAULT_THUMBNAIL_MAX_DIMENSION);
    let image = if image.width() > max_dimension || image.height() > max_dimension {
        image.thumbnail(max_dimension, max_dimension)
    } else {
        image
    }
    .to_rgba8();

    Ok(PlatformThumbnailRgba {
        source: png.source,
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

fn platform_thumbnail_source(path: QuickLookPngPath) -> PlatformThumbnailSource {
    match path {
        QuickLookPngPath::Preview => PlatformThumbnailSource::QuickLookPreview,
        QuickLookPngPath::Thumbnail => PlatformThumbnailSource::QuickLookThumbnail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Cursor, Write};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const PREVIEW_BYTES: &[u8] = b"\x89PNG\r\n\x1a\npreview";
    const THUMBNAIL_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nthumbnail";

    #[test]
    fn loads_quicklook_preview_before_thumbnail_from_procreate_path() {
        let path = temp_procreate_path("preview-before-thumbnail");
        fs::write(
            &path,
            zip_with_files([
                ("QuickLook/Thumbnail.png", THUMBNAIL_BYTES),
                ("QuickLook/Preview.png", PREVIEW_BYTES),
            ]),
        )
        .unwrap();

        let thumbnail = load_platform_thumbnail_png(&path).unwrap().unwrap();

        assert_eq!(thumbnail.source, PlatformThumbnailSource::QuickLookPreview);
        assert_eq!(thumbnail.bytes, PREVIEW_BYTES);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn falls_back_to_quicklook_thumbnail_when_preview_is_missing() {
        let path = temp_procreate_path("thumbnail-fallback");
        fs::write(
            &path,
            zip_with_files([("QuickLook/Thumbnail.png", THUMBNAIL_BYTES)]),
        )
        .unwrap();

        let thumbnail = load_platform_thumbnail_png(&path).unwrap().unwrap();

        assert_eq!(
            thumbnail.source,
            PlatformThumbnailSource::QuickLookThumbnail
        );
        assert_eq!(thumbnail.bytes, THUMBNAIL_BYTES);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn returns_none_when_archive_has_no_quicklook_png() {
        let path = temp_procreate_path("no-quicklook");
        fs::write(
            &path,
            zip_with_files([("Document.archive", b"plist".as_slice())]),
        )
        .unwrap();

        let thumbnail = load_platform_thumbnail_png(&path).unwrap();

        assert_eq!(thumbnail, None);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn decodes_quicklook_png_into_rgba_pixels_for_platform_hosts() {
        let path = temp_procreate_path("decoded-rgba");
        let png = png_with_rgba_pixels(2, 1, &[255, 0, 0, 255, 0, 128, 255, 64]);
        fs::write(
            &path,
            zip_with_files([("QuickLook/Preview.png", png.as_slice())]),
        )
        .unwrap();

        let thumbnail = load_platform_thumbnail_rgba(&path).unwrap().unwrap();

        assert_eq!(thumbnail.source, PlatformThumbnailSource::QuickLookPreview);
        assert_eq!(thumbnail.width, 2);
        assert_eq!(thumbnail.height, 1);
        assert_eq!(thumbnail.rgba, [255, 0, 0, 255, 0, 128, 255, 64]);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn decodes_quicklook_png_from_archive_bytes_for_extension_hosts() {
        let png = png_with_rgba_pixels(1, 2, &[10, 20, 30, 255, 40, 50, 60, 128]);
        let archive = zip_with_files([("QuickLook/Preview.png", png.as_slice())]);

        let thumbnail = load_platform_thumbnail_rgba_from_archive_bytes(&archive)
            .unwrap()
            .unwrap();

        assert_eq!(thumbnail.source, PlatformThumbnailSource::QuickLookPreview);
        assert_eq!(thumbnail.width, 1);
        assert_eq!(thumbnail.height, 2);
        assert_eq!(thumbnail.rgba, [10, 20, 30, 255, 40, 50, 60, 128]);
    }

    #[test]
    fn reports_decode_error_when_quicklook_png_is_invalid() {
        let path = temp_procreate_path("invalid-png");
        fs::write(
            &path,
            zip_with_files([("QuickLook/Preview.png", b"not a png".as_slice())]),
        )
        .unwrap();

        let error = load_platform_thumbnail_rgba(&path).unwrap_err();

        assert!(matches!(error, PlatformThumbnailError::Decode(_)));

        fs::remove_file(path).unwrap();
    }

    fn temp_procreate_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rizum-{label}-{nonce}.procreate"))
    }

    fn zip_with_files<const N: usize>(files: [(&str, &[u8]); N]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut archive = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (path, bytes) in files {
            archive.start_file(path, options).unwrap();
            archive.write_all(bytes).unwrap();
        }

        archive.finish().unwrap().into_inner()
    }

    fn png_with_rgba_pixels(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let image = image::RgbaImage::from_raw(width, height, rgba.to_vec()).unwrap();
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).unwrap();
        png.into_inner()
    }
}

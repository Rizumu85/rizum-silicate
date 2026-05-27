use std::path::Path;

use silica::quicklook::{QuickLookPngPath, extract_quicklook_png};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformThumbnailPng {
    pub source: PlatformThumbnailSource,
    pub bytes: Vec<u8>,
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
}

pub fn load_platform_thumbnail_png(
    path: impl AsRef<Path>,
) -> Result<Option<PlatformThumbnailPng>, PlatformThumbnailError> {
    let bytes = std::fs::read(path).map_err(PlatformThumbnailError::Read)?;
    let Some(image) = extract_quicklook_png(&bytes).map_err(PlatformThumbnailError::Archive)?
    else {
        return Ok(None);
    };

    Ok(Some(PlatformThumbnailPng {
        source: platform_thumbnail_source(image.path),
        bytes: image.bytes,
    }))
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
}

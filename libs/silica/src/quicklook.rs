use crate::{
    error::SilicaError,
    limits::{MAX_PROCREATE_ARCHIVE_BYTES, MAX_QUICKLOOK_PNG_BYTES, read_bounded},
};
use std::io::{Cursor, Read, Seek, SeekFrom};
use zip::{ZipArchive, result::ZipError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickLookPng {
    pub path: QuickLookPngPath,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickLookPngPath {
    Preview,
    Thumbnail,
}

impl QuickLookPngPath {
    pub fn as_archive_path(self) -> &'static str {
        match self {
            Self::Preview => "QuickLook/Preview.png",
            Self::Thumbnail => "QuickLook/Thumbnail.png",
        }
    }
}

pub fn extract_quicklook_png(bytes: &[u8]) -> Result<Option<QuickLookPng>, SilicaError> {
    extract_quicklook_png_from_reader(Cursor::new(bytes))
}

pub fn extract_quicklook_png_from_reader(
    mut reader: impl Read + Seek,
) -> Result<Option<QuickLookPng>, SilicaError> {
    let archive_size = reader.seek(SeekFrom::End(0))?;
    if archive_size > MAX_PROCREATE_ARCHIVE_BYTES {
        return Err(SilicaError::ResourceLimitExceeded {
            resource: "Procreate archive",
            limit: MAX_PROCREATE_ARCHIVE_BYTES,
            actual: archive_size,
        });
    }
    reader.seek(SeekFrom::Start(0))?;
    let mut archive = ZipArchive::new(reader)?;

    for path in [QuickLookPngPath::Preview, QuickLookPngPath::Thumbnail] {
        match archive.by_name(path.as_archive_path()) {
            Ok(mut file) => {
                let file_size = file.size();
                let image = read_bounded(
                    &mut file,
                    file_size,
                    "QuickLook PNG",
                    MAX_QUICKLOOK_PNG_BYTES,
                )?;
                return Ok(Some(QuickLookPng { path, bytes: image }));
            }
            Err(ZipError::FileNotFound) => {}
            Err(err) => return Err(err.into()),
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    const PREVIEW_BYTES: &[u8] = b"\x89PNG\r\n\x1a\npreview";
    const THUMBNAIL_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nthumbnail";

    #[test]
    fn extracts_preview_before_thumbnail() {
        let archive = zip_with_files([
            ("QuickLook/Thumbnail.png", THUMBNAIL_BYTES),
            ("QuickLook/Preview.png", PREVIEW_BYTES),
        ]);

        let image = extract_quicklook_png(&archive).unwrap().unwrap();

        assert_eq!(image.path, QuickLookPngPath::Preview);
        assert_eq!(image.bytes, PREVIEW_BYTES);
    }

    #[test]
    fn falls_back_to_thumbnail_when_preview_is_missing() {
        let archive = zip_with_files([("QuickLook/Thumbnail.png", THUMBNAIL_BYTES)]);

        let image = extract_quicklook_png(&archive).unwrap().unwrap();

        assert_eq!(image.path, QuickLookPngPath::Thumbnail);
        assert_eq!(image.bytes, THUMBNAIL_BYTES);
    }

    #[test]
    fn returns_none_when_no_quicklook_png_exists() {
        let archive = zip_with_files([("Document.archive", b"plist".as_slice())]);

        assert_eq!(extract_quicklook_png(&archive).unwrap(), None);
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

use std::path::Path;

use rizum_platform_thumbnail::{
    PlatformThumbnailError, PlatformThumbnailRgba, load_platform_thumbnail_rgba,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsThumbnailBitmap {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowsThumbnailBitmapError {
    PixelLengthMismatch {
        width: u32,
        height: u32,
        expected_len: usize,
        actual_len: usize,
    },
}

#[derive(Debug)]
pub enum WindowsThumbnailLoadError {
    Platform(PlatformThumbnailError),
    Bitmap(WindowsThumbnailBitmapError),
}

pub fn load_windows_thumbnail_bitmap(
    path: impl AsRef<Path>,
) -> Result<Option<WindowsThumbnailBitmap>, WindowsThumbnailLoadError> {
    let Some(thumbnail) =
        load_platform_thumbnail_rgba(path).map_err(WindowsThumbnailLoadError::Platform)?
    else {
        return Ok(None);
    };

    rgba_to_windows_bgra(&thumbnail)
        .map(Some)
        .map_err(WindowsThumbnailLoadError::Bitmap)
}

pub fn rgba_to_windows_bgra(
    thumbnail: &PlatformThumbnailRgba,
) -> Result<WindowsThumbnailBitmap, WindowsThumbnailBitmapError> {
    let expected_len = thumbnail.width as usize * thumbnail.height as usize * 4;
    if thumbnail.rgba.len() != expected_len {
        return Err(WindowsThumbnailBitmapError::PixelLengthMismatch {
            width: thumbnail.width,
            height: thumbnail.height,
            expected_len,
            actual_len: thumbnail.rgba.len(),
        });
    }

    let mut bgra = Vec::with_capacity(expected_len);
    for pixel in thumbnail.rgba.chunks_exact(4) {
        bgra.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }

    Ok(WindowsThumbnailBitmap {
        width: thumbnail.width,
        height: thumbnail.height,
        bgra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rizum_platform_thumbnail::PlatformThumbnailSource;
    use std::fs;
    use std::io::{Cursor, Write};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn converts_rgba_pixels_to_windows_bgra_order() {
        let thumbnail = PlatformThumbnailRgba {
            source: PlatformThumbnailSource::QuickLookPreview,
            width: 2,
            height: 1,
            rgba: vec![255, 0, 16, 255, 10, 20, 30, 128],
        };

        let bitmap = rgba_to_windows_bgra(&thumbnail).unwrap();

        assert_eq!(bitmap.width, 2);
        assert_eq!(bitmap.height, 1);
        assert_eq!(bitmap.bgra, [16, 0, 255, 255, 30, 20, 10, 128]);
    }

    #[test]
    fn reports_mismatched_rgba_buffer_length_before_conversion() {
        let thumbnail = PlatformThumbnailRgba {
            source: PlatformThumbnailSource::QuickLookPreview,
            width: 2,
            height: 2,
            rgba: vec![255, 0, 0, 255],
        };

        let error = rgba_to_windows_bgra(&thumbnail).unwrap_err();

        assert_eq!(
            error,
            WindowsThumbnailBitmapError::PixelLengthMismatch {
                width: 2,
                height: 2,
                expected_len: 16,
                actual_len: 4,
            }
        );
    }

    #[test]
    fn loads_windows_bitmap_from_procreate_quicklook_png() {
        let path = temp_procreate_path("windows-bitmap");
        let png = png_with_rgba_pixels(1, 1, &[1, 2, 3, 4]);
        fs::write(
            &path,
            zip_with_files([("QuickLook/Preview.png", png.as_slice())]),
        )
        .unwrap();

        let bitmap = load_windows_thumbnail_bitmap(&path).unwrap().unwrap();

        assert_eq!(bitmap.width, 1);
        assert_eq!(bitmap.height, 1);
        assert_eq!(bitmap.bgra, [3, 2, 1, 4]);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn returns_none_when_procreate_archive_has_no_quicklook_png() {
        let path = temp_procreate_path("no-quicklook");
        fs::write(
            &path,
            zip_with_files([("Document.archive", b"plist".as_slice())]),
        )
        .unwrap();

        let bitmap = load_windows_thumbnail_bitmap(&path).unwrap();

        assert_eq!(bitmap, None);

        fs::remove_file(path).unwrap();
    }

    fn temp_procreate_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rizum-windows-thumbnail-{label}-{nonce}.procreate"))
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

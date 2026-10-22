use super::{App, compositor::CompositorHandle};
use crate::export::{animation::AnimationExportProgress, still::StillExportBackground};
use eframe::wgpu;
use silicate_compositor::{buffer::BufferDimensions, tex::TextureExt};
use silicate_runtime::DocumentSnapshot;
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};

pub struct StillExportJob {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub compositor: CompositorHandle,
    pub snapshot: DocumentSnapshot,
    pub orientation: silica::Orientation,
    pub background: StillExportBackground,
    pub progress: Arc<AnimationExportProgress>,
}
impl StillExportJob {
    pub async fn export(mut self, path: PathBuf, replace_existing: bool) -> io::Result<()> {
        let format = image::ImageFormat::from_path(&path).map_err(io::Error::other)?;
        if self.background.is_transparent() && format == image::ImageFormat::Jpeg {
            return Err(io::Error::other("JPEG requires the document background"));
        }
        if format == image::ImageFormat::Jpeg {
            self.snapshot.background_color[3] = 1.0;
        }
        self.progress.check_cancelled()?;
        let texture = wgpu::Texture::empty(
            &self.device,
            self.snapshot.canvas_size.width,
            self.snapshot.canvas_size.height,
            wgpu::Texture::OUTPUT_USAGE,
        );
        self.compositor
            .render_still(&self.snapshot, self.background, &texture)
            .await
            .map_err(io::Error::other)?;
        let image = App::export(
            &texture,
            &self.device,
            &self.queue,
            BufferDimensions::from_extent(texture.size()),
            self.orientation,
        )
        .await
        .map_err(io::Error::other)?;
        self.progress.check_cancelled()?;
        tokio::task::spawn_blocking(move || {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(std::path::Path::new("."));
            let mut staged = tempfile::NamedTempFile::new_in(parent)?;
            {
                let mut writer = io::BufWriter::new(staged.as_file_mut());
                // DynamicImage converts RGBA to each encoder's supported color type (e.g. JPEG RGB).
                image::DynamicImage::ImageRgba8(image)
                    .write_to(&mut writer, format)
                    .map_err(io::Error::other)?;
                writer.flush()?;
            }
            self.progress.check_cancelled()?;
            if replace_existing {
                staged.persist(path).map_err(io::Error::other)?;
            } else {
                staged.persist_noclobber(path).map_err(io::Error::other)?;
            }
            Ok(())
        })
        .await
        .map_err(io::Error::other)?
    }
}

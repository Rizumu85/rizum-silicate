use eframe::wgpu;
use egui_dock::NodePath;
use egui_notify::Toast;
use silicate_compositor::buffer::BufferDimensions;
use std::sync::mpsc::Sender;

use crate::app::AppEvent;
#[cfg(not(target_arch = "wasm32"))]
use crate::export::{
    archived_video::{
        ArchivedVideoExportMode, ArchivedVideoStageDirectory, FsArchivedVideoStageWriter,
        export_archived_video_segments_with_ffmpeg_status_and_mode,
    },
    ffmpeg::{ProcessFfmpegCommandRunner, detect_current_ffmpeg_tool_status},
};

pub struct Dialog {
    event_sender: Sender<AppEvent>,
}

impl Dialog {
    pub fn new(event_sender: Sender<AppEvent>) -> Self {
        Self { event_sender }
    }

    fn send_toast(&self, toast: Toast) {
        self.event_sender.send(AppEvent::Toast(toast)).ok();
    }

    pub async fn load_dialog(self, node_path: NodePath) {
        let dialog = rfd::AsyncFileDialog::new()
            .add_filter("All Files", &["*"])
            .add_filter("Procreate Files", &["procreate"])
            .pick_file();

        let Some(handle) = dialog.await else {
            self.send_toast(Toast::info("Load cancelled."));
            return;
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.event_sender
                .send(AppEvent::LoadFile {
                    path: handle.path().to_path_buf(),
                    node_path: Some(node_path),
                })
                .unwrap();
        }
        #[cfg(target_arch = "wasm32")]
        {
            use std::sync::Arc;

            let data = handle.read().await;
            log::info!("File read complete, loading file...");
            self.event_sender
                .send(AppEvent::LoadFile {
                    bytes: Arc::from(data),
                    node_path: Some(node_path),
                })
                .unwrap();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn save_dialog(
        self,
        device: wgpu::Device,
        queue: wgpu::Queue,
        copied_texture: wgpu::Texture,
        orientation: silica_gpu::Orientation,
    ) {
        let dialog = rfd::AsyncFileDialog::new()
            .add_filter("png", image::ImageFormat::Png.extensions_str())
            .add_filter("jpeg", image::ImageFormat::Jpeg.extensions_str())
            .add_filter("tga", image::ImageFormat::Tga.extensions_str())
            .add_filter("tiff", image::ImageFormat::Tiff.extensions_str())
            .add_filter("webp", image::ImageFormat::WebP.extensions_str())
            .add_filter("bmp", image::ImageFormat::Bmp.extensions_str())
            .save_file();

        let Some(handle) = dialog.await else {
            self.send_toast(Toast::info("Export cancelled."));
            return;
        };

        let dim = BufferDimensions::from_extent(copied_texture.size());
        let path = handle.path().to_path_buf();
        let file_name = handle.file_name();

        let image =
            match crate::app::App::export(&copied_texture, &device, &queue, dim, orientation).await
            {
                Ok(image) => image,
                Err(error) => {
                    self.send_toast(Toast::error(format!(
                        "File {file_name} failed to export. Reason: {error}."
                    )));
                    return;
                }
            };

        log::info!("Saving the file to {}", path.display());
        let save_result = tokio::task::spawn_blocking(move || image.save(path)).await;

        match save_result {
            Ok(Ok(())) => {
                self.send_toast(Toast::success(format!(
                    "File {file_name} successfully exported."
                )));
            }
            Ok(Err(error)) => {
                self.send_toast(Toast::error(format!(
                    "File {file_name} failed to export. Reason: {error}."
                )));
            }
            Err(error) => {
                self.send_toast(Toast::error(format!(
                    "File {file_name} failed to export. Reason: {error}."
                )));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn archived_video_export_dialog(
        self,
        source_path: std::path::PathBuf,
        export_mode: ArchivedVideoExportMode,
    ) {
        let default_file_name = archived_video_default_file_name(&source_path, export_mode);
        let dialog = rfd::AsyncFileDialog::new()
            .set_file_name(&default_file_name)
            .add_filter("mp4", &["mp4"])
            .save_file();

        let Some(handle) = dialog.await else {
            self.send_toast(Toast::info("Video export cancelled."));
            return;
        };

        let output_path = handle.path().to_path_buf();
        let source_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Procreate file")
            .to_owned();

        let export_result = tokio::task::spawn_blocking(move || {
            let archive_bytes = silica::limits::read_procreate_archive(&source_path)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let ffmpeg_status = detect_current_ffmpeg_tool_status()?;
            let stage_dir =
                ArchivedVideoStageDirectory::create(&std::env::temp_dir(), &output_path)?;
            let mut writer = FsArchivedVideoStageWriter;
            let mut runner = ProcessFfmpegCommandRunner;

            export_archived_video_segments_with_ffmpeg_status_and_mode(
                &archive_bytes,
                stage_dir.path(),
                &ffmpeg_status,
                &output_path,
                export_mode,
                &mut writer,
                &mut runner,
            )
            .map_err(|err| std::io::Error::other(format!("{err:?}")))
        })
        .await
        .unwrap_or_else(|err| Err(std::io::Error::other(err.to_string())));

        if let Err(err) = export_result {
            self.send_toast(Toast::error(format!(
                "Video export for {source_name} failed. Reason: {err}."
            )));
        } else {
            self.send_toast(Toast::success(format!(
                "Video {} successfully exported.",
                handle.file_name()
            )));
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn save_dialog(
        self,
        device: wgpu::Device,
        queue: wgpu::Queue,
        copied_texture: wgpu::Texture,
        orientation: silica_gpu::Orientation,
    ) {
        let dim = BufferDimensions::from_extent(copied_texture.size());

        let image =
            match crate::app::App::export(&copied_texture, &device, &queue, dim, orientation).await
            {
                Ok(image) => image,
                Err(error) => {
                    self.send_toast(Toast::error(format!("Export failed. Reason: {error}.")));
                    return;
                }
            };

        let output_format = image::ImageFormat::Png;
        let mut writer = std::io::Cursor::new(Vec::new());
        if let Err(error) = image.write_to(&mut writer, output_format) {
            self.send_toast(Toast::error(format!("Export failed. Reason: {error}.")));
            return;
        }

        if crate::web::save_blob_as_png(writer.into_inner().as_slice())
            .await
            .is_err()
        {
            self.send_toast(Toast::error("Export download failed."));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn archived_video_default_file_name(
    source_path: &std::path::Path,
    export_mode: ArchivedVideoExportMode,
) -> String {
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Untitled Artwork");

    match export_mode {
        ArchivedVideoExportMode::FullLength => format!("{stem}.mp4"),
        ArchivedVideoExportMode::Preview30Seconds => format!("{stem} Preview.mp4"),
    }
}

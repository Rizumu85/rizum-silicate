use eframe::wgpu;
use egui_dock::NodePath;
use egui_notify::Toast;
#[cfg(target_arch = "wasm32")]
use silicate_compositor::buffer::BufferDimensions;
use std::sync::mpsc::Sender;

use crate::app::AppEvent;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::compositor::CompositorHandle;
#[cfg(not(target_arch = "wasm32"))]
use crate::export::archived_video::{ArchivedVideoExportMode, export_archived_video_file};
#[cfg(not(target_arch = "wasm32"))]
use crate::export::still::StillExportBackground;

pub struct Dialog {
    event_sender: Sender<AppEvent>,
}

impl Dialog {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn animation_export_dialog(
        self,
        job: crate::app::animation_export::AnimationExportJob,
        format: crate::export::animation_codec::AnimationExportFormat,
    ) {
        use std::sync::atomic::Ordering;
        let progress = job.progress.clone();
        let result = async {
            if format != crate::export::animation_codec::AnimationExportFormat::PngSequence {
                let file = rfd::AsyncFileDialog::new()
                    .set_file_name(format!("Animation.{}", format.extension()))
                    .add_filter(format.label(), &[format.extension()])
                    .save_file()
                    .await
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::Interrupted,
                            "Animation export cancelled",
                        )
                    })?;
                return job.export_encoded(file.path().to_owned(), format).await;
            }
            let folder = rfd::AsyncFileDialog::new()
                .set_title("Choose a folder for the animation sequence")
                .pick_folder()
                .await
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "Animation export cancelled",
                    )
                })?;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            job.export_sequence(folder.path().join(format!("Animation-{timestamp}")))
                .await
        }
        .await;
        progress.running.store(false, Ordering::Relaxed);
        match result {
            Ok(path) => {
                self.send_toast(Toast::success(format!(
                    "Animation exported to {}",
                    path.display()
                )));
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::Interrupted
                    || progress.cancelled.load(Ordering::Relaxed) =>
            {
                self.send_toast(Toast::info("Animation export cancelled."));
            }
            Err(error) => {
                self.send_toast(Toast::error(format!("Animation export failed: {error}")));
            }
        }
    }

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
        orientation: silica_gpu::Orientation,
        compositor: CompositorHandle,
        snapshot: silicate_runtime::DocumentSnapshot,
        background: StillExportBackground,
    ) {
        let mut dialog = rfd::AsyncFileDialog::new();
        for format in crate::export::still::StillExportFormat::ALL {
            if background.is_transparent()
                && format == crate::export::still::StillExportFormat::Jpeg
            {
                continue;
            }
            dialog = dialog.add_filter(format.label(), format.image_format().extensions_str());
        }
        let dialog = dialog.save_file();

        let Some(handle) = dialog.await else {
            self.send_toast(Toast::info("Export cancelled."));
            return;
        };

        let file_name = handle.file_name();
        let result = crate::app::still_export::StillExportJob {
            device,
            queue,
            compositor,
            snapshot,
            orientation,
            background,
            progress: std::sync::Arc::default(),
        }
        .export(handle.path().to_owned(), true)
        .await;
        match result {
            Ok(()) => self.send_toast(Toast::success(format!(
                "File {file_name} successfully exported."
            ))),
            Err(error) => self.send_toast(Toast::error(format!(
                "File {file_name} failed to export. Reason: {error}."
            ))),
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

        let export_result = export_archived_video_file(
            source_path,
            output_path,
            export_mode,
            std::sync::Arc::default(),
            true,
        )
        .await;

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

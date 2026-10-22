use super::{
    App, animation_export::AnimationExportJob, compositor::CompositorHandle,
    still_export::StillExportJob,
};
use crate::export::{
    animation_codec::AnimationExportFormat,
    archived_video::export_archived_video_file,
    batch::{BatchFormat, BatchItem, BatchQueue},
};
use std::{io, sync::Arc};

// A background document owns its runtime session and renderer for exactly one file.
struct ExportDocument {
    app: Arc<App>,
    snapshot: silicate_runtime::DocumentSnapshot,
    compositor: CompositorHandle,
    orientation: silica::Orientation,
    renderer: tokio::task::JoinHandle<()>,
}
impl Drop for ExportDocument {
    fn drop(&mut self) {
        self.renderer.abort();
        let _ = self.app.close_document(self.snapshot.document_id, true);
    }
}
pub async fn run(app: Arc<App>, queue: Arc<BatchQueue>) {
    let mut workers = Vec::new();
    for _ in 0..queue.options.parallelism {
        let app = app.clone();
        let queue = queue.clone();
        workers.push(tokio::spawn(async move {
            while let Some((index, item)) = queue.claim() {
                let result = export_one(app.clone(), &queue, item).await;
                queue.complete(index, result);
            }
        }));
    }
    for worker in workers {
        let _ = worker.await;
    }
    queue.finish();
    let message = queue.with_items(|items| {
        use crate::export::batch::BatchStatus;
        let exported = items
            .iter()
            .filter(|i| matches!(i.status, BatchStatus::Succeeded))
            .count();
        let failed = items
            .iter()
            .filter(|i| matches!(i.status, BatchStatus::Failed(_)))
            .count();
        let cancelled = items
            .iter()
            .filter(|i| matches!(i.status, BatchStatus::Cancelled))
            .count();
        format!("Batch finished: {exported} exported, {failed} failed, {cancelled} cancelled")
    });
    app.event_sender
        .send(super::AppEvent::Toast(egui_notify::Toast::info(message)))
        .ok();
}
async fn export_one(app: Arc<App>, queue: &BatchQueue, item: BatchItem) -> io::Result<()> {
    item.progress.check_cancelled()?;
    if item.output.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Output already exists; batch export never replaces files",
        ));
    }
    if let BatchFormat::ArchivedVideo(mode) = queue.options.format {
        return export_archived_video_file(item.source, item.output, mode, item.progress, false)
            .await;
    }
    let loader = app.clone();
    let loaded = tokio::task::spawn_blocking(move || loader.load_export_document(&item.source))
        .await
        .map_err(io::Error::other)?
        .map_err(io::Error::other)?;
    let renderer = tokio::spawn(loaded.compositor.export_thread());
    let document = ExportDocument {
        app: app.clone(),
        snapshot: loaded.snapshot,
        compositor: loaded.handle,
        orientation: loaded.file.orientation,
        renderer,
    };
    item.progress.check_cancelled()?;
    match queue.options.format {
        BatchFormat::Still(_) => {
            StillExportJob {
                device: app.device.clone(),
                queue: app.queue.clone(),
                compositor: document.compositor.clone(),
                snapshot: document.snapshot.clone(),
                orientation: document.orientation,
                background: queue.options.background,
                progress: item.progress,
            }
            .export(item.output, false)
            .await
        }
        BatchFormat::Animation(format) => {
            let job = AnimationExportJob {
                device: app.device.clone(),
                queue: app.queue.clone(),
                compositor: document.compositor.clone(),
                snapshot: document.snapshot.clone(),
                orientation: document.orientation,
                background: queue.options.background,
                repeat_holds: queue.options.repeat_holds,
                progress: item.progress,
            };
            if format == AnimationExportFormat::PngSequence {
                job.export_sequence(item.output).await?;
            } else {
                job.export_encoded(item.output, format).await?;
            }
            Ok(())
        }
        BatchFormat::ArchivedVideo(_) => unreachable!(),
    }
}

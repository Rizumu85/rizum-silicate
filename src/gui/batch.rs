use crate::{
    app::AppEvent,
    export::{
        animation_codec::AnimationExportFormat,
        archived_video::ArchivedVideoExportMode,
        batch::{BatchFormat, BatchOptions, BatchQueue, BatchStatus, output_name},
        still::{StillExportBackground, StillExportFormat},
    },
};
use egui::{Context, Ui};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::Ordering, mpsc::Sender},
    time::Duration,
};

pub struct BatchExportGui {
    pub open: bool,
    pub selecting: bool,
    sources: Vec<(PathBuf, bool)>,
    output: Option<PathBuf>,
    recursive: bool,
    template: String,
    format: BatchFormat,
    background: StillExportBackground,
    repeat_holds: bool,
    parallelism: usize,
    error: Option<String>,
    queue: Option<Arc<BatchQueue>>,
}
impl Default for BatchExportGui {
    fn default() -> Self {
        Self {
            open: false,
            selecting: false,
            sources: Vec::new(),
            output: None,
            recursive: false,
            template: "{name}-{index}".into(),
            format: BatchFormat::Still(StillExportFormat::Png),
            background: StillExportBackground::DocumentColor,
            repeat_holds: true,
            parallelism: 1,
            error: None,
            queue: None,
        }
    }
}
impl Drop for BatchExportGui {
    fn drop(&mut self) {
        if let Some(queue) = &self.queue {
            queue.cancel();
        }
    }
}
impl BatchExportGui {
    pub fn accept_sources(&mut self, result: Result<Vec<PathBuf>, String>) {
        self.selecting = false;
        match result {
            Ok(paths) => {
                let existing: std::collections::HashSet<_> =
                    self.sources.iter().map(|(p, _)| p.clone()).collect();
                let additions: Vec<_> = paths
                    .into_iter()
                    .filter(|p| !existing.contains(p))
                    .collect();
                if self.sources.len() + additions.len() > 10_000 {
                    self.error = Some(
                        "Choose at most 10,000 files; the last selection was not added".into(),
                    );
                } else {
                    self.sources
                        .extend(additions.into_iter().map(|p| (p, true)));
                }
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub fn accept_output(&mut self, output: Option<PathBuf>) {
        self.selecting = false;
        if output.is_some() {
            self.output = output;
        }
    }
    pub fn show(&mut self, ctx: &Context, sender: &Sender<AppEvent>) {
        let running = self
            .queue
            .as_ref()
            .is_some_and(|q| q.running.load(Ordering::Acquire));
        if running || self.selecting {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
        if !self.open {
            return;
        }
        let mut open = self.open;
        egui::Window::new("Batch export")
            .open(&mut open)
            .default_width(650.0)
            .max_height(ctx.content_rect().height() * 0.85)
            .vscroll(true)
            .resizable(true)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!running && !self.selecting, |ui| self.settings(ui, sender));
                if self.selecting { ui.label("Choosing files or reading the folder…"); }
                if let Some(error) = &self.error { ui.colored_label(ui.visuals().error_fg_color, error); }
                if let Some(queue) = &self.queue { show_results(ui, queue, self.selecting, sender); }
                ui.small("Existing outputs are preserved. Closing this window keeps the batch running; quitting the app cancels it.");
            });
        self.open = open;
    }
    fn settings(&mut self, ui: &mut Ui, sender: &Sender<AppEvent>) {
        ui.horizontal(|ui| {
            for (label, folder) in [("Add files…", false), ("Add folder…", true)] {
                if ui.button(label).clicked() {
                    self.selecting = true;
                    self.error = None;
                    sender
                        .send(AppEvent::SelectBatchSources {
                            folder,
                            recursive: self.recursive,
                        })
                        .ok();
                }
            }
            ui.checkbox(&mut self.recursive, "Include subfolders");
            if ui.button("Clear selection").clicked() {
                self.sources.clear();
            }
        });
        let selected_count = self
            .sources
            .iter()
            .filter(|(_, selected)| *selected)
            .count();
        ui.label(format!(
            "{selected_count} of {} files selected — saved files only",
            self.sources.len()
        ));
        egui::ScrollArea::vertical()
            .id_salt("batch-sources")
            .max_height(150.0)
            .show_rows(ui, 24.0, self.sources.len(), |ui, range| {
                for (path, selected) in &mut self.sources[range] {
                    ui.checkbox(
                        selected,
                        path.file_name().unwrap_or_default().to_string_lossy(),
                    )
                    .on_hover_text(path.display().to_string());
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Output folder…").clicked() {
                self.selecting = true;
                sender.send(AppEvent::SelectBatchOutput).ok();
            }
            ui.label(
                self.output
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Choose a folder".into()),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Format");
            egui::ComboBox::from_id_salt("batch-format")
                .selected_text(self.format.label())
                .show_ui(ui, |ui| {
                    ui.label("Still image");
                    for format in StillExportFormat::ALL {
                        ui.selectable_value(
                            &mut self.format,
                            BatchFormat::Still(format),
                            format.label(),
                        );
                    }
                    ui.separator();
                    ui.label("Animation");
                    for format in AnimationExportFormat::ALL {
                        ui.selectable_value(
                            &mut self.format,
                            BatchFormat::Animation(format),
                            format.label(),
                        );
                    }
                    ui.separator();
                    for mode in [
                        ArchivedVideoExportMode::FullLength,
                        ArchivedVideoExportMode::Preview30Seconds,
                    ] {
                        let format = BatchFormat::ArchivedVideo(mode);
                        ui.selectable_value(&mut self.format, format, format.label());
                    }
                });
            ui.label("Concurrent files");
            ui.add(egui::DragValue::new(&mut self.parallelism).range(1..=2));
        });
        if !matches!(self.format, BatchFormat::ArchivedVideo(_)) {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.background,
                    StillExportBackground::DocumentColor,
                    "Document background",
                );
                ui.selectable_value(
                    &mut self.background,
                    StillExportBackground::Transparent,
                    "Transparent",
                );
            });
        }
        if self.format == BatchFormat::Animation(AnimationExportFormat::PngSequence) {
            ui.checkbox(&mut self.repeat_holds, "Repeat held frames");
        }
        ui.horizontal(|ui| {
            ui.label("Filename");
            ui.text_edit_singleline(&mut self.template);
        });
        ui.small("Use {name}, {parent}, {index}. The format supplies the extension.");
        if let Some((first, _)) = self.sources.iter().find(|(_, selected)| *selected) {
            match output_name(first, 1, &self.template, self.format.extension()) {
                Ok(name) => {
                    ui.label(format!("Example: {name}"));
                }
                Err(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                }
            }
        }
        let needs_background = self.format == BatchFormat::Still(StillExportFormat::Jpeg)
            || matches!(self.format, BatchFormat::Animation(format) if !format.supports_alpha());
        let valid_background = !needs_background || !self.background.is_transparent();
        if !valid_background {
            ui.label("This format requires the document background.");
        }
        if ui
            .add_enabled(
                self.sources.iter().any(|(_, selected)| *selected)
                    && self.output.is_some()
                    && valid_background
                    && !self.selecting,
                egui::Button::new("Export selected files"),
            )
            .clicked()
        {
            self.error = None;
            let options = BatchOptions {
                format: self.format,
                background: self.background,
                repeat_holds: self.repeat_holds,
                parallelism: self.parallelism,
            };
            let selected: Vec<_> = self
                .sources
                .iter()
                .filter(|(_, selected)| *selected)
                .map(|(p, _)| p.clone())
                .collect();
            match BatchQueue::plan(
                &selected,
                self.output.as_ref().unwrap(),
                &self.template,
                options,
            ) {
                Ok(queue) => {
                    start(&queue, false, sender);
                    self.queue = Some(queue);
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        }
    }
}

fn start(queue: &Arc<BatchQueue>, retry: bool, sender: &Sender<AppEvent>) {
    if !queue.begin(retry) {
        return;
    }
    if sender
        .send(AppEvent::StartBatchExport(queue.clone()))
        .is_err()
    {
        queue.finish();
    }
}
fn show_results(ui: &mut Ui, queue: &Arc<BatchQueue>, selecting: bool, sender: &Sender<AppEvent>) {
    ui.separator();
    queue.with_items(|items| {
        let finished = items
            .iter()
            .filter(|i| !matches!(i.status, BatchStatus::Queued | BatchStatus::Running))
            .count();
        ui.label(format!(
            "{} results: {finished} / {}",
            queue.options.format.label(),
            items.len()
        ));
        egui::ScrollArea::vertical()
            .id_salt("batch-results")
            .max_height(220.0)
            .show_rows(ui, 44.0, items.len(), |ui, range| {
                for item in &items[range] {
                    result_row(ui, item);
                }
            });
    });
    if queue.running.load(Ordering::Acquire) {
        if ui.button("Cancel batch").clicked() {
            queue.cancel();
        }
    } else {
        let retry = queue.with_items(|items| {
            items
                .iter()
                .any(|i| matches!(i.status, BatchStatus::Failed(_) | BatchStatus::Cancelled))
        });
        if ui
            .add_enabled(
                retry && !selecting,
                egui::Button::new("Retry failed / cancelled (same settings)"),
            )
            .clicked()
        {
            start(queue, true, sender);
        }
    }
}
fn result_row(ui: &mut Ui, item: &crate::export::batch::BatchItem) {
    let status = match &item.status {
        BatchStatus::Queued => "Queued".into(),
        BatchStatus::Running => {
            let done = item.progress.completed.load(Ordering::Relaxed);
            let total = item.progress.total.load(Ordering::Relaxed);
            if item.progress.cancelled.load(Ordering::Relaxed) {
                "Cancelling…".into()
            } else if item.progress.encoding.load(Ordering::Relaxed) {
                "Encoding…".into()
            } else if total > 0 {
                format!("Frames {done}/{total}")
            } else {
                "Exporting…".into()
            }
        }
        BatchStatus::Succeeded => "Exported".into(),
        BatchStatus::Cancelled => "Cancelled".into(),
        BatchStatus::Failed(error) => format!("Failed: {error}"),
    };
    ui.columns(2, |columns| {
        columns[0]
            .add(
                egui::Label::new(
                    item.source
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                )
                .truncate(),
            )
            .on_hover_text(item.source.display().to_string());
        columns[1]
            .add(egui::Label::new(&status).truncate())
            .on_hover_text(status);
    });
    ui.add(
        egui::Label::new(
            egui::RichText::new(
                item.output
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            )
            .small(),
        )
        .truncate(),
    )
    .on_hover_text(item.output.display().to_string());
}

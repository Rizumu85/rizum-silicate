use crate::app::{
    AppEvent,
    file_browser::{Entry, Request, ResultKind, Worker},
};
use egui::{Context, TextureHandle, Vec2};
use egui_dock::NodePath;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::Ordering,
        mpsc::{Sender, TryRecvError},
    },
    time::Duration,
};

enum Preview {
    Image(TextureHandle),
    Missing,
    Failed(String),
}

#[derive(Default)]
pub struct FileBrowserGui {
    pub open: bool,
    pub selecting: bool,
    pub target: Option<NodePath>,
    folder: Option<PathBuf>,
    entries: Vec<Entry>,
    matches: Vec<usize>,
    worker: Option<Worker>,
    revision: u64,
    busy: bool,
    loading: bool,
    pending_folder: Option<PathBuf>,
    error: Option<String>,
    filter: String,
    previews: HashMap<PathBuf, (Preview, u64)>,
    clock: u64,
}

impl FileBrowserGui {
    pub fn accept_folder(&mut self, folder: Option<PathBuf>) {
        self.selecting = false;
        if let Some(folder) = folder {
            self.navigate(folder);
        }
    }

    fn navigate(&mut self, folder: PathBuf) {
        if self.worker.is_none() {
            match Worker::new() {
                Ok(worker) => self.worker = Some(worker),
                Err(error) => {
                    self.error = Some(error.to_string());
                    return;
                }
            }
        }
        self.revision = self.revision.wrapping_add(1);
        self.worker
            .as_ref()
            .unwrap()
            .generation
            .store(self.revision, Ordering::Relaxed);
        self.pending_folder = Some(folder);
        self.loading = true;
        self.error = None;
    }

    fn poll(&mut self, ctx: &Context) {
        if let Some(worker) = &self.worker {
            match worker.poll() {
                Ok((revision, result)) => {
                    self.busy = false;
                    if revision == self.revision {
                        match result {
                            ResultKind::Folder(path, result) => {
                                self.loading = false;
                                match result {
                                    Ok(entries) => {
                                        self.folder = Some(path);
                                        self.entries = entries;
                                        self.matches = (0..self.entries.len()).collect();
                                        self.previews.clear();
                                        self.filter.clear();
                                    }
                                    Err(error) => {
                                        self.error = Some(format!("{}: {error}", path.display()))
                                    }
                                }
                            }
                            ResultKind::Thumbnail(path, result) => {
                                let preview = match result {
                                    Ok(Some(image)) => Preview::Image(ctx.load_texture(
                                        format!("browser:{}:{}", revision, path.display()),
                                        egui::ColorImage::from_rgba_unmultiplied(
                                            [image.width as usize, image.height as usize],
                                            &image.rgba,
                                        ),
                                        egui::TextureOptions::LINEAR,
                                    )),
                                    Ok(None) => Preview::Missing,
                                    Err(error) => Preview::Failed(error),
                                };
                                // 128 previews at 256 px bound cached RGBA payload to 32 MiB.
                                if self.previews.len() >= 128
                                    && let Some(oldest) = self
                                        .previews
                                        .iter()
                                        .min_by_key(|(_, (_, stamp))| *stamp)
                                        .map(|(path, _)| path.clone())
                                {
                                    self.previews.remove(&oldest);
                                }
                                self.previews.insert(path, (preview, self.clock));
                            }
                        }
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    self.error =
                        Some("File browser worker stopped. Choose a folder to retry.".into());
                    self.worker = None;
                    self.busy = false;
                    self.loading = false;
                    self.pending_folder = None;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if !self.busy
            && let Some(path) = self.pending_folder.take()
        {
            self.busy = self
                .worker
                .as_ref()
                .is_some_and(|worker| worker.request(self.revision, Request::Folder(path)));
        }
        if self.busy || self.selecting {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    pub fn show(&mut self, ctx: &Context, sender: &Sender<AppEvent>) {
        self.poll(ctx);
        if !self.open {
            return;
        }
        self.clock = self.clock.wrapping_add(1);
        let mut open = self.open;
        let mut navigate = None;
        let mut selected = None;
        egui::Window::new("Browse artwork")
            .open(&mut open)
            .default_size([680.0, 480.0])
            .max_size(ctx.content_rect().size() * 0.85)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(!self.selecting, egui::Button::new("Choose folder…"))
                        .clicked()
                    {
                        self.selecting = true;
                        sender.send(AppEvent::SelectBrowserFolder).ok();
                    }
                    let parent = self
                        .folder
                        .as_ref()
                        .and_then(|path| path.parent())
                        .map(PathBuf::from);
                    if ui
                        .add_enabled(!self.loading && parent.is_some(), egui::Button::new("Up"))
                        .clicked()
                    {
                        navigate = parent;
                    }
                    if ui
                        .add_enabled(
                            !self.loading && self.folder.is_some(),
                            egui::Button::new("Refresh"),
                        )
                        .clicked()
                    {
                        navigate = self.folder.clone();
                    }
                    if ui.button("Open file…").clicked() {
                        sender
                            .send(AppEvent::LoadDialog(
                                self.target.unwrap_or(NodePath::MAIN_ROOT),
                            ))
                            .ok();
                    }
                });
                if let Some(folder) = &self.folder {
                    ui.label(folder.display().to_string());
                }
                if self.selecting {
                    ui.label("Choosing a folder…");
                }
                if self.loading {
                    ui.label("Reading folder…");
                }
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if self.folder.is_none() && !self.loading && self.error.is_none() {
                    ui.label("Choose a folder to browse Procreate artwork.");
                }
                if ui
                    .add_enabled(
                        !self.loading && self.folder.is_some(),
                        egui::TextEdit::singleline(&mut self.filter).hint_text("Filter by name"),
                    )
                    .changed()
                {
                    let filter = self.filter.to_lowercase();
                    self.matches = self
                        .entries
                        .iter()
                        .enumerate()
                        .filter(|(_, entry)| entry.search_name.contains(&filter))
                        .map(|(index, _)| index)
                        .collect();
                }
                if !self.loading && self.folder.is_some() && self.matches.is_empty() {
                    ui.label(if self.entries.is_empty() {
                        "No Procreate artwork or subfolders here."
                    } else {
                        "No names match this filter."
                    });
                }
                ui.add_enabled_ui(!self.loading, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(("artwork-browser", self.folder.clone()))
                        .auto_shrink([false, false])
                        .show_rows(ui, 72.0, self.matches.len(), |ui, rows| {
                            for row in rows {
                                let entry = &self.entries[self.matches[row]];
                                let path = entry.path.clone();
                                let directory = entry.directory;
                                ui.push_id(&path, |ui| {
                                    ui.horizontal(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            Vec2::splat(72.0),
                                            egui::Sense::hover(),
                                        );
                                        if directory {
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                "Folder",
                                                egui::FontId::proportional(12.0),
                                                ui.visuals().text_color(),
                                            );
                                        } else if let Some((preview, stamp)) =
                                            self.previews.get_mut(&path)
                                        {
                                            *stamp = self.clock;
                                            match preview {
                                                Preview::Image(texture) => {
                                                    let size = texture.size_vec2();
                                                    let scale = (rect.width() / size.x)
                                                        .min(rect.height() / size.y);
                                                    egui::Image::new(&*texture).paint_at(
                                                        ui,
                                                        egui::Rect::from_center_size(
                                                            rect.center(),
                                                            size * scale,
                                                        ),
                                                    );
                                                }
                                                Preview::Missing | Preview::Failed(_) => {
                                                    ui.painter().text(
                                                        rect.center(),
                                                        egui::Align2::CENTER_CENTER,
                                                        "No preview",
                                                        egui::FontId::proportional(11.0),
                                                        ui.visuals().weak_text_color(),
                                                    );
                                                }
                                            }
                                        } else {
                                            ui.painter().rect_filled(
                                                rect,
                                                ui.visuals().widgets.noninteractive.corner_radius,
                                                ui.visuals().faint_bg_color,
                                            );
                                            if !self.busy && !self.loading {
                                                self.busy =
                                                    self.worker.as_ref().is_some_and(|worker| {
                                                        worker.request(
                                                            self.revision,
                                                            Request::Thumbnail(path.clone()),
                                                        )
                                                    });
                                                ctx.request_repaint_after(Duration::from_millis(
                                                    50,
                                                ));
                                            }
                                        }
                                        let response = ui.add_sized(
                                            [ui.available_width(), 72.0],
                                            egui::Button::new(&entry.name).truncate(),
                                        );
                                        let preview_response = ui.interact(
                                            rect,
                                            ui.id().with("preview"),
                                            egui::Sense::click(),
                                        );
                                        if response.clicked() || preview_response.clicked() {
                                            if directory {
                                                navigate = Some(path.clone());
                                            } else {
                                                selected = Some(path.clone());
                                            }
                                        }
                                        response.on_hover_text(path.display().to_string());
                                        if let Some((Preview::Failed(error), _)) =
                                            self.previews.get(&path)
                                        {
                                            preview_response.on_hover_text(error);
                                        }
                                    });
                                });
                            }
                        });
                });
            });
        self.open = open;
        if let Some(path) = navigate {
            self.navigate(path);
            ctx.request_repaint();
        }
        if let Some(path) = selected {
            sender
                .send(AppEvent::LoadFile {
                    path,
                    node_path: self.target,
                })
                .ok();
            self.open = false;
        }
    }
}

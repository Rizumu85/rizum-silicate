mod canvas;
mod controls;
pub(crate) mod settings;
mod silicate;
pub(crate) mod theme;
mod widgets;
pub(crate) mod workspace;

use egui::{Frame, *};
use egui_dock::NodePath;
use egui_dock::tab_viewer::OnCloseResponse;
use lucide_icons::Icon;
use std::collections::HashMap;
use std::sync::{Arc, mpsc::Sender};
use std::time::{Duration, Instant};

use crate::app::{
    App, AppEvent,
    instance::{Instance, InstanceKey},
};
use canvas::CanvasView;
use controls::ControlsGui;
use settings::{SettingsGui, SettingsState};
use silicate::ContinuousMutation;
use silicate::background::BackgroundControl;
use silicate::hierarchy::{LayerMutationIntent, LayersHierarchy};
use silicate_runtime::{
    AnimationPlaybackSnapshot, DocumentSnapshot, HistoryGroupId, RuntimeError, RuntimeUpdate,
};
use theme::{ACCENT_TEAL, Palette, glass_frame, icon};
use workspace::{
    HistoryAction, PlaybackIntent, WorkspacePanel, show_dock, show_history_controls, show_panel,
    show_playback_controls,
};

pub struct ViewOptions {
    pub extended_crosshair: bool,
    pub smooth: bool,
    pub grid: bool,
}

struct CanvasGui<'a> {
    app: &'a Arc<App>,
    event_sender: &'a Sender<AppEvent>,
    instances: &'a mut HashMap<InstanceKey, Instance>,
    view_options: &'a mut ViewOptions,
    settings: &'a mut SettingsState,
    active_panel: &'a mut WorkspacePanel,
    history_grouping: &'a mut HistoryGrouping,
    pending_close: &'a mut Option<InstanceKey>,
    exit_after_dirty_closes: &'a mut bool,
    focused_tab: Option<InstanceKey>,
}

#[derive(Default)]
pub(crate) struct HistoryGrouping {
    next_id: u64,
    active: Option<HistoryGroupId>,
}

impl HistoryGrouping {
    fn group_for<T>(&mut self, edit: &ContinuousMutation<T>) -> Option<HistoryGroupId> {
        if edit.started || (edit.pointer_active && self.active.is_none()) {
            self.next_id = self.next_id.wrapping_add(1).max(1);
            self.active = Some(HistoryGroupId::new(self.next_id));
        }

        (edit.started || edit.pointer_active || edit.stopped)
            .then_some(self.active)
            .flatten()
    }

    fn finish<T>(&mut self, edit: &ContinuousMutation<T>) {
        if edit.stopped {
            self.active = None;
        }
    }

    pub(crate) fn reset(&mut self) {
        self.active = None;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CloseChoice {
    KeepEditing,
    DiscardChanges,
}

fn apply_document_update(
    instance: &mut Instance,
    event_sender: &Sender<AppEvent>,
    update: Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError>,
    action: &str,
) {
    match update {
        Ok(update) => instance.apply_runtime_update(update),
        Err(error) => {
            event_sender
                .send(AppEvent::Toast(egui_notify::Toast::error(format!(
                    "Failed to {action}: {error}"
                ))))
                .ok();
        }
    }
}

fn apply_animation_playback_update(
    instance: &mut Instance,
    event_sender: &Sender<AppEvent>,
    update: Result<RuntimeUpdate<AnimationPlaybackSnapshot>, RuntimeError>,
    action: &str,
) {
    match update {
        Ok(update) => instance.apply_animation_playback_update(update),
        Err(error) => {
            event_sender
                .send(AppEvent::Toast(egui_notify::Toast::error(format!(
                    "Failed to {action}: {error}"
                ))))
                .ok();
        }
    }
}

fn tick_animation_playback(
    ctx: &Context,
    app: &Arc<App>,
    event_sender: &Sender<AppEvent>,
    instance: &mut Instance,
) {
    if let Some(elapsed) = instance.animation_tick_elapsed(Instant::now()) {
        let update = app.advance_animation(instance.snapshot.document_id, elapsed);
        apply_animation_playback_update(instance, event_sender, update, "advance animation");
    }

    if instance
        .snapshot
        .animation_playback
        .is_some_and(|playback| playback.playing)
        && let Some(frame_rate) = instance
            .snapshot
            .animation
            .as_ref()
            .map(|animation| animation.frame_rate)
    {
        ctx.request_repaint_after(Duration::from_secs_f64(1.0 / f64::from(frame_rate)));
    }
}

impl egui_dock::TabViewer for CanvasGui<'_> {
    type Tab = InstanceKey;

    fn allowed_in_windows(&self, _: &mut Self::Tab) -> bool {
        false
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let Some(instance) = self.instances.get_mut(tab) else {
            return;
        };
        let workspace_bounds = ui.max_rect();
        tick_animation_playback(ui.ctx(), self.app, self.event_sender, instance);

        CanvasView::new(
            *tab,
            instance.canvas.map(Image::from_texture),
            &mut instance.rotation,
        )
        .show_extended_crosshair(self.view_options.extended_crosshair)
        .show_grid(self.view_options.grid)
        .show(ui);

        if let Some(action) = show_history_controls(
            ui.ctx(),
            workspace_bounds,
            &instance.snapshot,
            self.focused_tab == Some(*tab) && self.pending_close.is_none(),
        ) {
            let update = match action {
                HistoryAction::Undo => self.app.undo(instance.snapshot.document_id),
                HistoryAction::Redo => self.app.redo(instance.snapshot.document_id),
            };
            apply_document_update(instance, self.event_sender, update, "update edit history");
        }

        *self.active_panel = show_dock(
            ui.ctx(),
            workspace_bounds,
            *self.active_panel,
            instance.snapshot.animation_playback.is_some(),
        );

        let mut canvas_flip_intent = None;
        let mut layer_intents = Vec::new();
        let mut playback_intent = PlaybackIntent::default();
        let mut background_intent = Default::default();
        let panel_id = Id::new(("rizum-workspace-panel", *tab, *self.active_panel));
        let close_requested = match *self.active_panel {
            WorkspacePanel::Canvas => false,
            WorkspacePanel::Playback => {
                playback_intent =
                    show_playback_controls(ui.ctx(), workspace_bounds, &instance.snapshot);
                false
            }
            WorkspacePanel::Layers => {
                let layer_states = instance
                    .snapshot
                    .layers
                    .iter()
                    .map(|layer| (layer.layer_id, layer))
                    .collect::<HashMap<_, _>>();
                let (_, close_requested) = show_panel(
                    ui.ctx(),
                    panel_id,
                    workspace_bounds,
                    "Layers",
                    340.0,
                    |ui| {
                        LayersHierarchy {
                            rotation: instance.rotation,
                            flipped: silica_gpu::Flipped {
                                horizontally: instance.snapshot.flipped.horizontally,
                                vertically: instance.snapshot.flipped.vertically,
                            },
                            previews: &instance.previews,
                            layers: &instance.file.layers,
                            states: &layer_states,
                            intents: &mut layer_intents,
                        }
                        .ui(ui);

                        ui.add_space(6.0);
                        background_intent = BackgroundControl {
                            color: instance.snapshot.background_color,
                            visible: instance.snapshot.background_visible,
                        }
                        .ui(ui);
                    },
                );
                close_requested
            }
            WorkspacePanel::Info => {
                let (_, close_requested) = show_panel(
                    ui.ctx(),
                    panel_id,
                    workspace_bounds,
                    "Details",
                    300.0,
                    |ui| ControlsGui::layout_info(ui, instance),
                );
                close_requested
            }
            WorkspacePanel::Export => {
                let (_, close_requested) = show_panel(
                    ui.ctx(),
                    panel_id,
                    workspace_bounds,
                    "Export",
                    340.0,
                    |ui| ControlsGui::layout_export_control(ui, self.event_sender, instance),
                );
                close_requested
            }
            WorkspacePanel::Settings => {
                let (_, close_requested) = show_panel(
                    ui.ctx(),
                    panel_id,
                    workspace_bounds,
                    "Settings",
                    360.0,
                    |ui| {
                        canvas_flip_intent = ControlsGui::layout_canvas_settings(
                            ui,
                            self.app,
                            self.event_sender,
                            self.view_options,
                            instance,
                        );
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);
                        SettingsGui::new(self.settings).ui(ui);
                    },
                );
                close_requested
            }
        };

        if close_requested {
            *self.active_panel = WorkspacePanel::Canvas;
        }

        if let Some(active) = playback_intent.active {
            let update = self
                .app
                .set_animation_playback_active(instance.snapshot.document_id, active);
            apply_animation_playback_update(
                instance,
                self.event_sender,
                update,
                "change Animation Assist",
            );
        }
        if let Some(mode) = playback_intent.mode {
            let update = self
                .app
                .set_animation_playback_mode(instance.snapshot.document_id, mode);
            apply_animation_playback_update(
                instance,
                self.event_sender,
                update,
                "change playback mode",
            );
        }
        if let Some(direction) = playback_intent.direction {
            let update = self
                .app
                .set_animation_playback_direction(instance.snapshot.document_id, direction);
            apply_animation_playback_update(
                instance,
                self.event_sender,
                update,
                "change playback direction",
            );
        }
        if let Some(slot_index) = playback_intent.slot_index {
            let update = self
                .app
                .seek_animation_timeline(instance.snapshot.document_id, slot_index);
            apply_animation_playback_update(instance, self.event_sender, update, "seek animation");
        }
        if let Some(playing) = playback_intent.playing {
            let update = self
                .app
                .set_animation_playing(instance.snapshot.document_id, playing);
            apply_animation_playback_update(instance, self.event_sender, update, "change playback");
        }
        let history_group = self
            .history_grouping
            .group_for(&playback_intent.onion_skin_settings);
        if let Some(settings) = playback_intent.onion_skin_settings.value {
            let update = self.app.set_animation_onion_skin_settings(
                instance.snapshot.document_id,
                settings,
                history_group,
            );
            apply_document_update(instance, self.event_sender, update, "update onion skins");
        }
        self.history_grouping
            .finish(&playback_intent.onion_skin_settings);

        for intent in layer_intents {
            match intent {
                LayerMutationIntent::BlendMode {
                    layer_id,
                    blend_mode,
                } => {
                    let update = self.app.set_layer_blend_mode(
                        instance.snapshot.document_id,
                        layer_id,
                        blend_mode,
                    );
                    apply_document_update(instance, self.event_sender, update, "update layer");
                }
                LayerMutationIntent::Clipped { layer_id, clipped } => {
                    let update = self.app.set_layer_clipped(
                        instance.snapshot.document_id,
                        layer_id,
                        clipped,
                    );
                    apply_document_update(instance, self.event_sender, update, "update layer");
                }
                LayerMutationIntent::Opacity { layer_id, edit } => {
                    let history_group = self.history_grouping.group_for(&edit);
                    if let Some(opacity) = edit.value {
                        let update = self.app.set_layer_opacity(
                            instance.snapshot.document_id,
                            layer_id,
                            opacity,
                            history_group,
                        );
                        apply_document_update(instance, self.event_sender, update, "update layer");
                    }
                    self.history_grouping.finish(&edit);
                }
                LayerMutationIntent::Visibility { layer_id, visible } => {
                    let update = self.app.set_layer_visibility(
                        instance.snapshot.document_id,
                        layer_id,
                        visible,
                    );
                    apply_document_update(instance, self.event_sender, update, "update layer");
                }
            }
        }

        let history_group = self.history_grouping.group_for(&background_intent.color);
        if let Some(color) = background_intent.color.value {
            let update =
                self.app
                    .set_background_color(instance.snapshot.document_id, color, history_group);
            apply_document_update(
                instance,
                self.event_sender,
                update,
                "update background color",
            );
        }
        self.history_grouping.finish(&background_intent.color);
        if let Some(visible) = background_intent.visibility {
            let update = self
                .app
                .set_background_visibility(instance.snapshot.document_id, visible);
            apply_document_update(
                instance,
                self.event_sender,
                update,
                "update background visibility",
            );
        }
        if let Some(flipped) = canvas_flip_intent {
            let update = self
                .app
                .set_canvas_flipped(instance.snapshot.document_id, flipped);
            apply_document_update(instance, self.event_sender, update, "flip canvas");
        }

        if let Err(error) = instance.submit_to_compositor() {
            self.event_sender
                .send(AppEvent::Toast(egui_notify::Toast::error(format!(
                    "Failed to project document state: {error}"
                ))))
                .ok();
            self.event_sender
                .send(AppEvent::RemoveInstance {
                    key: *tab,
                    discard_changes: true,
                })
                .ok();
        }
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        if self
            .instances
            .get(tab)
            .is_some_and(|instance| instance.snapshot.dirty)
        {
            *self.exit_after_dirty_closes = false;
            *self.pending_close = Some(*tab);
            OnCloseResponse::Focus
        } else {
            self.event_sender
                .send(AppEvent::RemoveInstance {
                    key: *tab,
                    discard_changes: false,
                })
                .unwrap();
            OnCloseResponse::Close
        }
    }

    fn on_add(&mut self, node_path: egui_dock::NodePath) {
        self.event_sender.send(AppEvent::LoadDialog(node_path)).ok();
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        let Some(instance) = self.instances.get(tab) else {
            return "Untitled Artwork".into();
        };
        let mut title = instance
            .snapshot
            .title
            .to_owned()
            .unwrap_or_else(|| "Untitled Artwork".to_owned());
        if instance.snapshot.dirty {
            title.push_str(" *");
        }
        title.into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        Id::new(*tab)
    }
}

pub struct ViewerGui {
    pub app: Arc<App>,
    pub event_sender: Sender<AppEvent>,
    pub instances: HashMap<InstanceKey, Instance>,

    pub view_options: ViewOptions,
    pub settings: SettingsState,
    pub active_panel: WorkspacePanel,
    pub canvas_tree: egui_dock::DockState<InstanceKey>,
    pub(crate) history_grouping: HistoryGrouping,
    pub(crate) pending_close: Option<InstanceKey>,
    pub(crate) exit_after_dirty_closes: bool,
}

impl ViewerGui {
    pub(crate) fn intercept_window_close(&mut self, ctx: &Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }

        let dirty = self.pending_close.or_else(|| {
            self.instances
                .iter()
                .find_map(|(key, instance)| instance.snapshot.dirty.then_some(*key))
        });
        if let Some(key) = dirty {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.exit_after_dirty_closes = true;
            self.pending_close = Some(key);
        }
    }

    fn show_close_confirmation(&mut self, ui: &mut Ui) {
        let Some(key) = self.pending_close else {
            return;
        };
        let Some(instance) = self.instances.get(&key) else {
            self.pending_close = None;
            return;
        };
        let title = instance
            .snapshot
            .title
            .as_deref()
            .unwrap_or("Untitled Artwork")
            .to_owned();

        let response = Modal::new(Id::new(("discard-document-changes", key)))
            .frame(glass_frame(ui, false))
            .show(ui.ctx(), |ui| {
                let palette = Palette::from_ui(ui);
                ui.set_width(320.0);
                ui.heading(RichText::new("Discard unsaved changes?").color(palette.ink));
                ui.add_space(6.0);
                ui.label(format!(
                    "Changes to {title} have not been saved to a Procreate file."
                ));
                ui.add_space(16.0);
                let mut choice = None;
                ui.horizontal(|ui| {
                    if ui.button("Keep Editing").clicked() {
                        choice = Some(CloseChoice::KeepEditing);
                    }
                    if ui
                        .add(
                            Button::new(RichText::new("Discard Changes").color(palette.surface))
                                .fill(palette.ink),
                        )
                        .clicked()
                    {
                        choice = Some(CloseChoice::DiscardChanges);
                    }
                });
                choice
            });

        if response.inner == Some(CloseChoice::DiscardChanges) {
            self.event_sender
                .send(AppEvent::RemoveInstance {
                    key,
                    discard_changes: true,
                })
                .ok();
            self.pending_close = None;
        } else if response.inner == Some(CloseChoice::KeepEditing) || response.should_close() {
            self.pending_close = None;
            self.exit_after_dirty_closes = false;
        }
    }

    fn layout_view(&mut self, ui: &mut Ui) {
        ui.set_min_size(ui.available_size());

        if self.instances.is_empty() {
            self.history_grouping.reset();
            let bounds = ui.max_rect();
            if self.active_panel != WorkspacePanel::Settings {
                self.active_panel = WorkspacePanel::Canvas;
            }
            Area::new(Id::new("rizum-empty-state"))
                .order(Order::Foreground)
                .fixed_pos(bounds.center())
                .pivot(Align2::CENTER_CENTER)
                .show(ui.ctx(), |ui| {
                    ui.set_width(300.0_f32.min((bounds.width() - 32.0).max(220.0)));
                    glass_frame(ui, false).show(ui, |ui| {
                        let palette = Palette::from_ui(ui);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(icon(Icon::Image).to_string())
                                    .size(28.0)
                                    .color(ACCENT_TEAL),
                            );
                            ui.add_space(6.0);
                            ui.heading(RichText::new("Rizum Silicate").color(palette.ink));

                            let git_hash =
                                option_env!("SILICATE_GIT_HASH").unwrap_or("unknown hash");
                            let version = crate::built_info::PKG_VERSION;
                            ui.label(
                                RichText::new(format!("v{version}  {git_hash}"))
                                    .small()
                                    .color(palette.caption),
                            );
                            ui.add_space(14.0);

                            ui.horizontal(|ui| {
                                let settings_width = 38.0;
                                let open_width =
                                    (ui.available_width() - settings_width - 8.0).max(120.0);
                                if ui
                                    .add_sized(
                                        [open_width, 38.0],
                                        Button::new(format!(
                                            "{}  Open artwork",
                                            icon(Icon::FolderOpen)
                                        )),
                                    )
                                    .clicked()
                                {
                                    self.event_sender
                                        .send(AppEvent::LoadDialog(NodePath::MAIN_ROOT))
                                        .ok();
                                }

                                if ui
                                    .add_sized(
                                        [settings_width, 38.0],
                                        Button::new(icon(Icon::Settings).to_string()),
                                    )
                                    .on_hover_text("Settings")
                                    .clicked()
                                {
                                    self.active_panel =
                                        if self.active_panel == WorkspacePanel::Settings {
                                            WorkspacePanel::Canvas
                                        } else {
                                            WorkspacePanel::Settings
                                        };
                                }
                            });

                            #[cfg(target_arch = "wasm32")]
                            if ui.button("Load demo artwork").clicked() {
                                self.event_sender.send(AppEvent::LoadDemoFile).ok();
                            }
                        });
                    });
                });

            if self.active_panel == WorkspacePanel::Settings {
                let (_, close_requested) = show_panel(
                    ui.ctx(),
                    Id::new("rizum-empty-settings"),
                    bounds,
                    "Settings",
                    360.0,
                    |ui| {
                        ControlsGui::layout_appearance(ui, &self.event_sender);
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);
                        SettingsGui::new(&mut self.settings).ui(ui);
                    },
                );
                if close_requested {
                    self.active_panel = WorkspacePanel::Canvas;
                }
            }
        } else {
            let focused_tab = self.canvas_tree.find_active_focused().map(|(_, tab)| *tab);
            egui_dock::DockArea::new(&mut self.canvas_tree)
                .id(Id::new("view.dock"))
                .style({
                    let palette = Palette::from_ui(ui);
                    let corner_radius = CornerRadius::same(8);

                    let mut style = egui_dock::Style::from_egui(ui.style());
                    style.tab.tab_body.inner_margin = Margin::ZERO;
                    style.tab_bar.height = 44.0;
                    style.tab_bar.hline_color = Color32::TRANSPARENT;
                    style.tab_bar.inner_margin = Margin::same(8);

                    style.tab.spacing = 6.0;

                    style.tab_bar.bg_fill = Color32::TRANSPARENT;

                    style.tab.active.corner_radius = corner_radius;
                    style.tab.active.bg_fill = Color32::TRANSPARENT;
                    style.tab.active.outline_color = Color32::TRANSPARENT;

                    style.tab.inactive.corner_radius = corner_radius;
                    style.tab.inactive.bg_fill = Color32::TRANSPARENT;
                    style.tab.inactive.outline_color = Color32::TRANSPARENT;

                    style.tab.focused.corner_radius = corner_radius;
                    style.tab.focused.outline_color = palette.surface_line;
                    style.tab.focused.bg_fill = palette.surface;
                    style.tab.focused.text_color = palette.ink;

                    style.tab.hovered.corner_radius = corner_radius;
                    style.tab.hovered.bg_fill = palette.surface_muted;
                    style.tab.hovered.outline_color = Color32::TRANSPARENT;

                    style.buttons.close_tab_bg_fill = Color32::TRANSPARENT;

                    style
                })
                .show_add_buttons(true)
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(false)
                .show_inside(
                    ui,
                    &mut CanvasGui {
                        app: &self.app,
                        view_options: &mut self.view_options,
                        settings: &mut self.settings,
                        active_panel: &mut self.active_panel,
                        history_grouping: &mut self.history_grouping,
                        pending_close: &mut self.pending_close,
                        exit_after_dirty_closes: &mut self.exit_after_dirty_closes,
                        focused_tab,
                        instances: &mut self.instances,
                        event_sender: &self.event_sender,
                    },
                );
            self.show_close_confirmation(ui);
        }
    }

    pub fn layout_gui(&mut self, ui: &mut Ui) {
        CentralPanel::default()
            .frame(Frame::NONE.fill(ui.style().visuals.panel_fill))
            .show_inside(ui, |ui| {
                self.layout_view(ui);

                ui.input(|i| {
                    i.raw.dropped_files.iter().for_each(|file| {
                        if let Some(path) = &file.path {
                            #[cfg(not(target_arch = "wasm32"))]
                            self.event_sender
                                .send(AppEvent::LoadFile {
                                    path: path.to_path_buf(),
                                    node_path: None,
                                })
                                .ok();
                            #[cfg(target_arch = "wasm32")]
                            {
                                self.event_sender
                                    .send(AppEvent::Toast(egui_notify::Toast::error(
                                        "File drag/drop is not supported on this platform.",
                                    )))
                                    .ok();
                                let _ = path;
                            }
                        } else if let Some(bytes) = &file.bytes {
                            #[cfg(target_arch = "wasm32")]
                            self.event_sender
                                .send(AppEvent::LoadFile {
                                    bytes: bytes.clone(),
                                    node_path: None,
                                })
                                .ok();
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                self.event_sender
                                    .send(AppEvent::Toast(egui_notify::Toast::error(
                                        "File drag/drop with in-memory data is not supported on this platform.",
                                    )))
                                    .ok();
                                let _ = bytes;
                            }
                        }
                    });
                })
            });
    }
}

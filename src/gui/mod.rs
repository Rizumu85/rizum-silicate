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

use crate::app::{
    App, AppEvent,
    instance::{Instance, InstanceKey},
};
use canvas::CanvasView;
use controls::ControlsGui;
use settings::{SettingsGui, SettingsState};
use silicate::background::BackgroundControl;
use silicate::hierarchy::{LayerMutationIntent, LayersHierarchy};
use silicate_runtime::{DocumentSnapshot, RuntimeError, RuntimeUpdate};
use theme::{ACCENT_TEAL, Palette, glass_frame, icon};
use workspace::{WorkspacePanel, show_dock, show_panel};

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

        CanvasView::new(
            *tab,
            instance.canvas.map(Image::from_texture),
            &mut instance.rotation,
        )
        .show_extended_crosshair(self.view_options.extended_crosshair)
        .show_grid(self.view_options.grid)
        .show(ui);

        *self.active_panel = show_dock(ui.ctx(), workspace_bounds, *self.active_panel);

        let mut canvas_flip_intent = None;
        let mut layer_intents = Vec::new();
        let layer_states = instance
            .snapshot
            .layers
            .iter()
            .map(|layer| (layer.layer_id, layer))
            .collect::<HashMap<_, _>>();
        let mut background_intent = Default::default();
        let panel_id = Id::new(("rizum-workspace-panel", *tab, *self.active_panel));
        let close_requested = match *self.active_panel {
            WorkspacePanel::Canvas | WorkspacePanel::Playback => false,
            WorkspacePanel::Layers => {
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

        for intent in layer_intents {
            let update =
                match intent {
                    LayerMutationIntent::BlendMode {
                        layer_id,
                        blend_mode,
                    } => self.app.set_layer_blend_mode(
                        instance.snapshot.document_id,
                        layer_id,
                        blend_mode,
                    ),
                    LayerMutationIntent::Clipped { layer_id, clipped } => self
                        .app
                        .set_layer_clipped(instance.snapshot.document_id, layer_id, clipped),
                    LayerMutationIntent::Opacity { layer_id, opacity } => self
                        .app
                        .set_layer_opacity(instance.snapshot.document_id, layer_id, opacity),
                    LayerMutationIntent::Visibility { layer_id, visible } => self
                        .app
                        .set_layer_visibility(instance.snapshot.document_id, layer_id, visible),
                };
            apply_document_update(instance, self.event_sender, update, "update layer");
        }

        if let Some(color) = background_intent.color {
            let update = self
                .app
                .set_background_color(instance.snapshot.document_id, color);
            apply_document_update(
                instance,
                self.event_sender,
                update,
                "update background color",
            );
        }
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
            self.event_sender.send(AppEvent::RemoveInstance(*tab)).ok();
        }
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        self.event_sender
            .send(AppEvent::RemoveInstance(*tab))
            .unwrap();
        OnCloseResponse::Close
    }

    fn on_add(&mut self, node_path: egui_dock::NodePath) {
        self.event_sender.send(AppEvent::LoadDialog(node_path)).ok();
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        self.instances
            .get(tab)
            .and_then(|tab| tab.snapshot.title.to_owned())
            .unwrap_or("Untitled Artwork".to_string())
            .into()
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
}

impl ViewerGui {
    fn layout_view(&mut self, ui: &mut Ui) {
        ui.set_min_size(ui.available_size());

        if self.instances.is_empty() {
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
                        instances: &mut self.instances,
                        event_sender: &self.event_sender,
                    },
                );
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

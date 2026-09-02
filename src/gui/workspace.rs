use egui::{
    Align, Align2, Area, Button, Color32, Context, Id, Key, KeyboardShortcut, Layout, Modifiers,
    Order, Rect, Response, RichText, ScrollArea, Stroke, Ui, pos2, vec2,
};
use lucide_icons::Icon;
use silicate_runtime::DocumentSnapshot;

use super::theme::{Palette, glass_frame, icon};

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub enum WorkspacePanel {
    #[default]
    Canvas,
    Layers,
    Playback,
    Info,
    Export,
    Settings,
}

impl WorkspacePanel {
    pub const PRIMARY: [Self; 4] = [Self::Canvas, Self::Layers, Self::Playback, Self::Info];

    pub fn label(self) -> &'static str {
        match self {
            Self::Canvas => "Canvas",
            Self::Layers => "Layers",
            Self::Playback => "Playback",
            Self::Info => "Info",
            Self::Export => "Export",
            Self::Settings => "Settings",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Self::Canvas => Icon::Maximize,
            Self::Layers => Icon::Layers,
            Self::Playback => Icon::Play,
            Self::Info => Icon::Info,
            Self::Export => Icon::Share2,
            Self::Settings => Icon::Settings,
        }
    }
}

#[derive(Clone, Copy)]
pub enum HistoryAction {
    Undo,
    Redo,
}

pub fn show_history_controls(
    ctx: &Context,
    bounds: Rect,
    snapshot: &DocumentSnapshot,
    listen_for_shortcuts: bool,
) -> Option<HistoryAction> {
    let mut action = None;
    if listen_for_shortcuts {
        let redo = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
        let alternate_redo = KeyboardShortcut::new(Modifiers::COMMAND, Key::Y);
        let undo = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
        if snapshot.can_redo
            && (ctx.input_mut(|input| input.consume_shortcut(&redo))
                || ctx.input_mut(|input| input.consume_shortcut(&alternate_redo)))
        {
            action = Some(HistoryAction::Redo);
        } else if snapshot.can_undo && ctx.input_mut(|input| input.consume_shortcut(&undo)) {
            action = Some(HistoryAction::Undo);
        }
    }

    Area::new(Id::new(("rizum-history-controls", snapshot.document_id)))
        .order(Order::Foreground)
        .fixed_pos(pos2(bounds.left() + 18.0, bounds.top() + 18.0))
        .show(ctx, |ui| {
            glass_frame(ui, true).show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            snapshot.can_undo,
                            Button::new(icon(Icon::Undo2).to_string()).min_size(vec2(34.0, 32.0)),
                        )
                        .on_hover_text("Undo")
                        .clicked()
                    {
                        action = Some(HistoryAction::Undo);
                    }
                    if ui
                        .add_enabled(
                            snapshot.can_redo,
                            Button::new(icon(Icon::Redo2).to_string()).min_size(vec2(34.0, 32.0)),
                        )
                        .on_hover_text("Redo")
                        .clicked()
                    {
                        action = Some(HistoryAction::Redo);
                    }
                });
            });
        });

    action
}

fn dock_button(ui: &mut Ui, panel: WorkspacePanel, active: bool, show_label: bool) -> Response {
    let palette = Palette::from_ui(ui);
    let text = if show_label {
        format!("{}  {}", icon(panel.icon()), panel.label())
    } else {
        icon(panel.icon()).to_string()
    };

    let response = ui.add(
        Button::new(RichText::new(text).color(if active {
            palette.ink
        } else {
            palette.ink_muted
        }))
        .fill(if active {
            palette.surface
        } else {
            Color32::TRANSPARENT
        })
        .stroke(if active {
            Stroke::new(1.0, palette.surface_line)
        } else {
            Stroke::NONE
        })
        .corner_radius(8)
        .min_size(if show_label {
            vec2(84.0, 34.0)
        } else {
            vec2(36.0, 34.0)
        }),
    );

    if show_label {
        response
    } else {
        response.on_hover_text(panel.label())
    }
}

pub fn show_dock(ctx: &Context, bounds: Rect, active_panel: WorkspacePanel) -> WorkspacePanel {
    let mut next_panel = active_panel;
    let show_labels = bounds.width() >= 720.0;

    Area::new(Id::new("rizum-workspace-dock"))
        .order(Order::Foreground)
        .fixed_pos(pos2(bounds.center().x, bounds.bottom() - 18.0))
        .pivot(Align2::CENTER_BOTTOM)
        .show(ctx, |ui| {
            glass_frame(ui, true).show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                ui.horizontal(|ui| {
                    for panel in WorkspacePanel::PRIMARY {
                        if panel == WorkspacePanel::Playback {
                            let response = ui
                                .add_enabled_ui(false, |ui| {
                                    dock_button(ui, panel, false, show_labels)
                                })
                                .inner;
                            response
                                .on_disabled_hover_text("Animation Assist is not available yet");
                            continue;
                        }

                        if dock_button(ui, panel, panel == active_panel, show_labels).clicked() {
                            next_panel = if panel == active_panel {
                                WorkspacePanel::Canvas
                            } else {
                                panel
                            };
                        }
                    }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    if dock_button(
                        ui,
                        WorkspacePanel::Export,
                        active_panel == WorkspacePanel::Export,
                        show_labels,
                    )
                    .clicked()
                    {
                        next_panel = if active_panel == WorkspacePanel::Export {
                            WorkspacePanel::Canvas
                        } else {
                            WorkspacePanel::Export
                        };
                    }

                    if dock_button(
                        ui,
                        WorkspacePanel::Settings,
                        active_panel == WorkspacePanel::Settings,
                        false,
                    )
                    .clicked()
                    {
                        next_panel = if active_panel == WorkspacePanel::Settings {
                            WorkspacePanel::Canvas
                        } else {
                            WorkspacePanel::Settings
                        };
                    }
                });
            });
        });

    next_panel
}

pub fn show_panel<R>(
    ctx: &Context,
    id: Id,
    bounds: Rect,
    title: &str,
    preferred_width: f32,
    add_body: impl FnOnce(&mut Ui) -> R,
) -> (R, bool) {
    let mut close_requested = false;
    let width = preferred_width.min((bounds.width() - 32.0).max(220.0));
    let max_height = (bounds.height() - 104.0).max(180.0);

    let response = Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(pos2(bounds.right() - 18.0, bounds.top() + 18.0))
        .pivot(Align2::RIGHT_TOP)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.set_max_height(max_height);
            glass_frame(ui, false)
                .show(ui, |ui| {
                    let palette = Palette::from_ui(ui);
                    ui.horizontal(|ui| {
                        ui.heading(RichText::new(title).color(palette.ink));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .button(icon(Icon::X).to_string())
                                .on_hover_text("Close")
                                .clicked()
                            {
                                close_requested = true;
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ScrollArea::vertical()
                        .id_salt(id.with("scroll"))
                        .max_height(max_height - 58.0)
                        .show(ui, add_body)
                        .inner
                })
                .inner
        });

    (response.inner, close_requested)
}

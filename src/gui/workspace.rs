use egui::{
    Align, Align2, Area, Button, Color32, Context, Id, Key, KeyboardShortcut, Layout, Modifiers,
    Order, Rect, Response, RichText, ScrollArea, Slider, Stroke, Ui, pos2, vec2,
};
use lucide_icons::Icon;
use silicate_runtime::{
    AnimationOnionSkinSettings, AnimationPlaybackDirection, AnimationPlaybackMode, DocumentSnapshot,
};

use super::silicate::ContinuousMutation;
use super::theme::{Palette, glass_frame, icon};
use super::widgets::opacity_slider::OpacitySlider;
use super::widgets::timeline_slider::TimelineSlider;

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

#[derive(Default)]
pub struct PlaybackIntent {
    pub active: Option<bool>,
    pub playing: Option<bool>,
    pub mode: Option<AnimationPlaybackMode>,
    pub direction: Option<AnimationPlaybackDirection>,
    pub slot_index: Option<u64>,
    pub onion_skin_settings: ContinuousMutation<AnimationOnionSkinSettings>,
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

pub fn show_dock(
    ctx: &Context,
    bounds: Rect,
    active_panel: WorkspacePanel,
    playback_available: bool,
) -> WorkspacePanel {
    let mut next_panel = if active_panel == WorkspacePanel::Playback && !playback_available {
        WorkspacePanel::Canvas
    } else {
        active_panel
    };
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
                        if panel == WorkspacePanel::Playback && !playback_available {
                            let response = ui
                                .add_enabled_ui(false, |ui| {
                                    dock_button(ui, panel, false, show_labels)
                                })
                                .inner;
                            response.on_disabled_hover_text("No Animation Assist data");
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

pub fn show_playback_controls(
    ctx: &Context,
    bounds: Rect,
    snapshot: &DocumentSnapshot,
) -> PlaybackIntent {
    let Some(playback) = snapshot.animation_playback else {
        return PlaybackIntent::default();
    };
    let Some(animation) = snapshot.animation.as_ref() else {
        return PlaybackIntent::default();
    };
    let mut intent = PlaybackIntent::default();
    let width = 560.0_f32.min((bounds.width() - 32.0).max(260.0));

    Area::new(Id::new(("rizum-animation-playback", snapshot.document_id)))
        .order(Order::Foreground)
        .fixed_pos(pos2(bounds.center().x, bounds.bottom() - 82.0))
        .pivot(Align2::CENTER_BOTTOM)
        .show(ctx, |ui| {
            ui.set_width(width);
            glass_frame(ui, true).show(ui, |ui| {
                let palette = Palette::from_ui(ui);
                ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                ui.horizontal(|ui| {
                    let play_icon = if playback.playing {
                        Icon::Pause
                    } else {
                        Icon::Play
                    };
                    let play_label = if playback.playing { "Pause" } else { "Play" };
                    if ui
                        .add_enabled(
                            playback.slot_count > 0,
                            Button::new(icon(play_icon).to_string())
                                .corner_radius(8)
                                .min_size(vec2(36.0, 34.0)),
                        )
                        .on_hover_text(play_label)
                        .clicked()
                    {
                        intent.playing = Some(!playback.playing);
                    }

                    let mut slot_index = playback.slot_index;
                    if ui
                        .add_sized(
                            vec2((ui.available_width() - 104.0).max(90.0), 24.0),
                            TimelineSlider::new(&mut slot_index, playback.slot_count),
                        )
                        .on_hover_text("Animation frame")
                        .changed()
                    {
                        intent.slot_index = Some(slot_index);
                    }

                    let current = if playback.slot_count == 0 {
                        0
                    } else {
                        playback.slot_index + 1
                    };
                    ui.label(
                        RichText::new(format!("{current}/{}", playback.slot_count))
                            .monospace()
                            .color(palette.ink_muted),
                    );
                    ui.label(
                        RichText::new(format!("{} FPS", animation.frame_rate))
                            .color(palette.caption),
                    );
                });

                ui.horizontal_wrapped(|ui| {
                    let mut active = playback.active;
                    if ui.checkbox(&mut active, "Assist").changed() {
                        intent.active = Some(active);
                    }

                    ui.separator();
                    let mut mode = playback.mode;
                    let mut mode_changed = false;
                    mode_changed |= ui
                        .selectable_value(&mut mode, AnimationPlaybackMode::Loop, "Loop")
                        .changed();
                    mode_changed |= ui
                        .selectable_value(&mut mode, AnimationPlaybackMode::PingPong, "Ping Pong")
                        .changed();
                    mode_changed |= ui
                        .selectable_value(&mut mode, AnimationPlaybackMode::OneShot, "One Shot")
                        .changed();
                    if mode_changed {
                        intent.mode = Some(mode);
                    }

                    ui.separator();
                    let mut direction = playback.direction;
                    let mut direction_changed = false;
                    direction_changed |= ui
                        .selectable_value(
                            &mut direction,
                            AnimationPlaybackDirection::Forward,
                            "Forward",
                        )
                        .changed();
                    direction_changed |= ui
                        .selectable_value(
                            &mut direction,
                            AnimationPlaybackDirection::Reverse,
                            "Reverse",
                        )
                        .changed();
                    if direction_changed {
                        intent.direction = Some(direction);
                    }
                });

                let mut onion_skin_settings = animation.onion_skin_settings();
                ui.horizontal(|ui| {
                    ui.label("Onion frames");
                    let frame_response = ui.add_sized(
                        vec2(150.0, 24.0),
                        Slider::new(&mut onion_skin_settings.frame_count, 0..=12),
                    );
                    intent
                        .onion_skin_settings
                        .merge(ContinuousMutation::from_response(
                            frame_response.changed().then_some(onion_skin_settings),
                            &frame_response,
                        ));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let blend_response = ui.checkbox(
                            &mut onion_skin_settings.blend_primary_frame,
                            "Blend current",
                        );
                        intent
                            .onion_skin_settings
                            .merge(ContinuousMutation::from_response(
                                blend_response.changed().then_some(onion_skin_settings),
                                &blend_response,
                            ));
                    });
                });

                let opacity_response = OpacitySlider::new(&mut onion_skin_settings.opacity)
                    .label("Onion opacity")
                    .ui(ui);
                intent
                    .onion_skin_settings
                    .merge(ContinuousMutation::from_response(
                        opacity_response.changed().then_some(onion_skin_settings),
                        &opacity_response,
                    ));
            });
        });

    intent
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

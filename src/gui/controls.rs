use std::sync::{Arc, mpsc::Sender};

use egui::*;
use lucide_icons::Icon;
use silicate_runtime::CanvasFlipped;

use crate::app::{App, AppEvent, instance::Instance};
#[cfg(not(target_arch = "wasm32"))]
use crate::export::archived_video::ArchivedVideoExportMode;
use crate::export::still::StillExportBackground;

use super::{
    ViewOptions,
    theme::{Palette, icon},
};

pub struct ControlsGui;

impl ControlsGui {
    fn section_label(ui: &mut Ui, label: &str) {
        let palette = Palette::from_ui(ui);
        ui.label(
            RichText::new(label)
                .size(11.5)
                .strong()
                .color(palette.ink_muted),
        );
        ui.add_space(4.0);
    }

    fn info_row(ui: &mut Ui, label: &str, value: impl Into<WidgetText>) {
        let palette = Palette::from_ui(ui);
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).color(palette.ink_muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(value);
            });
        });
    }

    pub fn layout_info(ui: &mut Ui, instance: &Instance) {
        let snapshot = &instance.snapshot;
        let mut width = snapshot.canvas_size.width;
        let mut height = snapshot.canvas_size.height;
        if !instance.is_upright() {
            std::mem::swap(&mut width, &mut height);
        }

        Self::info_row(
            ui,
            "Name",
            snapshot.title.as_deref().unwrap_or("Not Specified"),
        );
        Self::info_row(
            ui,
            "Author",
            snapshot.author.as_deref().unwrap_or("Not Specified"),
        );
        Self::info_row(ui, "Canvas", format!("{width} x {height} px"));
        Self::info_row(ui, "Layers", snapshot.layer_count.to_string());
        Self::info_row(ui, "Strokes", snapshot.stroke_count.to_string());
    }

    pub fn layout_appearance(ui: &mut Ui, event_sender: &Sender<AppEvent>) {
        Self::section_label(ui, "Appearance");
        ui.horizontal(|ui| {
            let mut theme = ui.ctx().options(|options| options.theme_preference);
            let mut changed = false;
            changed |= ui
                .selectable_value(&mut theme, egui::ThemePreference::System, "System")
                .changed();
            changed |= ui
                .selectable_value(&mut theme, egui::ThemePreference::Light, "Light")
                .changed();
            changed |= ui
                .selectable_value(&mut theme, egui::ThemePreference::Dark, "Dark")
                .changed();
            if changed {
                event_sender.send(AppEvent::SetTheme(theme)).ok();
            }
        });
    }

    pub fn layout_canvas_settings(
        ui: &mut Ui,
        app: &Arc<App>,
        event_sender: &Sender<AppEvent>,
        view_options: &mut ViewOptions,
        instance: &mut Instance,
    ) -> Option<CanvasFlipped> {
        let mut flip_intent = None;
        Self::layout_appearance(ui, event_sender);

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);
        Self::section_label(ui, "Canvas");
        ui.horizontal(|ui| {
            ui.label("Grid");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.selectable_value(&mut view_options.grid, true, "On");
                ui.selectable_value(&mut view_options.grid, false, "Off");
            });
        });
        ui.horizontal(|ui| {
            ui.label("Crosshair");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.selectable_value(&mut view_options.extended_crosshair, true, "On");
                ui.selectable_value(&mut view_options.extended_crosshair, false, "Off");
            });
        });
        ui.horizontal(|ui| {
            ui.label("Sampling");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut changed = false;
                changed |= ui
                    .selectable_value(&mut view_options.smooth, true, "Linear")
                    .changed();
                changed |= ui
                    .selectable_value(&mut view_options.smooth, false, "Nearest")
                    .changed();
                if changed {
                    app.rebind_texture(instance.id);
                }
            });
        });

        ui.add_space(8.0);
        ui.label(RichText::new("Rotation").color(Palette::from_ui(ui).ink_muted));
        ui.add(
            Slider::new(&mut instance.rotation, 0.0..=std::f32::consts::TAU)
                .custom_formatter(|value, _| format!("{:.0}", value.to_degrees()))
                .custom_parser(|value| value.parse::<f64>().map(|d| d.to_radians()).ok())
                .suffix(" deg"),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Flip");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let is_upright = instance.is_upright();
                let mut horizontal = instance.snapshot.flipped.horizontally;
                let mut vertical = instance.snapshot.flipped.vertically;

                if !is_upright {
                    std::mem::swap(&mut horizontal, &mut vertical);
                }

                let mut changed = false;
                if ui
                    .button(format!("{}  Horizontal", icon(Icon::FlipHorizontal2)))
                    .clicked()
                {
                    horizontal = !horizontal;
                    changed = true;
                }
                if ui
                    .button(format!("{}  Vertical", icon(Icon::FlipVertical2)))
                    .clicked()
                {
                    vertical = !vertical;
                    changed = true;
                }

                if !is_upright {
                    std::mem::swap(&mut horizontal, &mut vertical);
                }

                if changed {
                    flip_intent = Some(CanvasFlipped {
                        horizontally: horizontal,
                        vertically: vertical,
                    });
                }
            });
        });
        flip_intent
    }

    pub fn layout_export_control(
        ui: &mut Ui,
        event_sender: &Sender<AppEvent>,
        instance: &mut Instance,
    ) {
        let palette = Palette::from_ui(ui);
        Self::section_label(ui, "Image");
        let mut transparent = instance.still_export_background.is_transparent();
        if ui
            .checkbox(&mut transparent, "Transparent background")
            .changed()
        {
            instance.still_export_background = if transparent {
                StillExportBackground::Transparent
            } else {
                StillExportBackground::DocumentColor
            };
        }
        ui.add_space(8.0);
        if ui
            .add_sized(
                [ui.available_width(), 38.0],
                Button::new(format!("{}  Export image", icon(Icon::Image))),
            )
            .clicked()
        {
            event_sender
                .send(AppEvent::SaveDialog {
                    key: instance.id,
                    background: instance.still_export_background,
                })
                .ok();
        }
        ui.label(
            RichText::new("Full canvas in the document orientation")
                .small()
                .color(palette.caption),
        );

        #[cfg(not(target_arch = "wasm32"))]
        if instance.has_archived_video_segments()
            && let Some(source_path) = &instance.source_path
        {
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            Self::section_label(ui, "Time-lapse");
            if ui
                .add_sized(
                    [ui.available_width(), 38.0],
                    Button::new(format!("{}  Full length", icon(Icon::Video))),
                )
                .clicked()
            {
                event_sender
                    .send(AppEvent::ExportArchivedVideoDialog {
                        source_path: source_path.clone(),
                        export_mode: ArchivedVideoExportMode::FullLength,
                    })
                    .ok();
            }
            if ui
                .add_sized(
                    [ui.available_width(), 38.0],
                    Button::new(format!("{}  30 second preview", icon(Icon::Clock))),
                )
                .clicked()
            {
                event_sender
                    .send(AppEvent::ExportArchivedVideoDialog {
                        source_path: source_path.clone(),
                        export_mode: ArchivedVideoExportMode::Preview30Seconds,
                    })
                    .ok();
            }
        }
    }
}

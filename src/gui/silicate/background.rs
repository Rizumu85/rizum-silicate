use egui::*;

use crate::gui::widgets::{color_picker::ColorPickerHsv, layer::collapsible::LayerCollapsible};

use super::ContinuousMutation;

pub struct BackgroundControl {
    pub color: [f32; 4],
    pub visible: bool,
}

#[derive(Default)]
pub struct BackgroundControlIntent {
    pub color: ContinuousMutation<[f32; 4]>,
    pub visibility: Option<bool>,
}

impl BackgroundControl {
    pub fn ui(self, ui: &mut Ui) -> BackgroundControlIntent {
        let [r, g, b, a] = self.color;

        let collapsible =
            LayerCollapsible::new(u32::MAX, "Background Color", self.visible).ui(ui, |ui| {
                ui.painter().rect(
                    ui.max_rect(),
                    5,
                    Color32::from(Rgba::from_srgba_premultiplied(
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        255,
                    )),
                    Stroke::NONE,
                    StrokeKind::Middle,
                );
            });
        let visibility = collapsible.visibility_intent;

        let color = collapsible
            .show_body_unindented(ui, |ui| {
                let mut rgb = Rgba::from_rgb(r, g, b);
                let response = ColorPickerHsv::new(&mut rgb).ui(ui);
                let value = response.changed.then(|| {
                    let [r, g, b, _] = rgb.to_array();
                    [r, g, b, a]
                });
                ContinuousMutation {
                    value,
                    pointer_active: response.pointer_active,
                    started: response.drag_started,
                    stopped: response.drag_stopped,
                }
            })
            .map(|response| response.inner)
            .unwrap_or_default();

        BackgroundControlIntent { color, visibility }
    }
}

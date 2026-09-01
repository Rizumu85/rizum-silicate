use egui::*;
use silica_gpu::ProcreateFile;

use crate::gui::widgets::{color_picker::ColorPickerHsv, layer::collapsible::LayerCollapsible};

pub struct BackgroundControl<'a> {
    pub file: &'a mut ProcreateFile,
    pub visible: bool,
}

impl BackgroundControl<'_> {
    pub fn ui(self, ui: &mut Ui) -> Option<bool> {
        let [r, g, b, _] = self.file.background_color;

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
        let visibility_intent = collapsible.visibility_intent;

        collapsible.show_body_unindented(ui, |ui| {
            let mut rgb = Rgba::from_rgb(r, g, b);
            ColorPickerHsv::new(&mut rgb).ui(ui);
            self.file.background_color = rgb.to_array();
        });

        visibility_intent
    }
}

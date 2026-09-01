use egui::*;
use silica_gpu::{BlendingMode, SilicaLayer};

use crate::gui::widgets::{blend_radio::BlendModeRadio, opacity_slider::OpacitySlider};

pub(super) struct LayerControl<'a> {
    pub layer: &'a mut SilicaLayer,
}

#[derive(Default)]
pub(super) struct LayerControlIntent {
    pub blend_mode: Option<BlendingMode>,
    pub clipped: Option<bool>,
}

impl LayerControl<'_> {
    pub fn ui(self, ui: &mut Ui) -> LayerControlIntent {
        let mut blend_mode = None;
        ui.push_id(self.layer.id, |ui| {
            OpacitySlider::new(&mut self.layer.opacity).ui(ui);
            ui.add_space(10.0);
            blend_mode = BlendModeRadio::new(self.layer.blend).ui(ui);
        });

        let mut clipped = self.layer.clipped;
        let mut clipped_intent = None;
        Grid::new(self.layer.id).show(ui, |ui| {
            if ui.toggle_value(&mut clipped, "Clipped").changed() {
                clipped_intent = Some(clipped);
            }
        });
        ui.add_space(10.0);

        LayerControlIntent {
            blend_mode,
            clipped: clipped_intent,
        }
    }
}

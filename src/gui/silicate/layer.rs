use egui::*;
use silica_gpu::BlendingMode;

use crate::gui::widgets::{blend_radio::BlendModeRadio, opacity_slider::OpacitySlider};

use super::ContinuousMutation;

pub(super) struct LayerControl {
    pub id: u32,
    pub opacity: f32,
    pub blend_mode: BlendingMode,
    pub clipped: bool,
}

#[derive(Default)]
pub(super) struct LayerControlIntent {
    pub blend_mode: Option<BlendingMode>,
    pub clipped: Option<bool>,
    pub opacity: ContinuousMutation<f32>,
}

impl LayerControl {
    pub fn ui(self, ui: &mut Ui) -> LayerControlIntent {
        let mut blend_mode = None;
        let mut opacity = self.opacity;
        let mut opacity_intent = ContinuousMutation::default();
        ui.push_id(self.id, |ui| {
            let response = OpacitySlider::new(&mut opacity).ui(ui);
            opacity_intent =
                ContinuousMutation::from_response(response.changed().then_some(opacity), &response);
            ui.add_space(10.0);
            blend_mode = BlendModeRadio::new(self.blend_mode).ui(ui);
        });

        let mut clipped = self.clipped;
        let mut clipped_intent = None;
        Grid::new(self.id).show(ui, |ui| {
            if ui.toggle_value(&mut clipped, "Clipped").changed() {
                clipped_intent = Some(clipped);
            }
        });
        ui.add_space(10.0);

        LayerControlIntent {
            blend_mode,
            clipped: clipped_intent,
            opacity: opacity_intent,
        }
    }
}

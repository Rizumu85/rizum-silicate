use crate::gui::widgets::layer::{collapsible::LayerCollapsible, mask::LayerMask};
use egui::{load::SizedTexture, *};
use silica_gpu::{BlendingMode, Flipped, SilicaHierarchy};
use silicate_runtime::{LayerId, LayerSnapshot};
use std::collections::HashMap;

use super::layer::LayerControl;

pub struct LayersHierarchy<'a> {
    pub rotation: f32,
    pub flipped: Flipped,
    pub previews: &'a HashMap<u32, SizedTexture>,
    pub layers: &'a [SilicaHierarchy],
    pub states: &'a HashMap<LayerId, &'a LayerSnapshot>,
    pub intents: &'a mut Vec<LayerMutationIntent>,
}

pub enum LayerMutationIntent {
    BlendMode {
        layer_id: LayerId,
        blend_mode: BlendingMode,
    },
    Clipped {
        layer_id: LayerId,
        clipped: bool,
    },
    Opacity {
        layer_id: LayerId,
        opacity: f32,
    },
    Visibility {
        layer_id: LayerId,
        visible: bool,
    },
}

impl LayersHierarchy<'_> {
    pub fn ui(self, ui: &mut Ui) {
        for layer in self.layers {
            let mut has_mask = false;

            if let SilicaHierarchy::Layer(layer) = layer
                && let Some(mask_layer) = &layer.mask
            {
                let mask_id = LayerId::from(mask_layer.hierarchy_id());
                let Some(mask_state) = self.states.get(&mask_id) else {
                    log::error!("Runtime state is missing mask {mask_id:?}");
                    continue;
                };
                let item_spacing_y = ui.spacing().item_spacing.y;
                ui.spacing_mut().item_spacing.y = 1.0;

                let id = mask_layer.id;
                let visibility_intent = LayerMask::new(
                    mask_layer
                        .name
                        .to_owned()
                        .unwrap_or_else(|| String::from("Unnamed Mask")),
                    mask_state.visible,
                )
                .ui(ui, |ui| {
                    // ui.painter().rect(
                    //     ui.max_rect(),
                    //     5,
                    //     Color32::WHITE,
                    //     Stroke::NONE,
                    //     StrokeKind::Middle,
                    // );
                    Self::paint_preview(ui, self.flipped, self.previews, self.rotation, id);
                });
                if let Some(visible) = visibility_intent {
                    self.intents.push(LayerMutationIntent::Visibility {
                        layer_id: mask_id,
                        visible,
                    });
                }
                has_mask = true;
                ui.spacing_mut().item_spacing.y = item_spacing_y;
            }

            let (id, hierarchy_id, layer_name, size_change) = match layer {
                SilicaHierarchy::Layer(layer) => {
                    let layer_name = layer
                        .name
                        .to_owned()
                        .unwrap_or_else(|| String::from("Unnamed Layer"));

                    (layer.id, layer.hierarchy_id(), layer_name, false)
                }
                SilicaHierarchy::Group(layer) => {
                    let layer_name = layer
                        .name
                        .to_owned()
                        .unwrap_or_else(|| String::from("Unnamed Group"));

                    (layer.id, layer.hierarchy_id(), layer_name, true)
                }
            };
            let layer_id = LayerId::from(hierarchy_id);
            let Some(state) = self.states.get(&layer_id) else {
                log::error!("Runtime state is missing hierarchy node {layer_id:?}");
                continue;
            };
            let blend_mode = state.blend_mode;

            let collapsible = LayerCollapsible::new(id, layer_name, state.visible)
                .size_change(size_change)
                .has_mask(has_mask)
                .blend_mode(blend_mode)
                .ui(ui, |ui| {
                    Self::paint_preview(ui, self.flipped, self.previews, self.rotation, id);
                });
            if let Some(visible) = collapsible.visibility_intent {
                self.intents
                    .push(LayerMutationIntent::Visibility { layer_id, visible });
            }

            match layer {
                SilicaHierarchy::Layer(layer) => {
                    let (Some(opacity), Some(blend_mode), Some(clipped)) =
                        (state.opacity, state.blend_mode, state.clipped)
                    else {
                        log::error!("Runtime state is missing layer properties for {layer_id:?}");
                        continue;
                    };
                    let control_intent = collapsible
                        .show_body_unindented(ui, |ui| {
                            LayerControl {
                                id: layer.id,
                                opacity,
                                blend_mode,
                                clipped,
                            }
                            .ui(ui)
                        })
                        .map(|response| response.inner);
                    if let Some(intent) = control_intent {
                        if let Some(blend_mode) = intent.blend_mode {
                            self.intents.push(LayerMutationIntent::BlendMode {
                                layer_id,
                                blend_mode,
                            });
                        }
                        if let Some(clipped) = intent.clipped {
                            self.intents
                                .push(LayerMutationIntent::Clipped { layer_id, clipped });
                        }
                        if let Some(opacity) = intent.opacity {
                            self.intents
                                .push(LayerMutationIntent::Opacity { layer_id, opacity });
                        }
                    }
                }
                SilicaHierarchy::Group(layer) => {
                    collapsible.show_body_indented(ui, |ui| {
                        LayersHierarchy {
                            rotation: self.rotation,
                            flipped: self.flipped,
                            previews: self.previews,
                            layers: &layer.children,
                            states: self.states,
                            intents: self.intents,
                        }
                        .ui(ui);
                    });
                }
            };
        }
    }

    fn paint_preview(
        ui: &mut Ui,
        flipped: Flipped,
        previews: &HashMap<u32, SizedTexture>,
        rotation: f32,
        id: u32,
    ) {
        if let Some(tex) = previews.get(&id) {
            let image = Image::from_texture(*tex);

            fn round_to_nearest_quarter_turn(theta: f32) -> f32 {
                let theta = theta.rem_euclid(std::f32::consts::TAU);
                (theta / std::f32::consts::FRAC_PI_2).round() * std::f32::consts::FRAC_PI_2
            }

            fn is_upright(theta: f32) -> bool {
                let deg = theta.rem_euclid(std::f32::consts::TAU).to_degrees();
                !(45.0..135.0).contains(&deg) && !(225.0..315.0).contains(&deg)
            }

            fn make_max_fit_rect(max_rect: Rect, size: Vec2) -> Rect {
                let scale_x = max_rect.width() / size.x;
                let scale_y = max_rect.height() / size.y;
                let size = scale_x.min(scale_y) * size;
                Rect::from_center_size(max_rect.center(), size)
            }

            let rotation = round_to_nearest_quarter_turn(rotation);

            let original_image_size = image.size().expect("wgpu texture have size");
            let mut image_size = original_image_size;
            if is_upright(rotation) {
                std::mem::swap(&mut image_size.x, &mut image_size.y);
            }

            let max_rect_fit = make_max_fit_rect(ui.max_rect(), image_size);
            image_size.x = max_rect_fit.width();
            image_size.y = max_rect_fit.height();

            if !is_upright(rotation) {
                std::mem::swap(&mut image_size.x, &mut image_size.y);
            }

            let image = image.uv(Rect {
                min: pos2(
                    1.0 - if flipped.horizontally { 0.0 } else { 1.0 },
                    1.0 - if flipped.vertically { 0.0 } else { 1.0 },
                ),
                max: pos2(
                    1.0 - if flipped.horizontally { 1.0 } else { 0.0 },
                    1.0 - if flipped.vertically { 1.0 } else { 0.0 },
                ),
            });
            image.rotate(rotation, Vec2::splat(0.5)).paint_at(
                ui,
                Rect::from_center_size(ui.max_rect().center(), image_size),
            );
        }
    }
}

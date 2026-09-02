use egui::{Rect, pos2};

pub mod blend_radio;
pub mod color_picker;
pub mod layer;
pub mod opacity_slider;

fn rail_rect(rect: &Rect) -> Rect {
    const RADIUS: f32 = 1.0;
    Rect::from_min_max(
        pos2(rect.left(), rect.center().y - RADIUS),
        pos2(rect.right(), rect.center().y + RADIUS),
    )
}

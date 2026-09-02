use egui::{EventFilter, Key, Response, Sense, Ui, Widget, epaint, pos2, remap_clamp, vec2};

use crate::gui::theme::Palette;

const HANDLE_RADIUS: f32 = 6.0;

pub struct TimelineSlider<'a> {
    value: &'a mut u64,
    slot_count: u64,
}

impl<'a> TimelineSlider<'a> {
    pub fn new(value: &'a mut u64, slot_count: u64) -> Self {
        Self { value, slot_count }
    }
}

impl Widget for TimelineSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let old_value = *self.value;
        let last_slot = self.slot_count.saturating_sub(1);
        *self.value = (*self.value).min(last_slot);

        let desired_size = vec2(ui.available_width(), ui.spacing().interact_size.y);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
        response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

        if let Some(pointer) = response.interact_pointer_pos() {
            let normalized =
                remap_clamp(pointer.x, rect.x_range().shrink(HANDLE_RADIUS), 0.0..=1.0);
            *self.value = (normalized * last_slot as f32).round() as u64;
        }

        if response.has_focus() {
            ui.ctx().memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    response.id,
                    EventFilter {
                        horizontal_arrows: true,
                        ..Default::default()
                    },
                );
            });
            let (left, right) = ui.input(|input| {
                (
                    input.num_presses(Key::ArrowLeft) as u64,
                    input.num_presses(Key::ArrowRight) as u64,
                )
            });
            *self.value = self
                .value
                .saturating_sub(left)
                .saturating_add(right)
                .min(last_slot);
        }

        if ui.is_rect_visible(rect) {
            let palette = Palette::from_ui(ui);
            let rail = super::rail_rect(&rect);
            ui.painter().rect_filled(rail, 1.0, palette.surface_line);
            let fraction = if last_slot == 0 {
                0.0
            } else {
                *self.value as f32 / last_slot as f32
            };
            let center = pos2(
                egui::lerp(rect.x_range().shrink(HANDLE_RADIUS), fraction),
                rail.center().y,
            );
            ui.painter().circle_filled(
                center,
                if response.hovered() { 13.0 } else { 9.0 },
                palette
                    .caption
                    .gamma_multiply(if response.hovered() { 0.18 } else { 0.10 }),
            );
            ui.painter().add(epaint::CircleShape {
                center,
                radius: HANDLE_RADIUS + ui.style().interact(&response).expansion,
                fill: palette.caption,
                stroke: egui::Stroke::NONE,
            });
        }

        if *self.value != old_value {
            response.mark_changed();
        }
        response
    }
}

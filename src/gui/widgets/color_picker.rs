use egui::{epaint::Hsva, *};

const HANDLE_RADIUS: f32 = 5.0;

pub struct ColorPickerHsv<'a> {
    rgba: &'a mut Rgba,
}

#[derive(Default)]
pub struct ColorPickerResponse {
    pub changed: bool,
    pub pointer_active: bool,
    pub drag_started: bool,
    pub drag_stopped: bool,
}

impl ColorPickerResponse {
    fn include(&mut self, response: &Response) {
        self.pointer_active |= response.is_pointer_button_down_on();
        self.drag_started |= response.drag_started();
        self.drag_stopped |= response.drag_stopped();
    }
}

impl<'a> ColorPickerHsv<'a> {
    /// Number of vertices per dimension in the color sliders.
    /// We need at least 6 for hues, and more for smooth 2D areas.
    /// Should always be a multiple of 6 to hit the peak hues in HSV/HSL (every 60°).
    const N: u32 = 6 * 6;

    pub fn new(rgba: &'a mut Rgba) -> Self {
        Self { rgba }
    }

    /// Shows a color picker where the user can change the given [`Color32`] color.
    ///
    pub fn ui(self, ui: &mut Ui) -> ColorPickerResponse {
        let mut hsva = Hsva::from(*self.rgba);
        let response = Self::color_picker_hsva_2d(ui, &mut hsva);
        *self.rgba = Rgba::from(hsva);
        response
    }

    fn color_picker_hsva_2d(ui: &mut Ui, hsva: &mut Hsva) -> ColorPickerResponse {
        let old_hsva = *hsva;
        let mut response = ui
            .vertical(|ui| Self::color_picker_hsvag_2d(ui, hsva))
            .inner;
        response.changed = old_hsva != *hsva;
        response
    }

    fn color_slider_1d(
        ui: &mut Ui,
        value: &mut f32,
        color_at: impl Fn(f32) -> Color32,
    ) -> Response {
        let desired_size = vec2(ui.available_width(), ui.spacing().interact_size.y);
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

        if let Some(mpos) = response.interact_pointer_pos() {
            *value = remap_clamp(mpos.x, rect.left()..=rect.right(), 0.0..=1.0);
        }

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            // rail
            {
                let rect = super::rail_rect(&rect);
                // fill color:
                let mut mesh = Mesh::default();
                for i in 0..=Self::N {
                    let t = i as f32 / (Self::N as f32);
                    let color = color_at(t);
                    let x = lerp(rect.left()..=rect.right(), t);
                    mesh.colored_vertex(pos2(x, rect.top()), color);
                    mesh.colored_vertex(pos2(x, rect.bottom()), color);
                    if i < Self::N {
                        mesh.add_triangle(2 * i + 0, 2 * i + 1, 2 * i + 2);
                        mesh.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
                    }
                }
                ui.painter().add(Shape::mesh(mesh));
            }

            {
                // Show where the slider is at:
                let x = lerp(rect.left()..=rect.right(), *value);
                let picked_color = color_at(*value);

                let center = pos2(x, rect.center().y);
                ui.painter().add(epaint::CircleShape {
                    center,
                    radius: HANDLE_RADIUS + visuals.expansion,
                    fill: picked_color,
                    stroke: Stroke::NONE,
                });
            }
        }

        response
    }

    /// # Arguments
    /// * `x_value` - X axis, either saturation or value (0.0-1.0).
    /// * `y_value` - Y axis, either saturation or value (0.0-1.0).
    /// * `color_at` - A function that dictates how the mix of saturation and value will be displayed in the 2d slider.
    ///
    /// e.g.: `|x_value, y_value| HsvaGamma { h: 1.0, s: x_value, v: y_value, a: 1.0 }.into()` displays the colors as follows:
    /// * top-left: white `[s: 0.0, v: 1.0]`
    /// * top-right: fully saturated color `[s: 1.0, v: 1.0]`
    /// * bottom-right: black `[s: 0.0, v: 1.0].`
    fn color_slider_2d(
        ui: &mut Ui,
        x_value: &mut f32,
        y_value: &mut f32,
        color_at: impl Fn(f32, f32) -> Color32,
    ) -> Response {
        let desired_size = Vec2::splat(ui.available_width());
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

        if let Some(mpos) = response.interact_pointer_pos() {
            *x_value = remap_clamp(mpos.x, rect.left()..=rect.right(), 0.0..=1.0);
            *y_value = remap_clamp(mpos.y, rect.bottom()..=rect.top(), 0.0..=1.0);
        }

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let mut mesh = Mesh::default();

            for xi in 0..=Self::N {
                for yi in 0..=Self::N {
                    let xt = xi as f32 / (Self::N as f32);
                    let yt = yi as f32 / (Self::N as f32);
                    let color = color_at(xt, yt);
                    let x = lerp(rect.left()..=rect.right(), xt);
                    let y = lerp(rect.bottom()..=rect.top(), yt);
                    mesh.colored_vertex(pos2(x, y), color);

                    if xi < Self::N && yi < Self::N {
                        let x_offset = 1;
                        let y_offset = Self::N + 1;
                        let tl = yi * y_offset + xi;
                        mesh.add_triangle(tl, tl + x_offset, tl + y_offset);
                        mesh.add_triangle(tl + x_offset, tl + y_offset, tl + y_offset + x_offset);
                    }
                }
            }
            ui.painter().add(Shape::mesh(mesh)); // fill

            // Show where the slider is at:
            let x = lerp(rect.left()..=rect.right(), *x_value);
            let y = lerp(rect.bottom()..=rect.top(), *y_value);
            let picked_color = color_at(*x_value, *y_value);
            ui.painter().add(epaint::CircleShape {
                center: pos2(x, y),
                radius: 10.0,
                fill: picked_color,
                stroke: Stroke::new(visuals.fg_stroke.width, Color32::WHITE),
            });
        }

        response
    }

    fn color_picker_hsvag_2d(ui: &mut Ui, hsva: &mut Hsva) -> ColorPickerResponse {
        let opaque = Hsva { a: 1.0, ..*hsva };

        let Hsva { h, s, v, a: _ } = hsva;

        let mut response = ColorPickerResponse::default();
        let saturation_value =
            Self::color_slider_2d(ui, s, v, |s, v| Hsva { s, v, ..opaque }.into());
        response.include(&saturation_value);

        let hue = Self::color_slider_1d(ui, h, |h| {
            Hsva {
                h,
                s: 1.0,
                v: 1.0,
                a: 1.0,
            }
            .into()
        })
        .on_hover_text("Hue");
        response.include(&hue);

        let saturation = Self::color_slider_1d(ui, s, |s| {
            Hsva {
                s,
                v: remap(s, 0.0..=1.0, 0.5..=1.0),
                ..opaque
            }
            .into()
        })
        .on_hover_text("Saturation");
        response.include(&saturation);

        let value = Self::color_slider_1d(ui, v, |v| {
            Hsva {
                v,
                s: 0.0,
                ..opaque
            }
            .into()
        })
        .on_hover_text("Value");
        response.include(&value);

        hsva.a = 1.0;
        response
    }
}

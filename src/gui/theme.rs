use std::sync::Arc;

use egui::{
    Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin,
    Shadow, Stroke, TextStyle, Theme, Visuals, vec2,
};
use lucide_icons::Icon;

const ICON_FONT: &str = "Rizum Lucide";

pub const ACCENT_TEAL: Color32 = Color32::from_rgb(45, 212, 191);

#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: Color32,
    pub surface: Color32,
    pub surface_muted: Color32,
    pub surface_line: Color32,
    pub ink: Color32,
    pub ink_soft: Color32,
    pub ink_muted: Color32,
    pub caption: Color32,
    pub glass: Color32,
    pub glass_border: Color32,
    pub shadow: Color32,
}

impl Palette {
    pub fn from_ui(ui: &egui::Ui) -> Self {
        if ui.visuals().dark_mode {
            Self::dark()
        } else {
            Self::light()
        }
    }

    fn light() -> Self {
        Self {
            canvas: Color32::from_rgb(240, 240, 240),
            surface: Color32::WHITE,
            surface_muted: Color32::from_rgb(244, 244, 245),
            surface_line: Color32::from_rgb(228, 228, 231),
            ink: Color32::from_rgb(24, 24, 27),
            ink_soft: Color32::from_rgb(63, 63, 70),
            ink_muted: Color32::from_rgb(113, 113, 122),
            caption: Color32::from_rgb(161, 161, 170),
            glass: Color32::from_rgba_unmultiplied(255, 255, 255, 224),
            glass_border: Color32::from_rgba_unmultiplied(255, 255, 255, 190),
            shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 18),
        }
    }

    fn dark() -> Self {
        Self {
            canvas: Color32::from_rgb(17, 17, 19),
            surface: Color32::from_rgb(39, 39, 42),
            surface_muted: Color32::from_rgb(63, 63, 70),
            surface_line: Color32::from_rgb(82, 82, 91),
            ink: Color32::from_rgb(244, 244, 245),
            ink_soft: Color32::from_rgb(212, 212, 216),
            ink_muted: Color32::from_rgb(161, 161, 170),
            caption: Color32::from_rgb(152, 152, 162),
            glass: Color32::from_rgba_unmultiplied(39, 39, 42, 236),
            glass_border: Color32::from_rgba_unmultiplied(255, 255, 255, 24),
            shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 110),
        }
    }
}

pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        ICON_FONT.to_owned(),
        Arc::new(FontData::from_static(lucide_icons::LUCIDE_FONT_BYTES)),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(ICON_FONT.to_owned());
    fonts.families.insert(
        FontFamily::Name(ICON_FONT.into()),
        vec![ICON_FONT.to_owned()],
    );
    ctx.set_fonts(fonts);

    install_style(ctx, Theme::Light, Palette::light());
    install_style(ctx, Theme::Dark, Palette::dark());
}

fn install_style(ctx: &Context, theme: Theme, palette: Palette) {
    let mut style = (*ctx.style_of(theme)).clone();
    style.spacing.item_spacing = vec2(8.0, 7.0);
    style.spacing.button_padding = vec2(10.0, 6.0);
    style.spacing.interact_size.y = 28.0;
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(19.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(12.0, FontFamily::Proportional),
    );
    style.visuals = rizum_visuals(theme, palette);
    ctx.set_style_of(theme, style);
}

fn rizum_visuals(theme: Theme, palette: Palette) -> Visuals {
    let mut visuals = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };

    visuals.override_text_color = Some(palette.ink_soft);
    visuals.panel_fill = palette.canvas;
    visuals.window_fill = palette.glass;
    visuals.window_stroke = Stroke::new(1.0, palette.glass_border);
    visuals.window_corner_radius = CornerRadius::same(16);
    visuals.window_shadow = Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: palette.shadow,
    };
    visuals.popup_shadow = visuals.window_shadow;
    visuals.menu_corner_radius = CornerRadius::same(12);
    visuals.extreme_bg_color = palette.surface_muted;
    visuals.code_bg_color = palette.surface_muted;
    visuals.faint_bg_color = palette.surface_muted.gamma_multiply(0.55);
    visuals.selection.bg_fill = palette.surface_muted;
    visuals.selection.stroke = Stroke::new(1.0, palette.surface_line);
    visuals.hyperlink_color = ACCENT_TEAL;
    visuals.slider_trailing_fill = false;
    visuals.indent_has_left_vline = false;

    let radius = CornerRadius::same(8);
    visuals.widgets.noninteractive.bg_fill = palette.surface;
    visuals.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.surface_line);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.ink_soft);
    visuals.widgets.noninteractive.corner_radius = radius;

    visuals.widgets.inactive.bg_fill = palette.surface;
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.surface_line);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.ink_muted);
    visuals.widgets.inactive.corner_radius = radius;

    visuals.widgets.hovered.bg_fill = palette.surface;
    visuals.widgets.hovered.weak_bg_fill = palette.surface;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.caption);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.2, palette.ink);
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.hovered.expansion = 1.0;

    visuals.widgets.active.bg_fill = palette.surface_muted;
    visuals.widgets.active.weak_bg_fill = palette.surface_muted;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.surface_line);
    visuals.widgets.active.fg_stroke = Stroke::new(1.2, palette.ink);
    visuals.widgets.active.corner_radius = radius;

    visuals.widgets.open = visuals.widgets.active;
    visuals
}

pub fn glass_frame(ui: &egui::Ui, compact: bool) -> Frame {
    let palette = Palette::from_ui(ui);
    Frame::new()
        .fill(palette.glass)
        .stroke(Stroke::new(1.0, palette.glass_border))
        .corner_radius(if compact { 16 } else { 20 })
        .inner_margin(if compact {
            Margin::same(12)
        } else {
            Margin::same(16)
        })
        .shadow(Shadow {
            offset: [0, 8],
            blur: 28,
            spread: 0,
            color: palette.shadow,
        })
}

pub fn icon(icon: Icon) -> char {
    char::from(icon)
}

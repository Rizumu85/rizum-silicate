use egui::*;
use silica_gpu::BlendingMode;

pub struct BlendModeRadio {
    value: BlendingMode,
}

#[derive(Clone, Copy, Debug)]
pub struct BlendModeRadioLoaded;

impl BlendModeRadioLoaded {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_temp(id))
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}

impl BlendModeRadio {
    pub fn new(value: BlendingMode) -> Self {
        Self { value }
    }

    fn layout_scroll_area(&self, ui: &mut Ui) -> Option<BlendingMode> {
        let min_y = ui.min_rect().min.y;
        let mut scroll_to_value = 0.0;
        let mut blend_mode_intent = None;
        let mut scroll = ScrollArea::vertical().max_height(70.0).show(ui, |ui| {
            ui.set_width(ui.available_width());

            for b in BlendingMode::all() {
                let selected = *b == self.value;

                let mut frame = egui::Frame::NONE
                    .inner_margin(Margin::symmetric(10, 3))
                    .begin(ui);
                {
                    let ui = &mut frame.content_ui;
                    ui.set_width(ui.available_width());
                    Label::new(RichText::new(b.as_str()).color(if selected {
                        Color32::WHITE
                    } else {
                        ui.visuals().weak_text_color()
                    }))
                    .selectable(false)
                    .ui(ui);
                }
                let response = ui.allocate_rect(frame.content_ui.min_rect(), Sense::click());

                if selected {
                    scroll_to_value = response.rect.min.y - min_y;
                    frame.frame.fill = super::ACCENT_COLOR;
                } else if response.hovered() {
                    frame.frame.fill = ui.visuals().widgets.hovered.bg_fill;
                }

                if response.clicked() && !selected {
                    blend_mode_intent = Some(*b);
                }
                frame.end(ui);
            }
        });

        let loaded = BlendModeRadioLoaded::load(ui.ctx(), ui.id()).is_some();
        if !loaded {
            scroll.state.offset = vec2(0.0, scroll_to_value);
            scroll.state.store(ui.ctx(), scroll.id);
        }
        BlendModeRadioLoaded.store(ui.ctx(), ui.id());
        blend_mode_intent
    }

    pub fn ui(self, ui: &mut Ui) -> Option<BlendingMode> {
        egui::Frame::default()
            .inner_margin(Margin::symmetric(0, 5))
            .corner_radius(4)
            .fill(ui.visuals().widgets.inactive.bg_fill)
            .show(ui, |ui| self.layout_scroll_area(ui))
            .inner
    }
}

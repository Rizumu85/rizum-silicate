pub mod background;
pub mod hierarchy;
mod layer;

pub(crate) struct ContinuousMutation<T> {
    pub value: Option<T>,
    pub pointer_active: bool,
    pub started: bool,
    pub stopped: bool,
}

impl<T> ContinuousMutation<T> {
    pub(crate) fn from_response(value: Option<T>, response: &egui::Response) -> Self {
        Self {
            value,
            pointer_active: response.is_pointer_button_down_on(),
            started: response.drag_started(),
            stopped: response.drag_stopped(),
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        if other.value.is_some() {
            self.value = other.value;
        }
        self.pointer_active |= other.pointer_active;
        self.started |= other.started;
        self.stopped |= other.stopped;
    }
}

impl<T> Default for ContinuousMutation<T> {
    fn default() -> Self {
        Self {
            value: None,
            pointer_active: false,
            started: false,
            stopped: false,
        }
    }
}

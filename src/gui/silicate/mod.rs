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
    fn from_response(value: Option<T>, response: &egui::Response) -> Self {
        Self {
            value,
            pointer_active: response.is_pointer_button_down_on(),
            started: response.drag_started(),
            stopped: response.drag_stopped(),
        }
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

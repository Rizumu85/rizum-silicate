use crate::platform::windows::status::{SummaryState, WindowsIntegrationSummary};
use egui::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsIntegrationSnapshot {
    Ready(WindowsIntegrationSummary),
    DetectionFailed(String),
    #[cfg(not(windows))]
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsState {
    pub windows_integration: SettingsIntegrationSnapshot,
}

impl SettingsState {
    pub fn detect_current() -> Self {
        Self {
            windows_integration: detect_current_windows_integration(),
        }
    }

    fn refresh_current(&mut self) {
        self.windows_integration = detect_current_windows_integration();
    }

    #[cfg(windows)]
    fn install_or_repair_current(&mut self) {
        match crate::platform::windows::registration::install_or_repair_current_windows_integration()
        {
            Ok(()) => self.refresh_current(),
            Err(err) => {
                self.windows_integration = SettingsIntegrationSnapshot::DetectionFailed(format!(
                    "Could not install or repair Windows integration: {err:?}"
                ));
            }
        }
    }
}

pub struct SettingsGui<'a> {
    state: &'a mut SettingsState,
}

impl<'a> SettingsGui<'a> {
    pub fn new(state: &'a mut SettingsState) -> Self {
        Self { state }
    }

    pub fn ui(self, ui: &mut Ui) {
        ui.label(
            RichText::new("System Integration")
                .small()
                .strong()
                .color(ui.visuals().strong_text_color()),
        );
        ui.add_space(6.0);

        if ui.button("Refresh").clicked() {
            self.state.refresh_current();
        }
        #[cfg(windows)]
        if ui.button("Install / Repair").clicked() {
            self.state.install_or_repair_current();
        }

        ui.add_space(6.0);

        match &self.state.windows_integration {
            SettingsIntegrationSnapshot::Ready(summary) => {
                for row in &summary.rows {
                    ui.horizontal(|ui| {
                        ui.label(row.label);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(summary_state_label(row.state))
                                    .small()
                                    .color(summary_state_color(ui, row.state)),
                            );
                        });
                    });
                    ui.label(RichText::new(&row.detail).small().weak());
                    ui.add_space(6.0);
                }
            }
            SettingsIntegrationSnapshot::DetectionFailed(message) => {
                ui.label(
                    RichText::new(message)
                        .small()
                        .color(ui.visuals().warn_fg_color),
                );
            }
            #[cfg(not(windows))]
            SettingsIntegrationSnapshot::Unsupported => {
                ui.label(RichText::new("Platform integration is not available here.").small());
            }
        }
    }
}

fn summary_state_label(state: SummaryState) -> &'static str {
    match state {
        SummaryState::Installed => "Installed",
        SummaryState::Missing => "Missing",
        SummaryState::NeedsRepair => "Needs Repair",
    }
}

fn summary_state_color(ui: &Ui, state: SummaryState) -> Color32 {
    match state {
        SummaryState::Installed => Color32::from_rgb(45, 180, 150),
        SummaryState::Missing => ui.visuals().warn_fg_color,
        SummaryState::NeedsRepair => Color32::from_rgb(245, 158, 11),
    }
}

#[cfg(windows)]
fn detect_current_windows_integration() -> SettingsIntegrationSnapshot {
    match crate::platform::windows::status::detect_current_windows_integration_summary() {
        Ok(summary) => SettingsIntegrationSnapshot::Ready(summary),
        Err(err) => SettingsIntegrationSnapshot::DetectionFailed(format!(
            "Could not read Windows integration status: {err:?}"
        )),
    }
}

#[cfg(not(windows))]
fn detect_current_windows_integration() -> SettingsIntegrationSnapshot {
    SettingsIntegrationSnapshot::Unsupported
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_summary_states_for_settings_rows() {
        assert_eq!(summary_state_label(SummaryState::Installed), "Installed");
        assert_eq!(summary_state_label(SummaryState::Missing), "Missing");
        assert_eq!(
            summary_state_label(SummaryState::NeedsRepair),
            "Needs Repair"
        );
    }
}

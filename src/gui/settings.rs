use crate::export::ffmpeg::{FfmpegToolSource, FfmpegToolStatus};
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
    pub video_tools: FfmpegToolStatus,
}

impl SettingsState {
    pub fn detect_current() -> Self {
        Self {
            windows_integration: detect_current_windows_integration(),
            video_tools: detect_current_video_tools(),
        }
    }

    fn refresh_current(&mut self) {
        self.windows_integration = detect_current_windows_integration();
        self.video_tools = detect_current_video_tools();
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

    #[cfg(windows)]
    fn uninstall_current(&mut self) {
        match crate::platform::windows::registration::uninstall_current_windows_integration() {
            Ok(()) => self.refresh_current(),
            Err(err) => {
                self.windows_integration = SettingsIntegrationSnapshot::DetectionFailed(format!(
                    "Could not uninstall Windows integration: {err:?}"
                ));
            }
        }
    }

    #[cfg(windows)]
    fn choose_default_app(&mut self) {
        if let Err(err) = crate::platform::windows::explorer::open_current_default_apps_settings() {
            self.windows_integration = SettingsIntegrationSnapshot::DetectionFailed(format!(
                "Could not open Windows Default Apps settings: {err:?}"
            ));
        }
    }

    #[cfg(windows)]
    fn restart_explorer(&mut self) {
        match crate::platform::windows::explorer::restart_current_explorer() {
            Ok(()) => self.refresh_current(),
            Err(err) => {
                self.windows_integration = SettingsIntegrationSnapshot::DetectionFailed(format!(
                    "Could not restart Explorer: {err:?}"
                ));
            }
        }
    }

    #[cfg(windows)]
    fn refresh_thumbnail_cache(&mut self) {
        match crate::platform::windows::thumbnail_cache::refresh_current_thumbnail_cache() {
            Ok(()) => self.refresh_current(),
            Err(err) => {
                self.windows_integration = SettingsIntegrationSnapshot::DetectionFailed(format!(
                    "Could not refresh Explorer thumbnail cache: {err:?}"
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
        #[cfg(windows)]
        if ui.button("Choose Default App").clicked() {
            self.state.choose_default_app();
        }
        #[cfg(windows)]
        if ui.button("Uninstall").clicked() {
            self.state.uninstall_current();
        }
        #[cfg(windows)]
        if ui.button("Restart Explorer").clicked() {
            self.state.restart_explorer();
        }
        #[cfg(windows)]
        if ui.button("Refresh Thumbnail Cache").clicked() {
            self.state.refresh_thumbnail_cache();
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

        ui.add_space(6.0);
        ui.label(
            RichText::new("Video Tools")
                .small()
                .strong()
                .color(ui.visuals().strong_text_color()),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Video Tools");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(ffmpeg_source_label(self.state.video_tools.source))
                        .small()
                        .color(ffmpeg_source_color(ui, self.state.video_tools.source)),
                );
            });
        });
        ui.label(RichText::new(&self.state.video_tools.detail).small().weak());
    }
}

fn summary_state_label(state: SummaryState) -> &'static str {
    match state {
        SummaryState::Installed => "Installed",
        SummaryState::Missing => "Missing",
        SummaryState::NeedsRepair => "Needs Repair",
        SummaryState::NotSelected => "Not Selected",
    }
}

fn summary_state_color(ui: &Ui, state: SummaryState) -> Color32 {
    match state {
        SummaryState::Installed => Color32::from_rgb(45, 180, 150),
        SummaryState::Missing => ui.visuals().warn_fg_color,
        SummaryState::NeedsRepair => Color32::from_rgb(245, 158, 11),
        SummaryState::NotSelected => ui.visuals().weak_text_color(),
    }
}

fn ffmpeg_source_label(source: FfmpegToolSource) -> &'static str {
    match source {
        FfmpegToolSource::Bundled => "Bundled",
        FfmpegToolSource::System => "System",
        FfmpegToolSource::Missing => "Missing",
    }
}

fn ffmpeg_source_color(ui: &Ui, source: FfmpegToolSource) -> Color32 {
    match source {
        FfmpegToolSource::Bundled => Color32::from_rgb(45, 180, 150),
        FfmpegToolSource::System => Color32::from_rgb(103, 142, 249),
        FfmpegToolSource::Missing => ui.visuals().warn_fg_color,
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

fn detect_current_video_tools() -> FfmpegToolStatus {
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::export::ffmpeg::detect_current_ffmpeg_tool_status().unwrap_or_else(|err| {
            FfmpegToolStatus {
                source: FfmpegToolSource::Missing,
                executable_path: None,
                detail: format!("Could not inspect ffmpeg tools: {err}"),
            }
        })
    }

    #[cfg(target_arch = "wasm32")]
    {
        FfmpegToolStatus {
            source: FfmpegToolSource::Missing,
            executable_path: None,
            detail: "Video export tools are not available in web builds".to_owned(),
        }
    }
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
        assert_eq!(
            summary_state_label(SummaryState::NotSelected),
            "Not Selected"
        );
    }

    #[test]
    fn labels_ffmpeg_tool_sources_for_settings_rows() {
        assert_eq!(ffmpeg_source_label(FfmpegToolSource::Bundled), "Bundled");
        assert_eq!(ffmpeg_source_label(FfmpegToolSource::System), "System");
        assert_eq!(ffmpeg_source_label(FfmpegToolSource::Missing), "Missing");
    }
}

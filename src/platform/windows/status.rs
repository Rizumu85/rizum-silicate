use super::association::{FileAssociationStatus, IntegrationState};
use super::thumbnails::{ThumbnailIntegrationState, ThumbnailRegistrationStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsIntegrationSummary {
    pub rows: Vec<IntegrationStatusRow>,
}

impl WindowsIntegrationSummary {
    pub fn from_statuses(
        file_association: &FileAssociationStatus,
        thumbnails: &ThumbnailRegistrationStatus,
    ) -> Self {
        Self {
            rows: vec![
                IntegrationStatusRow {
                    kind: IntegrationStatusKind::FileAssociation,
                    label: "File Association",
                    state: SummaryState::from_file_association(file_association.state),
                    detail: issue_detail(file_association.issues.len()),
                },
                IntegrationStatusRow {
                    kind: IntegrationStatusKind::ExplorerThumbnails,
                    label: "Explorer Thumbnails",
                    state: SummaryState::from_thumbnail_registration(thumbnails.state),
                    detail: issue_detail(thumbnails.issues.len()),
                },
            ],
        }
    }

    pub fn all_installed(&self) -> bool {
        self.rows
            .iter()
            .all(|row| row.state == SummaryState::Installed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationStatusRow {
    pub kind: IntegrationStatusKind,
    pub label: &'static str,
    pub state: SummaryState,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationStatusKind {
    FileAssociation,
    ExplorerThumbnails,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryState {
    Installed,
    Missing,
    NeedsRepair,
}

impl SummaryState {
    fn from_file_association(state: IntegrationState) -> Self {
        match state {
            IntegrationState::Installed => Self::Installed,
            IntegrationState::Missing => Self::Missing,
            IntegrationState::Incomplete => Self::NeedsRepair,
        }
    }

    fn from_thumbnail_registration(state: ThumbnailIntegrationState) -> Self {
        match state {
            ThumbnailIntegrationState::Installed => Self::Installed,
            ThumbnailIntegrationState::Missing => Self::Missing,
            ThumbnailIntegrationState::Incomplete => Self::NeedsRepair,
        }
    }
}

fn issue_detail(issue_count: usize) -> String {
    match issue_count {
        0 => "Ready".to_owned(),
        1 => "1 issue found".to_owned(),
        count => format!("{count} issues found"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::windows::association::FileAssociationIssue;
    use crate::platform::windows::thumbnails::ThumbnailRegistrationIssue;

    #[test]
    fn builds_settings_rows_for_installed_integrations() {
        let file_association = FileAssociationStatus {
            state: IntegrationState::Installed,
            issues: Vec::new(),
        };
        let thumbnails = ThumbnailRegistrationStatus {
            state: ThumbnailIntegrationState::Installed,
            issues: Vec::new(),
        };

        let summary = WindowsIntegrationSummary::from_statuses(&file_association, &thumbnails);

        assert!(summary.all_installed());
        assert_eq!(
            summary.rows,
            vec![
                IntegrationStatusRow {
                    kind: IntegrationStatusKind::FileAssociation,
                    label: "File Association",
                    state: SummaryState::Installed,
                    detail: "Ready".to_owned(),
                },
                IntegrationStatusRow {
                    kind: IntegrationStatusKind::ExplorerThumbnails,
                    label: "Explorer Thumbnails",
                    state: SummaryState::Installed,
                    detail: "Ready".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn maps_missing_and_incomplete_states_for_settings_rows() {
        let file_association = FileAssociationStatus {
            state: IntegrationState::Missing,
            issues: vec![FileAssociationIssue::MissingProgId],
        };
        let thumbnails = ThumbnailRegistrationStatus {
            state: ThumbnailIntegrationState::Incomplete,
            issues: vec![
                ThumbnailRegistrationIssue::WrongShellExtensionClsid,
                ThumbnailRegistrationIssue::MissingProviderDllFile,
            ],
        };

        let summary = WindowsIntegrationSummary::from_statuses(&file_association, &thumbnails);

        assert!(!summary.all_installed());
        assert_eq!(summary.rows[0].state, SummaryState::Missing);
        assert_eq!(summary.rows[0].detail, "1 issue found");
        assert_eq!(summary.rows[1].state, SummaryState::NeedsRepair);
        assert_eq!(summary.rows[1].detail, "2 issues found");
    }
}

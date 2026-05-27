use super::association::{
    evaluate_file_association, read_file_association_snapshot, ExpectedFileAssociation,
    FileAssociationStatus, IntegrationState,
};
use super::registry::{RegistryReadError, RegistryValueReader};
use super::thumbnails::{
    evaluate_thumbnail_registration, read_thumbnail_registration_snapshot,
    ExpectedThumbnailProvider, FilePresenceReader, ThumbnailIntegrationState,
    ThumbnailRegistrationStatus,
};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedWindowsIntegration {
    pub file_association: ExpectedFileAssociation,
    pub thumbnails: ExpectedThumbnailProvider,
}

impl ExpectedWindowsIntegration {
    pub fn new(
        app_executable_path: impl Into<PathBuf>,
        thumbnail_dll_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            file_association: ExpectedFileAssociation::for_executable(app_executable_path.into()),
            thumbnails: ExpectedThumbnailProvider::new(thumbnail_dll_path),
        }
    }
}

pub fn detect_windows_integration_summary(
    registry: &impl RegistryValueReader,
    files: &impl FilePresenceReader,
    expected: &ExpectedWindowsIntegration,
) -> Result<WindowsIntegrationSummary, RegistryReadError> {
    let file_association_snapshot = read_file_association_snapshot(registry)?;
    let file_association =
        evaluate_file_association(&file_association_snapshot, &expected.file_association);

    let thumbnails_snapshot =
        read_thumbnail_registration_snapshot(registry, files, &expected.thumbnails)?;
    let thumbnails = evaluate_thumbnail_registration(&thumbnails_snapshot, &expected.thumbnails);

    Ok(WindowsIntegrationSummary::from_statuses(
        &file_association,
        &thumbnails,
    ))
}

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
    use crate::platform::windows::association::{
        FileAssociationIssue, CONTENT_TYPE, PERCEIVED_TYPE, PROG_ID,
    };
    use crate::platform::windows::registry::RegistryValueName;
    use crate::platform::windows::thumbnails::{
        ThumbnailRegistrationIssue, THUMBNAIL_HANDLER_SHELLEX_GUID,
    };
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::path::Path;

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

    #[test]
    fn detects_summary_from_registry_and_file_presence_readers() {
        let expected = ExpectedWindowsIntegration::new(
            r"C:\Silicate\silicate.exe",
            r"C:\Silicate\rizum_silicate_thumb.dll",
        );
        let registry = FakeRegistryReader::new([
            ((r"Software\Classes\.procreate", None), PROG_ID.to_owned()),
            (
                (r"Software\Classes\.procreate", Some("Content Type")),
                CONTENT_TYPE.to_owned(),
            ),
            (
                (r"Software\Classes\.procreate", Some("PerceivedType")),
                PERCEIVED_TYPE.to_owned(),
            ),
            (
                (
                    r"Software\Classes\RizumSilicate.procreate\shell\open\command",
                    None,
                ),
                r#""C:\Silicate\silicate.exe" "%1""#.to_owned(),
            ),
            (
                (
                    r"Software\Classes\RizumSilicate.procreate\DefaultIcon",
                    None,
                ),
                r"C:\Silicate\silicate.exe,0".to_owned(),
            ),
            (
                (
                    r"Software\Classes\.procreate\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}",
                    None,
                ),
                expected.thumbnails.clsid.clone(),
            ),
            (
                (
                    r"Software\Classes\CLSID\{6F52A378-4E3D-4FE3-A49F-3E4D9CF03AF1}\InprocServer32",
                    None,
                ),
                expected.thumbnails.dll_path.to_string_lossy().into_owned(),
            ),
        ]);
        let files = FakeFilePresenceReader::new([expected.thumbnails.dll_path.clone()]);

        let summary = detect_windows_integration_summary(&registry, &files, &expected).unwrap();

        assert!(summary.all_installed());
        assert_eq!(
            summary.rows.iter().map(|row| row.state).collect::<Vec<_>>(),
            vec![SummaryState::Installed, SummaryState::Installed]
        );
        assert_eq!(
            registry.reads(),
            vec![
                (r"Software\Classes\.procreate".to_owned(), None),
                (
                    r"Software\Classes\.procreate".to_owned(),
                    Some("Content Type".to_owned())
                ),
                (
                    r"Software\Classes\.procreate".to_owned(),
                    Some("PerceivedType".to_owned())
                ),
                (
                    r"Software\Classes\RizumSilicate.procreate\shell\open\command".to_owned(),
                    None
                ),
                (
                    r"Software\Classes\RizumSilicate.procreate\DefaultIcon".to_owned(),
                    None
                ),
                (
                    format!(
                        r"Software\Classes\.procreate\ShellEx\{}",
                        THUMBNAIL_HANDLER_SHELLEX_GUID
                    ),
                    None
                ),
                (
                    r"Software\Classes\CLSID\{6F52A378-4E3D-4FE3-A49F-3E4D9CF03AF1}\InprocServer32"
                        .to_owned(),
                    None
                ),
            ]
        );
    }

    struct FakeRegistryReader {
        values: HashMap<(String, Option<String>), String>,
        reads: RefCell<Vec<(String, Option<String>)>>,
    }

    impl FakeRegistryReader {
        fn new<const N: usize>(values: [((&str, Option<&str>), String); N]) -> Self {
            Self {
                values: values
                    .into_iter()
                    .map(|((subkey, value_name), value)| {
                        ((subkey.to_owned(), value_name.map(str::to_owned)), value)
                    })
                    .collect(),
                reads: RefCell::new(Vec::new()),
            }
        }

        fn reads(&self) -> Vec<(String, Option<String>)> {
            self.reads.borrow().clone()
        }
    }

    impl RegistryValueReader for FakeRegistryReader {
        fn read_hkcu_string(
            &self,
            subkey: &str,
            value_name: RegistryValueName<'_>,
        ) -> Result<Option<String>, RegistryReadError> {
            let key = (subkey.to_owned(), value_name.to_option_string());
            self.reads.borrow_mut().push(key.clone());
            Ok(self.values.get(&key).cloned())
        }
    }

    struct FakeFilePresenceReader {
        existing_paths: HashSet<PathBuf>,
    }

    impl FakeFilePresenceReader {
        fn new<const N: usize>(paths: [PathBuf; N]) -> Self {
            Self {
                existing_paths: paths.into_iter().collect(),
            }
        }
    }

    impl FilePresenceReader for FakeFilePresenceReader {
        fn path_exists(&self, path: &Path) -> bool {
            self.existing_paths.contains(path)
        }
    }
}

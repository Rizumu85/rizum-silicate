use super::registry::{
    hkcu_classes_subkey, RegistryReadError, RegistryValueName, RegistryValueReader,
};
use std::path::Path;

pub const PROCREATE_EXTENSION: &str = ".procreate";
pub const PROG_ID: &str = "RizumSilicate.procreate";
pub const CONTENT_TYPE: &str = "application/x-procreate";
pub const PERCEIVED_TYPE: &str = "image";
const CONTENT_TYPE_VALUE: &str = "Content Type";
const PERCEIVED_TYPE_VALUE: &str = "PerceivedType";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAssociationSnapshot {
    pub extension_prog_id: Option<String>,
    pub content_type: Option<String>,
    pub perceived_type: Option<String>,
    pub open_command: Option<String>,
    pub default_icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedFileAssociation {
    pub executable_path: String,
}

impl ExpectedFileAssociation {
    pub fn for_executable(path: impl AsRef<Path>) -> Self {
        Self {
            executable_path: path.as_ref().to_string_lossy().into_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAssociationStatus {
    pub state: IntegrationState,
    pub issues: Vec<FileAssociationIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationState {
    Installed,
    Missing,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAssociationIssue {
    MissingProgId,
    WrongProgId,
    MissingContentType,
    WrongContentType,
    MissingPerceivedType,
    WrongPerceivedType,
    MissingOpenCommand,
    WrongOpenCommand,
    MissingDefaultIcon,
    WrongDefaultIcon,
}

pub fn read_file_association_snapshot(
    reader: &impl RegistryValueReader,
) -> Result<FileAssociationSnapshot, RegistryReadError> {
    let extension_key = hkcu_classes_subkey(PROCREATE_EXTENSION);
    let prog_id_key = hkcu_classes_subkey(PROG_ID);

    Ok(FileAssociationSnapshot {
        extension_prog_id: reader.read_hkcu_string(&extension_key, RegistryValueName::Default)?,
        content_type: reader
            .read_hkcu_string(&extension_key, RegistryValueName::Named(CONTENT_TYPE_VALUE))?,
        perceived_type: reader.read_hkcu_string(
            &extension_key,
            RegistryValueName::Named(PERCEIVED_TYPE_VALUE),
        )?,
        open_command: reader.read_hkcu_string(
            &format!(r"{prog_id_key}\shell\open\command"),
            RegistryValueName::Default,
        )?,
        default_icon: reader.read_hkcu_string(
            &format!(r"{prog_id_key}\DefaultIcon"),
            RegistryValueName::Default,
        )?,
    })
}

pub fn evaluate_file_association(
    snapshot: &FileAssociationSnapshot,
    expected: &ExpectedFileAssociation,
) -> FileAssociationStatus {
    let mut issues = Vec::new();

    match snapshot.extension_prog_id.as_deref() {
        None => {
            return FileAssociationStatus {
                state: IntegrationState::Missing,
                issues: vec![FileAssociationIssue::MissingProgId],
            };
        }
        Some(PROG_ID) => {}
        Some(_) => issues.push(FileAssociationIssue::WrongProgId),
    }

    push_value_issue(
        snapshot.content_type.as_deref(),
        CONTENT_TYPE,
        FileAssociationIssue::MissingContentType,
        FileAssociationIssue::WrongContentType,
        &mut issues,
    );
    push_value_issue(
        snapshot.perceived_type.as_deref(),
        PERCEIVED_TYPE,
        FileAssociationIssue::MissingPerceivedType,
        FileAssociationIssue::WrongPerceivedType,
        &mut issues,
    );

    match snapshot.open_command.as_deref() {
        None => issues.push(FileAssociationIssue::MissingOpenCommand),
        Some(command) if open_command_matches(command, expected) => {}
        Some(_) => issues.push(FileAssociationIssue::WrongOpenCommand),
    }

    match snapshot.default_icon.as_deref() {
        None => issues.push(FileAssociationIssue::MissingDefaultIcon),
        Some(icon) if default_icon_matches(icon, expected) => {}
        Some(_) => issues.push(FileAssociationIssue::WrongDefaultIcon),
    }

    let state = if issues.is_empty() {
        IntegrationState::Installed
    } else {
        IntegrationState::Incomplete
    };

    FileAssociationStatus { state, issues }
}

fn push_value_issue(
    actual: Option<&str>,
    expected: &str,
    missing: FileAssociationIssue,
    wrong: FileAssociationIssue,
    issues: &mut Vec<FileAssociationIssue>,
) {
    match actual {
        None => issues.push(missing),
        Some(value) if value.eq_ignore_ascii_case(expected) => {}
        Some(_) => issues.push(wrong),
    }
}

fn open_command_matches(command: &str, expected: &ExpectedFileAssociation) -> bool {
    let command = command.to_ascii_lowercase();
    let executable_path = expected.executable_path.to_ascii_lowercase();

    command.contains(&executable_path) && command.contains("%1")
}

fn default_icon_matches(icon: &str, expected: &ExpectedFileAssociation) -> bool {
    icon.to_ascii_lowercase()
        .contains(&expected.executable_path.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn reports_installed_when_registry_values_match() {
        let expected = ExpectedFileAssociation::for_executable(
            r"C:\Users\Rizum\AppData\Local\Rizum Silicate\silicate.exe",
        );
        let snapshot = FileAssociationSnapshot {
            extension_prog_id: Some(PROG_ID.to_owned()),
            content_type: Some(CONTENT_TYPE.to_owned()),
            perceived_type: Some(PERCEIVED_TYPE.to_owned()),
            open_command: Some(format!("\"{}\" \"%1\"", expected.executable_path)),
            default_icon: Some(format!("{},0", expected.executable_path)),
        };

        let status = evaluate_file_association(&snapshot, &expected);

        assert_eq!(status.state, IntegrationState::Installed);
        assert!(status.issues.is_empty());
    }

    #[test]
    fn reports_missing_when_extension_has_no_prog_id() {
        let expected = ExpectedFileAssociation::for_executable(r"C:\Silicate\silicate.exe");
        let snapshot = FileAssociationSnapshot {
            extension_prog_id: None,
            content_type: None,
            perceived_type: None,
            open_command: None,
            default_icon: None,
        };

        let status = evaluate_file_association(&snapshot, &expected);

        assert_eq!(status.state, IntegrationState::Missing);
        assert_eq!(status.issues, vec![FileAssociationIssue::MissingProgId]);
    }

    #[test]
    fn reports_incomplete_when_required_values_are_wrong_or_missing() {
        let expected = ExpectedFileAssociation::for_executable(r"C:\Silicate\silicate.exe");
        let snapshot = FileAssociationSnapshot {
            extension_prog_id: Some("OtherApp.procreate".to_owned()),
            content_type: Some("application/zip".to_owned()),
            perceived_type: Some(PERCEIVED_TYPE.to_owned()),
            open_command: Some(r#""C:\Other\viewer.exe" "%1""#.to_owned()),
            default_icon: Some(r"C:\Other\viewer.exe,0".to_owned()),
        };

        let status = evaluate_file_association(&snapshot, &expected);

        assert_eq!(status.state, IntegrationState::Incomplete);
        assert_eq!(
            status.issues,
            vec![
                FileAssociationIssue::WrongProgId,
                FileAssociationIssue::WrongContentType,
                FileAssociationIssue::WrongOpenCommand,
                FileAssociationIssue::WrongDefaultIcon,
            ]
        );
    }

    #[test]
    fn reads_file_association_snapshot_from_hkcu_classes_without_writes() {
        let reader = FakeRegistryReader::new([
            ((r"Software\Classes\.procreate", None), PROG_ID.to_owned()),
            (
                (r"Software\Classes\.procreate", Some(CONTENT_TYPE_VALUE)),
                CONTENT_TYPE.to_owned(),
            ),
            (
                (r"Software\Classes\.procreate", Some(PERCEIVED_TYPE_VALUE)),
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
        ]);

        let snapshot = read_file_association_snapshot(&reader).unwrap();

        assert_eq!(snapshot.extension_prog_id, Some(PROG_ID.to_owned()));
        assert_eq!(snapshot.content_type, Some(CONTENT_TYPE.to_owned()));
        assert_eq!(snapshot.perceived_type, Some(PERCEIVED_TYPE.to_owned()));
        assert_eq!(
            snapshot.open_command,
            Some(r#""C:\Silicate\silicate.exe" "%1""#.to_owned())
        );
        assert_eq!(
            reader.reads(),
            vec![
                (r"Software\Classes\.procreate".to_owned(), None),
                (
                    r"Software\Classes\.procreate".to_owned(),
                    Some(CONTENT_TYPE_VALUE.to_owned())
                ),
                (
                    r"Software\Classes\.procreate".to_owned(),
                    Some(PERCEIVED_TYPE_VALUE.to_owned())
                ),
                (
                    r"Software\Classes\RizumSilicate.procreate\shell\open\command".to_owned(),
                    None
                ),
                (
                    r"Software\Classes\RizumSilicate.procreate\DefaultIcon".to_owned(),
                    None
                ),
            ]
        );
    }

    struct FakeRegistryReader {
        values: HashMap<(String, Option<String>), String>,
        reads: std::cell::RefCell<Vec<(String, Option<String>)>>,
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
                reads: std::cell::RefCell::new(Vec::new()),
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
}

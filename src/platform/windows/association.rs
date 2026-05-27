use std::path::Path;

pub const PROCREATE_EXTENSION: &str = ".procreate";
pub const PROG_ID: &str = "RizumSilicate.procreate";
pub const CONTENT_TYPE: &str = "application/x-procreate";
pub const PERCEIVED_TYPE: &str = "image";

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
}

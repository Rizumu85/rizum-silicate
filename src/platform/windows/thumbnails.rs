use super::association::PROCREATE_EXTENSION;
use super::registry::{
    hkcu_classes_root, hkcu_classes_subkey, RegistryReadError, RegistryValueName,
    RegistryValueReader,
};
use std::path::{Path, PathBuf};

pub const THUMBNAIL_HANDLER_SHELLEX_GUID: &str = "{e357fccd-a995-4576-b01f-234630154e96}";
pub const THUMBNAIL_PROVIDER_CLSID: &str = "{6F52A378-4E3D-4FE3-A49F-3E4D9CF03AF1}";
pub const THUMBNAIL_PROVIDER_THREADING_MODEL: &str = "Apartment";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailRegistrationSnapshot {
    pub shell_extension_clsid: Option<String>,
    pub provider_dll_path: Option<String>,
    pub provider_threading_model: Option<String>,
    pub provider_dll_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedThumbnailProvider {
    pub clsid: String,
    pub dll_path: PathBuf,
}

impl ExpectedThumbnailProvider {
    pub fn new(dll_path: impl Into<PathBuf>) -> Self {
        Self {
            clsid: THUMBNAIL_PROVIDER_CLSID.to_owned(),
            dll_path: dll_path.into(),
        }
    }
}

pub trait FilePresenceReader {
    fn path_exists(&self, path: &Path) -> bool;
}

pub struct OsFilePresenceReader;

impl FilePresenceReader for OsFilePresenceReader {
    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailRegistrationStatus {
    pub state: ThumbnailIntegrationState,
    pub issues: Vec<ThumbnailRegistrationIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailIntegrationState {
    Installed,
    Missing,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailRegistrationIssue {
    MissingShellExtension,
    WrongShellExtensionClsid,
    MissingProviderDllRegistration,
    WrongProviderDllRegistration,
    MissingProviderThreadingModel,
    WrongProviderThreadingModel,
    MissingProviderDllFile,
}

pub fn read_thumbnail_registration_snapshot(
    registry: &impl RegistryValueReader,
    files: &impl FilePresenceReader,
    expected: &ExpectedThumbnailProvider,
) -> Result<ThumbnailRegistrationSnapshot, RegistryReadError> {
    let shell_extension_clsid = registry.read_hkcu_string(
        &format!(
            r"{}\ShellEx\{}",
            hkcu_classes_subkey(PROCREATE_EXTENSION),
            THUMBNAIL_HANDLER_SHELLEX_GUID
        ),
        RegistryValueName::Default,
    )?;
    let provider_dll_path = registry.read_hkcu_string(
        &format!(
            r"{}\CLSID\{}\InprocServer32",
            hkcu_classes_root(),
            expected.clsid
        ),
        RegistryValueName::Default,
    )?;
    let provider_threading_model = registry.read_hkcu_string(
        &format!(
            r"{}\CLSID\{}\InprocServer32",
            hkcu_classes_root(),
            expected.clsid
        ),
        RegistryValueName::Named("ThreadingModel"),
    )?;
    let provider_dll_exists = provider_dll_path
        .as_deref()
        .map(Path::new)
        .is_some_and(|path| files.path_exists(path));

    Ok(ThumbnailRegistrationSnapshot {
        shell_extension_clsid,
        provider_dll_path,
        provider_threading_model,
        provider_dll_exists,
    })
}

pub fn evaluate_thumbnail_registration(
    snapshot: &ThumbnailRegistrationSnapshot,
    expected: &ExpectedThumbnailProvider,
) -> ThumbnailRegistrationStatus {
    let mut issues = Vec::new();

    match snapshot.shell_extension_clsid.as_deref() {
        None => {
            return ThumbnailRegistrationStatus {
                state: ThumbnailIntegrationState::Missing,
                issues: vec![ThumbnailRegistrationIssue::MissingShellExtension],
            };
        }
        Some(clsid) if clsid.eq_ignore_ascii_case(&expected.clsid) => {}
        Some(_) => issues.push(ThumbnailRegistrationIssue::WrongShellExtensionClsid),
    }

    match snapshot.provider_dll_path.as_deref() {
        None => issues.push(ThumbnailRegistrationIssue::MissingProviderDllRegistration),
        Some(path) if same_path_text(path, &expected.dll_path) => {}
        Some(_) => issues.push(ThumbnailRegistrationIssue::WrongProviderDllRegistration),
    }

    match snapshot.provider_threading_model.as_deref() {
        None => issues.push(ThumbnailRegistrationIssue::MissingProviderThreadingModel),
        Some(model) if model.eq_ignore_ascii_case(THUMBNAIL_PROVIDER_THREADING_MODEL) => {}
        Some(_) => issues.push(ThumbnailRegistrationIssue::WrongProviderThreadingModel),
    }

    if !snapshot.provider_dll_exists {
        issues.push(ThumbnailRegistrationIssue::MissingProviderDllFile);
    }

    let state = if issues.is_empty() {
        ThumbnailIntegrationState::Installed
    } else {
        ThumbnailIntegrationState::Incomplete
    };

    ThumbnailRegistrationStatus { state, issues }
}

fn same_path_text(actual: &str, expected: &Path) -> bool {
    actual.eq_ignore_ascii_case(&expected.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn reports_installed_when_thumbnail_registration_matches_and_dll_exists() {
        let expected = ExpectedThumbnailProvider::new(
            r"C:\Users\Rizum\AppData\Local\Rizum Silicate\rizum_silicate_thumb.dll",
        );
        let snapshot = ThumbnailRegistrationSnapshot {
            shell_extension_clsid: Some(expected.clsid.clone()),
            provider_dll_path: Some(expected.dll_path.to_string_lossy().into_owned()),
            provider_threading_model: Some(THUMBNAIL_PROVIDER_THREADING_MODEL.to_owned()),
            provider_dll_exists: true,
        };

        let status = evaluate_thumbnail_registration(&snapshot, &expected);

        assert_eq!(status.state, ThumbnailIntegrationState::Installed);
        assert!(status.issues.is_empty());
    }

    #[test]
    fn reports_missing_when_shell_extension_is_absent() {
        let expected = ExpectedThumbnailProvider::new(r"C:\Silicate\rizum_silicate_thumb.dll");
        let snapshot = ThumbnailRegistrationSnapshot {
            shell_extension_clsid: None,
            provider_dll_path: None,
            provider_threading_model: None,
            provider_dll_exists: false,
        };

        let status = evaluate_thumbnail_registration(&snapshot, &expected);

        assert_eq!(status.state, ThumbnailIntegrationState::Missing);
        assert_eq!(
            status.issues,
            vec![ThumbnailRegistrationIssue::MissingShellExtension]
        );
    }

    #[test]
    fn reports_incomplete_when_registration_values_are_wrong_or_missing() {
        let expected = ExpectedThumbnailProvider::new(r"C:\Silicate\rizum_silicate_thumb.dll");
        let snapshot = ThumbnailRegistrationSnapshot {
            shell_extension_clsid: Some("{00000000-0000-0000-0000-000000000000}".to_owned()),
            provider_dll_path: Some(r"C:\Other\thumb.dll".to_owned()),
            provider_threading_model: Some("Free".to_owned()),
            provider_dll_exists: false,
        };

        let status = evaluate_thumbnail_registration(&snapshot, &expected);

        assert_eq!(status.state, ThumbnailIntegrationState::Incomplete);
        assert_eq!(
            status.issues,
            vec![
                ThumbnailRegistrationIssue::WrongShellExtensionClsid,
                ThumbnailRegistrationIssue::WrongProviderDllRegistration,
                ThumbnailRegistrationIssue::WrongProviderThreadingModel,
                ThumbnailRegistrationIssue::MissingProviderDllFile,
            ]
        );
    }

    #[test]
    fn reads_thumbnail_snapshot_without_writes() {
        let expected = ExpectedThumbnailProvider::new(r"C:\Silicate\rizum_silicate_thumb.dll");
        let registry = FakeRegistryReader::new([
            (
                (
                    r"Software\Classes\.procreate\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}",
                    None,
                ),
                expected.clsid.clone(),
            ),
            (
                (
                    r"Software\Classes\CLSID\{6F52A378-4E3D-4FE3-A49F-3E4D9CF03AF1}\InprocServer32",
                    Some("ThreadingModel"),
                ),
                THUMBNAIL_PROVIDER_THREADING_MODEL.to_owned(),
            ),
            (
                (
                    r"Software\Classes\CLSID\{6F52A378-4E3D-4FE3-A49F-3E4D9CF03AF1}\InprocServer32",
                    None,
                ),
                expected.dll_path.to_string_lossy().into_owned(),
            ),
        ]);
        let files = FakeFilePresenceReader::new([expected.dll_path.clone()]);

        let snapshot = read_thumbnail_registration_snapshot(&registry, &files, &expected).unwrap();

        assert_eq!(snapshot.shell_extension_clsid, Some(expected.clsid.clone()));
        assert_eq!(
            snapshot.provider_dll_path,
            Some(expected.dll_path.to_string_lossy().into_owned())
        );
        assert_eq!(
            snapshot.provider_threading_model,
            Some(THUMBNAIL_PROVIDER_THREADING_MODEL.to_owned())
        );
        assert!(snapshot.provider_dll_exists);
        assert_eq!(
            registry.reads(),
            vec![
                (
                    r"Software\Classes\.procreate\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}"
                        .to_owned(),
                    None
                ),
                (
                    r"Software\Classes\CLSID\{6F52A378-4E3D-4FE3-A49F-3E4D9CF03AF1}\InprocServer32"
                        .to_owned(),
                    None
                ),
                (
                    r"Software\Classes\CLSID\{6F52A378-4E3D-4FE3-A49F-3E4D9CF03AF1}\InprocServer32"
                        .to_owned(),
                    Some("ThreadingModel".to_owned())
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

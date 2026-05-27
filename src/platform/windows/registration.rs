use super::association::{
    CONTENT_TYPE, ExpectedFileAssociation, PERCEIVED_TYPE, PROCREATE_EXTENSION, PROG_ID,
};
use super::registry::{RegistryValueName, hkcu_classes_root, hkcu_classes_subkey};
use super::status::ExpectedWindowsIntegration;
use super::thumbnails::{ExpectedThumbnailProvider, THUMBNAIL_HANDLER_SHELLEX_GUID};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryInstallPlan {
    pub writes: Vec<RegistryWrite>,
}

impl RegistryInstallPlan {
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryWrite {
    pub subkey: String,
    pub value_name: Option<String>,
    pub value: String,
}

pub fn build_install_or_repair_registry_plan(
    expected: &ExpectedWindowsIntegration,
) -> RegistryInstallPlan {
    let mut writes = Vec::new();
    append_file_association_writes(&expected.file_association, &mut writes);
    append_thumbnail_registration_writes(&expected.thumbnails, &mut writes);

    RegistryInstallPlan { writes }
}

fn append_file_association_writes(
    expected: &ExpectedFileAssociation,
    writes: &mut Vec<RegistryWrite>,
) {
    let extension_key = hkcu_classes_subkey(PROCREATE_EXTENSION);
    let prog_id_key = hkcu_classes_subkey(PROG_ID);

    writes.extend([
        registry_write(&extension_key, RegistryValueName::Default, PROG_ID),
        registry_write(
            &extension_key,
            RegistryValueName::Named("Content Type"),
            CONTENT_TYPE,
        ),
        registry_write(
            &extension_key,
            RegistryValueName::Named("PerceivedType"),
            PERCEIVED_TYPE,
        ),
        registry_write(
            &format!(r"{prog_id_key}\shell\open\command"),
            RegistryValueName::Default,
            &format!(r#""{}" "%1""#, expected.executable_path),
        ),
        registry_write(
            &format!(r"{prog_id_key}\DefaultIcon"),
            RegistryValueName::Default,
            &format!("{},0", expected.executable_path),
        ),
    ]);
}

fn append_thumbnail_registration_writes(
    expected: &ExpectedThumbnailProvider,
    writes: &mut Vec<RegistryWrite>,
) {
    writes.extend([
        registry_write(
            &format!(
                r"{}\ShellEx\{}",
                hkcu_classes_subkey(PROCREATE_EXTENSION),
                THUMBNAIL_HANDLER_SHELLEX_GUID
            ),
            RegistryValueName::Default,
            &expected.clsid,
        ),
        registry_write(
            &format!(
                r"{}\CLSID\{}\InprocServer32",
                hkcu_classes_root(),
                expected.clsid
            ),
            RegistryValueName::Default,
            &expected.dll_path.to_string_lossy(),
        ),
    ]);
}

fn registry_write(subkey: &str, value_name: RegistryValueName<'_>, value: &str) -> RegistryWrite {
    RegistryWrite {
        subkey: subkey.to_owned(),
        value_name: value_name.to_option_string(),
        value: value.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::thumbnails::THUMBNAIL_PROVIDER_CLSID;
    use super::*;

    #[test]
    fn builds_install_or_repair_plan_for_file_association_and_thumbnails() {
        let expected = ExpectedWindowsIntegration::new(
            r"C:\Silicate\silicate.exe",
            r"C:\Silicate\rizum_silicate_thumb.dll",
        );

        let plan = build_install_or_repair_registry_plan(&expected);

        assert!(!plan.is_empty());
        assert_eq!(
            plan.writes,
            vec![
                write(r"Software\Classes\.procreate", None, PROG_ID),
                write(
                    r"Software\Classes\.procreate",
                    Some("Content Type"),
                    CONTENT_TYPE,
                ),
                write(
                    r"Software\Classes\.procreate",
                    Some("PerceivedType"),
                    PERCEIVED_TYPE,
                ),
                write(
                    r"Software\Classes\RizumSilicate.procreate\shell\open\command",
                    None,
                    r#""C:\Silicate\silicate.exe" "%1""#,
                ),
                write(
                    r"Software\Classes\RizumSilicate.procreate\DefaultIcon",
                    None,
                    r"C:\Silicate\silicate.exe,0",
                ),
                write(
                    r"Software\Classes\.procreate\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}",
                    None,
                    THUMBNAIL_PROVIDER_CLSID,
                ),
                write(
                    r"Software\Classes\CLSID\{6F52A378-4E3D-4FE3-A49F-3E4D9CF03AF1}\InprocServer32",
                    None,
                    r"C:\Silicate\rizum_silicate_thumb.dll",
                ),
            ]
        );
    }

    fn write(subkey: &str, value_name: Option<&str>, value: &str) -> RegistryWrite {
        RegistryWrite {
            subkey: subkey.to_owned(),
            value_name: value_name.map(str::to_owned),
            value: value.to_owned(),
        }
    }
}

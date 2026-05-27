use super::association::{
    CONTENT_TYPE, ExpectedFileAssociation, PERCEIVED_TYPE, PROCREATE_EXTENSION, PROG_ID,
};
use super::registry::{
    RegistryDeleteError, RegistryKeyDeleter, RegistryValueName, RegistryValueWriter,
    RegistryWriteError, hkcu_classes_root, hkcu_classes_subkey,
};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryUninstallPlan {
    pub delete_trees: Vec<String>,
}

impl RegistryUninstallPlan {
    pub fn is_empty(&self) -> bool {
        self.delete_trees.is_empty()
    }
}

pub fn build_install_or_repair_registry_plan(
    expected: &ExpectedWindowsIntegration,
) -> RegistryInstallPlan {
    let mut writes = Vec::new();
    append_file_association_writes(&expected.file_association, &mut writes);
    append_thumbnail_registration_writes(&expected.thumbnails, &mut writes);

    RegistryInstallPlan { writes }
}

pub fn build_uninstall_registry_plan(
    expected: &ExpectedWindowsIntegration,
) -> RegistryUninstallPlan {
    let mut delete_trees = vec![
        hkcu_classes_subkey(PROG_ID),
        format!(
            r"{}\CLSID\{}",
            hkcu_classes_root(),
            expected.thumbnails.clsid
        ),
        hkcu_classes_subkey(PROCREATE_EXTENSION),
    ];

    delete_trees.dedup();
    RegistryUninstallPlan { delete_trees }
}

pub fn apply_registry_install_plan(
    plan: &RegistryInstallPlan,
    writer: &impl RegistryValueWriter,
) -> Result<(), RegistryWriteError> {
    for write in &plan.writes {
        writer.write_hkcu_string(
            &write.subkey,
            registry_value_name(write.value_name.as_deref()),
            &write.value,
        )?;
    }

    Ok(())
}

pub fn apply_registry_uninstall_plan(
    plan: &RegistryUninstallPlan,
    deleter: &impl RegistryKeyDeleter,
) -> Result<(), RegistryDeleteError> {
    for subkey in &plan.delete_trees {
        deleter.delete_hkcu_tree(subkey)?;
    }

    Ok(())
}

#[cfg(windows)]
pub fn install_or_repair_current_windows_integration(
) -> Result<(), WindowsIntegrationInstallError> {
    use super::registry::WindowsRegistryWriter;

    let app_executable_path =
        std::env::current_exe().map_err(WindowsIntegrationInstallError::CurrentExe)?;
    let expected = ExpectedWindowsIntegration::for_app_executable(app_executable_path);
    let plan = build_install_or_repair_registry_plan(&expected);

    apply_registry_install_plan(&plan, &WindowsRegistryWriter)
        .map_err(WindowsIntegrationInstallError::Registry)
}

#[cfg(windows)]
#[derive(Debug)]
pub enum WindowsIntegrationInstallError {
    CurrentExe(std::io::Error),
    Registry(RegistryWriteError),
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

fn registry_value_name(value_name: Option<&str>) -> RegistryValueName<'_> {
    match value_name {
        None => RegistryValueName::Default,
        Some(name) => RegistryValueName::Named(name),
    }
}

#[cfg(test)]
mod tests {
    use super::super::thumbnails::THUMBNAIL_PROVIDER_CLSID;
    use super::*;
    use std::cell::RefCell;

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

    #[test]
    fn builds_uninstall_plan_for_owned_file_association_and_thumbnail_keys() {
        let expected = ExpectedWindowsIntegration::new(
            r"C:\Silicate\silicate.exe",
            r"C:\Silicate\rizum_silicate_thumb.dll",
        );

        let plan = build_uninstall_registry_plan(&expected);

        assert!(!plan.is_empty());
        assert_eq!(
            plan.delete_trees,
            vec![
                r"Software\Classes\RizumSilicate.procreate".to_owned(),
                r"Software\Classes\CLSID\{6F52A378-4E3D-4FE3-A49F-3E4D9CF03AF1}".to_owned(),
                r"Software\Classes\.procreate".to_owned(),
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

    #[test]
    fn applies_install_plan_to_writer_in_order() {
        let plan = RegistryInstallPlan {
            writes: vec![
                write(r"Software\Classes\.procreate", None, PROG_ID),
                write(
                    r"Software\Classes\.procreate",
                    Some("Content Type"),
                    CONTENT_TYPE,
                ),
            ],
        };
        let writer = FakeRegistryWriter::default();

        apply_registry_install_plan(&plan, &writer).unwrap();

        assert_eq!(writer.writes.borrow().as_slice(), plan.writes.as_slice());
    }

    #[test]
    fn stops_applying_install_plan_after_first_writer_error() {
        let plan = RegistryInstallPlan {
            writes: vec![
                write(r"Software\Classes\.procreate", None, PROG_ID),
                write(
                    r"Software\Classes\.procreate",
                    Some("Content Type"),
                    CONTENT_TYPE,
                ),
            ],
        };
        let writer = FakeRegistryWriter {
            fail_after_writes: Some(0),
            ..Default::default()
        };

        let error = apply_registry_install_plan(&plan, &writer).unwrap_err();

        assert_eq!(error.subkey, r"Software\Classes\.procreate");
        assert_eq!(error.value_name, None);
        assert!(writer.writes.borrow().is_empty());
    }

    #[test]
    fn applies_uninstall_plan_to_deleter_in_order() {
        let plan = RegistryUninstallPlan {
            delete_trees: vec![
                r"Software\Classes\RizumSilicate.procreate".to_owned(),
                r"Software\Classes\.procreate".to_owned(),
            ],
        };
        let deleter = FakeRegistryDeleter::default();

        apply_registry_uninstall_plan(&plan, &deleter).unwrap();

        assert_eq!(
            deleter.delete_trees.borrow().as_slice(),
            plan.delete_trees.as_slice()
        );
    }

    #[test]
    fn stops_applying_uninstall_plan_after_first_delete_error() {
        let plan = RegistryUninstallPlan {
            delete_trees: vec![
                r"Software\Classes\RizumSilicate.procreate".to_owned(),
                r"Software\Classes\.procreate".to_owned(),
            ],
        };
        let deleter = FakeRegistryDeleter {
            fail_after_deletes: Some(0),
            ..Default::default()
        };

        let error = apply_registry_uninstall_plan(&plan, &deleter).unwrap_err();

        assert_eq!(error.subkey, r"Software\Classes\RizumSilicate.procreate");
        assert!(deleter.delete_trees.borrow().is_empty());
    }

    #[derive(Default)]
    struct FakeRegistryWriter {
        writes: RefCell<Vec<RegistryWrite>>,
        fail_after_writes: Option<usize>,
    }

    impl RegistryValueWriter for FakeRegistryWriter {
        fn write_hkcu_string(
            &self,
            subkey: &str,
            value_name: RegistryValueName<'_>,
            value: &str,
        ) -> Result<(), RegistryWriteError> {
            if self
                .fail_after_writes
                .is_some_and(|limit| self.writes.borrow().len() >= limit)
            {
                return Err(RegistryWriteError {
                    subkey: subkey.to_owned(),
                    value_name: value_name.to_option_string(),
                    message: "planned failure".to_owned(),
                });
            }

            self.writes.borrow_mut().push(RegistryWrite {
                subkey: subkey.to_owned(),
                value_name: value_name.to_option_string(),
                value: value.to_owned(),
            });
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeRegistryDeleter {
        delete_trees: RefCell<Vec<String>>,
        fail_after_deletes: Option<usize>,
    }

    impl RegistryKeyDeleter for FakeRegistryDeleter {
        fn delete_hkcu_tree(&self, subkey: &str) -> Result<(), RegistryDeleteError> {
            if self
                .fail_after_deletes
                .is_some_and(|limit| self.delete_trees.borrow().len() >= limit)
            {
                return Err(RegistryDeleteError {
                    subkey: subkey.to_owned(),
                    message: "planned failure".to_owned(),
                });
            }

            self.delete_trees.borrow_mut().push(subkey.to_owned());
            Ok(())
        }
    }
}

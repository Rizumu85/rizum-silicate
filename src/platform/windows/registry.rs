pub const HKCU_CLASSES_ROOT: &str = r"Software\Classes";

pub trait RegistryValueReader {
    fn read_hkcu_string(
        &self,
        subkey: &str,
        value_name: RegistryValueName<'_>,
    ) -> Result<Option<String>, RegistryReadError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryValueName<'a> {
    Default,
    Named(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryReadError {
    pub subkey: String,
    pub value_name: Option<String>,
    pub message: String,
}

pub fn hkcu_classes_subkey(name: &str) -> String {
    format!(r"{HKCU_CLASSES_ROOT}\{name}")
}

#[cfg(windows)]
pub struct WindowsRegistryReader;

#[cfg(windows)]
impl RegistryValueReader for WindowsRegistryReader {
    fn read_hkcu_string(
        &self,
        subkey: &str,
        value_name: RegistryValueName<'_>,
    ) -> Result<Option<String>, RegistryReadError> {
        read_hkcu_registry_string(subkey, value_name)
    }
}

#[cfg(windows)]
fn read_hkcu_registry_string(
    subkey: &str,
    value_name: RegistryValueName<'_>,
) -> Result<Option<String>, RegistryReadError> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, REG_VALUE_TYPE, RRF_RT_REG_SZ,
    };

    let subkey_wide = wide_null(subkey);
    let value_name_wide = match value_name {
        RegistryValueName::Default => None,
        RegistryValueName::Named(name) => Some(wide_null(name)),
    };
    let value_name_pcwstr = value_name_wide
        .as_ref()
        .map(|wide| PCWSTR(wide.as_ptr()))
        .unwrap_or_else(PCWSTR::null);

    let mut value_type = REG_VALUE_TYPE::default();
    let mut byte_len = 0u32;

    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_wide.as_ptr()),
            value_name_pcwstr,
            RRF_RT_REG_SZ,
            Some(&mut value_type),
            None,
            Some(&mut byte_len),
        )
    };

    if result == ERROR_FILE_NOT_FOUND || result == ERROR_PATH_NOT_FOUND {
        return Ok(None);
    }
    if result != ERROR_SUCCESS {
        return Err(registry_error(subkey, value_name, result.0));
    }

    let mut buffer = vec![0u16; byte_len.div_ceil(2) as usize];
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_wide.as_ptr()),
            value_name_pcwstr,
            RRF_RT_REG_SZ,
            Some(&mut value_type),
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut byte_len),
        )
    };

    if result != ERROR_SUCCESS {
        return Err(registry_error(subkey, value_name, result.0));
    }

    while buffer.last() == Some(&0) {
        buffer.pop();
    }

    String::from_utf16(&buffer)
        .map(Some)
        .map_err(|err| RegistryReadError {
            subkey: subkey.to_owned(),
            value_name: value_name.to_option_string(),
            message: format!("Registry string is not valid UTF-16: {err}"),
        })
}

#[cfg(windows)]
fn registry_error(subkey: &str, value_name: RegistryValueName<'_>, code: u32) -> RegistryReadError {
    RegistryReadError {
        subkey: subkey.to_owned(),
        value_name: value_name.to_option_string(),
        message: format!("Win32 registry read failed with code {code}"),
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

impl RegistryValueName<'_> {
    pub(crate) fn to_option_string(self) -> Option<String> {
        match self {
            Self::Default => None,
            Self::Named(name) => Some(name.to_owned()),
        }
    }
}

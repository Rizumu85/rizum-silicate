#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::CString;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::core::{PCSTR, PCWSTR};

    let dll_path = thumbnail_dll_path()?;
    let wide_path = dll_path
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect::<Vec<_>>();
    let library = LoadedLibrary(unsafe { LoadLibraryW(PCWSTR::from_raw(wide_path.as_ptr()))? });

    for export_name in ["DllGetClassObject", "DllCanUnloadNow"] {
        let symbol = CString::new(export_name)?;
        let address = unsafe { GetProcAddress(library.0, PCSTR::from_raw(symbol.as_ptr().cast())) };
        if address.is_none() {
            return Err(format!("missing export {export_name} in {}", dll_path.display()).into());
        }
    }

    println!(
        "verified Windows thumbnail DLL exports: {}",
        dll_path.display()
    );
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    println!("Windows thumbnail DLL export verification is only available on Windows");
}

#[cfg(windows)]
struct LoadedLibrary(windows::Win32::Foundation::HMODULE);

#[cfg(windows)]
impl Drop for LoadedLibrary {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::FreeLibrary(self.0);
        }
    }
}

#[cfg(windows)]
fn thumbnail_dll_path() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::args_os().nth(1) {
        return Ok(std::path::PathBuf::from(path));
    }

    let example_exe = std::env::current_exe()?;
    let target_profile_dir = example_exe
        .parent()
        .and_then(|examples_dir| examples_dir.parent())
        .ok_or("could not derive target profile directory from example executable")?;

    Ok(target_profile_dir.join("rizum_silicate_thumb.dll"))
}

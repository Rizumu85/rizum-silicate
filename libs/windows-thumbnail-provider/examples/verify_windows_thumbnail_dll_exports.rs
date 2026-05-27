#[cfg(windows)]
use std::ffi::CString;
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetProcAddress;
#[cfg(windows)]
use windows::core::{GUID, HRESULT, PCSTR};

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::Com::IClassFactory;
    use windows::Win32::System::LibraryLoader::LoadLibraryW;
    use windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream;
    use windows::core::{Interface, PCWSTR};

    let dll_path = thumbnail_dll_path()?;
    let wide_path = dll_path
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect::<Vec<_>>();
    let library = LoadedLibrary(unsafe { LoadLibraryW(PCWSTR::from_raw(wide_path.as_ptr()))? });

    let dll_get_class_object: DllGetClassObject =
        unsafe { std::mem::transmute(load_export(&library, "DllGetClassObject")?) };
    let dll_can_unload_now: DllCanUnloadNow =
        unsafe { std::mem::transmute(load_export(&library, "DllCanUnloadNow")?) };

    let mut factory_raw = std::ptr::null_mut();
    let result = unsafe {
        dll_get_class_object(
            &rizum_silicate_thumb::THUMBNAIL_PROVIDER_CLSID,
            &IClassFactory::IID,
            &mut factory_raw,
        )
    };
    result.ok()?;

    let factory = unsafe { IClassFactory::from_raw(factory_raw) };
    let initializer: IInitializeWithStream = unsafe { factory.CreateInstance(None)? };
    if initializer.as_raw().is_null() {
        return Err("DllGetClassObject returned a null stream initializer".into());
    }

    let _ = unsafe { dll_can_unload_now() };

    println!(
        "verified Windows thumbnail DLL class factory exports: {}",
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
type DllGetClassObject = unsafe extern "system" fn(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut std::ffi::c_void,
) -> HRESULT;

#[cfg(windows)]
type DllCanUnloadNow = unsafe extern "system" fn() -> HRESULT;

#[cfg(windows)]
fn load_export(
    library: &LoadedLibrary,
    export_name: &str,
) -> Result<windows::Win32::Foundation::FARPROC, Box<dyn std::error::Error>> {
    let symbol = CString::new(export_name)?;
    let address = unsafe { GetProcAddress(library.0, PCSTR::from_raw(symbol.as_ptr().cast())) };
    if address.is_none() {
        return Err(format!("missing export {export_name}").into());
    }

    Ok(address)
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

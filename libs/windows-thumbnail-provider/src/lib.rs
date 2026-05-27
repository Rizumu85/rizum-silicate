use std::path::Path;

use rizum_platform_thumbnail::{
    PlatformThumbnailError, PlatformThumbnailRgba, load_platform_thumbnail_rgba,
    load_platform_thumbnail_rgba_from_archive_bytes,
};

#[cfg(windows)]
pub const THUMBNAIL_PROVIDER_CLSID: windows::core::GUID =
    windows::core::GUID::from_u128(0x6f52a378_4e3d_4fe3_a49f_3e4d9cf03af1);

#[cfg(windows)]
static ACTIVE_COM_OBJECT_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[cfg(windows)]
static SERVER_LOCK_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsThumbnailBitmap {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowsThumbnailBitmapError {
    PixelLengthMismatch {
        width: u32,
        height: u32,
        expected_len: usize,
        actual_len: usize,
    },
}

#[derive(Debug)]
pub enum WindowsThumbnailLoadError {
    Platform(PlatformThumbnailError),
    Bitmap(WindowsThumbnailBitmapError),
}

#[cfg(windows)]
#[derive(Debug)]
pub struct OwnedWindowsThumbnailHbitmap {
    handle: windows::Win32::Graphics::Gdi::HBITMAP,
}

#[cfg(windows)]
impl OwnedWindowsThumbnailHbitmap {
    pub fn handle(&self) -> windows::Win32::Graphics::Gdi::HBITMAP {
        self.handle
    }

    pub fn into_raw(self) -> windows::Win32::Graphics::Gdi::HBITMAP {
        let this = std::mem::ManuallyDrop::new(self);
        this.handle
    }
}

#[cfg(windows)]
#[derive(Debug)]
pub struct WindowsShellThumbnail {
    pub hbitmap: OwnedWindowsThumbnailHbitmap,
    pub alpha_type: windows::Win32::UI::Shell::WTS_ALPHATYPE,
}

#[cfg(windows)]
#[derive(Debug)]
pub enum WindowsShellThumbnailError {
    Load(WindowsThumbnailLoadError),
    Hbitmap(windows::core::Error),
}

#[cfg(windows)]
pub fn load_windows_shell_thumbnail(
    path: impl AsRef<Path>,
) -> Result<Option<WindowsShellThumbnail>, WindowsShellThumbnailError> {
    let Some(bitmap) =
        load_windows_thumbnail_bitmap(path).map_err(WindowsShellThumbnailError::Load)?
    else {
        return Ok(None);
    };
    let hbitmap =
        create_hbitmap_from_windows_bitmap(&bitmap).map_err(WindowsShellThumbnailError::Hbitmap)?;

    Ok(Some(WindowsShellThumbnail {
        hbitmap,
        alpha_type: windows::Win32::UI::Shell::WTSAT_ARGB,
    }))
}

#[cfg(windows)]
pub fn load_windows_shell_thumbnail_from_archive_bytes(
    bytes: &[u8],
) -> Result<Option<WindowsShellThumbnail>, WindowsShellThumbnailError> {
    let Some(bitmap) = load_windows_thumbnail_bitmap_from_archive_bytes(bytes)
        .map_err(WindowsShellThumbnailError::Load)?
    else {
        return Ok(None);
    };
    let hbitmap =
        create_hbitmap_from_windows_bitmap(&bitmap).map_err(WindowsShellThumbnailError::Hbitmap)?;

    Ok(Some(WindowsShellThumbnail {
        hbitmap,
        alpha_type: windows::Win32::UI::Shell::WTSAT_ARGB,
    }))
}

#[cfg(windows)]
pub unsafe fn write_shell_thumbnail_outputs(
    thumbnail: WindowsShellThumbnail,
    phbmp: *mut windows::Win32::Graphics::Gdi::HBITMAP,
    pdwalpha: *mut windows::Win32::UI::Shell::WTS_ALPHATYPE,
) -> Result<(), windows::core::Error> {
    if phbmp.is_null() || pdwalpha.is_null() {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80004003_u32 as _,
        )));
    }

    unsafe {
        *phbmp = thumbnail.hbitmap.into_raw();
        *pdwalpha = thumbnail.alpha_type;
    }

    Ok(())
}

#[cfg(windows)]
#[windows::core::implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile
)]
pub struct RizumWindowsThumbnailProvider {
    path: std::sync::Mutex<Option<std::path::PathBuf>>,
}

#[cfg(windows)]
impl Default for RizumWindowsThumbnailProvider {
    fn default() -> Self {
        increment_active_com_object_count();
        Self {
            path: std::sync::Mutex::new(None),
        }
    }
}

#[cfg(windows)]
impl Drop for RizumWindowsThumbnailProvider {
    fn drop(&mut self) {
        decrement_active_com_object_count();
    }
}

#[cfg(windows)]
pub fn create_windows_thumbnail_provider_com_object()
-> windows::core::ComObject<RizumWindowsThumbnailProvider> {
    windows::core::ComObject::new(RizumWindowsThumbnailProvider::default())
}

#[cfg(windows)]
#[allow(non_snake_case)]
impl windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile_Impl
    for RizumWindowsThumbnailProvider_Impl
{
    fn Initialize(
        &self,
        pszfilepath: &windows::core::PCWSTR,
        _grfmode: u32,
    ) -> windows::core::Result<()> {
        let path = unsafe { pszfilepath.to_string() }.map_err(|_| {
            windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005_u32 as _))
        })?;
        *self.path.lock().map_err(|_| {
            windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005_u32 as _))
        })? = Some(std::path::PathBuf::from(path));
        Ok(())
    }
}

#[cfg(windows)]
#[allow(non_snake_case)]
impl windows::Win32::UI::Shell::IThumbnailProvider_Impl for RizumWindowsThumbnailProvider_Impl {
    fn GetThumbnail(
        &self,
        _cx: u32,
        phbmp: *mut windows::Win32::Graphics::Gdi::HBITMAP,
        pdwalpha: *mut windows::Win32::UI::Shell::WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let path = self
            .path
            .lock()
            .map_err(|_| {
                windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005_u32 as _))
            })?
            .clone()
            .ok_or_else(|| {
                windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005_u32 as _))
            })?;
        let thumbnail = load_windows_shell_thumbnail(path)
            .map_err(|_| {
                windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005_u32 as _))
            })?
            .ok_or_else(|| {
                windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005_u32 as _))
            })?;

        unsafe { write_shell_thumbnail_outputs(thumbnail, phbmp, pdwalpha) }
    }
}

#[cfg(windows)]
#[windows::core::implement(windows::Win32::System::Com::IClassFactory)]
pub struct RizumWindowsThumbnailClassFactory;

#[cfg(windows)]
impl Default for RizumWindowsThumbnailClassFactory {
    fn default() -> Self {
        increment_active_com_object_count();
        Self
    }
}

#[cfg(windows)]
impl Drop for RizumWindowsThumbnailClassFactory {
    fn drop(&mut self) {
        decrement_active_com_object_count();
    }
}

#[cfg(windows)]
pub fn create_windows_thumbnail_class_factory_com_object()
-> windows::core::ComObject<RizumWindowsThumbnailClassFactory> {
    windows::core::ComObject::new(RizumWindowsThumbnailClassFactory::default())
}

#[cfg(windows)]
#[allow(non_snake_case)]
impl windows::Win32::System::Com::IClassFactory_Impl for RizumWindowsThumbnailClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: windows::core::Ref<windows::core::IUnknown>,
        riid: *const windows::core::GUID,
        ppvobject: *mut *mut std::ffi::c_void,
    ) -> windows::core::Result<()> {
        use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_POINTER};

        if ppvobject.is_null() || riid.is_null() {
            return Err(windows::core::Error::from_hresult(E_POINTER));
        }

        unsafe {
            *ppvobject = std::ptr::null_mut();
        }

        if !punkouter.is_null() {
            return Err(windows::core::Error::from_hresult(CLASS_E_NOAGGREGATION));
        }

        let object = create_windows_thumbnail_provider_com_object();
        unsafe { write_thumbnail_provider_interface(object, &*riid, ppvobject) }
    }

    fn LockServer(&self, flock: windows::core::BOOL) -> windows::core::Result<()> {
        use std::sync::atomic::Ordering;

        if flock.0 != 0 {
            SERVER_LOCK_COUNT.fetch_add(1, Ordering::Relaxed);
        } else {
            let _ = SERVER_LOCK_COUNT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                (count > 0).then_some(count - 1)
            });
        }

        Ok(())
    }
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const windows::core::GUID,
    riid: *const windows::core::GUID,
    ppv: *mut *mut std::ffi::c_void,
) -> windows::core::HRESULT {
    use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_POINTER, S_OK};

    if ppv.is_null() || rclsid.is_null() || riid.is_null() {
        return E_POINTER;
    }

    unsafe {
        *ppv = std::ptr::null_mut();
    }

    if unsafe { *rclsid } != THUMBNAIL_PROVIDER_CLSID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    let object = create_windows_thumbnail_class_factory_com_object();
    match unsafe { write_thumbnail_class_factory_interface(object, &*riid, ppv) } {
        Ok(()) => S_OK,
        Err(error) => error.code(),
    }
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> windows::core::HRESULT {
    use std::sync::atomic::Ordering;
    use windows::Win32::Foundation::{S_FALSE, S_OK};

    if ACTIVE_COM_OBJECT_COUNT.load(Ordering::Relaxed) == 0
        && SERVER_LOCK_COUNT.load(Ordering::Relaxed) == 0
    {
        S_OK
    } else {
        S_FALSE
    }
}

#[cfg(windows)]
unsafe fn write_thumbnail_provider_interface(
    object: windows::core::ComObject<RizumWindowsThumbnailProvider>,
    riid: &windows::core::GUID,
    out: *mut *mut std::ffi::c_void,
) -> windows::core::Result<()> {
    use windows::Win32::Foundation::E_NOINTERFACE;
    use windows::Win32::UI::Shell::IThumbnailProvider;
    use windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile;
    use windows::core::{IUnknown, Interface};

    unsafe {
        if riid == &IThumbnailProvider::IID {
            *out = object.into_interface::<IThumbnailProvider>().into_raw();
            Ok(())
        } else if riid == &IInitializeWithFile::IID {
            *out = object.into_interface::<IInitializeWithFile>().into_raw();
            Ok(())
        } else if riid == &IUnknown::IID {
            *out = object.into_interface::<IUnknown>().into_raw();
            Ok(())
        } else {
            Err(windows::core::Error::from_hresult(E_NOINTERFACE))
        }
    }
}

#[cfg(windows)]
unsafe fn write_thumbnail_class_factory_interface(
    object: windows::core::ComObject<RizumWindowsThumbnailClassFactory>,
    riid: &windows::core::GUID,
    out: *mut *mut std::ffi::c_void,
) -> windows::core::Result<()> {
    use windows::Win32::Foundation::E_NOINTERFACE;
    use windows::Win32::System::Com::IClassFactory;
    use windows::core::{IUnknown, Interface};

    unsafe {
        if riid == &IClassFactory::IID {
            *out = object.into_interface::<IClassFactory>().into_raw();
            Ok(())
        } else if riid == &IUnknown::IID {
            *out = object.into_interface::<IUnknown>().into_raw();
            Ok(())
        } else {
            Err(windows::core::Error::from_hresult(E_NOINTERFACE))
        }
    }
}

#[cfg(windows)]
fn increment_active_com_object_count() {
    ACTIVE_COM_OBJECT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(windows)]
fn decrement_active_com_object_count() {
    let _ = ACTIVE_COM_OBJECT_COUNT.fetch_update(
        std::sync::atomic::Ordering::Relaxed,
        std::sync::atomic::Ordering::Relaxed,
        |count| (count > 0).then_some(count - 1),
    );
}

#[cfg(windows)]
pub fn create_hbitmap_from_windows_bitmap(
    bitmap: &WindowsThumbnailBitmap,
) -> Result<OwnedWindowsThumbnailHbitmap, windows::core::Error> {
    use std::ptr::{copy_nonoverlapping, null_mut};
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDIBSection, DIB_RGB_COLORS, DeleteObject,
        HGDIOBJ,
    };

    let mut info = BITMAPINFO::default();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: bitmap.width as i32,
        biHeight: -(bitmap.height as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        biSizeImage: bitmap.bgra.len() as u32,
        ..Default::default()
    };

    let mut bits = null_mut();
    let handle = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)? };
    if bits.is_null() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(handle));
        }
        return Err(windows::core::Error::from_thread());
    }

    unsafe {
        copy_nonoverlapping(bitmap.bgra.as_ptr(), bits.cast::<u8>(), bitmap.bgra.len());
    }

    Ok(OwnedWindowsThumbnailHbitmap { handle })
}

#[cfg(windows)]
impl Drop for OwnedWindowsThumbnailHbitmap {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::DeleteObject(
                    windows::Win32::Graphics::Gdi::HGDIOBJ::from(self.handle),
                );
            }
        }
    }
}

pub fn load_windows_thumbnail_bitmap(
    path: impl AsRef<Path>,
) -> Result<Option<WindowsThumbnailBitmap>, WindowsThumbnailLoadError> {
    let Some(thumbnail) =
        load_platform_thumbnail_rgba(path).map_err(WindowsThumbnailLoadError::Platform)?
    else {
        return Ok(None);
    };

    rgba_to_windows_bgra(&thumbnail)
        .map(Some)
        .map_err(WindowsThumbnailLoadError::Bitmap)
}

pub fn load_windows_thumbnail_bitmap_from_archive_bytes(
    bytes: &[u8],
) -> Result<Option<WindowsThumbnailBitmap>, WindowsThumbnailLoadError> {
    let Some(thumbnail) = load_platform_thumbnail_rgba_from_archive_bytes(bytes)
        .map_err(WindowsThumbnailLoadError::Platform)?
    else {
        return Ok(None);
    };

    rgba_to_windows_bgra(&thumbnail)
        .map(Some)
        .map_err(WindowsThumbnailLoadError::Bitmap)
}

pub fn rgba_to_windows_bgra(
    thumbnail: &PlatformThumbnailRgba,
) -> Result<WindowsThumbnailBitmap, WindowsThumbnailBitmapError> {
    let expected_len = thumbnail.width as usize * thumbnail.height as usize * 4;
    if thumbnail.rgba.len() != expected_len {
        return Err(WindowsThumbnailBitmapError::PixelLengthMismatch {
            width: thumbnail.width,
            height: thumbnail.height,
            expected_len,
            actual_len: thumbnail.rgba.len(),
        });
    }

    let mut bgra = Vec::with_capacity(expected_len);
    for pixel in thumbnail.rgba.chunks_exact(4) {
        bgra.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }

    Ok(WindowsThumbnailBitmap {
        width: thumbnail.width,
        height: thumbnail.height,
        bgra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rizum_platform_thumbnail::PlatformThumbnailSource;
    use std::fs;
    use std::io::{Cursor, Write};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn converts_rgba_pixels_to_windows_bgra_order() {
        let thumbnail = PlatformThumbnailRgba {
            source: PlatformThumbnailSource::QuickLookPreview,
            width: 2,
            height: 1,
            rgba: vec![255, 0, 16, 255, 10, 20, 30, 128],
        };

        let bitmap = rgba_to_windows_bgra(&thumbnail).unwrap();

        assert_eq!(bitmap.width, 2);
        assert_eq!(bitmap.height, 1);
        assert_eq!(bitmap.bgra, [16, 0, 255, 255, 30, 20, 10, 128]);
    }

    #[test]
    fn reports_mismatched_rgba_buffer_length_before_conversion() {
        let thumbnail = PlatformThumbnailRgba {
            source: PlatformThumbnailSource::QuickLookPreview,
            width: 2,
            height: 2,
            rgba: vec![255, 0, 0, 255],
        };

        let error = rgba_to_windows_bgra(&thumbnail).unwrap_err();

        assert_eq!(
            error,
            WindowsThumbnailBitmapError::PixelLengthMismatch {
                width: 2,
                height: 2,
                expected_len: 16,
                actual_len: 4,
            }
        );
    }

    #[test]
    fn loads_windows_bitmap_from_procreate_quicklook_png() {
        let path = temp_procreate_path("windows-bitmap");
        let png = png_with_rgba_pixels(1, 1, &[1, 2, 3, 4]);
        fs::write(
            &path,
            zip_with_files([("QuickLook/Preview.png", png.as_slice())]),
        )
        .unwrap();

        let bitmap = load_windows_thumbnail_bitmap(&path).unwrap().unwrap();

        assert_eq!(bitmap.width, 1);
        assert_eq!(bitmap.height, 1);
        assert_eq!(bitmap.bgra, [3, 2, 1, 4]);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn loads_windows_bitmap_from_procreate_archive_bytes() {
        let png = png_with_rgba_pixels(1, 2, &[1, 2, 3, 4, 5, 6, 7, 8]);
        let archive = zip_with_files([("QuickLook/Preview.png", png.as_slice())]);

        let bitmap = load_windows_thumbnail_bitmap_from_archive_bytes(&archive)
            .unwrap()
            .unwrap();

        assert_eq!(bitmap.width, 1);
        assert_eq!(bitmap.height, 2);
        assert_eq!(bitmap.bgra, [3, 2, 1, 4, 7, 6, 5, 8]);
    }

    #[test]
    fn returns_none_when_procreate_archive_has_no_quicklook_png() {
        let path = temp_procreate_path("no-quicklook");
        fs::write(
            &path,
            zip_with_files([("Document.archive", b"plist".as_slice())]),
        )
        .unwrap();

        let bitmap = load_windows_thumbnail_bitmap(&path).unwrap();

        assert_eq!(bitmap, None);

        fs::remove_file(path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn creates_hbitmap_from_windows_bgra_bitmap() {
        let bitmap = WindowsThumbnailBitmap {
            width: 1,
            height: 1,
            bgra: vec![3, 2, 1, 255],
        };

        let hbitmap = create_hbitmap_from_windows_bitmap(&bitmap).unwrap();

        assert!(!hbitmap.handle().is_invalid());
    }

    #[cfg(windows)]
    #[test]
    fn transfers_hbitmap_ownership_to_shell_caller() {
        use windows::Win32::Graphics::Gdi::{DeleteObject, HGDIOBJ};

        let bitmap = WindowsThumbnailBitmap {
            width: 1,
            height: 1,
            bgra: vec![3, 2, 1, 255],
        };
        let hbitmap = create_hbitmap_from_windows_bitmap(&bitmap).unwrap();

        let raw = hbitmap.into_raw();

        assert!(!raw.is_invalid());
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(raw));
        }
    }

    #[cfg(windows)]
    #[test]
    fn loads_shell_thumbnail_handoff_with_argb_alpha_type() {
        use windows::Win32::UI::Shell::WTSAT_ARGB;

        let path = temp_procreate_path("shell-handoff");
        let png = png_with_rgba_pixels(1, 1, &[1, 2, 3, 4]);
        fs::write(
            &path,
            zip_with_files([("QuickLook/Preview.png", png.as_slice())]),
        )
        .unwrap();

        let handoff = load_windows_shell_thumbnail(&path).unwrap().unwrap();

        assert_eq!(handoff.alpha_type, WTSAT_ARGB);
        assert!(!handoff.hbitmap.handle().is_invalid());

        fs::remove_file(path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn loads_shell_thumbnail_handoff_from_archive_bytes() {
        use windows::Win32::UI::Shell::WTSAT_ARGB;

        let png = png_with_rgba_pixels(1, 1, &[1, 2, 3, 4]);
        let archive = zip_with_files([("QuickLook/Preview.png", png.as_slice())]);

        let handoff = load_windows_shell_thumbnail_from_archive_bytes(&archive)
            .unwrap()
            .unwrap();

        assert_eq!(handoff.alpha_type, WTSAT_ARGB);
        assert!(!handoff.hbitmap.handle().is_invalid());
    }

    #[cfg(windows)]
    #[test]
    fn writes_shell_thumbnail_outputs_and_transfers_bitmap_ownership() {
        use windows::Win32::Graphics::Gdi::{DeleteObject, HBITMAP, HGDIOBJ};
        use windows::Win32::UI::Shell::{WTS_ALPHATYPE, WTSAT_ARGB};

        let bitmap = WindowsThumbnailBitmap {
            width: 1,
            height: 1,
            bgra: vec![3, 2, 1, 255],
        };
        let thumbnail = WindowsShellThumbnail {
            hbitmap: create_hbitmap_from_windows_bitmap(&bitmap).unwrap(),
            alpha_type: WTSAT_ARGB,
        };
        let mut raw = HBITMAP::default();
        let mut alpha = WTS_ALPHATYPE::default();

        unsafe {
            write_shell_thumbnail_outputs(thumbnail, &mut raw, &mut alpha).unwrap();
        }

        assert!(!raw.is_invalid());
        assert_eq!(alpha, WTSAT_ARGB);
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(raw));
        }
    }

    #[cfg(windows)]
    #[test]
    fn rejects_null_shell_thumbnail_output_pointers() {
        use windows::Win32::UI::Shell::WTSAT_ARGB;

        let bitmap = WindowsThumbnailBitmap {
            width: 1,
            height: 1,
            bgra: vec![3, 2, 1, 255],
        };
        let thumbnail = WindowsShellThumbnail {
            hbitmap: create_hbitmap_from_windows_bitmap(&bitmap).unwrap(),
            alpha_type: WTSAT_ARGB,
        };

        let error = unsafe {
            write_shell_thumbnail_outputs(thumbnail, std::ptr::null_mut(), std::ptr::null_mut())
        }
        .unwrap_err();

        assert_eq!(error.code(), windows::core::HRESULT(0x80004003_u32 as _));
    }

    #[cfg(windows)]
    #[test]
    fn creates_com_object_for_thumbnail_provider_and_file_initialization() {
        use windows::Win32::UI::Shell::IThumbnailProvider;
        use windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile;

        let object = create_windows_thumbnail_provider_com_object();

        let _thumbnail_provider: IThumbnailProvider = object.to_interface();
        let _file_initializer: IInitializeWithFile = object.to_interface();
    }

    #[cfg(windows)]
    #[test]
    fn initialized_com_provider_returns_thumbnail_outputs() {
        use windows::Win32::Graphics::Gdi::{DeleteObject, HBITMAP, HGDIOBJ};
        use windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile;
        use windows::Win32::UI::Shell::{IThumbnailProvider, WTS_ALPHATYPE, WTSAT_ARGB};
        use windows::core::PCWSTR;

        let path = temp_procreate_path("com-provider-thumbnail");
        let png = png_with_rgba_pixels(1, 1, &[1, 2, 3, 4]);
        fs::write(
            &path,
            zip_with_files([("QuickLook/Preview.png", png.as_slice())]),
        )
        .unwrap();
        let wide_path = path_to_wide_null(&path);
        let object = create_windows_thumbnail_provider_com_object();
        let initializer: IInitializeWithFile = object.to_interface();
        let provider: IThumbnailProvider = object.to_interface();
        let mut raw = HBITMAP::default();
        let mut alpha = WTS_ALPHATYPE::default();

        unsafe {
            initializer
                .Initialize(PCWSTR::from_raw(wide_path.as_ptr()), 0)
                .unwrap();
            provider.GetThumbnail(256, &mut raw, &mut alpha).unwrap();
        }

        assert!(!raw.is_invalid());
        assert_eq!(alpha, WTSAT_ARGB);
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(raw));
        }
        fs::remove_file(path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn dll_class_object_creates_thumbnail_provider_instances() {
        use std::ffi::c_void;
        use windows::Win32::System::Com::IClassFactory;
        use windows::Win32::UI::Shell::IThumbnailProvider;
        use windows::core::Interface;

        let mut factory_raw = std::ptr::null_mut::<c_void>();

        let result = unsafe {
            DllGetClassObject(
                &THUMBNAIL_PROVIDER_CLSID,
                &IClassFactory::IID,
                &mut factory_raw,
            )
        };

        assert_eq!(result, windows::Win32::Foundation::S_OK);
        assert!(!factory_raw.is_null());

        let factory = unsafe { IClassFactory::from_raw(factory_raw) };
        let provider: IThumbnailProvider = unsafe { factory.CreateInstance(None).unwrap() };

        assert!(!provider.as_raw().is_null());
    }

    fn temp_procreate_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rizum-windows-thumbnail-{label}-{nonce}.procreate"))
    }

    fn zip_with_files<const N: usize>(files: [(&str, &[u8]); N]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut archive = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (path, bytes) in files {
            archive.start_file(path, options).unwrap();
            archive.write_all(bytes).unwrap();
        }

        archive.finish().unwrap().into_inner()
    }

    fn png_with_rgba_pixels(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let image = image::RgbaImage::from_raw(width, height, rgba.to_vec()).unwrap();
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).unwrap();
        png.into_inner()
    }

    #[cfg(windows)]
    fn path_to_wide_null(path: &std::path::Path) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;

        path.as_os_str().encode_wide().chain([0]).collect()
    }
}

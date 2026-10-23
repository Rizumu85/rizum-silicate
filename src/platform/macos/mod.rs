//! Finder document delivery, independent of the presentation adapter.

use std::{
    ffi::{CStr, c_char},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock, mpsc},
};

type Wake = Arc<dyn Fn() + Send + Sync>;
static SENDER: OnceLock<mpsc::SyncSender<PathBuf>> = OnceLock::new();
static WAKE: Mutex<Option<Wake>> = Mutex::new(None);

unsafe extern "C" {
    fn silicate_listen_for_documents(callback: extern "C" fn(*const c_char) -> bool);
}

pub struct DocumentEvents(mpsc::Receiver<PathBuf>);

impl DocumentEvents {
    /// Call on the main thread before entering the native event loop.
    pub fn listen() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1024);
        SENDER
            .set(sender)
            .expect("Finder document listener installed twice");
        // The native observer waits for AppKit startup; it does not create NSApplication.
        unsafe { silicate_listen_for_documents(receive_document) };
        Self(receiver)
    }

    pub fn set_waker(&self, wake: Wake) {
        *WAKE.lock().unwrap_or_else(|error| error.into_inner()) = Some(wake);
    }

    pub fn take_pending(&self) -> Vec<PathBuf> {
        self.0.try_iter().collect()
    }
}

extern "C" fn receive_document(path: *const c_char) -> bool {
    // Never unwind through AppKit. The native sender owns the C string for this call.
    std::panic::catch_unwind(|| {
        if path.is_null() {
            return false;
        }
        let bytes = unsafe { CStr::from_ptr(path) }.to_bytes();
        let path = PathBuf::from(std::ffi::OsStr::from_bytes(bytes));
        let accepted = SENDER
            .get()
            .is_some_and(|sender| sender.try_send(path).is_ok());
        let wake = WAKE
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if accepted && let Some(wake) = wake {
            wake();
        }
        accepted
    })
    .unwrap_or(false)
}

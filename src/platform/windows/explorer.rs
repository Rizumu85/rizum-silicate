pub trait ShellChangeNotifier {
    fn notify_association_changed(&self);
}

pub fn notify_explorer_association_changed(notifier: &impl ShellChangeNotifier) {
    notifier.notify_association_changed();
}

#[cfg(windows)]
pub struct WindowsShellChangeNotifier;

#[cfg(windows)]
impl ShellChangeNotifier for WindowsShellChangeNotifier {
    fn notify_association_changed(&self) {
        use windows::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};

        unsafe {
            SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn notifies_explorer_that_file_associations_changed() {
        let notifier = FakeShellChangeNotifier::default();

        notify_explorer_association_changed(&notifier);

        assert!(notifier.association_changed.get());
    }

    #[derive(Default)]
    struct FakeShellChangeNotifier {
        association_changed: Cell<bool>,
    }

    impl ShellChangeNotifier for FakeShellChangeNotifier {
        fn notify_association_changed(&self) {
            self.association_changed.set(true);
        }
    }
}

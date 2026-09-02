pub trait ShellChangeNotifier {
    fn notify_association_changed(&self);
}

pub trait ProcessCommandRunner {
    fn run(&self, command: &ProcessCommand) -> Result<(), ExplorerRestartError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplorerRestartError {
    pub command: ProcessCommand,
    pub message: String,
}

pub fn notify_explorer_association_changed(notifier: &impl ShellChangeNotifier) {
    notifier.notify_association_changed();
}

pub fn restart_explorer(runner: &impl ProcessCommandRunner) -> Result<(), ExplorerRestartError> {
    runner.run(&ProcessCommand {
        program: "taskkill".to_owned(),
        args: vec![
            "/f".to_owned(),
            "/im".to_owned(),
            "explorer.exe".to_owned(),
        ],
    })?;
    runner.run(&ProcessCommand {
        program: "explorer.exe".to_owned(),
        args: Vec::new(),
    })
}

pub fn open_default_apps_settings(
    runner: &impl ProcessCommandRunner,
) -> Result<(), ExplorerRestartError> {
    // UserChoice is protected by Windows, so default selection must stay inside the system-owned
    // settings flow instead of being simulated with registry writes.
    runner.run(&ProcessCommand {
        program: "explorer.exe".to_owned(),
        args: vec!["ms-settings:defaultapps".to_owned()],
    })
}

#[cfg(windows)]
pub struct WindowsShellChangeNotifier;

#[cfg(windows)]
pub struct WindowsProcessCommandRunner;

#[cfg(windows)]
impl ShellChangeNotifier for WindowsShellChangeNotifier {
    fn notify_association_changed(&self) {
        use windows::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};

        unsafe {
            SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
        }
    }
}

#[cfg(windows)]
impl ProcessCommandRunner for WindowsProcessCommandRunner {
    fn run(&self, command: &ProcessCommand) -> Result<(), ExplorerRestartError> {
        use std::process::Command;

        let status = Command::new(&command.program)
            .args(&command.args)
            .status()
            .map_err(|err| ExplorerRestartError {
                command: command.clone(),
                message: err.to_string(),
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(ExplorerRestartError {
                command: command.clone(),
                message: format!("process exited with {status}"),
            })
        }
    }
}

#[cfg(windows)]
pub fn restart_current_explorer() -> Result<(), ExplorerRestartError> {
    restart_explorer(&WindowsProcessCommandRunner)
}

#[cfg(windows)]
pub fn open_current_default_apps_settings() -> Result<(), ExplorerRestartError> {
    open_default_apps_settings(&WindowsProcessCommandRunner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn notifies_explorer_that_file_associations_changed() {
        let notifier = FakeShellChangeNotifier::default();

        notify_explorer_association_changed(&notifier);

        assert!(notifier.association_changed.get());
    }

    #[test]
    fn restarts_explorer_by_killing_then_starting_it() {
        let runner = FakeProcessCommandRunner::default();

        restart_explorer(&runner).unwrap();

        assert_eq!(
            runner.commands.borrow().as_slice(),
            [
                ProcessCommand {
                    program: "taskkill".to_owned(),
                    args: vec![
                        "/f".to_owned(),
                        "/im".to_owned(),
                        "explorer.exe".to_owned(),
                    ],
                },
                ProcessCommand {
                    program: "explorer.exe".to_owned(),
                    args: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn does_not_start_explorer_when_kill_step_fails() {
        let runner = FakeProcessCommandRunner {
            fail_after_commands: Some(0),
            ..Default::default()
        };

        let error = restart_explorer(&runner).unwrap_err();

        assert_eq!(error.command.program, "taskkill");
        assert!(runner.commands.borrow().is_empty());
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

    #[derive(Default)]
    struct FakeProcessCommandRunner {
        commands: RefCell<Vec<ProcessCommand>>,
        fail_after_commands: Option<usize>,
    }

    impl ProcessCommandRunner for FakeProcessCommandRunner {
        fn run(&self, command: &ProcessCommand) -> Result<(), ExplorerRestartError> {
            if self
                .fail_after_commands
                .is_some_and(|limit| self.commands.borrow().len() >= limit)
            {
                return Err(ExplorerRestartError {
                    command: command.clone(),
                    message: "planned failure".to_owned(),
                });
            }

            self.commands.borrow_mut().push(command.clone());
            Ok(())
        }
    }
}

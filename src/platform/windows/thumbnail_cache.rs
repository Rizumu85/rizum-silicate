use super::explorer::{ExplorerRestartError, ProcessCommand, ProcessCommandRunner};
use std::path::{Path, PathBuf};

pub trait ThumbnailCacheDeleter {
    fn delete_thumbnail_cache_files(&self) -> Result<(), ThumbnailCacheDeleteError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailCacheDeleteError {
    pub path: Option<PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailCacheRefreshError {
    StopExplorer(ExplorerRestartError),
    Delete(ThumbnailCacheDeleteError),
    StartExplorer(ExplorerRestartError),
}

pub fn refresh_thumbnail_cache(
    runner: &impl ProcessCommandRunner,
    deleter: &impl ThumbnailCacheDeleter,
) -> Result<(), ThumbnailCacheRefreshError> {
    runner
        .run(&stop_explorer_command())
        .map_err(ThumbnailCacheRefreshError::StopExplorer)?;

    let delete_result = deleter.delete_thumbnail_cache_files();
    let start_result = runner.run(&start_explorer_command());

    match (delete_result, start_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), _) => Err(ThumbnailCacheRefreshError::Delete(err)),
        (Ok(()), Err(err)) => Err(ThumbnailCacheRefreshError::StartExplorer(err)),
    }
}

fn stop_explorer_command() -> ProcessCommand {
    ProcessCommand {
        program: "taskkill".to_owned(),
        args: vec!["/f".to_owned(), "/im".to_owned(), "explorer.exe".to_owned()],
    }
}

fn start_explorer_command() -> ProcessCommand {
    ProcessCommand {
        program: "explorer.exe".to_owned(),
        args: Vec::new(),
    }
}

#[cfg(windows)]
pub struct WindowsThumbnailCacheDeleter;

#[cfg(windows)]
impl ThumbnailCacheDeleter for WindowsThumbnailCacheDeleter {
    fn delete_thumbnail_cache_files(&self) -> Result<(), ThumbnailCacheDeleteError> {
        let cache_dir = thumbnail_cache_directory()?;
        delete_thumbnail_cache_files_in(&cache_dir)
    }
}

#[cfg(windows)]
pub fn refresh_current_thumbnail_cache() -> Result<(), ThumbnailCacheRefreshError> {
    use super::explorer::WindowsProcessCommandRunner;

    refresh_thumbnail_cache(&WindowsProcessCommandRunner, &WindowsThumbnailCacheDeleter)
}

#[cfg(windows)]
fn thumbnail_cache_directory() -> Result<PathBuf, ThumbnailCacheDeleteError> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("Microsoft").join("Windows").join("Explorer"))
        .ok_or_else(|| ThumbnailCacheDeleteError {
            path: None,
            message: "LOCALAPPDATA is not set".to_owned(),
        })
}

#[cfg(windows)]
fn delete_thumbnail_cache_files_in(cache_dir: &Path) -> Result<(), ThumbnailCacheDeleteError> {
    let entries = std::fs::read_dir(cache_dir).map_err(|err| ThumbnailCacheDeleteError {
        path: Some(cache_dir.to_owned()),
        message: err.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|err| ThumbnailCacheDeleteError {
            path: Some(cache_dir.to_owned()),
            message: err.to_string(),
        })?;
        let path = entry.path();

        if is_windows_thumbnail_cache_file(&path) {
            std::fs::remove_file(&path).map_err(|err| ThumbnailCacheDeleteError {
                path: Some(path),
                message: err.to_string(),
            })?;
        }
    }

    Ok(())
}

#[cfg(windows)]
fn is_windows_thumbnail_cache_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    file_name
        .to_ascii_lowercase()
        .strip_prefix("thumbcache_")
        .is_some_and(|tail| tail.ends_with(".db"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn refreshes_thumbnail_cache_between_explorer_stop_and_start() {
        let runner = FakeProcessCommandRunner::default();
        let deleter = FakeThumbnailCacheDeleter::default();

        refresh_thumbnail_cache(&runner, &deleter).unwrap();

        assert_eq!(
            runner.commands.borrow().as_slice(),
            [
                ProcessCommand {
                    program: "taskkill".to_owned(),
                    args: vec!["/f".to_owned(), "/im".to_owned(), "explorer.exe".to_owned(),],
                },
                ProcessCommand {
                    program: "explorer.exe".to_owned(),
                    args: Vec::new(),
                },
            ]
        );
        assert_eq!(deleter.delete_calls.get(), 1);
    }

    #[test]
    fn still_restarts_explorer_when_thumbnail_cache_delete_fails() {
        let runner = FakeProcessCommandRunner::default();
        let deleter = FakeThumbnailCacheDeleter {
            error: Some(ThumbnailCacheDeleteError {
                path: Some(PathBuf::from(
                    r"C:\Users\Rizum\AppData\Local\Microsoft\Windows\Explorer\thumbcache_256.db",
                )),
                message: "planned failure".to_owned(),
            }),
            ..Default::default()
        };

        let error = refresh_thumbnail_cache(&runner, &deleter).unwrap_err();

        assert_eq!(deleter.delete_calls.get(), 1);
        assert!(matches!(error, ThumbnailCacheRefreshError::Delete(_)));
        assert_eq!(
            runner
                .commands
                .borrow()
                .iter()
                .map(|command| command.program.as_str())
                .collect::<Vec<_>>(),
            vec!["taskkill", "explorer.exe"]
        );
    }

    #[cfg(windows)]
    #[test]
    fn recognizes_only_windows_thumbnail_cache_database_files() {
        assert!(is_windows_thumbnail_cache_file(Path::new(
            r"C:\Users\Rizum\AppData\Local\Microsoft\Windows\Explorer\thumbcache_256.db"
        )));
        assert!(is_windows_thumbnail_cache_file(Path::new(
            r"C:\Users\Rizum\AppData\Local\Microsoft\Windows\Explorer\THUMBCACHE_IDX.DB"
        )));
        assert!(!is_windows_thumbnail_cache_file(Path::new(
            r"C:\Users\Rizum\AppData\Local\Microsoft\Windows\Explorer\iconcache_256.db"
        )));
        assert!(!is_windows_thumbnail_cache_file(Path::new(
            r"C:\Users\Rizum\AppData\Local\Microsoft\Windows\Explorer\thumbcache_256.txt"
        )));
    }

    #[derive(Default)]
    struct FakeProcessCommandRunner {
        commands: RefCell<Vec<ProcessCommand>>,
    }

    impl ProcessCommandRunner for FakeProcessCommandRunner {
        fn run(&self, command: &ProcessCommand) -> Result<(), ExplorerRestartError> {
            self.commands.borrow_mut().push(command.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeThumbnailCacheDeleter {
        delete_calls: Cell<usize>,
        error: Option<ThumbnailCacheDeleteError>,
    }

    impl ThumbnailCacheDeleter for FakeThumbnailCacheDeleter {
        fn delete_thumbnail_cache_files(&self) -> Result<(), ThumbnailCacheDeleteError> {
            self.delete_calls.set(self.delete_calls.get() + 1);
            match &self.error {
                Some(error) => Err(error.clone()),
                None => Ok(()),
            }
        }
    }
}

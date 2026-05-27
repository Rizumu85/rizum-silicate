use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegToolStatus {
    pub source: FfmpegToolSource,
    pub executable_path: Option<PathBuf>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegCommandRunError {
    pub command: FfmpegCommand,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfmpegToolSource {
    Bundled,
    System,
    Missing,
}

pub trait FfmpegCommandRunner {
    fn run(&mut self, command: &FfmpegCommand) -> Result<(), FfmpegCommandRunError>;
}

pub trait FilePresenceReader {
    fn path_exists(&self, path: &Path) -> bool;
}

pub trait PathLookup {
    fn find_tool(&self, tool_name: &str) -> Option<PathBuf>;
}

pub fn detect_ffmpeg_tool_status(
    bundled_path: &Path,
    files: &impl FilePresenceReader,
    path_lookup: &impl PathLookup,
) -> FfmpegToolStatus {
    if files.path_exists(bundled_path) {
        return FfmpegToolStatus {
            source: FfmpegToolSource::Bundled,
            executable_path: Some(bundled_path.to_owned()),
            detail: bundled_path.to_string_lossy().into_owned(),
        };
    }

    if let Some(system_path) = path_lookup.find_tool("ffmpeg") {
        return FfmpegToolStatus {
            source: FfmpegToolSource::System,
            detail: system_path.to_string_lossy().into_owned(),
            executable_path: Some(system_path),
        };
    }

    FfmpegToolStatus {
        source: FfmpegToolSource::Missing,
        executable_path: None,
        detail: "ffmpeg not found".to_owned(),
    }
}

pub fn bundled_ffmpeg_path_for_app_executable(app_executable: &Path) -> PathBuf {
    app_executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("tools")
        .join(ffmpeg_executable_name())
}

pub fn build_ffmpeg_version_probe_command(executable_path: impl Into<PathBuf>) -> FfmpegCommand {
    FfmpegCommand {
        program: executable_path.into(),
        args: vec!["-hide_banner".to_owned(), "-version".to_owned()],
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct ProcessFfmpegCommandRunner;

#[cfg(not(target_arch = "wasm32"))]
impl FfmpegCommandRunner for ProcessFfmpegCommandRunner {
    fn run(&mut self, command: &FfmpegCommand) -> Result<(), FfmpegCommandRunError> {
        use std::process::Command;

        let status = Command::new(&command.program)
            .args(&command.args)
            .status()
            .map_err(|err| FfmpegCommandRunError {
                command: command.clone(),
                message: err.to_string(),
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(FfmpegCommandRunError {
                command: command.clone(),
                message: format!("process exited with {status}"),
            })
        }
    }
}

pub struct OsFilePresenceReader;

impl FilePresenceReader for OsFilePresenceReader {
    fn path_exists(&self, path: &Path) -> bool {
        path.is_file()
    }
}

pub struct EnvironmentPathLookup;

impl PathLookup for EnvironmentPathLookup {
    fn find_tool(&self, tool_name: &str) -> Option<PathBuf> {
        find_tool_on_path(tool_name)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn detect_current_ffmpeg_tool_status() -> Result<FfmpegToolStatus, std::io::Error> {
    let app_executable = std::env::current_exe()?;
    let bundled_path = bundled_ffmpeg_path_for_app_executable(&app_executable);

    Ok(detect_ffmpeg_tool_status(
        &bundled_path,
        &OsFilePresenceReader,
        &EnvironmentPathLookup,
    ))
}

fn find_tool_on_path(tool_name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let executable_names = executable_names_for_lookup(tool_name);

    std::env::split_paths(&path)
        .flat_map(|dir| executable_names.iter().map(move |name| dir.join(name)))
        .find(|path| path.is_file())
}

fn executable_names_for_lookup(tool_name: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        if tool_name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("exe"))
        {
            vec![tool_name.to_owned()]
        } else {
            vec![format!("{tool_name}.exe"), tool_name.to_owned()]
        }
    }

    #[cfg(not(windows))]
    {
        vec![tool_name.to_owned()]
    }
}

fn ffmpeg_executable_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::path::{Path, PathBuf};

    #[test]
    fn reports_bundled_ffmpeg_before_system_ffmpeg() {
        let bundled_path = PathBuf::from(r"C:\Rizum\tools\ffmpeg.exe");
        let files = FakeFilePresenceReader::new([bundled_path.clone()]);
        let path_lookup =
            FakePathLookup::new([("ffmpeg", PathBuf::from(r"C:\Windows\ffmpeg.exe"))]);

        let status = detect_ffmpeg_tool_status(&bundled_path, &files, &path_lookup);

        assert_eq!(
            status,
            FfmpegToolStatus {
                source: FfmpegToolSource::Bundled,
                executable_path: Some(bundled_path.clone()),
                detail: bundled_path.to_string_lossy().into_owned(),
            }
        );
    }

    #[test]
    fn falls_back_to_system_ffmpeg_when_bundled_tool_is_missing() {
        let bundled_path = PathBuf::from(r"C:\Rizum\tools\ffmpeg.exe");
        let system_path = PathBuf::from(r"C:\Program Files\ffmpeg\bin\ffmpeg.exe");
        let files = FakeFilePresenceReader::new([]);
        let path_lookup = FakePathLookup::new([("ffmpeg", system_path.clone())]);

        let status = detect_ffmpeg_tool_status(&bundled_path, &files, &path_lookup);

        assert_eq!(
            status,
            FfmpegToolStatus {
                source: FfmpegToolSource::System,
                executable_path: Some(system_path.clone()),
                detail: system_path.to_string_lossy().into_owned(),
            }
        );
    }

    #[test]
    fn reports_missing_when_no_bundled_or_system_ffmpeg_exists() {
        let bundled_path = PathBuf::from(r"C:\Rizum\tools\ffmpeg.exe");
        let files = FakeFilePresenceReader::new([]);
        let path_lookup = FakePathLookup::new([]);

        let status = detect_ffmpeg_tool_status(&bundled_path, &files, &path_lookup);

        assert_eq!(
            status,
            FfmpegToolStatus {
                source: FfmpegToolSource::Missing,
                executable_path: None,
                detail: "ffmpeg not found".to_owned(),
            }
        );
    }

    #[test]
    fn derives_bundled_ffmpeg_path_next_to_the_app_executable() {
        let app_executable = if cfg!(windows) {
            PathBuf::from(r"C:\Rizum\silicate.exe")
        } else {
            PathBuf::from("/Applications/Rizum Silicate.app/Contents/MacOS/silicate")
        };

        let bundled_path = bundled_ffmpeg_path_for_app_executable(&app_executable);

        assert_eq!(
            bundled_path,
            app_executable
                .parent()
                .unwrap()
                .join("tools")
                .join(ffmpeg_executable_name())
        );
    }

    #[test]
    fn builds_probe_command_from_injected_ffmpeg_path() {
        let executable_path = PathBuf::from(r"C:\Rizum\tools\ffmpeg.exe");

        let command = build_ffmpeg_version_probe_command(&executable_path);

        assert_eq!(
            command,
            FfmpegCommand {
                program: executable_path,
                args: vec!["-hide_banner".to_owned(), "-version".to_owned()],
            }
        );
    }

    #[derive(Default)]
    struct FakeFilePresenceReader {
        existing_paths: HashSet<PathBuf>,
    }

    impl FakeFilePresenceReader {
        fn new<const N: usize>(paths: [PathBuf; N]) -> Self {
            Self {
                existing_paths: paths.into_iter().collect(),
            }
        }
    }

    impl FilePresenceReader for FakeFilePresenceReader {
        fn path_exists(&self, path: &Path) -> bool {
            self.existing_paths.contains(path)
        }
    }

    struct FakePathLookup {
        tools: HashMap<String, PathBuf>,
    }

    impl FakePathLookup {
        fn new<const N: usize>(tools: [(&str, PathBuf); N]) -> Self {
            Self {
                tools: tools
                    .into_iter()
                    .map(|(name, path)| (name.to_owned(), path))
                    .collect(),
            }
        }
    }

    impl PathLookup for FakePathLookup {
        fn find_tool(&self, tool_name: &str) -> Option<PathBuf> {
            self.tools.get(tool_name).cloned()
        }
    }
}

use super::{
    animation::AnimationExportProgress,
    animation_codec::AnimationExportFormat,
    archived_video::ArchivedVideoExportMode,
    still::{StillExportBackground, StillExportFormat},
};
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchFormat {
    Still(StillExportFormat),
    Animation(AnimationExportFormat),
    ArchivedVideo(ArchivedVideoExportMode),
}
impl BatchFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Still(format) => format.label(),
            Self::Animation(format) => format.label(),
            Self::ArchivedVideo(ArchivedVideoExportMode::FullLength) => "Time-lapse (full)",
            Self::ArchivedVideo(ArchivedVideoExportMode::Preview30Seconds) => {
                "Time-lapse (30 seconds)"
            }
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Still(format) => format.extension(),
            Self::Animation(format) => format.extension(),
            Self::ArchivedVideo(_) => "mp4",
        }
    }
}
#[derive(Clone)]
pub struct BatchOptions {
    pub format: BatchFormat,
    pub background: StillExportBackground,
    pub repeat_holds: bool,
    pub parallelism: usize,
}
#[derive(Clone, Debug)]
pub enum BatchStatus {
    Queued,
    Running,
    Succeeded,
    Failed(String),
    Cancelled,
}
#[derive(Clone)]
pub struct BatchItem {
    pub source: PathBuf,
    pub output: PathBuf,
    pub status: BatchStatus,
    pub progress: Arc<AnimationExportProgress>,
}
pub struct BatchQueue {
    pub options: BatchOptions,
    pub running: AtomicBool,
    cancelled: AtomicBool,
    items: Mutex<Vec<BatchItem>>,
}
impl BatchQueue {
    pub fn plan(
        sources: &[PathBuf],
        directory: &Path,
        template: &str,
        options: BatchOptions,
    ) -> io::Result<Arc<Self>> {
        if sources.is_empty() || sources.len() > 10_000 {
            return Err(io::Error::other("Choose 1–10,000 source files"));
        }
        if !(1..=2).contains(&options.parallelism) {
            return Err(io::Error::other("Choose one or two concurrent files"));
        }
        if options.background.is_transparent()
            && (options.format == BatchFormat::Still(StillExportFormat::Jpeg)
                || matches!(options.format, BatchFormat::Animation(format) if !format.supports_alpha()))
        {
            return Err(io::Error::other(
                "This format requires the document background",
            ));
        }
        let directory = directory.canonicalize()?;
        if !directory.is_dir() {
            return Err(io::Error::other("Choose an output folder"));
        }
        let mut names = HashSet::new();
        let mut items = Vec::new();
        for (index, source) in sources.iter().enumerate() {
            let name = output_name(source, index + 1, template, options.format.extension())?;
            if !names.insert(name.to_lowercase()) {
                return Err(io::Error::other(
                    "Duplicate output names; include {index} in the naming template",
                ));
            }
            items.push(BatchItem {
                source: source.clone(),
                output: directory.join(name),
                status: BatchStatus::Queued,
                progress: Arc::default(),
            });
        }
        Ok(Arc::new(Self {
            options,
            running: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            items: Mutex::new(items),
        }))
    }
    pub fn with_items<R>(&self, read: impl FnOnce(&[BatchItem]) -> R) -> R {
        read(&self.items.lock().unwrap_or_else(|e| e.into_inner()))
    }
    pub fn begin(&self, retry: bool) -> bool {
        if self.running.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.cancelled.store(false, Ordering::Release);
        if retry {
            for item in self
                .items
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter_mut()
            {
                if matches!(item.status, BatchStatus::Failed(_) | BatchStatus::Cancelled) {
                    item.status = BatchStatus::Queued;
                    item.progress = Arc::default();
                }
            }
        }
        true
    }
    pub fn claim(&self) -> Option<(usize, BatchItem)> {
        let mut items = self.items.lock().unwrap_or_else(|e| e.into_inner());
        if self.cancelled.load(Ordering::Acquire) {
            return None;
        }
        let (index, item) = items
            .iter_mut()
            .enumerate()
            .find(|(_, item)| matches!(item.status, BatchStatus::Queued))?;
        item.status = BatchStatus::Running;
        Some((index, item.clone()))
    }
    pub fn complete(&self, index: usize, result: io::Result<()>) {
        let mut items = self.items.lock().unwrap_or_else(|e| e.into_inner());
        let item = &mut items[index];
        item.status = match result {
            Ok(()) => BatchStatus::Succeeded,
            Err(error)
                if error.kind() == io::ErrorKind::Interrupted
                    || item.progress.cancelled.load(Ordering::Relaxed) =>
            {
                BatchStatus::Cancelled
            }
            Err(error) => BatchStatus::Failed(error.to_string()),
        };
    }
    pub fn cancel(&self) {
        let mut items = self.items.lock().unwrap_or_else(|e| e.into_inner());
        self.cancelled.store(true, Ordering::Release);
        for item in items.iter_mut() {
            item.progress.cancelled.store(true, Ordering::Relaxed);
            if matches!(item.status, BatchStatus::Queued) {
                item.status = BatchStatus::Cancelled;
            }
        }
    }
    pub fn finish(&self) {
        let mut items = self.items.lock().unwrap_or_else(|e| e.into_inner());
        for item in items.iter_mut() {
            if matches!(item.status, BatchStatus::Running | BatchStatus::Queued) {
                item.status = BatchStatus::Failed("Export worker stopped unexpectedly".into());
            }
        }
        self.running.store(false, Ordering::Release);
    }
}
pub fn discover_sources(roots: Vec<PathBuf>, recursive: bool) -> io::Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    let mut files = HashSet::new();
    let mut seen = HashSet::new();
    let mut visited = 0;
    for root in roots {
        if root.is_dir() {
            directories.push(root);
        } else if is_procreate(&root) {
            files.insert(root.canonicalize()?);
        }
    }
    while let Some(directory) = directories.pop() {
        if !seen.insert(directory.canonicalize()?) {
            continue;
        }
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            visited += 1;
            if visited > 200_000 || files.len() >= 10_000 {
                return Err(io::Error::other(
                    "Selection too large; choose a smaller folder",
                ));
            }
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() && recursive {
                directories.push(entry.path());
            } else if kind.is_file() && is_procreate(&entry.path()) {
                files.insert(entry.path().canonicalize()?);
            }
        }
    }
    let mut files: Vec<_> = files.into_iter().collect();
    files.sort();
    if files.is_empty() {
        return Err(io::Error::other(
            "No Procreate files found in this selection",
        ));
    }
    if files.len() > 10_000 {
        return Err(io::Error::other("Choose at most 10,000 files"));
    }
    Ok(files)
}
fn is_procreate(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("procreate"))
}
fn safe_part(value: &str) -> String {
    value
        .chars()
        .take(80)
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*{}".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect()
}
pub fn output_name(
    source: &Path,
    index: usize,
    template: &str,
    extension: &str,
) -> io::Result<String> {
    let name = safe_part(&source.file_stem().unwrap_or_default().to_string_lossy());
    let parent = safe_part(
        &source
            .parent()
            .and_then(Path::file_name)
            .unwrap_or_default()
            .to_string_lossy(),
    );
    let stem = template
        .replace("{name}", &name)
        .replace("{parent}", &parent)
        .replace("{index}", &format!("{index:04}"));
    let reserved = stem
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if stem.is_empty()
        || stem.ends_with([' ', '.'])
        || stem
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*{}".contains(c))
        || matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((reserved.starts_with("COM") || reserved.starts_with("LPT"))
            && reserved.len() == 4
            && matches!(reserved.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(io::Error::other(
            "Use a filename template with {name}, {parent}, or {index}; paths and reserved names are not allowed",
        ));
    }
    let filename = if extension.is_empty() {
        stem
    } else {
        format!("{stem}.{extension}")
    };
    if filename.len() > 200 {
        return Err(io::Error::other("Output filename is too long"));
    }
    Ok(filename)
}

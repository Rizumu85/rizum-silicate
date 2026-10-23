use rizum_platform_thumbnail::{PlatformThumbnailRgba, load_platform_thumbnail_rgba_at_size};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};

pub struct Entry {
    pub path: PathBuf,
    pub directory: bool,
    pub name: String,
    pub search_name: String,
}

pub enum Request {
    Folder(PathBuf),
    Thumbnail(PathBuf),
}

pub enum ResultKind {
    Folder(PathBuf, Result<Vec<Entry>, String>),
    Thumbnail(PathBuf, Result<Option<PlatformThumbnailRgba>, String>),
}

pub struct Worker {
    pub generation: Arc<AtomicU64>,
    requests: mpsc::SyncSender<(u64, Request)>,
    results: mpsc::Receiver<(u64, ResultKind)>,
}

impl Worker {
    pub fn new() -> std::io::Result<Self> {
        let (requests, receiver) = mpsc::sync_channel(1);
        let (sender, results) = mpsc::sync_channel(1);
        let generation = Arc::new(AtomicU64::new(0));
        let current = generation.clone();
        std::thread::Builder::new()
            .name("artwork-browser".into())
            .spawn(move || {
                while let Ok((revision, request)) = receiver.recv() {
                    let result = match request {
                        Request::Folder(path) => {
                            let result = scan(&path, revision, &current);
                            ResultKind::Folder(path, result)
                        }
                        Request::Thumbnail(path) => {
                            let result = if current.load(Ordering::Relaxed) != revision {
                                Ok(None)
                            } else {
                                load_platform_thumbnail_rgba_at_size(&path, 256)
                                    .map_err(|error| format!("{error:?}"))
                            };
                            ResultKind::Thumbnail(path, result)
                        }
                    };
                    if sender.send((revision, result)).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            generation,
            requests,
            results,
        })
    }

    pub fn request(&self, revision: u64, request: Request) -> bool {
        self.requests.try_send((revision, request)).is_ok()
    }

    pub fn poll(&self) -> Result<(u64, ResultKind), mpsc::TryRecvError> {
        self.results.try_recv()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }
}

fn scan(path: &PathBuf, revision: u64, current: &AtomicU64) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for (count, entry) in std::fs::read_dir(path)
        .map_err(|error| error.to_string())?
        .enumerate()
    {
        if current.load(Ordering::Relaxed) != revision {
            return Err("Folder changed".into());
        }
        if count >= 200_000 || entries.len() >= 10_000 {
            return Err("This folder is too large to browse. Choose a smaller folder.".into());
        }
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        let path = entry.path();
        if kind.is_dir()
            || (kind.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("procreate")))
        {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let search_name = name.to_lowercase();
            entries.push(Entry {
                path,
                directory: kind.is_dir(),
                name,
                search_name,
            });
        }
    }
    entries.sort_by(|left, right| {
        right
            .directory
            .cmp(&left.directory)
            .then_with(|| left.search_name.cmp(&right.search_name))
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(entries)
}

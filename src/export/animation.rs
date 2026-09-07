use silicate_runtime::{
    AnimationPlaybackDirection, AnimationPlaybackMode, DocumentSnapshot, LayerId,
};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

#[derive(Debug, Clone, Copy)]
pub struct AnimationExportFrame {
    pub source: LayerId,
    pub slots: u64,
}

#[derive(Debug)]
pub struct AnimationExportPlan {
    pub frames: Vec<AnimationExportFrame>,
    pub frame_rate: u32,
    pub total_slots: u64,
}

impl AnimationExportPlan {
    pub fn new(snapshot: &DocumentSnapshot) -> io::Result<Self> {
        let animation = snapshot
            .animation
            .ok_or_else(|| io::Error::other("Document has no animation"))?;
        let playback = snapshot
            .animation_playback
            .ok_or_else(|| io::Error::other("Document has no animation playback"))?;
        let mut frames: Vec<_> = snapshot
            .animation_frame_sources()
            .map(|frame| AnimationExportFrame {
                source: frame.source_layer_id,
                slots: u64::from(frame.hold_duration) + 1,
            })
            .collect();
        if frames.is_empty() || !(1..=60).contains(&animation.frame_rate) {
            return Err(io::Error::other(
                "Animation needs visible frames and a frame rate from 1 to 60",
            ));
        }
        if playback.direction == AnimationPlaybackDirection::Reverse {
            frames.reverse();
        }
        let forward_slots: u64 = frames.iter().map(|frame| frame.slots).sum();
        if playback.mode == AnimationPlaybackMode::PingPong && forward_slots > 2 {
            // Playback turns at end slots, not drawing boundaries. Trim one slot at
            // each end of the return leg to preserve held endpoint timing.
            let mut returning = frames.clone();
            returning.reverse();
            returning.first_mut().unwrap().slots -= 1;
            returning.last_mut().unwrap().slots -= 1;
            frames.extend(returning.into_iter().filter(|frame| frame.slots > 0));
        }
        let total_slots = frames.iter().map(|frame| frame.slots).sum();
        Ok(Self {
            frames,
            frame_rate: animation.frame_rate,
            total_slots,
        })
    }

    pub fn validate_sequence_size(
        &self,
        width: u32,
        height: u32,
        repeat_holds: bool,
    ) -> io::Result<u64> {
        let files = if repeat_holds {
            self.total_slots
        } else {
            self.frames.len() as u64
        };
        // Budget against uncompressed pixels before allocating or writing any output.
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_mul(files));
        if files > 100_000 || bytes.is_none_or(|n| n > 16 * 1024 * 1024 * 1024) {
            return Err(io::Error::other(
                "Animation sequence exceeds the 100,000 file or 16 GiB pixel budget",
            ));
        }
        Ok(files)
    }
}

#[derive(Debug, Default)]
pub struct AnimationExportProgress {
    pub completed: AtomicU64,
    pub total: AtomicU64,
    pub cancelled: AtomicBool,
    pub running: AtomicBool,
    pub encoding: AtomicBool,
}

impl AnimationExportProgress {
    pub fn check_cancelled(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Animation export cancelled",
            ))
        } else {
            Ok(())
        }
    }
}

/// Owns only a newly created directory; incomplete sequences are removed on failure.
pub struct PngSequenceWriter {
    path: PathBuf,
    manifest: Option<io::BufWriter<fs::File>>,
    next: u64,
    frame_rate: u32,
    finished: bool,
}

impl PngSequenceWriter {
    pub fn create(path: &Path, frame_rate: u32) -> io::Result<Self> {
        if !(1..=60).contains(&frame_rate) {
            return Err(io::Error::other("Sequence frame rate must be from 1 to 60"));
        }
        fs::create_dir(path)?;
        let mut writer = Self {
            path: path.to_owned(),
            manifest: None,
            next: 1,
            frame_rate,
            finished: false,
        };
        let mut manifest = io::BufWriter::new(fs::File::create(path.join("timing.csv"))?);
        writeln!(manifest, "file,duration_numerator,duration_denominator")?;
        writer.manifest = Some(manifest);
        Ok(writer)
    }

    pub fn write_frame(
        &mut self,
        image: &image::RgbaImage,
        slots: u64,
        repeat_holds: bool,
        progress: &AnimationExportProgress,
    ) -> io::Result<()> {
        let copies = if repeat_holds { slots } else { 1 };
        let first = self.path.join(format!("frame-{:06}.png", self.next));
        for copy in 0..copies {
            progress.check_cancelled()?;
            let name = format!("frame-{:06}.png", self.next);
            let path = self.path.join(&name);
            if copy == 0 {
                image.save(&path).map_err(io::Error::other)?;
            } else {
                fs::copy(&first, &path)?;
            }
            writeln!(
                self.manifest.as_mut().unwrap(),
                "{name},{},{}",
                if repeat_holds { 1 } else { slots },
                self.frame_rate
            )?;
            self.next += 1;
            progress.completed.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<PathBuf> {
        if self.next == 1 {
            return Err(io::Error::other("Cannot finish an empty sequence"));
        }
        self.manifest.take().unwrap().flush()?;
        self.finished = true;
        Ok(self.path.clone())
    }
}

impl Drop for PngSequenceWriter {
    fn drop(&mut self) {
        self.manifest.take();
        if !self.finished {
            if let Err(error) = fs::remove_dir_all(&self.path) {
                log::warn!(
                    "Could not remove incomplete animation sequence {}: {error}",
                    self.path.display()
                );
            }
        }
    }
}

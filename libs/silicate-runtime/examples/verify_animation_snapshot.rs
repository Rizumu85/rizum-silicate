use silica::ProcreateFile;
use silicate_runtime::{DocumentCommand, DocumentRuntime};
use std::{env, error::Error, fs, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: verify_animation_snapshot <document.procreate>")?;
    let bytes = fs::read(&path)?;
    let document = ProcreateFile::open(&bytes)?;
    let parser_frames = document
        .animation_frame_sources()
        .map(|frame| (frame.hierarchy_id.get(), frame.hold_duration))
        .collect::<Vec<_>>();

    let mut runtime = DocumentRuntime::new();
    let opened = runtime.open_document(&document)?.value;
    let animation = opened
        .animation
        .as_ref()
        .ok_or("runtime snapshot does not contain Animation Assist settings")?;
    let runtime_frames = opened
        .animation_frame_sources()
        .map(|frame| {
            (
                frame.source_layer_id.hierarchy_id().get(),
                frame.hold_duration,
            )
        })
        .collect::<Vec<_>>();
    if runtime_frames != parser_frames {
        return Err("runtime frame identity or ordering differs from the parser".into());
    }

    let initial_frame_count = runtime_frames.len();
    let initial_slot_count = opened.animation_timeline_slots().count();
    let visibility_projection = if let Some(first_frame) = opened.animation_frame_sources().next() {
        runtime.dispatch(DocumentCommand::SetLayerVisibility {
            document_id: opened.document_id,
            layer_id: first_frame.source_layer_id,
            visible: false,
        })?;
        let hidden = runtime.snapshot(opened.document_id)?;
        if hidden.animation_frame_sources().count() + 1 != initial_frame_count {
            return Err("hiding a top-level source did not remove exactly one frame".into());
        }
        if hidden.animation_timeline_slots().count() + first_frame.hold_duration as usize + 1
            != initial_slot_count
        {
            return Err("hiding a held source did not remove all of its timeline slots".into());
        }

        runtime.dispatch(DocumentCommand::SetLayerVisibility {
            document_id: opened.document_id,
            layer_id: first_frame.source_layer_id,
            visible: true,
        })?;
        let restored = runtime.snapshot(opened.document_id)?;
        if restored.animation_frame_sources().count() != initial_frame_count
            || restored.animation_timeline_slots().count() != initial_slot_count
        {
            return Err("restoring source visibility did not restore the timeline".into());
        }
        "verified"
    } else {
        "skipped_no_visible_sources"
    };

    println!("animation_snapshot=silicate_runtime_v1");
    println!("file={}", path.display());
    println!("assist_enabled={:?}", animation.assist_enabled());
    println!("frame_rate={}", animation.frame_rate);
    println!("frame_sources={initial_frame_count}");
    println!("timeline_slots={initial_slot_count}");
    println!("visibility_projection={visibility_projection}");
    Ok(())
}

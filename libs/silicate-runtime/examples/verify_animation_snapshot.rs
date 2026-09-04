use silica::ProcreateFile;
use silicate_runtime::{
    AnimationOnionSkinDirection, AnimationOnionSkinSettings, DocumentCommand, DocumentRuntime,
    HistoryGroupId, RuntimeEvent,
};
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
    let initial_playback = opened
        .animation_playback
        .ok_or("runtime snapshot does not contain Animation Assist playback state")?;
    if initial_playback.slot_count != initial_slot_count as u64 {
        return Err("initial playback slot count differs from the derived timeline".into());
    }
    runtime.dispatch(DocumentCommand::SetAnimationPlaybackActive {
        document_id: opened.document_id,
        active: true,
    })?;
    let first_settings = AnimationOnionSkinSettings {
        frame_count: 2,
        opacity: 0.75,
        blend_primary_frame: false,
    };
    let final_settings = AnimationOnionSkinSettings {
        opacity: 0.8,
        blend_primary_frame: true,
        ..first_settings
    };
    let history_group = HistoryGroupId::new(1);
    runtime.dispatch_grouped(
        DocumentCommand::SetAnimationOnionSkinSettings {
            document_id: opened.document_id,
            settings: first_settings,
        },
        history_group,
    )?;
    let settings_update = runtime.dispatch_grouped(
        DocumentCommand::SetAnimationOnionSkinSettings {
            document_id: opened.document_id,
            settings: final_settings,
        },
        history_group,
    )?;
    if !settings_update.events.iter().all(|event| {
        matches!(
            event,
            RuntimeEvent::AnimationOnionSkinSettingsChanged { settings, .. }
                if *settings == final_settings
        )
    }) {
        return Err("onion skin settings emitted an unexpected runtime event".into());
    }
    let configured = runtime.snapshot(opened.document_id)?;
    if configured
        .animation
        .is_none_or(|animation| animation.onion_skin_settings() != final_settings)
    {
        return Err("onion skin settings did not update the document snapshot".into());
    }
    let expected_sources = runtime_frames.iter().skip(1).take(2).collect::<Vec<_>>();
    let onion_frames = configured.animation_onion_skin_frames();
    if onion_frames.len() != expected_sources.len()
        || onion_frames.iter().zip(expected_sources).enumerate().any(
            |(index, (onion, expected))| {
                onion.source_layer_id.hierarchy_id().get() != expected.0
                    || onion.direction != AnimationOnionSkinDirection::Ahead
                    || onion.distance != index as u32 + 1
                    || (onion.opacity - (0.8 / (index as f32 + 1.0))).abs() > f32::EPSILON
            },
        )
    {
        return Err("onion skin selection did not preserve unique neighboring drawings".into());
    }
    runtime.dispatch(DocumentCommand::Undo {
        document_id: opened.document_id,
    })?;
    let restored_settings = runtime.snapshot(opened.document_id)?;
    if restored_settings
        .animation
        .zip(opened.animation)
        .is_none_or(|(restored, initial)| {
            restored.onion_skin_settings() != initial.onion_skin_settings()
        })
    {
        return Err("grouped onion skin edits did not undo as one settings change".into());
    }

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
        let hidden_playback = hidden
            .animation_playback
            .ok_or("hiding a frame removed playback state")?;
        if hidden_playback.slot_count != hidden.animation_timeline_slot_count()
            || hidden_playback.source_layer_id == Some(first_frame.source_layer_id)
        {
            return Err("hiding the active source did not reconcile the playback cursor".into());
        }

        runtime.dispatch(DocumentCommand::Undo {
            document_id: opened.document_id,
        })?;
        let restored = runtime.snapshot(opened.document_id)?;
        if restored.animation_frame_sources().count() != initial_frame_count
            || restored.animation_timeline_slots().count() != initial_slot_count
            || restored
                .animation_playback
                .is_none_or(|playback| playback.slot_count != initial_slot_count as u64)
        {
            return Err("undo did not restore the timeline and playback bounds".into());
        }

        runtime.dispatch(DocumentCommand::Redo {
            document_id: opened.document_id,
        })?;
        let redone = runtime.snapshot(opened.document_id)?;
        if redone.animation_frame_sources().count() + 1 != initial_frame_count
            || redone.animation_playback.is_none_or(|playback| {
                playback.source_layer_id == Some(first_frame.source_layer_id)
            })
        {
            return Err("redo did not reconcile the hidden playback source".into());
        }

        runtime.dispatch(DocumentCommand::Undo {
            document_id: opened.document_id,
        })?;
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
    println!("onion_skin_settings=verified");
    println!("onion_skin_selection=verified");
    println!("visibility_projection={visibility_projection}");
    Ok(())
}

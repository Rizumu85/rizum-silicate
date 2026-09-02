use silicate_runtime::{
    AnimationPlaybackDirection, AnimationPlaybackMode, DocumentCommand, DocumentRuntime,
    RuntimeEvent,
};
use std::{env, fs, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: verify_animation_playback <document.procreate>")?;
    let bytes = fs::read(path)?;
    let mut runtime = DocumentRuntime::new();
    let opened = runtime.open(&bytes)?.value;
    let document_id = opened.document_id;
    let frame_rate = opened
        .animation
        .as_ref()
        .ok_or("document does not contain Animation Assist metadata")?
        .frame_rate;
    let slots = opened.animation_timeline_slots().collect::<Vec<_>>();
    if slots.len() < 2 {
        return Err("playback smoke requires at least two visible timeline slots".into());
    }
    let last_slot = slots.len() as u64 - 1;
    let frame_nanoseconds_floor = 1_000_000_000_u64 / u64::from(frame_rate);
    let frame_nanoseconds_ceil = 1_000_000_000_u64.div_ceil(u64::from(frame_rate));

    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaying {
            document_id,
            playing: true,
        },
    )?;
    let subframe =
        runtime.advance_animation(document_id, Duration::from_nanos(frame_nanoseconds_floor))?;
    if frame_nanoseconds_floor * u64::from(frame_rate) < 1_000_000_000 {
        if !subframe.events.is_empty() || subframe.value.slot_index != 0 {
            return Err("fractional frame time advanced before a full frame elapsed".into());
        }
        let completed = runtime.advance_animation(document_id, Duration::from_nanos(1))?;
        expect_slot(&completed.value, 1, slots[1])?;
    }

    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaybackMode {
            document_id,
            mode: AnimationPlaybackMode::Loop,
        },
    )?;
    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaybackDirection {
            document_id,
            direction: AnimationPlaybackDirection::Forward,
        },
    )?;
    seek(&mut runtime, document_id, last_slot)?;
    let wrapped =
        runtime.advance_animation(document_id, Duration::from_nanos(frame_nanoseconds_ceil))?;
    expect_slot(&wrapped.value, 0, slots[0])?;

    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaybackDirection {
            document_id,
            direction: AnimationPlaybackDirection::Reverse,
        },
    )?;
    seek(&mut runtime, document_id, 0)?;
    let reverse_wrapped =
        runtime.advance_animation(document_id, Duration::from_nanos(frame_nanoseconds_ceil))?;
    expect_slot(&reverse_wrapped.value, last_slot, slots[last_slot as usize])?;

    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaybackMode {
            document_id,
            mode: AnimationPlaybackMode::PingPong,
        },
    )?;
    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaybackDirection {
            document_id,
            direction: AnimationPlaybackDirection::Forward,
        },
    )?;
    seek(&mut runtime, document_id, last_slot - 1)?;
    let ping_edge =
        runtime.advance_animation(document_id, Duration::from_nanos(frame_nanoseconds_ceil))?;
    expect_slot(&ping_edge.value, last_slot, slots[last_slot as usize])?;
    if ping_edge.value.direction != AnimationPlaybackDirection::Reverse {
        return Err("ping-pong playback did not reverse at the upper boundary".into());
    }
    let pong =
        runtime.advance_animation(document_id, Duration::from_nanos(frame_nanoseconds_ceil))?;
    expect_slot(&pong.value, last_slot - 1, slots[last_slot as usize - 1])?;

    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaybackMode {
            document_id,
            mode: AnimationPlaybackMode::OneShot,
        },
    )?;
    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaybackDirection {
            document_id,
            direction: AnimationPlaybackDirection::Forward,
        },
    )?;
    seek(&mut runtime, document_id, last_slot - 1)?;
    let one_shot_edge =
        runtime.advance_animation(document_id, Duration::from_nanos(frame_nanoseconds_ceil))?;
    if !one_shot_edge.value.playing {
        return Err("one-shot playback stopped before displaying its final slot".into());
    }
    let one_shot_stopped =
        runtime.advance_animation(document_id, Duration::from_nanos(frame_nanoseconds_ceil))?;
    expect_slot(
        &one_shot_stopped.value,
        last_slot,
        slots[last_slot as usize],
    )?;
    if one_shot_stopped.value.playing {
        return Err("one-shot playback remained active after its final slot duration".into());
    }

    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaying {
            document_id,
            playing: true,
        },
    )?;
    let restarted = runtime.animation_playback(document_id)?;
    expect_slot(&restarted, 0, slots[0])?;

    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaybackActive {
            document_id,
            active: false,
        },
    )?;
    let inactive = runtime.animation_playback(document_id)?;
    if inactive.active || inactive.playing {
        return Err("disabling Animation Assist did not pause playback".into());
    }
    dispatch(
        &mut runtime,
        DocumentCommand::SetAnimationPlaying {
            document_id,
            playing: true,
        },
    )?;
    if !runtime.animation_playback(document_id)?.active {
        return Err("starting playback did not activate Animation Assist".into());
    }

    println!("animation_playback=silicate_runtime_v1");
    println!("frame_rate={frame_rate}");
    println!("slot_count={}", slots.len());
    println!("fractional_clock=passed");
    println!("loop_forward=passed");
    println!("loop_reverse=passed");
    println!("ping_pong=passed");
    println!("one_shot=passed");
    println!("assist_activation=passed");

    Ok(())
}

fn dispatch(
    runtime: &mut DocumentRuntime,
    command: DocumentCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let update = runtime.dispatch(command)?;
    if !update
        .events
        .iter()
        .all(|event| matches!(event, RuntimeEvent::AnimationPlaybackChanged { .. }))
    {
        return Err("playback command emitted a non-playback event".into());
    }
    Ok(())
}

fn seek(
    runtime: &mut DocumentRuntime,
    document_id: silicate_runtime::DocumentId,
    slot_index: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    dispatch(
        runtime,
        DocumentCommand::SeekAnimationTimeline {
            document_id,
            slot_index,
        },
    )
}

fn expect_slot(
    playback: &silicate_runtime::AnimationPlaybackSnapshot,
    slot_index: u64,
    source_layer_id: silicate_runtime::LayerId,
) -> Result<(), Box<dyn std::error::Error>> {
    if playback.slot_index != slot_index || playback.source_layer_id != Some(source_layer_id) {
        return Err(format!(
            "expected slot {slot_index} and source {source_layer_id:?}, received {playback:?}"
        )
        .into());
    }
    Ok(())
}

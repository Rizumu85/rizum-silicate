#[cfg(not(target_arch = "wasm32"))]
use silica_gpu::{ProcreateFile as GpuDocument, error::SilicaError};
#[cfg(not(target_arch = "wasm32"))]
use silicate_runtime::{
    DocumentCommand, DocumentRuntime, DocumentSnapshot, LayerKind, LayerSnapshot, RuntimeError,
    RuntimeEvent,
};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    env,
    path::PathBuf,
    time::{Duration, Instant},
};

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: verify_runtime_visibility <document.procreate>")?;
    let bytes = std::fs::read(&path)?;
    let document = silica::ProcreateFile::open(&bytes)?;
    let mut runtime = DocumentRuntime::new();
    let opened = runtime.open_document(&document)?;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let (mut gpu_document, _) = GpuDocument::open_document(document, &bytes, &device, &queue)?;

    let runtime_ids = opened
        .value
        .layers
        .iter()
        .map(|layer| layer.layer_id.hierarchy_id())
        .collect::<Vec<_>>();
    if gpu_document.hierarchy_ids() != runtime_ids {
        return Err("runtime and GPU hierarchy identities differ".into());
    }

    let mut results = Vec::new();
    let mut absent_kinds = Vec::new();
    for kind in [LayerKind::Layer, LayerKind::Group, LayerKind::Mask] {
        let Some(target) = opened.value.layers.iter().find(|layer| layer.kind == kind) else {
            absent_kinds.push(kind);
            continue;
        };
        let elapsed =
            verify_visibility_target(&mut runtime, &mut gpu_document, &opened.value, target)?;
        results.push((kind, target.layer_id, !target.visible, elapsed));
    }
    if results.is_empty() {
        return Err("document has no hierarchy nodes to verify".into());
    }
    let background_elapsed =
        verify_background_visibility(&mut runtime, &mut gpu_document, &opened.value)?;
    let clipped_target = opened
        .value
        .layers
        .iter()
        .find(|layer| layer.kind == LayerKind::Layer)
        .ok_or("document has no layer to verify clipping")?;
    let clipped_elapsed = verify_layer_clipped(
        &mut runtime,
        &mut gpu_document,
        &opened.value,
        clipped_target,
    )?;
    verify_unsupported_clipping_kinds(&mut gpu_document, &opened.value)?;
    let blend_mode_elapsed = verify_layer_blend_mode(
        &mut runtime,
        &mut gpu_document,
        &opened.value,
        clipped_target,
    )?;
    verify_unsupported_blend_mode_kinds(&mut runtime, &mut gpu_document, &opened.value)?;

    println!("verification=runtime_mutations_to_gpu_v4");
    println!("fixture={}", path.display());
    println!("adapter={}", adapter.get_info().name);
    println!("hierarchy_nodes={}", runtime_ids.len());
    for (kind, layer_id, visible, elapsed) in results {
        println!("target_kind={kind:?}");
        println!("target_hierarchy_id={}", layer_id.hierarchy_id().get());
        println!("visible={visible}");
        println!(
            "command_to_gpu_state_us={:.3}",
            elapsed.as_secs_f64() * 1_000_000.0
        );
    }
    for kind in absent_kinds {
        println!("target_kind={kind:?}");
        println!("verification_status=absent");
    }
    println!("background_visible={}", !opened.value.background_visible);
    println!(
        "background_command_to_gpu_state_us={:.3}",
        background_elapsed.as_secs_f64() * 1_000_000.0
    );
    println!(
        "clipped_target_hierarchy_id={}",
        clipped_target.layer_id.hierarchy_id().get()
    );
    println!("clipped={}", !clipped_target.clipped.unwrap_or(false));
    println!(
        "clipped_command_to_gpu_state_us={:.3}",
        clipped_elapsed.as_secs_f64() * 1_000_000.0
    );
    println!(
        "blend_mode_target_hierarchy_id={}",
        clipped_target.layer_id.hierarchy_id().get()
    );
    println!(
        "blend_mode={}",
        gpu_document
            .layer_blend_mode(clipped_target.layer_id.hierarchy_id())?
            .as_str()
    );
    println!(
        "blend_mode_command_to_gpu_state_us={:.3}",
        blend_mode_elapsed.as_secs_f64() * 1_000_000.0
    );
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_visibility_target(
    runtime: &mut DocumentRuntime,
    gpu_document: &mut GpuDocument,
    snapshot: &DocumentSnapshot,
    target: &LayerSnapshot,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let next_visible = !target.visible;
    let started = Instant::now();
    let update = runtime.dispatch(DocumentCommand::SetLayerVisibility {
        document_id: snapshot.document_id,
        layer_id: target.layer_id,
        visible: next_visible,
    })?;
    if update.events.len() != 1 {
        return Err(format!(
            "visibility change emitted {} events instead of one",
            update.events.len()
        )
        .into());
    }
    apply_runtime_events(gpu_document, &update.events)?;
    let elapsed = started.elapsed();

    if gpu_document.hierarchy_visibility(target.layer_id.hierarchy_id())? != next_visible {
        return Err("GPU document did not apply the runtime visibility event".into());
    }

    let no_op = runtime.dispatch(DocumentCommand::SetLayerVisibility {
        document_id: snapshot.document_id,
        layer_id: target.layer_id,
        visible: next_visible,
    })?;
    if !no_op.events.is_empty() {
        return Err("idempotent visibility command emitted an event".into());
    }

    Ok(elapsed)
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_background_visibility(
    runtime: &mut DocumentRuntime,
    gpu_document: &mut GpuDocument,
    snapshot: &DocumentSnapshot,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let next_visible = !snapshot.background_visible;
    let started = Instant::now();
    let update = runtime.dispatch(DocumentCommand::SetBackgroundVisibility {
        document_id: snapshot.document_id,
        visible: next_visible,
    })?;
    if update.events.len() != 1 {
        return Err(format!(
            "background visibility change emitted {} events instead of one",
            update.events.len()
        )
        .into());
    }
    apply_runtime_events(gpu_document, &update.events)?;
    let elapsed = started.elapsed();

    let gpu_visible = !gpu_document.background_hidden;
    if gpu_visible != next_visible {
        return Err("GPU document did not apply the runtime background event".into());
    }

    let no_op = runtime.dispatch(DocumentCommand::SetBackgroundVisibility {
        document_id: snapshot.document_id,
        visible: next_visible,
    })?;
    if !no_op.events.is_empty() {
        return Err("idempotent background command emitted an event".into());
    }

    Ok(elapsed)
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_layer_clipped(
    runtime: &mut DocumentRuntime,
    gpu_document: &mut GpuDocument,
    snapshot: &DocumentSnapshot,
    target: &LayerSnapshot,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let next_clipped = !target.clipped.ok_or("target does not support clipping")?;
    let started = Instant::now();
    let update = runtime.dispatch(DocumentCommand::SetLayerClipped {
        document_id: snapshot.document_id,
        layer_id: target.layer_id,
        clipped: next_clipped,
    })?;
    if update.events.len() != 1 {
        return Err(format!(
            "clipped change emitted {} events instead of one",
            update.events.len()
        )
        .into());
    }
    apply_runtime_events(gpu_document, &update.events)?;
    let elapsed = started.elapsed();

    if gpu_document.layer_clipped(target.layer_id.hierarchy_id())? != next_clipped {
        return Err("GPU document did not apply the runtime clipped event".into());
    }

    let no_op = runtime.dispatch(DocumentCommand::SetLayerClipped {
        document_id: snapshot.document_id,
        layer_id: target.layer_id,
        clipped: next_clipped,
    })?;
    if !no_op.events.is_empty() {
        return Err("idempotent clipped command emitted an event".into());
    }

    Ok(elapsed)
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_unsupported_clipping_kinds(
    gpu_document: &mut GpuDocument,
    snapshot: &DocumentSnapshot,
) -> Result<(), Box<dyn std::error::Error>> {
    for target in snapshot
        .layers
        .iter()
        .filter(|layer| layer.kind != LayerKind::Layer)
    {
        let hierarchy_id = target.layer_id.hierarchy_id();
        match gpu_document.set_layer_clipped(hierarchy_id, true) {
            Err(SilicaError::HierarchyDoesNotSupportClipping(actual)) if actual == hierarchy_id => {
            }
            result => {
                return Err(format!(
                    "GPU clipping kind validation failed for {:?}: {result:?}",
                    target.kind
                )
                .into());
            }
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_layer_blend_mode(
    runtime: &mut DocumentRuntime,
    gpu_document: &mut GpuDocument,
    snapshot: &DocumentSnapshot,
    target: &LayerSnapshot,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let current = target
        .blend_mode
        .ok_or("target does not support blend modes")?;
    let next_blend_mode = if current == silica::BlendingMode::Normal {
        silica::BlendingMode::Multiply
    } else {
        silica::BlendingMode::Normal
    };
    let started = Instant::now();
    let update = runtime.dispatch(DocumentCommand::SetLayerBlendMode {
        document_id: snapshot.document_id,
        layer_id: target.layer_id,
        blend_mode: next_blend_mode,
    })?;
    if update.events.len() != 1 {
        return Err(format!(
            "blend mode change emitted {} events instead of one",
            update.events.len()
        )
        .into());
    }
    apply_runtime_events(gpu_document, &update.events)?;
    let elapsed = started.elapsed();

    if gpu_document.layer_blend_mode(target.layer_id.hierarchy_id())? != next_blend_mode {
        return Err("GPU document did not apply the runtime blend mode event".into());
    }

    let no_op = runtime.dispatch(DocumentCommand::SetLayerBlendMode {
        document_id: snapshot.document_id,
        layer_id: target.layer_id,
        blend_mode: next_blend_mode,
    })?;
    if !no_op.events.is_empty() {
        return Err("idempotent blend mode command emitted an event".into());
    }

    Ok(elapsed)
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_unsupported_blend_mode_kinds(
    runtime: &mut DocumentRuntime,
    gpu_document: &mut GpuDocument,
    snapshot: &DocumentSnapshot,
) -> Result<(), Box<dyn std::error::Error>> {
    for target in snapshot
        .layers
        .iter()
        .filter(|layer| layer.kind != LayerKind::Layer)
    {
        let revision = runtime.snapshot(snapshot.document_id)?.revision;
        match runtime.dispatch(DocumentCommand::SetLayerBlendMode {
            document_id: snapshot.document_id,
            layer_id: target.layer_id,
            blend_mode: silica::BlendingMode::Multiply,
        }) {
            Err(RuntimeError::LayerDoesNotSupportBlendMode {
                document_id,
                layer_id,
            }) if document_id == snapshot.document_id && layer_id == target.layer_id => {}
            result => {
                return Err(format!(
                    "runtime blend mode kind validation failed for {:?}: {result:?}",
                    target.kind
                )
                .into());
            }
        }
        if runtime.snapshot(snapshot.document_id)?.revision != revision {
            return Err("rejected blend mode command advanced the runtime revision".into());
        }

        let hierarchy_id = target.layer_id.hierarchy_id();
        match gpu_document.set_layer_blend_mode(hierarchy_id, silica::BlendingMode::Multiply) {
            Err(SilicaError::HierarchyDoesNotSupportBlendMode(actual))
                if actual == hierarchy_id => {}
            result => {
                return Err(format!(
                    "GPU blend mode kind validation failed for {:?}: {result:?}",
                    target.kind
                )
                .into());
            }
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_runtime_events(
    gpu_document: &mut GpuDocument,
    events: &[RuntimeEvent],
) -> Result<(), Box<dyn std::error::Error>> {
    for event in events {
        match event {
            RuntimeEvent::BackgroundVisibilityChanged { visible, .. } => {
                gpu_document.background_hidden = !visible;
            }
            RuntimeEvent::LayerVisibilityChanged {
                layer_id, visible, ..
            } => {
                gpu_document.set_hierarchy_visibility(layer_id.hierarchy_id(), *visible)?;
            }
            RuntimeEvent::LayerClippedChanged {
                layer_id, clipped, ..
            } => {
                gpu_document.set_layer_clipped(layer_id.hierarchy_id(), *clipped)?;
            }
            RuntimeEvent::LayerBlendModeChanged {
                layer_id,
                blend_mode,
                ..
            } => {
                gpu_document.set_layer_blend_mode(layer_id.hierarchy_id(), *blend_mode)?;
            }
            RuntimeEvent::DocumentOpened { .. } | RuntimeEvent::DocumentClosed { .. } => {
                return Err("unexpected lifecycle event during visibility verification".into());
            }
        }
    }
    Ok(())
}

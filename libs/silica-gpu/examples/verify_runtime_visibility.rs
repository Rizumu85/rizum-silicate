#[cfg(not(target_arch = "wasm32"))]
use silica_gpu::ProcreateFile as GpuDocument;
#[cfg(not(target_arch = "wasm32"))]
use silicate_runtime::{
    DocumentCommand, DocumentRuntime, DocumentSnapshot, LayerKind, LayerSnapshot, RuntimeEvent,
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

    println!("verification=runtime_visibility_to_gpu_v2");
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
            RuntimeEvent::DocumentOpened { .. } | RuntimeEvent::DocumentClosed { .. } => {
                return Err("unexpected lifecycle event during visibility verification".into());
            }
        }
    }
    Ok(())
}

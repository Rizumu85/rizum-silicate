use crate::app::{
    App, AppEvent,
    compositor::{CompositorApp, resolve_clipping_sources},
    instance::Instance,
};
use eframe::wgpu;
use silicate_compositor::buffer::BufferDimensions;
use silicate_runtime::{DocumentSnapshot, LayerKind, RuntimeUpdate};
use std::{
    io,
    path::Path,
    sync::mpsc::{Receiver, channel},
    time::Duration,
};

#[derive(Debug)]
pub struct RenderingFixtureReport {
    pub adapter: String,
    pub mask_changed_pixels: u64,
    pub group_changed_pixels: u64,
    pub clipping_changed_pixels: u64,
    pub clipping_base_visibility_changed_pixels: u64,
    pub clipping_topology_cases: usize,
    pub transparent_pixels: u64,
    pub partial_alpha_pixels: u64,
    pub opaque_pixels: u64,
}

pub fn verify_rendering_fixtures(
    mask_fixture: &Path,
    clipping_fixture: &Path,
) -> io::Result<RenderingFixtureReport> {
    let mut harness = RenderHarness::new()?;
    let clipping_topology_cases = verify_clipping_topology()?;

    let (mut mask_instance, mut mask_compositor) = harness.load(mask_fixture)?;
    let mask_baseline = harness.render(&mut mask_instance, &mut mask_compositor)?;
    let mask = mask_instance
        .snapshot
        .layers
        .iter()
        .find(|layer| layer.kind == LayerKind::Mask)
        .cloned()
        .ok_or_else(|| invalid_fixture(mask_fixture, "does not contain a layer mask"))?;
    let mask_update = harness
        .app
        .set_layer_visibility(
            mask_instance.snapshot.document_id,
            mask.layer_id,
            !mask.visible,
        )
        .map_err(other)?;
    apply_update(&mut mask_instance, mask_update)?;
    let mask_toggled = harness.render(&mut mask_instance, &mut mask_compositor)?;
    let mask_changed_pixels = changed_pixels(&mask_baseline, &mask_toggled)?;
    require_changed(mask_changed_pixels, "mask visibility")?;

    let (mut clipping_instance, mut clipping_compositor) = harness.load(clipping_fixture)?;
    let clipping_baseline = harness.render(&mut clipping_instance, &mut clipping_compositor)?;
    let group = clipping_instance
        .snapshot
        .layers
        .iter()
        .find(|layer| layer.kind == LayerKind::Group)
        .cloned()
        .ok_or_else(|| invalid_fixture(clipping_fixture, "does not contain a layer group"))?;
    let group_update = harness
        .app
        .set_layer_visibility(
            clipping_instance.snapshot.document_id,
            group.layer_id,
            !group.visible,
        )
        .map_err(other)?;
    apply_update(&mut clipping_instance, group_update)?;
    let group_toggled = harness.render(&mut clipping_instance, &mut clipping_compositor)?;
    let group_changed_pixels = changed_pixels(&clipping_baseline, &group_toggled)?;
    require_changed(group_changed_pixels, "group visibility")?;

    let group_restore = harness
        .app
        .set_layer_visibility(
            clipping_instance.snapshot.document_id,
            group.layer_id,
            group.visible,
        )
        .map_err(other)?;
    apply_update(&mut clipping_instance, group_restore)?;
    let group_restored = harness.render(&mut clipping_instance, &mut clipping_compositor)?;
    if group_restored != clipping_baseline {
        return Err(io::Error::other(
            "restoring group visibility did not reproduce the baseline image",
        ));
    }

    let clipped = clipping_instance
        .snapshot
        .layers
        .iter()
        .find(|layer| layer.clipped == Some(true))
        .cloned()
        .ok_or_else(|| invalid_fixture(clipping_fixture, "does not contain a clipped layer"))?;
    let clipping_base_id = clipping_instance
        .compositor
        .clipping_base_layer_id(&clipping_instance.snapshot, clipped.layer_id)
        .map_err(other)?
        .ok_or_else(|| {
            invalid_fixture(
                clipping_fixture,
                "contains a clipped layer without a renderable sibling base",
            )
        })?;
    if clipping_instance
        .snapshot
        .clipping_base_layer_id(clipped.layer_id)
        != Some(clipping_base_id)
    {
        return Err(io::Error::other(
            "runtime and compositor resolved different clipping bases",
        ));
    }
    let clipping_base = clipping_instance
        .snapshot
        .layers
        .iter()
        .find(|layer| layer.layer_id == clipping_base_id)
        .cloned()
        .ok_or_else(|| invalid_fixture(clipping_fixture, "is missing the clipping base"))?;
    let base_visibility_update = harness
        .app
        .set_layer_visibility(
            clipping_instance.snapshot.document_id,
            clipping_base.layer_id,
            !clipping_base.visible,
        )
        .map_err(other)?;
    apply_update(&mut clipping_instance, base_visibility_update)?;
    let base_visibility_toggled =
        harness.render(&mut clipping_instance, &mut clipping_compositor)?;
    let clipping_base_visibility_changed_pixels =
        changed_pixels(&clipping_baseline, &base_visibility_toggled)?;
    require_changed(
        clipping_base_visibility_changed_pixels,
        "clipping base visibility",
    )?;
    let base_visibility_restore = harness
        .app
        .set_layer_visibility(
            clipping_instance.snapshot.document_id,
            clipping_base.layer_id,
            clipping_base.visible,
        )
        .map_err(other)?;
    apply_update(&mut clipping_instance, base_visibility_restore)?;
    let base_visibility_restored =
        harness.render(&mut clipping_instance, &mut clipping_compositor)?;
    if base_visibility_restored != clipping_baseline {
        return Err(io::Error::other(
            "restoring clipping base visibility did not reproduce the baseline image",
        ));
    }

    let clipping_update = harness
        .app
        .set_layer_clipped(
            clipping_instance.snapshot.document_id,
            clipped.layer_id,
            false,
        )
        .map_err(other)?;
    apply_update(&mut clipping_instance, clipping_update)?;
    let clipping_disabled = harness.render(&mut clipping_instance, &mut clipping_compositor)?;
    let clipping_changed_pixels = changed_pixels(&clipping_baseline, &clipping_disabled)?;
    require_changed(clipping_changed_pixels, "layer clipping")?;

    set_layer_clipped(&harness, &mut clipping_instance, clipped.layer_id, true)?;
    let clipping_restored = harness.render(&mut clipping_instance, &mut clipping_compositor)?;
    if clipping_restored != clipping_baseline {
        return Err(io::Error::other(
            "restoring clipping did not reproduce the baseline image",
        ));
    }

    let (transparent_pixels, partial_alpha_pixels, opaque_pixels) =
        alpha_coverage(&clipping_baseline);
    Ok(RenderingFixtureReport {
        adapter: harness.adapter,
        mask_changed_pixels,
        group_changed_pixels,
        clipping_changed_pixels,
        clipping_base_visibility_changed_pixels,
        clipping_topology_cases,
        transparent_pixels,
        partial_alpha_pixels,
        opaque_pixels,
    })
}

fn verify_clipping_topology() -> io::Result<usize> {
    let layers = [
        (false, 0),
        (true, 0),
        (true, 0),
        (false, 1),
        (true, 1),
        (true, 2),
    ];
    let actual = resolve_clipping_sources(layers.len(), 3, |index| layers[index]);
    let expected = [None, Some(0), Some(0), None, Some(3), None];
    if actual != expected {
        return Err(io::Error::other(format!(
            "clipping topology resolved to {actual:?}, expected {expected:?}"
        )));
    }
    Ok(layers.len())
}

fn set_layer_clipped(
    harness: &RenderHarness,
    instance: &mut Instance,
    layer_id: silicate_runtime::LayerId,
    clipped: bool,
) -> io::Result<()> {
    let update = harness
        .app
        .set_layer_clipped(instance.snapshot.document_id, layer_id, clipped)
        .map_err(other)?;
    apply_update(instance, update)
}

struct RenderHarness {
    app: App,
    device: wgpu::Device,
    queue: wgpu::Queue,
    events: Receiver<AppEvent>,
    runtime: tokio::runtime::Runtime,
    adapter: String,
}

impl RenderHarness {
    fn new() -> io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = runtime
            .block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .map_err(other)?;
        let (device, queue) = runtime
            .block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(other)?;
        let adapter_name = adapter.get_info().name;
        let (sender, events) = channel();
        let app = App::new(device.clone(), queue.clone(), sender);

        Ok(Self {
            app,
            device,
            queue,
            events,
            runtime,
            adapter: adapter_name,
        })
    }

    fn load(&self, fixture: &Path) -> io::Result<(Instance, CompositorApp)> {
        self.app.load_file(fixture).map_err(other)?;
        match self.events.recv_timeout(Duration::from_secs(10)) {
            Ok(AppEvent::NewInstance(_, instance, compositor)) => Ok((instance, compositor)),
            Ok(event) => Err(io::Error::other(format!(
                "fixture load emitted an unexpected event: {event:?}"
            ))),
            Err(error) => Err(other(error)),
        }
    }

    fn render(
        &mut self,
        instance: &mut Instance,
        compositor: &mut CompositorApp,
    ) -> io::Result<image::RgbaImage> {
        compositor.rendering_tick_blocking(&instance.output_texture);
        let dimensions = BufferDimensions::from_extent(instance.output_texture.size());
        self.runtime
            .block_on(App::export(
                &instance.output_texture,
                &self.device,
                &self.queue,
                dimensions,
                instance.file.orientation,
            ))
            .map_err(other)
    }
}

fn apply_update(
    instance: &mut Instance,
    update: RuntimeUpdate<DocumentSnapshot>,
) -> io::Result<()> {
    instance.apply_runtime_update(update);
    instance.submit_to_compositor().map_err(other)
}

fn changed_pixels(before: &image::RgbaImage, after: &image::RgbaImage) -> io::Result<u64> {
    if before.dimensions() != after.dimensions() {
        return Err(io::Error::other(format!(
            "mutation changed output dimensions from {:?} to {:?}",
            before.dimensions(),
            after.dimensions()
        )));
    }
    Ok(before
        .pixels()
        .zip(after.pixels())
        .filter(|(before, after)| before != after)
        .count() as u64)
}

fn alpha_coverage(image: &image::RgbaImage) -> (u64, u64, u64) {
    image.pixels().fold((0, 0, 0), |mut counts, pixel| {
        match pixel[3] {
            0 => counts.0 += 1,
            255 => counts.2 += 1,
            _ => counts.1 += 1,
        }
        counts
    })
}

fn require_changed(changed: u64, operation: &str) -> io::Result<()> {
    if changed == 0 {
        Err(io::Error::other(format!(
            "{operation} did not change any rendered pixels"
        )))
    } else {
        Ok(())
    }
}

fn invalid_fixture(path: &Path, reason: &str) -> io::Error {
    io::Error::other(format!("fixture {} {reason}", path.display()))
}

fn other(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}

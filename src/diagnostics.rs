use crate::app::{
    App, AppEvent,
    compositor::{CompositorApp, resolve_clipping_sources},
    instance::Instance,
};
use eframe::wgpu;
use silica::quicklook::extract_quicklook_png_from_reader;
use silicate_compositor::buffer::BufferDimensions;
use silicate_runtime::{DocumentSnapshot, LayerKind, RuntimeUpdate};
use std::{
    fs::File,
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

#[derive(Debug)]
pub struct ProcreateRenderComparisonReport {
    pub adapter: String,
    pub rendered_dimensions: (u32, u32),
    pub quicklook_dimensions: (u32, u32),
    pub quicklook_compared_pixels: u64,
    pub quicklook_pixels_over_four_lsb: u64,
    pub quicklook_mean_absolute_error: f64,
    pub quicklook_root_mean_square_error: f64,
    pub quicklook_max_channel_error: u8,
    pub composite_compared_pixels: u64,
    pub composite_pixels_over_four_lsb: u64,
    pub composite_mean_absolute_error: f64,
    pub composite_root_mean_square_error: f64,
    pub composite_max_channel_error: u8,
}

struct ImageComparison {
    diff: image::RgbaImage,
    pixels_over_four_lsb: u64,
    mean_absolute_error: f64,
    root_mean_square_error: f64,
    max_channel_error: u8,
}

pub fn compare_procreate_fixture(
    fixture: &Path,
    output_directory: Option<&Path>,
) -> io::Result<ProcreateRenderComparisonReport> {
    let quicklook = extract_quicklook_png_from_reader(File::open(fixture)?)
        .map_err(other)?
        .ok_or_else(|| invalid_fixture(fixture, "does not contain a QuickLook PNG"))?;
    let reference = image::load_from_memory_with_format(&quicklook.bytes, image::ImageFormat::Png)
        .map_err(other)?
        .to_rgba8();

    let mut harness = RenderHarness::new()?;
    let (mut instance, mut compositor) = harness.load(fixture)?;
    let rendered = harness.render(&mut instance, &mut compositor)?;
    let persisted_composite = harness.render_persisted_composite(&instance, &mut compositor)?;
    let expected_reference_height = (u64::from(rendered.height()) * u64::from(reference.width()))
        .div_ceil(u64::from(rendered.width()));
    let expected_reference_width = (u64::from(rendered.width()) * u64::from(reference.height()))
        .div_ceil(u64::from(rendered.height()));
    if u64::from(reference.height()).abs_diff(expected_reference_height) > 1
        && u64::from(reference.width()).abs_diff(expected_reference_width) > 1
    {
        return Err(invalid_fixture(
            fixture,
            "has a QuickLook aspect ratio that differs from the production render",
        ));
    }

    let resized = image::imageops::resize(
        &rendered,
        reference.width(),
        reference.height(),
        image::imageops::FilterType::Lanczos3,
    );
    let quicklook_comparison = compare_images(&resized, &reference)?;
    let composite_comparison = compare_images(&rendered, &persisted_composite)?;

    if let Some(output_directory) = output_directory {
        std::fs::create_dir_all(output_directory)?;
        reference
            .save(output_directory.join("procreate_quicklook.png"))
            .map_err(other)?;
        rendered
            .save(output_directory.join("silicate_render.png"))
            .map_err(other)?;
        persisted_composite
            .save(output_directory.join("procreate_composite.png"))
            .map_err(other)?;
        resized
            .save(output_directory.join("silicate_resized.png"))
            .map_err(other)?;
        quicklook_comparison
            .diff
            .save(output_directory.join("quicklook_diff_x4.png"))
            .map_err(other)?;
        composite_comparison
            .diff
            .save(output_directory.join("composite_diff_x4.png"))
            .map_err(other)?;
    }

    Ok(ProcreateRenderComparisonReport {
        adapter: harness.adapter,
        rendered_dimensions: rendered.dimensions(),
        quicklook_dimensions: reference.dimensions(),
        quicklook_compared_pixels: u64::from(reference.width()) * u64::from(reference.height()),
        quicklook_pixels_over_four_lsb: quicklook_comparison.pixels_over_four_lsb,
        quicklook_mean_absolute_error: quicklook_comparison.mean_absolute_error,
        quicklook_root_mean_square_error: quicklook_comparison.root_mean_square_error,
        quicklook_max_channel_error: quicklook_comparison.max_channel_error,
        composite_compared_pixels: u64::from(rendered.width()) * u64::from(rendered.height()),
        composite_pixels_over_four_lsb: composite_comparison.pixels_over_four_lsb,
        composite_mean_absolute_error: composite_comparison.mean_absolute_error,
        composite_root_mean_square_error: composite_comparison.root_mean_square_error,
        composite_max_channel_error: composite_comparison.max_channel_error,
    })
}

fn compare_images(
    actual: &image::RgbaImage,
    expected: &image::RgbaImage,
) -> io::Result<ImageComparison> {
    if actual.dimensions() != expected.dimensions() {
        return Err(io::Error::other(format!(
            "cannot compare image dimensions {:?} and {:?}",
            actual.dimensions(),
            expected.dimensions()
        )));
    }

    let mut diff = image::RgbaImage::new(expected.width(), expected.height());
    let mut absolute_error = 0_u64;
    let mut squared_error = 0_u64;
    let mut pixels_over_four_lsb = 0_u64;
    let mut max_channel_error = 0_u8;

    for ((actual, expected), diff_pixel) in actual
        .pixels()
        .zip(expected.pixels())
        .zip(diff.pixels_mut())
    {
        let actual = premultiplied_rgba(*actual);
        let expected = premultiplied_rgba(*expected);
        let channel_errors =
            std::array::from_fn::<_, 4, _>(|index| actual[index].abs_diff(expected[index]));
        if channel_errors.iter().any(|error| *error > 4) {
            pixels_over_four_lsb += 1;
        }
        for error in channel_errors {
            absolute_error += u64::from(error);
            squared_error += u64::from(error) * u64::from(error);
            max_channel_error = max_channel_error.max(error);
        }
        *diff_pixel = image::Rgba([
            channel_errors[0].saturating_mul(4),
            channel_errors[1].saturating_mul(4),
            channel_errors[2].saturating_mul(4),
            255,
        ]);
    }

    let compared_pixels = u64::from(expected.width()) * u64::from(expected.height());
    let channel_count = compared_pixels * 4;
    Ok(ImageComparison {
        diff,
        pixels_over_four_lsb,
        mean_absolute_error: absolute_error as f64 / channel_count as f64,
        root_mean_square_error: (squared_error as f64 / channel_count as f64).sqrt(),
        max_channel_error,
    })
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

    fn render_persisted_composite(
        &mut self,
        instance: &Instance,
        compositor: &mut CompositorApp,
    ) -> io::Result<image::RgbaImage> {
        if !compositor.render_persisted_composite(&instance.output_texture) {
            return Err(io::Error::other(
                "fixture does not contain a persisted Procreate composite",
            ));
        }
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

fn premultiplied_rgba(pixel: image::Rgba<u8>) -> [u8; 4] {
    let alpha = u16::from(pixel[3]);
    [
        ((u16::from(pixel[0]) * alpha + 127) / 255) as u8,
        ((u16::from(pixel[1]) * alpha + 127) / 255) as u8,
        ((u16::from(pixel[2]) * alpha + 127) / 255) as u8,
        pixel[3],
    ]
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

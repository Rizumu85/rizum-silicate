#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use silicate_compositor::{
        CompositeIsolation, CompositePhase, Compositor,
        buffer::BufferDimensions,
        canvas::{CompositorAtlasTiling, CompositorCanvasTiling},
        pipeline::Pipeline,
        tex::TextureExt,
    };
    use std::num::NonZeroU32;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

    let atlas = wgpu::Texture::empty_layers(&device, 1, 1, 4, wgpu::Texture::LAYER_USAGE);
    write_pixel(&queue, &atlas, 1, [255, 0, 0, 255]);
    write_pixel(&queue, &atlas, 2, [0, 0, 128, 128]);
    write_pixel(&queue, &atlas, 3, [128, 128, 128, 128]);
    let output = wgpu::Texture::empty(&device, 1, 1, wgpu::Texture::OUTPUT_USAGE);
    let mut compositor = Compositor::new(
        &device,
        &queue,
        CompositorCanvasTiling::new((1, 1), (1, 1), 1),
        CompositorAtlasTiling::new(1, 1),
        atlas,
    );
    compositor.load_chunk_buffer(&[]);
    compositor.load_layer_buffer(&[]);
    compositor.set_background(Some([0.25, 0.5, 0.75, 1.0]));
    let pipeline = Pipeline::new(&device);
    compositor.render(&pipeline, output.create_default_view());

    let dimensions = BufferDimensions::new(1, 1);
    let background_pixel = read_pixel(&device, &queue, &output, dimensions)?;
    expect_pixel(background_pixel, [64, 128, 191, 255], "background")?;

    let isolation_id = NonZeroU32::new(1).expect("isolation id is non-zero");
    compositor.set_background(None);
    compositor.load_chunk_buffer(&[chunk(1, 0), chunk(2, 1)]);
    compositor.load_layer_buffer(&[
        layer(
            CompositePhase::Base,
            Some(CompositeIsolation {
                id: isolation_id,
                opacity: 0.5,
            }),
        ),
        layer(
            CompositePhase::Base,
            Some(CompositeIsolation {
                id: isolation_id,
                opacity: 0.5,
            }),
        ),
    ]);
    compositor.render(&pipeline, output.create_default_view());
    let isolated_pixel = read_pixel(&device, &queue, &output, dimensions)?;
    expect_pixel(isolated_pixel, [64, 0, 64, 128], "isolated opacity")?;

    compositor.load_layer_buffer(&[
        layer(CompositePhase::Primary, None),
        layer(CompositePhase::Base, None),
    ]);
    compositor.render(&pipeline, output.create_default_view());
    let phased_pixel = read_pixel(&device, &queue, &output, dimensions)?;
    expect_pixel(phased_pixel, [255, 0, 0, 255], "phase ordering")?;

    compositor.load_chunk_buffer(&[chunk(2, 0)]);
    compositor.load_layer_buffer(&[layer(CompositePhase::Base, None)]);
    compositor.render(&pipeline, output.create_default_view());
    let alpha_edge_pixel = read_pixel(&device, &queue, &output, dimensions)?;
    expect_pixel(
        alpha_edge_pixel,
        [0, 0, 128, 128],
        "premultiplied alpha edge",
    )?;

    let mut hidden_layer = layer(CompositePhase::Base, None);
    hidden_layer.hidden = true;
    compositor.load_chunk_buffer(&[chunk(1, 0)]);
    compositor.load_layer_buffer(&[hidden_layer]);
    compositor.render(&pipeline, output.create_default_view());
    let hidden_pixel = read_pixel(&device, &queue, &output, dimensions)?;
    expect_pixel(hidden_pixel, [0, 0, 0, 0], "hidden layer")?;

    let mut masked_layer = layer(CompositePhase::Base, None);
    masked_layer.mask_hidden = false;
    compositor.load_chunk_buffer(&[chunk_with_effects(1, 0, Some(3), None)]);
    compositor.load_layer_buffer(&[masked_layer]);
    compositor.render(&pipeline, output.create_default_view());
    let masked_pixel = read_pixel(&device, &queue, &output, dimensions)?;
    expect_pixel(masked_pixel, [128, 0, 0, 128], "visible mask")?;

    compositor.load_layer_buffer(&[layer(CompositePhase::Base, None)]);
    compositor.render(&pipeline, output.create_default_view());
    let hidden_mask_pixel = read_pixel(&device, &queue, &output, dimensions)?;
    expect_pixel(hidden_mask_pixel, [255, 0, 0, 255], "hidden mask")?;

    let mut clipped_layer = layer(CompositePhase::Base, None);
    clipped_layer.clipped = true;
    compositor.load_chunk_buffer(&[chunk_with_effects(1, 0, None, Some(3))]);
    compositor.load_layer_buffer(&[clipped_layer]);
    compositor.render(&pipeline, output.create_default_view());
    let clipped_pixel = read_pixel(&device, &queue, &output, dimensions)?;
    expect_pixel(clipped_pixel, [128, 0, 0, 128], "clipping alpha")?;

    println!("verification=compositor_composition_v3");
    println!("adapter={}", adapter.get_info().name);
    println!("background_rgba={background_pixel:?}");
    println!("isolated_rgba={isolated_pixel:?}");
    println!("phased_rgba={phased_pixel:?}");
    println!("alpha_edge_rgba={alpha_edge_pixel:?}");
    println!("hidden_rgba={hidden_pixel:?}");
    println!("masked_rgba={masked_pixel:?}");
    println!("mask_hidden_rgba={hidden_mask_pixel:?}");
    println!("clipped_rgba={clipped_pixel:?}");
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn chunk(atlas_index: u32, layer_index: u32) -> silicate_compositor::ChunkTile {
    chunk_with_effects(atlas_index, layer_index, None, None)
}

#[cfg(not(target_arch = "wasm32"))]
fn chunk_with_effects(
    atlas_index: u32,
    layer_index: u32,
    mask_atlas_index: Option<u32>,
    clip_atlas_index: Option<u32>,
) -> silicate_compositor::ChunkTile {
    silicate_compositor::ChunkTile {
        col: 0,
        row: 0,
        atlas_index: std::num::NonZeroU32::new(atlas_index).expect("atlas index is non-zero"),
        mask_atlas_index: mask_atlas_index.and_then(std::num::NonZeroU32::new),
        clip_atlas_index: clip_atlas_index.and_then(std::num::NonZeroU32::new),
        layer_index,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn layer(
    phase: silicate_compositor::CompositePhase,
    isolation: Option<silicate_compositor::CompositeIsolation>,
) -> silicate_compositor::CompositeLayer {
    silicate_compositor::CompositeLayer {
        opacity: 1.0,
        blend: silicate_compositor::blend::BlendingMode::Normal,
        clipped: false,
        hidden: false,
        mask_hidden: true,
        phase,
        isolation,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_pixel(queue: &wgpu::Queue, texture: &wgpu::Texture, layer: u32, pixel: [u8; 4]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: 0,
                y: 0,
                z: layer,
            },
            aspect: wgpu::TextureAspect::All,
        },
        &pixel,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn read_pixel(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    output: &wgpu::Texture,
    dimensions: silicate_compositor::buffer::BufferDimensions,
) -> Result<[u8; 4], Box<dyn std::error::Error>> {
    use silicate_compositor::tex::TextureExt;

    let buffer = output.export_buffer(device, queue, dimensions);
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(10)),
    })?;
    receiver.recv()??;

    let data = slice.get_mapped_range();
    Ok(data
        .get(..4)
        .ok_or("compositor readback was empty")?
        .try_into()
        .expect("pixel slice has four channels"))
}

#[cfg(not(target_arch = "wasm32"))]
fn expect_pixel(
    actual: [u8; 4],
    expected: [u8; 4],
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if actual
        .iter()
        .zip(expected)
        .any(|(actual, expected)| actual.abs_diff(expected) > 1)
    {
        return Err(format!("{label} pixel was {actual:?}, expected {expected:?}").into());
    }
    Ok(())
}

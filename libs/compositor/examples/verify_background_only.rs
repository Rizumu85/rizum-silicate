#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use silicate_compositor::{
        Compositor,
        buffer::BufferDimensions,
        canvas::{CompositorAtlasTiling, CompositorCanvasTiling},
        pipeline::Pipeline,
        tex::TextureExt,
    };

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

    let atlas = wgpu::Texture::empty_layers(&device, 1, 1, 1, wgpu::Texture::LAYER_USAGE);
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
    compositor.render(&Pipeline::new(&device), output.create_default_view());

    let dimensions = BufferDimensions::new(1, 1);
    let buffer = output.export_buffer(&device, &queue, dimensions);
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
    let pixel = data.get(..4).ok_or("background readback was empty")?;
    let expected = [64, 128, 191, 255];
    if pixel
        .iter()
        .zip(expected)
        .any(|(actual, expected)| actual.abs_diff(expected) > 1)
    {
        return Err(format!("background pixel was {pixel:?}, expected {expected:?}").into());
    }

    println!("verification=compositor_background_only_v1");
    println!("adapter={}", adapter.get_info().name);
    println!("pixel_rgba={pixel:?}");
    Ok(())
}

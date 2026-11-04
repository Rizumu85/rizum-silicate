use eframe::wgpu;
use silica_gpu::SilicaHierarchy;
use silicate_compositor::tex::TextureExt;
use std::collections::BTreeMap;

/// Renderer-local IDs map to views; neither hierarchy IDs nor array offsets cross this boundary.
#[derive(Debug)]
pub struct LayerPreviews {
    pub size: [u32; 2],
    pub views: BTreeMap<u32, wgpu::TextureView>,
}

impl LayerPreviews {
    pub fn new(device: &wgpu::Device, size: [u32; 2], layers: &[SilicaHierarchy]) -> Self {
        fn collect_ids(layers: &[SilicaHierarchy], ids: &mut Vec<u32>) {
            for node in layers {
                match node {
                    SilicaHierarchy::Group(group) => {
                        ids.push(group.id);
                        collect_ids(&group.children, ids);
                    }
                    SilicaHierarchy::Layer(layer) => {
                        ids.push(layer.id);
                        if let Some(mask) = &layer.mask {
                            ids.push(mask.id);
                        }
                    }
                }
            }
        }

        let mut ids = Vec::new();
        collect_ids(layers, &mut ids);
        let limits = device.limits();
        // Keep ordinary previews at 256 px; reduce resolution for very large hierarchies
        // so paging removes the device limit without allowing unbounded pixel payload.
        const PIXEL_BUDGET: usize = 64 * 1024 * 1024 / 4;
        let edge = ((PIXEL_BUDGET / ids.len().max(1)) as f64).sqrt().floor() as u32;
        let edge = edge.clamp(1, 256).min(limits.max_texture_dimension_2d);
        let longest = size[0].max(size[1]).max(1) as u64;
        let size = size.map(|axis| ((axis as u64 * edge as u64) / longest).max(1) as u32);
        let mut views = BTreeMap::new();
        for page in ids.chunks(limits.max_texture_array_layers as usize) {
            let texture = wgpu::Texture::empty_layers(
                device,
                size[0],
                size[1],
                page.len() as u32,
                wgpu::Texture::OUTPUT_USAGE,
            );
            for (slot, id) in page.iter().enumerate() {
                views.insert(*id, texture.create_view_layer(slot as u32));
            }
        }
        Self { size, views }
    }
}

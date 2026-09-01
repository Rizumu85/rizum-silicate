use crate::ZipArchiveMmap;
use crate::error::SilicaError;
use crate::tiling::{AtlasTextureTiling, CanvasTiling};
use crate::types::{hierarchy::SilicaHierarchy, layer::SilicaLayer};
#[cfg(not(target_arch = "wasm32"))]
use rayon::{iter::ParallelDrainRange, prelude::ParallelIterator};
use silica::ns_archive::Size;
use std::io::Cursor;
use std::sync::atomic::AtomicU32;
use zip::read::ZipArchive;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcreateFile {
    info: silica::ProcreateFile,
    pub composite: Option<SilicaLayer>,
    pub layers: Vec<SilicaHierarchy>,
}

impl std::ops::Deref for ProcreateFile {
    type Target = silica::ProcreateFile;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl std::ops::DerefMut for ProcreateFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.info
    }
}

pub struct ProcreateFileAtlas {
    pub atlas_texture: wgpu::Texture,
    pub canvas_tiling: CanvasTiling,
}

impl ProcreateFile {
    // Load a Procreate file asynchronously.
    pub fn open(
        bytes: &[u8],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(Self, ProcreateFileAtlas), SilicaError> {
        let document = silica::ProcreateFile::open(bytes)?;
        Self::open_document(document, bytes, device, queue)
    }

    /// Uploads an already parsed document without decoding `Document.archive` again.
    pub fn open_document(
        document: silica::ProcreateFile,
        bytes: &[u8],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(Self, ProcreateFileAtlas), SilicaError> {
        let archive = ZipArchive::new(Cursor::new(bytes))?;

        Self::load(document, &archive, device, queue)
    }

    pub fn layer_count(&self, include_groups: bool) -> u32 {
        self.layers
            .iter()
            .map(|layer| layer.layer_count(include_groups))
            .sum()
    }

    /// Returns renderer-neutral hierarchy identities in parser preorder.
    pub fn hierarchy_ids(&self) -> Vec<silica::HierarchyId> {
        let mut ids = Vec::with_capacity(self.layer_count(true) as usize);
        for hierarchy in &self.layers {
            hierarchy.append_hierarchy_ids(&mut ids);
        }
        ids
    }

    /// Applies visibility by renderer-neutral hierarchy identity.
    pub fn set_hierarchy_visibility(
        &mut self,
        hierarchy_id: silica::HierarchyId,
        visible: bool,
    ) -> Result<bool, SilicaError> {
        self.layers
            .iter_mut()
            .find_map(|hierarchy| hierarchy.set_hierarchy_visibility(hierarchy_id, visible))
            .ok_or(SilicaError::HierarchyNotFound(hierarchy_id))
    }

    /// Reads visibility by renderer-neutral hierarchy identity.
    pub fn hierarchy_visibility(
        &self,
        hierarchy_id: silica::HierarchyId,
    ) -> Result<bool, SilicaError> {
        self.layers
            .iter()
            .find_map(|hierarchy| hierarchy.hierarchy_visibility(hierarchy_id))
            .ok_or(SilicaError::HierarchyNotFound(hierarchy_id))
    }

    /// Applies clipping to a real layer by renderer-neutral hierarchy identity.
    pub fn set_layer_clipped(
        &mut self,
        hierarchy_id: silica::HierarchyId,
        clipped: bool,
    ) -> Result<bool, SilicaError> {
        self.layers
            .iter_mut()
            .find_map(|hierarchy| hierarchy.set_layer_clipped(hierarchy_id, clipped))
            .unwrap_or(Err(SilicaError::HierarchyNotFound(hierarchy_id)))
    }

    /// Reads clipping from a real layer by renderer-neutral hierarchy identity.
    pub fn layer_clipped(&self, hierarchy_id: silica::HierarchyId) -> Result<bool, SilicaError> {
        self.layers
            .iter()
            .find_map(|hierarchy| hierarchy.layer_clipped(hierarchy_id))
            .unwrap_or(Err(SilicaError::HierarchyNotFound(hierarchy_id)))
    }

    /// Applies a blend mode to a real layer by renderer-neutral hierarchy identity.
    pub fn set_layer_blend_mode(
        &mut self,
        hierarchy_id: silica::HierarchyId,
        blend_mode: silica::BlendingMode,
    ) -> Result<bool, SilicaError> {
        self.layers
            .iter_mut()
            .find_map(|hierarchy| hierarchy.set_layer_blend_mode(hierarchy_id, blend_mode))
            .unwrap_or(Err(SilicaError::HierarchyNotFound(hierarchy_id)))
    }

    /// Reads the blend mode from a real layer by renderer-neutral hierarchy identity.
    pub fn layer_blend_mode(
        &self,
        hierarchy_id: silica::HierarchyId,
    ) -> Result<silica::BlendingMode, SilicaError> {
        self.layers
            .iter()
            .find_map(|hierarchy| hierarchy.layer_blend_mode(hierarchy_id))
            .unwrap_or(Err(SilicaError::HierarchyNotFound(hierarchy_id)))
    }

    pub(crate) fn load(
        mut info: silica::ProcreateFile,
        archive: &ZipArchiveMmap<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(ProcreateFile, ProcreateFileAtlas), SilicaError> {
        let file_names = archive.file_names().collect::<Vec<_>>();
        let chunk_count = file_names.len() as u32;

        let size = info.size;
        let tile_size = info.tile_size;

        let (cols, rows) = (
            size.width.div_ceil(tile_size),
            size.height.div_ceil(tile_size),
        );

        let tiling = CanvasTiling {
            cols,
            rows,
            diff: Size {
                width: cols * tile_size - size.width,
                height: rows * tile_size - size.height,
            },
            size: tile_size,
            atlas: AtlasTextureTiling::compute_atlas_size(chunk_count, tile_size, &device.limits()),
        };

        let atlas_texture = Self::empty_layers(
            device,
            tiling.size * tiling.atlas.cols,
            tiling.size * tiling.atlas.rows,
            tiling.atlas.layers, // Make it an array
        );

        let params = crate::params::LoadParams {
            queue,
            archive,
            atlas_texture: &atlas_texture,
            file_names,
            tiling,
            chunk_id_counter: AtomicU32::new(1),
            layer_id_counter: AtomicU32::new(0),
        };

        Ok((
            ProcreateFile {
                composite: info
                    .composite
                    .take()
                    .and_then(|composite| SilicaLayer::load(composite, &params, false).ok()),
                layers: {
                    #[cfg(not(target_arch = "wasm32"))]
                    let iter = info.layers.par_drain(..);
                    #[cfg(target_arch = "wasm32")]
                    let iter = info.layers.drain(..);
                    iter
                }
                .map(|ir| SilicaHierarchy::load(ir, &params))
                .collect::<Result<_, _>>()?,
                info,
            },
            ProcreateFileAtlas {
                atlas_texture,
                canvas_tiling: tiling,
            },
        ))
    }

    pub fn empty_layers(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        layers: u32,
    ) -> wgpu::Texture {
        const TEX_DIM: wgpu::TextureDimension = wgpu::TextureDimension::D2;
        const TEX_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

        device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TEX_DIM,
            format: TEX_FORMAT,
            view_formats: &[
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ],
            usage: wgpu::TextureUsages::COPY_DST
                .union(wgpu::TextureUsages::COPY_SRC)
                .union(wgpu::TextureUsages::TEXTURE_BINDING),
            label: None,
        })
    }
}

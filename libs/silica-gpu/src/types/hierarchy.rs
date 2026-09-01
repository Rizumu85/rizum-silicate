use crate::error::SilicaError;
use crate::params::LoadParams;
use crate::types::{group::SilicaGroup, layer::SilicaLayer};

#[derive(Debug, Clone, PartialEq)]
pub enum SilicaHierarchy {
    Layer(SilicaLayer),
    Group(SilicaGroup),
}

enum LayerTarget<'a> {
    Layer(&'a SilicaLayer),
    Unsupported,
}

enum LayerTargetMut<'a> {
    Layer(&'a mut SilicaLayer),
    Unsupported,
}

impl SilicaHierarchy {
    pub fn layer_count(&self, include_groups: bool) -> u32 {
        match self {
            SilicaHierarchy::Layer(layer) => 1 + if layer.mask.is_some() { 1 } else { 0 },
            SilicaHierarchy::Group(silica_group) => silica_group.layer_count(include_groups),
        }
    }

    pub(crate) fn load<'a>(
        info: silica::SilicaHierarchy,
        params: &'a LoadParams<'a>,
    ) -> Result<SilicaHierarchy, SilicaError> {
        Ok(match info {
            silica::SilicaHierarchy::Layer(layer) => {
                SilicaHierarchy::Layer(SilicaLayer::load(layer, params, false)?)
            }
            silica::SilicaHierarchy::Group(group) => {
                SilicaHierarchy::Group(SilicaGroup::load(group, params)?)
            }
        })
    }

    pub(crate) fn append_hierarchy_ids(&self, ids: &mut Vec<silica::HierarchyId>) {
        match self {
            SilicaHierarchy::Layer(layer) => {
                ids.push(layer.hierarchy_id());
                if let Some(mask) = &layer.mask {
                    ids.push(mask.hierarchy_id());
                }
            }
            SilicaHierarchy::Group(group) => {
                ids.push(group.hierarchy_id());
                for child in &group.children {
                    child.append_hierarchy_ids(ids);
                }
            }
        }
    }

    pub(crate) fn set_hierarchy_visibility(
        &mut self,
        hierarchy_id: silica::HierarchyId,
        visible: bool,
    ) -> Option<bool> {
        match self {
            SilicaHierarchy::Layer(layer) => {
                if layer.hierarchy_id() == hierarchy_id {
                    let changed = layer.hidden == visible;
                    layer.hidden = !visible;
                    return Some(changed);
                }
                if let Some(mask) = &mut layer.mask
                    && mask.hierarchy_id() == hierarchy_id
                {
                    let changed = mask.hidden == visible;
                    mask.hidden = !visible;
                    return Some(changed);
                }
                None
            }
            SilicaHierarchy::Group(group) => {
                if group.hierarchy_id() == hierarchy_id {
                    let changed = group.hidden == visible;
                    group.hidden = !visible;
                    return Some(changed);
                }
                group
                    .children
                    .iter_mut()
                    .find_map(|child| child.set_hierarchy_visibility(hierarchy_id, visible))
            }
        }
    }

    pub(crate) fn hierarchy_visibility(&self, hierarchy_id: silica::HierarchyId) -> Option<bool> {
        match self {
            SilicaHierarchy::Layer(layer) => {
                if layer.hierarchy_id() == hierarchy_id {
                    return Some(!layer.hidden);
                }
                layer
                    .mask
                    .as_ref()
                    .and_then(|mask| (mask.hierarchy_id() == hierarchy_id).then_some(!mask.hidden))
            }
            SilicaHierarchy::Group(group) => {
                if group.hierarchy_id() == hierarchy_id {
                    return Some(!group.hidden);
                }
                group
                    .children
                    .iter()
                    .find_map(|child| child.hierarchy_visibility(hierarchy_id))
            }
        }
    }

    pub(crate) fn set_layer_clipped(
        &mut self,
        hierarchy_id: silica::HierarchyId,
        clipped: bool,
    ) -> Option<Result<bool, SilicaError>> {
        self.layer_target_mut(hierarchy_id)
            .map(|target| match target {
                LayerTargetMut::Layer(layer) => {
                    let changed = layer.clipped != clipped;
                    layer.clipped = clipped;
                    Ok(changed)
                }
                LayerTargetMut::Unsupported => {
                    Err(SilicaError::HierarchyDoesNotSupportClipping(hierarchy_id))
                }
            })
    }

    pub(crate) fn layer_clipped(
        &self,
        hierarchy_id: silica::HierarchyId,
    ) -> Option<Result<bool, SilicaError>> {
        self.layer_target(hierarchy_id).map(|target| match target {
            LayerTarget::Layer(layer) => Ok(layer.clipped),
            LayerTarget::Unsupported => {
                Err(SilicaError::HierarchyDoesNotSupportClipping(hierarchy_id))
            }
        })
    }

    pub(crate) fn set_layer_blend_mode(
        &mut self,
        hierarchy_id: silica::HierarchyId,
        blend_mode: silica::BlendingMode,
    ) -> Option<Result<bool, SilicaError>> {
        self.layer_target_mut(hierarchy_id)
            .map(|target| match target {
                LayerTargetMut::Layer(layer) => {
                    let changed = layer.blend != blend_mode;
                    layer.blend = blend_mode;
                    Ok(changed)
                }
                LayerTargetMut::Unsupported => {
                    Err(SilicaError::HierarchyDoesNotSupportBlendMode(hierarchy_id))
                }
            })
    }

    pub(crate) fn layer_blend_mode(
        &self,
        hierarchy_id: silica::HierarchyId,
    ) -> Option<Result<silica::BlendingMode, SilicaError>> {
        self.layer_target(hierarchy_id).map(|target| match target {
            LayerTarget::Layer(layer) => Ok(layer.blend),
            LayerTarget::Unsupported => {
                Err(SilicaError::HierarchyDoesNotSupportBlendMode(hierarchy_id))
            }
        })
    }

    fn layer_target(&self, hierarchy_id: silica::HierarchyId) -> Option<LayerTarget<'_>> {
        match self {
            SilicaHierarchy::Layer(layer) => {
                if layer.hierarchy_id() == hierarchy_id {
                    return Some(LayerTarget::Layer(layer));
                }
                if layer
                    .mask
                    .as_ref()
                    .is_some_and(|mask| mask.hierarchy_id() == hierarchy_id)
                {
                    return Some(LayerTarget::Unsupported);
                }
                None
            }
            SilicaHierarchy::Group(group) => {
                if group.hierarchy_id() == hierarchy_id {
                    return Some(LayerTarget::Unsupported);
                }
                group
                    .children
                    .iter()
                    .find_map(|child| child.layer_target(hierarchy_id))
            }
        }
    }

    fn layer_target_mut(
        &mut self,
        hierarchy_id: silica::HierarchyId,
    ) -> Option<LayerTargetMut<'_>> {
        match self {
            SilicaHierarchy::Layer(layer) => {
                if layer.hierarchy_id() == hierarchy_id {
                    return Some(LayerTargetMut::Layer(layer));
                }
                if layer
                    .mask
                    .as_ref()
                    .is_some_and(|mask| mask.hierarchy_id() == hierarchy_id)
                {
                    return Some(LayerTargetMut::Unsupported);
                }
                None
            }
            SilicaHierarchy::Group(group) => {
                if group.hierarchy_id() == hierarchy_id {
                    return Some(LayerTargetMut::Unsupported);
                }
                group
                    .children
                    .iter_mut()
                    .find_map(|child| child.layer_target_mut(hierarchy_id))
            }
        }
    }
}

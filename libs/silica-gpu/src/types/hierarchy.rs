use crate::error::SilicaError;
use crate::params::LoadParams;
use crate::types::{group::SilicaGroup, layer::SilicaLayer};

#[derive(Debug, Clone, PartialEq)]
pub enum SilicaHierarchy {
    Layer(SilicaLayer),
    Group(SilicaGroup),
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
        match self {
            SilicaHierarchy::Layer(layer) => {
                if layer.hierarchy_id() == hierarchy_id {
                    let changed = layer.clipped != clipped;
                    layer.clipped = clipped;
                    return Some(Ok(changed));
                }
                if layer
                    .mask
                    .as_ref()
                    .is_some_and(|mask| mask.hierarchy_id() == hierarchy_id)
                {
                    return Some(Err(SilicaError::HierarchyDoesNotSupportClipping(
                        hierarchy_id,
                    )));
                }
                None
            }
            SilicaHierarchy::Group(group) => {
                if group.hierarchy_id() == hierarchy_id {
                    return Some(Err(SilicaError::HierarchyDoesNotSupportClipping(
                        hierarchy_id,
                    )));
                }
                group
                    .children
                    .iter_mut()
                    .find_map(|child| child.set_layer_clipped(hierarchy_id, clipped))
            }
        }
    }

    pub(crate) fn layer_clipped(
        &self,
        hierarchy_id: silica::HierarchyId,
    ) -> Option<Result<bool, SilicaError>> {
        match self {
            SilicaHierarchy::Layer(layer) => {
                if layer.hierarchy_id() == hierarchy_id {
                    return Some(Ok(layer.clipped));
                }
                if layer
                    .mask
                    .as_ref()
                    .is_some_and(|mask| mask.hierarchy_id() == hierarchy_id)
                {
                    return Some(Err(SilicaError::HierarchyDoesNotSupportClipping(
                        hierarchy_id,
                    )));
                }
                None
            }
            SilicaHierarchy::Group(group) => {
                if group.hierarchy_id() == hierarchy_id {
                    return Some(Err(SilicaError::HierarchyDoesNotSupportClipping(
                        hierarchy_id,
                    )));
                }
                group
                    .children
                    .iter()
                    .find_map(|child| child.layer_clipped(hierarchy_id))
            }
        }
    }
}

use plist::{Dictionary, Value};

use crate::{
    ns_archive::{NsArchive, NsDecode, NsObjects, error::NsArchiveError},
    types::hierarchy::{HierarchyId, SilicaHierarchy},
};

#[derive(Debug, Clone, PartialEq)]
pub struct SilicaGroup {
    hierarchy_id: HierarchyId,
    pub name: Option<String>,
    pub hidden: bool,
    pub children: Vec<SilicaHierarchy>,
}

impl<'a> NsDecode<'a> for SilicaGroup {
    fn decode(nka: &'a NsArchive, key: &'a str, val: &'a Value) -> Result<Self, NsArchiveError> {
        let refs = nka.bind(<&'_ Dictionary>::decode(nka, key, val)?);

        Ok(Self {
            hierarchy_id: HierarchyId::UNASSIGNED,
            hidden: refs.resolve::<bool>("isHidden")?,
            name: refs.resolve::<Option<String>>("name")?,
            children: refs
                .resolve::<NsObjects<SilicaHierarchy>>("children")?
                .objects,
        })
    }
}

impl SilicaGroup {
    pub const fn hierarchy_id(&self) -> HierarchyId {
        self.hierarchy_id
    }

    pub(crate) fn set_hierarchy_id(&mut self, hierarchy_id: HierarchyId) {
        self.hierarchy_id = hierarchy_id;
    }
}

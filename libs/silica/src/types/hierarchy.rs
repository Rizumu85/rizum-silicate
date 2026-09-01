use crate::{
    ns_archive::{NsArchive, NsClass, NsDecode, error::NsArchiveError},
    types::{group::SilicaGroup, layer::SilicaLayer},
};
use plist::{Dictionary, Value};

/// Stable renderer-neutral identity assigned when a document hierarchy is parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HierarchyId(u64);

impl HierarchyId {
    pub(crate) const UNASSIGNED: Self = Self(u64::MAX);

    /// Reconstructs an identity at a runtime or renderer adapter boundary.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the transport-safe numeric representation.
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SilicaHierarchy {
    Layer(SilicaLayer),
    Group(SilicaGroup),
}

impl<'a> NsDecode<'a> for SilicaHierarchy {
    fn decode(nka: &'a NsArchive, key: &'a str, val: &'a Value) -> Result<Self, NsArchiveError> {
        let refs = nka.bind(<&'_ Dictionary>::decode(nka, key, val)?);
        let class = refs.resolve::<NsClass>("$class")?;

        match class.class_name.as_str() {
            "SilicaGroup" => Ok(SilicaGroup::decode(nka, key, val).map(Self::Group)?),
            "SilicaLayer" => Ok(SilicaLayer::decode(nka, key, val).map(Self::Layer)?),
            _ => Err(NsArchiveError::TypeMismatch("$class".to_string())),
        }
    }
}

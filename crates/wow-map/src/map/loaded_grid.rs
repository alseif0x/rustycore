//! Loaded-grid materialization inputs and immutable successful-add facts.

use wow_core::ObjectGuid;
use wow_entities::MapObjectRecord;

#[derive(Debug, PartialEq)]
pub struct LoadedGridRespawnRecordsLikeCpp {
    pub pre_add_records: Vec<MapObjectRecord>,
    pub primary_record: MapObjectRecord,
}

impl LoadedGridRespawnRecordsLikeCpp {
    pub fn primary_only(primary_record: MapObjectRecord) -> Self {
        Self {
            pre_add_records: Vec::new(),
            primary_record,
        }
    }
}

/// Identity facts captured before a loaded-grid primary record is consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadedGridObjectFacts {
    object_guid: ObjectGuid,
    typed_creature_guid: Option<ObjectGuid>,
}

impl LoadedGridObjectFacts {
    pub(super) fn from_record(record: &MapObjectRecord) -> Self {
        Self {
            object_guid: record.object().guid(),
            typed_creature_guid: record.creature().map(|creature| creature.guid()),
        }
    }

    pub const fn object_guid(&self) -> ObjectGuid {
        self.object_guid
    }

    pub const fn typed_creature_guid(&self) -> Option<ObjectGuid> {
        self.typed_creature_guid
    }
}

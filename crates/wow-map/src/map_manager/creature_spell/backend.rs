//! Private concrete storage adapters; both invoke the same spell algorithms.
use super::*;
use crate::map::{Map, CreatureActorWitness};

pub(crate) enum SpellMap<'a> {
    Legacy { manager: &'a mut LegacyMapManager, map: Option<&'a mut Map>, map_id: u16, instance_id: u32 },
    Canonical { map: &'a mut Map, witnesses: &'a HashMap<ObjectGuid, CreatureActorWitness> },
}
impl SpellMap<'_> {
    pub(super) fn actor(&self, guid: ObjectGuid) -> Option<&WorldCreature> {
        match self {
            Self::Legacy { manager, map_id, instance_id, .. } => manager.find_creature(*map_id, *instance_id, guid),
            Self::Canonical { map, witnesses } => {
                let expected = witnesses.get(&guid)?;
                let current = map.creature_actor_witness(guid)?;
                expected.same_actor(&current).then(|| map.creature_actor(guid)).flatten()
            }
        }
    }
    pub(super) fn actor_mut(&mut self, guid: ObjectGuid) -> Option<&mut WorldCreature> {
        match self {
            Self::Legacy { manager, map_id, instance_id, .. } => manager.find_creature_mut(*map_id, *instance_id, guid),
            Self::Canonical { map, witnesses } => {
                let expected = witnesses.get(&guid)?;
                let current = map.creature_actor_witness(guid)?;
                if !expected.same_actor(&current) { return None; }
                map.creature_actor_mut(guid)
            }
        }
    }
    pub(super) fn map(&self) -> Option<&Map> {
        match self { Self::Legacy { map, .. } => map.as_deref(), Self::Canonical { map, .. } => Some(map) }
    }
    pub(super) fn map_mut(&mut self) -> Option<&mut Map> {
        match self { Self::Legacy { map, .. } => map.as_deref_mut(), Self::Canonical { map, .. } => Some(map) }
    }
}

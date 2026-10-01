// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn read_legacy_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.read_legacy_creature_loot_authority_like_cpp(hub, guid)
    }
    pub(crate) fn read_canonical_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.read_canonical_creature_loot_authority_like_cpp(hub, guid)
    }
    pub(crate) fn read_canonical_gameobject_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.read_canonical_gameobject_loot_authority_like_cpp(hub, guid)
    }
    pub(crate) fn loot_specialization_id_like_cpp(&self) -> Option<u32> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.loot_specialization_id_like_cpp(hub)
    }
    pub(crate) fn set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
        &mut self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let (owner, mut hub) = crate::session::split_loot_mut(self);
        owner.set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
            &mut hub,
            guid,
            authority,
            object_generation,
            lifecycle_revision,
            state,
            unit_guid,
            chest_restock_time_secs,
            shared_loot_is_changed_like_cpp,
        )
    }
    pub(crate) fn set_active_loot_guid(&mut self, guid: ObjectGuid) {
        self.loot.set_active_loot_guid(guid)
    }
    pub(crate) fn durable_loot_money_persistence_tracker_like_cpp(
        &self,
    ) -> Arc<DurableLootMoneyPersistenceTrackerLikeCpp> {
        crate::session::cx_loot_ref(self).durable_loot_money_persistence_tracker_like_cpp()
    }
    pub(crate) fn is_active_loot_guid(&self, guid: ObjectGuid) -> bool {
        self.loot.is_active_loot_guid(guid)
    }
}

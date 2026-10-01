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
}

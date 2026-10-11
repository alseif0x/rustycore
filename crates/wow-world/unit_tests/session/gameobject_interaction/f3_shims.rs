// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;
use crate::session::{
    ObjectGuid, SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP,
    SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP, SummonPropertiesEntry, WorldSession,
};

impl crate::session::WorldSession {
    pub(crate) fn release_canonical_fishing_hole_like_cpp(
        &mut self,
        guid: ObjectGuid,
        max_opens: Option<u32>,
    ) -> Option<(
        u32,
        wow_entities::LootState,
        wow_map::map::GameObjectSetLootStateOutcomeLikeCpp,
    )> {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.release_canonical_fishing_hole_like_cpp(&mut hub, guid, max_opens)
    }
}

impl WorldSession {
    pub(crate) fn summon_private_object_owner_like_cpp(
        &self,
        caster_guid: ObjectGuid,
        caster_private_object_owner: ObjectGuid,
        properties: &SummonPropertiesEntry,
    ) -> ObjectGuid {
        let flags = u32::try_from(properties.flags[0]).unwrap_or(0);
        let only_visible_to_summoner =
            flags & SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP != 0;
        let only_visible_to_summoner_group =
            flags & SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP != 0;
        if !only_visible_to_summoner && !only_visible_to_summoner_group {
            return ObjectGuid::EMPTY;
        }

        if !caster_private_object_owner.is_empty() {
            return caster_private_object_owner;
        }

        if only_visible_to_summoner_group && caster_guid.is_player() {
            if let Some(group_guid) = self.resolved_group_guid_like_cpp() {
                return ObjectGuid::create_group(group_guid);
            }
        }

        caster_guid
    }
}

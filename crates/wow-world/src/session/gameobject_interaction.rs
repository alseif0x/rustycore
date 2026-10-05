// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Gameobject interaction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::ObjectGuid;
use super::SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP;
use super::WorldSession;
use super::{SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP, SummonPropertiesEntry};

#[cfg(test)]
pub(crate) use wow_world_entities::RepresentedGameObjectCriteriaEvent;
pub(crate) use wow_world_entities::{
    BattlegroundFlagDropClickTarget, RepresentedBattlegroundObjectUseRejection,
    RepresentedCapturePointStateLikeCpp, RepresentedGameObjectUseEffect,
    RepresentedGameObjectUseState, RepresentedNewFlagStateRequest,
};

impl WorldSession {
    pub fn summon_private_object_owner_like_cpp(
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

#[cfg(test)]
#[path = "../../unit_tests/session/gameobject_interaction/f3_shims.rs"]
mod f3_shims;

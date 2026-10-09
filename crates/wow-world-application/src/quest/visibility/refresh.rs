// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Complete gameobject and spell-click visibility refresh scans.
//!
//! C++ anchor: `Player::UpdateVisibleGameobjectsOrSpellClicks`
//! (`src/server/game/Entities/Player/Player.cpp:24433` at
//! `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`). Moved out of the Session
//! boundary under #1263 F4. The scans keep their skip gates, the selected
//! eligibility/condition reads and the GameObject-then-spell-click
//! publication order; only the access path changed. The World adapter lends
//! the participants and the two packet projections it cannot reach from this
//! crate, and holds no scan order of its own.

use wow_core::ObjectGuid;
use wow_data::spell_click::UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP;
use wow_packet::packets::update::{UnitDataValuesDeltaUpdate, UpdateObject};
use wow_world_core::session::{SessionCatalogs, SessionCore};
use wow_world_entities::{RepresentedGameObjectUseState, WorldEntitiesState};

use super::{
    represented_gameobject_is_for_quests_like_cpp, represented_has_quest_for_gameobject_like_cpp,
};
use crate::SessionQuestState;

/// Participants the World shell lends to the moved scans.
///
/// These are the exact owners the original bodies read: the visible identity
/// set and canonical gameobject lookup, the represented use-state store, and
/// the catalogs/quest state the existing eligibility and condition providers
/// consume.
pub struct VisibilityRefreshCxLikeCpp<'a> {
    pub core: &'a SessionCore,
    pub catalogs: &'a SessionCatalogs,
    pub quest_state: &'a SessionQuestState,
    pub world_entities: &'a WorldEntitiesState,
    pub consumer_test: bool,
}

/// Session capabilities the moved scans invoke at their original points.
///
/// Each method is an existing World operation the scan body already called:
/// the viewer-dependent `DynamicFlags` projection, the GameObject values-update
/// projection and the creature `NpcFlags` values-update projection with its
/// viewer-dependent spell-click filter. This trait adds no session behaviour.
pub trait VisibilityRefreshHostLikeCpp {
    fn visibility_refresh_gameobject_dynamic_flags_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> u32;

    fn visibility_refresh_gameobject_values_update_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        dynamic_flags: u32,
    ) -> Option<UpdateObject>;

    fn visibility_refresh_unit_npc_flags_update_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        packet_update: UnitDataValuesDeltaUpdate,
    ) -> UpdateObject;
}

/// C++ `Player::UpdateVisibleGameobjectsOrSpellClicks` GameObject half.
///
/// Preserved skips: a visible identity that is not a gameobject, a missing
/// canonical gameobject (absent owner), a missing represented use-state, a
/// gameobject that neither advances an active objective nor is for quests, and
/// a `DynamicFlags` update the projection refuses to build. None of them
/// publishes.
pub fn update_visible_gameobjects_like_cpp<H>(
    host: &H,
    cx: &VisibilityRefreshCxLikeCpp<'_>,
) -> usize
where
    H: VisibilityRefreshHostLikeCpp,
{
    let mut sent = 0;
    let visible_guids = cx
        .core
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .collect::<Vec<_>>();
    for guid in visible_guids {
        if !guid.is_game_object() {
            continue;
        }
        let Some(access) = cx.core.canonical_gameobject_access_like_cpp(guid) else {
            continue;
        };
        let Some(state) = cx
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid)
            .cloned()
        else {
            continue;
        };
        let objective_refresh = represented_has_quest_for_gameobject_like_cpp(
            &cx.core.quest_objective_access_like_cpp(),
            cx.catalogs,
            cx.quest_state,
            access.entry,
            cx.consumer_test,
        );
        let relation_refresh = represented_gameobject_is_for_quests_like_cpp(
            cx.catalogs,
            &cx.core.quest_objective_access_like_cpp(),
            cx.quest_state,
            access.entry,
            &state,
            cx.consumer_test,
        );
        if !objective_refresh && !relation_refresh {
            continue;
        }

        let dynamic_flags =
            host.visibility_refresh_gameobject_dynamic_flags_like_cpp(access.entry, &state);
        let Some(update) = host.visibility_refresh_gameobject_values_update_like_cpp(
            guid,
            cx.core.player_map_id_like_cpp(),
            dynamic_flags,
        ) else {
            continue;
        };
        cx.core.send_packet(&update);
        sent += 1;
    }

    sent
}

/// C++ `Player::UpdateVisibleGameobjectsOrSpellClicks` creature/spell-click half.
///
/// Preserved skips: an absent spell-click store or condition store aborts the
/// whole scan without publishing, a visible identity that is not a creature or
/// vehicle, a missing represented creature snapshot (absent owner), a creature
/// without `UNIT_NPC_FLAG_SPELLCLICK`, and click bounds whose any row has no
/// conditions for the spell-click event.
pub fn update_visible_spell_clicks_like_cpp<H>(
    host: &H,
    cx: &VisibilityRefreshCxLikeCpp<'_>,
) -> usize
where
    H: VisibilityRefreshHostLikeCpp,
{
    let Some(spell_click_store) = cx.catalogs.spell_catalogs.npc_spell_click_store.as_ref() else {
        return 0;
    };
    let Some(condition_store) = cx.catalogs.condition_store.as_ref() else {
        return 0;
    };

    let mut sent = 0;
    let visible_guids = cx
        .core
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .collect::<Vec<_>>();
    for guid in visible_guids {
        if !guid.is_creature_or_vehicle() {
            continue;
        }
        let Some(creature) = cx
            .world_entities
            .represented_spell_click_creature_snapshot_like_cpp(
                &cx.core.quest_objective_access_like_cpp(),
                guid,
            )
        else {
            continue;
        };
        if (u64::from(creature.npc_flags) & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP) == 0 {
            continue;
        }
        let click_bounds = spell_click_store.spell_click_info_map_bounds_like_cpp(creature.entry);
        if !click_bounds.iter().any(|click| {
            wow_conditions::has_conditions_for_spell_click_event_like_cpp(
                condition_store,
                creature.entry,
                click.spell_id,
            )
        }) {
            continue;
        }

        let mut packet_update = UnitDataValuesDeltaUpdate::default();
        packet_update.changed_object_type_mask = 1 << wow_entities::TYPEID_UNIT;
        packet_update.unit_data_mask[113 / 32] |= 1 << (113 % 32);
        packet_update.unit_data_mask[114 / 32] |= 1 << (114 % 32);
        packet_update.npc_flags = [creature.npc_flags, 0];
        let update = host.visibility_refresh_unit_npc_flags_update_like_cpp(
            guid,
            cx.core.player_map_id_like_cpp(),
            packet_update,
        );
        cx.core.send_packet(&update);
        sent += 1;
    }

    sent
}

/// The combined refresh, preserving the established publication order: every
/// GameObject update of the first scan precedes every spell-click update of the
/// second.
pub fn update_visible_gameobjects_or_spell_clicks_like_cpp<H>(
    host: &H,
    cx: &VisibilityRefreshCxLikeCpp<'_>,
) -> usize
where
    H: VisibilityRefreshHostLikeCpp,
{
    update_visible_gameobjects_like_cpp(host, cx) + update_visible_spell_clicks_like_cpp(host, cx)
}

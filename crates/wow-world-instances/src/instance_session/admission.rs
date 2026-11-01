// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `Map::PlayerCannotEnter` for a dungeon target, the `MapManager`
//! create-map player context, and the active instance lock it reads
//! (#1363 slice 11b; moved from the World `session/instances/{map_key,binding,
//! instance}.rs` and `session/map_admission.rs`).

use wow_core::ObjectGuid;
use wow_world_core::session::connection_identity::unix_now;
use wow_world_core::session::player_team_id_for_race_cpp;
use wow_world_entities::TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP;

use crate::create_map_instance_lock_token_like_cpp;

use super::{
    InstanceSessionHostLikeCpp, TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
    TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP, TRANSFER_ABORT_NEED_GROUP_LIKE_CPP,
    TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP, TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP,
};

fn create_map_decision_key_like_cpp(
    decision: &wow_map::CreateMapDecision,
) -> Option<wow_map::MapKey> {
    match decision {
        wow_map::CreateMapDecision::Existing { key, .. }
        | wow_map::CreateMapDecision::Create { key, .. } => Some(*key),
        wow_map::CreateMapDecision::Reject { .. } => None,
    }
}

pub fn create_map_player_context_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
    map_id: u32,
    map_entry: wow_data::map::MapEntry,
    player_guid: ObjectGuid,
) -> Option<wow_map::CreateMapPlayerContext> {
    let player_difficulty_id = {
        let (state, hub) = host.split_instances_ref();
        state.represented_player_difficulty_id_for_map_entry_like_cpp(hub, map_id, map_entry)
    }?;

    let group = host
        .resolved_group_guid_like_cpp()
        .and_then(|group_guid| {
            host.core()
                .directory
                .group_registry
                .as_ref()?
                .get(&group_guid)
        })
        .map(|group| {
            let difficulty_id = {
                let (state, hub) = host.split_instances_ref();
                state.represented_group_difficulty_id_for_map_entry_like_cpp(
                    hub, map_id, map_entry, &group,
                )
            };
            wow_map::CreateMapGroupContext {
                difficulty_id,
                recent_instance_owner_guid_counter: group
                    .recent_instance_owner_like_cpp(map_id)
                    .counter() as u64,
                recent_instance_id: group.recent_instance_id_like_cpp(map_id),
            }
        });

    Some(wow_map::CreateMapPlayerContext {
        guid_counter: player_guid.counter() as u64,
        team_id: player_team_id_for_race_cpp(host.hub_ref().player_race_like_cpp()),
        battleground_id: 0,
        has_battleground: false,
        player_difficulty_id,
        player_recent_instance_id: {
            let (state, hub) = host.split_instances_ref();
            state.resolved_player_recent_instance_id_like_cpp(hub, map_id)
        }?,
        group,
    })
}
pub fn player_cannot_enter_target_map_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
    map_id: u32,
) -> Option<(u32, u8, i32)> {
    let Some(map_store) = host.hub_ref().catalogs.maps.store.as_ref() else {
        return None;
    };
    let Some(map_entry) = map_store.get(map_id).copied() else {
        return Some((TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP, 0, 0));
    };
    if !map_entry.is_dungeon() {
        return None;
    }

    let player_guid = host.core().player_guid?;
    let player = create_map_player_context_like_cpp(host, map_id, map_entry, player_guid)?;
    let requested_difficulty = player
        .group
        .map(|group| group.difficulty_id)
        .unwrap_or(player.player_difficulty_id);

    if {
        let (state, hub) = host.split_instances_ref();
        state.create_map_db2_entries_like_cpp(hub, map_id, requested_difficulty)
    }
    .is_none()
    {
        return Some((TRANSFER_ABORT_DIFFICULTY_LIKE_CPP, 0, 0));
    }

    if host.hub_ref().player_is_game_master_like_cpp() == Some(true) {
        return None;
    }

    if let Some(abort) = host.access_requirement_abort_like_cpp(map_id, requested_difficulty as u8)
    {
        return Some(abort);
    }

    if map_entry.instance_type == wow_data::map::MAP_RAID
        && map_entry.expansion_like_cpp() >= host.core().realm_policy.server_expansion_like_cpp
        && !host.core().realm_policy.instance_ignore_raid_like_cpp
        && !host.current_player_is_in_raid_group_like_cpp()
    {
        return Some((TRANSFER_ABORT_NEED_GROUP_LIKE_CPP, 0, 0));
    }

    let Some(canonical_map_manager) = host.core().canonical_map_manager.as_ref() else {
        return None;
    };
    let entry = wow_map::CreateMapEntryContext {
        map_id,
        kind: wow_map::CreateMapEntryKind::Dungeon,
        split_by_faction: map_entry.is_split_by_faction(),
        flex_locking: map_entry.is_flex_locking(),
    };
    let active_instance_lock =
        create_map_active_instance_lock_context_like_cpp(host, map_id, requested_difficulty);
    let mut manager = canonical_map_manager.lock().ok()?;
    let decision = manager.create_map_decision_like_cpp(
        Some(entry),
        Some(player),
        |candidate_map_id, difficulty_id| {
            let (state, hub) = host.split_instances_ref();
            state.create_map_difficulty_context_like_cpp(hub, candidate_map_id, difficulty_id)
        },
        active_instance_lock,
        |_, _| None,
    );

    let existing_instance_lock_context = match &decision {
        wow_map::CreateMapDecision::Existing { key, .. } => manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|map| map.instance_lock_context()),
        _ => None,
    };
    let existing_instance_player_count = match &decision {
        wow_map::CreateMapDecision::Existing { key, .. } => manager
            .find_map(key.map_id, key.instance_id)
            .map(|map| map.players_count_except_gms_like_cpp()),
        _ => None,
    };
    let existing_instance_encounter_in_progress = match &decision {
        wow_map::CreateMapDecision::Existing { key, .. } => manager
            .find_map(key.map_id, key.instance_id)
            .map(|map| map.instance_encounter_in_progress_like_cpp()),
        _ => None,
    };
    let decision_key = create_map_decision_key_like_cpp(&decision);
    drop(manager);

    if let wow_map::CreateMapDecision::Existing {
        key, difficulty_id, ..
    } = &decision
    {
        if let Some(player_count) = existing_instance_player_count
            && let Some(entries) = {
                let (state, hub) = host.split_instances_ref();
                state.create_map_db2_entries_like_cpp(hub, key.map_id, *difficulty_id)
            }
            && player_count >= entries.max_players
        {
            return Some((TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP, 0, 0));
        }

        if map_entry.instance_type == wow_data::map::MAP_RAID
            && host.player_loading() != Some(player_guid)
            && existing_instance_encounter_in_progress == Some(true)
        {
            return Some((TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP, 0, 0));
        }

        if let Some(lock_context) = existing_instance_lock_context {
            let deny_reason = {
                let (state, hub) = host.split_instances_ref();
                state.cannot_enter_existing_instance_lock_like_cpp(
                    hub,
                    key.map_id,
                    *difficulty_id,
                    lock_context,
                )
            }
            .unwrap_or(wow_instances::TransferAbortReason::None);
            if deny_reason != wow_instances::TransferAbortReason::None {
                return Some((deny_reason as u32, 0, 0));
            }
        }
    }

    if !map_entry.ignores_instance_farm_limit_like_cpp()
        && let Some(key) = decision_key
        && !{
            let (state, hub) = host.split_instances_ref();
            state.check_instance_count_probe_like_cpp(hub, key.instance_id)
        }
        && host.hub_ref().resolved_player_is_alive_like_cpp() == Some(true)
    {
        return Some((TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP, 0, 0));
    }

    None
}
pub(super) fn set_active_instance_lock_instance_id_side_effect_like_cpp<
    H: InstanceSessionHostLikeCpp,
>(
    host: &H,
    map_id: u32,
    difficulty_id: wow_map::Difficulty,
    instance_id: u32,
) -> Option<()> {
    let entries = {
        let (state, hub) = host.split_instances_ref();
        state.create_map_db2_entries_like_cpp(hub, map_id, difficulty_id)
    }?;
    let owner_guid = create_map_instance_owner_guid_like_cpp(host, map_id)?;
    let now = u64::try_from(unix_now()).unwrap_or(0);
    let mgr = host.core().instance_lock_mgr.as_ref()?;
    let mut mgr = mgr.write().ok()?;
    mgr.set_active_instance_lock_instance_id_at(owner_guid, &entries, now, instance_id)
        .then_some(())
}
pub fn create_map_active_instance_lock_context_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
    map_id: u32,
    difficulty_id: wow_map::Difficulty,
) -> Option<wow_map::CreateMapInstanceLockContext> {
    let entries = {
        let (state, hub) = host.split_instances_ref();
        state.create_map_db2_entries_like_cpp(hub, map_id, difficulty_id)
    }?;
    let owner_guid = create_map_instance_owner_guid_like_cpp(host, map_id)?;
    let now = u64::try_from(unix_now()).unwrap_or(0);
    let mgr = host.core().instance_lock_mgr.as_ref()?;
    let mgr = mgr.read().ok()?;
    let lock = mgr.find_active_instance_lock_at(owner_guid, &entries, now)?;

    Some(wow_map::CreateMapInstanceLockContext {
        instance_id: lock.instance_id,
        difficulty_id: lock.difficulty_id,
        token: create_map_instance_lock_token_like_cpp(owner_guid, &entries, lock),
        owner_guid_counter: owner_guid.counter() as u64,
    })
}
pub fn create_map_instance_owner_guid_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
    map_id: u32,
) -> Option<ObjectGuid> {
    // #743: C++ reads `Player::GetGroup()`, which `Group::RemoveMember`
    // and `Group::Disband` clear in the same operation. Resolve instance
    // ownership through the authority so a member removed while a
    // notification is still queued cannot keep the group's instance.
    host.authoritative_group_membership_like_cpp()
        .and_then(|group_guid| {
            host.core()
                .directory
                .group_registry
                .as_ref()?
                .get(&group_guid)
        })
        .map(|group| group.recent_instance_owner_like_cpp(map_id))
        .or(host.core().player_guid)
}

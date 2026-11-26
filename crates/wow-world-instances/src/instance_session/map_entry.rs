// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map selection/admission and final Player attachment are separate synchronous phases.
//! C++ MapManager::CreateMap (MapManager.cpp:139-231) returns a map;
//! Player/WorldSession owns the subsequent binding and AddPlayerToMap.
//! The compatibility facade preserves the existing Rust order and failure results.
//! (#1363 slice 13a; moved from the World `session/lifecycle/map_entry.rs`,
//! with the canonical Player residence from `session/instances/map_resolution.rs`
//! and the decision difficulty from `session/map_admission.rs`.)
use std::sync::Arc;
use tracing::warn;
use wow_core::Position;
use wow_world_core::session::connection_identity::unix_now;
use wow_world_core::session::map_admission::PlayerGridLoadResolverLikeCpp;

use super::{
    InstanceSessionHostLikeCpp, TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
    TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP, TRANSFER_ABORT_NEED_GROUP_LIKE_CPP,
    TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP, TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP,
    apply_create_map_side_effects_like_cpp,
    create_map_active_instance_lock_context_with_expiry_like_cpp,
    create_map_player_context_like_cpp,
};

fn create_map_decision_difficulty_id_like_cpp(
    decision: &wow_map::CreateMapDecision,
) -> Option<wow_map::Difficulty> {
    match decision {
        wow_map::CreateMapDecision::Existing { difficulty_id, .. }
        | wow_map::CreateMapDecision::Create { difficulty_id, .. } => Some(*difficulty_id),
        wow_map::CreateMapDecision::Reject { .. } => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MapEntryPhaseLikeCpp {
    Login,
    Transfer,
}

/// Resolve the one canonical Player identity and move that exact value to
/// the selected map. Existing map records predate Player handles, so the
/// transition must adopt them before considering a new initial value.
pub fn ensure_canonical_player_owner_for_map_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    key: wow_map::MapKey,
    position: Position,
) -> bool {
    if !host.ensure_canonical_player_owner_exists_like_cpp(key) {
        return false;
    }
    let Some(manager) = host.core().canonical_map_manager.as_ref().map(Arc::clone) else {
        return false;
    };

    let Some(handle) = host.core().player_handle_like_cpp else {
        return false;
    };
    let Ok(mut manager) = manager.lock() else {
        return false;
    };
    match manager.player_residence_like_cpp(handle) {
        Some(wow_map::PlayerResidenceLikeCpp::Active(current)) if current == key => manager
            .relocate_player_like_cpp(handle, position)
            .is_ok_and(|outcome| outcome.relocated),
        Some(wow_map::PlayerResidenceLikeCpp::Active(_)) => {
            manager.detach_player_like_cpp(handle).is_ok()
                && manager
                    .attach_player_like_cpp(handle, key, position)
                    .is_ok()
        }
        Some(wow_map::PlayerResidenceLikeCpp::Detached) => manager
            .attach_player_like_cpp(handle, key, position)
            .is_ok(),
        None => false,
    }
}

/// Synchronous entry attempt. Failure retains the source coordinates and
/// transfer state; preparation can still create maps/admission side effects.
/// Full C++ before-add publication ordering and homebind recovery remain
/// separate obligations; this is not a reservation across asynchronous work.
pub fn try_attach_worldport_destination_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    position: wow_core::Position,
) -> bool {
    try_attach_worldport_destination_with_publication_like_cpp(host, map_id, position, true)
}

/// Attach a far-transfer Player after preparing the destination's active and
/// visible spawn grids. C++ `Map::AddPlayerToMap` calls
/// `EnsureGridLoadedForActiveObject` before adding the Player to the map;
/// worldport uses the same order so the post-add visibility scan can see the
/// destination's grid-loaded objects.
pub fn try_attach_worldport_destination_with_grid_load_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    position: wow_core::Position,
    player_grid_loader: &PlayerGridLoadResolverLikeCpp,
) -> bool {
    attach_worldport_destination(host, map_id, position, true, Some(player_grid_loader))
}

pub fn try_attach_worldport_destination_with_publication_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    position: wow_core::Position,
    publish: bool,
) -> bool {
    attach_worldport_destination(host, map_id, position, publish, None)
}

fn attach_worldport_destination<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    position: wow_core::Position,
    publish: bool,
    player_grid_loader: Option<&PlayerGridLoadResolverLikeCpp>,
) -> bool {
    if host
        .hub_ref()
        .player_teleport_state_snapshot_like_cpp()
        .is_some_and(|state| state.recovery == wow_entities::PlayerTransferRecovery::Terminal)
    {
        return false;
    }
    let Ok(map_id_u16) = u16::try_from(map_id) else {
        return false;
    };
    if !position.is_valid_map_coord_like_cpp() {
        return false;
    }
    let Some(
        wow_map::CreateMapDecision::Existing { key, .. }
        | wow_map::CreateMapDecision::Create { key, .. },
    ) = prepare_canonical_map_entry_with_publication_like_cpp(
        host,
        map_id,
        publish,
        MapEntryPhaseLikeCpp::Transfer,
        true,
    )
    else {
        return false;
    };
    if key.map_id != map_id
        || player_grid_loader
            .is_some_and(|load| load(map_id_u16, Some(key.instance_id), position).map_unavailable)
    {
        clear_pending_map_entry_like_cpp(host, key);
        return false;
    }
    if !host
        .hub_mut()
        .remove_current_player_from_canonical_current_map_like_cpp()
    {
        clear_pending_map_entry_like_cpp(host, key);
        return false;
    }
    if !ensure_canonical_player_owner_for_map_like_cpp(host, key, position) {
        clear_pending_map_entry_like_cpp(host, key);
        return false;
    }
    host.hub_mut()
        .set_player_map_position_like_cpp(map_id_u16, position);
    let _ = host
        .hub_mut()
        .update_player_teleport_state_like_cpp(|state| {
            state.recovery = wow_entities::PlayerTransferRecovery::None;
        });
    true
}

pub fn ensure_canonical_world_map_for_current_player_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
) -> Option<wow_map::CreateMapDecision> {
    ensure_canonical_map_for_current_player_in_phase_like_cpp(host, MapEntryPhaseLikeCpp::Transfer)
}

/// Select and attach the saved login map using `Player::LoadFromDB` admission.
/// Login creates the map and calls `Map::CannotEnter`; it does not run the
/// separate `Map::PlayerCannotEnter` access and raid-group preflight.
pub fn ensure_canonical_login_map_for_current_player_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
) -> Option<wow_map::CreateMapDecision> {
    ensure_canonical_map_for_current_player_in_phase_like_cpp(host, MapEntryPhaseLikeCpp::Login)
}

fn ensure_canonical_map_for_current_player_in_phase_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    phase: MapEntryPhaseLikeCpp,
) -> Option<wow_map::CreateMapDecision> {
    let map_id = u32::from(host.core().player_map_id_like_cpp());
    let position = host.hub_ref().player_position_like_cpp()?;
    let decision = prepare_canonical_map_entry_for_phase_like_cpp(host, map_id, phase, true)?;
    if let wow_map::CreateMapDecision::Existing { key, .. }
    | wow_map::CreateMapDecision::Create { key, .. } = &decision
        && !ensure_canonical_player_owner_for_map_like_cpp(host, *key, position)
    {
        clear_pending_map_entry_like_cpp(host, *key);
        return None;
    }
    Some(decision)
}

/// Prepare/create the destination and its existing admission side effects,
/// without relocating or attaching Player. This is not a reservation or a
/// pure query: callers must not carry this decision across asynchronous work.
pub fn prepare_canonical_map_entry_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
) -> Option<wow_map::CreateMapDecision> {
    prepare_canonical_map_entry_with_publication_like_cpp(
        host,
        map_id,
        true,
        MapEntryPhaseLikeCpp::Transfer,
        false,
    )
}

fn prepare_canonical_map_entry_for_phase_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    phase: MapEntryPhaseLikeCpp,
    mark_pending_entry: bool,
) -> Option<wow_map::CreateMapDecision> {
    prepare_canonical_map_entry_with_publication_like_cpp(
        host,
        map_id,
        true,
        phase,
        mark_pending_entry,
    )
}

fn prepare_canonical_map_entry_with_publication_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    publish: bool,
    phase: MapEntryPhaseLikeCpp,
    mark_pending_entry: bool,
) -> Option<wow_map::CreateMapDecision> {
    let map_entry = host
        .hub_ref()
        .catalogs
        .maps
        .store
        .as_ref()?
        .get(map_id)
        .copied()?;
    if map_entry.is_battleground_or_arena() {
        return None;
    }
    // C++ `Player::LoadFromDB` rejects a saved map requiring a newer
    // client expansion before calling `MapManager::CreateMap`
    // (Player.cpp:17577-17587). The login handler observes the missing
    // canonical key and performs the same homebind recovery.
    if host.core().expansion < map_entry.expansion_like_cpp() {
        warn!(
            account = host.core().account_id,
            map_id,
            session_expansion = host.core().expansion,
            required_expansion = map_entry.expansion_like_cpp(),
            "Login map rejected by C++ client expansion gate"
        );
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }

    let player_guid = host.core().player_guid?;
    let player = create_map_player_context_like_cpp(host, map_id, map_entry, player_guid)?;
    let is_dungeon = map_entry.is_dungeon();
    let requested_difficulty = player
        .group
        .map(|group| group.difficulty_id)
        .unwrap_or(player.player_difficulty_id);
    if is_dungeon && {
        let (state, hub) = host.split_instances_ref();
        state.create_map_db2_entries_like_cpp(hub, map_id, requested_difficulty)
    }
    .is_none()
    {
        if publish {
            host.hub_ref()
                .send_transfer_aborted_like_cpp(map_id, TRANSFER_ABORT_DIFFICULTY_LIKE_CPP);
        }
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }
    let is_game_master =
        is_dungeon && host.hub_ref().player_is_game_master_like_cpp() == Some(true);
    let run_player_cannot_enter_preflight =
        phase == MapEntryPhaseLikeCpp::Transfer && !is_game_master;
    let run_instance_map_cannot_enter = !is_game_master;
    if is_dungeon
        && run_player_cannot_enter_preflight
        && let Some((transfer_abort, arg, map_difficulty_x_condition_id)) =
            host.access_requirement_abort_like_cpp(map_id, requested_difficulty as u8)
    {
        if publish {
            host.hub_ref().send_transfer_aborted_with_params_like_cpp(
                map_id,
                transfer_abort,
                arg,
                map_difficulty_x_condition_id,
            );
        }
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }
    if is_dungeon
        && run_player_cannot_enter_preflight
        && map_entry.instance_type == wow_data::map::MAP_RAID
        && map_entry.expansion_like_cpp() >= host.core().realm_policy.server_expansion_like_cpp
        && !host.core().realm_policy.instance_ignore_raid_like_cpp
        && !host.current_player_is_in_raid_group_like_cpp()
    {
        if publish {
            host.hub_ref()
                .send_transfer_aborted_like_cpp(map_id, TRANSFER_ABORT_NEED_GROUP_LIKE_CPP);
        }
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }
    // Login follows C++ `MapManager::CreateMap` (MapManager.cpp:139-231),
    // whose final world-map branch includes garrisons and selects only
    // instance 0 / team split. Do not use Rust's represented
    // `CreateMapEntryKind::Garrison` here: that mirrors the distinct
    // `FindInstanceIdForPlayer` branch (MapManager.cpp:247-288), which is
    // not the map-creation path called by `Player::LoadFromDB`.
    let entry = wow_map::CreateMapEntryContext {
        map_id,
        kind: if is_dungeon {
            wow_map::CreateMapEntryKind::Dungeon
        } else {
            wow_map::CreateMapEntryKind::World
        },
        split_by_faction: map_entry.is_split_by_faction(),
        flex_locking: map_entry.is_flex_locking(),
    };
    let (active_instance_lock, initialization_data, expiry_context) = if is_dungeon {
        super::admission::with_active_map_lock_like_cpp(
            host,
            map_id,
            requested_difficulty,
            |context, lock, entries, reset_schedule| {
                (
                    Some(context),
                    (!lock.is_new).then(|| lock.instance_initialization_data()),
                    Some(wow_map::manager::InstanceMapExpiryContextLikeCpp {
                        current_expiry_time: lock.expiry_time,
                        entries: *entries,
                        reset_schedule,
                    }),
                )
            },
        )
        .unwrap_or_default()
    } else {
        (None, None, None)
    };
    let owning_group_guid = is_dungeon
        .then(|| host.authoritative_group_membership_like_cpp())
        .flatten();

    let manager = Arc::clone(host.core().canonical_map_manager.as_ref()?);
    let mut manager = manager.lock().ok()?;
    let decision = manager.create_map_decision_like_cpp(
        Some(entry),
        Some(player),
        |map_id, difficulty_id| {
            let (state, hub) = host.split_instances_ref();
            state.create_map_difficulty_context_like_cpp(hub, map_id, difficulty_id)
        },
        active_instance_lock,
        |_, _| None,
    );

    if let wow_map::CreateMapDecision::Create {
        key,
        difficulty_id,
        kind,
        ..
    } = &decision
    {
        let map = manager.create_map_entry(key.map_id, key.instance_id, *difficulty_id, *kind);
        if is_dungeon {
            map.set_instance_lock_context(active_instance_lock);
            map.set_instance_expiry_context_like_cpp(expiry_context);
            manager.initialize_instance_script_for_map(
                key.map_id,
                key.instance_id,
                initialization_data.as_ref(),
            );
        }
    }

    let key = decision.map_key();
    // Publish the Group backlink under the same manager guard as the map's
    // single-owner check, so a destroy/recreate of this key cannot reorder it.
    let group_link_failure = if let (Some(key), Some(group_guid)) = (key, owning_group_guid)
        && let Some(map) = manager.find_map_mut(key.map_id, key.instance_id)
        && (map.try_set_owning_group_like_cpp(group_guid)
            || map.owning_group_guid_like_cpp() == Some(group_guid))
    {
        let linked = host
            .core()
            .directory
            .group_registry
            .as_ref()
            .and_then(|registry| {
                registry
                    .link_owned_instance_transition_like_cpp(
                        group_guid,
                        key.map_id,
                        key.instance_id,
                    )
                    .ok()
            })
            .is_some();
        (!linked).then_some((key, group_guid))
    } else {
        None
    };
    let existing_instance_lock_context = match &decision {
        wow_map::CreateMapDecision::Existing { key, .. } if is_dungeon => manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|map| map.instance_lock_context()),
        _ => None,
    };
    let existing_instance_player_count = match &decision {
        wow_map::CreateMapDecision::Existing { key, .. } if is_dungeon => manager
            .find_map(key.map_id, key.instance_id)
            .map(|map| map.players_count_except_gms_like_cpp()),
        _ => None,
    };
    let existing_instance_encounter_in_progress = match &decision {
        wow_map::CreateMapDecision::Existing { key, .. } if is_dungeon => manager
            .find_map(key.map_id, key.instance_id)
            .map(|map| map.instance_encounter_in_progress_like_cpp()),
        _ => None,
    };
    if mark_pending_entry && let Some(key) = key {
        manager.mark_instance_player_entry_pending_like_cpp(key, player_guid);
    }
    drop(manager);

    // C++ `InstanceMap::InstanceMap` marks an already attached lock in use at
    // construction, before any later admission check can reject the entry.
    if is_dungeon
        && matches!(&decision, wow_map::CreateMapDecision::Create { .. })
        && let Some(lock) = active_instance_lock
        && !set_final_map_lock_in_use_like_cpp(host, map_id, lock)
    {
        warn!(
            account = host.core().account_id,
            map_id,
            instance_id = lock.instance_id,
            difficulty_id = lock.difficulty_id,
            "failed to mark a new canonical map's initial instance lock in use"
        );
    }

    if let Some((key, group_guid)) = group_link_failure {
        warn!(
            account = host.core().account_id,
            group_guid,
            map_id = key.map_id,
            instance_id = key.instance_id,
            "failed to link canonical instance map to its owning group"
        );
    }

    // C++ `MapManager::CreateMap` receives the live `Player*` before it
    // performs admission checks and instance-entry accounting. Materialize
    // that canonical owner before any check can read or mutate
    // `_instanceResetTimes`; the Session retains only a test fallback.
    if let Some(key) = key
        && !host.ensure_canonical_player_owner_exists_like_cpp(key)
    {
        clear_pending_map_entry_like_cpp(host, key);
        return None;
    }

    if is_dungeon
        && run_instance_map_cannot_enter
        && let wow_map::CreateMapDecision::Existing {
            key, difficulty_id, ..
        } = &decision
        && let Some(player_count) = existing_instance_player_count
        && let Some(entries) = {
            let (state, hub) = host.split_instances_ref();
            state.create_map_db2_entries_like_cpp(hub, key.map_id, *difficulty_id)
        }
        && player_count >= entries.max_players
    {
        if publish {
            host.hub_ref()
                .send_transfer_aborted_like_cpp(key.map_id, TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP);
        }
        clear_pending_map_entry_like_cpp(host, *key);
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }

    if is_dungeon
        && run_instance_map_cannot_enter
        && map_entry.instance_type == wow_data::map::MAP_RAID
        && phase != MapEntryPhaseLikeCpp::Login
        && host.player_loading() != Some(player_guid)
        && let wow_map::CreateMapDecision::Existing { key, .. } = &decision
        && existing_instance_encounter_in_progress == Some(true)
    {
        if publish {
            host.hub_ref()
                .send_transfer_aborted_like_cpp(key.map_id, TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP);
        }
        clear_pending_map_entry_like_cpp(host, *key);
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }

    if is_dungeon
        && run_instance_map_cannot_enter
        && let wow_map::CreateMapDecision::Existing {
            key, difficulty_id, ..
        } = &decision
        && let Some(lock_context) = existing_instance_lock_context
    {
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
            if publish {
                host.hub_ref()
                    .send_transfer_aborted_like_cpp(key.map_id, deny_reason as u32);
            }
            clear_pending_map_entry_like_cpp(host, *key);
            return Some(wow_map::CreateMapDecision::Reject {
                side_effects: Vec::new(),
            });
        }
    }

    if is_dungeon
        && run_player_cannot_enter_preflight
        && !map_entry.ignores_instance_farm_limit_like_cpp()
        && let Some(key) = key
        && !{
            let (state, mut hub) = host.split_instances_mut();
            state.check_instance_count_like_cpp(&mut hub, key.instance_id)
        }
        && host.hub_ref().resolved_player_is_alive_like_cpp() == Some(true)
    {
        if publish {
            host.hub_ref().send_transfer_aborted_like_cpp(
                key.map_id,
                TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP,
            );
        }
        clear_pending_map_entry_like_cpp(host, key);
        return Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new(),
        });
    }

    if is_dungeon && let Some(key) = key {
        let now_secs = u64::try_from(unix_now()).unwrap_or(0);
        {
            let (state, mut hub) = host.split_instances_mut();
            state.add_instance_enter_time_like_cpp(&mut hub, key.instance_id, now_secs)
        };
    }

    let _ = apply_create_map_side_effects_like_cpp(host, map_id, &decision);

    if is_dungeon
        && let Some(key) = key
        && let Some(difficulty_id) = create_map_decision_difficulty_id_like_cpp(&decision)
    {
        let has_reset_schedule = {
            let (state, hub) = host.split_instances_ref();
            state
                .create_map_db2_entries_like_cpp(hub, map_id, difficulty_id)
                .is_some_and(|entries| entries.has_reset_schedule())
        };
        if let Some((lock, expiry_context)) =
            create_map_active_instance_lock_context_with_expiry_like_cpp(
                host,
                map_id,
                difficulty_id,
            )
        {
            let manager_arc = Arc::clone(host.core().canonical_map_manager.as_ref()?);
            if let Ok(mut manager) = manager_arc.lock() {
                if let Some(map) = manager.find_map_mut(key.map_id, key.instance_id) {
                    map.set_instance_lock_context(Some(lock));
                    map.set_instance_expiry_context_like_cpp(Some(expiry_context));
                }
                if mark_pending_entry {
                    manager.mark_instance_player_entry_pending_like_cpp(key, player_guid);
                }
            }

            if !set_final_map_lock_in_use_like_cpp(host, map_id, lock) {
                warn!(
                    account = host.core().account_id,
                    map_id,
                    instance_id = lock.instance_id,
                    difficulty_id = lock.difficulty_id,
                    "failed to mark the canonical map's exact instance lock in use"
                );
            }
        } else if has_reset_schedule {
            warn!(
                account = host.core().account_id,
                map_id,
                instance_id = key.instance_id,
                difficulty_id,
                "failed to resolve the canonical map's final active instance lock"
            );
        }
    }

    Some(decision)
}

fn clear_pending_map_entry_like_cpp<H: InstanceSessionHostLikeCpp>(host: &H, key: wow_map::MapKey) {
    let Some(guid) = host.core().player_guid else {
        return;
    };
    let Some(manager) = host.core().canonical_map_manager.as_ref().map(Arc::clone) else {
        return;
    };
    if let Ok(mut manager) = manager.lock() {
        manager.complete_instance_player_entry_like_cpp(key, guid);
    }
}

fn set_final_map_lock_in_use_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
    map_id: u32,
    context: wow_map::CreateMapInstanceLockContext,
) -> bool {
    let Ok(owner_guid_counter) = i64::try_from(context.owner_guid_counter) else {
        return false;
    };
    let owner_guid =
        wow_core::ObjectGuid::create_player(host.core().realm_id(), owner_guid_counter);
    host.core()
        .instance_lock_mgr
        .as_ref()
        .and_then(|manager| manager.write().ok())
        .is_some_and(|mut manager| {
            manager.set_instance_lock_in_use_like_cpp(
                owner_guid,
                map_id,
                context.difficulty_id,
                context.instance_id,
                true,
            )
        })
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player's session-side visibility reads and steps, moved from the World
//! `session/visibility/operations.rs` and `session/deferred_visibility.rs`
//! (#1363 slice 12b6): the seer-detection sync, the registry-backed other
//! players with their canonical phase gate, the phase shift and its
//! `CanSeeOrDetect` phase test, the resting flag, `Player::m_seer`, the forced
//! visibility update and the active-mover completion
//! (`MovementHandler.cpp:808`). The World session's host impls name these
//! functions.

use wow_core::{ObjectGuid, Position};
use wow_entities::{ObjectNotifyFlags, PhaseShift};
use wow_world_core::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
use wow_world_core::session::PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP;
use wow_world_core::session::directory::PlayerVisibilityCreateSnapshot;

use super::VisibilitySessionHostLikeCpp;
use crate::CreatureSpawnCatalogsLikeCpp;

fn apply_player_session_visibility_detection_like_cpp(
    player: &mut wow_entities::Player,
    never_visible_for_seer: bool,
    seer_can_never_see_target: bool,
) {
    player
        .unit_mut()
        .set_never_visible_for_seer_like_cpp(never_visible_for_seer);
    player
        .unit_mut()
        .set_seer_can_never_see_target_like_cpp(seer_can_never_see_target);
}

fn player_session_never_visible_for_seer_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    guid: ObjectGuid,
) -> bool {
    host.lifecycle().player_logout_like_cpp() || host.lifecycle().player_loading() == Some(guid)
}
pub fn sync_current_player_session_visibility_detection_like_cpp<
    H: VisibilitySessionHostLikeCpp,
>(
    host: &mut H,
) {
    let Some(guid) = host.player_guid() else {
        return;
    };
    let never_visible_for_seer = player_session_never_visible_for_seer_like_cpp(host, guid);
    let seer_can_never_see_target = host.hub_ref().player_can_never_see_target_like_cpp();
    let _ = host
        .core()
        .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
            apply_player_session_visibility_detection_like_cpp(
                player,
                never_visible_for_seer,
                seer_can_never_see_target,
            );
        });
}

pub fn visible_other_players_from_registry_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    map_id: u16,
    _position: &Position,
    _visibility_radius: f32,
) -> Vec<(ObjectGuid, PlayerVisibilityCreateSnapshot)> {
    let Some(viewer_handle) = host.core().player_handle_like_cpp else {
        return Vec::new();
    };
    let viewer_guid = viewer_handle.guid();
    if host.player_guid() != Some(viewer_guid) {
        return Vec::new();
    }
    let Some(registry) = &host.core().player_registry else {
        return Vec::new();
    };
    let mode = host.hub_ref().config.group_visibility_mode;
    let viewer_group_guid = host
        .core()
        .with_owned_player_like_cpp(group_guid_from_player)
        .flatten();
    let group_snapshot = if matches!(mode, 2 | 3) {
        None
    } else {
        viewer_group_guid.and_then(|guid| {
            host.core()
                .group_registry()
                .and_then(|groups| groups.get(&guid))
        })
    };
    let committed_visible = host
        .core()
        .client_visible_guids_like_cpp
        .snapshot_like_cpp();

    let (map_key, selected) = {
        let Some(manager) = host.core().canonical_map_manager.as_ref() else {
            return Vec::new();
        };
        let Ok(manager) = manager.lock() else {
            return Vec::new();
        };
        let Some((map_key, _viewer_residence_revision)) =
            manager.player_active_residence_revision_like_cpp(viewer_handle)
        else {
            return Vec::new();
        };
        if map_key.map_id != u32::from(map_id) {
            return Vec::new();
        }
        let Some(map_entry) = manager.find_map(map_key.map_id, map_key.instance_id) else {
            return Vec::new();
        };
        let map = map_entry.map();
        let Some(selected) = manager.with_player_like_cpp(viewer_handle, |viewer| {
            let viewer_world = viewer.unit().world();
            if !viewer_world.object().is_in_world()
                || viewer_world.map_id() != map_key.map_id
                || viewer_world.instance_id() != map_key.instance_id
                || group_guid_from_player(viewer) != viewer_group_guid
            {
                return Vec::new();
            }

            let viewpoint = super::ghost_visibility::canonical_viewpoint(viewer, map);
            let search_radius = viewer_world.get_sight_range(None, map) + viewpoint.combat_reach();
            let nearby = map.nearby_cell_guids_like_cpp(
                viewpoint.position().x,
                viewpoint.position().y,
                search_radius,
            );
            let corpse = {
                let seer = viewer.unit();
                (seer.is_dead() && seer.data().health > 0)
                    .then(|| {
                        map.corpse_guid_by_player_like_cpp(viewer.guid())
                            .and_then(|guid| map.get_typed_corpse(guid))
                    })
                    .flatten()
            };

            let mut target_guids: Vec<_> = nearby.world.players.into_iter().collect();
            target_guids.sort_unstable();
            target_guids.dedup();
            let mut selected = Vec::new();
            for target_guid in target_guids {
                if target_guid == viewer_guid {
                    continue;
                }
                let Some((target_handle, _)) =
                    manager.current_player_admission_like_cpp(target_guid)
                else {
                    continue;
                };
                let Some((target_key, target_residence_revision)) =
                    manager.player_active_residence_revision_like_cpp(target_handle)
                else {
                    continue;
                };
                if target_key != map_key {
                    continue;
                }
                let Some(Some(())) = manager.with_player_like_cpp(target_handle, |target| {
                    let target_world = target.unit().world();
                    if !target_world.object().is_in_world()
                        || target_world.map_id() != map_key.map_id
                        || target_world.instance_id() != map_key.instance_id
                    {
                        return None;
                    }

                    let group_visible = match mode {
                        2 => viewer.team() == target.team(),
                        3 => false,
                        _ => group_snapshot.as_ref().is_some_and(|group| {
                            let Some(group_guid) = viewer_group_guid else {
                                return false;
                            };
                            if group.group_guid != group_guid
                                || group_guid_from_player(target) != Some(group_guid)
                                || !group.members.contains(&viewer.guid())
                                || !group.members.contains(&target.guid())
                            {
                                return false;
                            }
                            if mode == 1 {
                                return true;
                            }
                            let viewer_subgroup = group
                                .member_slots
                                .iter()
                                .find(|slot| slot.guid == viewer.guid())
                                .map(|slot| slot.subgroup);
                            let target_subgroup = group
                                .member_slots
                                .iter()
                                .find(|slot| slot.guid == target.guid())
                                .map(|slot| slot.subgroup);
                            viewer_subgroup.is_some() && viewer_subgroup == target_subgroup
                        }),
                    };
                    let context = super::ghost_visibility::unit_context(
                        viewer,
                        target.unit(),
                        viewpoint,
                        map,
                        corpse,
                    );
                    let vehicle_visible = target
                        .unit()
                        .subsystems()
                        .vehicle
                        .vehicle_guid
                        .is_none_or(|vehicle| committed_visible.contains(&vehicle));
                    let context = wow_entities::UnitVisibilityContext {
                        distance_allowed: context.distance_allowed && vehicle_visible,
                        player_group_visibility: group_visible,
                        ..context
                    };
                    viewer
                        .unit()
                        .can_see_or_detect_unit_with_context(
                            target.unit(),
                            false,
                            true,
                            false,
                            context,
                        )
                        .then_some(())
                }) else {
                    continue;
                };
                selected.push((target_guid, target_handle, target_residence_revision));
            }
            selected
        }) else {
            return Vec::new();
        };
        drop(manager);
        (map_key, selected)
    };

    selected
        .into_iter()
        .filter_map(|(guid, handle, residence_revision)| {
            let registration = registry.control_address(guid)?.registration();
            let candidate = registry.player_visibility_create_snapshot_for_candidate(
                registration,
                handle,
                map_key,
                residence_revision,
                viewer_guid,
            )?;
            Some((guid, candidate))
        })
        .collect()
}

fn group_guid_from_player(player: &wow_entities::Player) -> Option<u64> {
    let guid = player.gameplay_state().group.as_ref()?.group_guid;
    (!guid.is_empty()).then_some(guid.counter() as u64)
}
pub fn represented_player_phase_shift_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
) -> Option<PhaseShift> {
    let canonical = host
        .core()
        .with_owned_player_like_cpp(|player| player.unit().world().phase_shift().clone());
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        return Some(
            host.visibility()
                .represented_player_phase_shift_fixture_like_cpp(),
        );
    }
    canonical
}
pub fn can_see_phase_shift_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    other: &PhaseShift,
) -> bool {
    represented_player_phase_shift_like_cpp(host)
        .is_some_and(|phase_shift| phase_shift.can_see(other))
}
pub fn resolved_visible_resting_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
) -> Option<bool> {
    // RestMgr::SetRestFlag/RemoveRestFlag (RestMgr.cpp:99-125): the
    // mask and Player flag belong to the same Player. Read them together.
    let canonical = host.core().with_owned_player_for_rest_like_cpp(|player| {
        let rest = player.rest_state_like_cpp();
        if rest.is_location_initialized_like_cpp() {
            rest.is_resting_by_flag_like_cpp()
        } else {
            (player.data().player_flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0
        }
    });
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        let rest = host.hub_ref().player_rest_state_snapshot_like_cpp()?;
        if rest.is_location_initialized_like_cpp() {
            return Some(rest.is_resting_by_flag_like_cpp());
        }
        return Some(
            host.lifecycle()
                .represented_loaded_player_flags_for_test_like_cpp()
                .map(|flags| (flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0)
                .unwrap_or(rest.is_resting_by_flag_like_cpp()),
        );
    }
    canonical
}
// The represented PLAYER_FLAGS reads (#1363 slice 12d; moved from the World
// `session/rest_progression.rs`).
pub fn represented_player_has_flag_like_cpp<H: VisibilitySessionHostLikeCpp + ?Sized>(
    host: &H,
    flag: u32,
) -> bool {
    let canonical = host.player_guid().and_then(|guid| {
        host.core()
            .canonical_player_has_player_flag_like_cpp(guid, flag)
    });
    if let Some(value) = canonical {
        return value;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        return host
            .lifecycle()
            .represented_loaded_player_flags_for_test_like_cpp()
            .is_some_and(|flags| (flags & flag) != 0);
    }
    false
}

pub fn represented_player_flags_value_like_cpp<H: VisibilitySessionHostLikeCpp + ?Sized>(
    host: &H,
) -> Option<u32> {
    let canonical = host
        .core()
        .canonical_player_snapshot_like_cpp(|player| player.data().player_flags);
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        return host
            .lifecycle()
            .represented_loaded_player_flags_for_test_like_cpp();
    }
    canonical
}

// The position visibility is evaluated from: the current seer's map object
// when it is not the Player (C++ `Player::m_seer`), else the Player (#1363
// slice 12d; moved from the World `session/movement/state.rs`).
pub fn represented_visibility_source_position_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
) -> Option<Position> {
    let player_position = host.hub_ref().player_position_like_cpp()?;
    let Some(player_guid) = host.player_guid() else {
        return Some(player_position);
    };
    let Some(seer_guid) = current_seer_guid_like_cpp(host) else {
        return Some(player_position);
    };
    if seer_guid.is_empty() || seer_guid == player_guid {
        return Some(player_position);
    }

    let Some(key) = host.core().current_canonical_player_map_key_like_cpp() else {
        return Some(player_position);
    };
    let Some(manager) = host.core().canonical_map_manager.as_ref() else {
        return Some(player_position);
    };
    let Ok(manager) = manager.lock() else {
        return Some(player_position);
    };
    manager
        .find_map(key.map_id, key.instance_id)
        .and_then(|managed| {
            managed.map().with_world_object_by_kinds_like_cpp(
                seer_guid,
                wow_entities::represented_seer_kinds_like_cpp(),
                |object| object.position(),
            )
        })
        .or(Some(player_position))
}

/// Resolve the current C++ `Player::m_seer` projection from the canonical
/// Player. C++ keeps a pointer, but every Rust consumer in this boundary
/// needs only its GUID; `ActivePlayerData::FarsightObject` is the durable
/// owner and an empty value means the Player itself.
pub fn current_seer_guid_like_cpp<H: VisibilitySessionHostLikeCpp>(host: &H) -> Option<ObjectGuid> {
    let player_guid = host.player_guid()?;
    if let Some(farsight) = host
        .core()
        .current_canonical_player_farsight_object_value_like_cpp()
    {
        if !farsight.is_empty() {
            return Some(farsight);
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if host.test_fixtures_ref_like_cpp().is_some()
            && let Some(seer_guid) = host.visibility().represented_seer_guid_fixture_like_cpp()
        {
            // Detached fixtures can model the short C++ ordering window
            // between writing FarsightObject and SetSeer(this).
            if !seer_guid.is_empty() && seer_guid != player_guid {
                return Some(seer_guid);
            }
        }
        return Some(player_guid);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() {
        return host.visibility().represented_seer_guid_fixture_like_cpp();
    }

    None
}
pub async fn force_update_visibility_with_catalogs_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
) {
    host.visibility_mut().clear_last_visibility_pos_like_cpp();
    crate::update_visibility_with_catalogs_like_cpp(host, creature_spawn_catalogs).await;
}

pub fn apply_move_init_active_mover_complete_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    ticks: u32,
) {
    let transport_server_time =
        wow_world_core::session::game_time_ms_like_cpp().saturating_sub(ticks) as i32;
    if host
        .hub_mut()
        .mutate_active_player_update_state_like_cpp(|state| {
            state.active_local_flags |= PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP;
            state.active_transport_server_time = transport_server_time;
        })
        .is_none()
    {
        return;
    }
    sync_current_player_session_visibility_detection_like_cpp(host);
    // MovementHandler.cpp:808 -> Player::UpdateObjectVisibility(false).
    // No scanner or packet fanout here: ProcessRelocationNotifies owns the
    // active-grid/timer phase and consumes this mark after selection.
    host.core().with_owned_player_mut_like_cpp(|player| {
        let object = player.unit_mut().world_mut().object_mut();
        if object.is_in_world() {
            object.add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        }
    });
    host.hub_ref()
        .send_active_player_transport_server_time_update_like_cpp();
}

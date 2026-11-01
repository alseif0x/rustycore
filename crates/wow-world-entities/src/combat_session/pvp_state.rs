// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player combat stop and PvP flag state at the session boundary (#1363 slice
//! 7a; moved from the World `session/combat/state.rs`). Behaviour is
//! preserved; the canonical owner of this state is unchanged.

use wow_constants::unit::UnitPvpFlags;
use wow_core::ObjectGuid;
use wow_entities::Player;

use super::CombatSessionHostLikeCpp;

pub fn update_pvp_flag_like_cpp<H: CombatSessionHostLikeCpp>(host: &mut H, curr_time: i64) {
    let Some(guid) = host.player_guid() else {
        return;
    };

    if host.hub_ref().player_is_pvp_like_cpp(guid) != Some(true) {
        return;
    }

    let Some(state) = host.hub_ref().player_world_local_state_like_cpp() else {
        return;
    };
    let Some(end_timer) = state.pvp_end_timer_like_cpp() else {
        return;
    };

    if curr_time < end_timer.saturating_add(300) || state.is_pvp_hostile_like_cpp() {
        return;
    }

    if end_timer <= curr_time {
        #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_mut))]
        let mut mutated = host.core().with_owned_player_mut_like_cpp(|player| {
            player.expire_pvp_timer_like_cpp();
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if host.test_fixtures_like_cpp().is_some()
            && mutated.is_none()
            && host.core().player_handle_like_cpp.is_none()
        {
            mutated = host
                .core()
                .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player.expire_pvp_timer_like_cpp();
                });
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if host.core().player_handle_like_cpp.is_none()
            && let Some(fixtures) = host.test_fixtures_like_cpp()
        {
            fixtures.combat.player_pvp_end_timer_like_cpp = None;
        }
        let _ = mutated;
    }

    host.hub_mut().update_player_pvp_like_cpp(false, false);
    host.sync_player_registry_state_like_cpp();
}
pub fn combat_stop_like_cpp<H: CombatSessionHostLikeCpp>(host: &mut H) {
    let Some(player_guid) = host.player_guid() else {
        host.hub_mut().set_combat_target_like_cpp(None);
        host.hub_mut().set_in_combat_like_cpp(false);
        return;
    };

    let stopped_target = host.hub_mut().stop_player_attack_like_cpp();
    let owner_guids = {
        let Some(manager) = host.core().canonical_map_manager.as_ref().cloned() else {
            return finish_combat_stop_like_cpp(host, player_guid, stopped_target, Vec::new());
        };
        let Ok(mut manager) = manager.lock() else {
            return finish_combat_stop_like_cpp(host, player_guid, stopped_target, Vec::new());
        };
        let Some(key) = host
            .core()
            .player_handle_like_cpp
            .and_then(|handle| manager.player_residence_like_cpp(handle))
            .and_then(|residence| match residence {
                wow_map::PlayerResidenceLikeCpp::Active(key) => Some(key),
                wow_map::PlayerResidenceLikeCpp::Detached => None,
            })
        else {
            drop(manager);
            return finish_combat_stop_like_cpp(host, player_guid, stopped_target, Vec::new());
        };
        let Some(managed) = manager.find_map_mut(key.map_id, key.instance_id) else {
            drop(manager);
            return finish_combat_stop_like_cpp(host, player_guid, stopped_target, Vec::new());
        };
        let map = managed.map_mut();
        let Some(player) = map.get_typed_player_mut(player_guid) else {
            drop(manager);
            return finish_combat_stop_like_cpp(host, player_guid, stopped_target, Vec::new());
        };

        let mut owner_guids: Vec<ObjectGuid> = player
            .unit()
            .subsystems()
            .combat
            .pve_refs
            .keys()
            .chain(player.unit().subsystems().combat.pvp_refs.keys())
            .chain(player.unit().subsystems().combat.attackers.iter())
            .copied()
            .collect();
        owner_guids.sort_unstable();
        owner_guids.dedup();

        player.unit_mut().subsystems_mut().combat.end_all_combat();
        player.unit_mut().subsystems_mut().combat.clear_attackers();

        for owner_guid in &owner_guids {
            if let Some(owner) = map.get_typed_player_mut(*owner_guid) {
                if owner.unit().attacking() == Some(player_guid) {
                    let _ = owner.unit_mut().attack_stop_like_cpp();
                }
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(player_guid);
                owner.unit_mut().remove_attacker_like_cpp(player_guid);
            } else if let Some(owner) = map.get_typed_creature_mut(*owner_guid) {
                if owner.unit().attacking() == Some(player_guid) {
                    let _ = owner.unit_mut().attack_stop_like_cpp();
                }
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(player_guid);
                owner.unit_mut().remove_attacker_like_cpp(player_guid);
            } else if let Some(owner) = map.get_typed_pet_mut(*owner_guid) {
                let unit = owner.creature_mut().unit_mut();
                if unit.attacking() == Some(player_guid) {
                    let _ = unit.attack_stop_like_cpp();
                }
                unit.subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(player_guid);
                unit.remove_attacker_like_cpp(player_guid);
            }
        }

        owner_guids
    };

    finish_combat_stop_like_cpp(host, player_guid, stopped_target, owner_guids);
}
fn finish_combat_stop_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    player_guid: ObjectGuid,
    stopped_target: Option<ObjectGuid>,
    owner_guids: Vec<ObjectGuid>,
) {
    host.hub_mut().set_combat_target_like_cpp(None);
    host.hub_mut().set_in_combat_like_cpp(false);
    for owner_guid in owner_guids {
        let _ = host.core_mut().mutate_world_creature(owner_guid, |owner| {
            if owner.unit().attacking() == Some(player_guid) {
                let _ = owner.unit_mut().attack_stop_like_cpp();
            }
            owner
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_combat_ref_like_cpp(player_guid);
            owner
                .unit_mut()
                .subsystems_mut()
                .combat
                .scale_threat(player_guid, 0.0);
            owner.unit_mut().remove_attacker_like_cpp(player_guid);
            if owner.ai_ownership().combat_target == Some(player_guid) {
                owner.ai_ownership_mut().combat_target = None;
            }
        });
    }

    if let Some(target) = stopped_target {
        host.send_packet(&wow_packet::packets::combat::SAttackStop {
            attacker: player_guid,
            victim: target,
            now_dead: false,
        });
    }
    host.core()
        .send_packet_realm(&wow_packet::packets::combat::CancelCombat);
    host.sync_player_registry_state_like_cpp();
}
pub fn reset_contested_pvp_like_cpp<H: CombatSessionHostLikeCpp>(host: &mut H) {
    let Some(_guid) = host.player_guid() else {
        return;
    };

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_mut))]
    let mut mutated = host.core().with_owned_player_mut_like_cpp(|player| {
        player.clear_contested_pvp_like_cpp();
    });
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_like_cpp().is_some()
        && mutated.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        mutated = host
            .core()
            .mutate_canonical_player_by_guid_like_cpp(_guid, |player| {
                player.clear_contested_pvp_like_cpp();
            });
        if let Some(fixtures) = host.test_fixtures_like_cpp() {
            fixtures.combat.player_contested_pvp_timer_like_cpp = 0;
        }
    }
    let _ = mutated;
    host.sync_player_registry_state_like_cpp();
}
/// Mirror `Unit::SetPvpFlag` / `RemovePvpFlag` for the realm-wide FFA bit.
///
/// The canonical player remains the runtime authority. The isolated delta
/// keeps this direct owner update limited to `UnitData::PvpFlags` without
/// clearing or leaking any other dirty canonical fields that the map tick
/// still owns.
pub fn set_represented_ffa_pvp_flag_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    enabled: bool,
) -> bool {
    let update = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            let before = player.unit().pvp_flags_like_cpp();
            if before.contains(UnitPvpFlags::FFA_PVP) == enabled {
                return None;
            }

            if enabled {
                player
                    .unit_mut()
                    .set_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP);
            } else {
                player
                    .unit_mut()
                    .remove_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP);
            }
            let after = player.unit().pvp_flags_like_cpp();

            let mut delta = Player::new(None, false);
            delta.unit_mut().replace_all_pvp_flags_like_cpp(before);
            delta.clear_data_changes();
            delta.unit_mut().replace_all_pvp_flags_like_cpp(after);
            Some(delta.values_update(true))
        })
        .flatten();
    let Some(update) = update else {
        return false;
    };

    host.core().send_player_values_update_like_cpp(&update);
    host.sync_player_registry_state_like_cpp();
    true
}
/// C++ `HandlePlayerLogin`: realm-wide FFA is enabled after the player is
/// in the map, except for GMs and players whose loaded/zone-restored flags
/// already say they are resting.
pub fn apply_represented_ffa_pvp_login_state_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
) -> bool {
    if !host.is_ffa_pvp_realm_like_cpp()
        || host.hub_ref().player_is_game_master_like_cpp() != Some(false)
        || host.resolved_visible_resting_like_cpp() != Some(false)
    {
        return false;
    }

    set_represented_ffa_pvp_flag_like_cpp(host, true)
}

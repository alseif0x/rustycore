// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `Player::UpdateZone`/`UpdateArea` with the RestMgr area, city and
//! tavern flags and the zone hostility update (#1363 slice 11b; moved from the
//! World `session/world_state/operations.rs` and `session/rest_progression.rs`).

use wow_constants::Team;
use wow_world_core::session::{
    PLAYER_FLAGS_IN_PVP_LIKE_CPP, PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP, player_team_for_race_cpp,
};
use wow_world_spell::AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedAreaZoneCriteriaLikeCpp;

use super::rest_flags::{
    clear_player_deferred_rest_flag_update_like_cpp, defer_player_rest_flag_sync_like_cpp,
    end_player_rest_flag_sync_like_cpp, remove_player_rest_flag_like_cpp,
    set_player_rest_flag_like_cpp,
};
use super::{
    AREA_FLAG_COMBAT_ZONE_LIKE_CPP, AREA_FLAG_CONTESTED_LIKE_CPP,
    AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP, InstanceSessionHostLikeCpp, REST_FLAG_IN_CITY_LIKE_CPP,
    REST_FLAG_IN_FACTION_AREA_LIKE_CPP, REST_FLAG_IN_TAVERN_LIKE_CPP,
};

const PLAYER_FLAGS_WAR_MODE_ACTIVE_LIKE_CPP: u32 = 0x0000_0400;
const PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP: u32 = 0x0000_0800;
const WAR_MODE_ENLISTED_SPELL_OUTSIDE_LIKE_CPP: i32 = 269_083;
const WAR_MODE_AURA_INSIDE_LIKE_CPP: i32 = 282_559;
const WAR_MODE_LEAVE_INTERRUPT_FLAGS2_LIKE_CPP: u32 = 0x0002_0000;

/// C++ `Player::UpdateWarModeAuras`' disabled branch (Player.cpp:28934-28966).
/// UpdateZone calls it after finding the zone row and before hostile-area state.
fn update_represented_war_mode_auras_like_cpp<H: InstanceSessionHostLikeCpp>(host: &mut H) {
    let Some(war_mode_desired) = host.core().with_owned_player_like_cpp(|player| {
        player.has_player_flag(PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP)
    }) else {
        return;
    };
    if war_mode_desired {
        return;
    }

    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player.remove_player_local_flags_like_cpp(PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP);
    });
    let _ = wow_world_spell::remove_represented_auras_due_to_spell_like_cpp(
        host,
        WAR_MODE_ENLISTED_SPELL_OUTSIDE_LIKE_CPP,
    );
    let _ = wow_world_spell::remove_represented_auras_due_to_spell_like_cpp(
        host,
        WAR_MODE_AURA_INSIDE_LIKE_CPP,
    );
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player.remove_player_flags_like_cpp(PLAYER_FLAGS_WAR_MODE_ACTIVE_LIKE_CPP);
        if !player.has_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP) {
            player
                .unit_mut()
                .remove_pvp_flag_like_cpp(wow_constants::UnitPvpFlags::PVP);
        }
    });
    let _ = wow_world_spell::remove_auras_with_interrupt_flags_like_cpp(
        host,
        0,
        WAR_MODE_LEAVE_INTERRUPT_FLAGS2_LIKE_CPP,
    );
}

fn update_represented_hostile_area_state_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    zone: &wow_data::AreaTableEntry,
) {
    let war_mode_active = host.hub_ref().player_war_mode_local_active_like_cpp();
    let zone_hostile = if zone.is_sanctuary_like_cpp() {
        false
    } else if (zone.flags & (AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP | AREA_FLAG_COMBAT_ZONE_LIKE_CPP))
        != 0
    {
        true
    } else if (zone.flags & AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP) != 0 {
        if (zone.flags & AREA_FLAG_CONTESTED_LIKE_CPP) != 0 {
            war_mode_active
        } else {
            let faction_group_mask = host
                .hub_ref()
                .catalogs
                .area_table_store
                .as_ref()
                .map(|store| store.faction_group_mask_like_cpp(zone.id))
                .unwrap_or(0);
            host.hub_ref()
                .player_faction_template_id_like_cpp()
                .and_then(|id| {
                    host.hub_ref()
                        .catalogs
                        .factions
                        .template_store
                        .as_ref()
                        .and_then(|store| store.get(id))
                })
                .is_some_and(|faction_template| {
                    if (faction_template.friend_group & faction_group_mask) != 0 {
                        false
                    } else if (faction_template.enemy_group & faction_group_mask) != 0 {
                        true
                    } else {
                        host.is_pvp_realm_like_cpp()
                    }
                })
        }
    } else {
        false
    };
    let _ = host
        .hub_mut()
        .set_player_pvp_hostile_like_cpp(zone_hostile || war_mode_active);
}
fn update_represented_rest_flag_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    rest_flag: u32,
    active: bool,
) -> bool {
    if active {
        set_player_rest_flag_like_cpp(host, rest_flag, 0)
    } else {
        remove_player_rest_flag_like_cpp(host, rest_flag)
    }
}

pub fn set_represented_tavern_resting_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    trigger_id: u32,
    entered: bool,
) -> bool {
    if entered {
        set_player_rest_flag_like_cpp(host, REST_FLAG_IN_TAVERN_LIKE_CPP, trigger_id)
    } else {
        remove_player_rest_flag_like_cpp(host, REST_FLAG_IN_TAVERN_LIKE_CPP)
    }
}
pub fn update_area_represented_with_rest_update_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    new_area: u32,
    send_rest_update: bool,
) -> bool {
    let Some(world_local) = host.hub_ref().player_world_local_state_like_cpp() else {
        return false;
    };
    let old_area = world_local.area_id_like_cpp();
    if old_area != new_area {
        host.core()
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    if !host.hub_mut().set_player_area_id_like_cpp(new_area) {
        return false;
    }
    let zone_id = world_local.zone_id_like_cpp();
    // A known catalog's missing row means non-sanctuary in C++ UpdateArea.
    // An unavailable catalog supplies no authority to clear a native flag.
    let sanctuary = host
        .hub_ref()
        .catalogs
        .area_table_store
        .as_ref()
        .map(|store| {
            store
                .get(new_area)
                .is_some_and(|area| area.is_sanctuary_like_cpp())
        });
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .world_mut()
            .set_zone_and_area(zone_id, new_area);
        if let Some(sanctuary) = sanctuary {
            player
                .unit_mut()
                .set_sanctuary_from_area_like_cpp(sanctuary);
        }
    });

    let mut rest_changed = false;
    let area_resting = host
        .hub_ref()
        .catalogs
        .area_table_store
        .as_ref()
        .and_then(|store| {
            store.get(new_area).map(|area| {
                let team = player_team_for_race_cpp(host.hub_ref().player_race_like_cpp());
                match team {
                    Team::Alliance => area.alliance_resting_like_cpp(),
                    Team::Horde | Team::Other => area.horde_resting_like_cpp(),
                }
            })
        });
    rest_changed |= update_represented_rest_flag_like_cpp(
        host,
        REST_FLAG_IN_FACTION_AREA_LIKE_CPP,
        area_resting.unwrap_or(false),
    );
    if send_rest_update && rest_changed {
        host.send_represented_resting_player_flag_update_like_cpp();
    }

    if old_area == new_area {
        return false;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() {
        host.split_instances_mut()
            .0
            .record_represented_area_zone_criteria_for_test_like_cpp(
                RepresentedAreaZoneCriteriaLikeCpp::EnterArea(new_area),
            );
        host.split_instances_mut()
            .0
            .record_represented_area_zone_criteria_for_test_like_cpp(
                RepresentedAreaZoneCriteriaLikeCpp::LeaveArea(old_area),
            );
    }
    true
}
/// Represented C++ `Player::UpdateZone` criteria branch.
///
/// C++ first updates `m_zoneUpdateId`, then calls `UpdateArea(newArea)`, then
/// returns early if the new zone has no `AreaTableEntry`. Therefore top-level
/// area criteria are recorded only when the zone changes and the new zone row
/// exists, while area criteria may already have been recorded by `UpdateArea`.
pub fn update_zone_represented_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    new_zone: u32,
    new_area: u32,
) -> bool {
    update_zone_represented_with_rest_update_like_cpp(host, new_zone, new_area, true)
}
pub fn update_zone_represented_without_rest_update_packet_like_cpp<
    H: InstanceSessionHostLikeCpp,
>(
    host: &mut H,
    new_zone: u32,
    new_area: u32,
) -> bool {
    update_zone_represented_with_rest_update_like_cpp(host, new_zone, new_area, false)
}
fn update_zone_represented_with_rest_update_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    new_zone: u32,
    new_area: u32,
    send_rest_update: bool,
) -> bool {
    if host.player_guid().is_none() {
        return false;
    }

    let Some(world_local) = host.hub_ref().player_world_local_state_like_cpp() else {
        return false;
    };
    let old_zone = world_local.zone_id_like_cpp();
    if old_zone != new_zone {
        host.core()
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    if !host.hub_mut().set_player_zone_id_like_cpp(new_zone) {
        return false;
    }
    let area_id = world_local.area_id_like_cpp();
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .world_mut()
            .set_zone_and_area(new_zone, area_id);
    });
    // Pending publication survives same-zone reentry after a cancelled post-add.
    if !defer_player_rest_flag_sync_like_cpp(host) {
        return false;
    }
    update_area_represented_with_rest_update_like_cpp(host, new_area, false);

    let zone_entry = host
        .hub_ref()
        .catalogs
        .area_table_store
        .as_ref()
        .and_then(|store| store.get(new_zone).copied());
    let Some(zone) = zone_entry else {
        let rest_flag_update_dirty = end_player_rest_flag_sync_like_cpp(host);
        if send_rest_update
            && rest_flag_update_dirty
            && host.send_represented_resting_player_flag_update_like_cpp()
        {
            let _ = clear_player_deferred_rest_flag_update_like_cpp(host);
        }
        return true;
    };

    update_represented_war_mode_auras_like_cpp(host);
    update_represented_hostile_area_state_like_cpp(host, &zone);
    let Some(world_local) = host.hub_ref().player_world_local_state_like_cpp() else {
        return false;
    };
    // C++ keeps an existing city-rest flag in a hostile, non-sanctuary
    // LinkedChat zone. It removes the flag only in the outer non-LinkedChat
    // branch; the hostile inner branch performs no RestMgr mutation.
    if zone.linked_chat_like_cpp() {
        if !world_local.is_pvp_hostile_like_cpp() || zone.is_sanctuary_like_cpp() {
            set_player_rest_flag_like_cpp(host, REST_FLAG_IN_CITY_LIKE_CPP, 0);
        }
    } else {
        remove_player_rest_flag_like_cpp(host, REST_FLAG_IN_CITY_LIKE_CPP);
    }
    let rest_flag_update_dirty = end_player_rest_flag_sync_like_cpp(host);
    if send_rest_update
        && rest_flag_update_dirty
        && host.send_represented_resting_player_flag_update_like_cpp()
    {
        let _ = clear_player_deferred_rest_flag_update_like_cpp(host);
    }

    if old_zone == new_zone {
        return false;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() {
        host.split_instances_mut()
            .0
            .record_represented_area_zone_criteria_for_test_like_cpp(
                RepresentedAreaZoneCriteriaLikeCpp::EnterTopLevelArea(new_zone),
            );
        host.split_instances_mut()
            .0
            .record_represented_area_zone_criteria_for_test_like_cpp(
                RepresentedAreaZoneCriteriaLikeCpp::LeaveTopLevelArea(old_zone),
            );
    }
    true
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player resurrection and corpse reclaim (#43 slice 2).
//!
//! C++ source of truth: `Player::ResurrectPlayer` (`Player.cpp:4320`) and
//! `WorldSession::HandleReclaimCorpse` (`MiscHandler.cpp:416-445`).
//!
//! Not represented (no Rust owner yet): `RemovePlayerFlag(IS_OUT_OF_BOUNDS)`,
//! the Recruit-a-Friend dynamic flag, `SetWaterWalking(false)`/`SetRooted`,
//! `OutdoorPvPMgr::HandlePlayerResurrects`, `Battleground::HandlePlayerResurrect`
//! and `CastAllObtainSpells`.

use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_packet::packets::misc::DeathReleaseLoc;
use wow_world_core::session::connection_identity::unix_now;

use crate::player_death::{
    CORPSE_RECLAIM_RADIUS_LIKE_CPP, CorpseLifecycleCatalogsLikeCpp, SPELL_GHOST_LIKE_CPP,
    SPELL_WISP_SPIRIT_AURA_LIKE_CPP, corpse_reclaim_delay_secs_like_cpp,
    player_death_expire_time_like_cpp, terrain_zone_and_area_like_cpp, with_current_map_like_cpp,
};
use crate::{CorpseHostLikeCpp, player_corpse_like_cpp, player_in_arena_like_cpp};

/// C++ `Player::ResurrectPlayer(restore_percent, applySickness = false)`.
pub async fn resurrect_player_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: CorpseLifecycleCatalogsLikeCpp<'_>,
    restore_percent: f32,
) {
    if host.player_guid().is_none() {
        return;
    }
    // SetAreaSpiritHealer(nullptr)
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player
            .resurrection_state_mut_like_cpp()
            .area_spirit_healer_guid = ObjectGuid::EMPTY;
    });

    // SMSG_DEATH_RELEASE_LOC is a realm-connection opcode (Opcodes.cpp:1333).
    host.core()
        .send_packet_realm(&DeathReleaseLoc::cleared_like_cpp());

    // This must be called always even on Players with race != RACE_NIGHTELF
    // in case of faction change.
    host.remove_auras_due_to_spell_like_cpp(SPELL_WISP_SPIRIT_AURA_LIKE_CPP);
    host.remove_auras_due_to_spell_like_cpp(SPELL_GHOST_LIKE_CPP);
    // C++ `AuraEffect::HandleAuraGhost` unapply removes PLAYER_FLAGS_GHOST.
    crate::set_player_ghost_flag_like_cpp(host, false);

    let restored = host.core().with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Alive);
        player.resurrection_state_mut_like_cpp().death_timer_active = false;

        // set health/powers (0- will be set in caller)
        if restore_percent > 0.0 {
            let max_health = player.unit().data().max_health;
            player
                .unit_mut()
                .set_health((max_health as f32 * restore_percent) as u64);
            for power in [PowerType::Mana, PowerType::Energy, PowerType::Focus] {
                let value = (player.get_max_power(power) as f32 * restore_percent) as i32;
                if power == PowerType::Mana {
                    player.unit_mut().set_power(PowerType::Mana, value);
                    player.unit_mut().set_power(PowerType::Rage, 0);
                } else {
                    player.unit_mut().set_power(power, value);
                }
            }
        }
        player.values_update(true)
    });
    let Some(values_update) = restored else {
        return;
    };
    host.sync_player_registry_state_like_cpp();
    host.core()
        .send_player_values_update_like_cpp(&values_update);

    // trigger update zone for alive state zone updates
    if let Some(position) = host.hub_ref().player_position_like_cpp() {
        let map_id = u32::from(host.core().player_map_id_like_cpp());
        let (zone_id, area_id) = terrain_zone_and_area_like_cpp(host, map_id, position);
        host.update_zone_represented_like_cpp(zone_id, area_id);
    }

    // update visibility
    host.update_visibility_with_catalogs_like_cpp(catalogs.creature_spawns)
        .await;
    // Resurrection sickness (`applySickness`) is the spirit healer's path
    // (#43 slice 3); corpse reclaim calls `ResurrectPlayer(pct)` without it.
}

/// C++ `WorldSession::HandleReclaimCorpse` after the packet is parsed.
pub async fn reclaim_corpse_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: CorpseLifecycleCatalogsLikeCpp<'_>,
) -> bool {
    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(false) {
        return false;
    }
    // do not allow corpse reclaim in arena
    if player_in_arena_like_cpp(host) {
        return false;
    }
    // body not released yet
    if !host.hub_ref().player_has_ghost_flag_like_cpp() {
        return false;
    }
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    let Some((corpse_guid, ghost_time, corpse_type)) = player_corpse_like_cpp(host) else {
        return false;
    };

    // prevent resurrect before 30-sec delay after body release not finished
    let delay = corpse_reclaim_delay_secs_like_cpp(
        corpse_type == wow_entities::CorpseType::ResurrectablePvp,
        player_death_expire_time_like_cpp(host),
        unix_now(),
        host.hub_ref().config.death_policy_like_cpp,
    );
    if ghost_time + i64::from(delay) > unix_now() {
        return false;
    }

    // corpse->IsWithinDistInMap(_player, CORPSE_RECLAIM_RADIUS, true)
    let in_range = with_current_map_like_cpp(host, |map| {
        let corpse = map.get_typed_corpse(corpse_guid)?;
        let player = map.get_typed_player(player_guid)?;
        let corpse_world = corpse.world();
        let player_world = player.unit().world();
        if corpse_world.map_id() != player_world.map_id()
            || corpse_world.instance_id() != player_world.instance_id()
            || !corpse_world
                .phase_shift()
                .can_see(player_world.phase_shift())
        {
            return Some(false);
        }
        let max_dist = CORPSE_RECLAIM_RADIUS_LIKE_CPP
            + corpse_world.combat_reach()
            + player_world.combat_reach();
        let a = corpse_world.position();
        let b = player_world.position();
        let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
        Some(dx * dx + dy * dy + dz * dz < max_dist * max_dist)
    })
    .flatten()
    .unwrap_or(false);
    if !in_range {
        return false;
    }

    // resurrect
    let restore_percent = if host.hub_ref().player_in_represented_battleground_like_cpp() {
        1.0
    } else {
        0.5
    };
    resurrect_player_like_cpp(host, catalogs, restore_percent).await;

    // spawn bones
    crate::spawn_corpse_bones_like_cpp(host, catalogs.item_guid_generator, true).await;
    true
}

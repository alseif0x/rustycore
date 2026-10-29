// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player resurrection and corpse reclaim (#43 slice 2).
//!
//! C++ source of truth: `Player::ResurrectPlayer` (`Player.cpp:4320`) and
//! `WorldSession::HandleReclaimCorpse` (`MiscHandler.cpp:416-445`).
//!
//! Still outstanding: `OutdoorPvPMgr::HandlePlayerResurrects` and
//! `Battleground::HandlePlayerResurrect`.

use tracing::warn;
use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_packet::packets::misc::DeathReleaseLoc;
use wow_world_core::session::connection_identity::unix_now;

use crate::player_death::{
    CORPSE_RECLAIM_RADIUS_LIKE_CPP, CorpseLifecycleCatalogsLikeCpp, ResurrectionOperationCatalogs,
    SPELL_GHOST_LIKE_CPP, SPELL_WISP_SPIRIT_AURA_LIKE_CPP, corpse_reclaim_delay_secs_like_cpp,
    player_death_expire_time_like_cpp, terrain_zone_and_area_like_cpp, with_current_map_like_cpp,
};
use crate::{CorpseHostLikeCpp, player_corpse_like_cpp, player_in_arena_like_cpp};

/// C++ `PLAYER_FLAGS_IS_OUT_OF_BOUNDS` (`Player.h:438`).
pub(crate) const PLAYER_FLAGS_IS_OUT_OF_BOUNDS: u32 = 0x0000_4000;

/// One-shot test/helper boundary for C++ `Player::ResurrectPlayer`.
///
/// Production resurrection consumers must retain their Player-owned
/// continuation through visibility and obtain spells; this wrapper has no
/// resumable owner if its future is cancelled and is not production-composition
/// evidence.
pub async fn resurrect_player_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: ResurrectionOperationCatalogs<'_>,
    restore_percent: f32,
    apply_sickness: bool,
) -> Result<(), &'static str> {
    let owner = crate::recovery_fences::owner(host);
    if host.core().player_handle_like_cpp.is_some() && owner.is_none() {
        return Err("canonical Player incarnation unavailable for resurrection");
    }
    if let Err(reason) = resurrect_player_native(host, restore_percent) {
        warn!(
            account = host.core().account_id,
            reason, "Native resurrection phase did not complete"
        );
        return Err(reason);
    }
    finish_resurrection_visibility(host, catalogs).await;
    if owner
        .is_some_and(|owner| !crate::recovery_fences::owner_is_current_in_residence(host, owner))
    {
        return Err("canonical Player incarnation or residence changed after visibility update");
    }
    host.cast_all_obtain_spells_like_cpp(catalogs).await?;
    if owner
        .is_some_and(|owner| !crate::recovery_fences::owner_is_current_in_residence(host, owner))
    {
        return Err("canonical Player incarnation or residence changed after obtain spells");
    }
    if apply_sickness {
        apply_resurrection_sickness(host, catalogs).await;
    }
    Ok(())
}

/// C++ `Spell::EffectSelfResurrect` through the same Player-owned recovery
/// continuation used by reclaim and spirit-healer resurrection.
pub async fn self_resurrect_player_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: ResurrectionOperationCatalogs<'_>,
    health: u32,
    mana: u32,
) -> Result<(), &'static str> {
    if !crate::recovery_fences::begin(
        host,
        wow_entities::PlayerResurrectionOperation::SelfRes { health, mana },
        wow_entities::PlayerResurrectionPhase::ResurrectPlayerReady,
    ) {
        host.core_mut()
            .kick("native self-resurrection could not claim the canonical Player continuation");
        return Err("Canonical Player resurrection continuation is unavailable");
    }
    match crate::recovery_fences::continue_operation(host, catalogs).await {
        crate::recovery_fences::Progress::Complete => Ok(()),
        crate::recovery_fences::Progress::Held => {
            host.core_mut()
                .kick("native self-resurrection failed; retain Player continuation");
            Err("Native self-resurrection continuation is held")
        }
        crate::recovery_fences::Progress::Waiting => {
            host.core_mut()
                .kick("native self-resurrection is waiting; retain Player continuation");
            Err("Native self-resurrection continuation is waiting")
        }
        crate::recovery_fences::Progress::Continue => {
            host.core_mut()
                .kick("native self-resurrection did not complete; retain Player continuation");
            Err("Native self-resurrection continuation did not complete")
        }
    }
}

/// Synchronous half of C++ `Player::ResurrectPlayer`, through its native
/// update-field mutations and zone publication. Keeping the forced visibility
/// phase separate lets the continuation preserve its owner fence before
/// `CastAllObtainSpells`.
pub(crate) fn resurrect_player_native<H: CorpseHostLikeCpp>(
    host: &mut H,
    restore_percent: f32,
) -> Result<(), &'static str> {
    if host.player_guid().is_none() {
        return Err("Canonical Player identity is unavailable for resurrection");
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

    // C++ `Player::ResurrectPlayer` removes `PLAYER_FLAGS_IS_OUT_OF_BOUNDS`
    // before aura cleanup (`Player.cpp:4331`).
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player.remove_player_flags_like_cpp(PLAYER_FLAGS_IS_OUT_OF_BOUNDS);
    });

    // This must be called always even on Players with race != RACE_NIGHTELF
    // in case of faction change.
    host.remove_auras_due_to_spell_like_cpp(SPELL_WISP_SPIRIT_AURA_LIKE_CPP)?;
    host.remove_auras_due_to_spell_like_cpp(SPELL_GHOST_LIKE_CPP)?;

    let has_recruiter_link =
        host.core().is_a_recruiter_like_cpp() || host.core().recruiter_id_like_cpp() != 0;
    let alive = host.core().with_owned_player_mut_like_cpp(|player| {
        // C++ restores REFER_A_FRIEND for either recruiter predicate before
        // Player::setDeathState(ALIVE) (`Player.cpp:4337-4338`).
        if has_recruiter_link {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .set_dynamic_flag(wow_constants::unit::UnitDynFlags::ReferAFriend as u32);
        }

        let was_alive = player.unit().is_alive();
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Alive);
        if !was_alive && player.unit().is_alive() {
            // C++ `Player::setDeathState` clears self-res spells on an
            // alive-from-dead transition (`Player.cpp:1196-1198`).
            player.clear_self_res_spells();
        }
    });
    if alive.is_none() {
        return Err("Canonical Player disappeared before resurrection Alive transition");
    }

    // C++ adds this bit first so SetWaterWalking(false) always emits land-walk,
    // including when the incoming Player did not have WATER_WALK set.
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        let mut flags = player.unit().movement_flags_like_cpp();
        flags.insert(wow_constants::MovementFlag::WATER_WALK);
        player.unit_mut().set_movement_flags_like_cpp(flags);
    });
    host.hub_mut().set_player_water_walking(false);
    let stunned = host
        .core()
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .has_unit_state(wow_constants::UnitState::STUNNED.bits())
        })
        .unwrap_or(true);
    if !stunned {
        host.hub_mut().set_player_rooted_like_cpp(false);
    }

    let restored = host.core().with_owned_player_mut_like_cpp(|player| {
        // C++ clears m_deathTimer after land-walk and the guarded unroot.
        player
            .resurrection_state_mut_like_cpp()
            .death_timer_remaining_ms = 0;

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
    });
    let Some(()) = restored else {
        return Err("Canonical Player disappeared while applying resurrection values");
    };
    host.sync_player_registry_state_like_cpp();

    // trigger update zone for alive state zone updates
    if let Some(position) = host.hub_ref().player_position_like_cpp() {
        let map_id = u32::from(host.core().player_map_id_like_cpp());
        let (zone_id, area_id) = terrain_zone_and_area_like_cpp(host, map_id, position);
        host.update_zone_represented_like_cpp(zone_id, area_id);
    }

    Ok(())
}

pub(crate) async fn finish_resurrection_visibility<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: ResurrectionOperationCatalogs<'_>,
) {
    if crate::player_death::current_player_is_in_world_like_cpp(host) {
        let _ = host
            .force_player_visibility_with_catalogs_like_cpp(catalogs.creature_spawns)
            .await;
    }
}

pub(crate) async fn apply_resurrection_sickness<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: ResurrectionOperationCatalogs<'_>,
) {
    let (threshold, level, race_id, sickness_spell_id) = {
        let hub = host.hub_ref();
        let race_id = hub.player_race_like_cpp();
        let sickness_spell_id = hub
            .catalogs
            .chr
            .races_store
            .as_ref()
            .and_then(|store| store.get(u32::from(race_id)))
            .map(|race| race.res_sickness_spell_id);
        (
            i64::from(hub.config.death_policy_like_cpp.sickness_level),
            i64::from(hub.player_level_like_cpp()),
            race_id,
            sickness_spell_id,
        )
    };
    let Some(sickness_spell_id) = sickness_spell_id else {
        warn!(
            account = host.core().account_id,
            race = race_id,
            "Resurrection sickness skipped without ChrRaces data"
        );
        return;
    };
    if level < threshold {
        return;
    }

    let sickness_remaining_ms = if level < threshold + 9 {
        u32::try_from((level - threshold + 1) * 60_000).ok()
    } else {
        None
    };
    if !host
        .cast_triggered_self_spell_like_cpp(
            catalogs,
            sickness_spell_id,
            sickness_remaining_ms,
            None,
        )
        .await
    {
        warn!(
            account = host.core().account_id,
            race = race_id,
            spell_id = sickness_spell_id,
            "Resurrection sickness triggered cast failed"
        );
    }
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

    let restore_percent = if host.hub_ref().player_in_represented_battleground_like_cpp() {
        1.0
    } else {
        0.5
    };
    if !crate::recovery_fences::begin(
        host,
        wow_entities::PlayerResurrectionOperation::ReclaimCorpse { restore_percent },
        wow_entities::PlayerResurrectionPhase::ResurrectPlayerReady,
    ) {
        return false;
    }
    match crate::recovery_fences::continue_operation(host, catalogs.resurrection()).await {
        crate::recovery_fences::Progress::Complete => true,
        crate::recovery_fences::Progress::Waiting => false,
        crate::recovery_fences::Progress::Continue | crate::recovery_fences::Progress::Held => {
            host.core_mut()
                .kick("corpse reclaim recovery did not complete; retain Player owner");
            false
        }
    }
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player auto-attack start and delivery (#1363 slice 4a; moved from the World
//! `handlers/combat.rs`, `session/combat/melee.rs` and the target-visibility
//! context of `session/visibility/operations.rs`).
//!
//! C++ refs: `WorldSession::HandleAttackSwingOpcode`
//! (`Handlers/CombatHandler.cpp`) and `Unit::Attack`. The registration stays in
//! the `ApplicationCombat` area registrar (`combat_handlers`).

use tracing::{debug, warn};
use wow_constants::unit::{UnitFlags, UnitPvpFlags};
use wow_core::ObjectGuid;
use wow_packet::ClientPacket;
use wow_packet::packets::combat::{AttackStart, AttackSwing, SAttackStop};
use wow_world_core::session::mailbox::ApplyPlayerMeleeResultLikeCppCommand;
use wow_world_core::session::{PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP, SessionState};

use crate::world_entities_host::WorldEntitiesHostLikeCpp;

pub const PLAYER_FLAGS_UBER_LIKE_CPP: u32 = 0x0008_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAttackStartLikeCppResult {
    Rejected,
    Accepted { send_attack_start: bool },
}

/// Deliver one map-owned player auto-attack resolution to its attacker.
///
/// #28 moved the transition itself to the map-owned player melee phase, so
/// this is delivery only: it applies no damage and decides no swing (a kill
/// still ends the killer's attack and combat reference, C++ `CombatStop`).
/// The order below is the retired session combat tick's tail verbatim, and
/// the order is what
/// keeps the bytes identical — `SMSG_ATTACKSWING_ERROR` before the swings,
/// the values update gated on what this client already has, the kill queues
/// before `SMSG_ATTACKSTOP`.
pub fn handle_apply_player_melee_result_like_cpp_command_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    command: ApplyPlayerMeleeResultLikeCppCommand,
) {
    use wow_packet::ServerPacket;
    use wow_packet::packets::combat::AttackerStateUpdate;
    use wow_packet::packets::movement::MonsterMoveStop;

    // Re-gate on arrival: a command resolved for one incarnation must not
    // land on a reconnect, another character, or another map.
    if host.state() != SessionState::LoggedIn {
        return;
    }
    if host.player_guid() != Some(command.attacker_guid) {
        return;
    }
    if host.core().player_map_id_like_cpp() != command.map_id {
        return;
    }
    let session_instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    if session_instance_id != command.instance_id {
        return;
    }

    if let Some(swing_error) = command.swing_error_after {
        host.core_mut()
            .set_player_attack_swing_error_like_cpp(swing_error);
    }

    // The construction below is the retired session combat tick's tail
    // verbatim (#1263 F6-8E); the
    // constants and field order are what make the bytes identical.
    if let Some(victim_guid) = command.victim_guid {
        for swing in &command.swings {
            host.send_raw_packet(
                &AttackerStateUpdate {
                    attacker: command.attacker_guid,
                    victim: victim_guid,
                    hit_info: swing.hit_info,
                    damage: swing.damage as i32,
                    original_damage: swing.original_damage as i32,
                    over_damage: swing.over_damage,
                    blocked: swing.blocked,
                    absorbed: 0,
                    victim_state: swing.victim_state,
                    school_mask: 1,
                }
                .to_bytes(),
            );
        }
    }

    if let (Some(values_update), Some(victim_guid)) =
        (command.victim_values_update.as_ref(), command.victim_guid)
        && host
            .core()
            .client_visible_guids_like_cpp
            .contains(&victim_guid)
        && let Some(update) = host.represented_unit_values_update_to_update_object_like_cpp(
            victim_guid,
            command.map_id,
            values_update,
        )
    {
        host.send_raw_packet(&update.to_bytes());
    }

    if let Some(kill) = command.killed_creature.as_ref() {
        // C++ `Unit::setDeathState(JUST_DIED)` → `CombatStop` →
        // `RemoveAllAttackers`: the killer runs `AttackStop`, which also
        // clears its target and `UNIT_STATE_MELEE_ATTACKING` (#1263 F6-8E:
        // the retired session combat tick did this; the map-owned phase
        // only reported the kill).
        let _ = host.core().with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            if unit.attacking() == Some(kill.creature_guid) {
                let _ = unit.attack_stop_like_cpp();
            }
            unit.subsystems_mut()
                .combat
                .purge_combat_ref_like_cpp(kill.creature_guid);
        });
        // Both ends of the reference go together (C++ `Unit::Kill` →
        // `CombatStop`).
        let _ = host
            .core_mut()
            .mutate_world_creature(kill.creature_guid, |creature| {
                creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(command.attacker_guid);
            });
        host.hub_mut()
            .revalidate_canonical_player_combat_refs_like_cpp(command.attacker_guid);
        host.world_entities_mut()
            .queue_pending_creature_kill_like_cpp(
                command.attacker_guid,
                kill.creature_guid,
                kill.creature_entry,
                kill.creature_level,
            );
        if let Some((current_pos, spline_id)) = kill.move_stop {
            host.send_raw_packet(
                &MonsterMoveStop {
                    mover_guid: kill.creature_guid,
                    current_pos,
                    spline_id,
                }
                .to_bytes(),
            );
        }
        host.send_raw_packet(
            &SAttackStop {
                attacker: command.attacker_guid,
                victim: kill.creature_guid,
                now_dead: true,
            }
            .to_bytes(),
        );
    }

    if let Some(combat_target) = command.combat_target_after {
        host.hub_mut().set_combat_target_like_cpp(combat_target);
    }
    if let Some(in_combat) = command.in_combat_after {
        host.hub_mut().set_in_combat_like_cpp(in_combat);
    }
}

/// CMSG_ATTACK_SWING — client requests to attack a target.
///
/// The target must be a known creature in the current map.
/// Sends SMSG_ATTACK_START if the target is valid.
pub async fn handle_attack_swing<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    mut pkt: wow_packet::WorldPacket,
) {
    let swing = match AttackSwing::read(&mut pkt) {
        Ok(s) => s,
        Err(e) => {
            warn!(
                account = host.core().account_id,
                "Failed to read AttackSwing: {e}"
            );
            return;
        }
    };

    let player_guid = match host.player_guid() {
        Some(g) => g,
        None => return,
    };

    debug!(
        account = host.core().account_id,
        target = ?swing.victim,
        "CMSG_ATTACK_SWING"
    );

    // Check the creature exists and is alive.
    let creature_alive = host
        .core()
        .read_world_creature_like_cpp(swing.victim, |c| c.is_alive())
        .unwrap_or(false);

    if !creature_alive {
        // Send attack stop so client clears the attack state.
        let stop = SAttackStop {
            attacker: player_guid,
            victim: swing.victim,
            now_dead: true,
        };
        host.send_packet(&stop);
        return;
    }

    let attack_start = match start_player_attack_like_cpp(host, swing.victim) {
        PlayerAttackStartLikeCppResult::Rejected => {
            let stop = SAttackStop {
                attacker: player_guid,
                victim: swing.victim,
                now_dead: false,
            };
            host.send_packet(&stop);
            return;
        }
        PlayerAttackStartLikeCppResult::Accepted { send_attack_start } => send_attack_start,
    };

    // Engage the canonical map-owned creature after C++-style attack
    // validation succeeds. Like C++ `EngageWithTarget` this sets no victim:
    // the creature's own `UpdateVictim` (aggro tick) picks it and publishes
    // `SMSG_AI_REACTION` + `SMSG_ATTACK_START` (#1344).
    let _ = host
        .core_mut()
        .mutate_world_creature(swing.victim, |creature| {
            creature.engage_with_target_like_cpp(player_guid);
        });

    // Unit::Attack only emits a melee start packet for new targets or a
    // same-target ranged-to-melee switch; same-target no-op is accepted.
    if attack_start {
        let start = AttackStart {
            attacker: player_guid,
            victim: swing.victim,
        };
        host.send_packet(&start);
    }
}

pub fn canonical_unit_attack_target_state_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    guid: ObjectGuid,
) -> (bool, bool, wow_entities::UnitAttackContextLikeCpp) {
    // Resolve the Player-owned control state before taking the map lock;
    // visibility checks below run while that same manager is borrowed.
    let moved_unit_guid = host.hub_ref().player_moved_unit_guid_like_cpp();
    let current_group_guid = host.resolved_group_guid_like_cpp();
    let player_phase_shift = host.represented_player_phase_shift_like_cpp();
    let Some(manager) = host.core().canonical_map_manager.as_ref() else {
        return (
            true,
            true,
            wow_entities::UnitAttackContextLikeCpp::default(),
        );
    };
    let Ok(manager) = manager.lock() else {
        return (
            true,
            true,
            wow_entities::UnitAttackContextLikeCpp::default(),
        );
    };
    let Some(map) = manager.find_map(u32::from(host.core().player_map_id_like_cpp()), 0) else {
        return (
            true,
            true,
            wow_entities::UnitAttackContextLikeCpp::default(),
        );
    };
    if let Some(player) = map.map().get_typed_player(guid) {
        let pvp_flags = player.unit().pvp_flags_like_cpp();
        let attacker_can_see_or_detect_target = player_phase_shift
            .as_ref()
            .is_some_and(|phase| phase.can_see(player.unit().world().phase_shift()))
            && host
                .player_guid()
                .and_then(|attacker_guid| map.map().get_typed_player(attacker_guid))
                .map(|attacker| {
                    let mut target_unit = player.unit().clone();
                    let mut seer_unit = attacker.unit().clone();
                    apply_target_visibility_context_for_current_player_like_cpp(
                        host,
                        &mut target_unit,
                        &mut seer_unit,
                        moved_unit_guid,
                        current_group_guid,
                    );
                    seer_unit.can_see_or_detect_unit_like_cpp(&target_unit, false, true, false)
                })
                .unwrap_or(true);
        return (
            player.unit().is_alive(),
            player.unit().world().object().is_in_world(),
            wow_entities::UnitAttackContextLikeCpp {
                victim_is_game_master_player: player.is_game_master_like_cpp(),
                victim_unit_state: player.unit().unit_state(),
                victim_unit_flags: player.unit().unit_flags_like_cpp().bits(),
                victim_has_affecting_player: true,
                visibility_represented: true,
                attacker_can_see_or_detect_target,
                victim_in_sanctuary: pvp_flags.contains(UnitPvpFlags::SANCTUARY),
                victim_is_pvp: pvp_flags.contains(UnitPvpFlags::PVP),
                victim_is_ffa_pvp: pvp_flags.contains(UnitPvpFlags::FFA_PVP),
                victim_has_pvp_unk1_flag: pvp_flags.contains(UnitPvpFlags::UNK1),
                ..Default::default()
            },
        );
    }
    if let Some(result) = map.map().with_creature_like_cpp(guid, |creature| {
        let attacker_can_see_or_detect_target = player_phase_shift
            .as_ref()
            .is_some_and(|phase| phase.can_see(creature.unit().world().phase_shift()))
            && host
                .player_guid()
                .and_then(|attacker_guid| map.map().get_typed_player(attacker_guid))
                .map(|attacker| {
                    let mut target_unit = creature.unit().clone();
                    let mut seer_unit = attacker.unit().clone();
                    apply_target_visibility_context_for_current_player_like_cpp(
                        host,
                        &mut target_unit,
                        &mut seer_unit,
                        moved_unit_guid,
                        current_group_guid,
                    );
                    seer_unit.can_see_or_detect_unit_like_cpp(&target_unit, false, true, false)
                })
                .unwrap_or(true);
        let mut context = wow_entities::UnitAttackContextLikeCpp {
            victim_is_evading_creature: creature.is_evading_attacks_like_cpp(),
            victim_unit_state: creature.unit().unit_state(),
            victim_unit_flags: creature.unit().unit_flags_like_cpp().bits(),
            visibility_represented: true,
            attacker_can_see_or_detect_target,
            ..Default::default()
        };
        if let Some(reputation_snapshot) = host
            .hub_ref()
            .catalogs
            .attack_reputation_faction_snapshot_like_cpp(creature)
            && let Some(attacker_guid) = host.player_guid()
            && let Some(attacker) = map.map().get_typed_player(attacker_guid)
        {
            let player_has_contested_pvp_flag =
                attacker.has_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
            let creature_has_forced_reputation_rank =
                attacker.has_forced_reputation_rank_like_cpp(reputation_snapshot.faction_id);
            let player_has_reputation_state = match reputation_snapshot.can_have_reputation {
                Some(false) => false,
                Some(true) => {
                    attacker.has_reputation_state_like_cpp(reputation_snapshot.faction_id)
                }
                None => true,
            };
            if (reputation_snapshot.contested_guard && player_has_contested_pvp_flag)
                || creature_has_forced_reputation_rank
                || player_has_reputation_state
            {
                context.player_creature_reputation_represented = true;
                context.creature_is_contested_guard = reputation_snapshot.contested_guard;
                context.player_has_contested_pvp_flag = player_has_contested_pvp_flag;
                context.creature_has_forced_reputation_rank = creature_has_forced_reputation_rank;
                context.player_at_war_with_creature_faction = player_has_reputation_state
                    && attacker.is_at_war_with_faction_like_cpp(reputation_snapshot.faction_id);
            }
        }
        (
            creature.is_alive(),
            creature.unit().world().object().is_in_world(),
            context,
        )
    }) {
        return result;
    }
    (
        true,
        true,
        wow_entities::UnitAttackContextLikeCpp::default(),
    )
}

pub fn start_player_attack_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    victim: ObjectGuid,
) -> PlayerAttackStartLikeCppResult {
    let _ = host.ensure_canonical_world_map_for_current_player_like_cpp();
    let player_guid = host.player_guid();
    let Some((attacker_unit_flags, _, _)) =
        host.hub_ref().player_unit_presentation_snapshot_like_cpp()
    else {
        return PlayerAttackStartLikeCppResult::Rejected;
    };
    let attacker_is_mounted_player = attacker_unit_flags.contains(UnitFlags::MOUNT);
    let (victim_alive, victim_in_world, mut attack_context) =
        canonical_unit_attack_target_state_like_cpp(host, victim);
    if !host.hub_ref().player_vehicle_seat_allows_attack_like_cpp() {
        host.hub_mut().set_combat_target_like_cpp(None);
        host.hub_mut().set_in_combat_like_cpp(false);
        if host.hub_ref().selection_guid_like_cpp() == Some(victim) {
            host.hub_mut().set_selection_guid_like_cpp(None);
        }
        return PlayerAttackStartLikeCppResult::Rejected;
    }
    attack_context.attacker_is_mounted_player = attacker_is_mounted_player;
    attack_context.attacker_unit_flags = attacker_unit_flags.bits();
    attack_context.attacker_has_affecting_player = true;
    let mut attacker_pvp_flags = host
        .core()
        .with_owned_player_like_cpp(|player| player.unit().pvp_flags_like_cpp());
    // The World test build falls back to its represented pvp flags; the host
    // owns that cfg(test) step and returns None otherwise.
    if attacker_pvp_flags.is_none() {
        attacker_pvp_flags = host.attacker_pvp_flags_test_fallback_like_cpp(player_guid);
    }
    let attacker_pvp_flags = attacker_pvp_flags.unwrap_or_default();
    attack_context.attacker_in_sanctuary = attacker_pvp_flags.contains(UnitPvpFlags::SANCTUARY);
    attack_context.attacker_is_ffa_pvp = attacker_pvp_flags.contains(UnitPvpFlags::FFA_PVP);
    attack_context.attacker_has_pvp_unk1_flag = attacker_pvp_flags.contains(UnitPvpFlags::UNK1);
    if attack_context.victim_has_affecting_player {
        attack_context.sanctuary_represented = true;
        attack_context.pvp_represented = true;
        attack_context.player_player_duel_in_progress = player_guid
            .and_then(|guid| host.canonical_player_duel_in_progress_like_cpp(guid, victim))
            .unwrap_or(false);
    }
    attack_context.attacker_is_player_uber = player_guid
        .and_then(|guid| {
            host.core()
                .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_UBER_LIKE_CPP)
        })
        .unwrap_or(false);
    let combat_relation_represented = attack_context.relation_represented;
    let combat_attacker_is_friendly_to_victim = attack_context.attacker_is_friendly_to_victim;
    let combat_victim_is_friendly_to_attacker = attack_context.victim_is_friendly_to_attacker;
    host.hub_mut().set_selection_guid_like_cpp(Some(victim));
    let outcome = host.core().mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().attack_with_context_like_cpp(
            victim,
            victim_alive,
            victim_in_world,
            true,
            attack_context,
        )
    });
    let previous = match outcome {
        Some(wow_entities::UnitAttackStartOutcome::NewTarget { previous }) => previous,
        Some(
            wow_entities::UnitAttackStartOutcome::MeleeStartedSameTarget
            | wow_entities::UnitAttackStartOutcome::MeleeStoppedSameTarget
            | wow_entities::UnitAttackStartOutcome::NoChangeSameTarget,
        ) => None,
        Some(
            wow_entities::UnitAttackStartOutcome::InvalidSelfTarget
            | wow_entities::UnitAttackStartOutcome::InvalidDeadAttacker
            | wow_entities::UnitAttackStartOutcome::InvalidDeadVictim
            | wow_entities::UnitAttackStartOutcome::InvalidVictimNotInWorld
            | wow_entities::UnitAttackStartOutcome::InvalidMountedAttacker
            | wow_entities::UnitAttackStartOutcome::InvalidAttackerEvading
            | wow_entities::UnitAttackStartOutcome::InvalidVictimGameMaster
            | wow_entities::UnitAttackStartOutcome::InvalidVictimEvading
            | wow_entities::UnitAttackStartOutcome::InvalidAttackTarget,
        )
        | None => {
            host.hub_mut().set_combat_target_like_cpp(None);
            host.hub_mut().set_in_combat_like_cpp(false);
            if host.hub_ref().selection_guid_like_cpp() == Some(victim) {
                host.hub_mut().set_selection_guid_like_cpp(None);
            }
            return PlayerAttackStartLikeCppResult::Rejected;
        }
    };
    if let Some(player_guid) = player_guid {
        if let Some(previous) = previous {
            host.hub_mut()
                .remove_canonical_attacker_like_cpp(previous, player_guid);
        }
        host.hub_mut()
            .add_canonical_attacker_like_cpp(victim, player_guid);
        let _ = host.core_mut().mutate_world_creature(victim, |victim| {
            victim.unit_mut().add_attacker_like_cpp(player_guid);
        });
        let _ = host.hub_mut().begin_canonical_player_combat_ref_like_cpp(
            player_guid,
            victim,
            combat_relation_represented,
            combat_attacker_is_friendly_to_victim,
            combat_victim_is_friendly_to_attacker,
        );
    }
    host.hub_mut().set_in_combat_like_cpp(true);
    let send_attack_start = matches!(
        outcome,
        Some(
            wow_entities::UnitAttackStartOutcome::NewTarget { .. }
                | wow_entities::UnitAttackStartOutcome::MeleeStartedSameTarget
        )
    );
    PlayerAttackStartLikeCppResult::Accepted { send_attack_start }
}

pub fn apply_target_visibility_context_for_current_player_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    target_unit: &mut wow_entities::Unit,
    seer_unit: &mut wow_entities::Unit,
    moved_unit_guid: Option<ObjectGuid>,
    current_group_guid: Option<u64>,
) {
    target_unit.set_object_id_visibility_conditions_met_like_cpp(
        host.hub_ref()
            .catalogs
            .object_id_visibility_conditions_met_like_cpp(target_unit.world(), seer_unit.world()),
    );

    let target_guid = target_unit.world().object().guid();
    if moved_unit_guid == Some(target_guid) {
        seer_unit.set_seer_can_always_see_target_like_cpp(true);
    }

    let private_owner = target_unit.private_object_owner_like_cpp();
    if !private_owner.is_empty() {
        seer_unit.set_seer_group_visible_for_private_owner_like_cpp(
            host.current_player_is_in_group_guid_like_cpp(current_group_guid, private_owner),
        );
    }

    let owner_group_visible = target_unit
        .subsystems()
        .control
        .charmer_or_owner_guid()
        .is_some_and(|owner_guid| {
            host.current_player_is_group_visible_for_owner_like_cpp(current_group_guid, owner_guid)
        });
    target_unit.set_target_owner_group_visible_for_seer_like_cpp(owner_group_visible);
}

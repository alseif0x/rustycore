// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The canonical commit of the aggro phase's attack start/stop transitions.
//!
//! #1263 F6-8D3a-1 moves these out of the world-server delivery module so the
//! legacy bridge (which locks the canonical manager per batch) and the
//! admitted canonical executor (which already holds that guard) commit the
//! same transitions through the same body. C++ `Unit::Attack` / `CombatStop`
//! mutate both participants inside one map update; here each command is one
//! `MapCommandLikeCpp` through the map's single writer.

/// Commit creature attack-start commands through each map's single writer.
///
/// The returned outcomes stay in command order so delivery can publish only
/// transitions accepted by the canonical owner.
pub fn apply_creature_attack_start_commands_on_manager_like_cpp(
    manager: &mut wow_map::MapManager,
    commands: &[crate::session::mailbox::CreatureAttackStartLikeCppCommand],
) -> Vec<wow_map::MapCommandOutcomeLikeCpp> {
    commands
        .iter()
        .map(|command| {
            manager.execute_map_command_like_cpp(
                u32::from(command.map_id),
                command.instance_id,
                wow_map::MapCommandLikeCpp::CreatureAttackStart {
                    attacker_guid: command.attacker_guid,
                    victim_guid: command.victim_guid,
                    previous_victim_guid: command.previous_victim_guid,
                },
            )
        })
        .collect()
}

/// Commit creature evade/combat-stop commands through each map's single writer.
///
/// C++ `Unit::CombatStop` removes every attacker and clears the corresponding
/// `CombatReference` from both participants. Player and Creature victims now
/// take the same map-owned path; the session command is delivery-only.
pub fn apply_creature_attack_stop_commands_on_manager_like_cpp(
    manager: &mut wow_map::MapManager,
    commands: &[crate::session::mailbox::CreatureAttackStopLikeCppCommand],
) -> Vec<wow_map::MapCommandOutcomeLikeCpp> {
    commands
        .iter()
        .map(|command| {
            manager.execute_map_command_like_cpp(
                u32::from(command.map_id),
                command.instance_id,
                wow_map::MapCommandLikeCpp::CreatureCombatStop {
                    attacker_guid: command.attacker_guid,
                    victim_guid: command.victim_guid,
                },
            )
        })
        .collect()
}

/// The commands whose canonical transition was applied, in command order.
/// Delivery is an adapter over committed map outcomes: a stale or missing map
/// command never publishes.
pub fn committed_creature_combat_commands_like_cpp<T: Clone>(
    commands: &[T],
    outcomes: &[wow_map::MapCommandOutcomeLikeCpp],
) -> Vec<T> {
    commands
        .iter()
        .zip(outcomes)
        .filter(|(_, outcome)| outcome.is_applied())
        .map(|(command, _)| command.clone())
        .collect()
}

/// Remove attack-start/stop fanout that has no committed canonical map
/// transition behind it while retaining unrelated aggro-plan events.
///
/// The legacy tick still produces wire events before the transitional map
/// command bridge runs. Exact `(source, packet)` signatures couple those two
/// products until the legacy producer itself emits `MapCommandLikeCpp`.
pub fn retain_committed_creature_combat_events_like_cpp(
    plan: &mut crate::map_manager::RuntimePlan,
    start_commands: &[crate::session::mailbox::CreatureAttackStartLikeCppCommand],
    start_outcomes: &[wow_map::MapCommandOutcomeLikeCpp],
    stop_commands: &[crate::session::mailbox::CreatureAttackStopLikeCppCommand],
    stop_outcomes: &[wow_map::MapCommandOutcomeLikeCpp],
) {
    use wow_packet::ServerPacket as _;

    let start_signature = |command: &crate::session::mailbox::CreatureAttackStartLikeCppCommand| {
        (
            command.attacker_guid,
            wow_packet::packets::combat::AttackStart {
                attacker: command.attacker_guid,
                victim: command.victim_guid,
            }
            .to_bytes(),
        )
    };
    let stop_signature = |command: &crate::session::mailbox::CreatureAttackStopLikeCppCommand| {
        (
            command.attacker_guid,
            wow_packet::packets::combat::SAttackStop {
                attacker: command.attacker_guid,
                victim: command.victim_guid,
                now_dead: false,
            }
            .to_bytes(),
        )
    };
    // The hostile `SMSG_AI_REACTION` that precedes a creature's own attack
    // start (C++ `Unit::Attack`) commits or drops together with it (#1344).
    let reaction_signature =
        |command: &crate::session::mailbox::CreatureAttackStartLikeCppCommand| {
            (
                command.attacker_guid,
                wow_packet::packets::combat::AIReaction {
                    unit_guid: command.attacker_guid,
                    reaction: wow_constants::creature::AiReaction::Hostile,
                }
                .to_bytes(),
            )
        };
    let mut all_combat_signatures: Vec<_> = start_commands.iter().map(start_signature).collect();
    all_combat_signatures.extend(start_commands.iter().map(reaction_signature));
    all_combat_signatures.extend(stop_commands.iter().map(stop_signature));
    let mut applied_combat_signatures: Vec<_> = start_commands
        .iter()
        .zip(start_outcomes)
        .filter(|(_, outcome)| outcome.is_applied())
        .flat_map(|(command, _)| [start_signature(command), reaction_signature(command)])
        .collect();
    applied_combat_signatures.extend(
        stop_commands
            .iter()
            .zip(stop_outcomes)
            .filter(|(_, outcome)| outcome.is_applied())
            .map(|(command, _)| stop_signature(command)),
    );

    plan.events.retain(|event| {
        let matches = |(source_guid, packet_bytes): &(wow_core::ObjectGuid, Vec<u8>)| {
            *source_guid == event.source_guid && *packet_bytes == event.packet_bytes
        };
        !all_combat_signatures.iter().any(matches) || applied_combat_signatures.iter().any(matches)
    });
}

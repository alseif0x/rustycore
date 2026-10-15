// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session delivery of map-owned creature combat commands: the creature attack
//! start and stop, the PvP combat expiry and the creature melee-damage
//! receiver (#1363 slice 7a; moved from the World
//! `handlers/loot/combat_commands.rs` and `handlers/loot/handlers.rs`).

use wow_world_core::session::SessionState;
use wow_world_core::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, CreatureAttackStartLikeCppCommand,
    CreatureAttackStopLikeCppCommand, ReconcilePvpCombatExpiryLikeCppCommand,
};

use super::{CombatSessionHostLikeCpp, publish_melee_absorb_consumption_like_cpp};

pub fn handle_creature_attack_start_like_cpp_command_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    command: CreatureAttackStartLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn
        || host.player_guid() != Some(command.victim_guid)
        || host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true)
        || host.core().player_map_id_like_cpp() != command.map_id
    {
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
    let attacker_is_visible = host
        .core()
        .client_visible_guids_like_cpp
        .contains(&command.attacker_guid);
    host.hub_mut().set_in_combat_like_cpp(true);
    if attacker_is_visible && !command.packet_already_broadcast {
        use wow_packet::packets::combat::AttackStart;
        host.send_packet(&AttackStart {
            attacker: command.attacker_guid,
            victim: command.victim_guid,
        });
    }
}

pub fn handle_creature_attack_stop_like_cpp_command_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    command: CreatureAttackStopLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn
        || host.player_guid() != Some(command.victim_guid)
        || host.core().player_map_id_like_cpp() != command.map_id
    {
        return;
    }
    let Some(map_key) = host.core().current_canonical_player_map_key_like_cpp() else {
        return;
    };
    if map_key.instance_id != command.instance_id {
        return;
    }
    let Some(still_in_combat) = host.hub_ref().resolved_in_combat_like_cpp() else {
        return;
    };
    host.hub_mut().set_in_combat_like_cpp(still_in_combat);
}

pub fn handle_reconcile_pvp_combat_expiry_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    command: ReconcilePvpCombatExpiryLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn
        || host.player_guid() != Some(command.player_guid)
        || host.core().player_map_id_like_cpp() != command.map_id
    {
        return;
    }
    let Some(map_key) = host.core().current_canonical_player_map_key_like_cpp() else {
        return;
    };
    if map_key.instance_id != command.instance_id {
        return;
    }
    let still_in_combat = host
        .core()
        .canonical_map_manager
        .as_ref()
        .and_then(|manager| manager.lock().ok())
        .and_then(|manager| {
            manager
                .find_map(map_key.map_id, map_key.instance_id)
                .and_then(|managed| managed.map().get_typed_player(command.player_guid))
                .map(|player| player.unit().subsystems().combat.has_combat())
        })
        .unwrap_or(false);
    host.hub_mut().set_in_combat_like_cpp(still_in_combat);
}

/// Apply a transitional map-owned creature melee compatibility hit to this
/// player session.
///
/// C++ contrast: `Creature::Update` calls `DoMeleeAttackIfReady()`, which
/// eventually emits `AttackerStateUpdate` from the map update tick and
/// then applies damage to the victim. This driver preserves the earlier
/// normal-hit bridge; it does not claim full `CalculateMeleeDamage` parity.
/// It owns the swing timer/damage/canonical health mutation once, and this
/// command is only the victim-session delivery rail. Delivery rereads the
/// current canonical health/death tuple and advances a presentation-only
/// revision, so neither retries nor a delayed command can write an older
/// value over a newer heal, hit, death, or resurrection.
pub fn handle_apply_creature_melee_damage_like_cpp_command_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    command: ApplyCreatureMeleeDamageLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn {
        return;
    }
    if host.player_guid() != Some(command.victim_guid) {
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
    let Some(canonical_health) = present_committed_creature_melee_health_like_cpp(
        host,
        command.victim_health_state_revision_after,
    ) else {
        return;
    };
    // C++ `Unit::Kill` applies the creature-killer durability loss while
    // the lethal damage is applied, before the melee result presentation.
    host.publish_creature_melee_death_durability_loss_like_cpp(command.over_damage);

    // C++ `Unit::CalcAbsorbResist`'s absorb log and shield removal (`Unit.cpp:1876-1889`).
    publish_melee_absorb_consumption_like_cpp(
        host,
        command.attacker_guid,
        command.victim_guid,
        command.original_damage.min(i32::MAX as u32) as i32,
        command.mana_spent,
        &command.absorb_consumptions,
        &command.split_combat_log_packets,
    );
    use wow_packet::packets::combat::{AttackerStateUpdate, HealthUpdate};
    // Visibility gates only the attacker-facing combat packet, never the
    // authoritative victim health/death reconciliation.
    if host
        .core()
        .client_visible_guids_like_cpp
        .contains(&command.attacker_guid)
    {
        host.send_packet(&AttackerStateUpdate {
            attacker: command.attacker_guid,
            victim: command.victim_guid,
            hit_info: command.hit_info,
            damage: command.damage.min(i32::MAX as u32) as i32,
            original_damage: command.original_damage.min(i32::MAX as u32) as i32,
            over_damage: command.over_damage,
            blocked: 0,
            absorbed: command.absorbed.min(i32::MAX as u32) as i32,
            victim_state: command.victim_state,
            school_mask: 1,
            target_level: command.target_level,
            expansion: 2,
        });
    }
    host.hub_ref().publish_self_share_health_like_cpp(&command);
    // An avoided swing commits no health transition.
    if command.damage > 0 {
        host.send_packet(&HealthUpdate {
            guid: command.victim_guid,
            health: canonical_health.min(i64::MAX as u64) as i64,
        });
    }
}

/// Present a health transition already committed by the canonical map
/// owner without replaying the delayed value into canonical state.
///
/// The command revision is a presentation high-water mark. The health and
/// death tuple itself is always reread from the current canonical Player,
/// so a heal, later hit, death, or resurrection that won after the queued
/// swing cannot be rolled back by session delivery.
fn present_committed_creature_melee_health_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    committed_revision: u64,
) -> Option<u64> {
    if committed_revision == 0
        || committed_revision
            <= *host.last_presented_creature_melee_health_state_revision_like_cpp()
    {
        return None;
    }

    let canonical = host.core().with_owned_player_like_cpp(|player| {
        (
            player.unit().health_state_revision_like_cpp(),
            player.unit().data().health,
            player.unit().data().max_health,
            player.unit().is_alive(),
        )
    });
    #[cfg(any(test, feature = "test-fixtures"))]
    let canonical = canonical.or_else(|| {
        if host.test_fixtures_ref_like_cpp().is_none()
            || host.core().player_handle_like_cpp.is_some()
        {
            return None;
        }
        host.core().mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().health_state_revision_like_cpp(),
                player.unit().data().health,
                player.unit().data().max_health,
                player.unit().is_alive(),
            )
        })
    });
    let (canonical_revision, canonical_health, _canonical_max_health, _canonical_alive) =
        canonical?;
    if canonical_revision < committed_revision {
        return None;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures.combat.player_max_health_like_cpp =
            _canonical_max_health.clamp(1, u64::from(u32::MAX)) as u32;
        fixtures.combat.player_health_like_cpp =
            canonical_health.min(u64::from(fixtures.combat.player_max_health_like_cpp)) as u32;
        fixtures.combat.player_alive_like_cpp =
            _canonical_alive && fixtures.combat.player_health_like_cpp > 0;
    }
    *host.last_presented_creature_melee_health_state_revision_like_cpp() = committed_revision;
    host.sync_player_registry_state_like_cpp();
    Some(canonical_health)
}

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical combat-reference transition shared with World.

use wow_core::ObjectGuid;

/// C++ `CombatManager::SetInCombatWith` for a player attacker, on an already
/// locked map.
///
/// Lifted by #28 so the global loop can begin a combat reference without a
/// session. The session variant keeps the lock acquisition and delegates here.
pub fn begin_combat_ref_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    relation_represented: bool,
    attacker_is_friendly_to_victim: bool,
    victim_is_friendly_to_attacker: bool,
) -> bool {
    let Some(attacker) = map.get_typed_player(attacker_guid) else {
        return false;
    };
    let attacker_unit = attacker.unit();
    let attacker_world = attacker_unit.world();
    let attacker_combat = &attacker_unit.subsystems().combat;

    let (context, both_player_controlled) = if let Some(victim) = map.get_typed_player(victim_guid)
    {
        let victim_unit = victim.unit();
        let victim_world = victim_unit.world();
        let victim_combat = &victim_unit.subsystems().combat;
        (
            wow_entities::CombatBeginContextLikeCpp {
                same_unit: attacker_guid == victim_guid,
                attacker_in_world: attacker_world.object().is_in_world(),
                victim_in_world: victim_world.object().is_in_world(),
                attacker_alive: attacker_unit.is_alive(),
                victim_alive: victim_unit.is_alive(),
                same_map: attacker_world.is_in_map(victim_world),
                same_phase: attacker_world.in_same_phase(victim_world),
                attacker_unit_state: attacker_unit.unit_state(),
                victim_unit_state: victim_unit.unit_state(),
                attacker_combat_disallowed: attacker_combat.combat_disallowed,
                victim_combat_disallowed: victim_combat.combat_disallowed,
                relation_represented,
                attacker_is_friendly_to_victim,
                victim_is_friendly_to_attacker,
                attacker_or_owner_player_is_game_master: attacker.is_game_master_like_cpp(),
                victim_or_owner_player_is_game_master: victim.is_game_master_like_cpp(),
            },
            true,
        )
    } else if let Some(result) = map.with_creature_like_cpp(victim_guid, |victim| {
        let victim_unit = victim.unit();
        let victim_world = victim_unit.world();
        let victim_combat = &victim_unit.subsystems().combat;
        (
            wow_entities::CombatBeginContextLikeCpp {
                same_unit: false,
                attacker_in_world: attacker_world.object().is_in_world(),
                victim_in_world: victim_world.object().is_in_world(),
                attacker_alive: attacker_unit.is_alive(),
                victim_alive: victim_unit.is_alive(),
                same_map: attacker_world.is_in_map(victim_world),
                same_phase: attacker_world.in_same_phase(victim_world),
                attacker_unit_state: attacker_unit.unit_state(),
                victim_unit_state: victim_unit.unit_state(),
                attacker_combat_disallowed: attacker_combat.combat_disallowed,
                victim_combat_disallowed: victim_combat.combat_disallowed,
                relation_represented,
                attacker_is_friendly_to_victim,
                victim_is_friendly_to_attacker,
                attacker_or_owner_player_is_game_master: attacker.is_game_master_like_cpp(),
                victim_or_owner_player_is_game_master: false,
            },
            false,
        )
    }) {
        result
    } else {
        return false;
    };

    if !wow_entities::CombatSubsystem::can_begin_combat_like_cpp(context) {
        return false;
    }

    let Some(attacker) = map.get_typed_player_mut(attacker_guid) else {
        return false;
    };
    let attacker_started = attacker
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(victim_guid, both_player_controlled, false);

    let victim_started = if let Some(victim) = map.get_typed_player_mut(victim_guid) {
        victim
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_in_combat_with(attacker_guid, both_player_controlled, false)
    } else if let Some(victim) = map.get_typed_creature_mut(victim_guid) {
        victim
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_in_combat_with(attacker_guid, both_player_controlled, false)
    } else {
        false
    };

    attacker_started && victim_started
}

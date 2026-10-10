//! Shared legacy creature tick entry points.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;
use wow_core::{position_is_in_dist_strict_2d_like_cpp, position_is_in_dist_strict_3d_like_cpp};

pub(in crate::session) fn legacy_creature_snapshot_is_hostile_to_creature_like_cpp(
    creature: &wow_entities::Creature,
    target: &LegacyCreatureAggroOwnerSnapshotLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<bool> {
    legacy_creature_reaction_to_snapshot_like_cpp(creature, target, config)
        .map(|rank| rank <= wow_data::reputation::ReputationRankLikeCpp::Hostile)
}
/// C++ `WorldObject::GetFactionReactionTo` common faction branch for a
/// represented unit snapshot.
pub(in crate::session) fn legacy_creature_reaction_to_snapshot_like_cpp(
    creature: &wow_entities::Creature,
    target: &LegacyCreatureAggroOwnerSnapshotLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<wow_data::reputation::ReputationRankLikeCpp> {
    use wow_data::reputation::ReputationRankLikeCpp;
    let faction_templates = config.faction_template_store.as_ref()?;
    let creature_faction =
        faction_templates.get(u32::try_from(creature.unit().data().faction_template).ok()?)?;
    let target_faction = faction_templates.get(target.faction_template_id?)?;

    if creature_faction.is_hostile_to_like_cpp(target_faction) {
        return Some(ReputationRankLikeCpp::Hostile);
    }
    if creature_faction.is_friendly_to_like_cpp(target_faction)
        || target_faction.is_friendly_to_like_cpp(creature_faction)
    {
        return Some(ReputationRankLikeCpp::Friendly);
    }
    Some(if creature_faction.is_hostile_by_default_like_cpp() {
        ReputationRankLikeCpp::Hostile
    } else {
        ReputationRankLikeCpp::Neutral
    })
}
pub(in crate::session) fn legacy_creature_ai_selection_decision_like_cpp(
    creature: &wow_entities::Creature,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureAiSelectionDecisionLikeCpp {
    let metadata = creature.lifecycle_metadata();
    let is_pet = creature.guid().is_pet();

    // C++ pet override runs before ScriptName and AIName.
    if !is_pet && !metadata.script_name.is_empty() {
        return LegacyCreatureAiSelectionDecisionLikeCpp::ScriptRegistryUnrepresented;
    }

    let flags_extra = CreatureFlagsExtra::from_bits_truncate(metadata.flags_extra);
    let input = CreatureAiSelectionInputLikeCpp {
        ai_name: metadata.ai_name.clone(),
        script_name: metadata.script_name.clone(),
        script_can_create_creature_ai: false,
        is_pet,
        is_vehicle: creature.is_vehicle_unit_type_like_cpp(),
        is_totem: creature.is_totem_unit_type_like_cpp(),
        is_trigger: flags_extra.contains(CreatureFlagsExtra::TRIGGER),
        first_spell_id: creature.spells()[0],
        is_critter: metadata.creature_type == CreatureType::Critter as u32,
        is_guardian: creature.is_guardian_unit_type_like_cpp(),
        is_guard: flags_extra.contains(CreatureFlagsExtra::GUARD),
        is_civilian: creature.is_civilian_like_cpp(),
        is_neutral_to_all: config.creature_faction_template_is_neutral_to_all_like_cpp(
            creature.unit().data().faction_template.max(0) as u32,
        ),
        has_spellclick_npc_flag: NPCFlags1::from_bits_truncate(creature.npc_flags())
            .contains(NPCFlags1::SPELL_CLICK),
        is_controllable_guardian: creature.is_controlable_guardian_unit_type_like_cpp(),
        controllable_guardian_owner_is_player: creature
            .unit()
            .subsystems()
            .control
            .charmer_or_owner_guid()
            .is_some_and(|guid| guid.is_player()),
    };

    LegacyCreatureAiSelectionDecisionLikeCpp::Selected(select_creature_ai_like_cpp(&input))
}
pub(in crate::session) fn legacy_creature_ai_can_attack_decision_like_cpp(
    ai_kind: &CreatureAiKindLikeCpp,
    creature: &wow_entities::Creature,
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureAiCanAttackDecisionLikeCpp {
    let mut input = CreatureAiCanAttackInputLikeCpp::default();
    if matches!(ai_kind, CreatureAiKindLikeCpp::TurretAI) {
        let first_spell_id = creature.spells()[0];
        let Some(misc_store) = config.spell_misc_store.as_ref() else {
            return LegacyCreatureAiCanAttackDecisionLikeCpp::Unrepresented;
        };
        let Some(range_store) = config.spell_range_store.as_ref() else {
            return LegacyCreatureAiCanAttackDecisionLikeCpp::Unrepresented;
        };
        // `TurretAI` caches GetMin/MaxRange from the active map difficulty in
        // its constructor (CombatAI.cpp:196-199).
        let Some(misc) = misc_store.entry_for_spell_difficulty_with_fallback_like_cpp(
            first_spell_id,
            candidate.map_difficulty_id,
            config.difficulty_store.as_deref(),
        ) else {
            return LegacyCreatureAiCanAttackDecisionLikeCpp::Unrepresented;
        };
        let Some(range) = range_store.get(u32::from(misc.range_index)) else {
            return LegacyCreatureAiCanAttackDecisionLikeCpp::Unrepresented;
        };
        // `Unit::IsWithinCombatRange` compares squared center distance with
        // `(requested range + both combat reaches)^2` using strict `<`.
        let distance_sq = creature.position().distance_sq(&candidate.position);
        let reach_sum =
            creature.unit().world().combat_reach() + candidate.player_combat_reach.max(0.0);
        let combat_distance = range.range_max[0] + reach_sum;
        let minimum_range = range.range_min[0];
        input.target_within_turret_combat_range = distance_sq < combat_distance * combat_distance;
        input.target_within_turret_min_range = minimum_range != 0.0
            && distance_sq < (minimum_range + reach_sum) * (minimum_range + reach_sum);
    }

    if creature_ai_can_attack_like_cpp(ai_kind, &input) {
        LegacyCreatureAiCanAttackDecisionLikeCpp::Allowed
    } else {
        LegacyCreatureAiCanAttackDecisionLikeCpp::Rejected
    }
}
pub(in crate::session) fn legacy_creature_can_attack_leash_decision_like_cpp(
    creature: &wow_entities::Creature,
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
    owner_snapshots: &HashMap<ObjectGuid, LegacyCreatureAggroOwnerSnapshotLikeCpp>,
) -> LegacyCreatureCanAttackLeashDecisionLikeCpp {
    let charmer_or_owner_guid = creature.unit().subsystems().control.charmer_or_owner_guid();
    let charmer_or_owner_is_player = charmer_or_owner_guid.is_some_and(|guid| guid.is_player());

    if !charmer_or_owner_is_player && config.map_is_dungeon_like_cpp(candidate.map_id) {
        return LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed;
    }
    if !charmer_or_owner_is_player
        && !creature.is_world_boss_like_cpp()
        && (creature.last_damaged_time() > wow_entities::game_time_secs_like_cpp()
            || creature
                .unit()
                .subsystems()
                .auras
                .has_aura_type_like_cpp(wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT))
    {
        return LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed;
    }

    let mut max_home_distance = config
        .map_visibility_range_like_cpp(candidate.map_id)
        .min(SIZE_OF_GRID_CELL * 2.0);

    // C++ uses `GetCharmerOrOwner()` as the leash center after the non-player
    // dungeon/recent-damage bypasses. This transitional scan represents active
    // player candidates and map-owned creatures on the same map instance;
    // anything else fails closed instead of pretending home position is
    // equivalent.
    if let Some(owner_guid) = charmer_or_owner_guid {
        let Some(owner) = owner_snapshots.get(&owner_guid) else {
            return LegacyCreatureCanAttackLeashDecisionLikeCpp::OwnerPositionUnrepresented;
        };
        if owner.map_id != candidate.map_id || owner.instance_id != candidate.instance_id {
            return LegacyCreatureCanAttackLeashDecisionLikeCpp::OwnerPositionUnrepresented;
        }

        max_home_distance += candidate.player_combat_reach.max(0.0) + owner.combat_reach.max(0.0);
        return if position_is_in_dist_strict_3d_like_cpp(
            &candidate.position,
            &owner.position,
            max_home_distance,
        ) {
            LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed
        } else {
            LegacyCreatureCanAttackLeashDecisionLikeCpp::HomeRangeRejected
        };
    }

    max_home_distance +=
        creature.unit().world().combat_reach() + candidate.player_combat_reach.max(0.0);
    let home_position = creature.home_position();

    if creature.flight_movement_type_like_cpp()
        != wow_constants::CreatureFlightMovementType::None as u8
    {
        if position_is_in_dist_strict_2d_like_cpp(
            &candidate.position,
            &home_position,
            max_home_distance,
        ) {
            LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed
        } else {
            LegacyCreatureCanAttackLeashDecisionLikeCpp::HomeRangeRejected
        }
    } else if position_is_in_dist_strict_3d_like_cpp(
        &candidate.position,
        &home_position,
        max_home_distance,
    ) {
        LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed
    } else {
        LegacyCreatureCanAttackLeashDecisionLikeCpp::HomeRangeRejected
    }
}
pub(in crate::session) fn legacy_creature_can_attack_snapshot_leash_decision_like_cpp(
    creature: &wow_entities::Creature,
    target: &LegacyCreatureAggroOwnerSnapshotLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
    owner_snapshots: &HashMap<ObjectGuid, LegacyCreatureAggroOwnerSnapshotLikeCpp>,
) -> LegacyCreatureCanAttackLeashDecisionLikeCpp {
    let charmer_or_owner_guid = creature.unit().subsystems().control.charmer_or_owner_guid();
    let charmer_or_owner_is_player = charmer_or_owner_guid.is_some_and(|guid| guid.is_player());

    if !charmer_or_owner_is_player && config.map_is_dungeon_like_cpp(target.map_id) {
        return LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed;
    }
    if !charmer_or_owner_is_player
        && !creature.is_world_boss_like_cpp()
        && (creature.last_damaged_time() > wow_entities::game_time_secs_like_cpp()
            || creature
                .unit()
                .subsystems()
                .auras
                .has_aura_type_like_cpp(wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT))
    {
        return LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed;
    }

    let mut max_home_distance = config
        .map_visibility_range_like_cpp(target.map_id)
        .min(SIZE_OF_GRID_CELL * 2.0);
    if let Some(owner_guid) = charmer_or_owner_guid {
        let Some(owner) = owner_snapshots.get(&owner_guid) else {
            return LegacyCreatureCanAttackLeashDecisionLikeCpp::OwnerPositionUnrepresented;
        };
        if owner.map_id != target.map_id || owner.instance_id != target.instance_id {
            return LegacyCreatureCanAttackLeashDecisionLikeCpp::OwnerPositionUnrepresented;
        }

        max_home_distance += target.combat_reach.max(0.0) + owner.combat_reach.max(0.0);
        return if position_is_in_dist_strict_3d_like_cpp(
            &target.position,
            &owner.position,
            max_home_distance,
        ) {
            LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed
        } else {
            LegacyCreatureCanAttackLeashDecisionLikeCpp::HomeRangeRejected
        };
    }

    max_home_distance += creature.unit().world().combat_reach() + target.combat_reach.max(0.0);
    let home_position = creature.home_position();
    let in_range = if creature.flight_movement_type_like_cpp()
        != wow_constants::CreatureFlightMovementType::None as u8
    {
        position_is_in_dist_strict_2d_like_cpp(&target.position, &home_position, max_home_distance)
    } else {
        position_is_in_dist_strict_3d_like_cpp(&target.position, &home_position, max_home_distance)
    };
    if in_range {
        LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed
    } else {
        LegacyCreatureCanAttackLeashDecisionLikeCpp::HomeRangeRejected
    }
}
/// The decision half of C++ `CreatureAI::TriggerAlert` after
/// `CreatureUnitRelocationWorker`: only hostile stealthed players can distract
/// an alive, non-engaged, non-controlled aggressive creature. Returns the
/// `MoveDistract` facing; the caller runs the distract movement on its store
/// and then sends `SMSG_AI_REACTION` (alert) to the visible set.
pub(in crate::session) fn legacy_creature_alert_orientation_like_cpp(
    creature: &wow_entities::Creature,
    candidate: &LegacyCreatureAggroCandidateLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<f32> {
    if !legacy_creature_aggro_candidate_has_stealth_aura_like_cpp(candidate) {
        return None;
    }
    if creature.ai_ownership().combat_target.is_some() {
        return None;
    }
    if creature.is_civilian_like_cpp()
        || creature.has_react_state(wow_entities::ReactState::Passive)
    {
        return None;
    }
    if creature.unit().has_unit_state(
        (UnitState::CONFUSED | UnitState::STUNNED | UnitState::FLEEING | UnitState::DISTRACTED)
            .bits(),
    ) {
        return None;
    }
    if !legacy_creature_aggro_candidate_is_targetable_for_attack_like_cpp(candidate) {
        return None;
    }
    if !legacy_creature_aggro_candidate_is_hostile_to_creature_like_cpp(creature, candidate, config)
        .unwrap_or(false)
    {
        return None;
    }
    Some(creature.position().angle_to(&candidate.position))
}

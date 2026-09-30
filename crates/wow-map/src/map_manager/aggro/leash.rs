//! Shared Aggro rules, preserving the legacy source gates and order.
use super::*;

pub fn candidate_leash(
    creature: &WorldCreature,
    candidate: &AggroCandidate,
    config: &AggroSettings,
    owner_snapshots: &HashMap<ObjectGuid, AggroOwnerSnapshot>,
) -> AggroLeash {
    let charmer_or_owner_guid = creature
        .creature
        .unit()
        .subsystems()
        .control
        .charmer_or_owner_guid();
    let charmer_or_owner_is_player = charmer_or_owner_guid.is_some_and(|guid| guid.is_player());

    if !charmer_or_owner_is_player && config.map_is_dungeon {
        return AggroLeash::Allowed;
    }
    if !charmer_or_owner_is_player
        && !creature.creature.is_world_boss_like_cpp()
        && (creature.creature.last_damaged_time() > wow_entities::game_time_secs_like_cpp()
            || creature
                .creature
                .unit()
                .subsystems()
                .auras
                .has_aura_type_like_cpp(wow_constants::spell::aura_types::SPELL_AURA_MOD_TAUNT))
    {
        return AggroLeash::Allowed;
    }

    let mut max_home_distance = config.map_visibility_range.min(SIZE_OF_GRID_CELL * 2.0);

    // C++ uses `GetCharmerOrOwner()` as the leash center after the non-player
    // dungeon/recent-damage bypasses. This transitional scan represents active
    // player candidates and map-owned creatures on the same map instance;
    // anything else fails closed instead of pretending home position is
    // equivalent.
    if let Some(owner_guid) = charmer_or_owner_guid {
        let Some(owner) = owner_snapshots.get(&owner_guid) else {
            return AggroLeash::OwnerPositionUnrepresented;
        };
        if owner.map_id != candidate.map_id || owner.instance_id != candidate.instance_id {
            return AggroLeash::OwnerPositionUnrepresented;
        }

        max_home_distance += candidate.player_combat_reach.max(0.0) + owner.combat_reach.max(0.0);
        return if position_is_in_dist_strict_3d_like_cpp(
            &candidate.position,
            &owner.position,
            max_home_distance,
        ) {
            AggroLeash::Allowed
        } else {
            AggroLeash::HomeRangeRejected
        };
    }

    max_home_distance +=
        creature.creature.unit().world().combat_reach() + candidate.player_combat_reach.max(0.0);
    let home_position = creature.home_position();

    if creature.creature.flight_movement_type_like_cpp()
        != wow_constants::CreatureFlightMovementType::None as u8
    {
        if position_is_in_dist_strict_2d_like_cpp(
            &candidate.position,
            &home_position,
            max_home_distance,
        ) {
            AggroLeash::Allowed
        } else {
            AggroLeash::HomeRangeRejected
        }
    } else if position_is_in_dist_strict_3d_like_cpp(
        &candidate.position,
        &home_position,
        max_home_distance,
    ) {
        AggroLeash::Allowed
    } else {
        AggroLeash::HomeRangeRejected
    }
}
pub fn snapshot_leash(
    creature: &WorldCreature,
    target: &AggroOwnerSnapshot,
    config: &AggroSettings,
    owner_snapshots: &HashMap<ObjectGuid, AggroOwnerSnapshot>,
) -> AggroLeash {
    let charmer_or_owner_guid = creature
        .creature
        .unit()
        .subsystems()
        .control
        .charmer_or_owner_guid();
    let charmer_or_owner_is_player = charmer_or_owner_guid.is_some_and(|guid| guid.is_player());

    if !charmer_or_owner_is_player && config.map_is_dungeon {
        return AggroLeash::Allowed;
    }
    if !charmer_or_owner_is_player
        && !creature.creature.is_world_boss_like_cpp()
        && (creature.creature.last_damaged_time() > wow_entities::game_time_secs_like_cpp()
            || creature
                .creature
                .unit()
                .subsystems()
                .auras
                .has_aura_type_like_cpp(wow_constants::spell::aura_types::SPELL_AURA_MOD_TAUNT))
    {
        return AggroLeash::Allowed;
    }

    let mut max_home_distance = config.map_visibility_range.min(SIZE_OF_GRID_CELL * 2.0);
    if let Some(owner_guid) = charmer_or_owner_guid {
        let Some(owner) = owner_snapshots.get(&owner_guid) else {
            return AggroLeash::OwnerPositionUnrepresented;
        };
        if owner.map_id != target.map_id || owner.instance_id != target.instance_id {
            return AggroLeash::OwnerPositionUnrepresented;
        }

        max_home_distance += target.combat_reach.max(0.0) + owner.combat_reach.max(0.0);
        return if position_is_in_dist_strict_3d_like_cpp(
            &target.position,
            &owner.position,
            max_home_distance,
        ) {
            AggroLeash::Allowed
        } else {
            AggroLeash::HomeRangeRejected
        };
    }

    max_home_distance +=
        creature.creature.unit().world().combat_reach() + target.combat_reach.max(0.0);
    let home_position = creature.home_position();
    let in_range = if creature.creature.flight_movement_type_like_cpp()
        != wow_constants::CreatureFlightMovementType::None as u8
    {
        position_is_in_dist_strict_2d_like_cpp(&target.position, &home_position, max_home_distance)
    } else {
        position_is_in_dist_strict_3d_like_cpp(&target.position, &home_position, max_home_distance)
    };
    if in_range {
        AggroLeash::Allowed
    } else {
        AggroLeash::HomeRangeRejected
    }
}

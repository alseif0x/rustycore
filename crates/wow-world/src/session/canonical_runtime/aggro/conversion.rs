//! Narrow conversions retain legacy public DTO paths and full input facts.
use super::*;

pub(in crate::session) fn owned_candidate(candidate: LegacyCreatureAggroCandidateLikeCpp) -> AggroCandidate {
    AggroCandidate {
        player_guid: candidate.player_guid,
        map_id: candidate.map_id,
        instance_id: candidate.instance_id,
        map_difficulty_id: candidate.map_difficulty_id,
        position: candidate.position,
        player_visibility_represented: candidate.player_visibility_represented,
        player_phase_shift: candidate.player_phase_shift,
        player_visibility_detection: candidate.player_visibility_detection,
        player_combat_reach: candidate.player_combat_reach,
        player_detected_range_aura_mod: candidate.player_detected_range_aura_mod,
        player_liquid_status: candidate.player_liquid_status_like_cpp,
        player_level: candidate.player_level,
        player_gray_level: candidate.player_gray_level,
        player_unit_flags: candidate.player_unit_flags,
        player_unit_flags2: candidate.player_unit_flags2,
        player_unit_state: candidate.player_unit_state,
        player_is_game_master: candidate.player_is_game_master,
        player_is_contested_pvp: candidate.player_is_contested_pvp,
        player_faction_template_id: candidate.player_faction_template_id,
        player_reputation_standings: candidate.player_reputation_standings,
        player_reputation_state_flags: candidate.player_reputation_state_flags,
        player_forced_reputation_ranks: candidate.player_forced_reputation_ranks,
        player_forced_reputation_faction_ids: candidate.player_forced_reputation_faction_ids,
        player_school_immunity_mask: candidate.player_school_immunity_mask,
        player_damage_immunity_mask: candidate.player_damage_immunity_mask,
        player_has_confuse_aura: candidate.player_has_confuse_aura,
        player_has_breakable_stun_aura: candidate.player_has_breakable_stun_aura,
    }
}

pub(in crate::session) fn settings(config: &LegacyCreatureAggroConfigLikeCpp, map_id: u16) -> AggroSettings {
    AggroSettings {
        no_gray_aggro_above: config.no_gray_aggro_above, no_gray_aggro_below: config.no_gray_aggro_below,
        creature_aggro_rate: config.creature_aggro_rate, max_player_level_config: config.max_player_level_config,
        family_assistance_radius: config.family_assistance_radius,
        family_assistance_delay_ms: config.family_assistance_delay_ms,
        map_is_dungeon: config.map_is_dungeon_like_cpp(map_id),
        map_visibility_range: config.map_visibility_range_like_cpp(map_id),
    }
}

pub(in crate::session) fn owner(snapshot: &LegacyCreatureAggroOwnerSnapshotLikeCpp) -> AggroOwnerSnapshot {
    AggroOwnerSnapshot {
        map_id: snapshot.map_id,
        instance_id: snapshot.instance_id,
        position: snapshot.position,
        phase_shift: snapshot.phase_shift.clone(),
        combat_reach: snapshot.combat_reach,
        alive: snapshot.alive,
        in_water: snapshot.in_water,
        in_evade_mode: snapshot.in_evade_mode,
        unit_flags: snapshot.unit_flags,
        faction_template_id: snapshot.faction_template_id,
        school_immunity_mask: snapshot.school_immunity_mask,
        damage_immunity_mask: snapshot.damage_immunity_mask,
        has_confuse_aura: snapshot.has_confuse_aura,
        has_breakable_stun_aura: snapshot.has_breakable_stun_aura,
    }
}

pub(in crate::session) fn owners(snapshots: &HashMap<ObjectGuid, LegacyCreatureAggroOwnerSnapshotLikeCpp>)
    -> HashMap<ObjectGuid, AggroOwnerSnapshot> {
    snapshots.iter().map(|(guid, snapshot)| (*guid, owner(snapshot))).collect()
}

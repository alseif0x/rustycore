// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature aggro contracts shared by the legacy runtime adapters.
//!
//! Session-independent: moved out of `wow-world` under #1263 F5.

use wow_ai::CreatureAiKindLikeCpp;
use wow_constants::UnitFlags;
use wow_core::{ObjectGuid, Position};
use wow_data::CreatureSpellDisableDecisionLikeCpp;
use wow_entities::{PhaseShift, UnitVisibilityDetectionStateLikeCpp};
use wow_world_core::map_manager::RuntimePlan;

pub use wow_world_core::session::DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP;
pub use wow_world_core::session::LegacyCreatureAggroConfigLikeCpp;
pub use wow_world_core::session::spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp;

#[derive(Debug, Clone)]
pub struct LegacyCreatureMovementTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub movement_packets: usize,
    pub canonical_syncs: usize,
    pub plan: wow_world_core::map_manager::RuntimePlan,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyCreatureLifecycleTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub corpses_despawned: usize,
    pub respawns_processed: usize,
    pub respawn_db_mutations: Vec<wow_persistence::RespawnPersistenceMutationLikeCpp>,
    pub canonical_removes: usize,
    pub canonical_inserts: usize,
    pub canonical_respawn_adds: usize,
    pub canonical_respawn_removes: usize,
    /// Ready respawns whose canonical admission was **refused** because a
    /// competing claimable pool already owns the GUID. The legacy store
    /// published nothing for them (F6-8A publication gate).
    pub respawn_publications_refused_like_cpp: usize,
    /// Ready respawns whose canonical admission **deferred** them — no canonical
    /// map instance for the exact `(map_id, instance_id)` key, or the
    /// installation failed — so they were returned to the map's own spawn queue
    /// instead of being published without an admitted owner.
    pub respawn_publications_deferred_like_cpp: usize,
    /// Ready respawns whose legacy publication **failed** because the store
    /// refused the insertion (a concurrent publisher won the GUID), counted
    /// **before** the fresh-insertion condition is examined. It therefore also
    /// counts failures with **no** canonical rollback: only a failure that
    /// followed a fresh canonical incarnation installed by this same tick undoes
    /// that incarnation. Nothing stays published either way.
    pub respawn_publications_failed_like_cpp: usize,
    /// Map instances whose sessions must recompute creature visibility.
    pub refresh_map_keys: Vec<(u16, u32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyCreatureAggroCandidateLikeCpp {
    pub player_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub map_difficulty_id: u8,
    pub position: Position,
    pub player_visibility_represented: bool,
    pub player_phase_shift: PhaseShift,
    pub player_visibility_detection: UnitVisibilityDetectionStateLikeCpp,
    pub player_combat_reach: f32,
    pub player_detected_range_aura_mod: f32,
    pub player_liquid_status_like_cpp: u32,
    pub player_level: u8,
    pub player_gray_level: u8,
    pub player_unit_flags: u32,
    pub player_unit_flags2: u32,
    pub player_unit_state: u32,
    pub player_is_game_master: bool,
    pub player_is_contested_pvp: bool,
    pub player_faction_template_id: u32,
    pub player_reputation_standings: Vec<(u32, i32)>,
    pub player_reputation_state_flags: Vec<(u32, u32)>,
    pub player_forced_reputation_ranks: Vec<(u32, wow_data::reputation::ReputationRankLikeCpp)>,
    pub player_forced_reputation_faction_ids: Vec<u32>,
    pub player_school_immunity_mask: u32,
    pub player_damage_immunity_mask: u32,
    pub player_has_confuse_aura: bool,
    pub player_has_breakable_stun_aura: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyCreatureAggroOwnerSnapshotLikeCpp {
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub phase_shift: PhaseShift,
    pub combat_reach: f32,
    pub alive: bool,
    pub in_water: bool,
    pub in_evade_mode: bool,
    pub unit_flags: UnitFlags,
    pub faction_template_id: Option<u32>,
    pub school_immunity_mask: u32,
    pub damage_immunity_mask: u32,
    pub has_confuse_aura: bool,
    pub has_breakable_stun_aura: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyCreatureAggroTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub sightless_creatures_skipped: usize,
    pub candidates_seen: usize,
    pub targetability_rejections: usize,
    pub visibility_unrepresented: usize,
    pub visibility_rejections: usize,
    pub hostility_rejections: usize,
    pub hostility_unrepresented: usize,
    pub accessibility_rejections: usize,
    pub owner_position_unrepresented: usize,
    pub attacker_evade_rejections: usize,
    pub home_range_rejections: usize,
    pub gray_aggro_rejections: usize,
    pub ai_selection_unrepresented: usize,
    pub ai_los_suppressed: usize,
    pub ai_can_attack_unrepresented: usize,
    pub ai_can_attack_rejections: usize,
    pub alert_triggers: usize,
    pub alert_rejections: usize,
    /// #1263 F6-8D3a-1: an alert whose `MoveDistract` the phase store does not
    /// represent (the admitted canonical store until F6-8D3a-2). Refused, so no
    /// alert reaction is published without its movement.
    pub alert_movement_unrepresented: usize,
    pub movement_interrupts: usize,
    pub victim_switches: usize,
    pub evades_started: usize,
    pub assistance_scheduled: usize,
    pub assistance_starts: usize,
    /// #1263 F6-8C: creature objects the aggro selector refused because the
    /// canonical designated owner holds no incarnation for them. A surviving
    /// legacy copy decides nothing by itself.
    pub canonical_incarnation_rejections: usize,
    pub plan: wow_world_core::map_manager::RuntimePlan,
    pub aggro_starts: usize,
    pub commands: Vec<wow_world_core::session::mailbox::CreatureAttackStartLikeCppCommand>,
    pub stop_commands: Vec<wow_world_core::session::mailbox::CreatureAttackStopLikeCppCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyCreatureThreatUpdateLikeCpp {
    Unchanged,
    Switched {
        previous_victim: ObjectGuid,
    },
    Evade {
        previous_victim: Option<ObjectGuid>,
        participant_guids: Vec<ObjectGuid>,
        removed_taunt_slots: Vec<u8>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct LegacyCreatureMeleeTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub swings_ready: usize,
    pub runtime_rng_authority_rejections: usize,
    pub melee_outcomes_unrepresented: usize,
    pub melee_precondition_rejections: usize,
    pub melee_range_rejections: usize,
    pub melee_facing_rejections: usize,
    pub attacker_state_rejections: usize,
    pub attacker_incarnation_rejections: usize,
    pub melee_los_rejections: usize,
    pub attacking_interrupt_auras_removed: usize,
    pub canonical_hits: usize,
    pub canonical_creature_hits: usize,
    pub legacy_creature_victim_syncs: usize,
    pub legacy_creature_victim_sync_cas_rejections: usize,
    /// #1263 F6-8C: swing selections refused because the attacker's canonical
    /// designated owner holds no incarnation at its residence.
    pub canonical_incarnation_rejections: usize,
    pub commands: Vec<wow_world_core::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand>,
    pub plan: RuntimePlan,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyCreatureSpellTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub ai_selection_unrepresented: usize,
    pub missing_spell_metadata: usize,
    pub schedules_initialized: usize,
    pub casts_ready: usize,
    pub noninstant_casts_unrepresented: usize,
    pub spell_runtime_hooks_unrepresented: usize,
    pub spell_casting_requirements_unrepresented: usize,
    pub spell_disable_context_unrepresented: usize,
    pub spells_disabled: usize,
    /// TurretAI attempts whose rejected `CastSpell` still consumed BASE_ATTACK
    /// because the raw combat-range gate had admitted them.
    pub turret_rejected_attempt_swings: usize,
    /// Casts dropped because the live creature was no longer the incarnation the
    /// plan had been captured from.
    pub caster_incarnation_rejections: usize,
    pub spell_effects_unrepresented: usize,
    pub spell_projectiles_unrepresented: usize,
    pub spell_visuals_unrepresented: usize,
    pub unit_state_casting_skips: usize,
    pub spell_range_rejections: usize,
    pub spell_los_rejections: usize,
    pub spell_hit_results_unrepresented: usize,
    pub runtime_rng_authority_rejections: usize,
    pub spell_hits: usize,
    pub spell_misses: usize,
    pub canonical_cast_preconditions_passed: usize,
    pub canonical_cast_missing_target: usize,
    pub canonical_cast_target_rejections: usize,
    pub canonical_cast_cooldown_rejections: usize,
    /// #1263 F6-8C: template-spell selections refused because the caster's
    /// canonical designated owner holds no incarnation at its residence.
    pub canonical_incarnation_rejections: usize,
    pub plan: RuntimePlan,
}

pub fn creature_ai_spell_disable_decision_like_cpp(
    spell_id: u32,
    map_id: u16,
    creature: &wow_entities::Creature,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> CreatureSpellDisableDecisionLikeCpp {
    let Some(disable_mgr) = config.disable_mgr.as_deref() else {
        return CreatureSpellDisableDecisionLikeCpp::ContextUnrepresented;
    };
    let instance_type = config
        .map_store
        .as_deref()
        .and_then(|store| store.get(u32::from(map_id)))
        .map(|entry| entry.instance_type);
    let area_id = creature.unit().world().area_id();
    disable_mgr.creature_spell_disable_decision_like_cpp(
        spell_id,
        u32::from(map_id),
        (area_id != 0).then_some(area_id),
        instance_type.map(|kind| kind == wow_data::map::MAP_ARENA),
        instance_type.map(|kind| kind == wow_data::map::MAP_BATTLEGROUND),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureSpellTargetHitResultLikeCpp {
    Hit,
    Miss,
}

pub fn check_no_gray_aggro_config_like_cpp(
    config: &LegacyCreatureAggroConfigLikeCpp,
    player_level: u8,
    player_gray_level: u8,
    creature_level: u8,
) -> bool {
    if creature_level > player_gray_level {
        return false;
    }

    let not_above = config.no_gray_aggro_above;
    let not_below = config.no_gray_aggro_below;
    if not_above == 0 && not_below == 0 {
        return false;
    }

    let player_level = u32::from(player_level);
    player_level <= not_below || (not_above > 0 && player_level >= not_above)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyCreatureAggroVisibilityDecisionLikeCpp {
    Allowed,
    Rejected,
    Unrepresented,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyCreatureAiSelectionDecisionLikeCpp {
    Selected(CreatureAiKindLikeCpp),
    ScriptRegistryUnrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyCreatureAiCanAttackDecisionLikeCpp {
    Allowed,
    Rejected,
    Unrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyCreatureCanAttackLeashDecisionLikeCpp {
    Allowed,
    OwnerPositionUnrepresented,
    HomeRangeRejected,
}

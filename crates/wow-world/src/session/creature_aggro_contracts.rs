// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature aggro contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::creature_ai_spell_difficulty_chain_like_cpp;
use super::{Arc, BTreeSet, ChrRacesStore, ConditionEntriesByTypeStore, CreatureAiKindLikeCpp};
use super::{
    CreatureSpellDisableDecisionLikeCpp, DifficultyStore, DisableMgrLikeCpp, FactionStore,
};
use super::{FactionTemplateStore, MapStore, ObjectGuid, PhaseShift, Position, RuntimePlan};
use super::{SpellAuraRestrictionsStore, SpellCategoryStore, SpellChainStoreLikeCpp};
use super::{SpellCustomAttributeStoreLikeCpp, SpellDurationStore, SpellLinkedStoreLikeCpp};
use super::{SpellLinkedTypeLikeCpp, SpellMiscStore, SpellRangeStore, SpellStore};
use super::{SpellTargetRestrictionsStore, UnitFlags, UnitVisibilityDetectionStateLikeCpp};

#[derive(Debug, Clone)]
pub struct LegacyCreatureMovementTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub movement_packets: usize,
    pub canonical_syncs: usize,
    pub plan: crate::map_manager::RuntimePlan,
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
pub(in crate::session) struct LegacyCreatureAggroOwnerSnapshotLikeCpp {
    pub(in crate::session) map_id: u16,
    pub(in crate::session) instance_id: u32,
    pub(in crate::session) position: Position,
    pub(in crate::session) phase_shift: PhaseShift,
    pub(in crate::session) combat_reach: f32,
    pub(in crate::session) alive: bool,
    pub(in crate::session) in_water: bool,
    pub(in crate::session) in_evade_mode: bool,
    pub(in crate::session) unit_flags: UnitFlags,
    pub(in crate::session) faction_template_id: Option<u32>,
    pub(in crate::session) school_immunity_mask: u32,
    pub(in crate::session) damage_immunity_mask: u32,
    pub(in crate::session) has_confuse_aura: bool,
    pub(in crate::session) has_breakable_stun_aura: bool,
}

pub(in crate::session) const DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP: f32 = 533.0;

/// Prove that one spell cannot enter any process-wide C++ runtime hook that
/// this bounded Rust spell path does not execute.
///
/// Every source is optional because both sessions and the global Creature
/// runtime can be constructed before startup authority is installed. Missing
/// or indeterminate authority must therefore reject the spell rather than
/// treating an empty runtime registry as proof that no DB hook exists.
#[allow(clippy::too_many_arguments)]
pub(in crate::session) fn spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp(
    spell_id: u32,
    exact_spell_ids: Option<&BTreeSet<u32>>,
    all_rank_root_spell_ids: Option<&BTreeSet<u32>>,
    legacy_spell_ids: Option<&BTreeSet<u32>>,
    rejected_linked_trigger_spell_ids: Option<&BTreeSet<u32>>,
    chains: Option<&SpellChainStoreLikeCpp>,
    linked: Option<&SpellLinkedStoreLikeCpp>,
) -> bool {
    let (
        Some(exact_spell_ids),
        Some(all_rank_root_spell_ids),
        Some(legacy_spell_ids),
        Some(rejected_linked_trigger_spell_ids),
        Some(chains),
        Some(linked),
    ) = (
        exact_spell_ids,
        all_rank_root_spell_ids,
        legacy_spell_ids,
        rejected_linked_trigger_spell_ids,
        chains,
        linked,
    )
    else {
        return false;
    };
    if chains
        .indeterminate_diagnostics_for_spell_like_cpp(spell_id)
        .is_some()
    {
        return false;
    }
    let first_rank = chains.first_spell_in_chain_like_cpp(spell_id);
    if exact_spell_ids.contains(&spell_id)
        || all_rank_root_spell_ids.contains(&first_rank)
        || legacy_spell_ids.contains(&spell_id)
        || rejected_linked_trigger_spell_ids.contains(&spell_id)
    {
        return false;
    }

    [
        SpellLinkedTypeLikeCpp::Cast,
        SpellLinkedTypeLikeCpp::Hit,
        SpellLinkedTypeLikeCpp::Aura,
        SpellLinkedTypeLikeCpp::Remove,
    ]
    .into_iter()
    .all(|kind| linked.get_spell_linked_like_cpp(kind, spell_id).is_none())
}

/// Map-owned creature aggro fidelity switches derived from C++ world configs.
///
/// C++ anchor: `Creature::CheckNoGrayAggroConfig` reads
/// `CONFIG_NO_GRAY_AGGRO_ABOVE` and `CONFIG_NO_GRAY_AGGRO_BELOW` after
/// `Trinity::XP::GetColorCode(playerLevel, creatureLevel) == XP_GRAY`.
#[derive(Clone)]
pub struct LegacyCreatureAggroConfigLikeCpp {
    pub no_gray_aggro_above: u32,
    pub no_gray_aggro_below: u32,
    pub creature_aggro_rate: f32,
    pub max_player_level_config: u32,
    pub faction_template_store: Option<Arc<FactionTemplateStore>>,
    pub faction_store: Option<Arc<FactionStore>>,
    pub map_store: Option<Arc<MapStore>>,
    pub disable_mgr: Option<Arc<DisableMgrLikeCpp>>,
    pub spell_misc_store: Option<Arc<SpellMiscStore>>,
    pub spell_range_store: Option<Arc<SpellRangeStore>>,
    pub spell_duration_store: Option<Arc<SpellDurationStore>>,
    pub spell_cooldowns_store: Option<Arc<wow_data::SpellCooldownsStore>>,
    pub spell_category_store: Option<Arc<SpellCategoryStore>>,
    pub spell_x_spell_visual_store: Option<Arc<wow_data::SpellXSpellVisualStore>>,
    pub spell_target_restrictions_store: Option<Arc<SpellTargetRestrictionsStore>>,
    pub spell_casting_requirements_store: Option<Arc<wow_data::SpellCastingRequirementsStore>>,
    pub spell_aura_restrictions_store: Option<Arc<SpellAuraRestrictionsStore>>,
    pub spell_store: Option<Arc<SpellStore>>,
    pub spell_threat_store: Option<Arc<wow_data::SpellThreatStoreLikeCpp>>,
    pub spell_chain_store: Option<Arc<SpellChainStoreLikeCpp>>,
    pub spell_linked_store: Option<Arc<SpellLinkedStoreLikeCpp>>,
    pub spell_condition_store: Option<Arc<ConditionEntriesByTypeStore>>,
    pub spell_script_exact_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub spell_script_all_rank_root_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub legacy_spell_script_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub spell_linked_rejected_trigger_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub spell_custom_attribute_store: Option<Arc<SpellCustomAttributeStoreLikeCpp>>,
    pub difficulty_store: Option<Arc<DifficultyStore>>,
    /// C++ `sDB2Manager`'s `ExpectedStat` table (`Player::GetBlockPercent`).
    pub expected_stat_store: Option<Arc<wow_data::ExpectedStatStore>>,
    /// C++ `sObjectMgr->GetCreatureTemplate` subset the map-owned runtime needs
    /// to resolve a victim's `GetCreatureTypeMask` for `MeleeDamageBonusDone`.
    pub creature_template_lifecycle_store:
        Option<Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>>,
    /// C++ `sChrRacesStore` source for a player victim's `GetCreatureTypeMask`
    /// in the creature-attacker melee path.
    pub chr_races_store: Option<Arc<ChrRacesStore>>,
    pub visibility_distance_continents: f32,
    pub visibility_distance_instances: f32,
    pub visibility_distance_battlegrounds: f32,
    pub visibility_distance_arenas: f32,
    pub family_assistance_radius: f32,
    pub family_assistance_delay_ms: u32,
}

impl Default for LegacyCreatureAggroConfigLikeCpp {
    fn default() -> Self {
        Self {
            no_gray_aggro_above: 0,
            no_gray_aggro_below: 0,
            creature_aggro_rate: 1.0,
            max_player_level_config: 80,
            faction_template_store: None,
            faction_store: None,
            map_store: None,
            disable_mgr: Some(Arc::new(DisableMgrLikeCpp::default())),
            spell_misc_store: None,
            spell_range_store: None,
            spell_duration_store: None,
            spell_cooldowns_store: None,
            spell_category_store: None,
            spell_x_spell_visual_store: None,
            spell_target_restrictions_store: None,
            spell_casting_requirements_store: None,
            spell_aura_restrictions_store: None,
            spell_store: None,
            spell_threat_store: None,
            spell_chain_store: None,
            spell_linked_store: None,
            spell_condition_store: None,
            spell_script_exact_spell_ids_like_cpp: None,
            spell_script_all_rank_root_spell_ids_like_cpp: None,
            legacy_spell_script_spell_ids_like_cpp: None,
            spell_linked_rejected_trigger_spell_ids_like_cpp: None,
            spell_custom_attribute_store: None,
            difficulty_store: None,
            expected_stat_store: None,
            creature_template_lifecycle_store: None,
            chr_races_store: None,
            visibility_distance_continents: wow_entities::DEFAULT_VISIBILITY_DISTANCE,
            visibility_distance_instances: wow_entities::DEFAULT_VISIBILITY_INSTANCE,
            visibility_distance_battlegrounds: DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP,
            visibility_distance_arenas: DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP,
            family_assistance_radius: 10.0,
            family_assistance_delay_ms: 1_500,
        }
    }
}

impl LegacyCreatureAggroConfigLikeCpp {
    pub(in crate::session) fn spell_has_no_unrepresented_runtime_hooks_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let Some(conditions) = self.spell_condition_store.as_deref() else {
            return false;
        };
        let Ok(signed_spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        if conditions
            .conditions_for_like_cpp(
                wow_constants::ConditionSourceType::Spell,
                wow_data::conditions::ConditionId::new(0, signed_spell_id, 0),
            )
            .is_some()
        {
            // C++ Spell::CheckCast evaluates SourceType 17 before explicit
            // target validation. M2.6 cannot evaluate those predicates.
            return false;
        }
        spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp(
            spell_id,
            self.spell_script_exact_spell_ids_like_cpp.as_deref(),
            self.spell_script_all_rank_root_spell_ids_like_cpp
                .as_deref(),
            self.legacy_spell_script_spell_ids_like_cpp.as_deref(),
            self.spell_linked_rejected_trigger_spell_ids_like_cpp
                .as_deref(),
            self.spell_chain_store.as_deref(),
            self.spell_linked_store.as_deref(),
        )
    }

    /// Prove that C++ `Spell::CheckCast` has no caster-facing requirement
    /// that this bounded creature publication path would otherwise skip.
    ///
    /// Startup installs the effective DB2 + SQL + hotfix authority. Missing
    /// authority and spell IDs outside DB2's signed key domain fail closed;
    /// an absent effective row means the spell has no such requirement.
    pub(in crate::session) fn spell_has_no_unrepresented_casting_requirements_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let Some(store) = self.spell_casting_requirements_store.as_deref() else {
            return false;
        };
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        match store.entry_for_spell_id_like_cpp(spell_id) {
            Some(requirement) => {
                requirement.facing_caster_flags == 0
                    && requirement.required_areas_id == 0
                    && requirement.requires_spell_focus == 0
            }
            None => true,
        }
    }

    /// This bounded creature-cast slice does not yet own the complete
    /// shapeshift-form authority needed by `SpellInfo::CheckShapeshift`.
    /// A neutral mask is provably safe; every non-neutral mask fails closed.
    pub(in crate::session) fn spell_has_no_unrepresented_shapeshift_requirements_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let Some(store) = self.spell_store.as_deref() else {
            return false;
        };
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        let (stances, stances_not) = store.shapeshift_masks_like_cpp(spell_id);
        stances == 0 && stances_not == 0
    }

    /// C++ applies the effective `SpellAuraRestrictions` row during
    /// `Spell::CheckCast`/`SpellInfo::CheckExplicitTarget`. This bounded path
    /// does not yet evaluate aura states or required/excluded aura spells, so
    /// only an absent or entirely neutral effective row is safe to publish.
    pub(in crate::session) fn spell_has_no_unrepresented_aura_restrictions_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> bool {
        let Some(store) = self.spell_aura_restrictions_store.as_deref() else {
            return false;
        };
        let Some(restriction) = store.resolved_for_difficulty_chain_like_cpp(
            spell_id,
            creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, self)
                .into_iter()
                .map(u32::from),
        ) else {
            return true;
        };

        restriction.caster_aura_state == 0
            && restriction.target_aura_state == 0
            && restriction.exclude_caster_aura_state == 0
            && restriction.exclude_target_aura_state == 0
            && restriction.caster_aura_spell == 0
            && restriction.target_aura_spell == 0
            && restriction.exclude_caster_aura_spell == 0
            && restriction.exclude_target_aura_spell == 0
    }

    pub(in crate::session) fn creature_faction_template_is_neutral_to_all_like_cpp(
        &self,
        faction_template_id: u32,
    ) -> bool {
        let Some(faction_template_store) = self.faction_template_store.as_ref() else {
            return faction_template_id == 35;
        };
        let Some(faction_template) = faction_template_store.get(faction_template_id) else {
            return false;
        };

        if faction_template.faction == 0 {
            return true;
        }

        if let Some(faction_store) = self.faction_store.as_ref()
            && let Some(raw_faction) = faction_store.get(u32::from(faction_template.faction))
            && raw_faction.can_have_reputation_like_cpp()
        {
            return false;
        }

        faction_template.is_neutral_to_all_like_cpp()
    }

    pub(in crate::session) fn map_is_dungeon_like_cpp(&self, map_id: u16) -> bool {
        self.map_store
            .as_ref()
            .and_then(|store| store.get(u32::from(map_id)))
            .is_some_and(|entry| entry.is_dungeon())
    }

    pub(in crate::session) fn map_visibility_range_like_cpp(&self, map_id: u16) -> f32 {
        let Some(entry) = self
            .map_store
            .as_ref()
            .and_then(|store| store.get(u32::from(map_id)))
        else {
            return self.visibility_distance_continents;
        };

        match entry.instance_type {
            wow_data::map::MAP_ARENA => self.visibility_distance_arenas,
            wow_data::map::MAP_BATTLEGROUND => self.visibility_distance_battlegrounds,
            wow_data::map::MAP_INSTANCE | wow_data::map::MAP_RAID | wow_data::map::MAP_SCENARIO
                if !entry.is_garrison() =>
            {
                self.visibility_distance_instances
            }
            _ => self.visibility_distance_continents,
        }
    }
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
    pub movement_interrupts: usize,
    pub victim_switches: usize,
    pub evades_started: usize,
    pub assistance_scheduled: usize,
    pub assistance_starts: usize,
    pub plan: crate::map_manager::RuntimePlan,
    pub aggro_starts: usize,
    pub commands: Vec<crate::session::mailbox::CreatureAttackStartLikeCppCommand>,
    pub stop_commands: Vec<crate::session::mailbox::CreatureAttackStopLikeCppCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) enum LegacyCreatureThreatUpdateLikeCpp {
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
    pub commands: Vec<crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand>,
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
    pub plan: RuntimePlan,
}

pub(in crate::session) fn creature_ai_spell_disable_decision_like_cpp(
    spell_id: u32,
    map_id: u16,
    creature: &crate::map_manager::WorldCreature,
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
    let area_id = creature.creature.unit().world().area_id();
    disable_mgr.creature_spell_disable_decision_like_cpp(
        spell_id,
        u32::from(map_id),
        (area_id != 0).then_some(area_id),
        instance_type.map(|kind| kind == wow_data::map::MAP_ARENA),
        instance_type.map(|kind| kind == wow_data::map::MAP_BATTLEGROUND),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum CreatureSpellTargetHitResultLikeCpp {
    Hit,
    Miss,
}

pub(in crate::session) fn check_no_gray_aggro_config_like_cpp(
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
pub(in crate::session) enum LegacyCreatureAggroVisibilityDecisionLikeCpp {
    Allowed,
    Rejected,
    Unrepresented,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) enum LegacyCreatureAiSelectionDecisionLikeCpp {
    Selected(CreatureAiKindLikeCpp),
    ScriptRegistryUnrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum LegacyCreatureAiCanAttackDecisionLikeCpp {
    Allowed,
    Rejected,
    Unrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum LegacyCreatureCanAttackLeashDecisionLikeCpp {
    Allowed,
    OwnerPositionUnrepresented,
    HomeRangeRejected,
}

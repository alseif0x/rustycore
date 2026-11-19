// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Effective spell-acquisition bootstrap.
//!
//! This keeps the dependency-sensitive part of `SpellMgr` startup together:
//! the compact effective metadata catalog, SQL custom attributes, rank chains,
//! learn-skill nodes, and learn-spell edges. The order mirrors
//! `World.cpp::SetInitialWorldSettings` and the projections mirror
//! `SpellMgr.cpp::LoadSpellRanks`, `LoadSpellLearnSkills`, and
//! `LoadSpellLearnSpells`.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

use anyhow::{Context, Result};
use tracing::info;
use wow_data::spell::spell_effect_types::{
    SPELL_EFFECT_DUAL_WIELD, SPELL_EFFECT_LEARN_SPELL, SPELL_EFFECT_SKILL, SPELL_EFFECT_SKILL_STEP,
};
use wow_data::spell_acquisition::{
    SpellAcquisitionEffectLikeCpp, SpellAcquisitionMetadataLookupLikeCpp,
    SpellAcquisitionResolvedEffectsLookupLikeCpp, SpellAcquisitionResolvedMetadataLookupLikeCpp,
    SpellAcquisitionSqlOverlayFutureLikeCpp, SpellAcquisitionSqlOverlayRowLikeCpp,
    SpellAcquisitionSqlOverlaySourceLikeCpp, SpellAcquisitionTableLikeCpp,
};
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, DifficultyStore, SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
    ServersideSpellStoreLikeCpp, SkillStore, SpellAcquisitionCatalogLikeCpp,
    SpellAcquisitionCoverageSeedLikeCpp, SpellAcquisitionDiagnosticSeverityLikeCpp,
    SpellAcquisitionIndeterminateReasonLikeCpp, SpellAcquisitionTalentLookupLikeCpp,
    SpellAreaStoreLikeCpp, SpellAuraRestrictionsEntry, SpellAuraRestrictionsStore,
    SpellCastCostStoreLikeCpp, SpellCastingRequirementsEntry, SpellCastingRequirementsStore,
    SpellChainStoreLikeCpp, SpellCustomAttributeKeyLikeCpp,
    SpellCustomAttributeLoadErrorKindLikeCpp, SpellCustomAttributeSourceVariantLikeCpp,
    SpellCustomAttributeStoreLikeCpp, SpellEquippedItemsEntry, SpellEquippedItemsStore,
    SpellLearnSkillEffectLikeCpp, SpellLearnSkillIndeterminateReasonLikeCpp,
    SpellLearnSkillSourceSpellInfoLikeCpp, SpellLearnSkillStoreLikeCpp,
    SpellLearnSourceSpellInfoLikeCpp, SpellLearnSpellEffectLikeCpp, SpellLearnSpellEntry,
    SpellLearnSpellStoreLikeCpp, SpellLinkedStoreLikeCpp, SpellLinkedTypeLikeCpp,
    SpellPetAuraStoreLikeCpp, SpellReagentsCurrencyEntry, SpellReagentsCurrencyStore,
    SpellReagentsEntry, SpellReagentsStore, SpellStore, wdc4::Wdc4Reader,
};
use wow_persistence::{
    SpellAcquisitionHotfixPersistenceRowLikeCpp, SpellAcquisitionHotfixTablePersistenceLikeCpp,
    SpellAcquisitionStartupLoadOutcomeLikeCpp, SpellAcquisitionStartupPersistencePortLikeCpp,
    SpellReagentsCurrencyPersistenceRowLikeCpp,
};

struct SpellAcquisitionSqlOverlaySourceBridgeLikeCpp<'a> {
    persistence: &'a dyn SpellAcquisitionStartupPersistencePortLikeCpp,
}

fn persistence_table_like_cpp(
    table: SpellAcquisitionTableLikeCpp,
) -> SpellAcquisitionHotfixTablePersistenceLikeCpp {
    match table {
        SpellAcquisitionTableLikeCpp::SpellEffect => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellEffect
        }
        SpellAcquisitionTableLikeCpp::SpellLearnSpell => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLearnSpell
        }
        SpellAcquisitionTableLikeCpp::SpellMisc => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellMisc
        }
        SpellAcquisitionTableLikeCpp::SpellLevels => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SpellLevels
        }
        SpellAcquisitionTableLikeCpp::Talent => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::Talent
        }
        SpellAcquisitionTableLikeCpp::SummonProperties => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::SummonProperties
        }
        SpellAcquisitionTableLikeCpp::BattlePetSpecies => {
            SpellAcquisitionHotfixTablePersistenceLikeCpp::BattlePetSpecies
        }
    }
}

fn overlay_row_like_cpp(
    table: SpellAcquisitionTableLikeCpp,
    row: SpellAcquisitionHotfixPersistenceRowLikeCpp,
) -> Result<SpellAcquisitionSqlOverlayRowLikeCpp> {
    let (integer_columns, float_columns_bits) = match (table, row) {
        (
            SpellAcquisitionTableLikeCpp::SpellEffect,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellEffect(row),
        ) => {
            let integers = vec![
                row.record_id,
                row.difficulty_id,
                row.effect_index,
                row.effect,
                row.effect_base_points,
                row.effect_die_sides,
                row.effect_trigger_spell,
                row.effect_misc_value[0],
                row.effect_misc_value[1],
                row.implicit_target[0],
                row.implicit_target[1],
                None,
                None,
                row.spell_id,
                row.effect_chain_targets,
                None,
                None,
                row.effect_item_type,
                row.effect_aura,
                row.effect_mechanic,
                row.effect_attributes,
            ];
            let mut floats = vec![None; integers.len()];
            floats[11] = row.coefficient_bits;
            floats[12] = row.variance_bits;
            floats[15] = row.effect_points_per_resource_bits;
            floats[16] = row.effect_real_points_per_level_bits;
            (integers, floats)
        }
        (
            SpellAcquisitionTableLikeCpp::SpellLearnSpell,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellLearnSpell(row),
        ) => (
            vec![
                row.record_id,
                row.spell_id,
                row.learn_spell_id,
                row.overrides_spell_id,
            ],
            vec![None; 4],
        ),
        (
            SpellAcquisitionTableLikeCpp::SpellMisc,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellMisc(row),
        ) => (
            vec![
                row.record_id,
                row.attributes[0],
                row.attributes[1],
                row.difficulty_id,
                row.show_future_spell_player_condition_id,
                row.spell_id,
            ],
            vec![None; 6],
        ),
        (
            SpellAcquisitionTableLikeCpp::SpellLevels,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellLevels(row),
        ) => (
            vec![
                row.record_id,
                row.difficulty_id,
                row.base_level,
                row.spell_level,
                row.spell_id,
            ],
            vec![None; 5],
        ),
        (
            SpellAcquisitionTableLikeCpp::Talent,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::Talent(row),
        ) => {
            let mut integers = Vec::with_capacity(10);
            integers.push(row.record_id);
            integers.extend(row.spell_rank);
            (integers, vec![None; 10])
        }
        (
            SpellAcquisitionTableLikeCpp::SummonProperties,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SummonProperties(row),
        ) => (vec![row.record_id, row.slot, row.flags_1], vec![None; 3]),
        (
            SpellAcquisitionTableLikeCpp::BattlePetSpecies,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::BattlePetSpecies(row),
        ) => (vec![row.record_id, row.creature_id], vec![None; 2]),
        _ => {
            return Err(anyhow::anyhow!(
                "spell-acquisition overlay returned the wrong typed row family"
            ));
        }
    };
    Ok(SpellAcquisitionSqlOverlayRowLikeCpp {
        integer_columns,
        float_columns_bits,
    })
}

impl SpellAcquisitionSqlOverlaySourceLikeCpp for SpellAcquisitionSqlOverlaySourceBridgeLikeCpp<'_> {
    fn load_overlay_like_cpp(
        &self,
        table: SpellAcquisitionTableLikeCpp,
        official: bool,
    ) -> SpellAcquisitionSqlOverlayFutureLikeCpp<'_> {
        Box::pin(async move {
            match self
                .persistence
                .load_hotfix_overlay_like_cpp(persistence_table_like_cpp(table), official)
                .await
            {
                SpellAcquisitionStartupLoadOutcomeLikeCpp::Loaded(rows) => rows
                    .into_iter()
                    .map(|row| overlay_row_like_cpp(table, row))
                    .collect(),
                SpellAcquisitionStartupLoadOutcomeLikeCpp::Failed { reason } => {
                    Err(anyhow::anyhow!(reason))
                }
            }
        })
    }
}

fn loaded_startup_like_cpp<T>(outcome: SpellAcquisitionStartupLoadOutcomeLikeCpp<T>) -> Result<T> {
    match outcome {
        SpellAcquisitionStartupLoadOutcomeLikeCpp::Loaded(value) => Ok(value),
        SpellAcquisitionStartupLoadOutcomeLikeCpp::Failed { reason } => {
            Err(anyhow::anyhow!(reason))
        }
    }
}

pub(crate) async fn load_serverside_spell_effects_like_cpp<
    RegularSpellExists,
    DifficultyExists,
    RadiusExists,
>(
    persistence: &dyn SpellAcquisitionStartupPersistencePortLikeCpp,
    regular_spell_exists: RegularSpellExists,
    difficulty_exists: DifficultyExists,
    radius_exists: RadiusExists,
) -> Result<wow_data::ServersideSpellEffectLoadOutcomeLikeCpp>
where
    RegularSpellExists: FnMut(u32) -> bool,
    DifficultyExists: FnMut(u32) -> bool,
    RadiusExists: FnMut(u32) -> bool,
{
    let rows = loaded_startup_like_cpp(persistence.load_serverside_spell_effects_like_cpp().await)?
        .into_iter()
        .map(|row| wow_data::ServersideSpellEffectRowLikeCpp {
            spell_id: row.spell_id,
            effect_index: row.effect_index,
            difficulty_id: row.difficulty_id,
            effect: row.effect,
            effect_aura: row.effect_aura,
            effect_amplitude: row.effect_amplitude,
            effect_attributes: row.effect_attributes,
            effect_aura_period: row.effect_aura_period,
            effect_bonus_coefficient: row.effect_bonus_coefficient,
            effect_chain_amplitude: row.effect_chain_amplitude,
            effect_chain_targets: row.effect_chain_targets,
            effect_item_type: row.effect_item_type,
            effect_mechanic: row.effect_mechanic,
            effect_points_per_resource: row.effect_points_per_resource,
            effect_pos_facing: row.effect_pos_facing,
            effect_real_points_per_level: row.effect_real_points_per_level,
            effect_trigger_spell: row.effect_trigger_spell,
            bonus_coefficient_from_ap: row.bonus_coefficient_from_ap,
            pvp_multiplier: row.pvp_multiplier,
            coefficient: row.coefficient,
            variance: row.variance,
            resource_coefficient: row.resource_coefficient,
            group_size_base_points_coefficient: row.group_size_base_points_coefficient,
            effect_base_points: row.effect_base_points,
            effect_misc_value_1: row.effect_misc_value[0],
            effect_misc_value_2: row.effect_misc_value[1],
            effect_radius_index_1: row.effect_radius_index[0],
            effect_radius_index_2: row.effect_radius_index[1],
            effect_spell_class_mask: row.effect_spell_class_mask,
            implicit_target_1: row.implicit_target[0],
            implicit_target_2: row.implicit_target[1],
        });
    Ok(
        wow_data::ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
            rows,
            regular_spell_exists,
            difficulty_exists,
            radius_exists,
        ),
    )
}

pub(crate) async fn load_serverside_spells_like_cpp<RegularSpellExists>(
    persistence: &dyn SpellAcquisitionStartupPersistencePortLikeCpp,
    effects: &wow_data::ServersideSpellEffectStoreLikeCpp,
    regular_spell_exists: RegularSpellExists,
) -> Result<wow_data::ServersideSpellLoadOutcomeLikeCpp>
where
    RegularSpellExists: FnMut(u32) -> bool,
{
    let rows = loaded_startup_like_cpp(persistence.load_serverside_spells_like_cpp().await)?
        .into_iter()
        .map(|row| wow_data::ServersideSpellRowLikeCpp {
            spell_id: row.spell_id,
            difficulty_id: row.difficulty_id,
            category_id: row.category_id,
            dispel: row.dispel,
            mechanic: row.mechanic,
            attributes: row.attributes,
            attributes_ex: row.attributes_ex,
            stances: row.stances,
            stances_not: row.stances_not,
            targets: row.targets,
            target_creature_type: row.target_creature_type,
            requires_spell_focus: row.requires_spell_focus,
            facing_caster_flags: row.facing_caster_flags,
            caster_aura_state: row.caster_aura_state,
            target_aura_state: row.target_aura_state,
            exclude_caster_aura_state: row.exclude_caster_aura_state,
            exclude_target_aura_state: row.exclude_target_aura_state,
            caster_aura_spell: row.caster_aura_spell,
            target_aura_spell: row.target_aura_spell,
            exclude_caster_aura_spell: row.exclude_caster_aura_spell,
            exclude_target_aura_spell: row.exclude_target_aura_spell,
            caster_aura_type: row.caster_aura_type,
            target_aura_type: row.target_aura_type,
            exclude_caster_aura_type: row.exclude_caster_aura_type,
            exclude_target_aura_type: row.exclude_target_aura_type,
            casting_time_index: row.casting_time_index,
            recovery_time: row.recovery_time,
            category_recovery_time: row.category_recovery_time,
            start_recovery_category: row.start_recovery_category,
            start_recovery_time: row.start_recovery_time,
            interrupt_flags: row.interrupt_flags,
            aura_interrupt_flags: row.aura_interrupt_flags,
            channel_interrupt_flags: row.channel_interrupt_flags,
            proc_flags: row.proc_flags,
            proc_chance: row.proc_chance,
            proc_charges: row.proc_charges,
            proc_cooldown: row.proc_cooldown,
            proc_base_ppm: row.proc_base_ppm,
            max_level: row.max_level,
            base_level: row.base_level,
            spell_level: row.spell_level,
            duration_index: row.duration_index,
            range_index: row.range_index,
            speed: row.speed,
            launch_delay: row.launch_delay,
            stack_amount: row.stack_amount,
            equipped_item_class: row.equipped_item_class,
            equipped_item_sub_class_mask: row.equipped_item_sub_class_mask,
            equipped_item_inventory_type_mask: row.equipped_item_inventory_type_mask,
            content_tuning_id: row.content_tuning_id,
            spell_name: row.spell_name,
            cone_angle: row.cone_angle,
            cone_width: row.cone_width,
            max_target_level: row.max_target_level,
            max_affected_targets: row.max_affected_targets,
            spell_family_name: row.spell_family_name,
            spell_family_flags: row.spell_family_flags,
            dmg_class: row.dmg_class,
            prevention_type: row.prevention_type,
            area_group_id: row.area_group_id,
            school_mask: row.school_mask,
            charge_category_id: row.charge_category_id,
        });
    Ok(wow_data::ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        rows,
        effects,
        regular_spell_exists,
    ))
}

#[derive(Debug)]
pub(crate) struct SpellAcquisitionBootstrapLikeCpp {
    pub(crate) catalog: Arc<SpellAcquisitionCatalogLikeCpp>,
    pub(crate) chain_store: Arc<SpellChainStoreLikeCpp>,
    pub(crate) learn_skill_store: Arc<SpellLearnSkillStoreLikeCpp>,
    pub(crate) learn_spell_store: Arc<SpellLearnSpellStoreLikeCpp>,
    pub(crate) custom_attribute_store: Arc<SpellCustomAttributeStoreLikeCpp>,
}

#[derive(Debug)]
pub(crate) struct TrainerSpellStaticAuthorityLikeCpp {
    pub(crate) safe_cast_spell_ids: BTreeSet<u32>,
    pub(crate) valid_craft_spell_ids: BTreeSet<u32>,
    /// Exact positive `spell_script_names.spell_id` bindings.
    ///
    /// These process-wide sets are also consumed by the spell-hit aura
    /// authority. Keeping the raw exact/all-ranks distinction lets each
    /// session resolve the final rank through the same effective
    /// `SpellChainStoreLikeCpp` instead of treating an empty runtime script
    /// registry as source proof.
    pub(crate) spell_script_exact_spell_ids: BTreeSet<u32>,
    /// Absolute roots from negative `spell_script_names.spell_id` rows.
    pub(crate) spell_script_all_rank_root_spell_ids: BTreeSet<u32>,
    /// C++ `spell_scripts.id & 0x00FF_FFFF` bindings.
    pub(crate) legacy_spell_script_spell_ids: BTreeSet<u32>,
    pub(crate) cast_cost_store: SpellCastCostStoreLikeCpp,
}

#[derive(Debug, Clone, Copy, Default)]
struct TrainerCastWorldHookAuditLikeCpp {
    script_binding: bool,
    legacy_script: bool,
    condition: bool,
    aura_restriction: bool,
    equipped_item_restriction: bool,
    channeled: bool,
    spell_focus_requirement: bool,
    required_area_requirement: bool,
    spell_area_requirement: bool,
    linked_spell: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TrainerSpellScriptBindingsLikeCpp {
    exact_spell_ids: BTreeSet<u32>,
    all_rank_root_spell_ids: BTreeSet<u32>,
}

impl TrainerSpellScriptBindingsLikeCpp {
    fn contains_like_cpp(&self, spell_id: u32, first_rank_spell_id: u32) -> bool {
        self.exact_spell_ids.contains(&spell_id)
            || self.all_rank_root_spell_ids.contains(&first_rank_spell_id)
    }
}

const SPELL_EFFECT_CREATE_ITEM_LIKE_CPP: u32 = 24;
const SPELL_EFFECT_CREATE_RANDOM_ITEM_LIKE_CPP: u32 = 59;
const SPELL_EFFECT_CREATE_LOOT_LIKE_CPP: u32 = 157;

fn trainer_cast_world_hooks_are_static_safe_like_cpp(
    audit: TrainerCastWorldHookAuditLikeCpp,
) -> bool {
    !(audit.script_binding
        || audit.legacy_script
        || audit.condition
        || audit.aura_restriction
        || audit.equipped_item_restriction
        || audit.channeled
        || audit.spell_focus_requirement
        || audit.required_area_requirement
        || audit.spell_area_requirement
        || audit.linked_spell)
}

fn trainer_cast_effective_casting_requirement_audit_like_cpp(
    spell_info_present: bool,
    requirements: Option<&SpellCastingRequirementsEntry>,
) -> (bool, bool) {
    (
        !spell_info_present
            || requirements.is_some_and(|requirements| requirements.requires_spell_focus != 0),
        requirements.is_some_and(|requirements| requirements.required_areas_id != 0),
    )
}

fn trainer_cast_has_unsupported_effective_aura_state_restriction_like_cpp(
    entry: &SpellAuraRestrictionsEntry,
) -> bool {
    entry.caster_aura_state != 0
        || entry.target_aura_state != 0
        || entry.exclude_caster_aura_state != 0
        || entry.exclude_target_aura_state != 0
}

fn trainer_cast_has_unsupported_difficulty_none_aura_state_restriction_like_cpp(
    store: &SpellAuraRestrictionsStore,
    spell_id: u32,
) -> bool {
    // C++ attaches SpellAuraRestrictions to the exact `(SpellID,
    // Difficulty)` SpellInfo key. Preserve Rust's represented wildcard only
    // when no exact DIFFICULTY_NONE row exists; rows for other difficulties
    // must not contaminate a normal trainer cast.
    let has_exact = store
        .entries_for_spell_id_like_cpp(spell_id)
        .any(|entry| entry.difficulty_id == 0);
    let selected_difficulty = if has_exact { 0_u32 } else { u32::from(u8::MAX) };
    store
        .resolved_for_difficulty_chain_like_cpp(spell_id, [selected_difficulty])
        .is_some_and(trainer_cast_has_unsupported_effective_aura_state_restriction_like_cpp)
}

fn trainer_cast_has_effective_equipped_item_restriction_like_cpp(
    entry: &SpellEquippedItemsEntry,
) -> bool {
    // C++ `SpellInfo::IsItemFitToSpellRequirements` treats class -1 as item
    // neutral before consulting either mask.
    entry.equipped_item_class != -1
}

fn trainer_cast_effects_are_static_safe_like_cpp(
    spell_id: u32,
    effects: &[SpellAcquisitionEffectLikeCpp],
    mut has_pet_aura: impl FnMut(u8) -> bool,
) -> bool {
    let mut has_acquisition_effect = false;
    for effect in effects {
        let (Ok(effect_type), Ok(effect_index)) =
            (effect.effect_type_checked(), effect.effect_index_checked())
        else {
            return false;
        };
        if has_pet_aura(effect_index) {
            return false;
        }
        match effect_type {
            SPELL_EFFECT_LEARN_SPELL
            | SPELL_EFFECT_SKILL
            | SPELL_EFFECT_SKILL_STEP
            | SPELL_EFFECT_DUAL_WIELD => {
                has_acquisition_effect = true;
                // This bounded runtime does not yet own C++'s mechanic/state
                // immunity containers. Keep accepted trainer effects neutral
                // in both dimensions so active immunity auras can be compared
                // exactly rather than guessed at purchase time.
                if effect.effect_mechanic_raw != 0 || effect.effect_aura_raw != 0 {
                    return false;
                }
                if effect_type != SPELL_EFFECT_SKILL && !effect.targets_player_like_cpp() {
                    return false;
                }
            }
            0 => {}
            // The effective 3.4.3 audit found these two inert DUMMY
            // effects in the otherwise deterministic riding closure.
            3 if matches!(spell_id, 33_388 | 34_090) => {}
            other if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(other) => {}
            _ => return false,
        }
    }
    has_acquisition_effect
}

/// Build the immutable, process-wide half of trainer wrapper authority.
///
/// This deliberately proves only the narrow acquisition-effect closure. Every
/// world-table hook that can alter a cast is queried from the final DB, and
/// every unsupported effective effect/target remains absent from the result.
/// The difficulty-specific target restriction and per-player immunity/effect
/// mask are resolved separately by `wow-world` immediately before the atomic
/// trainer commit. Spell disables deliberately remain in that dynamic half:
/// C++ `DisableMgr::IsDisabledFor` evaluates caster type, map, area, arena,
/// and battleground state, so excluding every spell that merely has a
/// `disables` row here would reject casts that C++ permits outside that row's
/// scope. Missing runtime disable authority fails closed in the same adapter.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_trainer_static_authority_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellAcquisitionStartupPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    spell_store: &SpellStore,
    spell_chains: &SpellChainStoreLikeCpp,
    catalog: &SpellAcquisitionCatalogLikeCpp,
    linked: &SpellLinkedStoreLikeCpp,
    pet_auras: &SpellPetAuraStoreLikeCpp,
    aura_restrictions: &SpellAuraRestrictionsStore,
    casting_requirements: &SpellCastingRequirementsStore,
    equipped_items: &SpellEquippedItemsStore,
    spell_areas: &SpellAreaStoreLikeCpp,
    item_exists: impl Fn(u32) -> bool,
) -> Result<TrainerSpellStaticAuthorityLikeCpp> {
    let audit = loaded_startup_like_cpp(persistence.load_trainer_spell_audit_like_cpp().await)
        .context("Failed to audit trainer spell World hooks")?;
    let mut script_bindings = TrainerSpellScriptBindingsLikeCpp::default();
    for id in audit.script_binding_ids {
        if id > 0 {
            script_bindings.exact_spell_ids.insert(id as u32);
        } else if let Some(id) = id.checked_abs().and_then(|id| u32::try_from(id).ok())
            && id != 0
        {
            script_bindings.all_rank_root_spell_ids.insert(id);
        }
    }
    let legacy_scripts = audit.legacy_script_ids.into_iter().collect::<BTreeSet<_>>();
    let conditions = audit
        .condition_spell_ids
        .into_iter()
        .filter_map(|id| id.checked_abs().and_then(|id| u32::try_from(id).ok()))
        .collect::<BTreeSet<_>>();
    let cast_cost_store =
        load_effective_spell_cast_costs_like_cpp(data_dir, locale, persistence, removals)
            .await
            .context("Failed to compose effective spell reagent and currency costs")?;
    let trainer_reagents = cast_cost_store.trainer_reagent_ids_like_cpp();

    let mut safe_cast_spell_ids = BTreeSet::new();
    for (spell_id, difficulty_id) in spell_store.spell_info_keys_in_order_like_cpp() {
        if difficulty_id != 0 {
            continue;
        }
        let spell_id_i32 = i32::try_from(spell_id).ok();
        let effective_casting_requirements = spell_id_i32
            .and_then(|spell_id| casting_requirements.entry_for_spell_id_like_cpp(spell_id));
        let (spell_focus_requirement, required_area_requirement) =
            trainer_cast_effective_casting_requirement_audit_like_cpp(
                spell_id_i32
                    .and_then(|spell_id| spell_store.get(spell_id))
                    .is_some(),
                effective_casting_requirements,
            );
        // Shapeshift metadata is intentionally absent from this audit. C++
        // `Trainer::TeachSpell` invokes `CastSpell(..., true)`; the bool
        // constructor selects `TRIGGERED_FULL_MASK`, which contains
        // `TRIGGERED_IGNORE_SHAPESHIFT`, so `Spell::CheckCast` does not call
        // `SpellInfo::CheckShapeshift` for the trainer wrapper. Rejecting
        // stance-gated wrappers here would therefore be stricter than C++.
        let hook_audit = TrainerCastWorldHookAuditLikeCpp {
            script_binding: script_bindings.contains_like_cpp(
                spell_id,
                spell_chains.first_spell_in_chain_like_cpp(spell_id),
            ),
            legacy_script: legacy_scripts.contains(&spell_id),
            condition: conditions.contains(&spell_id),
            // Aura-spell presence/exclusion gates are resolved from the
            // complete player aura authority at purchase time. Unit
            // AuraState remains outside the reduced projection and is the
            // only SpellAuraRestrictions shape that blocks static authority.
            aura_restriction:
                trainer_cast_has_unsupported_difficulty_none_aura_state_restriction_like_cpp(
                    aura_restrictions,
                    spell_id,
                ),
            equipped_item_restriction: equipped_items
                .entry_for_spell_id_like_cpp(i32::try_from(spell_id).unwrap_or(i32::MAX))
                .is_some_and(trainer_cast_has_effective_equipped_item_restriction_like_cpp),
            // The reduced trainer projection does not own channel state or
            // periodic channel updates. Audit the final effective SpellMisc
            // attributes, including official/custom hotfix overlays.
            channeled: spell_id_i32.is_none_or(|spell_id| {
                spell_store.get(spell_id).is_none() || spell_store.is_channeled_like_cpp(spell_id)
            }) || match catalog
                .misc_for_spell_like_cpp(spell_id, u32::from(difficulty_id))
            {
                SpellAcquisitionMetadataLookupLikeCpp::Present(misc) => {
                    misc.is_channeled_checked().unwrap_or(true)
                }
                SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow => false,
                SpellAcquisitionMetadataLookupLikeCpp::MissingCoverage
                | SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(_) => true,
            },
            // The reduced trainer path does not execute C++ `SearchSpellFocus`.
            // C++ builds SpellInfo from the effective SpellCastingRequirements
            // row, so the audit must use that same overlay-composed authority.
            // A missing SpellInfo key remains indeterminate and fails closed.
            spell_focus_requirement,
            // C++ copies DB2 `RequiredAreasID` into SpellInfo and always runs
            // `CheckLocation`, including for this triggered trainer cast.
            // The reduced path cannot resolve AreaGroupMember ancestry yet,
            // so such wrappers must remain outside static authority.
            required_area_requirement,
            // C++ `SpellInfo::CheckLocation` requires at least one matching
            // `spell_area` row whenever the spell has any. The reduced
            // trainer projection does not yet evaluate the player's complete
            // zone/quest/aura/race/gender context, so those wrappers fail
            // closed rather than bypassing the cast gate.
            spell_area_requirement: !spell_areas
                .spell_area_map_bounds_like_cpp(spell_id)
                .is_empty(),
            linked_spell: [
                SpellLinkedTypeLikeCpp::Cast,
                SpellLinkedTypeLikeCpp::Hit,
                SpellLinkedTypeLikeCpp::Aura,
            ]
            .into_iter()
            .any(|kind| linked.get_spell_linked_like_cpp(kind, spell_id).is_some()),
        };
        if !trainer_cast_world_hooks_are_static_safe_like_cpp(hook_audit) {
            continue;
        }
        let effects = match catalog.difficulty_none_effects_like_cpp(spell_id) {
            wow_data::SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) => effects,
            wow_data::SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage
            | wow_data::SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_) => continue,
        };
        if trainer_cast_effects_are_static_safe_like_cpp(spell_id, effects, |effect_index| {
            pet_auras
                .get_pet_aura_like_cpp(spell_id, effect_index)
                .is_some()
        }) {
            safe_cast_spell_ids.insert(spell_id);
        }
    }

    // C++ `SpellMgr::IsSpellValid` evaluates the final SpellEffect payload,
    // recursively follows LEARN_SPELL, and validates every created item and
    // positive reagent. Derive this authority from the same effective catalog
    // so official/custom overlays cannot leave a stale dataset-specific pin.
    let mut effective_effects_by_spell = BTreeMap::new();
    for (spell_id, difficulty_id) in spell_store.spell_info_keys_in_order_like_cpp() {
        if difficulty_id != 0 {
            continue;
        }
        if let wow_data::SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
            catalog.difficulty_none_effects_like_cpp(spell_id)
        {
            effective_effects_by_spell.insert(spell_id, effects.to_vec());
        }
    }
    let valid_craft_spell_ids = derive_valid_craft_spell_ids_like_cpp(
        &effective_effects_by_spell,
        &trainer_reagents,
        item_exists,
    );

    Ok(TrainerSpellStaticAuthorityLikeCpp {
        safe_cast_spell_ids,
        valid_craft_spell_ids,
        spell_script_exact_spell_ids: script_bindings.exact_spell_ids,
        spell_script_all_rank_root_spell_ids: script_bindings.all_rank_root_spell_ids,
        legacy_spell_script_spell_ids: legacy_scripts,
        cast_cost_store,
    })
}

async fn load_effective_spell_cast_costs_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellAcquisitionStartupPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellCastCostStoreLikeCpp> {
    let reagent_path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("SpellReagents.db2");
    let reagent_hash = Wdc4Reader::open(&reagent_path)
        .with_context(|| format!("failed to read table hash from {}", reagent_path.display()))?
        .table_hash();
    let reagent_store =
        SpellReagentsStore::load(data_dir, locale).context("failed to load SpellReagents.db2")?;
    let reagent_base = reagent_store
        .entries_like_cpp()
        .cloned()
        .collect::<Vec<_>>();
    let mut reagent_overlays = [Vec::new(), Vec::new()];
    for (batch_index, official) in [true, false].into_iter().enumerate() {
        reagent_overlays[batch_index] = loaded_startup_like_cpp(
            persistence
                .load_spell_reagents_overlay_like_cpp(official)
                .await,
        )?
        .into_iter()
        .map(|row| SpellReagentsEntry {
            id: row.id,
            spell_id: row.spell_id,
            reagent: row.reagent,
            reagent_count: row.reagent_count,
        })
        .collect();
    }
    let [reagent_official, reagent_custom] = reagent_overlays;
    let removed_reagent_ids = removals
        .removed_records_in_order_like_cpp()
        .into_iter()
        .filter_map(|(removed_table_hash, record_id)| {
            (removed_table_hash == reagent_hash).then_some(record_id)
        });
    let currency_path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("SpellReagentsCurrency.db2");
    let currency_hash = Wdc4Reader::open(&currency_path)
        .with_context(|| format!("failed to read table hash from {}", currency_path.display()))?
        .table_hash();
    let currency_store = SpellReagentsCurrencyStore::load(data_dir, locale)
        .context("failed to load SpellReagentsCurrency.db2")?;
    let currency_base = currency_store
        .entries_like_cpp()
        .cloned()
        .collect::<Vec<_>>();
    let mut currency_overlays = [Vec::new(), Vec::new()];
    for (batch_index, official) in [true, false].into_iter().enumerate() {
        currency_overlays[batch_index] = loaded_startup_like_cpp(
            persistence
                .load_spell_reagents_currency_overlay_like_cpp(official)
                .await,
        )?
        .into_iter()
        .map(
            |row: SpellReagentsCurrencyPersistenceRowLikeCpp| SpellReagentsCurrencyEntry {
                id: row.id,
                spell_id: row.spell_id,
                currency_types_id: row.currency_types_id,
                currency_count: row.currency_count,
            },
        )
        .collect();
    }
    let [currency_official, currency_custom] = currency_overlays;
    let removed_currency_ids = removals
        .removed_records_in_order_like_cpp()
        .into_iter()
        .filter_map(|(removed_table_hash, record_id)| {
            (removed_table_hash == currency_hash).then_some(record_id)
        });
    Ok(
        SpellCastCostStoreLikeCpp::from_effective_rows_and_removals_like_cpp(
            reagent_base,
            reagent_official,
            reagent_custom,
            removed_reagent_ids,
            currency_base,
            currency_official,
            currency_custom,
            removed_currency_ids,
        ),
    )
}

fn derive_valid_craft_spell_ids_like_cpp(
    effects_by_spell: &BTreeMap<u32, Vec<SpellAcquisitionEffectLikeCpp>>,
    reagents_by_spell: &BTreeMap<u32, [i32; 8]>,
    item_exists: impl Fn(u32) -> bool,
) -> BTreeSet<u32> {
    fn is_valid(
        spell_id: u32,
        effects_by_spell: &BTreeMap<u32, Vec<SpellAcquisitionEffectLikeCpp>>,
        reagents_by_spell: &BTreeMap<u32, [i32; 8]>,
        item_exists: &impl Fn(u32) -> bool,
        visiting: &mut BTreeSet<u32>,
        memo: &mut BTreeMap<u32, bool>,
    ) -> bool {
        if let Some(valid) = memo.get(&spell_id) {
            return *valid;
        }
        let Some(effects) = effects_by_spell.get(&spell_id) else {
            return false;
        };
        // C++ assumes acyclic LEARN_SPELL data and would recurse forever on a
        // cycle. The startup authority must fail closed instead.
        if !visiting.insert(spell_id) {
            return false;
        }

        let is_loot_crafting = effects.iter().any(|effect| {
            matches!(
                effect.effect_type_checked(),
                Ok(SPELL_EFFECT_CREATE_RANDOM_ITEM_LIKE_CPP | SPELL_EFFECT_CREATE_LOOT_LIKE_CPP)
            )
        });
        let mut need_check_reagents = false;
        let mut valid = true;
        for effect in effects {
            let Ok(effect_type) = effect.effect_type_checked() else {
                valid = false;
                break;
            };
            match effect_type {
                SPELL_EFFECT_CREATE_ITEM_LIKE_CPP | SPELL_EFFECT_CREATE_LOOT_LIKE_CPP => {
                    need_check_reagents = true;
                    let Ok(item_id) = effect.item_type_checked() else {
                        valid = false;
                        break;
                    };
                    if (item_id == 0 && !is_loot_crafting)
                        || (item_id != 0 && !item_exists(item_id))
                    {
                        valid = false;
                        break;
                    }
                }
                SPELL_EFFECT_LEARN_SPELL => {
                    let Ok(learned_spell_id) = effect.trigger_spell_id_checked() else {
                        valid = false;
                        break;
                    };
                    if !is_valid(
                        learned_spell_id,
                        effects_by_spell,
                        reagents_by_spell,
                        item_exists,
                        visiting,
                        memo,
                    ) {
                        valid = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if valid && need_check_reagents {
            valid = reagents_by_spell
                .get(&spell_id)
                .into_iter()
                .flatten()
                .all(|reagent_id| {
                    *reagent_id <= 0 || u32::try_from(*reagent_id).ok().is_some_and(item_exists)
                });
        }
        visiting.remove(&spell_id);
        memo.insert(spell_id, valid);
        valid
    }

    let craft_spell_ids = effects_by_spell.iter().filter_map(|(spell_id, effects)| {
        effects
            .iter()
            .any(|effect| {
                matches!(
                    effect.effect_type_checked(),
                    Ok(SPELL_EFFECT_CREATE_ITEM_LIKE_CPP | SPELL_EFFECT_CREATE_LOOT_LIKE_CPP)
                )
            })
            .then_some(*spell_id)
    });
    let mut memo = BTreeMap::new();
    craft_spell_ids
        .filter(|spell_id| {
            is_valid(
                *spell_id,
                effects_by_spell,
                reagents_by_spell,
                &item_exists,
                &mut BTreeSet::new(),
                &mut memo,
            )
        })
        .collect()
}

fn compose_effective_spell_reagents_like_cpp(
    base_rows: impl IntoIterator<Item = SpellReagentsEntry>,
    official_rows: impl IntoIterator<Item = SpellReagentsEntry>,
    custom_rows: impl IntoIterator<Item = SpellReagentsEntry>,
    removed_record_ids: impl IntoIterator<Item = i32>,
) -> BTreeMap<u32, [i32; 8]> {
    SpellCastCostStoreLikeCpp::from_effective_rows_and_removals_like_cpp(
        base_rows,
        official_rows,
        custom_rows,
        removed_record_ids,
        [],
        [],
        [],
        [],
    )
    .trainer_reagent_ids_like_cpp()
}

#[derive(Debug, PartialEq, Eq)]
enum LearnSkillProjectionLikeCpp {
    Covered(SpellLearnSkillSourceSpellInfoLikeCpp),
    Indeterminate(SpellLearnSkillIndeterminateReasonLikeCpp),
}

/// Compose the effective spell-acquisition stores in C++ startup order.
///
/// C++ anchors:
/// - `World.cpp:1858-1956`: custom attributes precede ranks/learn stores;
/// - `SpellMgr.cpp:812-902`: rank chains from final SkillLineAbility rows;
/// - `SpellMgr.cpp:947-1135`: learn-skill and learn-spell projections;
/// - `SpellMgr.cpp:2608-2690`: slot-by-slot effect fallback and first-present
///   singleton metadata.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellAcquisitionStartupPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    spell_store: &SpellStore,
    serverside_spell_store: &ServersideSpellStoreLikeCpp,
    difficulty_store: &DifficultyStore,
    skill_store: &SkillStore,
) -> Result<SpellAcquisitionBootstrapLikeCpp> {
    let regular_keys = spell_store.spell_info_keys_in_order_like_cpp();
    let serverside_keys = serverside_spell_store
        .spell_infos_by_spell_and_difficulty
        .keys()
        .copied()
        .collect::<Vec<_>>();

    let mut coverage = Vec::with_capacity(regular_keys.len() + serverside_keys.len());
    coverage.extend(regular_keys.iter().map(|(spell_id, difficulty_id)| {
        SpellAcquisitionCoverageSeedLikeCpp::covered(*spell_id, u32::from(*difficulty_id))
    }));
    coverage.extend(serverside_keys.iter().map(|key| {
        SpellAcquisitionCoverageSeedLikeCpp::indeterminate(
            key.spell_id,
            key.difficulty_id,
            SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
        )
    }));
    let indeterminate_seed_count = serverside_keys.len();

    let overlay_source = SpellAcquisitionSqlOverlaySourceBridgeLikeCpp { persistence };
    let catalog = Arc::new(
        SpellAcquisitionCatalogLikeCpp::load_effective_like_cpp(
            data_dir,
            locale,
            &overlay_source,
            removals,
            coverage,
        )
        .await
        .context("Failed to load the effective spell-acquisition catalog")?,
    );
    let catalog_indeterminate_diagnostics = catalog
        .diagnostics_like_cpp()
        .iter()
        .filter(|diagnostic| {
            diagnostic.severity == SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate
        })
        .count();
    info!(
        regular_seed_count = regular_keys.len(),
        serverside_seed_count = serverside_keys.len(),
        indeterminate_seed_count,
        diagnostic_count = catalog.diagnostics_like_cpp().len(),
        indeterminate_diagnostic_count = catalog_indeterminate_diagnostics,
        removed_row_count = catalog.removed_rows_like_cpp().len(),
        "Loaded effective spell-acquisition catalog"
    );

    // C++ applies SQL custom attributes to every exact SpellInfo variant
    // before deriving Talent flags and before rank/learn stores are built.
    let custom_variants = custom_attribute_variants_like_cpp(
        &regular_keys,
        serverside_spell_store,
        difficulty_store,
        &catalog,
    );
    let custom_attribute_rows =
        loaded_startup_like_cpp(persistence.load_spell_custom_attributes_like_cpp().await)
            .context("Failed to load C++ spell_custom_attr rows")?
            .into_iter()
            .map(|row| wow_data::SpellCustomAttributeRowLikeCpp {
                spell_id: row.spell_id,
                attributes: row.attributes,
            })
            .collect::<Vec<_>>();
    let mut custom_attribute_outcome =
        SpellCustomAttributeStoreLikeCpp::from_sql_rows_for_variants_like_cpp(
            custom_attribute_rows,
            |spell_id| custom_variants.get(&spell_id).cloned().unwrap_or_default(),
        );
    let custom_talent_unknown_keys =
        custom_talent_unknown_keys_like_cpp(&custom_attribute_outcome.errors);
    let mut derived_talent_variant_count = 0usize;
    for (spell_id, variants) in &custom_variants {
        if matches!(
            catalog.talent_membership_like_cpp(*spell_id),
            SpellAcquisitionTalentLookupLikeCpp::Talent
        ) {
            derived_talent_variant_count += apply_proven_talent_to_variants_like_cpp(
                &mut custom_attribute_outcome.store,
                *spell_id,
                variants.iter().map(|variant| variant.difficulty),
            );
        }
    }
    info!(
        loaded_row_count = custom_attribute_outcome.loaded_row_count,
        applied_variant_count = custom_attribute_outcome.applied_variant_count,
        derived_talent_variant_count,
        error_count = custom_attribute_outcome.errors.len(),
        talent_indeterminate_variant_count = custom_talent_unknown_keys.len(),
        "Loaded C++ spell custom attributes before rank and learn stores"
    );
    let custom_attribute_store = Arc::new(custom_attribute_outcome.store);

    let regular_difficulty_none_ids = regular_keys
        .iter()
        .filter_map(|(spell_id, difficulty_id)| (*difficulty_id == 0).then_some(*spell_id))
        .collect::<BTreeSet<_>>();
    let spell_exists_at_difficulty_none = |spell_id| {
        resolved_difficulty_none_like_cpp(
            spell_store,
            serverside_spell_store,
            difficulty_store,
            spell_id,
        )
        .is_some()
    };

    let chain_outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            skill_store
                .skill_line_ability_rank_rows_like_cpp()
                .iter()
                .cloned(),
            spell_exists_at_difficulty_none,
        );
    let invalid_rank_source_count = chain_outcome
        .diagnostics_in_order_like_cpp
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic,
                wow_data::SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                    ..
                }
            )
        })
        .count();
    info!(
        node_count = chain_outcome.store.chains_by_spell_id.len(),
        rank_load_diagnostic_count = chain_outcome.diagnostics_in_order_like_cpp.len(),
        invalid_effective_source_count = invalid_rank_source_count,
        "Loaded represented C++ spell rank-chain nodes from effective SkillLineAbility rows"
    );
    let chain_store = Arc::new(chain_outcome.store);

    let serverside_difficulty_none_count = serverside_keys
        .iter()
        .filter(|key| key.difficulty_id == 0)
        .count();
    let mut learn_skill_sources = Vec::new();
    let mut learn_skill_indeterminate_sources = BTreeMap::new();
    for key in serverside_keys.iter().filter(|key| key.difficulty_id == 0) {
        learn_skill_indeterminate_sources.insert(
            key.spell_id,
            SpellLearnSkillIndeterminateReasonLikeCpp::EffectiveMetadata(vec![
                SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
            ]),
        );
    }
    for spell_id in &regular_difficulty_none_ids {
        let chain = difficulty_chain_like_cpp(difficulty_store, 0);
        let effects = match catalog
            .resolved_effects_for_difficulty_chain_like_cpp(*spell_id, chain.iter().copied())
        {
            SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => effects,
            SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { difficulty_id } => {
                learn_skill_indeterminate_sources.insert(
                    *spell_id,
                    SpellLearnSkillIndeterminateReasonLikeCpp::MissingEffectiveCoverage {
                        difficulty_id,
                    },
                );
                continue;
            }
            SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(reasons) => {
                learn_skill_indeterminate_sources.insert(
                    *spell_id,
                    SpellLearnSkillIndeterminateReasonLikeCpp::EffectiveMetadata(reasons),
                );
                continue;
            }
        };
        match project_learn_skill_source_like_cpp(*spell_id, &effects) {
            LearnSkillProjectionLikeCpp::Covered(source) => learn_skill_sources.push(source),
            LearnSkillProjectionLikeCpp::Indeterminate(reason) => {
                learn_skill_indeterminate_sources.insert(*spell_id, reason);
            }
        }
    }
    let mut learn_skill_outcome =
        SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp(learn_skill_sources);
    for (spell_id, reason) in learn_skill_indeterminate_sources {
        learn_skill_outcome
            .store
            .mark_spell_learn_skill_indeterminate_like_cpp(spell_id, reason);
    }
    info!(
        loaded_node_count = learn_skill_outcome.store.skill_by_spell_id.len(),
        compatibility_error_count = learn_skill_outcome.errors.len(),
        indeterminate_source_count = learn_skill_outcome.store.indeterminate_by_spell_id.len(),
        "Loaded C++ spell-learn-skill entries from effective acquisition metadata"
    );
    let learn_skill_store = Arc::new(learn_skill_outcome.store);

    // SQL spell_learn_spell validation needs only GetSpellInfo(NONE)
    // existence and effective IS_TALENT. Keep this map separate from the
    // richer auto-edge projection so missing Effects/Misc cannot reject a
    // C++-valid SQL row.
    let all_spell_ids = regular_keys
        .iter()
        .map(|(spell_id, _)| *spell_id)
        .chain(serverside_keys.iter().map(|key| key.spell_id))
        .collect::<BTreeSet<_>>();
    let mut sql_source_infos = BTreeMap::new();
    let mut sql_talent_indeterminate_sources = 0usize;
    for spell_id in all_spell_ids {
        let Some(resolved_difficulty) = resolved_difficulty_none_like_cpp(
            spell_store,
            serverside_spell_store,
            difficulty_store,
            spell_id,
        ) else {
            continue;
        };
        let custom_proves_talent = custom_attribute_store
            .attributes_for_spell_difficulty_like_cpp(spell_id, resolved_difficulty)
            & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
            != 0;
        let custom_talent_unknown =
            custom_talent_unknown_keys.contains(&(spell_id, resolved_difficulty));
        let Some(is_talent) = effective_talent_like_cpp(
            custom_proves_talent,
            custom_talent_unknown,
            catalog.talent_membership_like_cpp(spell_id),
        ) else {
            sql_talent_indeterminate_sources += 1;
            continue;
        };
        sql_source_infos.insert(
            spell_id,
            SpellLearnSourceSpellInfoLikeCpp {
                spell_id,
                difficulty_none: true,
                is_talent,
                is_passive: false,
                has_skill_step_effect: false,
                learn_spell_effects: Vec::new(),
            },
        );
    }

    let mut automatic_sources = Vec::new();
    let mut automatic_indeterminate_sources = serverside_difficulty_none_count;
    for spell_id in &regular_difficulty_none_ids {
        let chain = difficulty_chain_like_cpp(difficulty_store, 0);
        let effects = match catalog
            .resolved_effects_for_difficulty_chain_like_cpp(*spell_id, chain.iter().copied())
        {
            SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => effects,
            SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { .. }
            | SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(_) => {
                automatic_indeterminate_sources += 1;
                continue;
            }
        };
        let is_passive = match catalog
            .resolved_misc_for_difficulty_chain_like_cpp(*spell_id, chain.iter().copied())
        {
            SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(misc) => {
                let Ok(is_passive) = misc.is_passive_checked() else {
                    automatic_indeterminate_sources += 1;
                    continue;
                };
                is_passive
            }
            SpellAcquisitionResolvedMetadataLookupLikeCpp::CoveredWithoutRow => false,
            SpellAcquisitionResolvedMetadataLookupLikeCpp::MissingCoverage { .. }
            | SpellAcquisitionResolvedMetadataLookupLikeCpp::Indeterminate(_) => {
                automatic_indeterminate_sources += 1;
                continue;
            }
        };
        let custom_proves_talent = custom_attribute_store
            .attributes_for_spell_difficulty_like_cpp(*spell_id, 0)
            & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
            != 0;
        let custom_talent_unknown = custom_talent_unknown_keys.contains(&(*spell_id, 0));
        let Some(is_talent) = effective_talent_like_cpp(
            custom_proves_talent,
            custom_talent_unknown,
            catalog.talent_membership_like_cpp(*spell_id),
        ) else {
            automatic_indeterminate_sources += 1;
            continue;
        };
        match project_learn_spell_source_like_cpp(*spell_id, is_talent, is_passive, &effects) {
            Some(source) => automatic_sources.push(source),
            None => automatic_indeterminate_sources += 1,
        }
    }

    let mut invalid_dependency_rows = 0usize;
    let dependency_rows = catalog
        .effective_dependency_rows_like_cpp()
        .filter_map(|row| {
            let (Ok(spell_id), Ok(learn_spell_id), Ok(overrides_spell_id)) = (
                row.spell_id_checked(),
                row.learned_spell_id_checked(),
                row.overrides_spell_id_checked(),
            ) else {
                invalid_dependency_rows += 1;
                return None;
            };
            let (Ok(spell_id), Ok(learn_spell_id), Ok(overrides_spell_id)) = (
                i32::try_from(spell_id),
                i32::try_from(learn_spell_id),
                i32::try_from(overrides_spell_id.unwrap_or(0)),
            ) else {
                invalid_dependency_rows += 1;
                return None;
            };
            Some(SpellLearnSpellEntry {
                id: row.record_id,
                spell_id,
                learn_spell_id,
                overrides_spell_id,
            })
        })
        .collect::<Vec<_>>();

    let sql_source_infos_for_lookup = sql_source_infos;
    let sql_learn_spell_rows =
        loaded_startup_like_cpp(persistence.load_spell_learn_spells_like_cpp().await)
            .context("Failed to load C++ spell_learn_spell rows")?
            .into_iter()
            .map(|row| wow_data::SpellLearnSpellSqlRowLikeCpp {
                entry: row.entry,
                spell_id: row.spell_id,
                active: row.active,
            })
            .collect::<Vec<_>>();
    let learn_spell_outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        sql_learn_spell_rows,
        automatic_sources,
        dependency_rows,
        |spell_id| sql_source_infos_for_lookup.get(&spell_id).cloned(),
        spell_exists_at_difficulty_none,
    );
    info!(
        sql_loaded_row_count = learn_spell_outcome.sql_loaded_row_count,
        canonical_loaded_edge_count = learn_spell_outcome.dbc_loaded_row_count,
        validation_error_count = learn_spell_outcome.errors.len(),
        warning_count = learn_spell_outcome.warnings.len(),
        sql_result_empty = learn_spell_outcome.sql_result_empty,
        sql_talent_indeterminate_source_count = sql_talent_indeterminate_sources,
        automatic_indeterminate_source_count = automatic_indeterminate_sources,
        invalid_dependency_row_count = invalid_dependency_rows,
        "Loaded C++ spell-learn-spell graph from separated SQL and canonical projections"
    );
    let learn_spell_store = Arc::new(learn_spell_outcome.store);

    Ok(SpellAcquisitionBootstrapLikeCpp {
        catalog,
        chain_store,
        learn_skill_store,
        learn_spell_store,
        custom_attribute_store,
    })
}

/// C++ starts with the requested difficulty and then follows
/// `FallbackDifficultyID` while the current Difficulty row exists. Rust stops
/// a malformed cycle, and retains the final fallback id even when it has no
/// Difficulty row so the metadata resolver can perform the same final lookup.
fn difficulty_chain_like_cpp(
    difficulty_store: &DifficultyStore,
    requested_difficulty: u32,
) -> Vec<u32> {
    let mut chain = vec![requested_difficulty];
    let mut visited = BTreeSet::from([requested_difficulty]);
    let mut current = requested_difficulty;
    while let Some(difficulty) = difficulty_store.get(current) {
        let fallback = u32::from(difficulty.fallback_difficulty_id);
        if !visited.insert(fallback) {
            break;
        }
        chain.push(fallback);
        current = fallback;
    }
    chain
}

fn exact_spell_variant_exists_like_cpp(
    spell_store: &SpellStore,
    serverside_spell_store: &ServersideSpellStoreLikeCpp,
    spell_id: u32,
    difficulty_id: u32,
) -> bool {
    u8::try_from(difficulty_id)
        .ok()
        .is_some_and(|difficulty_id| {
            spell_store.contains_spell_info_exact_like_cpp(spell_id, difficulty_id)
        })
        || serverside_spell_store
            .get_serverside_spell_like_cpp(spell_id, difficulty_id)
            .is_some()
}

fn resolved_difficulty_none_like_cpp(
    spell_store: &SpellStore,
    serverside_spell_store: &ServersideSpellStoreLikeCpp,
    difficulty_store: &DifficultyStore,
    spell_id: u32,
) -> Option<u32> {
    difficulty_chain_like_cpp(difficulty_store, 0)
        .into_iter()
        .find(|difficulty_id| {
            exact_spell_variant_exists_like_cpp(
                spell_store,
                serverside_spell_store,
                spell_id,
                *difficulty_id,
            )
        })
}

fn custom_attribute_variants_like_cpp(
    regular_keys: &[(u32, u8)],
    serverside_spell_store: &ServersideSpellStoreLikeCpp,
    difficulty_store: &DifficultyStore,
    catalog: &SpellAcquisitionCatalogLikeCpp,
) -> BTreeMap<u32, Vec<SpellCustomAttributeSourceVariantLikeCpp>> {
    let mut variants = BTreeMap::<u32, Vec<SpellCustomAttributeSourceVariantLikeCpp>>::new();
    for (spell_id, difficulty_id) in regular_keys {
        let difficulty = u32::from(*difficulty_id);
        let chain = difficulty_chain_like_cpp(difficulty_store, difficulty);
        let effect_types =
            match catalog.resolved_effects_for_difficulty_chain_like_cpp(*spell_id, chain) {
                SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => effects
                    .into_iter()
                    .map(SpellAcquisitionEffectLikeCpp::effect_type_checked)
                    .collect::<Result<Vec<_>, _>>()
                    .ok(),
                SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { .. }
                | SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(_) => None,
            };
        variants
            .entry(*spell_id)
            .or_default()
            .push(SpellCustomAttributeSourceVariantLikeCpp {
                spell_id: *spell_id,
                difficulty,
                effect_types,
            });
    }
    for (key, spell_info) in &serverside_spell_store.spell_infos_by_spell_and_difficulty {
        // The server-side effect loader has already validated the type domain.
        // Keep the checked conversion here so this boundary remains fail-closed
        // if that loader's contract changes.
        let effect_types = spell_info
            .effects
            .iter()
            .map(|effect| u32::try_from(effect.effect).ok())
            .collect::<Option<Vec<_>>>();
        variants
            .entry(key.spell_id)
            .or_default()
            .push(SpellCustomAttributeSourceVariantLikeCpp {
                spell_id: key.spell_id,
                difficulty: key.difficulty_id,
                effect_types,
            });
    }
    for spell_variants in variants.values_mut() {
        spell_variants.sort_by_key(|variant| variant.difficulty);
    }
    variants
}

/// A rejected SHARE_DAMAGE variant is relevant to talent classification only
/// when that same SQL row also requested IS_TALENT.
fn custom_talent_unknown_keys_like_cpp(
    errors: &[wow_data::SpellCustomAttributeLoadErrorLikeCpp],
) -> BTreeSet<(u32, u32)> {
    errors
        .iter()
        .filter_map(|error| {
            (error.kind
                == SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable
                && error.attributes & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP != 0)
                .then(|| {
                    error
                        .difficulty
                        .map(|difficulty| (error.spell_id, difficulty))
                })
                .flatten()
        })
        .collect()
}

/// The second half of C++ `LoadSpellInfoCustomAttributes` derives
/// `SPELL_ATTR0_CU_IS_TALENT` from final Talent rows for every exact
/// SpellInfo variant, after applying the SQL bits.
fn apply_proven_talent_to_variants_like_cpp(
    store: &mut SpellCustomAttributeStoreLikeCpp,
    spell_id: u32,
    difficulties: impl IntoIterator<Item = u32>,
) -> usize {
    let mut applied = 0usize;
    for difficulty in difficulties {
        *store
            .attributes_by_spell_and_difficulty
            .entry(SpellCustomAttributeKeyLikeCpp {
                spell_id,
                difficulty,
            })
            .or_default() |= SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP;
        applied += 1;
    }
    applied
}

fn effective_talent_like_cpp(
    custom_proves_talent: bool,
    custom_talent_unknown: bool,
    talent_lookup: SpellAcquisitionTalentLookupLikeCpp<'_>,
) -> Option<bool> {
    if custom_proves_talent {
        return Some(true);
    }
    match talent_lookup {
        SpellAcquisitionTalentLookupLikeCpp::Talent => Some(true),
        SpellAcquisitionTalentLookupLikeCpp::NotTalent if !custom_talent_unknown => Some(false),
        SpellAcquisitionTalentLookupLikeCpp::NotTalent
        | SpellAcquisitionTalentLookupLikeCpp::Indeterminate(_) => None,
    }
}

fn project_learn_skill_source_like_cpp(
    spell_id: u32,
    effects: &[&SpellAcquisitionEffectLikeCpp],
) -> LearnSkillProjectionLikeCpp {
    let mut projected_effects = Vec::new();
    for effect in effects {
        let effect_type = match effect.effect_type_checked() {
            Ok(effect_type) => effect_type,
            Err(invalid) => {
                return LearnSkillProjectionLikeCpp::Indeterminate(
                    SpellLearnSkillIndeterminateReasonLikeCpp::InvalidEffectiveValue {
                        record_id: effect.record_id,
                        field: invalid.field,
                        raw: invalid.raw,
                    },
                );
            }
        };
        match effect_type {
            SPELL_EFFECT_SKILL => {
                let misc_value = match effect
                    .misc_value_id_checked(0)
                    .ok()
                    .and_then(|value| i32::try_from(value).ok())
                {
                    Some(value) => value,
                    None => {
                        return LearnSkillProjectionLikeCpp::Indeterminate(
                            SpellLearnSkillIndeterminateReasonLikeCpp::InvalidEffectiveValue {
                                record_id: effect.record_id,
                                field: "SpellEffect.EffectMiscValue",
                                raw: effect.effect_misc_value_raw[0],
                            },
                        );
                    }
                };
                let domain = match effect.base_points_die_sides_domain_checked() {
                    Ok(domain) => domain,
                    Err(invalid) => {
                        return LearnSkillProjectionLikeCpp::Indeterminate(
                            SpellLearnSkillIndeterminateReasonLikeCpp::InvalidEffectiveValue {
                                record_id: effect.record_id,
                                field: invalid.field,
                                raw: invalid.raw,
                            },
                        );
                    }
                };
                let Some(calc_value) = domain.deterministic_value() else {
                    return LearnSkillProjectionLikeCpp::Indeterminate(
                        SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
                            record_id: effect.record_id,
                            domain,
                        },
                    );
                };
                projected_effects.push(SpellLearnSkillEffectLikeCpp {
                    effect: effect_type,
                    misc_value,
                    calc_value,
                });
                break;
            }
            SPELL_EFFECT_DUAL_WIELD => {
                projected_effects.push(SpellLearnSkillEffectLikeCpp {
                    effect: effect_type,
                    misc_value: 0,
                    calc_value: 0,
                });
                break;
            }
            _ => {}
        }
    }
    LearnSkillProjectionLikeCpp::Covered(SpellLearnSkillSourceSpellInfoLikeCpp {
        spell_id,
        difficulty_none: true,
        effects: projected_effects,
    })
}

fn project_learn_spell_source_like_cpp(
    spell_id: u32,
    is_talent: bool,
    is_passive: bool,
    effects: &[&SpellAcquisitionEffectLikeCpp],
) -> Option<SpellLearnSourceSpellInfoLikeCpp> {
    let mut has_skill_step_effect = false;
    let mut learn_spell_effects = Vec::new();
    for effect in effects {
        let effect_type = effect.effect_type_checked().ok()?;
        has_skill_step_effect |= effect_type == SPELL_EFFECT_SKILL_STEP;
        if effect_type == SPELL_EFFECT_LEARN_SPELL {
            learn_spell_effects.push(SpellLearnSpellEffectLikeCpp {
                trigger_spell: effect.trigger_spell_id_checked().ok()?,
                target_unit_pet: effect.targets_unit_pet_like_cpp(),
            });
        }
    }
    Some(SpellLearnSourceSpellInfoLikeCpp {
        spell_id,
        difficulty_none: true,
        is_talent,
        is_passive,
        has_skill_step_effect,
        learn_spell_effects,
    })
}

#[cfg(test)]
#[path = "acquisition_loader_tests.rs"]
mod tests;

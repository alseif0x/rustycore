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

mod overlay;
mod trainer;

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
    SpellCastingRequirementsEntry, SpellCastingRequirementsStore, SpellChainStoreLikeCpp,
    SpellCustomAttributeKeyLikeCpp, SpellCustomAttributeLoadErrorKindLikeCpp,
    SpellCustomAttributeSourceVariantLikeCpp, SpellCustomAttributeStoreLikeCpp,
    SpellEquippedItemsEntry, SpellEquippedItemsStore, SpellLearnSkillEffectLikeCpp,
    SpellLearnSkillIndeterminateReasonLikeCpp, SpellLearnSkillSourceSpellInfoLikeCpp,
    SpellLearnSkillStoreLikeCpp, SpellLearnSourceSpellInfoLikeCpp, SpellLearnSpellEffectLikeCpp,
    SpellLearnSpellEntry, SpellLearnSpellStoreLikeCpp, SpellLinkedStoreLikeCpp,
    SpellLinkedTypeLikeCpp, SpellPetAuraStoreLikeCpp, SpellReagentsEntry, SpellReagentsStore,
    SpellStore, wdc4::Wdc4Reader,
};
use wow_persistence::{
    SpellAcquisitionHotfixPersistenceRowLikeCpp, SpellAcquisitionHotfixTablePersistenceLikeCpp,
    SpellAcquisitionStartupLoadOutcomeLikeCpp, SpellAcquisitionStartupPersistencePortLikeCpp,
};

pub(crate) use trainer::{
    TrainerSpellStaticAuthorityLikeCpp, load_trainer_static_authority_like_cpp,
};

use overlay::SpellAcquisitionSqlOverlaySourceBridgeLikeCpp;
#[cfg(test)]
use overlay::overlay_row_like_cpp;

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
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use super::*;
use wow_data::skill::{SKILL_CATEGORY_ARMOR_LIKE_CPP, SKILL_CATEGORY_LANGUAGES_LIKE_CPP};
use wow_data::{
    EffectiveSpellAcquisitionRowsLikeCpp, MountEntry, SkillLineAbilityRecord, SkillLineEntry,
    SkillRaceClassInfoRecord, SkillTiersRowLikeCpp, SpellAcquisitionCoverageSeedLikeCpp,
    SpellAcquisitionDependencyLikeCpp, SpellAcquisitionLevelsLikeCpp, SpellAcquisitionMiscLikeCpp,
    SpellAcquisitionTableHashesLikeCpp,
};

#[derive(Default)]
struct FixtureInput {
    spell_ids: Vec<u32>,
    effects: Vec<SpellAcquisitionEffectLikeCpp>,
    dependencies: Vec<SpellAcquisitionDependencyLikeCpp>,
    dependency_nodes: Vec<(u32, SpellLearnSpellNodeLikeCpp)>,
    required_edges: Vec<(u32, u32)>,
    chains: Vec<(u32, SpellChainNodeLikeCpp)>,
    learn_skills: Vec<(u32, SpellLearnSkillNodeLikeCpp)>,
    cast_safe_spell_ids: Vec<u32>,
    misc_rows: Vec<SpellAcquisitionMiscLikeCpp>,
    level_rows: Vec<SpellAcquisitionLevelsLikeCpp>,
    skill_lines: Vec<SkillLineEntry>,
    incomplete_skill_line_ids: Vec<u32>,
    skill_abilities: Vec<SkillLineAbilityRecord>,
    skill_race_class: Vec<SkillRaceClassInfoRecord>,
    skill_tiers: Vec<SkillTiersRowLikeCpp>,
}

struct MetadataFixture {
    catalog: SpellAcquisitionCatalogLikeCpp,
    spell_chains: SpellChainStoreLikeCpp,
    spell_learn_skills: SpellLearnSkillStoreLikeCpp,
    spell_learn_spells: SpellLearnSpellStoreLikeCpp,
    spell_required: SpellRequiredStoreLikeCpp,
    spell_custom_attributes: SpellCustomAttributeStoreLikeCpp,
    trait_definitions: TraitDefinitionStore,
    cast_authority: SpellAcquisitionCastAuthorityLikeCpp,
    craft_validity_authority: SpellAcquisitionCraftValidityAuthorityLikeCpp,
    mounts: MountStore,
    mounts_complete: bool,
    skills: SkillStore,
    skill_lines: SkillLineStore,
    skill_tiers: SkillTiersStoreLikeCpp,
}

impl MetadataFixture {
    fn new(mut input: FixtureInput) -> Self {
        input.spell_ids.sort_unstable();
        input.spell_ids.dedup();

        let catalog = SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            input
                .spell_ids
                .iter()
                .copied()
                .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: input.effects,
                spell_learn_spells: input.dependencies.clone(),
                spell_misc: input.misc_rows,
                spell_levels: input.level_rows,
                ..Default::default()
            },
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        );

        let mut spell_chains = SpellChainStoreLikeCpp::default();
        spell_chains.chains_by_spell_id.extend(input.chains);

        let mut spell_learn_skills = SpellLearnSkillStoreLikeCpp::default();
        spell_learn_skills
            .covered_spell_ids
            .extend(input.spell_ids.iter().copied());
        spell_learn_skills
            .skill_by_spell_id
            .extend(input.learn_skills);

        let mut spell_learn_spells = SpellLearnSpellStoreLikeCpp::default();
        if input.dependency_nodes.is_empty() {
            for dependency in input.dependencies {
                let source_spell = u32::try_from(dependency.spell_id_raw)
                    .expect("fixture dependency source must fit u32");
                let learned_spell = u32::try_from(dependency.learn_spell_id_raw)
                    .expect("fixture dependency target must fit u32");
                let overrides_spell = u32::try_from(dependency.overrides_spell_id_raw)
                    .expect("fixture override must fit u32");
                spell_learn_spells
                    .learned_by_spell_id
                    .entry(source_spell)
                    .or_default()
                    .push(SpellLearnSpellNodeLikeCpp {
                        spell: learned_spell,
                        overrides_spell,
                        active: true,
                        auto_learned: false,
                    });
            }
        } else {
            for (source_spell, node) in input.dependency_nodes {
                spell_learn_spells
                    .learned_by_spell_id
                    .entry(source_spell)
                    .or_default()
                    .push(node);
            }
        }

        let mut spell_required = SpellRequiredStoreLikeCpp::default();
        for (spell_id, required_spell_id) in input.required_edges {
            spell_required
                .required_by_spell_id
                .entry(spell_id)
                .or_default()
                .push(required_spell_id);
            spell_required
                .requiring_by_required_spell_id
                .entry(required_spell_id)
                .or_default()
                .push(spell_id);
        }
        let mut effective_skill_line_ids = input
            .skill_lines
            .iter()
            .map(|entry| entry.id)
            .collect::<BTreeSet<_>>();
        effective_skill_line_ids.extend(input.incomplete_skill_line_ids);

        Self {
            catalog,
            spell_chains,
            spell_learn_skills,
            spell_learn_spells,
            spell_required,
            spell_custom_attributes: SpellCustomAttributeStoreLikeCpp::default(),
            trait_definitions: TraitDefinitionStore::from_entries([]),
            cast_authority: SpellAcquisitionCastAuthorityLikeCpp::from_audited_rows_like_cpp(
                input.cast_safe_spell_ids,
                [],
            ),
            craft_validity_authority:
                SpellAcquisitionCraftValidityAuthorityLikeCpp::from_audited_rows_like_cpp(
                    input.spell_ids,
                    [],
                ),
            mounts: MountStore::from_entries([]),
            mounts_complete: true,
            skills: SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
                input.skill_abilities,
                input.skill_race_class,
            ),
            skill_lines: SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
                input.skill_lines,
                effective_skill_line_ids,
            ),
            skill_tiers: SkillTiersStoreLikeCpp::from_rows_like_cpp(input.skill_tiers),
        }
    }

    fn metadata(&self) -> SpellAcquisitionMetadataLikeCpp<'_> {
        SpellAcquisitionMetadataLikeCpp {
            catalog: &self.catalog,
            spell_chains: &self.spell_chains,
            spell_learn_skills: &self.spell_learn_skills,
            spell_learn_spells: &self.spell_learn_spells,
            spell_required: &self.spell_required,
            spell_custom_attributes: &self.spell_custom_attributes,
            trait_definitions: &self.trait_definitions,
            cast_authority: &self.cast_authority,
            craft_validity_authority: &self.craft_validity_authority,
            mounts: self.mounts_complete.then_some(&self.mounts),
            skills: &self.skills,
            skill_lines: &self.skill_lines,
            skill_tiers: &self.skill_tiers,
        }
    }
}

fn snapshot() -> PlayerSpellAcquisitionSnapshotLikeCpp {
    PlayerSpellAcquisitionSnapshotLikeCpp {
        spells: Vec::new(),
        skills: Vec::new(),
        occupied_skill_slots: 0,
        overrides: Vec::new(),
        race: 1,
        class: 1,
        level: 80,
        lifecycle: PlayerAcquisitionLifecycleLikeCpp::InWorld,
        future_player_condition_resolutions: Vec::new(),
        cast_resolutions: BTreeMap::new(),
    }
}

fn snapshot_with_cast(
    spell_id: u32,
    reached_immediate_phase: bool,
    executed_hit_target_effect_indices: impl IntoIterator<Item = u8>,
) -> PlayerSpellAcquisitionSnapshotLikeCpp {
    let mut snapshot = snapshot();
    let executed_hit_target_effect_mask = executed_hit_target_effect_indices
        .into_iter()
        .fold(0_u32, |mask, effect_index| {
            mask | (1_u32 << u32::from(effect_index))
        });
    snapshot.cast_resolutions.insert(
        spell_id,
        PlayerCastAcquisitionResolutionLikeCpp {
            reached_immediate_phase,
            executed_hit_target_effect_mask,
            executed_dual_wield_effects: Vec::new(),
        },
    );
    snapshot
}

fn spell_row(spell_id: u32, active: bool, dependent: bool) -> PlayerSpellAcquisitionRowLikeCpp {
    PlayerSpellAcquisitionRowLikeCpp {
        spell_id,
        active,
        disabled: false,
        dependent,
        favorite: false,
        trait_definition_id: None,
        state: PlayerSpellPersistenceStateLikeCpp::Unchanged,
    }
}

fn deleted_skill_row(skill_id: u32) -> PlayerSkillAcquisitionRowLikeCpp {
    PlayerSkillAcquisitionRowLikeCpp {
        skill_id,
        step: 0,
        value: 0,
        maximum: 0,
        profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
        state: PlayerSkillPersistenceStateLikeCpp::Deleted,
    }
}

fn effect(
    record_id: u32,
    spell_id: u32,
    effect_index: u8,
    effect_type: u32,
) -> SpellAcquisitionEffectLikeCpp {
    SpellAcquisitionEffectLikeCpp {
        record_id,
        spell_id_raw: i64::from(spell_id),
        difficulty_id_raw: 0,
        effect_index_raw: i64::from(effect_index),
        effect_type_raw: i64::from(effect_type),
        effect_base_points_raw: 0,
        effect_die_sides_raw: 0,
        effect_chain_targets_raw: 0,
        effect_points_per_resource_bits: 0.0_f32.to_bits(),
        effect_real_points_per_level_bits: 0.0_f32.to_bits(),
        effect_coefficient_bits: 0.0_f32.to_bits(),
        effect_variance_bits: 0.0_f32.to_bits(),
        effect_trigger_spell_raw: 0,
        effect_misc_value_raw: [0, 0],
        implicit_target_raw: [0, 0],
    }
}

fn learn_effect(
    record_id: u32,
    wrapper_spell_id: u32,
    effect_index: u8,
    learned_spell_id: u32,
) -> SpellAcquisitionEffectLikeCpp {
    let mut row = effect(
        record_id,
        wrapper_spell_id,
        effect_index,
        SPELL_EFFECT_LEARN_SPELL,
    );
    row.effect_trigger_spell_raw = i64::from(learned_spell_id);
    row
}

fn skill_step_effect(
    record_id: u32,
    wrapper_spell_id: u32,
    effect_index: u8,
    skill_id: u32,
    step: i32,
) -> SpellAcquisitionEffectLikeCpp {
    let mut row = effect(
        record_id,
        wrapper_spell_id,
        effect_index,
        SPELL_EFFECT_SKILL_STEP,
    );
    row.effect_misc_value_raw[0] = i64::from(skill_id);
    row.effect_base_points_raw = i64::from(step);
    row
}

fn skill_effect(
    record_id: u32,
    wrapper_spell_id: u32,
    effect_index: u8,
    skill_id: u32,
    step: i32,
) -> SpellAcquisitionEffectLikeCpp {
    let mut row = effect(
        record_id,
        wrapper_spell_id,
        effect_index,
        SPELL_EFFECT_SKILL,
    );
    row.effect_misc_value_raw[0] = i64::from(skill_id);
    row.effect_base_points_raw = i64::from(step);
    row
}

fn dependency(
    record_id: u32,
    source_spell_id: u32,
    learned_spell_id: u32,
) -> SpellAcquisitionDependencyLikeCpp {
    SpellAcquisitionDependencyLikeCpp {
        record_id,
        spell_id_raw: i64::from(source_spell_id),
        learn_spell_id_raw: i64::from(learned_spell_id),
        overrides_spell_id_raw: 0,
    }
}

fn rank_node(
    previous: Option<u32>,
    next: Option<u32>,
    first: u32,
    last: u32,
    rank: u8,
) -> SpellChainNodeLikeCpp {
    SpellChainNodeLikeCpp {
        prev_spell_id: previous,
        next_spell_id: next,
        first_spell_id: first,
        last_spell_id: last,
        rank,
    }
}

fn skill_line(skill_id: u32, parent: u32, parent_tier: i32) -> SkillLineEntry {
    skill_line_with_category(
        skill_id,
        SKILL_CATEGORY_PROFESSION_LIKE_CPP,
        parent,
        parent_tier,
    )
}

fn skill_line_with_category(
    skill_id: u32,
    category_id: i8,
    parent: u32,
    parent_tier: i32,
) -> SkillLineEntry {
    SkillLineEntry {
        id: skill_id,
        display_name: format!("skill-{skill_id}"),
        alternate_verb: String::new(),
        description: String::new(),
        horde_display_name: String::new(),
        override_source_info_display_name: String::new(),
        category_id,
        spell_icon_file_id: 0,
        can_link: 0,
        parent_skill_line_id: parent,
        parent_tier_index: parent_tier,
        flags: 0,
        spell_book_spell_id: 0,
    }
}

fn ability(
    record_id: u32,
    skill_id: u16,
    spell_id: u32,
    acquire_method: i8,
) -> SkillLineAbilityRecord {
    SkillLineAbilityRecord {
        id: record_id,
        race_mask: 0,
        skill_line: skill_id,
        spell: spell_id as i32,
        min_skill_line_rank: 0,
        class_mask: 0,
        supercedes_spell: 0,
        acquire_method,
        trivial_rank_high: 0,
        trivial_rank_low: 0,
        flags: 0,
        num_skill_ups: 1,
        skillup_skill_line_id: 0,
    }
}

fn race_class(skill_id: u16, record_id: u32, tier_id: i16) -> SkillRaceClassInfoRecord {
    SkillRaceClassInfoRecord {
        id: record_id,
        race_mask: 0,
        skill_id,
        class_mask: 0,
        flags: 0,
        availability: 1,
        min_level: 1,
        skill_tier_id: tier_id,
    }
}

fn tiers(tier_id: u32) -> SkillTiersRowLikeCpp {
    let mut value = [0; wow_data::MAX_SKILL_STEP_LIKE_CPP];
    value[0] = 75;
    value[1] = 150;
    SkillTiersRowLikeCpp { id: tier_id, value }
}

fn misc(
    spell_id: u32,
    attributes_0: i64,
    attributes_1: i64,
    condition: i64,
) -> SpellAcquisitionMiscLikeCpp {
    SpellAcquisitionMiscLikeCpp {
        record_id: spell_id,
        spell_id_raw: i64::from(spell_id),
        difficulty_id_raw: 0,
        attributes_raw: [attributes_0, attributes_1],
        show_future_spell_player_condition_id_raw: condition,
    }
}

fn levels(spell_id: u32, base_level: i64, spell_level: i64) -> SpellAcquisitionLevelsLikeCpp {
    SpellAcquisitionLevelsLikeCpp {
        record_id: spell_id,
        spell_id_raw: i64::from(spell_id),
        difficulty_id_raw: 0,
        base_level_raw: base_level,
        spell_level_raw: spell_level,
    }
}

fn cast_evidence(spell_id: u32) -> SpellAcquisitionCastAuditEvidenceLikeCpp {
    SpellAcquisitionCastAuditEvidenceLikeCpp {
        spell_id,
        all_sources_complete: true,
        has_script_binding: false,
        has_legacy_spell_script_command: false,
        has_spell_pet_aura: false,
        has_linked_cast: false,
        has_linked_hit: false,
        has_linked_aura: false,
        has_cast_condition: false,
        has_target_condition: false,
        has_spell_modifier_class_options: false,
        has_spell_modifier_label: false,
        has_aura_learn_spell: false,
        has_runtime_calc_value: false,
        is_disabled: false,
        has_hardcoded_dummy_handler: false,
        is_delayed_or_channeled: false,
        has_unsupported_target_selection: false,
        has_unmodelled_check_cast: false,
        has_runtime_state_mutation_before_closure: false,
        passive_cast_prerequisites_proven: true,
        is_passive_cast: false,
    }
}

fn deterministic(outcome: SpellAcquisitionOutcomeLikeCpp) -> SpellAcquisitionPlanLikeCpp {
    match outcome {
        SpellAcquisitionOutcomeLikeCpp::Deterministic(plan) => plan,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(reason) => {
            panic!("expected deterministic plan, got {reason:?}")
        }
    }
}

mod authority;
mod cast;
mod skill_parent;
mod skill_rewards;
mod skill_state;
mod spell;

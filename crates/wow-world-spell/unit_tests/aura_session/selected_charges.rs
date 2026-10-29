use std::time::Instant;

use super::{
    selected_aura_max_charges, validate_selected_aura_modifier_absence,
    validate_selected_aura_modifier_absence_from_player_auras,
};
use wow_core::ObjectGuid;
use wow_data::{
    DifficultyEntry, DifficultyStore, SpellAuraOptionsEntry, SpellAuraOptionsStore,
    SpellClassOptionsEntry, SpellClassOptionsStore, SpellLabelEntry, SpellLabelStore,
    ServersideSpellEffectKeyLikeCpp, ServersideSpellEffectLikeCpp, ServersideSpellInfoLikeCpp,
    ServersideSpellRowLikeCpp, ServersideSpellStoreLikeCpp, SpellStore,
    spell::{
        ImplicitSpellProcEffectLikeCpp, ImplicitSpellProcSourceLikeCpp, SpellProcRowLikeCpp,
        SpellProcSourceSpellInfoLikeCpp, SpellProcStoreLikeCpp as ProcStore,
        aura_types::{
            SPELL_AURA_ADD_FLAT_MODIFIER, SPELL_AURA_ADD_PCT_MODIFIER,
            SPELL_AURA_PROC_TRIGGER_SPELL,
        },
        spell_effect_types::{
            SPELL_EFFECT_APPLY_AREA_AURA_PARTY, SPELL_EFFECT_APPLY_AURA,
        },
    },
};
use wow_entities::{AuraApplicationLikeCpp, AuraSubsystem};

const SPELL_ID: u32 = 82_041;
const OTHER_SPELL_ID: u32 = 82_042;
const MODIFIER_SPELL_ID: u32 = 82_043;

fn player_guid() -> ObjectGuid {
    ObjectGuid::create_player(1, 82_044)
}

fn difficulty(id: u32, fallback_difficulty_id: u8) -> DifficultyEntry {
    DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id,
        toggle_difficulty_id: 0,
    }
}

fn aura_options_row(
    id: u32,
    difficulty_id: u8,
    spell_id: u32,
    charges: i32,
) -> SpellAuraOptionsEntry {
    SpellAuraOptionsEntry {
        id,
        difficulty_id,
        cumulative_aura: 0,
        proc_category_recovery: 0,
        proc_chance: 0,
        proc_charges: charges,
        spell_procs_per_minute_id: 0,
        proc_type_mask: [0; 2],
        spell_id,
    }
}

fn proc_row(spell_id: u32, difficulty: u32, row_charges: u8, source_charges: u32) -> ProcStore {
    let source = SpellProcSourceSpellInfoLikeCpp {
        spell_id,
        difficulty,
        first_rank_spell_id: spell_id,
        next_rank_spell_id: None,
        spell_family_name: 0,
        proc_flags: [0; 2],
        proc_charges: source_charges,
        proc_chance: 0.0,
        proc_cooldown_ms: 0,
        proc_base_ppm: 0.0,
        attributes3: 0,
        effects: Vec::new(),
    };
    let outcome = ProcStore::from_rows_like_cpp(
        [SpellProcRowLikeCpp {
            spell_id: spell_id as i32,
            school_mask: 0,
            spell_family_name: 0,
            spell_family_mask: [0; 4],
            proc_flags: [0; 2],
            spell_type_mask: 0,
            spell_phase_mask: 0,
            hit_mask: 0,
            attributes_mask: 0,
            disable_effects_mask: 0,
            procs_per_minute: 0.0,
            chance: 0.0,
            cooldown_ms: 0,
            charges: row_charges,
        }],
        |_| Some(source.clone()),
    );
    outcome.store
}

fn proc_store(rows: &[(u32, u32, u8, u32)]) -> ProcStore {
    let mut store = ProcStore::default();
    for &(spell_id, difficulty, row_charges, source_charges) in rows {
        store.proc_entries_by_spell_and_difficulty.extend(
            proc_row(spell_id, difficulty, row_charges, source_charges)
                .proc_entries_by_spell_and_difficulty,
        );
    }
    store
}

fn generated_zero_proc(spell_id: u32, difficulty: u32) -> ProcStore {
    ProcStore::from_rows_and_implicit_sources_like_cpp(
        std::iter::empty::<SpellProcRowLikeCpp>(),
        |_| None,
        [ImplicitSpellProcSourceLikeCpp {
            spell_id,
            difficulty,
            spell_family_name: 0,
            proc_flags: [1, 0],
            proc_chance: 0.0,
            proc_cooldown_ms: 0,
            proc_charges: 0,
            proc_base_ppm: 0.0,
            attributes3: 0,
            effects: [ImplicitSpellProcEffectLikeCpp {
                effect_index: 0,
                is_effect: true,
                is_aura: true,
                aura_type: SPELL_AURA_PROC_TRIGGER_SPELL,
                spell_class_mask: [0; 4],
                calc_value: 0,
                trigger_spell: 0,
            }]
            .into_iter()
            .collect(),
        }],
    )
    .store
}

fn class_row(id: u32, spell_id: i32, flags: [u32; 4]) -> SpellClassOptionsEntry {
    SpellClassOptionsEntry {
        id,
        spell_id,
        modal_next_spell: 0,
        spell_class_set: 0,
        spell_class_mask: flags,
    }
}

fn label_row(id: u32, spell_id: u32, label_id: u32) -> SpellLabelEntry {
    SpellLabelEntry {
        id,
        label_id,
        spell_id,
    }
}

fn validate_selected_aura(
    attributes: [u32; 15],
    class_options: Option<&SpellClassOptionsStore>,
    labels: Option<&SpellLabelStore>,
    difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<(), &'static str> {
    validate_selected_aura_modifier_absence(
        SPELL_ID,
        difficulty,
        attributes,
        class_options,
        labels,
        difficulties,
    )
}

#[test]
fn charge_conversion_preserves_full_width_until_after_proc_override() {
    let empty_aura_options = SpellAuraOptionsStore::from_entries([]);
    let empty_procs = ProcStore::default();

    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&SpellAuraOptionsStore::from_entries([aura_options_row(
                1, 4, SPELL_ID, 256,
            )])),
            Some(&empty_procs),
            None,
        ),
        Ok(0),
    );
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&SpellAuraOptionsStore::from_entries([aura_options_row(
                1, 4, SPELL_ID, -1,
            )])),
            Some(&empty_procs),
            None,
        ),
        Ok(u8::MAX),
    );

    let full_width_proc = proc_store(&[(SPELL_ID, 4, 0, 256)]);
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&SpellAuraOptionsStore::from_entries([aura_options_row(
                1, 4, SPELL_ID, 7,
            )])),
            Some(&full_width_proc),
            None,
        ),
        Ok(0),
    );
    let signed_bits_proc = proc_store(&[(SPELL_ID, 4, 0, u32::MAX)]);
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&empty_aura_options),
            Some(&signed_bits_proc),
            None,
        ),
        Ok(u8::MAX),
    );
}

#[test]
fn zero_and_nonzero_proc_entries_override_aura_options() {
    let options = SpellAuraOptionsStore::from_entries([aura_options_row(1, 4, SPELL_ID, 6)]);
    let zero = proc_store(&[(SPELL_ID, 4, 0, 0)]);
    let nonzero = proc_store(&[(SPELL_ID, 4, 7, 0)]);

    assert_eq!(
        selected_aura_max_charges(SPELL_ID, 4, Some(&options), Some(&zero), None),
        Ok(0),
    );
    assert_eq!(
        selected_aura_max_charges(SPELL_ID, 4, Some(&options), Some(&nonzero), None),
        Ok(7),
    );
}

#[test]
fn proc_lookup_uses_the_actual_difficulty_then_fallback_four_three_zero() {
    let difficulties =
        DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 0), difficulty(0, 0)]);
    let options = SpellAuraOptionsStore::from_entries([aura_options_row(1, 4, SPELL_ID, 1)]);

    let exact_and_fallback = proc_store(&[
        (SPELL_ID, 4, 10, 0),
        (SPELL_ID, 3, 8, 0),
        (SPELL_ID, 0, 6, 0),
    ]);
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options),
            Some(&exact_and_fallback),
            Some(&difficulties),
        ),
        Ok(10),
    );

    let fallback_three = proc_store(&[(SPELL_ID, 3, 8, 0), (SPELL_ID, 0, 6, 0)]);
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options),
            Some(&fallback_three),
            Some(&difficulties),
        ),
        Ok(8),
    );

    let fallback_zero = proc_store(&[(SPELL_ID, 0, 6, 0)]);
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options),
            Some(&fallback_zero),
            Some(&difficulties),
        ),
        Ok(6),
    );
}

#[test]
fn generated_or_existing_exact_zero_stops_a_nonzero_proc_fallback() {
    let difficulties = DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 0)]);
    let options = SpellAuraOptionsStore::from_entries([aura_options_row(1, 4, SPELL_ID, 5)]);
    let fallback = proc_store(&[(SPELL_ID, 3, 9, 0)]);
    let existing_zero = proc_store(&[(SPELL_ID, 4, 0, 0), (SPELL_ID, 3, 9, 0)]);
    let mut generated_zero_with_fallback = generated_zero_proc(SPELL_ID, 4);
    generated_zero_with_fallback
        .proc_entries_by_spell_and_difficulty
        .extend(fallback.proc_entries_by_spell_and_difficulty.clone());

    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options),
            Some(&fallback),
            Some(&difficulties),
        ),
        Ok(9),
    );
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options),
            Some(&existing_zero),
            Some(&difficulties),
        ),
        Ok(0),
    );
    assert_eq!(
        generated_zero_with_fallback
            .spell_proc_entry_like_cpp(SPELL_ID, 4)
            .map(|entry| entry.charges),
        Some(0),
    );
    assert_eq!(
        generated_zero_with_fallback
            .spell_proc_entry_like_cpp(SPELL_ID, 3)
            .map(|entry| entry.charges),
        Some(9),
    );
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options),
            Some(&generated_zero_with_fallback),
            Some(&difficulties),
        ),
        Ok(0),
    );
}

#[test]
fn required_catalogs_distinguish_unavailable_from_known_empty() {
    let empty_options = SpellAuraOptionsStore::from_entries([]);
    let empty_procs = ProcStore::default();

    assert!(selected_aura_max_charges(SPELL_ID, 4, None, Some(&empty_procs), None).is_err());
    assert!(selected_aura_max_charges(SPELL_ID, 4, Some(&empty_options), None, None).is_err());
    assert_eq!(
        selected_aura_max_charges(SPELL_ID, 4, Some(&empty_options), Some(&empty_procs), None),
        Ok(0),
    );

    let unrelated_cycle = DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 4)]);
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&empty_options),
            Some(&empty_procs),
            Some(&unrelated_cycle),
        ),
        Ok(0),
    );
}

#[test]
fn required_graph_errors_and_known_missing_nodes_remain_distinct() {
    let options_at_zero =
        SpellAuraOptionsStore::from_entries([aura_options_row(1, 0, SPELL_ID, 3)]);
    let empty_procs = ProcStore::default();
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options_at_zero),
            Some(&empty_procs),
            None
        ),
        Err("Selected spell catalog difficulty is unavailable"),
    );

    let options_at_four =
        SpellAuraOptionsStore::from_entries([aura_options_row(1, 4, SPELL_ID, 5)]);
    let proc_at_zero = proc_store(&[(SPELL_ID, 0, 8, 0)]);
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options_at_four),
            Some(&proc_at_zero),
            None
        ),
        Err("Selected spell catalog difficulty is unavailable"),
    );

    let cycle = DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 4)]);
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options_at_zero),
            Some(&empty_procs),
            Some(&cycle),
        ),
        Err("Selected spell catalog difficulty chain contains a cycle"),
    );

    let missing_node = DifficultyStore::from_entries([]);
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options_at_zero),
            Some(&empty_procs),
            Some(&missing_node),
        ),
        Ok(0),
    );
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&options_at_four),
            Some(&proc_at_zero),
            Some(&missing_node),
        ),
        Ok(5),
    );
}

#[test]
fn ignore_caster_modifiers_bypasses_unknown_modifier_catalogs_only() {
    let mut attributes = [0; 15];
    attributes[3] = 0x2000_0000;
    assert_eq!(
        validate_selected_aura(attributes, None, None, 4, None),
        Ok(())
    );

    assert!(selected_aura_max_charges(SPELL_ID, 4, None, None, None).is_err());
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            4,
            Some(&SpellAuraOptionsStore::from_entries([])),
            Some(&ProcStore::default()),
            None,
        ),
        Ok(0),
    );
}

#[test]
fn class_mask_admission_uses_highest_record_and_requires_known_zero_flags() {
    let lower_nonzero = SpellClassOptionsStore::from_entries([
        class_row(10, SPELL_ID as i32, [0, 0, 0, 7]),
        class_row(20, SPELL_ID as i32, [0; 4]),
    ]);
    let empty_labels = SpellLabelStore::from_entries([]);
    assert_eq!(
        validate_selected_aura([0; 15], Some(&lower_nonzero), Some(&empty_labels), 0, None),
        Ok(()),
    );

    let highest_nonzero = SpellClassOptionsStore::from_entries([
        class_row(10, SPELL_ID as i32, [0; 4]),
        class_row(20, SPELL_ID as i32, [0, 0, 0, 7]),
    ]);
    assert_eq!(
        validate_selected_aura([0; 15], Some(&highest_nonzero), None, 0, None),
        Err("Selected aura may have applicable class-mask SpellMods"),
    );

    let high_bit_spell_id = 0x8001_2345;
    let high_bit_class = SpellClassOptionsStore::from_entries([class_row(
        30,
        high_bit_spell_id as i32,
        [0, 0, 0, 1],
    )]);
    assert_eq!(
        validate_selected_aura_modifier_absence(
            high_bit_spell_id,
            0,
            [0; 15],
            Some(&high_bit_class),
            None,
            None,
        ),
        Err("Selected aura may have applicable class-mask SpellMods"),
    );
}

#[test]
fn label_zero_counts_and_labels_inherit_only_after_reaching_difficulty_zero() {
    let empty_classes = SpellClassOptionsStore::from_entries([]);
    let label_zero = SpellLabelStore::from_entries([label_row(1, SPELL_ID, 0)]);
    let difficulties =
        DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 0), difficulty(0, 0)]);

    assert_eq!(
        validate_selected_aura(
            [0; 15],
            Some(&empty_classes),
            Some(&label_zero),
            4,
            Some(&difficulties),
        ),
        Err("Selected aura may have applicable label SpellMods"),
    );
    assert_eq!(
        validate_selected_aura([0; 15], Some(&empty_classes), Some(&label_zero), 0, None),
        Err("Selected aura may have applicable label SpellMods"),
    );

    let empty_labels = SpellLabelStore::from_entries([label_row(2, OTHER_SPELL_ID, 99)]);
    assert_eq!(
        validate_selected_aura([0; 15], Some(&empty_classes), Some(&empty_labels), 4, None),
        Ok(()),
    );
    assert!(validate_selected_aura([0; 15], Some(&empty_classes), None, 0, None).is_err());
}

#[test]
fn unresolved_class_or_label_metadata_is_an_error_not_modifier_absence() {
    let base_class_options =
        SpellClassOptionsStore::from_entries([class_row(1, SPELL_ID as i32, [0; 4])]);
    let labels = SpellLabelStore::from_entries([]);
    let unreachable_nonzero_class =
        SpellClassOptionsStore::from_entries([class_row(1, SPELL_ID as i32, [0, 0, 0, 9])]);
    assert!(validate_selected_aura([0; 15], None, Some(&labels), 0, None).is_err());
    assert!(validate_selected_aura([0; 15], Some(&base_class_options), None, 0, None).is_err());

    let missing_node = DifficultyStore::from_entries([]);
    assert_eq!(
        validate_selected_aura(
            [0; 15],
            Some(&unreachable_nonzero_class),
            Some(&labels),
            4,
            Some(&missing_node),
        ),
        Ok(()),
    );
    assert!(
        validate_selected_aura(
            [0; 15],
            Some(&unreachable_nonzero_class),
            Some(&labels),
            4,
            None
        )
        .is_err()
    );

    let label_rows = SpellLabelStore::from_entries([label_row(1, SPELL_ID, 0)]);
    assert_eq!(
        validate_selected_aura(
            [0; 15],
            Some(&SpellClassOptionsStore::from_entries([])),
            Some(&label_rows),
            4,
            Some(&missing_node),
        ),
        Ok(()),
    );
    assert!(
        validate_selected_aura(
            [0; 15],
            Some(&SpellClassOptionsStore::from_entries([])),
            Some(&label_rows),
            4,
            None
        )
        .is_err()
    );

    let cycle = DifficultyStore::from_entries([difficulty(4, 3), difficulty(3, 4)]);
    assert!(
        validate_selected_aura(
            [0; 15],
            Some(&unreachable_nonzero_class),
            Some(&labels),
            4,
            Some(&cycle)
        )
        .is_err()
    );
    assert!(
        validate_selected_aura(
            [0; 15],
            Some(&SpellClassOptionsStore::from_entries([])),
            Some(&label_rows),
            4,
            Some(&cycle)
        )
        .is_err()
    );
}

fn modifier_effect(
    index: i32,
    difficulty_id: u8,
    effect_type: i32,
    aura_type: i32,
    operation: i32,
    label: i32,
    class_mask: [u32; 4],
) -> ServersideSpellEffectLikeCpp {
    ServersideSpellEffectLikeCpp {
        effect_index: index,
        difficulty_id: u32::from(difficulty_id),
        effect: effect_type,
        effect_aura: aura_type,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura_period: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_item_type: 0,
        effect_mechanic: 0,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_base_points: 0,
        effect_misc_value: [operation, label],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: class_mask.map(|word| word as i32),
        implicit_target: [0; 2],
    }
}

fn server_info(
    spell_id: u32,
    difficulty_id: u8,
    family: u32,
    effects: Vec<ServersideSpellEffectLikeCpp>,
) -> (ServersideSpellEffectKeyLikeCpp, ServersideSpellInfoLikeCpp) {
    let row = ServersideSpellRowLikeCpp {
        spell_id,
        difficulty_id: u32::from(difficulty_id),
        category_id: 0,
        dispel: 0,
        mechanic: 0,
        attributes: 0,
        attributes_ex: [0; 14],
        stances: 0,
        stances_not: 0,
        targets: 0,
        target_creature_type: 0,
        requires_spell_focus: 0,
        facing_caster_flags: 0,
        caster_aura_state: 0,
        target_aura_state: 0,
        exclude_caster_aura_state: 0,
        exclude_target_aura_state: 0,
        caster_aura_spell: 0,
        target_aura_spell: 0,
        exclude_caster_aura_spell: 0,
        exclude_target_aura_spell: 0,
        caster_aura_type: 0,
        target_aura_type: 0,
        exclude_caster_aura_type: 0,
        exclude_target_aura_type: 0,
        casting_time_index: 0,
        recovery_time: 0,
        category_recovery_time: 0,
        start_recovery_category: 0,
        start_recovery_time: 0,
        interrupt_flags: 0,
        aura_interrupt_flags: [0; 2],
        channel_interrupt_flags: [0; 2],
        proc_flags: [0; 2],
        proc_chance: 0,
        proc_charges: 0,
        proc_cooldown: 0,
        proc_base_ppm: 0.0,
        max_level: 0,
        base_level: 0,
        spell_level: 0,
        duration_index: 0,
        range_index: 0,
        speed: 0.0,
        launch_delay: 0.0,
        stack_amount: 0,
        equipped_item_class: 0,
        equipped_item_sub_class_mask: 0,
        equipped_item_inventory_type_mask: 0,
        content_tuning_id: 0,
        spell_name: String::new(),
        cone_angle: 0.0,
        cone_width: 0.0,
        max_target_level: 0,
        max_affected_targets: 0,
        spell_family_name: family,
        spell_family_flags: [0; 4],
        dmg_class: 0,
        prevention_type: 0,
        area_group_id: 0,
        school_mask: 0,
        charge_category_id: 0,
    };
    (
        ServersideSpellEffectKeyLikeCpp {
            spell_id,
            difficulty_id: u32::from(difficulty_id),
        },
        ServersideSpellInfoLikeCpp { row, effects },
    )
}

fn server_store(
    entries: impl IntoIterator<
        Item = (u32, u8, u32, Vec<ServersideSpellEffectLikeCpp>),
    >,
) -> ServersideSpellStoreLikeCpp {
    let mut store = ServersideSpellStoreLikeCpp::default();
    for (spell_id, difficulty_id, family, effects) in entries {
        let (key, info) = server_info(spell_id, difficulty_id, family, effects);
        store.spell_infos_by_spell_and_difficulty.insert(key, info);
    }
    store
}

fn active_aura(spell_id: u32, difficulty_id: u8, effect_mask: u32) -> AuraApplicationLikeCpp {
    AuraApplicationLikeCpp {
        spell_id: spell_id as i32,
        difficulty_id,
        caster_guid: ObjectGuid::create_player(1, 82_045),
        slot: 0,
        duration_total: 60_000,
        duration_remaining: 60_000,
        stack_count: 1,
        aura_flags: 0,
        effect_mask,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    }
}

fn aura_snapshot(
    applications: impl IntoIterator<Item = AuraApplicationLikeCpp>,
    complete: bool,
    tombstoned: bool,
) -> AuraSubsystem {
    let mut auras = AuraSubsystem::default();
    for application in applications {
        auras.insert_runtime_application_like_cpp(application);
    }
    auras.set_persisted_player_aura_authority_complete_like_cpp(complete);
    if tombstoned {
        auras.tombstone_spell_hit_aura_authority_like_cpp();
    }
    auras
}

fn validate_with_auras(
    auras: Option<&AuraSubsystem>,
    caster_guid: ObjectGuid,
    actual_difficulty: u8,
    target_family: u8,
    target_flags: [u32; 4],
    target_labels: &[u32],
    server_spells: Option<&ServersideSpellStoreLikeCpp>,
    difficulties: Option<&DifficultyStore>,
) -> Result<(), &'static str> {
    let classes = SpellClassOptionsStore::from_entries([SpellClassOptionsEntry {
        id: 90_001,
        spell_id: SPELL_ID as i32,
        modal_next_spell: 0,
        spell_class_set: target_family,
        spell_class_mask: target_flags,
    }]);
    let labels = SpellLabelStore::from_entries(
        target_labels
            .iter()
            .enumerate()
            .map(|(index, &label_id)| label_row(90_010 + index as u32, SPELL_ID, label_id)),
    );
    validate_selected_aura_modifier_absence_from_player_auras(
        SPELL_ID,
        actual_difficulty,
        [0; 15],
        caster_guid,
        player_guid(),
        Some(&classes),
        Some(&labels),
        difficulties,
        Some(&SpellStore::new()),
        server_spells,
        auras,
    )
}

#[test]
fn complete_self_aura_snapshot_proves_nonzero_target_mask_absent_and_keeps_charges() {
    let auras = aura_snapshot([], true, false);
    assert_eq!(
        validate_with_auras(
            Some(&auras),
            player_guid(),
            0,
            1,
            [1, 0, 0, 0],
            &[],
            None,
            None,
        ),
        Ok(()),
    );
    assert_eq!(
        selected_aura_max_charges(
            SPELL_ID,
            0,
            Some(&SpellAuraOptionsStore::from_entries([aura_options_row(
                90_020, 0, SPELL_ID, 5,
            )])),
            Some(&ProcStore::default()),
            None,
        ),
        Ok(5),
    );
}

#[test]
fn only_active_proc_charge_class_modifiers_block_including_area_aura_effects() {
    let cases = [
        (
            "matching flat modifier",
            SPELL_AURA_ADD_FLAT_MODIFIER,
            SPELL_EFFECT_APPLY_AURA as i32,
            4,
            [1, 0, 0, 0],
            1,
            1,
            Err("Selected aura may have applicable class-mask SpellMods"),
        ),
        (
            "matching percent modifier",
            SPELL_AURA_ADD_PCT_MODIFIER,
            SPELL_EFFECT_APPLY_AURA as i32,
            4,
            [1, 0, 0, 0],
            1,
            1,
            Err("Selected aura may have applicable class-mask SpellMods"),
        ),
        (
            "matching area-aura modifier",
            SPELL_AURA_ADD_FLAT_MODIFIER,
            SPELL_EFFECT_APPLY_AREA_AURA_PARTY as i32,
            4,
            [1, 0, 0, 0],
            1,
            1,
            Err("Selected aura may have applicable class-mask SpellMods"),
        ),
        (
            "different family",
            SPELL_AURA_ADD_FLAT_MODIFIER,
            SPELL_EFFECT_APPLY_AURA as i32,
            4,
            [1, 0, 0, 0],
            2,
            1,
            Ok(()),
        ),
        (
            "disjoint class mask",
            SPELL_AURA_ADD_FLAT_MODIFIER,
            SPELL_EFFECT_APPLY_AURA as i32,
            4,
            [2, 0, 0, 0],
            1,
            1,
            Ok(()),
        ),
        (
            "different operation",
            SPELL_AURA_ADD_FLAT_MODIFIER,
            SPELL_EFFECT_APPLY_AURA as i32,
            3,
            [1, 0, 0, 0],
            1,
            1,
            Ok(()),
        ),
        (
            "inactive effect",
            SPELL_AURA_ADD_FLAT_MODIFIER,
            SPELL_EFFECT_APPLY_AURA as i32,
            4,
            [1, 0, 0, 0],
            1,
            0,
            Ok(()),
        ),
    ];

    for (index, (name, aura_type, effect_type, operation, mask, family, active_mask, expected)) in
        cases.into_iter().enumerate()
    {
        let auras = aura_snapshot(
            [active_aura(
                MODIFIER_SPELL_ID,
                0,
                active_mask,
            )],
            true,
            false,
        );
        let servers = server_store([(
            MODIFIER_SPELL_ID,
            0,
            family,
            vec![modifier_effect(
                0,
                0,
                effect_type,
                aura_type,
                operation,
                0,
                mask,
            )],
        )]);
        assert_eq!(
            validate_with_auras(
                Some(&auras),
                player_guid(),
                0,
                1,
                [1, 0, 0, 0],
                &[],
                Some(&servers),
                None,
            ),
            expected,
            "{name} (case {index})",
        );
    }
}

#[test]
fn only_matching_active_label_proc_charge_modifiers_block() {
    for (name, label, operation, active_mask, expected) in [
        (
            "matching label",
            77,
            4,
            1,
            Err("Selected aura may have applicable label SpellMods"),
        ),
        ("different label", 78, 4, 1, Ok(())),
        ("different operation", 77, 3, 1, Ok(())),
        ("inactive effect", 77, 4, 0, Ok(())),
    ] {
        let auras = aura_snapshot(
            [active_aura(MODIFIER_SPELL_ID, 0, active_mask)],
            true,
            false,
        );
        let servers = server_store([(
            MODIFIER_SPELL_ID,
            0,
            0,
            vec![modifier_effect(
                0,
                0,
                SPELL_EFFECT_APPLY_AURA as i32,
                wow_data::spell::aura_types::SPELL_AURA_ADD_PCT_MODIFIER_BY_SPELL_LABEL,
                operation,
                label,
                [0; 4],
            )],
        )]);
        assert_eq!(
            validate_with_auras(
                Some(&auras),
                player_guid(),
                0,
                0,
                [0; 4],
                &[77],
                Some(&servers),
                None,
            ),
            expected,
            "{name}",
        );
    }
}

#[test]
fn source_family_uses_target_difficulty_while_modifier_effect_uses_aura_difficulty() {
    let difficulties = DifficultyStore::from_entries([difficulty(3, 0), difficulty(4, 0)]);
    let servers = server_store([
        (
            MODIFIER_SPELL_ID,
            3,
            1,
            Vec::new(),
        ),
        (
            MODIFIER_SPELL_ID,
            4,
            2,
            vec![modifier_effect(
                0,
                4,
                SPELL_EFFECT_APPLY_AREA_AURA_PARTY as i32,
                wow_data::spell::aura_types::SPELL_AURA_ADD_FLAT_MODIFIER,
                4,
                0,
                [1, 0, 0, 0],
            )],
        ),
    ]);
    let auras = aura_snapshot([active_aura(MODIFIER_SPELL_ID, 4, 1)], true, false);
    assert_eq!(
        validate_with_auras(
            Some(&auras),
            player_guid(),
            3,
            1,
            [1, 0, 0, 0],
            &[],
            Some(&servers),
            Some(&difficulties),
        ),
        Err("Selected aura may have applicable class-mask SpellMods"),
    );

    let source_only_at_aura_difficulty = server_store([(
        MODIFIER_SPELL_ID,
        4,
        2,
        vec![modifier_effect(
            0,
            4,
            SPELL_EFFECT_APPLY_AREA_AURA_PARTY as i32,
            wow_data::spell::aura_types::SPELL_AURA_ADD_FLAT_MODIFIER,
            4,
            0,
            [1, 0, 0, 0],
        )],
    )]);
    assert_eq!(
        validate_with_auras(
            Some(&auras),
            player_guid(),
            3,
            1,
            [1, 0, 0, 0],
            &[],
            Some(&source_only_at_aura_difficulty),
            Some(&difficulties),
        ),
        Ok(()),
        "native GetSpellInfo lookup is absent at the target difficulty",
    );
}

#[test]
fn incomplete_tombstoned_foreign_and_unresolved_aura_sources_stay_rejected() {
    let incomplete = aura_snapshot([], false, false);
    let tombstoned = aura_snapshot([], true, true);
    assert!(validate_with_auras(
        Some(&incomplete),
        player_guid(),
        0,
        1,
        [1, 0, 0, 0],
        &[],
        None,
        None,
    )
    .is_err());
    assert!(validate_with_auras(
        Some(&tombstoned),
        player_guid(),
        0,
        1,
        [1, 0, 0, 0],
        &[],
        None,
        None,
    )
    .is_err());
    assert!(validate_with_auras(
        Some(&aura_snapshot([], true, false)),
        ObjectGuid::create_player(1, 82_046),
        0,
        1,
        [1, 0, 0, 0],
        &[],
        None,
        None,
    )
    .is_err());

    let unknown_key = aura_snapshot([active_aura(MODIFIER_SPELL_ID, 0, 1)], true, false);
    assert!(validate_with_auras(
        Some(&unknown_key),
        player_guid(),
        0,
        1,
        [1, 0, 0, 0],
        &[],
        None,
        None,
    )
    .is_err());

    let unknown_effect = aura_snapshot([active_aura(MODIFIER_SPELL_ID, 0, 2)], true, false);
    let known_spell = server_store([(
        MODIFIER_SPELL_ID,
        0,
        1,
        vec![modifier_effect(
            0,
            0,
            SPELL_EFFECT_APPLY_AURA as i32,
            wow_data::spell::aura_types::SPELL_AURA_ADD_FLAT_MODIFIER,
            4,
            0,
            [1, 0, 0, 0],
        )],
    )]);
    assert!(validate_with_auras(
        Some(&unknown_effect),
        player_guid(),
        0,
        1,
        [1, 0, 0, 0],
        &[],
        Some(&known_spell),
        None,
    )
    .is_err());

    let inert_effect = aura_snapshot([active_aura(MODIFIER_SPELL_ID, 0, 1)], true, false);
    let inert_spell = server_store([(
        MODIFIER_SPELL_ID,
        0,
        1,
        vec![modifier_effect(
            0,
            0,
            0,
            SPELL_AURA_ADD_FLAT_MODIFIER,
            4,
            0,
            [1, 0, 0, 0],
        )],
    )]);
    assert!(validate_with_auras(
        Some(&inert_effect),
        player_guid(),
        0,
        1,
        [1, 0, 0, 0],
        &[],
        Some(&inert_spell),
        None,
    )
    .is_err());

    let mismatched_effect_difficulty =
        aura_snapshot([active_aura(MODIFIER_SPELL_ID, 0, 1)], true, false);
    let mismatched_spell = server_store([(
        MODIFIER_SPELL_ID,
        0,
        1,
        vec![modifier_effect(
            0,
            1,
            SPELL_EFFECT_APPLY_AURA as i32,
            SPELL_AURA_ADD_FLAT_MODIFIER,
            4,
            0,
            [1, 0, 0, 0],
        )],
    )]);
    assert!(validate_with_auras(
        Some(&mismatched_effect_difficulty),
        player_guid(),
        0,
        1,
        [1, 0, 0, 0],
        &[],
        Some(&mismatched_spell),
        None,
    )
    .is_err());

    let unknown_difficulty = aura_snapshot([active_aura(MODIFIER_SPELL_ID, 4, 1)], true, false);
    assert!(validate_with_auras(
        Some(&unknown_difficulty),
        player_guid(),
        0,
        1,
        [1, 0, 0, 0],
        &[],
        Some(&known_spell),
        None,
    )
    .is_err());
}

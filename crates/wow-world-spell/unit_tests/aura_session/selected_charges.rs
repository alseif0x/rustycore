use super::{selected_aura_max_charges, validate_selected_aura_modifier_absence};
use wow_data::{
    DifficultyEntry, DifficultyStore, SpellAuraOptionsEntry, SpellAuraOptionsStore,
    SpellClassOptionsEntry, SpellClassOptionsStore, SpellLabelEntry, SpellLabelStore,
    spell::{
        ImplicitSpellProcEffectLikeCpp, ImplicitSpellProcSourceLikeCpp, SpellProcRowLikeCpp,
        SpellProcSourceSpellInfoLikeCpp, SpellProcStoreLikeCpp as ProcStore,
        aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
    },
};

const SPELL_ID: u32 = 82_041;
const OTHER_SPELL_ID: u32 = 82_042;

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

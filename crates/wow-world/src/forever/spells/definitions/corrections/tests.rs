//! Synthetic owned-startup definitions. No Player/DB save or ready-state claim.
use super::*;
use crate::forever::spells::{ServerSpellCounts, SpellLoadCounts, SpellLoadPlan};
use std::sync::Arc;
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};
use wow_persistence::forever::spells::server::ServerSpellRows;

fn catalog(rows: SpellRecords) -> Arc<SpellCatalog> {
    Arc::new(
        rows.finish(
            SpellRecords::default(),
            SpellRecords::default(),
            6,
            SpellLocaleRecords::default(),
            SpellLocaleRecords::default(),
            &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
        )
        .unwrap(),
    )
}
fn definition(slots: usize) -> Definition {
    let mut definition = Definition::empty_server(b"synthetic".to_vec());
    definition.effects = (0..slots)
        .map(|index| SpellEffectValues {
            index: index as u32,
            ..Default::default()
        })
        .collect();
    definition
}
fn seeds(
    rows: SpellRecords,
    definitions: impl IntoIterator<Item = (Key, Definition)>,
) -> SpellDefinitionSeeds {
    SpellDefinitionSeeds {
        catalog: catalog(rows),
        definitions: definitions.into_iter().collect(),
        languages: BTreeMap::new(),
        battle_pets_by_spell: BTreeMap::new(),
        client_counts: SpellLoadCounts::default(),
        server_counts: ServerSpellCounts::default(),
        id_corrections: None,
        global_corrections: None,
        traversal_inputs: None,
        source_order: None,
        skill_line_abilities: None,
        sql_custom_attributes: None,
        custom_attributes: None,
        diminishing: None,
        immunities: None,
        target_caps: None,
        ranks: None,
        required: None,
        learn_skills: None,
        specific: None,
        learn_spells: None,
        value_game_tables: None,
    }
}

#[test]
fn empty_registry_retains_exact_patch_request_inventory_without_manufacturing_spells() {
    let result = seeds(SpellRecords::default(), [])
        .with_id_corrections()
        .unwrap();
    assert!(result.is_empty());
    assert_eq!(
        result.id_correction_counts(),
        Some(IdCorrectionCounts {
            patch_groups: 191,
            requested_spell_ids: 374,
            missing_spell_requests: 374,
            ..Default::default()
        })
    );
}

#[test]
fn source_spell_range_applies_to_every_signed_difficulty_not_only_default() {
    let definitions =
        [i16::MIN, -1, 0, 1, i16::MAX].map(|difficulty| ((59630, difficulty), definition(0)));
    let result = seeds(SpellRecords::default(), definitions)
        .with_id_corrections()
        .unwrap();
    for difficulty in [i16::MIN, -1, 0, 1, i16::MAX] {
        let view = result.get_exact(59630, difficulty).unwrap();
        assert_eq!(view.fields().attributes[0], 0x40);
        assert!(view.is_server_defined());
    }
    let counts = result.id_correction_counts().unwrap();
    assert_eq!(counts.spell_patch_applications, 5);
    assert_eq!(counts.missing_spell_requests, 373);
    assert_eq!(counts.effect_patch_applications, 0);
}

#[test]
fn blank_existing_slot_is_corrected_but_missing_slot_is_not_appended() {
    let result = seeds(
        SpellRecords::default(),
        [((61874, 0), definition(3)), ((71068, 0), definition(1))],
    )
    .with_id_corrections()
    .unwrap();
    let complete = result.get_exact(61874, 0).unwrap();
    let blank = complete.effect(1).unwrap();
    assert_eq!(blank.values().index, 1);
    assert_eq!(blank.values().effect, 6);
    assert_eq!(blank.values().implicit_targets, [1, 0]);
    assert_eq!(blank.values().aura, 23);
    assert_eq!(blank.values().aura_period, 10_000);
    assert_eq!(blank.values().trigger_spell, 24870);
    assert_eq!(complete.effect(0).unwrap().values().effect, 0);
    assert_eq!(complete.effect(2).unwrap().values().effect, 0);
    assert_eq!(result.get_exact(71068, 0).unwrap().effect_count(), 1);
    let counts = result.id_correction_counts().unwrap();
    assert_eq!(counts.effect_patch_applications, 1);
    assert_eq!(counts.missing_effect_requests, 1);
}

#[test]
fn repeated_spell_request_runs_both_groups_in_source_order() {
    let mut source = definition(1);
    source.effects[0].base_points = -91.0;
    source.effects[0].scaling_variance = 2.0;
    let result = seeds(SpellRecords::default(), [((51597, 0), source)])
        .with_id_corrections()
        .unwrap();
    let view = result.get_exact(51597, 0).unwrap();
    let effect = view.effect(0).unwrap();
    assert_eq!(effect.values().base_points, 1.0);
    assert_eq!(effect.values().scaling_variance, 0.0);
    let counts = result.id_correction_counts().unwrap();
    assert_eq!(counts.spell_patch_applications, 2);
    assert_eq!(counts.effect_patch_applications, 2);
    assert_eq!(counts.missing_spell_requests, 372);
}

#[test]
fn checked_dependency_assignment_replaces_missing_link_with_null() {
    let mut range = definition(0);
    range.range = Some(5);
    let mut duration = definition(0);
    duration.duration = Some(5);
    let mut radius = definition(1);
    radius.effects[0].radius_ids = [Some(5), Some(5)];
    let rows = SpellRecords {
        spell_ranges: vec![SpellRangeRecord {
            id: 5,
            display_name: SpellText::default(),
            display_name_short: SpellText::default(),
            flags: 0,
            range_min: [0.0; 2],
            range_max: [40.0; 2],
        }],
        spell_durations: vec![SpellDurationRecord {
            id: 5,
            duration: 10,
            max_duration: 10,
            duration_per_resource: 0,
        }],
        spell_radii: vec![SpellRadiusRecord {
            id: 5,
            radius: 5.0,
            radius_per_level: 0.0,
            radius_min: 0.0,
            radius_max: 5.0,
        }],
        ..Default::default()
    };
    let result = seeds(
        rows,
        [
            ((720, 0), range),
            ((53525, 0), duration),
            ((27892, 0), radius),
        ],
    )
    .with_id_corrections()
    .unwrap();
    assert!(result.get_exact(720, 0).unwrap().range().is_none()); // source lookup 6
    assert!(result.get_exact(53525, 0).unwrap().duration().is_none()); // lookup 4
    let view = result.get_exact(27892, 0).unwrap();
    let effect = view.effect(0).unwrap();
    assert_eq!(effect.values().radius_ids, [None, Some(5)]); // lookup 13, B untouched
    assert_eq!(effect.radii()[1].unwrap().id, 5);
}

#[test]
fn independent_attribute_and_interrupt_words_preserve_unselected_bits() {
    let mut channel = definition(0);
    channel.fields.channel_interrupt_flags = [u32::MAX, 0x8000_0000];
    let mut aura = definition(0);
    aura.fields.aura_interrupt_flags = [u32::MAX, 0x8000_0000];
    let mut creator = definition(0);
    creator.fields.attributes = [u32::MAX; 17];
    let mut m = definition(1);
    m.fields.attributes = [1; 17];
    let result = seeds(
        SpellRecords::default(),
        [
            ((29726, 0), channel),
            ((61719, 0), aura),
            ((258344, 0), creator),
            ((85123, 0), m),
        ],
    )
    .with_id_corrections()
    .unwrap();
    assert_eq!(
        result
            .get_exact(29726, 0)
            .unwrap()
            .fields()
            .channel_interrupt_flags,
        [0xffff_fffb, 0x8000_0000]
    );
    assert_eq!(
        result
            .get_exact(61719, 0)
            .unwrap()
            .fields()
            .aura_interrupt_flags,
        [3, 0x8000_0000]
    );
    let view = result.get_exact(258344, 0).unwrap();
    for index in 0..17 {
        assert_eq!(
            view.fields().attributes[index],
            if index == 8 { 0xffff_fff7 } else { u32::MAX }
        );
    }
    assert_eq!(
        result.get_exact(85123, 0).unwrap().fields().attributes,
        [1; 17]
    );
    assert_eq!(
        result
            .get_exact(85123, 0)
            .unwrap()
            .effect(0)
            .unwrap()
            .values()
            .implicit_targets,
        [7, 0]
    );
}

#[test]
fn custom_flags_and_negative_effect_bits_are_not_raw_client_attribute_words() {
    let result = seeds(
        SpellRecords::default(),
        [
            ((404468, 0), definition(0)),
            ((61882, 0), definition(0)),
            ((1225826, 0), definition(0)),
            ((5420, 0), definition(0)),
        ],
    )
    .with_id_corrections()
    .unwrap();
    let custom = result.get_exact(404468, 0).unwrap();
    assert_eq!(custom.custom_attributes(), 0x0100_0000);
    assert_eq!(custom.fields().attributes, [0; 17]);
    assert_eq!(
        result
            .get_exact(61882, 0)
            .unwrap()
            .negative_effects()
            .iter()
            .enumerate()
            .filter(|(_, flag)| **flag)
            .map(|(i, _)| i)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(result.get_exact(1225826, 0).unwrap().negative_effects()[0]);
    assert_eq!(result.get_exact(5420, 0).unwrap().fields().stances, 2);
}

#[test]
fn additive_patch_cannot_be_reapplied_to_the_same_startup_owner() {
    let mut source = definition(3);
    source.effects[2].base_points = 0.5;
    let result = seeds(SpellRecords::default(), [((30421, 0), source)])
        .with_id_corrections()
        .unwrap();
    assert_eq!(
        result
            .get_exact(30421, 0)
            .unwrap()
            .effect(2)
            .unwrap()
            .values()
            .base_points,
        30000.5
    );
    assert!(matches!(
        result.with_id_corrections(),
        Err(SpellDefinitionError::IdCorrectionsAlreadyApplied)
    ));
}

#[test]
fn production_join_keeps_raw_client_namespace_while_correcting_derived_definition() {
    let raw = catalog(SpellRecords {
        spell_names: vec![SpellNameRecord {
            id: 783,
            name: SpellText::default(),
        }],
        spell_misc: vec![SpellMiscRecord {
            id: 1,
            difficulty_id: 0,
            attributes: [0; 17],
            casting_time_index: 0,
            duration_index: 0,
            pv_p_duration_index: 0,
            range_index: 0,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_visual_script: 0,
            active_spell_visual_script: 0,
            spell_id: 783,
        }],
        ..Default::default()
    });
    let original_pointer = raw.spell_misc(1).unwrap() as *const SpellMiscRecord;
    let result = SpellLoadPlan::build(raw.clone())
        .unwrap()
        .with_server_spells(ServerSpellRows::default())
        .unwrap()
        .with_id_corrections()
        .unwrap();
    assert_eq!(
        result.get_exact(783, 0).unwrap().fields().attributes[0],
        0x8000
    );
    assert_eq!(raw.spell_misc(1).unwrap().attributes, [0; 17]);
    assert_eq!(
        original_pointer,
        result.catalog.spell_misc(1).unwrap() as *const SpellMiscRecord
    );
    assert_eq!(raw.counts(), result.catalog.counts());
}

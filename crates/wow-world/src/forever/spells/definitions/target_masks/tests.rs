//! Fresh readonly derivation, not cached custom-attribute or Player admission.
use super::*;
use crate::forever::spells::SpellEffectValues;
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};

fn catalog() -> SpellCatalog {
    SpellRecords {
        spell_ranges: [[0.0, 15.0], [15.0, 0.0], [-0.0, 0.0], [f32::NAN, 0.0]]
            .into_iter()
            .enumerate()
            .map(|(index, range_max)| SpellRangeRecord {
                id: index as u32 + 1,
                display_name: SpellText::default(),
                display_name_short: SpellText::default(),
                flags: 0,
                range_min: [0.0; 2],
                range_max,
            })
            .collect(),
        ..Default::default()
    }
    .finish(
        SpellRecords::default(),
        SpellRecords::default(),
        6,
        SpellLocaleRecords::default(),
        SpellLocaleRecords::default(),
        &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
    )
    .unwrap()
}
fn effect(kind: u32, targets: [u32; 2]) -> SpellEffectValues {
    SpellEffectValues {
        effect: kind,
        implicit_targets: targets,
        ..Default::default()
    }
}
fn spell(effects: Vec<SpellEffectValues>, range: Option<u32>) -> Definition {
    let mut result = Definition::empty_server(Vec::new());
    result.effects = effects;
    result.range = range;
    result
}
fn masks(available: u32, required: u32) -> ExplicitTargetMasks {
    ExplicitTargetMasks {
        available,
        required,
    }
}

#[test]
fn nonzero_either_range_keeps_missing_targets_nan_is_not_zero_and_null_suppression_is_exact() {
    let raw = catalog();
    for range in [1, 2, 4] {
        assert_eq!(
            derive(&raw, &spell(vec![effect(2, [0, 0])], Some(range))),
            masks(2, 2)
        );
    }
    assert_eq!(
        derive(&raw, &spell(vec![effect(2, [0, 0])], Some(3))),
        masks(0, 0)
    );
    for kind in [2, 13, 18, 86] {
        // unit, unit+dest, corpse, gameobject
        assert_eq!(
            derive(&raw, &spell(vec![effect(kind, [0, 0])], None)),
            masks(0, 0)
        );
    }
    assert_eq!(
        derive(&raw, &spell(vec![effect(5, [0, 0])], None)),
        masks(0x10, 0x10)
    );
    assert_eq!(
        derive(&raw, &spell(vec![effect(33, [0, 0])], None)),
        masks(0x4000, 0x4000)
    );
    // Zero range strips missing flags only, NOT the implicit target's explicit flags.
    assert_eq!(
        derive(&raw, &spell(vec![effect(2, [6, 0])], None)),
        masks(0x80, 0x80)
    );
}

#[test]
fn a_before_b_location_order_is_visible_within_the_same_effect() {
    let raw = catalog();
    assert_eq!(
        derive(&raw, &spell(vec![effect(3, [89, 18])], None)),
        masks(0x60, 0x60)
    );
    assert_eq!(
        derive(&raw, &spell(vec![effect(3, [18, 89])], None)),
        masks(0x20, 0x20)
    );
}

#[test]
fn locations_flow_between_active_effects_but_blank_slots_are_skipped() {
    let raw = catalog();
    assert_eq!(
        derive(
            &raw,
            &spell(vec![effect(3, [18, 0]), effect(3, [89, 0])], None)
        ),
        masks(0x20, 0x20)
    );
    assert_eq!(
        derive(
            &raw,
            &spell(vec![effect(0, [18, 0]), effect(3, [89, 0])], None)
        ),
        masks(0x60, 0x60)
    );
    let mut source = effect(3, [22, 0]);
    source.attributes = 0x0010_0000;
    assert_eq!(
        derive(&raw, &spell(vec![source, effect(3, [89, 0])], None)),
        masks(0x40, 0x40)
    );
}

#[test]
fn optional_effect_suppresses_only_its_required_flags_not_available_or_shared_locations() {
    let raw = catalog();
    let mut optional = effect(2, [6, 0]);
    optional.attributes = 0x0010_0000;
    assert_eq!(
        derive(&raw, &spell(vec![optional, effect(2, [21, 0])], Some(1))),
        masks(0x180, 0x100)
    );
}

#[test]
fn spell_no_target_attribute_affects_raw_targets_only_not_required_effect_mask() {
    let raw = catalog();
    let mut source = spell(vec![effect(2, [0, 0])], Some(1));
    source.fields.targets = 0x8000_0000;
    source.fields.attributes[13] = 0x8000;
    assert_eq!(derive(&raw, &source), masks(0x8000_0002, 2));
    source.fields.attributes[13] = 0;
    assert_eq!(derive(&raw, &source), masks(0x8000_0002, 0x8000_0002));
}

#[test]
fn nonexplicit_effect_object_metadata_does_not_manufacture_an_explicit_requirement() {
    let raw = catalog();
    assert_eq!(
        derive(&raw, &spell(vec![effect(30, [0, 0])], Some(1))),
        masks(0, 0)
    );
}

#[test]
fn production_view_derives_current_fields_without_mutating_or_claiming_initialized_state() {
    let raw = catalog();
    let source = spell(vec![effect(2, [6, 0])], None);
    let before = raw.counts();
    let view = SpellDefinitionView {
        catalog: &raw,
        definition: &source,
        key: (1, 0),
    };
    assert_eq!(view.derive_explicit_target_masks(), masks(0x80, 0x80));
    assert_eq!(source.effects[0].implicit_targets, [6, 0]);
    assert_eq!(source.custom_attributes, 0);
    assert!(source.negative_effects.iter().all(|value| !value));
    assert_eq!(before, raw.counts());
}

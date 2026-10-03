//! Pure queries on the production definition view, not raw-value-only aura inference.
use super::*;
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};

fn catalog() -> SpellCatalog {
    SpellRecords::default()
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
fn view<'a>(catalog: &'a SpellCatalog, values: &'a SpellEffectValues) -> SpellEffectView<'a> {
    SpellEffectView {
        catalog,
        values,
        immunity: None,
    }
}

#[test]
fn each_of_the_361_effect_kinds_uses_exact_area_and_owned_aura_sets() {
    let catalog = catalog();
    for effect in 0..361 {
        let values = SpellEffectValues {
            effect,
            aura: 96,
            ..Default::default()
        };
        let result = view(&catalog, &values);
        let area = [35, 65, 119, 128, 129, 143, 202, 271].contains(&effect);
        let owned = area || [6, 174].contains(&effect);
        assert_eq!(result.is_effect(), effect != 0, "effect {effect}");
        assert!(result.is_effect_kind(effect));
        assert_eq!(result.is_area_aura_effect(), area, "effect {effect}");
        assert_eq!(result.is_unit_owned_aura_effect(), owned, "effect {effect}");
        assert_eq!(result.is_aura(), owned || effect == 27, "effect {effect}");
        assert_eq!(result.is_aura_kind(96), owned || effect == 27);
        assert!(!result.is_aura_kind(236));
    }
}

#[test]
fn a_non_aura_effect_and_blank_slot_do_not_become_auras_from_nonzero_aura_data() {
    let catalog = catalog();
    for effect in [0, 3, 39, 96, 360] {
        let values = SpellEffectValues {
            effect,
            aura: 236,
            ..Default::default()
        };
        let result = view(&catalog, &values);
        assert!(!result.is_aura());
        assert!(!result.is_aura_kind(236));
    }
    let values = SpellEffectValues {
        effect: 6,
        aura: 0,
        ..Default::default()
    };
    assert!(view(&catalog, &values).is_unit_owned_aura_effect());
    assert!(!view(&catalog, &values).is_aura());
}

#[test]
fn aura_classification_is_independent_of_area_target_classification() {
    let catalog = catalog();
    let values = SpellEffectValues {
        effect: 0,
        implicit_targets: [0, 24],
        ..Default::default()
    };
    assert!(!view(&catalog, &values).is_effect());
    assert!(view(&catalog, &values).is_targeting_area()); // source query has no IsEffect guard
    let values = SpellEffectValues {
        effect: 35,
        implicit_targets: [89, 133],
        ..Default::default()
    };
    assert!(view(&catalog, &values).is_area_aura_effect());
    assert!(!view(&catalog, &values).is_targeting_area()); // TRAJ and LINE are not AREA/CONE
}

#[test]
fn both_production_target_slots_resolve_exact_metadata_including_blank_target_zero() {
    let catalog = catalog();
    for id in 0..153 {
        let values = SpellEffectValues {
            implicit_targets: [id, 0],
            ..Default::default()
        };
        let result = view(&catalog, &values);
        let [a, b] = result.implicit_targets();
        assert_eq!(a.id(), id);
        assert_eq!(b.id(), 0);
        assert_eq!(b.object_type(), super::super::super::TargetObject::None);
        assert_eq!(result.is_targeting_area(), a.is_area());
    }
}

#[test]
fn production_effect_target_queries_use_the_complete_checked_metadata() {
    let raw = catalog();
    for effect in 0..361 {
        let values = SpellEffectValues {
            effect,
            ..Default::default()
        };
        assert_eq!(
            view(&raw, &values).target_info(),
            EffectTargetInfo::from_id(effect).unwrap()
        );
    }
}

#[test]
fn provided_and_external_masks_combine_both_target_slots_before_missing_group_removal() {
    let raw = catalog();
    let values = SpellEffectValues {
        effect: 13,
        implicit_targets: [1, 18],
        ..Default::default()
    };
    let result = view(&raw, &values);
    assert_eq!(result.provided_target_mask(), 0x42);
    assert_eq!(result.missing_target_mask(false, false, 0), 0);
    let values = SpellEffectValues {
        effect: 13,
        ..Default::default()
    };
    let result = view(&raw, &values);
    assert_eq!(result.missing_target_mask(false, false, 0x100), 0x40);
    assert_eq!(result.missing_target_mask(false, true, 0x100), 0);
    assert_eq!(result.missing_target_mask(false, false, 0x8000), 0x40);
}

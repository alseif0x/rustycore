use super::*;
use wow_entities::{AuraSubsystem, VisibleAuraApplicationLikeCpp, VisibleAuraEffectAmountLikeCpp};

#[test]
fn raw_slot_matches_spell_and_caster_across_slots_and_preserves_mask_order() {
    let caster = ObjectGuid::create_player(1, 77);
    let other = ObjectGuid::create_player(1, 78);
    let mut auras = AuraSubsystem::default();
    auras.visible_auras.insert(3, AuraRef::new(822, caster));
    auras.applied_auras = vec![
        AppliedAuraRef::new(822, caster, 9, 4),
        AppliedAuraRef::new(823, caster, 3, 8),
        AppliedAuraRef::new(822, other, 3, 16),
        AppliedAuraRef::new(822, caster, 3, 1),
        AppliedAuraRef::new(822, caster, 8, 0x8000_0000),
    ];
    let slot = CreatureAuraSlotFacts::capture(&auras, 3).unwrap();
    assert_eq!(slot.slot(), 3);
    assert_eq!(slot.aura_ref(), AuraRef::new(822, caster));
    assert_eq!(slot.matching_effect_masks(), &[4, 1, 0x8000_0000]);
    assert!(slot.application().is_none());
}

#[test]
fn raw_application_and_provenance_survive_source_replacement_without_packet_calculation() {
    let mut auras = AuraSubsystem::default();
    let caster = ObjectGuid::create_player(1, 77);
    auras.visible_auras.insert(7, AuraRef::new(u32::MAX, caster));
    let amounts = vec![
        VisibleAuraEffectAmountLikeCpp { effect_index: 32, amount: i32::MIN },
        VisibleAuraEffectAmountLikeCpp { effect_index: 1, amount: i32::MAX },
    ];
    auras.visible_aura_applications_like_cpp.insert(7,
        VisibleAuraApplicationLikeCpp::new(u32::MAX, amounts.clone()));
    let provenance = AuraCastProvenanceLikeCpp {
        cast_id: ObjectGuid::new(19, 88), spell_visual_id: -7,
    };
    auras.set_aura_cast_provenance_like_cpp(7, provenance);
    let slot = CreatureAuraSlotFacts::capture(&auras, 7).unwrap();
    auras.visible_auras.clear();
    auras.visible_aura_applications_like_cpp.clear();
    auras.set_aura_cast_provenance_like_cpp(7, AuraCastProvenanceLikeCpp::default());
    assert_eq!(slot.aura_ref().spell_id, u32::MAX);
    assert_eq!(slot.application().unwrap().flags, u32::MAX);
    assert_eq!(slot.application().unwrap().effect_amounts, amounts);
    assert_eq!(slot.provenance(), provenance);
}

#[test]
fn missing_slot_and_default_provenance_keep_distinct_meanings() {
    let mut auras = AuraSubsystem::default();
    assert!(CreatureAuraSlotFacts::capture(&auras, 1).is_none());
    auras.visible_auras.insert(1, AuraRef::new(822, ObjectGuid::EMPTY));
    let slot = CreatureAuraSlotFacts::capture(&auras, 1).unwrap();
    assert_eq!(slot.provenance(), AuraCastProvenanceLikeCpp::default());
    assert_eq!(slot.aura_ref().caster_guid, ObjectGuid::EMPTY);
    assert!(slot.matching_effect_masks().is_empty());
}

#[test]
fn initial_aura_capture_contains_only_visible_slots_and_keeps_original_level() {
    let mut source = legacy(101);
    let guid = source.guid();
    let auras = &mut source.creature.unit_mut().subsystems_mut().auras;
    auras.visible_auras.insert(9, AuraRef::new(822, guid));
    auras.visible_auras.insert(2, AuraRef::new(823, guid));
    auras.applied_auras.push(AppliedAuraRef::new(824, guid, 5, 1));
    let captured = source.capture_visibility_candidate();
    source.creature.unit_mut().set_level(80);
    source.creature.unit_mut().subsystems_mut().auras.visible_auras.clear();
    assert_eq!(captured.initial_auras().guid(), guid);
    assert_eq!(captured.initial_auras().level(), 12);
    let mut slots: Vec<_> = captured.initial_auras().slots().iter().map(|slot| slot.slot()).collect();
    slots.sort_unstable();
    assert_eq!(slots, vec![2, 9]);
}

#[test]
fn empty_initial_aura_snapshot_does_not_gain_later_slots() {
    let mut source = legacy(102);
    let captured = source.capture_visibility_candidate();
    source.creature.unit_mut().subsystems_mut().auras.visible_auras
        .insert(1, AuraRef::new(822, ObjectGuid::EMPTY));
    assert!(captured.initial_auras().slots().is_empty());
}

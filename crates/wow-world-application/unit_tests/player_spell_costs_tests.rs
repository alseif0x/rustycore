use super::*;

use std::collections::BTreeSet;

fn item_effect(
    id: u32,
    trigger_type: wow_constants::ItemSpelltriggerType,
    spell_id: i32,
    parent_item_id: u32,
) -> wow_data::ItemEffectEntry {
    wow_data::ItemEffectEntry {
        id,
        legacy_slot_index: u8::try_from(4 - id).expect("test effect slot fits u8"),
        trigger_type: trigger_type as i8,
        charges: 0,
        cooldown_msec: 0,
        category_cooldown_msec: 0,
        spell_category_id: 0,
        spell_id,
        chr_specialization_id: 0,
        parent_item_id,
    }
}

#[test]
fn item_removal_selects_source_on_pickup_spells_and_removes_each_aura_in_order() {
    let effects = wow_data::ItemEffectStore::from_entries([
        item_effect(4, wow_constants::ItemSpelltriggerType::OnPickup, 702, 42),
        item_effect(2, wow_constants::ItemSpelltriggerType::OnUse, 701, 42),
        item_effect(1, wow_constants::ItemSpelltriggerType::OnPickup, 700, 42),
        item_effect(3, wow_constants::ItemSpelltriggerType::OnPickup, -1, 42),
    ]);
    let spell_ids = item_obtain_spell_ids_for_removal_like_cpp(
        42,
        Some(wow_constants::ItemFlags::empty()),
        Some(&effects),
    )
    .expect("complete item metadata resolves removal auras");
    assert_eq!(spell_ids, [700, 702]);

    let mut active_auras = BTreeSet::from([700, 702, 900]);
    let mut removed = Vec::new();
    let removed_count = remove_item_obtain_aura_ids_like_cpp(spell_ids, |spell_id| {
        removed.push(spell_id);
        if active_auras.remove(&spell_id) { 1 } else { 0 }
    });
    assert_eq!(removed, [700, 702]);
    assert_eq!(removed_count, 2);
    assert_eq!(active_auras, BTreeSet::from([900]));
}

#[test]
fn item_removal_obeys_legacy_and_missing_metadata_source_boundaries() {
    let effects = wow_data::ItemEffectStore::from_entries([item_effect(
        1,
        wow_constants::ItemSpelltriggerType::OnPickup,
        700,
        42,
    )]);
    assert_eq!(
        item_obtain_spell_ids_for_removal_like_cpp(
            42,
            Some(wow_constants::ItemFlags::LEGACY),
            None,
        )
        .expect("Source checks LEGACY before item effects"),
        Vec::<i32>::new()
    );
    assert!(item_obtain_spell_ids_for_removal_like_cpp(42, None, Some(&effects)).is_err());
    assert!(
        item_obtain_spell_ids_for_removal_like_cpp(
            42,
            Some(wow_constants::ItemFlags::empty()),
            None,
        )
        .is_err()
    );
}

#[test]
fn item_removal_uses_first_on_looted_record_and_source_legacy_gate() {
    let effects = wow_data::ItemEffectStore::from_entries([
        item_effect(3, wow_constants::ItemSpelltriggerType::OnLooted, 803, 42),
        item_effect(1, wow_constants::ItemSpelltriggerType::OnLooted, 801, 42),
        item_effect(2, wow_constants::ItemSpelltriggerType::OnPickup, 802, 42),
    ]);
    assert_eq!(
        item_looted_spell_id_for_removal_like_cpp(
            42,
            Some(wow_constants::ItemFlags::empty()),
            Some(&effects),
        )
        .expect("complete nonlegacy item metadata resolves OnLooted"),
        Some(801),
        "the first OnLooted record is selected in ascending record ID order"
    );
    assert_eq!(
        item_looted_spell_id_for_removal_like_cpp(
            42,
            Some(wow_constants::ItemFlags::LEGACY),
            None,
        )
        .expect("Source skips Legacy before item-effect lookup"),
        None
    );
    assert!(item_looted_spell_id_for_removal_like_cpp(42, None, Some(&effects)).is_err());
    assert!(
        item_looted_spell_id_for_removal_like_cpp(
            42,
            Some(wow_constants::ItemFlags::empty()),
            None,
        )
        .is_err()
    );
}

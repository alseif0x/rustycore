use super::*;

#[test]
fn represented_item_set_skips_unknown_spell_info_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 920);
    let hands_guid = ObjectGuid::create_item(1, 921);
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        9041,
        SpellInfo {
            spell_id: 9041,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );

    session.set_player_guid(Some(player_guid));
    session.set_spell_store(Arc::new(spell_store));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 710,
        name: "Unknown Spell Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 114,
            1 => 115,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 34,
            chr_spec_id: 0,
            spell_id: 9040,
            threshold: 2,
            item_set_id: 710,
        },
        ItemSetSpellEntry {
            id: 35,
            chr_spec_id: 0,
            spell_id: 9041,
            threshold: 2,
            item_set_id: 710,
        },
    ])));

    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        114,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        115,
        InventoryType::Hands,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 710,
            spell_entry_id: 35,
            spell_id: 9041,
            threshold: 2,
            apply: true,
        }],
        "C++ AddItemsSetItem logs and continues before SetBonuses.insert when sSpellMgr has no SpellInfo"
    );
    assert_eq!(
        session
            .represented_item_set_effect_like_cpp(710)
            .expect("known spell still creates represented set effect")
            .set_bonuses,
        BTreeSet::from([35])
    );
}
#[test]
fn represented_update_item_set_auras_replays_active_bonuses_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 915);
    let hands_guid = ObjectGuid::create_item(1, 916);
    session.set_player_guid(Some(player_guid));
    session.set_represented_primary_specialization_id_like_cpp(66);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 707,
        name: "Refresh Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 109,
            1 => 110,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 30,
            chr_spec_id: 65,
            spell_id: 9030,
            threshold: 2,
            item_set_id: 707,
        },
        ItemSetSpellEntry {
            id: 31,
            chr_spec_id: 66,
            spell_id: 9031,
            threshold: 2,
            item_set_id: 707,
        },
    ])));

    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        109,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        110,
        InventoryType::Hands,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 707,
            spell_entry_id: 31,
            spell_id: 9031,
            threshold: 2,
            apply: true,
        }],
        "C++ AddItemsSetItem stores all threshold-met set bonuses but only casts the current-spec spell"
    );
    assert_eq!(
        session
            .represented_item_set_effect_like_cpp(707)
            .expect("set effect")
            .set_bonuses,
        BTreeSet::from([30, 31])
    );

    session.set_represented_primary_specialization_id_like_cpp(65);
    assert_eq!(
        session.record_represented_update_item_set_auras_like_cpp(true),
        2
    );
    assert_eq!(
        session.represented_item_set_aura_refresh_events_like_cpp(),
        &[
            RepresentedItemSetAuraRefreshEventLikeCpp {
                item_set_id: 707,
                spell_entry_id: 30,
                spell_id: 9030,
                apply: true,
                form_change: true,
            },
            RepresentedItemSetAuraRefreshEventLikeCpp {
                item_set_id: 707,
                spell_entry_id: 31,
                spell_id: 9031,
                apply: false,
                form_change: false,
            },
        ],
        "C++ ApplyEquipSpell(false, formChange=true) skips removal when the spell still fits the current shapeshift"
    );
}
#[test]
fn represented_update_item_set_auras_skips_apply_when_shapeshift_rejected_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 922);
    let mut spell_store = SpellStore::new();
    spell_store.insert(9042, test_spell_info_like_cpp(9042));
    spell_store.insert_spell_shapeshift_masks_like_cpp(9042, 1 << 4, 0);

    session.set_player_guid(Some(player_guid));
    session.set_spell_store(Arc::new(spell_store));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 711,
        name: "Form Restricted Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { 116 } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 36,
            chr_spec_id: 0,
            spell_id: 9042,
            threshold: 1,
            item_set_id: 711,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        116,
        InventoryType::Chest,
    );

    assert!(session.record_represented_items_set_item_like_cpp(item_guid, true));
    assert_eq!(
        session.record_represented_update_item_set_auras_like_cpp(false),
        1
    );
    assert_eq!(
        session.represented_item_set_aura_refresh_events_like_cpp(),
        &[RepresentedItemSetAuraRefreshEventLikeCpp {
            item_set_id: 711,
            spell_entry_id: 36,
            spell_id: 9042,
            apply: false,
            form_change: false,
        }],
        "C++ ApplyEquipSpell(true) returns without casting when CheckShapeshift is not OK"
    );
}
#[test]
fn represented_update_item_set_auras_form_change_skips_remove_when_form_still_fits_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 923);
    let mut spell_store = SpellStore::new();
    spell_store.insert(9043, test_spell_info_like_cpp(9043));
    spell_store.insert_spell_shapeshift_masks_like_cpp(9043, 1 << 4, 0);

    session.set_player_guid(Some(player_guid));
    session.set_spell_store(Arc::new(spell_store));
    session.set_represented_shapeshift_form_like_cpp(5);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 712,
        name: "Matching Form Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { 117 } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 37,
            chr_spec_id: 0,
            spell_id: 9043,
            threshold: 1,
            item_set_id: 712,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        117,
        InventoryType::Chest,
    );

    assert!(session.record_represented_items_set_item_like_cpp(item_guid, true));
    assert_eq!(
        session.record_represented_update_item_set_auras_like_cpp(true),
        1
    );
    assert_eq!(
        session.represented_item_set_aura_refresh_events_like_cpp(),
        &[RepresentedItemSetAuraRefreshEventLikeCpp {
            item_set_id: 712,
            spell_entry_id: 37,
            spell_id: 9043,
            apply: true,
            form_change: true,
        }],
        "C++ ApplyEquipSpell(false, formChange=true) returns early when CheckShapeshift is OK"
    );
}

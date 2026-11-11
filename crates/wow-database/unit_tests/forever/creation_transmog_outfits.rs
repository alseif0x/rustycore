use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{
    NewSavedEquipmentSet, NewSavedSetKind, NewSavedSetState,
};

fn outfit(name: &str) -> NewTransmogOutfit {
    NewTransmogOutfit {
        name: name.into(),
        icon: 7,
        situations_enabled: false,
        situations: Vec::new(),
        slots: Vec::new(),
    }
}

fn situation(id: u32) -> NewTransmogOutfitSituation {
    NewTransmogOutfitSituation {
        situation_id: id,
        spec_id: 1,
        loadout_id: 2,
        equipment_set_id: 3,
    }
}

fn slot(index: i8) -> NewTransmogOutfitSlot {
    NewTransmogOutfitSlot {
        slot: index,
        slot_option: 0,
        sheathe_category: 2,
        item_modified_appearance_id: 17,
        appearance_display_type: 0,
        spell_item_enchantment_id: 19,
        illusion_display_type: 0,
        flags: 0,
    }
}

#[test]
fn empty_dirty_map_is_noop_and_missing_lookup_emits_only_three_scoped_deletes() {
    assert_eq!(statements(17, &BTreeMap::new()).count(), 0);
    for guid in [17, u64::MAX] {
        for id in [0, 23, u32::MAX] {
            let data = BTreeMap::from([(id, None)]);
            let rows: Vec<_> = statements(guid, &data).collect();
            assert_eq!(
                rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
                [
                    "DELETE FROM character_transmog_outfit_slot WHERE guid = ? AND transmogOutfitId = ?",
                    "DELETE FROM character_transmog_outfit_situation WHERE guid = ? AND transmogOutfitId = ?",
                    "DELETE FROM character_transmog_outfit WHERE guid = ? AND transmogOutfitId = ?",
                ]
            );
            for row in rows {
                assert_eq!(row.params(), &[SqlParam::U64(guid), SqlParam::U32(id)]);
            }
        }
    }
}

#[test]
fn outfit_parent_binds_dirty_id_unicode_name_icon_and_both_bool_values() {
    // Scalar bounds check setters, not admission into the source's signed columns.
    for enabled in [false, true] {
        let mut data = outfit("Élan 装い🌙");
        data.icon = u32::MAX;
        data.situations_enabled = enabled;
        let row = outfit_statement(u64::MAX, u32::MAX, &data);
        assert_eq!(
            row.sql(),
            "INSERT INTO character_transmog_outfit (guid, transmogOutfitId, name, icon, situationsEnabled) VALUES (?, ?, ?, ?, ?)"
        );
        assert_eq!(row.sql().matches('?').count(), 5);
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U32(u32::MAX),
                SqlParam::String("Élan 装い🌙".into()),
                SqlParam::U32(u32::MAX),
                SqlParam::Bool(enabled),
            ]
        );
    }
}

#[test]
fn situation_and_slot_binding_bounds_preserve_all_six_and_ten_source_fields() {
    // Encoding limits only, not canonical outfit/slot admission proof.
    let data = NewTransmogOutfitSituation {
        situation_id: u32::MAX,
        spec_id: 0x8000_0000,
        loadout_id: 7,
        equipment_set_id: 8,
    };
    let row = situation_statement(u64::MAX, u32::MAX, &data);
    assert_eq!(
        row.sql(),
        "INSERT INTO character_transmog_outfit_situation (guid, transmogOutfitId, situationID, specID, loadoutID, equipmentSetID) VALUES (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(row.sql().matches('?').count(), 6);
    assert_eq!(
        row.params(),
        &[
            SqlParam::U64(u64::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::U32(u32::MAX),
            SqlParam::U32(0x8000_0000),
            SqlParam::U32(7),
            SqlParam::U32(8),
        ]
    );
    for index in [i8::MIN, i8::MAX] {
        let data = NewTransmogOutfitSlot {
            slot: index,
            slot_option: u8::MAX,
            sheathe_category: u8::MAX - 1,
            item_modified_appearance_id: u32::MAX,
            appearance_display_type: 0x80,
            spell_item_enchantment_id: u32::MAX - 1,
            illusion_display_type: 0x7F,
            flags: 0x8000_0001,
        };
        let row = slot_statement(u64::MAX, u32::MAX, &data);
        assert_eq!(
            row.sql(),
            "INSERT INTO character_transmog_outfit_slot (guid, transmogOutfitId, slot, slotOption, sheatheCategory, itemModifiedAppearanceID, appearanceDisplayType, spellItemEnchantmentID, illusionDisplayType, flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        );
        assert_eq!(row.sql().matches('?').count(), 10);
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U32(u32::MAX),
                SqlParam::I8(index),
                SqlParam::U8(u8::MAX),
                SqlParam::U8(u8::MAX - 1),
                SqlParam::U32(u32::MAX),
                SqlParam::U8(0x80),
                SqlParam::U32(u32::MAX - 1),
                SqlParam::U8(0x7F),
                SqlParam::U32(0x8000_0001),
            ]
        );
    }
}

#[test]
fn present_outfit_writes_parent_then_situations_then_slots_in_unique_supplied_vector_order() {
    let mut data = outfit("Synthetic");
    data.situations = vec![situation(90), situation(2)];
    data.slots = vec![slot(2), slot(0)];
    let rows = outfit_statements(31, 7, Some(&data));
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            SLOT_DELETE,
            SITUATION_DELETE,
            OUTFIT_DELETE,
            OUTFIT_INSERT,
            SITUATION_INSERT,
            SITUATION_INSERT,
            SLOT_INSERT,
            SLOT_INSERT,
        ]
    );
    for row in &rows {
        assert_eq!(&row.params()[..2], &[SqlParam::U64(31), SqlParam::U32(7)]);
    }
    assert_eq!(rows[3].params()[4], SqlParam::Bool(false));
    assert_eq!(rows[4].params()[2], SqlParam::U32(90));
    assert_eq!(rows[5].params()[2], SqlParam::U32(2));
    assert_eq!(rows[6].params()[2], SqlParam::I8(2));
    assert_eq!(rows[7].params()[2], SqlParam::I8(0));
}

#[test]
fn dirty_keys_are_ascending_including_explicit_zero_without_clearing_frozen_snapshots() {
    let mut data = BTreeMap::new();
    data.insert(90, Some(outfit("third")));
    data.insert(2, None);
    data.insert(0, Some(outfit("zero")));
    data.insert(7, Some(outfit("second")));
    let rows: Vec<_> = statements(37, &data).collect();
    assert_eq!(rows.len(), 15);
    for (row, id) in rows
        .iter()
        .zip([0, 0, 0, 0, 2, 2, 2, 7, 7, 7, 7, 90, 90, 90, 90])
    {
        assert_eq!(&row.params()[..2], &[SqlParam::U64(37), SqlParam::U32(id)]);
    }
    for (index, name) in [(3, "zero"), (10, "second"), (14, "third")] {
        assert_eq!(rows[index].sql(), OUTFIT_INSERT);
        assert_eq!(rows[index].params()[2], SqlParam::String(name.into()));
    }
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    append(&mut transaction, 37, &data);
    assert_eq!(
        transaction.sqls_for_test(),
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
    );
    // Borrowed DTO preservation is not canonical dirty-flag acknowledgement.
    assert_eq!(data.len(), 4);
    assert!(data[&2].is_none());
    assert!(!data[&0].as_ref().unwrap().situations_enabled);
}

#[test]
fn existing_through_cache_orders_equipment_then_modern_outfits_then_visual_cache() {
    let mut character = super::super::tests::character();
    character.equipment_sets.insert(
        2,
        NewSavedEquipmentSet {
            guid: 77,
            set_index: 1,
            name: "Synthetic set".into(),
            icon: String::new(),
            ignore_mask: 0,
            kind: NewSavedSetKind::Equipment {
                assigned_spec: None,
                items: [0; 19],
            },
            state: NewSavedSetState::New,
        },
    );
    let mut present = outfit("Synthetic outfit");
    present.situations = vec![situation(1)];
    present.slots = vec![slot(0)];
    character.transmog_outfits.insert(7, Some(present));
    character.transmog_outfits.insert(9, None);
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let equipment = sqls
        .iter()
        .position(|sql| sql.starts_with("INSERT INTO character_equipmentsets ("))
        .unwrap();
    let cache = super::super::visual_statement(character.guid, &character.visual_items);
    assert_eq!(
        &sqls[equipment + 1..],
        &[
            SLOT_DELETE,
            SITUATION_DELETE,
            OUTFIT_DELETE,
            OUTFIT_INSERT,
            SITUATION_INSERT,
            SLOT_INSERT,
            SLOT_DELETE,
            SITUATION_DELETE,
            OUTFIT_DELETE,
            "DELETE FROM character_select_screen_equipment_cache WHERE guid = ?",
            cache.sql(),
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?",
            "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?",
        ]
    );
}

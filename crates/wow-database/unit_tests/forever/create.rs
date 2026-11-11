use super::{
    action_statement, append_customizations, append_spells, append_visual_cache,
    character_statement, customization_insert_statement, customizations_delete_statement, next,
    spell_favorite_delete_statement, spell_favorite_insert_statement, spell_insert_statement,
    visual_cache_delete_statement, visual_statement,
};
use crate::{SqlParam, SqlTransaction};
use wow_persistence::forever::creation::save::{
    CreateError, NewAchievementSave, NewAction, NewBattlegroundData, NewCharacter,
    NewCharacterSessionSave, NewCharacterState, NewCustomization, NewQuestCriteriaSave,
    NewQuestSave, NewSpecializations, NewSpell, NewSpellHistory, NewTaxiPath, NewTutorialSave,
    NewVisualItem,
};

#[test]
fn action_save_keeps_full_action_width_and_the_selected_trait_configuration() {
    let statement = action_statement(
        17,
        &NewAction {
            spec: 2,
            trait_config: -19,
            button: u8::MAX,
            action: 0x0000_FFFF_FFFF_FFFF,
            kind: 128,
        },
    );
    assert_eq!(
        statement.params(),
        &[
            SqlParam::U64(17),
            SqlParam::U8(2),
            SqlParam::I32(-19),
            SqlParam::U8(255),
            SqlParam::U64(0x0000_FFFF_FFFF_FFFF),
            SqlParam::U8(128),
        ]
    );
}

#[test]
fn customizations_delete_even_when_empty_then_insert_choices_in_source_order() {
    let mut transaction = SqlTransaction::new();
    append_customizations(&mut transaction, 17, &[]);
    assert_eq!(
        transaction.sqls_for_test(),
        ["DELETE FROM character_customizations WHERE guid = ?"]
    );
    assert_eq!(
        customizations_delete_statement(17).params(),
        &[SqlParam::U64(17)]
    );

    let customizations = [
        NewCustomization {
            option: 101,
            choice: 201,
        },
        NewCustomization {
            option: 102,
            choice: 202,
        },
    ];
    let mut transaction = SqlTransaction::new();
    append_customizations(&mut transaction, 19, &customizations);
    assert_eq!(
        transaction.sqls_for_test(),
        [
            "DELETE FROM character_customizations WHERE guid = ?",
            "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)",
            "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)",
        ]
    );
    for (customization, (expected_option, expected_choice)) in
        customizations.iter().zip([(101, 201), (102, 202)])
    {
        assert_eq!(
            customization_insert_statement(19, customization).params(),
            &[
                SqlParam::U64(19),
                SqlParam::U32(expected_option),
                SqlParam::U32(expected_choice),
            ]
        );
    }
}

#[test]
fn every_spell_deletes_favorite_after_optional_spell_insert_and_before_optional_favorite_insert() {
    for dependent in [false, true] {
        for favorite in [false, true] {
            let mut transaction = SqlTransaction::new();
            append_spells(
                &mut transaction,
                17,
                &[NewSpell {
                    spell: 42,
                    active: true,
                    disabled: false,
                    dependent,
                    favorite,
                }],
            );
            let sqls = transaction.sqls_for_test();
            let mut expected = Vec::new();
            if !dependent {
                expected.push(
                    "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)",
                );
            }
            expected.push("DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?");
            if favorite {
                expected.push("INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)");
            }
            assert_eq!(sqls, expected);
            assert_eq!(
                spell_favorite_delete_statement(17, 42).params(),
                &[SqlParam::U64(17), SqlParam::U32(42)]
            );
            if !dependent {
                assert_eq!(
                    spell_insert_statement(
                        17,
                        &NewSpell {
                            spell: 42,
                            active: true,
                            disabled: false,
                            dependent,
                            favorite,
                        },
                    )
                    .params(),
                    &[
                        SqlParam::U64(17),
                        SqlParam::U32(42),
                        SqlParam::Bool(true),
                        SqlParam::Bool(false),
                    ]
                );
            }
            if favorite {
                assert_eq!(
                    spell_favorite_insert_statement(17, 42).params(),
                    &[SqlParam::U64(17), SqlParam::U32(42)]
                );
            }
        }
    }
}

#[test]
fn multiple_spells_keep_each_favorite_delete_scoped_and_ordered_with_its_spell() {
    let spells = [
        NewSpell {
            spell: 100,
            active: true,
            disabled: false,
            dependent: false,
            favorite: false,
        },
        NewSpell {
            spell: 200,
            active: false,
            disabled: true,
            dependent: true,
            favorite: true,
        },
        NewSpell {
            spell: 300,
            active: true,
            disabled: true,
            dependent: false,
            favorite: true,
        },
    ];
    let mut transaction = SqlTransaction::new();
    append_spells(&mut transaction, 29, &spells);
    assert_eq!(
        transaction.sqls_for_test(),
        [
            "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)",
            "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?",
            "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?",
            "INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)",
            "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)",
            "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?",
            "INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)",
        ]
    );
    for spell in &spells {
        assert_eq!(
            spell_favorite_delete_statement(29, spell.spell).params(),
            &[SqlParam::U64(29), SqlParam::U32(spell.spell)]
        );
    }
}

pub(super) fn character() -> NewCharacter {
    NewCharacter {
        guid: 9,
        account: 7,
        realm: 0x02010001,
        name: "Synthetic".into(),
        surname: String::new(),
        race: 1,
        class: 1,
        gender: 0,
        level: 3,
        experience: 42,
        money: 43,
        map: 48,
        position: [1.0, 2.0, 3.0, 4.0],
        create_time: 17,
        create_mode: 0,
        cinematic: 1,
        at_login: 32,
        health: 123,
        powers: [11, 22, 33, 44, 55, 66, 77, 88, 99, 111],
        latency: 45,
        primary_specialization: 46,
        active_talent_group: 1,
        client_version: Some([1, 60, 1]),
        state: NewCharacterState {
            inventory_slots: 19,
            inventory_bag_flags: 0x80000000,
            bag_slot_flags: [101, 102, 103, 104, 105],
            bank_slots: 7,
            bank_tabs: 8,
            bank_bag_flags: 9,
            rest_state: 2,
            player_flags: 17,
            player_flags_ex: 18,
            instance_id: 19,
            dungeon_difficulty: 1,
            raid_difficulty: 14,
            legacy_raid_difficulty: 3,
            transport_position: [5.0, 6.0, 7.0, 8.0],
            transport_guid: 12345,
            taxi_mask: vec![0, 128, 255],
            total_played_time: 21,
            level_played_time: 22,
            rest_bonus: 1.5,
            save_time: 500,
            logout_resting: true,
            talent_reset_cost: 32,
            talent_reset_time: -12,
            extra_flags: 0x8000,
            death_expire_time: -10,
            taxi_path: Some(NewTaxiPath {
                flight_master_faction: 123,
                nodes: vec![256, 257],
            }),
            lifetime_honorable_kills: 42,
            today_honorable_kills: 43,
            yesterday_honorable_kills: 44,
            chosen_title: 45,
            watched_faction: u32::MAX,
            drunk: 46,
            loot_specialization: 47,
            explored_zones: vec![0x89ABCDEF01234567, u64::MAX],
            known_titles: vec![0x8000000000000001],
            action_bars: 4,
            personal_tabard: [-1, -2, -3, -4, -5],
            transmog_outfit: -2,
            transmog_locked: true,
        },
        fishing_steps: 0,
        battleground: NewBattlegroundData {
            instance: 0,
            team: 0,
            position: [0.0; 4],
            map: u16::MAX,
            taxi: [0; 2],
            mount_spell: 0,
            queue_id: 0,
        },
        quests: NewQuestSave {
            keep_abandoned: true,
            statuses: std::collections::BTreeMap::new(),
            rewarded: std::collections::BTreeMap::new(),
            daily: None,
            weekly: None,
            seasonal: None,
            monthly: None,
        },
        specializations: NewSpecializations {
            glyphs: std::array::from_fn(|_| Vec::new()),
            talents: std::array::from_fn(|_| Vec::new()),
            pvp: [[0; 4]; 5],
        },
        traits: Vec::new(),
        spell_history: NewSpellHistory {
            cooldowns: Vec::new(),
            charges: Vec::new(),
        },
        auras: Vec::new(),
        stored_aura_locations: Vec::new(),
        achievements: NewAchievementSave {
            completed: Vec::new(),
            progress: Vec::new(),
        },
        reputation: std::collections::BTreeMap::new(),
        quest_criteria: NewQuestCriteriaSave {
            completed_objectives: Vec::new(),
            progress: Vec::new(),
        },
        equipment_sets: std::collections::BTreeMap::new(),
        transmog_outfits: std::collections::BTreeMap::new(),
        session_save: NewCharacterSessionSave {
            tutorials: NewTutorialSave::Unchanged,
            instance_restrictions: Vec::new(),
        },
        currencies: Vec::new(),
        cuf_profiles: std::array::from_fn(|_| None),
        player_data: wow_persistence::forever::creation::save::NewPlayerDataSave {
            elements: std::collections::BTreeMap::new(),
            flags: std::collections::BTreeMap::new(),
        },
        bank_tabs: Vec::new(),
        garrison: None,
        statistics: None,
        mail: wow_persistence::forever::creation::save::NewMailSave {
            updated: false,
            messages: Vec::new(),
        },
        customizations: vec![],
        actions: vec![],
        skills: vec![],
        spells: vec![],
        items: vec![],
        visual_items: [NewVisualItem::default(); 19],
    }
}

#[test]
fn character_insert_binds_values_to_their_target_columns_without_a_bank_offset() {
    let character = character();
    let statement = character_statement(9, &character);
    let columns = statement
        .sql()
        .split_once('(')
        .unwrap()
        .1
        .split_once(')')
        .unwrap()
        .0;
    let columns: Vec<_> = columns.split(',').map(str::trim).collect();
    assert_eq!(columns.len(), 84);
    assert_eq!(statement.params().len(), columns.len());
    assert_eq!(statement.sql().matches('?').count(), columns.len());
    let bindings: std::collections::BTreeMap<_, _> =
        columns.into_iter().zip(statement.params()).collect();
    for (column, expected) in [
        ("guid", SqlParam::U64(9)),
        ("account", SqlParam::U32(7)),
        ("inventorySlots", SqlParam::U8(19)),
        ("inventoryBagFlags", SqlParam::U32(0x80000000)),
        ("bagSlotFlags1", SqlParam::U32(101)),
        ("bagSlotFlags5", SqlParam::U32(105)),
        ("bankSlots", SqlParam::U8(7)),
        ("bankTabs", SqlParam::U8(8)),
        ("bankBagFlags", SqlParam::U32(9)),
        ("restState", SqlParam::U8(2)),
        ("playerFlags", SqlParam::U32(17)),
        ("playerFlagsEx", SqlParam::U32(18)),
        ("map", SqlParam::U16(48)),
        ("instance_id", SqlParam::U32(19)),
        ("dungeonDifficulty", SqlParam::U8(1)),
        ("raidDifficulty", SqlParam::U8(14)),
        ("legacyRaidDifficulty", SqlParam::U8(3)),
        ("position_x", SqlParam::F32(1.0)),
        ("orientation", SqlParam::F32(4.0)),
        ("trans_x", SqlParam::F32(5.0)),
        ("trans_o", SqlParam::F32(8.0)),
        ("transguid", SqlParam::U64(12345)),
        ("taximask", SqlParam::String("0 128 255 ".into())),
        ("createTime", SqlParam::I64(17)),
        ("totaltime", SqlParam::U32(21)),
        ("leveltime", SqlParam::U32(22)),
        ("rest_bonus", SqlParam::F32(1.5)),
        ("logout_time", SqlParam::U64(500)),
        ("is_logout_resting", SqlParam::U8(1)),
        ("resettalents_cost", SqlParam::U32(32)),
        ("resettalents_time", SqlParam::I64(-12)),
        ("extra_flags", SqlParam::U16(0x8000)),
        ("death_expire_time", SqlParam::I64(-10)),
        ("taxi_path", SqlParam::String("123 256 257 ".into())),
        ("totalKills", SqlParam::U32(42)),
        ("todayKills", SqlParam::U16(43)),
        ("yesterdayKills", SqlParam::U16(44)),
        ("chosenTitle", SqlParam::U32(45)),
        ("watchedFaction", SqlParam::U32(u32::MAX)),
        ("drunk", SqlParam::U8(46)),
        ("health", SqlParam::U32(123)),
        ("power1", SqlParam::U32(11)),
        ("power10", SqlParam::U32(111)),
        ("latency", SqlParam::U32(45)),
        ("lootSpecId", SqlParam::U32(47)),
        (
            "exploredZones",
            SqlParam::String("19088743 2309737967 4294967295 4294967295 ".into()),
        ),
        ("knownTitles", SqlParam::String("1 2147483648 ".into())),
        ("actionBars", SqlParam::U8(4)),
        ("lastLoginBuild", SqlParam::U32(16001)),
        ("personalTabardEmblemStyle", SqlParam::I32(-1)),
        ("personalTabardBackgroundColor", SqlParam::I32(-5)),
        ("transmogOutfitEquippedId", SqlParam::I32(-2)),
        ("transmogOutfitLocked", SqlParam::Bool(true)),
    ] {
        assert_eq!(bindings[column], &expected, "{column}");
    }
}

#[test]
fn primary_row_sanitizes_only_nonfinite_floats_and_keeps_the_other_values() {
    let mut character = character();
    character.position = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.0];
    character.state.transport_position = [f32::NEG_INFINITY, 1.25, -3.5, f32::NAN];
    character.state.rest_bonus = f32::NAN;
    let statement = character_statement(9, &character);
    for (index, expected) in [
        (27, 0.0_f32),
        (28, 0.0),
        (29, 0.0),
        (30, -0.0),
        (31, 0.0),
        (32, 1.25),
        (33, -3.5),
        (34, 0.0),
        (42, 0.0),
    ] {
        let SqlParam::F32(value) = statement.params()[index] else {
            panic!("float binding")
        };
        assert_eq!(value.to_bits(), expected.to_bits(), "column {index}");
    }
}

#[test]
fn absent_realm_and_empty_persisted_vectors_keep_the_source_representation() {
    let mut character = character();
    character.client_version = None;
    character.state.taxi_mask.clear();
    character.state.taxi_path = None;
    character.state.explored_zones.clear();
    character.state.known_titles.clear();
    let statement = character_statement(9, &character);
    for index in [36, 52, 73, 74] {
        assert_eq!(statement.params()[index], SqlParam::String(String::new()));
    }
    assert_eq!(statement.params()[76], SqlParam::U32(0));
}

#[test]
fn realm_version_components_keep_the_source_uint32_width() {
    let mut character = character();
    character.client_version = Some([256, 257, 258]);
    assert_eq!(
        character_statement(9, &character).params()[76],
        SqlParam::U32(2585958)
    );
    character.client_version = Some([u32::MAX; 3]);
    // (2^32 - 1) * (10000 + 100 + 1), modulo uint32.
    assert_eq!(
        character_statement(9, &character).params()[76],
        SqlParam::U32(u32::MAX - 10100)
    );
}

#[test]
fn guid_allocation_starts_after_persisted_maximum_and_retains_its_owner() {
    for (maximum, first) in [(None, 1), (Some(0), 1), (Some(41), 42)] {
        let mut counter = None;
        assert_eq!(next(&mut counter, maximum), Ok(first));
        assert_eq!(next(&mut counter, Some(900)), Ok(first + 1));
        assert_eq!(counter, Some(first + 2));
    }
}

#[test]
fn guid_exhaustion_cannot_wrap_or_restart_from_a_cached_counter() {
    // ObjectGuid.h::GetMaxCounter and CheckGeneratedGuidValue at 02245dcd.
    let cutoff = 0xFFFFFFFFFF - 1;
    let mut counter = Some(cutoff - 1);
    assert_eq!(next(&mut counter, None), Ok(cutoff - 1));
    for _ in 0..2 {
        assert_eq!(next(&mut counter, None), Err(CreateError::InvalidRow));
        assert_eq!(counter, Some(cutoff));
    }
    for maximum in [cutoff - 1, cutoff, u64::MAX] {
        let mut counter = None;
        assert_eq!(
            next(&mut counter, Some(maximum)),
            Err(CreateError::InvalidRow)
        );
        assert_eq!(counter, None);
    }
    let mut counter = Some(0);
    assert_eq!(next(&mut counter, None), Err(CreateError::InvalidRow));
    assert_eq!(counter, Some(0));
}

#[test]
fn equipment_cache_preserves_first_and_last_slot_bindings_and_signed_appearance() {
    let mut items = [NewVisualItem::default(); 19];
    items[0].item_id = 42;
    items[18].secondary_appearance_id = -1;
    items[18].sheathe_category = 7;
    let statement = visual_statement(23, &items);
    assert_eq!(statement.params().len(), 153);
    assert_eq!(statement.sql().matches('?').count(), 153);
    assert!(
        statement
            .sql()
            .contains("(guid,headEquippedItemID,headVisibleItemID,")
    );
    assert!(
        statement
            .sql()
            .contains("tabardSecondaryItemModifiedAppearanceID,tabardSheatheCategory)")
    );
    assert_eq!(statement.params()[0], SqlParam::U64(23));
    assert_eq!(statement.params()[1], SqlParam::U32(42));
    assert_eq!(statement.params()[151], SqlParam::I32(-1));
    assert_eq!(statement.params()[152], SqlParam::U8(7));
    assert_eq!(visual_statement(24, &items).params()[0], SqlParam::U64(24));

    let mut transaction = SqlTransaction::new();
    append_visual_cache(&mut transaction, 23, &items);
    assert_eq!(
        transaction.sqls_for_test(),
        [
            "DELETE FROM character_select_screen_equipment_cache WHERE guid = ?",
            statement.sql(),
        ]
    );
    assert_eq!(
        visual_cache_delete_statement(23).params(),
        &[SqlParam::U64(23)]
    );
}

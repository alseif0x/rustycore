use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{NewGarrisonBuilding, NewGarrisonFollower};

fn empty() -> NewGarrisonSave {
    NewGarrisonSave {
        site_level_id: 7,
        follower_activations_remaining_today: 3,
        known_buildings: Vec::new(),
        plots: Vec::new(),
        followers: Vec::new(),
    }
}

fn follower(db_id: u64, abilities: Vec<u32>) -> NewGarrisonFollower {
    NewGarrisonFollower {
        db_id,
        follower_id: 17,
        quality: 2,
        level: 60,
        item_level_weapon: 100,
        item_level_armor: 101,
        xp: 19,
        current_building: 3,
        current_mission: 4,
        status: 5,
        abilities,
    }
}

#[test]
fn absent_skips_everything_but_present_empty_deletes_four_groups_then_inserts_root() {
    assert!(statements(17, None).is_empty());
    let data = empty();
    let rows = statements(17, Some(&data));
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            "DELETE FROM character_garrison WHERE guid = ?",
            "DELETE FROM character_garrison_blueprints WHERE guid = ?",
            "DELETE FROM character_garrison_buildings WHERE guid = ?",
            "DELETE gfab, gf FROM character_garrison_follower_abilities gfab INNER JOIN character_garrison_followers gf ON gfab.dbId = gf.dbId WHERE gf.guid = ?",
            "INSERT INTO character_garrison (guid, siteLevelId, followerActivationsRemainingToday) VALUES (?, ?, ?)",
        ]
    );
    for row in &rows[..4] {
        assert_eq!(row.params(), &[SqlParam::U64(17)]);
    }
    assert_eq!(
        rows[4].params(),
        &[SqlParam::U64(17), SqlParam::U32(7), SqlParam::U32(3)]
    );
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    append(&mut transaction, 17, None);
    assert!(transaction.sqls_for_test().is_empty());
    append(&mut transaction, 17, Some(&data));
    assert_eq!(
        transaction.sqls_for_test(),
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
    );
}

#[test]
fn source_vectors_preserve_blueprint_plot_and_follower_ability_group_order() {
    let mut data = empty();
    data.known_buildings = vec![90, 2];
    data.plots = vec![
        None,
        Some(NewGarrisonBuilding {
            plot_instance_id: 90,
            building_id: 7,
            time_built: -1,
            active: true,
        }),
        None,
        Some(NewGarrisonBuilding {
            plot_instance_id: 2,
            building_id: 3,
            time_built: 0,
            active: false,
        }),
    ];
    data.followers = vec![follower(90, vec![17, 2]), follower(2, vec![90])];
    data.followers[1].follower_id = 2;
    let rows = statements(31, Some(&data));
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [
            DELETES[0], DELETES[1], DELETES[2], DELETES[3], ROOT, BLUEPRINT, BLUEPRINT, BUILDING,
            BUILDING, FOLLOWER, ABILITY, ABILITY, FOLLOWER, ABILITY,
        ]
    );
    assert_eq!(
        rows[5].sql(),
        "INSERT INTO character_garrison_blueprints (guid, buildingId) VALUES (?, ?)"
    );
    assert_eq!(rows[5].params(), &[SqlParam::U64(31), SqlParam::U32(90)]);
    assert_eq!(rows[6].params(), &[SqlParam::U64(31), SqlParam::U32(2)]);
    assert_eq!(
        rows[7].sql(),
        "INSERT INTO character_garrison_buildings (guid, plotInstanceId, buildingId, timeBuilt, active) VALUES (?, ?, ?, ?, ?)"
    );
    assert_eq!(
        rows[7].params(),
        &[
            SqlParam::U64(31),
            SqlParam::U32(90),
            SqlParam::U32(7),
            SqlParam::I64(-1),
            SqlParam::Bool(true)
        ]
    );
    assert_eq!(
        rows[8].params(),
        &[
            SqlParam::U64(31),
            SqlParam::U32(2),
            SqlParam::U32(3),
            SqlParam::I64(0),
            SqlParam::Bool(false)
        ]
    );
    for (index, db_id, ability, slot) in [(10, 90, 17, 0), (11, 90, 2, 1), (13, 2, 90, 0)] {
        assert_eq!(
            rows[index].params(),
            &[
                SqlParam::U64(db_id),
                SqlParam::U32(ability),
                SqlParam::U8(slot)
            ]
        );
    }
    assert_eq!(rows[9].params()[0], SqlParam::U64(90));
    assert_eq!(rows[12].params()[0], SqlParam::U64(2));
}

#[test]
fn follower_binds_all_eleven_source_fields_then_ability_with_supplied_db_identity() {
    let mut data = empty();
    data.followers = vec![follower(90, vec![7])];
    let rows = statements(0x12_0000_0009, Some(&data));
    assert_eq!(
        rows[5].sql(),
        "INSERT INTO character_garrison_followers (dbId, guid, followerId, quality, level, itemLevelWeapon, itemLevelArmor, xp, currentBuilding, currentMission, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(rows[5].sql().matches('?').count(), 11);
    assert_eq!(
        rows[5].params(),
        &[
            SqlParam::U64(90),
            SqlParam::U64(0x12_0000_0009),
            SqlParam::U32(17),
            SqlParam::U32(2),
            SqlParam::U32(60),
            SqlParam::U32(100),
            SqlParam::U32(101),
            SqlParam::U32(19),
            SqlParam::U32(3),
            SqlParam::U32(4),
            SqlParam::U32(5),
        ]
    );
    assert_eq!(
        rows[6].sql(),
        "INSERT INTO character_garrison_follower_abilities (dbId, abilityId, slot) VALUES (?, ?, ?)"
    );
    assert_eq!(
        rows[6].params(),
        &[SqlParam::U64(90), SqlParam::U32(7), SqlParam::U8(0)]
    );
}

#[test]
fn encoder_bounds_preserve_unsigned_fields_signed_raw_time_and_bool() {
    // Scalar binding ranges only, not canonical identities or DB admission proof.
    for (time, active) in [(i64::MIN, false), (i64::MAX, true)] {
        let data = NewGarrisonSave {
            site_level_id: u32::MAX,
            follower_activations_remaining_today: u32::MAX,
            known_buildings: vec![u32::MAX],
            plots: vec![Some(NewGarrisonBuilding {
                plot_instance_id: u32::MAX,
                building_id: u32::MAX,
                time_built: time,
                active,
            })],
            followers: vec![NewGarrisonFollower {
                db_id: u64::MAX,
                follower_id: u32::MAX,
                quality: u32::MAX,
                level: u32::MAX,
                item_level_weapon: u32::MAX,
                item_level_armor: u32::MAX,
                xp: u32::MAX,
                current_building: u32::MAX,
                current_mission: u32::MAX,
                status: u32::MAX,
                abilities: vec![u32::MAX],
            }],
        };
        let rows = statements(17, Some(&data));
        assert_eq!(
            rows[4].params(),
            &[
                SqlParam::U64(17),
                SqlParam::U32(u32::MAX),
                SqlParam::U32(u32::MAX)
            ]
        );
        assert_eq!(
            rows[5].params(),
            &[SqlParam::U64(17), SqlParam::U32(u32::MAX)]
        );
        assert_eq!(
            rows[6].params(),
            &[
                SqlParam::U64(17),
                SqlParam::U32(u32::MAX),
                SqlParam::U32(u32::MAX),
                SqlParam::I64(time),
                SqlParam::Bool(active)
            ]
        );
        assert_eq!(
            &rows[7].params()[..2],
            &[SqlParam::U64(u64::MAX), SqlParam::U64(17)]
        );
        assert_eq!(rows[7].params().len(), 11);
        assert!(
            rows[7].params()[2..]
                .iter()
                .all(|p| *p == SqlParam::U32(u32::MAX))
        );
        assert_eq!(
            rows[8].params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U32(u32::MAX),
                SqlParam::U8(0)
            ]
        );
        for (row, count) in rows[4..].iter().zip([3, 2, 5, 11, 3]) {
            assert_eq!(row.sql().matches('?').count(), count);
        }
    }
}

#[test]
fn encoder_only_ability_slot_counter_wraps_like_source_uint8() {
    // 257 slots exercise narrowing only, not canonical ability-list admission.
    let mut data = empty();
    data.followers = vec![follower(90, (0..257).collect())];
    let rows = statements(17, Some(&data));
    for (slot, row) in rows[6..].iter().enumerate() {
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(90),
                SqlParam::U32(slot as u32),
                SqlParam::U8(slot as u8)
            ]
        );
    }
    assert_eq!(rows[6 + 256].params()[2], SqlParam::U8(0));
}

#[test]
fn character_save_places_optional_garrison_immediately_after_bank_tabs() {
    let mut character = super::super::tests::character();
    character.guid = 0x12_0000_0009;
    character.bank_tabs = vec![
        wow_persistence::forever::creation::save::NewBankTabSettings {
            name: "Raid".into(),
            icon: "INV_Sword_01".into(),
            description: "Reserve".into(),
            deposit_flags: 3,
        },
    ];
    character.garrison = Some(empty());
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_character_save(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let bank = sqls
        .iter()
        .position(|sql| *sql == "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?")
        .unwrap();
    assert_eq!(
        &sqls[bank..],
        &[
            "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?",
            "INSERT INTO character_bank_tab_settings (characterGuid, tabId, name, icon, description, depositFlags) VALUES (?, ?, ?, ?, ?, ?)",
            DELETES[0],
            DELETES[1],
            DELETES[2],
            DELETES[3],
            ROOT,
        ]
    );
    // Frozen DTO preservation, not canonical Garrison lifecycle/ID reservation proof.
    assert_eq!(character.garrison.as_ref().unwrap().site_level_id, 7);
}

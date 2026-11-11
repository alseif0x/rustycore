use super::*;
use crate::SqlParam;

fn zero() -> NewCharacterStatistics {
    NewCharacterStatistics {
        max_health: 0,
        powers: [0; 10],
        stats: [0; 5],
        resistances: [0; 7],
        percentages: [0.0; 6],
        attack_power: 0,
        ranged_attack_power: 0,
        spell_power: 0,
        resilience: 0,
        mastery: 0.0,
        versatility: 0,
    }
}

#[test]
fn frozen_ineligible_skips_all_sql_but_present_zero_statistics_delete_and_insert() {
    // Option consumption only, not execution of the source logout/config/level predicate.
    assert!(statements(17, None).is_empty());
    let data = zero();
    let rows = statements(17, Some(&data));
    assert_eq!(
        rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
        [DELETE, INSERT]
    );
    assert_eq!(rows[0].sql(), "DELETE FROM character_stats WHERE guid = ?");
    assert_eq!(rows[0].params(), &[SqlParam::U64(17)]);
    assert_eq!(rows[1].params().len(), 36);
    assert!(
        rows[1].params()[1..24]
            .iter()
            .all(|p| *p == SqlParam::U32(0))
    );
    assert!(
        rows[1].params()[24..30]
            .iter()
            .all(|p| *p == SqlParam::F32(0.0))
    );
    assert_eq!(
        &rows[1].params()[30..],
        &[
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::U32(0),
            SqlParam::F32(0.0),
            SqlParam::I32(0),
            SqlParam::Bool(false),
        ]
    );
    let mut transaction = SqlTransaction::new();
    append(&mut transaction, 17, None);
    assert!(transaction.sqls_for_test().is_empty());
    append(&mut transaction, 17, Some(&data));
    assert_eq!(transaction.sqls_for_test(), [DELETE, INSERT]);
    assert_eq!(data.max_health, 0);
}

#[test]
fn all_thirty_six_parameters_follow_source_sql_including_overwritten_resilience() {
    let data = NewCharacterStatistics {
        max_health: 101,
        powers: std::array::from_fn(|i| 102 + i as u32),
        stats: std::array::from_fn(|i| 112 + i as u32),
        resistances: std::array::from_fn(|i| 117 + i as u32),
        percentages: std::array::from_fn(|i| 124.5 + i as f32),
        attack_power: 130,
        ranged_attack_power: 131,
        spell_power: 132,
        resilience: 999,
        mastery: 133.5,
        versatility: -134,
    };
    let rows = statements(0x12_0000_0009, Some(&data));
    let row = &rows[1];
    assert_eq!(
        row.sql(),
        "INSERT INTO character_stats (guid, maxhealth, maxpower1, maxpower2, maxpower3, maxpower4, maxpower5, maxpower6, maxpower7, maxpower8, maxpower9, maxpower10, strength, agility, stamina, intellect, spirit, armor, resHoly, resFire, resNature, resFrost, resShadow, resArcane, blockPct, dodgePct, parryPct, critPct, rangedCritPct, spellCritPct, attackPower, rangedAttackPower, spellPower, resilience, mastery, versatility) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(row.sql().matches('?').count(), 36);
    let mut expected = vec![SqlParam::U64(0x12_0000_0009)];
    expected.extend((101..124).map(SqlParam::U32));
    expected.extend((0..6).map(|i| SqlParam::F32(124.5 + i as f32)));
    expected.extend([
        SqlParam::U32(130),
        SqlParam::U32(131),
        SqlParam::U32(132),
        SqlParam::F32(133.5),
        SqlParam::I32(-134),
        SqlParam::Bool(false),
    ]);
    assert_eq!(row.params(), expected);
    // Frozen setter inputs remain untouched; no canonical conversion/ack proof.
    assert_eq!(data.resilience, 999);
    assert_eq!(data.mastery, 133.5);
    assert_eq!(data.versatility, -134);
}

#[test]
fn encoder_bounds_preserve_all_unsigned_setter_inputs_and_signed_versatility() {
    // SQL setter ranges only: uint64 health/int32 resistance/float stat conversions
    // and canonical admission are not executed by this projection.
    for versatility in [i32::MIN, i32::MAX] {
        let data = NewCharacterStatistics {
            max_health: u32::MAX,
            powers: [u32::MAX; 10],
            stats: [u32::MAX; 5],
            resistances: [u32::MAX; 7],
            percentages: [f32::MAX; 6],
            attack_power: u32::MAX,
            ranged_attack_power: u32::MAX,
            spell_power: u32::MAX,
            resilience: u32::MAX,
            mastery: f32::MIN,
            versatility,
        };
        let rows = statements(17, Some(&data));
        let params = rows[1].params();
        assert_eq!(params[0], SqlParam::U64(17));
        assert!(params[1..24].iter().all(|p| *p == SqlParam::U32(u32::MAX)));
        assert!(params[24..30].iter().all(|p| *p == SqlParam::F32(f32::MAX)));
        assert!(params[30..33].iter().all(|p| *p == SqlParam::U32(u32::MAX)));
        assert_eq!(params[33], SqlParam::F32(f32::MIN));
        assert_eq!(params[34], SqlParam::I32(versatility));
        assert_eq!(params[35], SqlParam::Bool(false));
    }
}

#[test]
fn encoder_raw_float_bits_survive_in_all_percentage_slots_and_mastery() {
    // Raw encoding only, not database nonfinite-value admission proof.
    for value in [
        f32::from_bits(0x7fc0_1234),
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0,
    ] {
        let mut data = zero();
        data.percentages = [value; 6];
        data.mastery = value;
        let rows = statements(17, Some(&data));
        for index in [24, 25, 26, 27, 28, 29, 33] {
            match rows[1].params()[index] {
                SqlParam::F32(actual) => assert_eq!(actual.to_bits(), value.to_bits()),
                _ => panic!("expected raw F32"),
            }
        }
        assert_eq!(rows[1].params()[35], SqlParam::Bool(false));
    }
}

#[test]
fn character_save_places_statistics_after_bank_and_optional_garrison() {
    let mut character = super::super::tests::character();
    character.guid = 0x12_0000_0009;
    character.garrison = Some(wow_persistence::forever::creation::save::NewGarrisonSave {
        site_level_id: 7,
        follower_activations_remaining_today: 3,
        known_buildings: Vec::new(),
        plots: Vec::new(),
        followers: Vec::new(),
    });
    character.statistics = Some(zero());
    let mut transaction = SqlTransaction::new();
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
            "DELETE FROM character_garrison WHERE guid = ?",
            "DELETE FROM character_garrison_blueprints WHERE guid = ?",
            "DELETE FROM character_garrison_buildings WHERE guid = ?",
            "DELETE gfab, gf FROM character_garrison_follower_abilities gfab INNER JOIN character_garrison_followers gf ON gfab.dbId = gf.dbId WHERE gf.guid = ?",
            "INSERT INTO character_garrison (guid, siteLevelId, followerActivationsRemainingToday) VALUES (?, ?, ?)",
            DELETE,
            INSERT,
        ]
    );
    assert!(character.statistics.is_some());
}

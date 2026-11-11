use super::*;
use crate::SqlParam;
use wow_persistence::forever::creation::save::{
    NewQuestChange, NewRewardedQuestChange, NewSpell, NewTalent,
};

fn snapshot() -> NewSpecializations {
    super::super::tests::character().specializations
}

#[test]
fn empty_specializations_always_emit_three_deletes_and_five_zero_pvp_rows() {
    for guid in [17, u64::MAX] {
        let data = snapshot();
        let rows = statements(guid, &data);
        assert_eq!(
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>(),
            [
                "DELETE FROM character_glyphs WHERE guid = ?",
                "DELETE FROM character_talent WHERE guid = ?",
                "DELETE FROM character_pvp_talent WHERE guid = ?",
                "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
                "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
                "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
                "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
                "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            ]
        );
        for row in &rows[..3] {
            assert_eq!(row.params(), &[SqlParam::U64(guid)]);
        }
        for (group, row) in rows[3..].iter().enumerate() {
            assert_eq!(
                row.params(),
                &[
                    SqlParam::U64(guid),
                    SqlParam::U32(0),
                    SqlParam::U32(0),
                    SqlParam::U32(0),
                    SqlParam::U32(0),
                    SqlParam::U8(group as u8),
                ]
            );
        }
        let mut transaction = crate::forever::create::tests::isolated_transaction();
        append(&mut transaction, guid, &data);
        assert_eq!(
            transaction.sqls_for_test(),
            rows.iter().map(|row| row.sql()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn glyphs_visit_all_five_groups_preserving_vector_order_duplicates_and_u16_narrowing() {
    let mut data = snapshot();
    data.glyphs = std::array::from_fn(|group| vec![0x1_0000 + group as u32, u32::MAX, 7, 7, 0]);
    let rows = statements(u64::MAX, &data);
    assert_eq!(rows[0].sql(), GLYPH_DELETE);
    assert_eq!(rows[26].sql(), TALENT_DELETE);
    assert_eq!(rows.len(), 33);
    for (group, group_rows) in rows[1..26].chunks_exact(5).enumerate() {
        for (row, glyph) in group_rows.iter().zip([group as u16, u16::MAX, 7, 7, 0]) {
            assert_eq!(row.sql(), "INSERT INTO character_glyphs VALUES(?, ?, ?)");
            assert_eq!(
                row.params(),
                &[
                    SqlParam::U64(u64::MAX),
                    SqlParam::U8(group as u8),
                    SqlParam::U16(glyph),
                ]
            );
        }
    }
}

#[test]
fn normal_talents_skip_only_removed_and_keep_unsorted_unique_u32_ids_per_group() {
    let mut data = snapshot();
    data.talents = std::array::from_fn(|_| {
        vec![
            NewTalent {
                id: 99,
                removed: true,
            },
            NewTalent {
                id: u32::MAX,
                removed: false,
            },
            NewTalent {
                id: 7,
                removed: true,
            },
            NewTalent {
                id: 2,
                removed: false,
            },
            NewTalent {
                id: 17,
                removed: false,
            },
            NewTalent {
                id: 0,
                removed: false,
            },
        ]
    });
    let rows = statements(29, &data);
    assert_eq!(rows[0].sql(), GLYPH_DELETE);
    assert_eq!(rows[1].sql(), TALENT_DELETE);
    assert_eq!(rows[22].sql(), PVP_DELETE);
    assert_eq!(rows.len(), 28);
    for (group, group_rows) in rows[2..22].chunks_exact(4).enumerate() {
        for (row, talent) in group_rows.iter().zip([u32::MAX, 2, 17, 0]) {
            assert_eq!(
                row.sql(),
                "INSERT INTO character_talent (guid, talentId, talentGroup) VALUES (?, ?, ?)"
            );
            assert_eq!(
                row.params(),
                &[
                    SqlParam::U64(29),
                    SqlParam::U32(talent),
                    SqlParam::U8(group as u8),
                ]
            );
        }
    }
}

#[test]
fn pvp_all_twenty_slots_bind_u32_before_each_u8_group_and_use_the_same_guid() {
    let mut data = snapshot();
    data.pvp = std::array::from_fn(|group| [u32::MAX, 0x8000_0000 + group as u32, group as u32, 0]);
    let rows = statements(u64::MAX, &data);
    assert_eq!(rows.len(), 8);
    for (group, row) in rows[3..].iter().enumerate() {
        assert_eq!(row.sql().matches('?').count(), 6);
        assert_eq!(
            row.params(),
            &[
                SqlParam::U64(u64::MAX),
                SqlParam::U32(u32::MAX),
                SqlParam::U32(0x8000_0000 + group as u32),
                SqlParam::U32(group as u32),
                SqlParam::U32(0),
                SqlParam::U8(group as u8),
            ]
        );
    }
}

#[test]
fn production_prefix_places_glyphs_normal_and_pvp_talents_after_quests_before_spells() {
    let mut character = super::super::tests::character();
    character.quests.statuses.insert(23, NewQuestChange::Delete);
    character
        .quests
        .rewarded
        .insert(7, NewRewardedQuestChange::Default);
    character.quests.monthly = Some(std::collections::BTreeSet::from([6]));
    character.specializations.glyphs[4] = vec![0x1_0001];
    character.specializations.talents[4] = vec![NewTalent {
        id: u32::MAX,
        removed: false,
    }];
    character.spells = vec![NewSpell {
        spell: 42,
        active: true,
        disabled: false,
        dependent: false,
        favorite: false,
    }];
    let mut transaction = crate::forever::create::tests::isolated_transaction();
    super::super::append_creation_prefix(&mut transaction, character.guid, &character);
    let sqls = transaction.sqls_for_test();
    let start = sqls
        .iter()
        .position(|sql| *sql == "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?")
        .unwrap();
    assert_eq!(
        &sqls[start..],
        &[
            "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?",
            "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?",
            "DELETE FROM character_queststatus_objectives_spawn_tracking WHERE guid = ? AND quest = ?",
            "INSERT IGNORE INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, 1)",
            "DELETE FROM character_queststatus_monthly WHERE guid = ?",
            "INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)",
            "DELETE FROM character_glyphs WHERE guid = ?",
            "INSERT INTO character_glyphs VALUES(?, ?, ?)",
            "DELETE FROM character_talent WHERE guid = ?",
            "INSERT INTO character_talent (guid, talentId, talentGroup) VALUES (?, ?, ?)",
            "DELETE FROM character_pvp_talent WHERE guid = ?",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)",
            "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)",
            "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?",
            "DELETE FROM character_spell_cooldown WHERE guid = ?",
            "DELETE FROM character_spell_charges WHERE guid = ?",
        ]
    );
}

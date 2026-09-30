//! Login skill-rewarded quest fallback remains private to the login data owner.

use super::*;
use std::sync::Arc;
use wow_data::{PlayerConditionEntry, SpellMiscEntry, SpellMiscStore};
use wow_packet::WorldPacket;

#[test]
fn skill_rewarded_quest_fallback_uses_future_player_condition_like_cpp() {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let mut session = WorldSession::new(
        1, "TestAccount".into(), 0, 2, 9, 54261, vec![0u8; 40], "esES".into(), pkt_rx,
        send_tx,
    );
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 10, 0);
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([
        SpellMiscEntry {
            id: 1,
            spell_id: 900,
            show_future_spell_player_condition_id: 77,
            ..SpellMiscEntry::default()
        },
        SpellMiscEntry {
            id: 2,
            spell_id: 901,
            show_future_spell_player_condition_id: 0,
            ..SpellMiscEntry::default()
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        PlayerConditionEntry {
            id: 77,
            class_mask: 1,
            ..PlayerConditionEntry::default()
        },
    ])));

    assert!(session.skill_rewarded_quest_fallback_allowed_like_cpp(900));
    assert!(
        !session.skill_rewarded_quest_fallback_allowed_like_cpp(901),
        "C++ MeetsFutureSpellPlayerCondition returns false when the condition id is zero"
    );

    session.set_loaded_player_identity_like_cpp(0, 1, 2, 10, 0);
    assert!(
        !session.skill_rewarded_quest_fallback_allowed_like_cpp(900),
        "the fallback must evaluate the real PlayerCondition against the current player"
    );
}

#[test]
fn skill_rewarded_login_changes_use_real_spell_levels_and_conditions_like_cpp() {
    fn ability(
        id: u32,
        spell: i32,
        acquire_method: i8,
        min_skill_line_rank: i16,
        flags: i8,
    ) -> wow_data::SkillLineAbilityRecord {
        wow_data::SkillLineAbilityRecord {
            id,
            race_mask: 1,
            skill_line: 164,
            spell,
            min_skill_line_rank,
            class_mask: 1,
            supercedes_spell: 0,
            acquire_method,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        }
    }

    fn spell_info(spell_id: i32) -> wow_data::SpellInfo {
        wow_data::SpellInfo {
            spell_id,
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
        }
    }

    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 10, 0);
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_like_cpp([
            ability(
                1,
                900,
                wow_data::skill::SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
                50,
                0,
            ),
            ability(
                2,
                901,
                wow_data::skill::SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                0,
                0,
            ),
            ability(
                3,
                902,
                wow_data::skill::SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP,
                0,
                wow_data::skill::SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            ability(
                4,
                903,
                wow_data::skill::SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                0,
                0,
            ),
        ]),
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in [900, 901, 902] {
        spell_store.insert(spell_id, spell_info(spell_id));
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_levels_store(Arc::new(wow_data::SpellLevelsStore::from_entries([
        wow_data::SpellLevelsEntry {
            id: 1,
            difficulty_id: 0,
            base_level: 1,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 900,
        },
        wow_data::SpellLevelsEntry {
            id: 2,
            difficulty_id: 0,
            base_level: 20,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 901,
        },
        wow_data::SpellLevelsEntry {
            id: 3,
            difficulty_id: 0,
            base_level: 1,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 902,
        },
        wow_data::SpellLevelsEntry {
            id: 4,
            difficulty_id: 0,
            base_level: 1,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 903,
        },
    ])));
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
        id: 1,
        spell_id: 902,
        show_future_spell_player_condition_id: 77,
        ..SpellMiscEntry::default()
    }])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        PlayerConditionEntry {
            id: 77,
            class_mask: 1,
            ..PlayerConditionEntry::default()
        },
    ])));

    let changes = session.skill_rewarded_spell_changes_for_login_like_cpp(164, 40, 1, 1, 10);

    assert_eq!(
        changes.remove,
        vec![900],
        "C++ removes an OnSkillValue spell while the skill is below its required rank"
    );
    assert_eq!(
        changes.learn,
        vec![902],
        "the level-gated spell and the ability without real SpellInfo must be skipped"
    );
}

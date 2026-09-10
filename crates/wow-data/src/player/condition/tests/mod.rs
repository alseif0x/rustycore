//! Tests for the player_condition module.
//!
//! Separated from player_condition.rs under #687.

use super::*;

fn pc(id: u32) -> PlayerConditionEntry {
    PlayerConditionEntry {
        id,
        ..PlayerConditionEntry::default()
    }
}

#[test]
fn load_player_condition_uses_physical_wdc4_array_fields_like_cpp_meta() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "enUS";
    let path = std::path::Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("PlayerCondition.db2");
    if !path.exists() {
        eprintln!("Skipping test: PlayerCondition.db2 fixture not found at {path:?}");
        return;
    }

    let reader = Wdc4Reader::open(&path).expect("open PlayerCondition.db2 fixture");
    assert_eq!(
        reader.field_count(),
        81,
        "C++ PlayerConditionMeta stores 147 logical fields in 81 physical WDC4 fields"
    );

    let store = PlayerConditionStore::load(data_dir, locale).expect("load PlayerCondition.db2");
    assert!(!store.is_empty());
    assert!(
        store.entries.values().any(|entry| {
            entry.skill_id.iter().any(|value| *value != 0)
                || entry.prev_quest_id.iter().any(|value| *value != 0)
                || entry.quest_kill_monster.iter().any(|value| *value != 0)
        }),
        "fixture should exercise at least one compacted physical array field"
    );
}

#[test]
fn player_condition_compare_matches_cpp_table() {
    assert!(player_condition_compare_like_cpp(1, 7, 7));
    assert!(player_condition_compare_like_cpp(2, 7, 8));
    assert!(player_condition_compare_like_cpp(3, 8, 7));
    assert!(player_condition_compare_like_cpp(4, 7, 7));
    assert!(player_condition_compare_like_cpp(5, 6, 7));
    assert!(player_condition_compare_like_cpp(6, 7, 7));
    assert!(!player_condition_compare_like_cpp(0, 7, 7));
}

#[test]
fn player_condition_logic_applies_invert_and_and_or_like_cpp() {
    assert!(player_condition_logic_like_cpp(1, [true, true]));
    assert!(player_condition_logic_like_cpp(2, [false, true]));
    assert!(!player_condition_logic_like_cpp(
        1 | (1 << 17),
        [true, true]
    ));
}

#[test]
fn player_condition_filters_race_class_gender_like_cpp() {
    let condition = PlayerConditionEntry {
        race_mask: 1 << 0,
        class_mask: 1 << 1,
        gender: 1,
        ..pc(1)
    };
    let context = PlayerConditionContextLikeCpp {
        race: 1,
        class_mask: 1 << 1,
        gender: 1,
        native_gender: 0,
        ..Default::default()
    };

    assert!(is_player_meeting_condition_like_cpp(&condition, &context));
    assert!(!is_player_meeting_condition_like_cpp(
        &PlayerConditionEntry {
            race_mask: 1 << 1,
            ..condition.clone()
        },
        &context
    ));
}

#[test]
fn player_condition_filters_spells_items_currency_and_auras_like_cpp() {
    let condition = PlayerConditionEntry {
        spell_id: [133, 0, 0, 0],
        item_id: [6948, 0, 0, 0],
        item_count: [1, 0, 0, 0],
        currency_id: [61, 0, 0, 0],
        currency_count: [3, 0, 0, 0],
        aura_spell_id: [21562, 0, 0, 0],
        aura_stacks: [2, 0, 0, 0],
        ..pc(1)
    };
    let context = PlayerConditionContextLikeCpp {
        spells: &[133],
        items: &[PlayerConditionCountLikeCpp { id: 6948, count: 1 }],
        currencies: &[PlayerConditionCountLikeCpp { id: 61, count: 3 }],
        auras: &[PlayerConditionAuraLikeCpp {
            spell_id: 21562,
            stacks: 2,
        }],
        ..Default::default()
    };

    assert!(is_player_meeting_condition_like_cpp(&condition, &context));
}

#[test]
fn player_condition_filters_party_and_area_like_cpp() {
    let condition = PlayerConditionEntry {
        party_status: 3,
        area_id: [1519, 0, 0, 0],
        ..pc(1)
    };
    let context = PlayerConditionContextLikeCpp {
        party_status: PlayerConditionPartyStatusLikeCpp::InParty,
        area_id: 42,
        parent_area_ids: &[1519],
        ..Default::default()
    };

    assert!(is_player_meeting_condition_like_cpp(&condition, &context));
}

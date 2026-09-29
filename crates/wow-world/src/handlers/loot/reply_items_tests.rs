use super::{
    direct_item_count_after_loot_release_like_cpp, represented_loot_response_items_like_cpp,
};
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_loot::{
    LOOT_METHOD_GROUP_LIKE_CPP, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP,
    LOOT_SLOT_TYPE_OWNER_LIKE_CPP, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
    mark_loot_allowed_for_player_like_cpp, mark_loot_item_looted_for_player_like_cpp,
};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_CORPSE_LIKE_CPP, LootEntry, LootEntryFlags,
};

fn represented_loot_entry(loot_list_id: u8, item_id: u32, player_guid: ObjectGuid) -> LootEntry {
    LootEntry {
        loot_list_id,
        item_id,
        quantity: 1,
        random_properties_id: 0,
        random_properties_seed: 0,
        item_context: 0,
        flags: LootEntryFlags {
            follow_loot_rules: true,
            freeforall: false,
            blocked: false,
            counted: false,
            under_threshold: false,
            needs_quest: false,
        },
        allowed_looters: vec![player_guid],
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    }
}

#[test]
fn represented_loot_response_items_use_cpp_ui_type_decision_tree() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 100);

    let mut rolling_entry = represented_loot_entry(0, 25, player_guid);
    rolling_entry.flags.blocked = true;

    let mut won_entry = represented_loot_entry(1, 26, player_guid);
    won_entry.roll_winner = player_guid;

    let mut hidden_entry = represented_loot_entry(2, 27, player_guid);
    hidden_entry.roll_winner = other_guid;

    let mut allowed_entry = represented_loot_entry(3, 28, player_guid);
    allowed_entry.flags.under_threshold = true;

    let loot = CreatureLoot {
        loot_guid,
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player_guid],
        items: vec![rolling_entry, won_entry, hidden_entry, allowed_entry],
        looted_by_player: false,
    };

    let items = represented_loot_response_items_like_cpp(&loot, player_guid);

    assert_eq!(items.len(), 3);
    assert_eq!(items[0].loot_list_id, 0);
    assert_eq!(items[0].ui_type, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP);
    assert_eq!(items[1].loot_list_id, 1);
    assert_eq!(items[1].ui_type, LOOT_SLOT_TYPE_OWNER_LIKE_CPP);
    assert_eq!(items[2].loot_list_id, 3);
    assert_eq!(items[2].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
}

#[test]
fn represented_ffa_loot_uses_player_ffa_items_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 101);

    let mut ffa_entry = represented_loot_entry(0, 25, player_guid);
    ffa_entry.flags.freeforall = true;
    ffa_entry.allowed_looters.clear();

    let mut loot = CreatureLoot {
        loot_guid,
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: vec![ffa_entry],
        looted_by_player: false,
    };

    mark_loot_allowed_for_player_like_cpp(&mut loot, player_guid);
    mark_loot_allowed_for_player_like_cpp(&mut loot, other_guid);
    assert_eq!(loot.unlooted_count, 2);

    let player_items = represented_loot_response_items_like_cpp(&loot, player_guid);
    let other_items = represented_loot_response_items_like_cpp(&loot, other_guid);
    assert_eq!(player_items.len(), 1);
    assert_eq!(other_items.len(), 1);
    assert_eq!(player_items[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
    assert_eq!(other_items[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);

    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
    assert_eq!(loot.unlooted_count, 1);

    let player_ffa = loot
        .player_ffa_items
        .iter()
        .find(|(player, _)| *player == player_guid)
        .and_then(|(_, items)| items.iter().find(|item| item.loot_list_id == 0))
        .unwrap();
    let other_ffa = loot
        .player_ffa_items
        .iter()
        .find(|(player, _)| *player == other_guid)
        .and_then(|(_, items)| items.iter().find(|item| item.loot_list_id == 0))
        .unwrap();

    assert!(player_ffa.is_looted);
    assert!(!other_ffa.is_looted);
    assert!(represented_loot_response_items_like_cpp(&loot, player_guid).is_empty());
    assert_eq!(
        represented_loot_response_items_like_cpp(&loot, other_guid).len(),
        1
    );
}

#[test]
fn prospecting_and_milling_release_consume_at_most_five_source_items_like_cpp() {
    assert_eq!(
        direct_item_count_after_loot_release_like_cpp(20, Some(5)),
        15
    );
    assert_eq!(direct_item_count_after_loot_release_like_cpp(5, Some(5)), 0);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(3, Some(5)), 0);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(20, None), 0);
}

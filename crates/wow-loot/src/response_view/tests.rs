use super::loot_response_item_views;
use crate::{
    CreatureLoot, LOOT_METHOD_GROUP_LIKE_CPP, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP,
    LOOT_SLOT_TYPE_OWNER_LIKE_CPP, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP, LootEntry, LootEntryFlags,
    NotNormalLootItem, mark_loot_allowed_for_player_like_cpp,
    mark_loot_item_looted_for_player_like_cpp,
};
use wow_core::{ObjectGuid, guid::HighGuid};

const CORPSE_LOOT_TYPE: u8 = 1;

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
        loot_type: CORPSE_LOOT_TYPE,
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

    let items = loot_response_item_views(&loot, player_guid).collect::<Vec<_>>();

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
        loot_type: CORPSE_LOOT_TYPE,
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

    let player_items = loot_response_item_views(&loot, player_guid).collect::<Vec<_>>();
    let other_items = loot_response_item_views(&loot, other_guid).collect::<Vec<_>>();
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
    assert!(
        loot_response_item_views(&loot, player_guid)
            .collect::<Vec<_>>()
            .is_empty()
    );
    assert_eq!(
        loot_response_item_views(&loot, other_guid)
            .collect::<Vec<_>>()
            .len(),
        1
    );
}

fn response_loot(items: Vec<LootEntry>, player_guid: ObjectGuid) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins: 0,
        unlooted_count: 0,
        loot_type: CORPSE_LOOT_TYPE,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player_guid],
        items,
        looted_by_player: false,
    }
}

#[test]
fn response_rows_preserve_storage_order_and_duplicate_list_ids() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let loot = response_loot(
        vec![
            represented_loot_entry(9, 25, player),
            represented_loot_entry(3, 99, other),
            represented_loot_entry(1, 26, player),
            represented_loot_entry(9, 25, player),
        ],
        player,
    );

    let rows: Vec<_> = loot_response_item_views(&loot, player)
        .map(|view| (view.loot_list_id, view.item_id, view.quantity, view.ui_type))
        .collect();

    assert_eq!(
        rows,
        vec![
            (9, 25, 1, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
            (1, 26, 1, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
            (9, 25, 1, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
        ]
    );
}

#[test]
fn response_rows_copy_quantities_and_ids_without_normalization() {
    let player = ObjectGuid::create_player(1, 42);
    let mut zero_quantity = represented_loot_entry(u8::MAX, u32::MAX, player);
    zero_quantity.quantity = 0;
    let mut maximum_quantity = represented_loot_entry(0, 0, player);
    maximum_quantity.quantity = u32::MAX;
    let loot = response_loot(vec![zero_quantity, maximum_quantity], player);

    let rows: Vec<_> = loot_response_item_views(&loot, player)
        .map(|view| (view.loot_list_id, view.item_id, view.quantity, view.ui_type))
        .collect();

    assert_eq!(
        rows,
        vec![
            (u8::MAX, u32::MAX, 0, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
            (0, 0, u32::MAX, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
        ]
    );
}

#[test]
fn ffa_visibility_precedes_quest_bypass() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut entry = represented_loot_entry(7, 25, player);
    entry.flags.freeforall = true;
    entry.flags.needs_quest = true;
    entry.flags.follow_loot_rules = false;
    entry.flags.under_threshold = true;
    entry.flags.blocked = true;
    entry.roll_winner = other;
    let mut loot = response_loot(vec![entry], player);
    loot.round_robin_player = other;

    assert!(loot_response_item_views(&loot, player).next().is_none());

    loot.player_ffa_items.push((
        other,
        vec![NotNormalLootItem {
            loot_list_id: 7,
            is_looted: false,
        }],
    ));
    assert!(loot_response_item_views(&loot, player).next().is_none());

    loot.player_ffa_items.push((
        player,
        vec![NotNormalLootItem {
            loot_list_id: 7,
            is_looted: false,
        }],
    ));
    let rows: Vec<_> = loot_response_item_views(&loot, player).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);

    loot.player_ffa_items[1].1[0].is_looted = true;
    assert!(loot_response_item_views(&loot, player).next().is_none());
}

#[test]
fn quest_without_loot_rules_bypasses_normal_method_guards() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut entry = represented_loot_entry(7, 25, player);
    entry.flags.needs_quest = true;
    entry.flags.follow_loot_rules = false;
    entry.flags.under_threshold = true;
    entry.flags.blocked = true;
    entry.roll_winner = other;
    let mut loot = response_loot(vec![entry], player);
    loot.round_robin_player = other;

    let rows: Vec<_> = loot_response_item_views(&loot, player).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);

    loot.items[0].flags.follow_loot_rules = true;
    assert!(loot_response_item_views(&loot, player).next().is_none());

    loot.items[0].flags.under_threshold = false;
    let rows: Vec<_> = loot_response_item_views(&loot, player).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].ui_type, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP);

    loot.items[0].flags.blocked = false;
    assert!(loot_response_item_views(&loot, player).next().is_none());

    loot.items[0].roll_winner = player;
    assert_eq!(
        loot_response_item_views(&loot, player)
            .next()
            .unwrap()
            .ui_type,
        LOOT_SLOT_TYPE_OWNER_LIKE_CPP
    );
}

#[test]
fn quest_bypass_still_rejects_consumed_and_disallowed_rows() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut entry = represented_loot_entry(7, 25, player);
    entry.flags.needs_quest = true;
    entry.flags.follow_loot_rules = false;
    entry.taken = true;
    let mut loot = response_loot(vec![entry], player);

    assert!(loot_response_item_views(&loot, player).next().is_none());

    loot.items[0].taken = false;
    loot.items[0].allowed_looters = vec![other];
    assert!(loot_response_item_views(&loot, player).next().is_none());

    loot.items[0].allowed_looters = vec![player];
    assert_eq!(
        loot_response_item_views(&loot, player)
            .next()
            .unwrap()
            .ui_type,
        LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP
    );
}

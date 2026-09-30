//! Pure admission policy regressions; application and entity cases stay with their owners.

use super::*;
use crate::{LootEntryFlags, NotNormalLootItem};
use wow_constants::ItemContext;
use wow_entities::{Item, ItemCreateInfo, MAX_ITEM_SPELLS};

fn entry(list_id: u8, allowed_looters: Vec<ObjectGuid>) -> LootEntry {
    LootEntry {
        loot_list_id: list_id,
        item_id: 25,
        quantity: 1,
        random_properties_id: 0,
        random_properties_seed: 0,
        item_context: 0,
        flags: LootEntryFlags::default(),
        allowed_looters,
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    }
}

fn loot(items: Vec<LootEntry>, allowed_looters: Vec<ObjectGuid>) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::create_item(1, 700),
        coins: 0,
        unlooted_count: 0,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters,
        items,
        looted_by_player: false,
    }
}

#[test]
fn direct_rejections_prioritize_allowed_looter_then_blocked_then_winner() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut item = entry(0, vec![other]);
    item.flags.blocked = true;
    item.roll_winner = other;
    assert_eq!(direct_item_rejection(&item, player), Some(DirectItemRejection::NotAllowed));

    item.allowed_looters.push(player);
    assert_eq!(direct_item_rejection(&item, player), Some(DirectItemRejection::Blocked));
    item.flags.blocked = false;
    assert_eq!(direct_item_rejection(&item, player), Some(DirectItemRejection::OtherWinner));
    item.roll_winner = player;
    assert_eq!(direct_item_rejection(&item, player), None);
    item.roll_winner = ObjectGuid::EMPTY;
    assert_eq!(direct_item_rejection(&item, player), None);
}

#[test]
fn direct_selection_uses_list_id_while_master_selection_uses_vector_index() {
    let player = ObjectGuid::create_player(1, 42);
    let mut first = entry(9, vec![player]);
    first.item_id = 25;
    let mut second = entry(0, vec![player]);
    second.item_id = 26;
    let pool = loot(vec![first, second], vec![player]);

    assert_eq!(find_unlooted_item(&pool, 0, player).unwrap().item_id, 26);
    assert_eq!(select_master_item(&pool, 0, player).unwrap().item_id, 25);
    assert_eq!(find_unlooted_item(&pool, 9, player).unwrap().item_id, 25);
    assert_eq!(select_master_item(&pool, 9, player).err(), Some(MasterItemRejection::InvalidSlot));
    assert!(find_unlooted_item(&pool, 8, player).is_none());
}

#[test]
fn direct_selection_skips_consumed_rows_and_takes_the_first_available_duplicate() {
    let player = ObjectGuid::create_player(1, 42);
    let mut consumed = entry(7, vec![player]);
    consumed.taken = true;
    let mut first_available = entry(7, vec![player]);
    first_available.item_id = 26;
    let mut later_available = entry(7, vec![player]);
    later_available.item_id = 27;
    let mut pool = loot(vec![consumed, first_available, later_available], vec![player]);

    assert_eq!(find_unlooted_item(&pool, 7, player).unwrap().item_id, 26);
    pool.items[1].taken = true;
    assert_eq!(find_unlooted_item(&pool, 7, player).unwrap().item_id, 27);
    pool.items[2].taken = true;
    assert!(find_unlooted_item(&pool, 7, player).is_none());
}

#[test]
fn direct_ffa_selection_uses_the_player_pool_marker() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut item = entry(3, vec![player, other]);
    item.flags.freeforall = true;
    item.taken = true;
    item.ffa_looted_by.push(player);
    let mut pool = loot(vec![item], vec![player, other]);
    assert!(find_unlooted_item(&pool, 3, player).is_none());

    pool.player_ffa_items.push((player, vec![NotNormalLootItem {
        loot_list_id: 3,
        is_looted: false,
    }]));
    pool.player_ffa_items.push((other, vec![NotNormalLootItem {
        loot_list_id: 3,
        is_looted: true,
    }]));
    assert!(find_unlooted_item(&pool, 3, player).is_some());
    assert!(find_unlooted_item(&pool, 3, other).is_none());
    pool.player_ffa_items[0].1[0].is_looted = true;
    assert!(find_unlooted_item(&pool, 3, player).is_none());
}

#[test]
fn master_rejections_prioritize_method_pool_membership_index_then_item_membership() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut pool = loot(vec![entry(0, vec![other])], Vec::new());
    pool.loot_method = 0;
    assert_eq!(select_master_item(&pool, 9, player).err(), Some(MasterItemRejection::WrongMethod));
    pool.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    assert_eq!(select_master_item(&pool, 9, player).err(), Some(MasterItemRejection::TargetNotAllowed));
    pool.allowed_looters.push(player);
    assert_eq!(select_master_item(&pool, 9, player).err(), Some(MasterItemRejection::InvalidSlot));
    assert_eq!(select_master_item(&pool, 0, player).err(), Some(MasterItemRejection::ItemTargetNotAllowed));
    pool.items[0].allowed_looters.push(player);
    assert!(select_master_item(&pool, 0, player).is_ok());
}

#[test]
fn empty_item_tap_list_is_allowed_for_master_selection_but_not_remote_delivery() {
    let player = ObjectGuid::create_player(1, 42);
    let pool = loot(vec![entry(0, Vec::new())], vec![player]);
    let item = select_master_item(&pool, 0, player).unwrap();
    assert!(!master_award_recipient_allowed(item, player));
    assert!(!roll_award_batch_allowed(&pool.items, false, player));
    assert_eq!(direct_item_rejection(item, player), Some(DirectItemRejection::NotAllowed));
}

#[test]
fn master_selection_and_remote_admission_do_not_add_direct_or_winner_checks() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut item = entry(0, vec![player]);
    item.flags.blocked = true;
    item.roll_winner = other;
    item.taken = true;
    let pool = loot(vec![item], vec![player]);
    let selected = select_master_item(&pool, 0, player).unwrap();
    assert!(master_award_recipient_allowed(selected, player));
    assert_eq!(direct_item_rejection(selected, player), Some(DirectItemRejection::Blocked));
    assert!(!roll_award_batch_allowed(&pool.items, false, player));
}

#[test]
fn roll_batches_require_one_item_and_disenchant_batches_require_at_least_one() {
    let player = ObjectGuid::create_player(1, 42);
    assert!(!roll_award_batch_allowed(&[], false, player));
    assert!(!roll_award_batch_allowed(&[], true, player));
    let mut items = vec![entry(0, vec![player])];
    assert!(roll_award_batch_allowed(&items, false, player));
    assert!(roll_award_batch_allowed(&items, true, player));
    items.push(entry(1, vec![player]));
    assert!(!roll_award_batch_allowed(&items, false, player));
    assert!(roll_award_batch_allowed(&items, true, player));
}

#[test]
fn roll_batches_require_every_recipient_and_winner_but_not_direct_flags() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut items = vec![entry(0, vec![player]), entry(1, vec![player])];
    items[0].flags.blocked = true;
    items[0].taken = true;
    assert!(roll_award_batch_allowed(&items, true, player));
    items[1].allowed_looters.clear();
    assert!(!roll_award_batch_allowed(&items, true, player));
    items[1].allowed_looters.push(other);
    assert!(!roll_award_batch_allowed(&items, true, player));
    items[1].allowed_looters.push(player);
    items[1].roll_winner = other;
    assert!(!roll_award_batch_allowed(&items, true, player));
    items[1].roll_winner = player;
    assert!(roll_award_batch_allowed(&items, true, player));
}

#[test]
fn remote_master_admission_requires_explicit_recipient_membership() {
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut item = entry(0, Vec::new());
    assert!(!master_award_recipient_allowed(&item, player));
    item.allowed_looters.push(other);
    assert!(!master_award_recipient_allowed(&item, player));
    item.allowed_looters.push(player);
    assert!(master_award_recipient_allowed(&item, player));
}

#[test]
fn loot_item_random_context_stack_compatibility_uses_cpp_store_metadata() {
    let item_guid = ObjectGuid::create_item(1, 901);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let mut item = Item::new(0);
    item.initialize_created_state(ItemCreateInfo {
        guid: item_guid,
        item_id: 25,
        context: ItemContext::DungeonHeroic,
        owner: Some(owner_guid),
        max_durability: 0,
        expiration: 0,
        spell_charges: [0; MAX_ITEM_SPELLS],
    });
    item.set_random_properties_id(-77);
    item.set_property_seed(456);

    let matching = LootEntry {
        loot_list_id: 0,
        item_id: 25,
        quantity: 1,
        random_properties_id: -77,
        random_properties_seed: 456,
        item_context: 2,
        flags: LootEntryFlags::default(),
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    };
    assert!(loot_store_item_matches(
        &matching,
        -77, 456,
        &item
    ));

    let different_random = LootEntry {
        random_properties_id: -78,
        ..matching.clone()
    };
    assert!(loot_store_item_matches(
        &different_random,
        -77, 456,
        &item
    ));
    assert!(!loot_store_item_matches(
        &matching,
        0, 0,
        &item
    ));
}

//! State regressions relocated from world; packet/transaction tests stay there.
use super::*;
use crate::LootEntryFlags;
use wow_core::guid::HighGuid;

// Only fixture metadata: this suite performs no wire serialization.
const LOOT_TYPE_CORPSE_LIKE_CPP: u8 = 1;

#[test]
fn consumed_ffa_entries_stay_consumed_after_authority_view_rebuild() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.unlooted_count = 0;
    loot.items[0].flags.freeforall = true;
    mark_loot_allowed_for_player_like_cpp(&mut loot, first);
    mark_loot_allowed_for_player_like_cpp(&mut loot, second);
    assert_eq!(loot.unlooted_count, 2);

    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, first);
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(&mut loot);
    assert_eq!(loot.unlooted_count, 1);
    assert!(loot_item_is_looted_for_player_like_cpp(
        &loot,
        &loot.items[0],
        first
    ));
    assert!(!loot_item_is_looted_for_player_like_cpp(
        &loot,
        &loot.items[0],
        second
    ));

    let after_first = loot.clone();
    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, first);
    assert_eq!(loot, after_first);
    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, second);
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(&mut loot);
    assert_eq!(loot.unlooted_count, 0);
    assert!(loot_is_looted_like_cpp(&loot));
}

#[test]
fn shared_generation_preparation_does_not_resurrect_consumed_items() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.items[0].allowed_looters = vec![first, second];
    prepare_represented_shared_loot_generation_like_cpp(&mut loot, &[first, second]);
    assert_eq!(loot.unlooted_count, 1);
    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, first);
    prepare_represented_shared_loot_generation_like_cpp(&mut loot, &[second, first, first]);
    assert_eq!(loot.allowed_looters, vec![first, second]);
    assert_eq!(loot.unlooted_count, 0);
    assert!(loot.items[0].taken);
}

#[test]
fn consuming_an_unknown_slot_leaves_the_generation_unchanged() {
    let player = ObjectGuid::create_player(1, 42);
    let mut loot = authoritative_test_loot_like_cpp(0, true);
    prepare_represented_shared_creature_loot_generation_like_cpp(&mut loot, &[player]);
    let before = loot.clone();
    mark_loot_item_looted_for_player_like_cpp(&mut loot, 99, player);
    assert_eq!(loot, before);
}

#[test]
fn group_threshold_access_still_requires_allowed_looter_membership() {
    let owner = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_method = LOOT_METHOD_GROUP_LIKE_CPP;
    loot.round_robin_player = owner;
    loot.allowed_looters = vec![owner];
    loot.items[0].allowed_looters = vec![owner, other];
    loot.items[0].flags.follow_loot_rules = true;
    assert!(loot_has_over_threshold_item_like_cpp(&loot));
    assert!(!creature_loot_is_allowed_to_player_like_cpp(
        true, false, &loot, other
    ));
    loot.allowed_looters.push(other);
    assert!(creature_loot_is_allowed_to_player_like_cpp(
        true, false, &loot, other
    ));
    loot.items[0].flags.under_threshold = true;
    assert!(!creature_loot_is_allowed_to_player_like_cpp(
        true, false, &loot, other
    ));
}

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

fn authoritative_test_loot_like_cpp(coins: u32, with_item: bool) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins,
        unlooted_count: u8::from(with_item),
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: with_item
            .then(|| LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            })
            .into_iter()
            .collect(),
        looted_by_player: false,
    }
}
#[test]
fn represented_unlooted_count_counts_shared_items_once_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 102);

    let mut entry = represented_loot_entry(0, 25, player_guid);
    entry.allowed_looters.clear();

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
        items: vec![entry],
        looted_by_player: false,
    };

    mark_loot_allowed_for_player_like_cpp(&mut loot, player_guid);
    assert_eq!(loot.unlooted_count, 1);
    assert!(loot.items[0].flags.counted);

    mark_loot_allowed_for_player_like_cpp(&mut loot, other_guid);
    assert_eq!(loot.unlooted_count, 1);

    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
    assert_eq!(loot.unlooted_count, 0);
    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
    assert_eq!(loot.unlooted_count, 0);
}
#[test]
fn creature_loot_visibility_applies_full_cpp_allowed_to_loot_gate() {
    let round_robin_owner = ObjectGuid::create_player(1, 64);
    let other_player = ObjectGuid::create_player(1, 65);
    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_method = LOOT_METHOD_ROUND_ROBIN_LIKE_CPP;
    loot.round_robin_player = round_robin_owner;
    loot.allowed_looters = vec![round_robin_owner, other_player];
    loot.items[0].allowed_looters = vec![round_robin_owner, other_player];
    loot.items[0].flags.follow_loot_rules = true;

    assert!(creature_loot_is_allowed_to_player_like_cpp(
        true,
        false,
        &loot,
        round_robin_owner,
    ));
    assert!(
        !creature_loot_is_allowed_to_player_like_cpp(true, false, &loot, other_player),
        "ordinary shared round-robin loot belongs only to the selected player"
    );
    assert!(
        !creature_loot_is_allowed_to_player_like_cpp(false, false, &loot, round_robin_owner,),
        "C++ rejects loot visibility for a living creature"
    );
    assert!(
        !creature_loot_is_allowed_to_player_like_cpp(true, true, &loot, round_robin_owner,),
        "C++ HasPendingBind suppresses loot visibility"
    );

    loot.items[0].flags.follow_loot_rules = false;
    assert!(
        creature_loot_is_allowed_to_player_like_cpp(true, false, &loot, other_player),
        "quest/conditional/free-for-player loot remains visible outside round robin"
    );
}

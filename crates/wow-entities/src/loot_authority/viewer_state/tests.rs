use super::*;
use crate::{LootEntry, LootEntryFlags, NotNormalLootItem};

fn player(counter: i64) -> ObjectGuid {
    ObjectGuid::create_player(1, counter)
}

fn loot() -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::LootObject,
            0,
            1,
            0,
            0,
            0,
            100,
        ),
        coins: 37,
        unlooted_count: 9,
        loot_type: 1,
        dungeon_encounter_id: 733,
        loot_method: 3,
        loot_master: player(7),
        round_robin_player: player(3),
        player_ffa_items: vec![(
            player(1),
            vec![NotNormalLootItem {
                loot_list_id: 7,
                is_looted: true,
            }],
        )],
        players_looting: Vec::new(),
        allowed_looters: vec![player(1), player(2)],
        items: vec![LootEntry {
            loot_list_id: 7,
            item_id: 25,
            quantity: 3,
            random_properties_id: -77,
            random_properties_seed: 456,
            item_context: 1,
            flags: LootEntryFlags {
                follow_loot_rules: true,
                freeforall: true,
                blocked: true,
                counted: false,
                under_threshold: false,
                needs_quest: true,
            },
            allowed_looters: vec![player(1), player(2)],
            roll_winner: player(2),
            ffa_looted_by: vec![player(1)],
            taken: false,
        }],
        looted_by_player: false,
    }
}

#[test]
fn viewer_membership_accepts_empty_and_preserves_existing_order() {
    let first = player(1);
    let second = player(2);
    let third = player(3);
    let mut pool = loot();
    pool.players_looting = vec![second, first, second];

    assert!(!pool.add_viewer(first));
    assert!(pool.add_viewer(ObjectGuid::EMPTY));
    assert!(!pool.add_viewer(ObjectGuid::EMPTY));
    assert!(pool.add_viewer(third));
    assert_eq!(
        pool.players_looting,
        vec![second, first, second, ObjectGuid::EMPTY, third]
    );

    let mut expected = loot();
    expected.players_looting = vec![second, first, second, ObjectGuid::EMPTY, third];
    assert_eq!(pool, expected);
}

#[test]
fn viewer_registration_and_first_open_are_separate_transitions() {
    let viewer = player(1);
    let mut pool = loot();

    assert!(pool.add_viewer(viewer));
    assert!(!pool.looted_by_player);
    assert!(pool.mark_first_open());
    assert!(pool.looted_by_player);
    assert!(!pool.mark_first_open());

    let mut expected = loot();
    expected.players_looting = vec![viewer];
    expected.looted_by_player = true;
    assert_eq!(pool, expected);
}

#[test]
fn authority_prepare_and_partial_rollback_preserve_previous_state() {
    let viewer = player(1);
    let other = player(2);
    for was_open in [false, true] {
        for had_viewer in [false, true] {
            let mut pool = loot();
            pool.looted_by_player = was_open;
            pool.players_looting = if had_viewer {
                vec![other, viewer, viewer, ObjectGuid::EMPTY]
            } else {
                vec![other, ObjectGuid::EMPTY]
            };

            let (inserted, first_viewer) = pool.prepare_viewer_open(viewer);
            assert_eq!(inserted, !had_viewer);
            assert_eq!(first_viewer, !was_open);
            assert!(pool.looted_by_player);
            assert!(pool.players_looting.contains(&viewer));

            pool.rollback_viewer_open(viewer, inserted, first_viewer);
            let mut expected = loot();
            expected.looted_by_player = was_open;
            expected.players_looting = if had_viewer {
                vec![other, viewer, viewer, ObjectGuid::EMPTY]
            } else {
                vec![other, ObjectGuid::EMPTY]
            };
            assert_eq!(pool, expected);
        }
    }
}

#[test]
fn single_viewer_removal_removes_duplicates_without_resetting_first_open() {
    let first = player(1);
    let second = player(2);
    let mut pool = loot();
    pool.looted_by_player = true;
    pool.players_looting = vec![second, first, ObjectGuid::EMPTY, first, second];

    assert!(pool.remove_viewer(first));
    assert_eq!(
        pool.players_looting,
        vec![second, ObjectGuid::EMPTY, second]
    );
    assert!(!pool.remove_viewer(first));
    assert!(pool.remove_viewer(ObjectGuid::EMPTY));

    let mut expected = loot();
    expected.looted_by_player = true;
    expected.players_looting = vec![second, second];
    assert_eq!(pool, expected);
}

#[test]
fn bulk_viewer_removal_keeps_survivor_order_and_handles_empty_targets() {
    let first = player(1);
    let second = player(2);
    let third = player(3);
    let mut pool = loot();
    pool.players_looting = vec![third, first, second, third, ObjectGuid::EMPTY, first];

    assert!(pool.remove_viewers(&[third, ObjectGuid::EMPTY, third]));
    assert_eq!(pool.players_looting, vec![first, second, first]);
    assert!(!pool.remove_viewers(&[third, ObjectGuid::EMPTY]));
    assert!(!pool.remove_viewers(&[]));

    let mut expected = loot();
    expected.players_looting = vec![first, second, first];
    assert_eq!(pool, expected);
}

#[test]
fn round_robin_release_preserves_empty_equality_and_other_pool_fields() {
    let owner = player(3);
    let other = player(2);
    let mut pool = loot();

    assert!(!pool.release_round_robin(other));
    assert_eq!(pool.round_robin_player, owner);
    assert!(pool.release_round_robin(owner));
    assert!(pool.round_robin_player.is_empty());
    assert!(!pool.release_round_robin(owner));
    assert!(pool.release_round_robin(ObjectGuid::EMPTY));

    let mut expected = loot();
    expected.round_robin_player = ObjectGuid::EMPTY;
    assert_eq!(pool, expected);
}

#[test]
fn reopening_after_last_viewer_close_does_not_repeat_first_open() {
    let first = player(1);
    let second = player(2);
    let mut pool = loot();

    assert_eq!(pool.prepare_viewer_open(first), (true, true));
    assert_eq!(pool.prepare_viewer_open(second), (true, false));
    assert!(pool.remove_viewers(&[first, second]));
    assert!(pool.players_looting.is_empty());
    assert!(pool.looted_by_player);
    assert_eq!(pool.prepare_viewer_open(first), (true, false));

    pool.rollback_viewer_open(first, true, false);
    let mut expected = loot();
    expected.looted_by_player = true;
    assert_eq!(pool, expected);
}

#[test]
fn rejected_open_rollback_restores_only_tentative_viewer_fields() {
    let viewer = ObjectGuid::EMPTY;
    let mut pool = loot();
    let (inserted, first_viewer) = pool.prepare_viewer_open(viewer);
    assert_eq!((inserted, first_viewer), (true, true));

    pool.coins = 11;
    pool.unlooted_count = 4;
    pool.items[0].quantity = 2;
    pool.items[0].taken = true;
    pool.rollback_viewer_open(viewer, inserted, first_viewer);

    let mut expected = loot();
    expected.coins = 11;
    expected.unlooted_count = 4;
    expected.items[0].quantity = 2;
    expected.items[0].taken = true;
    assert_eq!(pool, expected);
}

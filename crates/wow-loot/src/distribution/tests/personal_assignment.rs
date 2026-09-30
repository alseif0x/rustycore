//! Original personal-assignment regression and its test-only adapter.
//! The adapter is the existing Rust bridge, not literal ProcessPersonalLoot.
use super::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

// Original fixture metadata; this owner performs no packet serialization.
const LOOT_TYPE_CHEST_LIKE_CPP: u8 = 9;

fn test_gameobject_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 1, counter)
}

fn loot_object_guid(owner: ObjectGuid) -> ObjectGuid {
    if owner.is_empty() {
        return ObjectGuid::EMPTY;
    }
    ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner.realm_id(),
        owner.map_id(),
        0,
        0,
        owner.counter(),
    )
}

fn assign_represented_personal_loot_items_like_cpp<R: Rng + ?Sized>(
    loot: &mut CreatureLoot,
    tappers: &[ObjectGuid],
    rng: &mut R,
) {
    if tappers.is_empty() {
        return;
    }

    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        entry.allowed_looters.clear();
        entry.flags.counted = false;

        let chosen_tapper = tappers[rng.gen_range(0..tappers.len())];
        entry.add_allowed_looter_like_cpp(chosen_tapper);
    }

    rebuild_represented_personal_loot_counts_like_cpp(loot);
}

#[test]
fn represented_gameobject_personal_encounter_items_are_single_tapper_like_cpp() {
    let first_tapper = ObjectGuid::create_player(1, 42);
    let second_tapper = ObjectGuid::create_player(1, 77);
    let mut loot = CreatureLoot {
        loot_guid: loot_object_guid(test_gameobject_guid(91_014)),
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
        dungeon_encounter_id: 733,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![first_tapper, second_tapper],
        items: vec![
            LootEntry {
                loot_list_id: 0,
                item_id: 1_001,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![first_tapper, second_tapper],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            },
            LootEntry {
                loot_list_id: 1,
                item_id: 1_002,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    freeforall: true,
                    ..LootEntryFlags::default()
                },
                allowed_looters: vec![first_tapper, second_tapper],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: vec![first_tapper],
                taken: false,
            },
        ],
        looted_by_player: false,
    };
    let mut rng = StdRng::seed_from_u64(7);

    assign_represented_personal_loot_items_like_cpp(
        &mut loot,
        &[first_tapper, second_tapper],
        &mut rng,
    );

    assert_eq!(loot.unlooted_count, 2);
    assert_eq!(loot.items[0].allowed_looters.len(), 1);
    assert_eq!(loot.items[1].allowed_looters.len(), 1);
    assert!([first_tapper, second_tapper].contains(&loot.items[0].allowed_looters[0]));
    assert!([first_tapper, second_tapper].contains(&loot.items[1].allowed_looters[0]));
    assert!(loot.items[0].flags.counted);
    assert!(!loot.items[1].flags.counted);
    assert_eq!(loot.items[1].ffa_looted_by, Vec::<ObjectGuid>::new());
    assert_eq!(loot.player_ffa_items.len(), 1);
    assert_eq!(loot.player_ffa_items[0].1[0].loot_list_id, 1);
}

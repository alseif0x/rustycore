use wow_core::{ObjectGuid, guid::HighGuid};
use wow_loot::{
    CreatureLoot, LOOT_METHOD_GROUP_LIKE_CPP, LootEntry, LootEntryFlags,
    prepare_represented_shared_loot_generation_like_cpp,
};
use wow_packet::packets::loot::LOOT_TYPE_CHEST_LIKE_CPP;

#[test]
fn shared_gameobject_normal_item_with_two_looters_counts_once_like_cpp() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let owner = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 1, 91_101);
    let mut entry = LootEntry {
        loot_list_id: 0,
        item_id: 25,
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
        allowed_looters: vec![first],
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    };
    entry.allowed_looters = vec![first, second];
    let mut loot = CreatureLoot {
        loot_guid: ObjectGuid::create_world_object(
            HighGuid::LootObject,
            0,
            owner.realm_id(),
            owner.map_id(),
            0,
            0,
            owner.counter(),
        ),
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
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

    prepare_represented_shared_loot_generation_like_cpp(&mut loot, &[first, second]);

    assert_eq!(loot.allowed_looters, vec![first, second]);
    assert_eq!(loot.items[0].allowed_looters, vec![first, second]);
    assert_eq!(loot.unlooted_count, 1);
    assert!(loot.items[0].flags.counted);
}

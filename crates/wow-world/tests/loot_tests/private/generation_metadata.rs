//! Original loot source and group metadata cases using the same generators.
use super::recovery_support::*;
use wow_loot::LOOT_METHOD_MASTER_LIKE_CPP;
use wow_world::test_fixtures::loot::{
    attach_loot_allocator_for_test as attach_loot_guid_allocator_for_owner,
    generate_creature_money_loot_for_test, gameobject_loot_group_state_for_test,
};

#[tokio::test]
async fn represented_creature_loot_captures_group_method_master_and_round_robin_like_cpp() {
    let mut session = make_session();
    let master_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_049);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);
    install_master_loot_group(&mut session, master_guid, candidate_guid);

    let loot = generate_creature_money_loot_for_test(&mut session, owner_guid, master_guid, 10, 25, 0, 0, 0, 0)
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.loot_method, LOOT_METHOD_MASTER_LIKE_CPP);
    assert_eq!(loot.loot_master, master_guid);
    assert_eq!(loot.round_robin_player, master_guid);
}

#[tokio::test]
async fn represented_creature_loot_generation_carries_cpp_dungeon_encounter_id() {
    let mut session = make_session();
    let owner_guid = test_creature_guid(19_097);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    let loot = generate_creature_money_loot_for_test(&mut session, 
            owner_guid,
            ObjectGuid::create_player(1, 42),
            10,
            25,
            0,
            0,
            0,
            615,
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.dungeon_encounter_id, 615);
}

#[test]
fn represented_gameobject_group_loot_keeps_round_robin_empty_like_cpp() {
    let mut session = make_session();
    let opener = ObjectGuid::create_player(1, 42);
    let candidate = ObjectGuid::create_player(1, 77);
    install_master_loot_group(&mut session, opener, candidate);

    let (loot_method, loot_master, round_robin_player) =
        gameobject_loot_group_state_for_test(&session, true, opener);

    assert_eq!(loot_method, LOOT_METHOD_MASTER_LIKE_CPP);
    assert_eq!(loot_master, opener);
    assert_eq!(round_robin_player, ObjectGuid::EMPTY);
}

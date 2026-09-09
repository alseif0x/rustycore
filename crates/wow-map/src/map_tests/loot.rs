//! Loot scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn remove_from_map_nondelete_detaches_orphaned_typed_loot_authority() {
    let mut map = test_map();
    let player = ObjectGuid::create_player(1, 484_109);
    let mut creature = test_creature_for_spawn(484_109, 4_841_090, true);
    let guid = creature.guid();
    assert!(
        creature
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                guid, 17, player,
            ))
            .installed()
    );
    let retained_authority = creature.loot_authority_like_cpp().clone();
    let lease = poll_immediately_ready(retained_authority.reserve_money_like_cpp(player))
        .expect("the live typed Creature owns the reservation");
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert!(
        removed.object.is_some(),
        "the erased WorldObject remains available to the non-delete caller"
    );
    assert_eq!(
        retained_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached,
        "the returned erased object cannot preserve the destroyed typed loot owner"
    );
    assert!(matches!(
        lease.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration | LootClaimCommitError::RolledBack)
    ));
}

//! Preserved canonical loot lifecycle scenarios.
use super::recovery_support::*;
use std::collections::HashMap;
use wow_entities::ObjectChangedFields;
use wow_world::test_fixtures::loot::*;

#[test]
fn stale_player_map_key_does_not_rebind_creature_loot_authorities_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_709);
    let owner_guid = test_creature_guid(61_710);
    session.set_player_guid(Some(player_guid));
    session.set_state(SessionState::LoggedIn);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    let canonical_creature = make_canonical_creature_for_session(&session, owner_guid);
    attach_canonical_creature(&mut session, canonical_creature);

    let (map_id, instance_id) = legacy_loot_map_key_for_test(&session);
    let map_key = wow_map::MapKey::new(u32::from(map_id), instance_id);
    let legacy_before = legacy_creature_loot_authority_for_test(&session, owner_guid, map_key)
        .expect("the legacy creature owns its pristine authority");
    let canonical_before =
        canonical_creature_loot_authority_for_test(&session, owner_guid, map_key)
            .expect("the canonical creature owns a separate pristine authority");
    assert!(!legacy_before.shares_storage_like_cpp(&canonical_before));
    assert_eq!(canonical_loot_player_map_key_for_test(&session), None);

    assert!(
        loot_recovery_authority_for_test(&mut session, owner_guid).is_none(),
        "a logged-in player between maps must fail closed"
    );

    let legacy_after =
        legacy_creature_loot_authority_for_test(&session, owner_guid, map_key).unwrap();
    let canonical_after =
        canonical_creature_loot_authority_for_test(&session, owner_guid, map_key).unwrap();
    assert!(legacy_after.shares_storage_like_cpp(&legacy_before));
    assert!(canonical_after.shares_storage_like_cpp(&canonical_before));
    assert!(
        !legacy_after.shares_storage_like_cpp(&canonical_after),
        "reconciliation must not mutate either stale-map mirror"
    );
}

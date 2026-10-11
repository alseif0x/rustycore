//! #1263 F6-8C: the loot consumer resolves the canonical designated authority.
//!
//! F6-7 R2/R3 made the designated owner the one authority a loot consumer may
//! resolve, and F6-8C removed the legacy store's reachability from the World
//! loot surface. #1263 F6-8E deleted the legacy store, so the consumer path is
//! canonical-only; this module drives the production consumer
//! (`represented_owned_loot_authority_outcome_like_cpp`) through the found and
//! proven-absent canonical configurations. (The source contract that kept the
//! legacy probe out of production is subsumed by the R3 retired-symbol test.)

use super::{
    attach_addressed_empty_canonical_loot_store_like_cpp, make_session_with_send_capacity,
    register_test_creature_like_cpp, test_creature, test_creature_guid,
};
#[allow(unused_imports)]
use crate::canonical_creature_fixtures::*;
use wow_core::{ObjectGuid, Position};
use wow_world_core::session::OwnedLootAuthorityLookupOutcomeLikeCpp;

const TEST_MAP_ID: u16 = 571;

fn map_key_like_cpp() -> wow_map::MapKey {
    wow_map::MapKey::new(u32::from(TEST_MAP_ID), 0)
}

#[test]
fn loot_consumer_path_resolves_the_canonical_designated_authority_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 65_001);
    let owner_guid = test_creature_guid(65_002);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(TEST_MAP_ID, Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));

    // 1. The registration admitted the canonical incarnation: it is the
    // designated owner and the consumer resolves its own allocation.
    let found = session.represented_owned_loot_authority_outcome_like_cpp(owner_guid);
    let OwnedLootAuthorityLookupOutcomeLikeCpp::Found(canonical_authority) = &found else {
        panic!("the canonical incarnation is the designated owner; got {found:?}");
    };
    assert!(
        session
            .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key_like_cpp())
            .is_some_and(|authority| authority.shares_storage_like_cpp(canonical_authority)),
        "the consumer answers with the canonical incarnation's own allocation"
    );

    // 2. A configured canonical owner whose addressed map holds no incarnation
    // proves the object absent (#1263 F6-8E: there is no other store whose
    // surviving copy could make that unreadable).
    attach_addressed_empty_canonical_loot_store_like_cpp(&mut session);
    let absent = session.represented_owned_loot_authority_outcome_like_cpp(owner_guid);
    assert!(
        matches!(absent, OwnedLootAuthorityLookupOutcomeLikeCpp::Absent),
        "an addressed canonical map without the incarnation is proven absence; got {absent:?}"
    );
}

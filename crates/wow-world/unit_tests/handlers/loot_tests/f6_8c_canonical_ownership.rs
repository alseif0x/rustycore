//! #1263 F6-8C: the loot consumer resolves the canonical designated authority.
//!
//! F6-7 R2/R3 made the designated owner the one authority a loot consumer may
//! resolve: the canonical incarnation whenever a canonical store is configured,
//! and the legacy store only in the legitimate legacy-only configuration. What
//! remained was the *reachability* of the legacy store from the World loot
//! surface. C retires that production seam, so a surviving legacy allocation can
//! no longer answer a consumer, and this module proves the consumer path keeps
//! the same refusal semantics.
//!
//! The first test drives the production consumer
//! (`represented_owned_loot_authority_outcome_like_cpp`) across the three
//! configurations. The second is the source contract that keeps the legacy
//! probe out of production builds.

use super::{
    adopt_registered_creature_as_canonical_incarnation_like_cpp,
    attach_addressed_empty_canonical_loot_store_like_cpp, make_session_with_send_capacity,
    register_test_creature_like_cpp, test_creature, test_creature_guid,
};
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

    // 1. Legitimate legacy-only configuration (no canonical store configured at
    // all): the legacy representation is the designated owner and its
    // allocation is the object's authority. This configuration is unchanged.
    let legacy_only = session.represented_owned_loot_authority_outcome_like_cpp(owner_guid);
    let OwnedLootAuthorityLookupOutcomeLikeCpp::Found(legacy_authority) = &legacy_only else {
        panic!("the legacy-only configuration keeps answering; got {legacy_only:?}");
    };
    let legacy_allocation = legacy_authority.clone();

    // 2. The same live legacy allocation, with a canonical owner configured that
    // holds no incarnation. The consumer must refuse: a surviving legacy copy
    // confers no authority, and an unadmitted representation is not proof that
    // the object is gone either (F6-7 R2/R4).
    attach_addressed_empty_canonical_loot_store_like_cpp(&mut session);
    let refused = session.represented_owned_loot_authority_outcome_like_cpp(owner_guid);
    assert!(
        matches!(refused, OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable),
        "a configured canonical owner without an incarnation is unreadable ownership, not the legacy allocation; got {refused:?}"
    );

    // The refusal is an ownership decision, not a deletion: the legacy
    // representation and its allocation survive it untouched (slice E owns the
    // store's retirement).
    let legacy_manager = session
        .core
        .map_manager
        .clone()
        .expect("the fixture registered the legacy store");
    let legacy_after = session
        .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key_like_cpp())
        .expect("the legacy representation is still published");
    assert!(
        legacy_after.shares_storage_like_cpp(&legacy_allocation),
        "the refusal leaves the legacy allocation exactly as it was"
    );

    // 3. The canonical owner now holds the incarnation: that incarnation is the
    // designated owner, and the consumer resolves its allocation — one
    // incarnation, one allocation, no rebind of either store.
    let canonical = session
        .core
        .canonical_map_manager
        .clone()
        .expect("the fixture attached the canonical owner");
    adopt_registered_creature_as_canonical_incarnation_like_cpp(
        &legacy_manager,
        &canonical,
        owner_guid,
        u32::from(TEST_MAP_ID),
        0,
    );
    let found = session.represented_owned_loot_authority_outcome_like_cpp(owner_guid);
    let OwnedLootAuthorityLookupOutcomeLikeCpp::Found(canonical_authority) = &found else {
        panic!("the canonical incarnation is the designated owner; got {found:?}");
    };
    assert!(
        canonical_authority.shares_storage_like_cpp(&legacy_allocation),
        "the fixture's one incarnation is the allocation the designated owner answers with"
    );
    assert!(
        session
            .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key_like_cpp())
            .is_some_and(|authority| authority.shares_storage_like_cpp(&legacy_allocation)),
        "the canonical store holds that same one incarnation"
    );
}

/// The World loot surface keeps no production path to the legacy store: the
/// single-store legacy probe is `cfg(test)`-gated, so a production consumer
/// cannot compile a reference to it.
#[test]
fn legacy_loot_authority_probe_is_not_a_production_surface_like_cpp() {
    const WORLD_LOOT_OPERATIONS: &str = include_str!("../../../src/session/loot/operations.rs");
    const MARKER: &str = "pub(crate) fn read_legacy_creature_loot_authority_on_map_like_cpp";
    let index = WORLD_LOOT_OPERATIONS
        .find(MARKER)
        .expect("the World loot surface keeps the legacy probe as a named item");
    let head = &WORLD_LOOT_OPERATIONS[..index];
    let gate = head
        .rfind("#[cfg(test)]")
        .expect("the World loot surface must gate the legacy probe on cfg(test)");
    assert!(
        head[gate + "#[cfg(test)]".len()..].trim().is_empty(),
        "the `#[cfg(test)]` attribute must be the gate directly above the legacy probe, and no other item may sit between them"
    );
}

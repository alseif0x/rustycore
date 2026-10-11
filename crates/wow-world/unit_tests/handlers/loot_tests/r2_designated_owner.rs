//! F6-7 R2 regressions: one designated owner with explicit outcomes.
//!
//! Reviewer signature §5.3.1 R2 and repair order §5.3.2: "resolve through that
//! designated authority with incarnation/lifetime validation; survival of
//! either mirror must not confer authority". The lookup is
//! `WorldSession::represented_owned_loot_authority_outcome_like_cpp`.
//!
//! The designated owner is the canonical store (#1263 F6-8E deleted the legacy
//! store and with it the legacy-only and surviving-mirror cases). `Absent` is
//! only ever reported when the designated store was actually addressed and
//! proved the object absent; a missing manager, an unresolved residence, a
//! missing map instance and a quarantined/detached allocation are all
//! `Unavailable`, which is not absent loot.
//!
//! The cases below assert the exact outcome **and** the backing-storage
//! identity: `shares_storage_like_cpp` is `Arc` identity, not state equality.

use super::{
    attach_canonical_creature, make_canonical_creature_for_session,
    make_session_with_send_capacity, test_creature_guid,
};
#[allow(unused_imports)]
use crate::canonical_creature_fixtures::*;
use crate::session::{SessionState, WorldSession};
use wow_core::{ObjectGuid, Position};
use wow_loot::OwnedLootAuthority;
use wow_map::MapKey;
use wow_world_core::session::OwnedLootAuthorityLookupOutcomeLikeCpp;

const TEST_MAP_ID: u16 = 571;

/// The designated-owner configurations R2 must decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesignatedOwnerCaseLikeCpp {
    /// The canonical incarnation exists.
    CanonicalOnly,
    /// The store is readable, the object is not in it: proven absence.
    MissingOwner,
    /// No store is configured at all: no owner source can be addressed, so the
    /// object is neither found nor proved absent.
    NoOwnerStoreConfigured,
    /// The canonical store is configured but the session's residence cannot be
    /// resolved. That may not be answered as absence.
    UnreadableOwner,
    /// The designated incarnation's allocation was displaced by a replacement.
    IncarnationReplacement,
}

/// The fact each case must produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedDesignatedOwnerLikeCpp {
    CanonicalAllocation,
    Absent,
    Unavailable,
}

struct DesignatedOwnerFixtureLikeCpp {
    session: WorldSession,
    owner_guid: ObjectGuid,
    canonical: Option<OwnedLootAuthority>,
}

fn map_key_like_cpp() -> MapKey {
    MapKey::new(u32::from(TEST_MAP_ID), 0)
}

fn owner_session_like_cpp(counter: i64) -> (WorldSession, ObjectGuid) {
    let (mut session, _send_rx) = make_session_with_send_capacity(2);
    let owner_guid = test_creature_guid(counter);
    session.set_player_guid(Some(ObjectGuid::create_player(1, counter)));
    session.set_player_map_position_like_cpp(TEST_MAP_ID, Position::ZERO);
    (session, owner_guid)
}

fn designated_owner_fixture_like_cpp(
    case: DesignatedOwnerCaseLikeCpp,
    counter: i64,
) -> DesignatedOwnerFixtureLikeCpp {
    let (mut session, owner_guid) = owner_session_like_cpp(counter);
    match case {
        DesignatedOwnerCaseLikeCpp::CanonicalOnly => {
            let canonical = make_canonical_creature_for_session(&session, owner_guid);
            attach_canonical_creature(&mut session, canonical);
        }
        DesignatedOwnerCaseLikeCpp::MissingOwner => {
            // The canonical map instance exists but holds a *different* GUID,
            // so this GUID is proven absent from the designated owner.
            let other_guid = test_creature_guid(counter + 1);
            let canonical = make_canonical_creature_for_session(&session, other_guid);
            attach_canonical_creature(&mut session, canonical);
        }
        DesignatedOwnerCaseLikeCpp::NoOwnerStoreConfigured => {}
        DesignatedOwnerCaseLikeCpp::UnreadableOwner => {
            // A configured canonical store whose player residence cannot be
            // resolved (logged in, not in any canonical map): the map key the
            // designated owner resolves under does not resolve at all.
            let canonical = make_canonical_creature_for_session(&session, owner_guid);
            attach_canonical_creature(&mut session, canonical);
            session.set_state(SessionState::LoggedIn);
        }
        DesignatedOwnerCaseLikeCpp::IncarnationReplacement => {
            let canonical = make_canonical_creature_for_session(&session, owner_guid);
            attach_canonical_creature(&mut session, canonical);
            // The replacement incarnation displaced this allocation from its
            // owning entity mirror; it can never own loot again.
            let displaced = session
                .read_canonical_creature_loot_authority_on_map_like_cpp(
                    owner_guid,
                    map_key_like_cpp(),
                )
                .expect("the canonical incarnation holds an allocation to displace");
            displaced.detach_like_cpp();
        }
    }
    let canonical = session
        .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key_like_cpp());
    DesignatedOwnerFixtureLikeCpp {
        session,
        owner_guid,
        canonical,
    }
}

#[test]
fn lookup_uses_designated_incarnation_authority() {
    let table = [
        (
            DesignatedOwnerCaseLikeCpp::CanonicalOnly,
            ExpectedDesignatedOwnerLikeCpp::CanonicalAllocation,
            "canonical-only: the canonical incarnation is the designated owner",
        ),
        (
            DesignatedOwnerCaseLikeCpp::MissingOwner,
            ExpectedDesignatedOwnerLikeCpp::Absent,
            "absence is proven only by the designated owner that was addressed",
        ),
        (
            DesignatedOwnerCaseLikeCpp::NoOwnerStoreConfigured,
            ExpectedDesignatedOwnerLikeCpp::Unavailable,
            "no owner store configured means nothing was addressed: absence is not proven",
        ),
        (
            DesignatedOwnerCaseLikeCpp::UnreadableOwner,
            ExpectedDesignatedOwnerLikeCpp::Unavailable,
            "an unresolved residence is never read as absence",
        ),
        (
            DesignatedOwnerCaseLikeCpp::IncarnationReplacement,
            ExpectedDesignatedOwnerLikeCpp::Unavailable,
            "an allocation of a replaced incarnation cannot confer authority",
        ),
    ];

    for (index, (case, expected, description)) in table.into_iter().enumerate() {
        let counter = 63_000 + i64::try_from(index).expect("small table index") * 10;
        let mut fixture = designated_owner_fixture_like_cpp(case, counter);
        let canonical_before = fixture
            .canonical
            .as_ref()
            .map(OwnedLootAuthority::stamp_like_cpp);
        // `Absent` is only reported for the case where the store was addressed
        // and is readable for this GUID but holds no allocation: the
        // unconfigured-store case is `Unavailable`, because an unaddressed
        // store proves nothing.
        if matches!(
            expected,
            ExpectedDesignatedOwnerLikeCpp::CanonicalAllocation
        ) {
            assert!(
                fixture.canonical.is_some(),
                "{case:?}: the fixture must expose the canonical incarnation's allocation"
            );
        }

        let outcome = fixture
            .session
            .represented_owned_loot_authority_outcome_like_cpp(fixture.owner_guid);

        match expected {
            ExpectedDesignatedOwnerLikeCpp::CanonicalAllocation => {
                let OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority) = &outcome else {
                    panic!("{case:?}: {description}; got {outcome:?}");
                };
                let canonical = fixture
                    .canonical
                    .as_ref()
                    .expect("the canonical incarnation exists");
                assert!(
                    authority.shares_storage_like_cpp(canonical),
                    "{case:?}: the found authority must be the canonical incarnation's own allocation"
                );
            }
            ExpectedDesignatedOwnerLikeCpp::Absent => {
                assert!(
                    matches!(outcome, OwnedLootAuthorityLookupOutcomeLikeCpp::Absent),
                    "{case:?}: {description}; got {outcome:?}"
                );
                assert!(
                    fixture.canonical.is_none(),
                    "{case:?}: proven absence means the store holds no allocation to identify"
                );
            }
            ExpectedDesignatedOwnerLikeCpp::Unavailable => {
                assert!(
                    matches!(outcome, OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable),
                    "{case:?}: {description}; got {outcome:?}"
                );
                if case == DesignatedOwnerCaseLikeCpp::NoOwnerStoreConfigured {
                    assert!(
                        fixture.canonical.is_none(),
                        "the no-owner-store case must have no store that could hold an allocation"
                    );
                    assert!(
                        fixture.session.core.canonical_map_manager.is_none(),
                        "the no-owner-store case must configure no designated store"
                    );
                }
            }
        }

        // R2 is a lookup, not a reconciliation: whatever it answered, the store
        // keeps the exact allocation and stamp it had.
        let canonical_after = fixture
            .session
            .read_canonical_creature_loot_authority_on_map_like_cpp(
                fixture.owner_guid,
                map_key_like_cpp(),
            )
            .as_ref()
            .map(OwnedLootAuthority::stamp_like_cpp);
        assert_eq!(
            canonical_before, canonical_after,
            "{case:?}: the designated store's allocation and stamp must be untouched"
        );
    }
}

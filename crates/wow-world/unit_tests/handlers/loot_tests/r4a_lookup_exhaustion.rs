//! F6-7 R4a regressions: reconciliation exhaustion is not absent loot.
//!
//! Reviewer signature §5.3.1 R4 and repair order §5.3.2: the object-owned
//! authority lookup must separate "no authority" from "the dual-store
//! reconciliation did not converge", and the loot-open consumer must reject an
//! exhausted attempt without publishing packets or changing session loot-view
//! state. These tests drive the *production* consumer
//! (`requests.rs::represented_on_loot_opened_with_catalogs_like_cpp`), not a
//! test double.
//!
//! Exhaustion is forced deterministically, without sleeps, threads or fixture
//! overrides: the reconciliation loop re-checks
//! `loot_reconciliation_map_key_still_valid_like_cpp` after every phase and
//! `continue`s while the key it reconciles under is no longer the key the
//! session resolves (`wow-world-loot/src/state/authority_access.rs`). A
//! logged-in session whose canonical player is not in the canonical map fails
//! that lookup closed (`instances/map_resolution.rs:70-72`), so all eight
//! attempts lose their key while both mirrors stay readable at the legacy
//! fallback key — the loop then reports `Unavailable` instead of the old
//! `None`. Whether that key loss occurs in production is unverified (§5.3 Q2);
//! this slice bounds the behaviour when it does.

use super::{
    attach_canonical_creature, authoritative_test_loot_like_cpp,
    authoritative_test_loot_response_like_cpp,
    install_cached_test_creature_loot_authority_like_cpp, make_canonical_creature_for_session,
    make_session_with_send_capacity, register_test_creature_like_cpp,
    represented_loot_object_guid_like_cpp, test_creature, test_creature_guid,
};
use crate::session::{SessionState, WorldSession};
use wow_core::{ObjectGuid, Position};
use wow_loot::OwnedLootAuthority;
use wow_map::MapKey;
use wow_world_core::session::OwnedLootAuthorityLookupOutcomeLikeCpp;

/// Everything a rejected loot open must leave untouched: the represented loot
/// cache and its generation, the active loot GUID, the represented viewers and
/// their view generation, the recorded view authority, and both mirrors'
/// generations (with their storage identity, which reconciliation would
/// change).
#[derive(Debug, PartialEq, Eq)]
struct LootViewStateLikeCpp {
    cached_present: bool,
    cached_generation: Option<u64>,
    cached_coins: Option<u32>,
    cached_first_open: Option<bool>,
    active_loot_guid: ObjectGuid,
    viewers: Vec<ObjectGuid>,
    view_generations: Vec<(ObjectGuid, u64)>,
    view_authority_is_legacy_mirror: bool,
    legacy_mirror_generation: Option<u64>,
    legacy_mirror_coins: Option<u32>,
    legacy_mirror_viewers: usize,
    canonical_mirror_generation: Option<u64>,
    mirrors_are_distinct: bool,
}

fn mirror_authorities_like_cpp(
    session: &WorldSession,
    owner_guid: ObjectGuid,
) -> (Option<OwnedLootAuthority>, Option<OwnedLootAuthority>) {
    let map_key = MapKey::new(u32::from(session.core.player_map_id_like_cpp()), 0);
    let access = session.core.loot_release_access_like_cpp();
    (
        access.read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key),
        access.read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key),
    )
}

fn loot_view_state_like_cpp(
    session: &WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> LootViewStateLikeCpp {
    let (legacy, canonical) = mirror_authorities_like_cpp(session, owner_guid);
    let mut viewers = session.loot.active_loot_view_owners_snapshot_like_cpp();
    viewers.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
    let active_authority = session.loot.active_loot_view_authority_like_cpp(owner_guid);
    LootViewStateLikeCpp {
        cached_present: session.loot.cached_loot_contains_owner_like_cpp(owner_guid),
        cached_generation: session
            .loot
            .represented_loot_cache_generation_for_test_like_cpp(owner_guid)
            .copied(),
        cached_coins: session
            .loot
            .cached_loot_for_owner_like_cpp(owner_guid)
            .map(|loot| loot.coins),
        cached_first_open: session
            .loot
            .cached_loot_for_owner_like_cpp(owner_guid)
            .map(|loot| loot.looted_by_player),
        active_loot_guid: session.loot.active_loot_guid_like_cpp(),
        viewers,
        view_generations: session
            .loot
            .active_loot_view_owners_snapshot_like_cpp()
            .into_iter()
            .filter_map(|owner| {
                session
                    .loot
                    .active_loot_view_generation_like_cpp(owner)
                    .copied()
                    .map(|generation| (owner, generation))
            })
            .collect(),
        view_authority_is_legacy_mirror: active_authority.is_some_and(|authority| {
            legacy
                .as_ref()
                .is_some_and(|legacy| authority.shares_storage_like_cpp(legacy))
        }),
        legacy_mirror_generation: legacy
            .as_ref()
            .and_then(|authority| authority.snapshot_for_player_like_cpp(player_guid))
            .map(|snapshot| snapshot.generation),
        legacy_mirror_coins: legacy
            .as_ref()
            .and_then(|authority| authority.snapshot_for_player_like_cpp(player_guid))
            .map(|snapshot| snapshot.loot.coins),
        legacy_mirror_viewers: legacy
            .as_ref()
            .and_then(|authority| authority.snapshot_for_player_like_cpp(player_guid))
            .map(|snapshot| snapshot.loot.players_looting.len())
            .unwrap_or_default(),
        canonical_mirror_generation: canonical
            .as_ref()
            .and_then(|authority| authority.snapshot_for_player_like_cpp(player_guid))
            .map(|snapshot| snapshot.generation),
        mirrors_are_distinct: matches!(
            (&legacy, &canonical),
            (Some(legacy), Some(canonical)) if !legacy.shares_storage_like_cpp(canonical)
        ),
    }
}

/// Seed an openable creature loot view (cache, mirrors at the legacy key,
/// active represented guid) and then break the map key the reconciliation
/// uses, so every attempt of the eight-round loop is refused by the still-valid
/// gate. The authoritative loot is installed while the canonical map is still
/// absent, exactly as the existing packet fixtures do it.
fn exhausted_reconciliation_session_like_cpp() -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    ObjectGuid,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 61_950);
    let owner_guid = test_creature_guid(61_951);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));

    let mut loot = authoritative_test_loot_like_cpp(7, true);
    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![player_guid];
    loot.items[0].allowed_looters = vec![player_guid];
    session
        .loot
        .insert_cached_loot_for_owner_like_cpp(owner_guid, loot);
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    let legacy_mirror = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .expect("the legacy mirror must hold the installed authority");
    session.loot.set_active_loot_guid(owner_guid);
    session
        .loot
        .insert_active_loot_view_authority_for_test_like_cpp(owner_guid, legacy_mirror);

    // A second, independently allocated mirror appears at the same key, and the
    // session is logged in with no canonical player, so the key it reconciles
    // under stops resolving: every attempt is refused before any rebind.
    let canonical_mirror = make_canonical_creature_for_session(&session, owner_guid);
    attach_canonical_creature(&mut session, canonical_mirror);
    session.set_state(SessionState::LoggedIn);
    (session, send_rx, owner_guid, player_guid)
}

#[test]
fn exhausted_reconciliation_rejects_loot_open_without_side_effects_like_cpp() {
    let (mut session, send_rx, owner_guid, player_guid) =
        exhausted_reconciliation_session_like_cpp();
    let before = loot_view_state_like_cpp(&session, owner_guid, player_guid);
    assert!(
        before.cached_present && before.legacy_mirror_generation.is_some(),
        "the fixture must expose an openable authority behind a readable mirror"
    );
    assert!(
        before.mirrors_are_distinct,
        "the fixture must hold two independently allocated mirrors"
    );
    assert!(send_rx.try_recv().is_err(), "the fixture queues no packet");

    assert!(
        matches!(
            session.represented_owned_loot_authority_outcome_like_cpp(owner_guid),
            OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable
        ),
        "all eight reconciliation attempts must be refused by the still-valid gate"
    );
    assert!(
        session
            .represented_owned_loot_authority_like_cpp(owner_guid)
            .is_none(),
        "the compatibility wrapper stays fail-closed for untouched consumers"
    );

    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        session
            .loot
            .cached_loot_for_owner_like_cpp(owner_guid)
            .expect("the fixture seeds the loot cache"),
        player_guid,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);

    assert!(
        send_rx.try_recv().is_err(),
        "an exhausted reconciliation must publish no packet"
    );
    assert_eq!(
        loot_view_state_like_cpp(&session, owner_guid, player_guid),
        before,
        "the rejected attempt must not change cache, active guid, viewers or generations"
    );
}

#[test]
fn absent_loot_authority_keeps_the_existing_open_and_release_paths_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 61_960);
    let owner_guid = test_creature_guid(61_961);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_state(SessionState::LoggedIn);

    // No creature, map or mirror at all: this is genuine absence, not a failed
    // reconciliation.
    assert!(matches!(
        session.represented_owned_loot_authority_outcome_like_cpp(owner_guid),
        OwnedLootAuthorityLookupOutcomeLikeCpp::Absent
    ));

    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &authoritative_test_loot_like_cpp(0, false),
        player_guid,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);
    assert!(
        send_rx.try_recv().is_ok(),
        "absence keeps today's fallback, which still answers this open attempt"
    );
    assert!(send_rx.try_recv().is_err());

    // The release reply the non-fixture absence branch uses is unchanged; it is
    // exercised directly because that consumer branch is `!cfg!(test)`-gated.
    session.loot.set_active_loot_guid(owner_guid);
    session
        .loot
        .ensure_active_loot_view_generation_like_cpp(owner_guid, 3);
    session.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
    assert!(
        send_rx.try_recv().is_ok(),
        "the release reply still publishes"
    );
    assert!(!session.loot.has_active_loot_view_owner_like_cpp(owner_guid));
    assert!(session.loot.active_loot_guid_like_cpp().is_empty());
}

#[test]
fn found_loot_authority_still_opens_and_records_the_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 61_970);
    let owner_guid = test_creature_guid(61_971);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));

    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![player_guid];
    loot.items[0].allowed_looters = vec![player_guid];
    session
        .loot
        .insert_cached_loot_for_owner_like_cpp(owner_guid, loot);
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.loot.set_active_loot_guid(owner_guid);

    let outcome = session
        .prepare_owned_loot_authority_for_active_request_outcome_like_cpp(owner_guid, player_guid);
    assert!(
        matches!(outcome, OwnedLootAuthorityLookupOutcomeLikeCpp::Found(_)),
        "a single readable mirror is still a found authority"
    );

    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        session
            .loot
            .cached_loot_for_owner_like_cpp(owner_guid)
            .expect("the fixture seeds the loot cache"),
        player_guid,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);

    assert!(send_rx.try_recv().is_ok(), "a successful open publishes");
    assert!(session.loot.cached_loot_contains_owner_like_cpp(owner_guid));
    assert!(session.loot.has_active_loot_view_owner_like_cpp(owner_guid));
    assert!(
        session
            .loot
            .active_loot_view_authority_like_cpp(owner_guid)
            .is_some()
    );
    assert!(
        session
            .loot
            .active_loot_view_generation_like_cpp(owner_guid)
            .is_some()
    );
}

#[test]
fn rejected_response_enqueue_still_rolls_back_the_open_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    // Dropping the receiver makes the socket queue reject the enqueue
    // immediately, without blocking and without a saturated-queue race.
    drop(send_rx);
    let player_guid = ObjectGuid::create_player(1, 61_980);
    let owner_guid = test_creature_guid(61_981);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));

    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![player_guid];
    loot.items[0].allowed_looters = vec![player_guid];
    session
        .loot
        .insert_cached_loot_for_owner_like_cpp(owner_guid, loot);
    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .expect("the fixture installs a readable authority");
    session.loot.set_active_loot_guid(owner_guid);

    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        session
            .loot
            .cached_loot_for_owner_like_cpp(owner_guid)
            .expect("the fixture seeds the loot cache"),
        player_guid,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);

    assert!(!session.loot.cached_loot_contains_owner_like_cpp(owner_guid));
    assert!(!session.loot.has_active_loot_view_owner_like_cpp(owner_guid));
    assert!(session.loot.active_loot_guid_like_cpp().is_empty());
    let rolled_back = authority
        .snapshot_for_player_like_cpp(player_guid)
        .expect("the installed authority keeps its snapshot");
    assert!(rolled_back.loot.players_looting.is_empty());
    assert!(!rolled_back.loot.looted_by_player);
}

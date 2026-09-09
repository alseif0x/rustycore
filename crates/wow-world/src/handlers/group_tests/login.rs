//! Login scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn session_combat_transition_updates_the_registry_member_view_like_cpp() {
    // The LFG combat gate can only read members if their registry view
    // tracks the owning session's transitions.
    let guid = ObjectGuid::create_player(1, 42);
    let (mut session, _send_rx) = make_session_with_send();
    session.set_player_guid(Some(guid));
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (tx, _rx) = flume::bounded(8);
    player_registry.register_or_replace(
        guid,
        broadcast_info_with_command_tx(guid, tx, session.session_command_tx()),
        Default::default(),
    );
    let canonical = bind_canonical_party_players_like_cpp(&player_registry, [guid]);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&player_registry));

    session.set_in_combat_like_cpp(true);
    assert!(
        player_registry
            .group_presence(guid)
            .expect("member")
            .in_combat
    );
    session.set_in_combat_like_cpp(false);
    assert!(
        !player_registry
            .group_presence(guid)
            .expect("member")
            .in_combat
    );
}

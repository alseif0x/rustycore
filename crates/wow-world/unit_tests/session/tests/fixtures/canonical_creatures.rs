//! The single-phase driver that delivers player melee results to a session.
//!
//! #1363 slice 3b moved the session-free canonical phase drivers to
//! `wow_world_creature::canonical_creatures`; this one needs a `WorldSession`.

use super::*;

/// Run the map-owned player melee phase for this session's player the way the
/// canonical map loop does (C++ `Player::Update` → `DoMeleeAttackIfReady`),
/// then deliver its results to the session as the registry would.
///
/// #1263 F6-8E replaced the per-session combat tick with this phase.
pub(in crate::session::tests) fn run_session_player_melee_phase_for_test_like_cpp(
    session: &mut WorldSession,
    diff_ms: u32,
) -> PlayerMeleeTickOutcomeLikeCpp {
    let player_guid = session
        .player_guid()
        .expect("the player melee phase needs a player");
    let canonical = session
        .core
        .canonical_map_manager
        .clone()
        .expect("the player melee phase runs on the canonical map");
    let (map_id, instance_id) = session.core.current_runtime_map_key_like_cpp();
    // The phase reaches the session through its own outbound and command
    // channels, as through its registry registration in production.
    let registration = PlayerRegistry::new().register_or_replace(
        player_guid,
        broadcast_info_with_command(
            player_guid,
            session.send_tx().clone(),
            session.core.session_command_tx.clone(),
        ),
        Default::default(),
    );
    let attackers = vec![PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid,
        map_id,
        instance_id,
        in_combat_mirror: crate::session::hub_ref(session)
            .resolved_in_combat_like_cpp()
            .unwrap_or(false),
        tap_group_guids: Vec::new(),
    }];
    let config = session.config.creature_aggro_config_like_cpp.clone();
    let mut phase_state = PlayerMeleePhaseStateLikeCpp::default();
    let outcome = run_canonical_player_melee_phase_for_test_like_cpp(
        &canonical,
        &attackers,
        diff_ms,
        &mut phase_state,
        &config,
    );
    // The registry delivers to an in-world session; its handler re-gates on
    // `LoggedIn`, which these fixture sessions model by construction.
    if session.state() != SessionState::LoggedIn {
        session.set_state(SessionState::LoggedIn);
    }
    for command in outcome.commands.clone() {
        wow_world_entities::handle_apply_player_melee_result_like_cpp_command_like_cpp(
            session, command,
        );
    }
    outcome
}

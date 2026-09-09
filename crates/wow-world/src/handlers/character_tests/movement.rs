//! Movement scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn total_stat_percentage_ability_preserves_health_pct_on_apply_and_remove_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 82);
    let spell_id = 90_082;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1_320, 5, 10);
    session.set_player_health_like_cpp(5, 10);
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, true,
    )));
    session.set_state(crate::session::SessionState::LoggedIn);

    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply stamina ability");
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((10, 20)),
        "C++ restores 50% health after the stamina ability raises max health"
    );

    let slot = session
        .visible_aura_slot_for_spell_like_cpp(spell_id)
        .expect("stamina ability aura slot");
    session.remove_aura(slot).expect("remove stamina ability");
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((5, 10)),
        "C++ restores 50% health after the stamina ability lowers max health"
    );
}
#[tokio::test]
async fn hearth_and_resurrect_allowed_area_resurrects_and_teleports_home_like_cpp() {
    let (mut session, send_rx) = make_hearth_and_resurrect_session(
        wow_data::AREA_FLAG_ALLOW_HEARTH_AND_RESURRECT_FROM_AREA_LIKE_CPP,
    );

    session
        .handle_hearth_and_resurrect(WorldPacket::new_empty())
        .await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(
        std::iter::from_fn(|| send_rx.try_recv().ok())
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![
            wow_constants::ServerOpcodes::CancelCombat as u16,
            wow_constants::ServerOpcodes::MoveTeleport as u16,
        ]
    );
}

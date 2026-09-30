//! Original loot admission and cast/aura interruption scenarios.
use super::support::*;
use wow_world::test_fixtures::loot::{prepare_money_player_residence_for_test, install_loot_interruptible_cast_for_test, install_loot_interrupt_aura_for_test, loot_cast_pending_for_test, loot_aura_slot_present_for_test};
const SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP: u32 = 0x0000_0800;

#[tokio::test]
async fn loot_unit_dead_player_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_033);
    session.set_player_guid(Some(player_guid));
    prepare_money_player_residence_for_test(&mut session);
    session.set_player_alive_like_cpp(false);
    install_loot_interruptible_cast_for_test(&mut session, player_guid);
    install_loot_interrupt_aura_for_test(
        &mut session,
        3,
        777,
        player_guid,
        SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP,
    );
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

    handle_loot_unit_for_test(&mut session, loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!is_active_loot_guid_for_test(&session, loot_guid));
    assert!(!has_loot_for_test(&session, loot_guid));
    assert!(loot_cast_pending_for_test(&session));
    assert!(loot_aura_slot_present_for_test(&session, 3));
}

#[tokio::test]
async fn loot_unit_valid_target_interrupts_active_cast_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_034);
    session.set_player_guid(Some(player_guid));
    prepare_money_player_residence_for_test(&mut session);
    install_loot_interruptible_cast_for_test(&mut session, player_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

    handle_loot_unit_for_test(&mut session, loot_unit_packet(loot_guid)).await;

    assert!(!loot_cast_pending_for_test(&session));
}

#[tokio::test]
async fn loot_unit_valid_target_removes_looting_interrupt_auras_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_035);
    session.set_player_guid(Some(player_guid));
    prepare_money_player_residence_for_test(&mut session);
    install_loot_interrupt_aura_for_test(
        &mut session,
        3,
        777,
        player_guid,
        SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP,
    );
    install_loot_interrupt_aura_for_test(&mut session, 4, 778, player_guid, 0);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

    handle_loot_unit_for_test(&mut session, loot_unit_packet(loot_guid)).await;

    assert!(!loot_aura_slot_present_for_test(&session, 3));
    assert!(loot_aura_slot_present_for_test(&session, 4));
}


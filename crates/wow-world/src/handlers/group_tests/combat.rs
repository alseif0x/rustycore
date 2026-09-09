//! Combat scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn lfg_uninvite_in_combat_returns_code_without_removal_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);
    session.in_combat = true;

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("in combat result")),
        party_result::PARTY_LFG_BOOT_IN_COMBAT
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}
#[tokio::test]
async fn lfg_uninvite_member_in_combat_returns_code_without_removal_like_cpp() {
    // C++ checks every member on the uninviter's map: another member in
    // combat blocks the kick even when the uninviter is at peace.
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, _target_rx) = flume::bounded(8);
    let target_info = broadcast_info(target, target_tx);
    player_registry.register_or_replace(target, target_info, Default::default());
    let canonical = bind_canonical_party_players_like_cpp(&player_registry, [target]);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(target)
        .unwrap()
        .unit_mut()
        .set_unit_flags_like_cpp(wow_constants::UnitFlags::IN_COMBAT);
    session.set_player_registry(Arc::clone(&player_registry));
    assert!(!session.in_combat);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("in combat result")),
        party_result::PARTY_LFG_BOOT_IN_COMBAT
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}
#[tokio::test]
async fn lfg_uninvite_member_in_combat_on_another_map_passes_gate_like_cpp() {
    // C++ only counts members in the uninviter's map (`IsInMap(this)`):
    // a member fighting on another map does not block the kick.
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, _target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.placement.map_id = 1;
    target_info.placement.instance_id = 1;
    player_registry.register_or_replace(target, target_info, Default::default());
    session.set_player_registry(Arc::clone(&player_registry));

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ ignores combat on other maps: the gate passes silently into the vote-owned swallow"
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}

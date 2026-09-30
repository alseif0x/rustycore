//! Loot scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn party_update_sends_master_looter_only_for_master_loot_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let master = ObjectGuid::create_player(1, 77);
    let (tx, rx) = bounded(8);
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    registry.register_or_replace(leader, broadcast_info(leader, tx), Default::default());
    let mut group = GroupInfo::new(leader);
    group.loot_method = 2;
    group.master_looter_guid = master;
    let master_bytes = packed_guid_bytes(master);

    send_party_update(&group, &registry, 0);

    let sent = recv_dispatched_packet(&rx, "raid PartyUpdate");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::PartyUpdate as u16
    );
    assert!(
        sent.windows(master_bytes.len())
            .any(|window| window == master_bytes.as_slice())
    );

    let (tx, rx) = bounded(8);
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    registry.register_or_replace(leader, broadcast_info(leader, tx), Default::default());
    group.loot_method = 0;

    send_party_update(&group, &registry, 0);

    let sent = recv_dispatched_packet(&rx, "non-master-loot PartyUpdate");
    assert!(
        !sent
            .windows(master_bytes.len())
            .any(|window| window == master_bytes.as_slice())
    );
}
#[tokio::test]
async fn party_invite_closed_command_channel_rolls_back_pending_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());
    session.set_player_guid(Some(inviter));
    session.set_loaded_player_name_like_cpp("Leader".to_string());

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_send_tx, target_send_rx) = bounded(8);
    let (target_command_tx, target_command_rx) = bounded(1);
    drop(target_command_rx);
    player_registry.register_or_replace(
        target,
        broadcast_info_with_command_tx(target, target_send_tx, target_command_tx),
        Default::default(),
    );
    let pending = Arc::new(PendingInvites::default());
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::new(GroupRegistry::default()), Arc::clone(&pending));

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("failed invite result")),
        party_result::BAD_PLAYER_NAME
    );
    assert!(pending.get(&target).is_none());
    assert!(pending.get(&inviter).is_none());
    assert!(target_send_rx.try_recv().is_err());
}
#[tokio::test]
async fn lfg_uninvite_target_loot_rolls_returns_code_without_removal_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 100);
    let target = ObjectGuid::create_player(1, 101);
    let group = lfg_group_like_cpp(leader, 5);
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, sender);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, _target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.active_loot_rolls.push(
        crate::session::mailbox::LootRollCommandIdentityLikeCpp::new_like_cpp(
            ObjectGuid::create_item(1, 9001),
            1,
            wow_loot::OwnedLootAuthority::default(),
            1,
        ),
    );
    player_registry.register_or_replace(target, target_info, Default::default());
    session.set_player_registry(player_registry);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "boot"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("loot rolls result")),
        party_result::PARTY_LFG_BOOT_LOOT_ROLLS
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
async fn set_loot_method_is_represented_noop_like_this_cpp_branch() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let requested_master = ObjectGuid::create_player(1, 77);
    let original_master = ObjectGuid::create_player(1, 88);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.loot_method = 2;
    group.master_looter_guid = original_master;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_set_loot_method(set_loot_method_packet(true, 0, requested_master, 4))
        .await;

    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(group.loot_method, 2);
    assert_eq!(group.master_looter_guid, original_master);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn opt_out_of_loot_sets_pass_on_group_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    assert!(!session.pass_on_group_loot);

    session
        .handle_opt_out_of_loot(opt_out_of_loot_packet(true))
        .await;

    assert!(session.pass_on_group_loot);
    assert!(send_rx.try_recv().is_err());

    session
        .handle_opt_out_of_loot(opt_out_of_loot_packet(false))
        .await;

    assert!(!session.pass_on_group_loot);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn opt_out_of_loot_without_loaded_player_is_ignored_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();

    session
        .handle_opt_out_of_loot(opt_out_of_loot_packet(true))
        .await;

    assert!(!session.pass_on_group_loot);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn random_roll_without_group_sends_to_self_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let sender = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(sender));

    session
        .handle_random_roll(random_roll_packet(1, 100, None))
        .await;

    let sent = send_rx
        .try_recv()
        .expect("solo random roll should be sent to self");
    assert_random_roll_packet(&sent, sender, 1, 1, 100);
}
#[tokio::test]
async fn random_roll_ignores_party_index_for_home_group_lookup_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let sender = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender);
    group.add_member(other);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (sender_tx, sender_rx) = bounded(8);
    let (other_tx, other_rx) = bounded(8);
    player_registry.register_or_replace(
        sender,
        broadcast_info(sender, sender_tx),
        Default::default(),
    );
    player_registry.register_or_replace(other, broadcast_info(other, other_tx), Default::default());

    session.set_player_guid(Some(sender));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_random_roll(random_roll_packet(1, 2, Some(1)))
        .await;

    assert!(
        sender_rx.try_recv().is_ok(),
        "RandomRoll parses but ignores PartyIndex like C++ DoRandomRoll/GetGroup"
    );
    assert!(
        other_rx.try_recv().is_ok(),
        "RandomRoll must still broadcast to represented HOME group"
    );
}
#[tokio::test]
async fn random_roll_rejects_invalid_bounds_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_random_roll(random_roll_packet(100, 1, None))
        .await;
    session
        .handle_random_roll(random_roll_packet(1, 1_000_001, None))
        .await;

    assert!(send_rx.try_recv().is_err());
}

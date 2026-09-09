//! Quest scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn raid_target_list_request_sends_all_icons_to_caller_without_mutation_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let marked = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.target_icons[2] = marked.to_raw_bytes();
    let original_icons = group.target_icons;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_update_raid_target(update_raid_target_packet(ObjectGuid::EMPTY, -1, None))
        .await;

    let sent = send_rx.try_recv().expect("target icon list to caller");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::SendRaidTargetUpdateAll as u16
    );
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 8);
    for symbol in 0..8 {
        let target = pkt.read_packed_guid().unwrap();
        assert_eq!(pkt.read_uint8().unwrap(), symbol);
        if symbol == 2 {
            assert_eq!(target, marked);
        }
    }
    assert_eq!(
        group_registry.get(&group_guid).unwrap().target_icons,
        original_icons
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn request_party_member_stats_offline_replies_only_to_requester_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let target = ObjectGuid::create_player(1, 77);
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (_target_tx, target_rx) = bounded::<Vec<u8>>(4);

    session.set_player_registry(registry);

    session
        .handle_request_party_member_stats(request_party_member_stats_packet(target, Some(0)))
        .await;

    let sent = send_rx.try_recv().expect("requester full state");
    let mut target_guid_pkt = WorldPacket::new_empty();
    target_guid_pkt.write_packed_guid(&target);
    let target_guid_bytes = target_guid_pkt.into_data();
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::PartyMemberFullState as u16
    );
    assert!(!pkt.read_bit().unwrap());
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_int16().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_int16().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.read_uint16().unwrap(), 0);
    assert_eq!(pkt.read_uint16().unwrap(), 0);
    assert_eq!(pkt.read_uint16().unwrap(), 0);
    assert_eq!(pkt.read_uint16().unwrap(), 0);
    assert_eq!(pkt.read_uint16().unwrap(), 0);
    assert_eq!(pkt.read_uint16().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_int16().unwrap(), 0);
    assert_eq!(pkt.read_int16().unwrap(), 0);
    assert_eq!(pkt.read_int16().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert!(!pkt.read_bit().unwrap());
    assert!(sent.ends_with(&target_guid_bytes));
    assert!(send_rx.try_recv().is_err());
    assert!(target_rx.try_recv().is_err());
}
#[tokio::test]
async fn request_party_member_stats_routes_reply_through_realm_like_cpp() {
    let (mut session, instance_rx) = make_session_with_send();
    let (realm_tx, realm_rx) = bounded(4);
    session.install_realm_send_channel_for_test(realm_tx);
    let target = ObjectGuid::create_player(1, 77);
    session.set_player_registry(Arc::new(
        PlayerRegistry::with_canonical_player_fixtures_like_cpp(),
    ));

    session
        .handle_request_party_member_stats(request_party_member_stats_packet(target, None))
        .await;

    let packet = realm_rx.try_recv().expect("realm PartyMemberFullState");
    assert_eq!(
        u16::from_le_bytes([packet[0], packet[1]]),
        ServerOpcodes::PartyMemberFullState as u16
    );
    assert!(instance_rx.try_recv().is_err());
}
#[tokio::test]
async fn request_party_member_stats_online_replies_snapshot_without_fanout_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let target = ObjectGuid::create_player(1, 78);
    let (target_tx, target_rx) = bounded::<Vec<u8>>(4);
    let registry = Arc::new(PlayerRegistry::default());
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let mut registration = broadcast_info(target, target_tx);
    registration.identity.class = 4;
    registry.register_or_replace(target, registration, Default::default());
    registry.fixture_update(target, |placement| {
        placement.position = Position::new(11.0, 22.0, 33.0, 0.0);
    });
    registry.fixture_update(target, |placement| placement.level = 80);
    let position = Position::new(11.0, 22.0, 33.0, 0.0);
    let mut player = wow_entities::Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(target);
    player.unit_mut().world_mut().set_map(0, 0).unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().set_zone_and_area(618, 0);
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_max_health(123);
    player.unit_mut().set_health(77);
    player.unit_mut().replace_all_pvp_flags_like_cpp(
        wow_constants::UnitPvpFlags::PVP | wow_constants::UnitPvpFlags::FFA_PVP,
    );
    player.set_player_flag(crate::session::PLAYER_FLAGS_GHOST_LIKE_CPP);
    player.set_player_flag(crate::session::PLAYER_FLAGS_AFK_LIKE_CPP);
    player.set_player_flag(crate::session::PLAYER_FLAGS_DND_LIKE_CPP);
    player.set_primary_specialization(260);
    let pet_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 42_000, 100);
    player.gameplay_state_mut().vehicle_seat_flags = Some(0);
    player.gameplay_state_mut().vehicle_seat_id = Some(1001);
    player.gameplay_state_mut().pet_guid = Some(pet_guid);
    {
        let phase = player.unit_mut().world_mut().phase_shift_mut();
        phase.add_phase_like_cpp(20, wow_constants::PhaseFlags::PERSONAL, 1);
        phase.set_flags_like_cpp(wow_constants::PhaseShiftFlags::UNPHASED);
    }
    let aura = wow_entities::AppliedAuraRef::new(12_345, target, 3, 0x04);
    player.unit_mut().subsystems_mut().auras.add_applied(aura);
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_visible_with_application_like_cpp(
            3,
            aura.aura_ref(),
            wow_entities::VisibleAuraApplicationLikeCpp::new(
                0x29,
                vec![wow_entities::VisibleAuraEffectAmountLikeCpp {
                    effect_index: 2,
                    amount: 17,
                }],
            ),
        );
    player
        .unit_mut()
        .set_display_power(wow_constants::PowerType::Energy);
    player.set_power_index(wow_constants::PowerType::Energy, Some(0));
    player
        .unit_mut()
        .set_max_power(wow_constants::PowerType::Energy, 100);
    player
        .unit_mut()
        .set_power(wow_constants::PowerType::Energy, 42);
    assert!(player.set_party_type_like_cpp(0, 1));
    let mut pet = wow_entities::Pet::new(target, wow_entities::PetType::Summon);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid);
    pet.creature_mut().unit_mut().world_mut().set_name("Wolf");
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .unwrap();
    pet.creature_mut().unit_mut().world_mut().relocate(position);
    pet.creature_mut().unit_mut().set_display_id(987, true);
    pet.creature_mut().unit_mut().set_max_health(66);
    pet.creature_mut().unit_mut().set_health(55);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    let mut manager = canonical.lock().unwrap();
    let map = manager.create_world_map(0, 0).map_mut();
    map.insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
    drop(manager);
    session.set_canonical_map_manager(canonical);
    session.set_player_registry(registry);

    session
        .handle_request_party_member_stats(request_party_member_stats_packet(target, None))
        .await;

    let sent = send_rx.try_recv().expect("requester full state");
    let mut target_guid_pkt = WorldPacket::new_empty();
    target_guid_pkt.write_packed_guid(&target);
    let target_guid_bytes = target_guid_pkt.into_data();
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::PartyMemberFullState as u16
    );
    assert!(!pkt.read_bit().unwrap());
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(
        pkt.read_int16().unwrap(),
        0x0001 | 0x0002 | 0x0010 | 0x0040 | 0x0080 | 0x0200
    );
    assert_eq!(pkt.read_uint8().unwrap(), 3);
    assert_eq!(pkt.read_int16().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 77);
    assert_eq!(pkt.read_int32().unwrap(), 123);
    assert_eq!(pkt.read_uint16().unwrap(), 42);
    assert_eq!(pkt.read_uint16().unwrap(), 100);
    assert_eq!(pkt.read_uint16().unwrap(), 80);
    assert_eq!(pkt.read_uint16().unwrap(), 260);
    assert_eq!(pkt.read_uint16().unwrap(), 618);
    assert_eq!(pkt.read_uint16().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_int16().unwrap(), 11);
    assert_eq!(pkt.read_int16().unwrap(), 22);
    assert_eq!(pkt.read_int16().unwrap(), 33);
    assert_eq!(pkt.read_int32().unwrap(), 1001);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), 0x08);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(pkt.read_uint32().unwrap(), 0x02);
    assert_eq!(pkt.read_uint16().unwrap(), 20);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 12_345);
    assert_eq!(pkt.read_uint16().unwrap(), 0x29);
    assert_eq!(pkt.read_uint32().unwrap(), 0x04);
    assert_eq!(pkt.read_int32().unwrap(), 1);
    assert_eq!(pkt.read_float().unwrap(), 17.0);
    assert!(pkt.read_bit().unwrap());
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_float().unwrap(), 0.0);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    let pet_guid = pkt.read_packed_guid().unwrap();
    assert_eq!(pet_guid.high_type(), wow_core::guid::HighGuid::Pet);
    assert_eq!(pkt.read_int32().unwrap(), 987);
    assert_eq!(pkt.read_int32().unwrap(), 55);
    assert_eq!(pkt.read_int32().unwrap(), 66);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    let pet_name_len = pkt.read_bits(8).unwrap() as usize;
    assert_eq!(pkt.read_string(pet_name_len).unwrap(), "Wolf");
    assert_eq!(pkt.read_packed_guid().unwrap(), target);
    assert!(sent.ends_with(&target_guid_bytes));
    assert!(
        sent.windows([0x08, 0x00, 0x00, 0x00].len())
            .any(|window| window == [0x08, 0x00, 0x00, 0x00])
    );
    assert!(send_rx.try_recv().is_err());
    assert!(target_rx.try_recv().is_err());
}
#[tokio::test]
async fn silence_party_talker_leader_records_request_before_cpp_todo_boundary() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(target);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_silence_party_talker(silence_party_talker_packet(target, true))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.represented_silence_party_talker_like_cpp().len(), 1);
    assert_eq!(
        session.represented_silence_party_talker_like_cpp()[0].target,
        target
    );
    assert!(session.represented_silence_party_talker_like_cpp()[0].silent);
}

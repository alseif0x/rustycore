//! Original construction, registry and canonical fixtures; no implicit adoption.

use super::*;

pub(super) fn make_session() -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (pkt_tx, pkt_rx) = flume::bounded(100);
    let (send_tx, send_rx) = flume::unbounded();

    let mut session = WorldSession::new_character_lifecycle_fixture(
        1,
        "TestAccount".into(),
        0,
        2,
        9, // account_expansion (raw from DB)
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.character_set_active_player_local_flags_for_test(
        PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
    );

    (session, pkt_tx, send_rx)
}

pub(super) fn shared_canonical_map_manager() -> SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

pub(super) fn session_with_canonical_player_for_away_like_cpp()
-> (WorldSession, SharedCanonicalMapManager, ObjectGuid) {
    let (session, _, canonical, player_guid) =
        session_with_canonical_player_for_away_like_cpp_with_packet_tx();
    (session, canonical, player_guid)
}

pub(super) fn session_with_canonical_player_for_away_like_cpp_with_packet_tx() -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    SharedCanonicalMapManager,
    ObjectGuid,
) {
    let (mut session, pkt_tx, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xAFD0);
    session.character_ensure_login_player_controller_for_test(
        player_guid,
        "AwayTester".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    (session, pkt_tx, canonical, player_guid)
}

pub(super) fn add_canonical_test_player_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    add_canonical_test_player_on_map_with_difficulty(
        canonical,
        guid,
        position,
        map_id,
        instance_id,
        0,
    );
}

pub(super) fn add_canonical_test_player_on_map_with_difficulty(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
    difficulty_id: u8,
) {
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_name("InstanceOwner");
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();

    canonical
        .lock()
        .unwrap()
        .create_map_entry(
            map_id,
            instance_id,
            difficulty_id,
            wow_map::ManagedMapKind::World,
        )
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

pub(super) fn canonical_party_type_for_test(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> [u8; 2] {
    with_canonical_player_at_like_cpp(canonical, guid, 571, 0, |player| player.data().party_type)
        .expect("canonical player")
}

pub(super) fn broadcast_info(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> PlayerSessionRegistrationLikeCpp {
    let (command_tx, _command_rx) = flume::bounded(1);
    broadcast_info_with_command(guid, send_tx, command_tx)
}

pub(super) fn broadcast_info_with_command(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
) -> PlayerSessionRegistrationLikeCpp {
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp::new(
            format!("Player{}", guid.counter()),
            guid.counter() as u32,
            0,
            1,
            1,
            0,
            2,
        ),
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: Position::ZERO,
            is_in_world: true,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: wow_world::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        client_visible_transports_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

pub(super) fn canonical_player_transfer_test_map_store_like_cpp() -> Arc<wow_data::MapStore> {
    Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]))
}

pub(super) fn with_canonical_player_at_like_cpp<R>(
    manager: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    read: impl FnOnce(&Player) -> R,
) -> Option<R> {
    let manager = manager.lock().ok()?;
    let map = manager.find_map(map_id, instance_id)?;
    let player = map.map().get_typed_player(guid)?;
    Some(read(player))
}

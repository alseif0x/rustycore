// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! instance capability handler tests.

use super::*;
use wow_constants::UnitStandStateType;
use wow_packet::packets::misc::SetSavedInstanceExtend;
use wow_persistence::{
    InstanceLockPersistenceLoadOutcomeLikeCpp, InstanceLockPersistenceMutationLikeCpp,
    InstanceLockPersistenceOutcomeLikeCpp, InstanceLockPersistencePlanLikeCpp,
    InstanceLockPersistencePortLikeCpp, PersistenceFutureLikeCpp,
    RepresentedGroupPersistenceOutcomeLikeCpp, RepresentedGroupPersistencePortLikeCpp,
    RepresentedGroupPersistenceRequestLikeCpp,
};

struct InstanceLockPersistenceFixtureLikeCpp {
    outcome: InstanceLockPersistenceOutcomeLikeCpp,
    plans: Mutex<Vec<InstanceLockPersistencePlanLikeCpp>>,
}

impl InstanceLockPersistenceFixtureLikeCpp {
    fn new(outcome: InstanceLockPersistenceOutcomeLikeCpp) -> Self {
        Self {
            outcome,
            plans: Mutex::new(Vec::new()),
        }
    }
}

impl InstanceLockPersistencePortLikeCpp for InstanceLockPersistenceFixtureLikeCpp {
    fn load_all_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, InstanceLockPersistenceLoadOutcomeLikeCpp> {
        Box::pin(async {
            InstanceLockPersistenceLoadOutcomeLikeCpp::Failed {
                reason: "not used by handler fixture".to_string(),
            }
        })
    }

    fn commit_plan_like_cpp<'a>(
        &'a self,
        plan: InstanceLockPersistencePlanLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, InstanceLockPersistenceOutcomeLikeCpp> {
        self.plans.lock().unwrap().push(plan);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }
}

struct LiveResetFixtureLikeCpp {
    canonical: crate::session::SharedCanonicalMapManager,
    locks: Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>,
    entries: wow_instances::MapDb2Entries,
    now: u64,
    expiry_time: u64,
}

fn install_live_reset_fixture_like_cpp(
    session: &mut crate::session::WorldSession,
    player_guid: ObjectGuid,
    is_new: bool,
) -> LiveResetFixtureLikeCpp {
    let entries = wow_instances::MapDb2Entries {
        map_id: 631,
        difficulty_id: 4,
        lock_id: 10,
        reset_interval: wow_instances::MapDifficultyResetInterval::Weekly,
        max_players: 10,
        is_flex_locking: true,
        is_using_encounter_locks: false,
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut mgr = wow_instances::InstanceLockMgr::default();
    let lock = if is_new {
        mgr.create_instance_lock_for_new_instance_at(
            player_guid,
            &entries,
            100,
            wow_instances::ResetSchedule::default(),
            now,
        )
    } else {
        mgr.update_instance_lock_for_player_at(
            player_guid,
            &entries,
            wow_instances::InstanceLockUpdateEvent {
                instance_id: 100,
                new_data: String::new(),
                instance_completed_encounters_mask: 0,
                completed_encounter_bit: None,
                entrance_world_safe_loc_id: None,
            },
            wow_instances::ResetSchedule::default(),
            now,
        )
    }
    .unwrap();
    let expiry_time = lock.expiry_time;
    let token =
        wow_world_instances::create_map_instance_lock_token_like_cpp(player_guid, &entries, lock);

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(0, Position::ZERO);
    session.set_map_store(Arc::new(MapStore::from_entries([
        MapEntry {
            id: 0,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 631,
            instance_type: 2,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
            flags2: 0,
        },
    ])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id: 631,
            difficulty_id: 4,
            lock_id: 10,
            reset_interval: 2,
            max_players: 0,
            flags: 0,
        },
    ])));
    let locks = Arc::new(std::sync::RwLock::new(mgr));
    session.set_instance_lock_mgr(Arc::clone(&locks));

    let canonical = shared_canonical_map_manager_for_misc_test();
    canonical
        .lock()
        .unwrap()
        .create_map_entry(
            631,
            100,
            4,
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
        )
        .set_instance_lock_context(Some(wow_map::CreateMapInstanceLockContext {
            instance_id: 100,
            difficulty_id: 4,
            token,
            owner_guid_counter: player_guid.counter() as u64,
        }));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_represented_player_recent_instance_like_cpp(631, 100);

    LiveResetFixtureLikeCpp {
        canonical,
        locks,
        entries,
        now,
        expiry_time,
    }
}

struct GroupPersistenceFixtureLikeCpp {
    outcome: RepresentedGroupPersistenceOutcomeLikeCpp,
    requests: Mutex<Vec<RepresentedGroupPersistenceRequestLikeCpp>>,
}

impl GroupPersistenceFixtureLikeCpp {
    fn new(outcome: RepresentedGroupPersistenceOutcomeLikeCpp) -> Arc<Self> {
        Arc::new(Self {
            outcome,
            requests: Mutex::new(Vec::new()),
        })
    }
}

impl RepresentedGroupPersistencePortLikeCpp for GroupPersistenceFixtureLikeCpp {
    fn persist_group_commands_like_cpp(
        &self,
        request: RepresentedGroupPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, RepresentedGroupPersistenceOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }
}

fn install_reset_difficulty_store_like_cpp(session: &mut crate::session::WorldSession) {
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
}

fn install_reset_group_like_cpp(
    session: &mut crate::session::WorldSession,
    player_guid: ObjectGuid,
) -> (Arc<GroupRegistry>, u64) {
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.link_owned_instance_like_cpp(631, 100);
    group.set_recent_instance_like_cpp(631, player_guid, 100);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session
        .social
        .set_group_guid_for_test_like_cpp(Some(group_guid));
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    (group_registry, group_guid)
}

fn add_reset_occupant_like_cpp(fixture: &LiveResetFixtureLikeCpp, occupant_guid: ObjectGuid) {
    let mut occupant = wow_entities::Player::new(Some(1), false);
    occupant
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(occupant_guid);
    occupant.unit_mut().world_mut().set_map(631, 100).unwrap();
    fixture
        .canonical
        .lock()
        .unwrap()
        .find_map_mut(631, 100)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_player(occupant).unwrap(),
        )
        .unwrap();
}

fn register_reset_occupant_like_cpp(
    session: &mut crate::session::WorldSession,
    occupant_guid: ObjectGuid,
) -> flume::Receiver<Vec<u8>> {
    let player_registry = Arc::new(PlayerRegistry::new());
    let (occupant_tx, occupant_rx) = flume::bounded::<Vec<u8>>(2);
    let (command_tx, _command_rx) = flume::bounded(2);
    player_registry.register_or_replace(
        occupant_guid,
        PlayerSessionRegistrationLikeCpp {
            identity: PlayerDirectoryIdentityLikeCpp::new_with_bnet(
                "Occupant", 1, 0, 0, 1, 1, 0, 2, "enUS", None,
            ),
            placement: PlayerDirectoryPlacementLikeCpp {
                map_id: 631,
                instance_id: 100,
                position: Position::ZERO,
                is_in_world: true,
                level: 80,
                is_alive: true,
            },
            active_loot_rolls: Vec::new(),
            realm_send_tx: occupant_tx.clone(),
            send_tx: occupant_tx,
            command_tx,
            session_phase_tx: crate::session::directory::detached_session_phase_rail_like_cpp(),
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            client_visible_transports_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        },
        Default::default(),
    );
    session.set_player_registry(player_registry);
    occupant_rx
}

fn assert_reset_lock_expiry_unchanged_like_cpp(
    fixture: &LiveResetFixtureLikeCpp,
    player_guid: ObjectGuid,
    expected_expiry: u64,
) {
    let locks = fixture.locks.read().unwrap();
    let lock = locks
        .find_instance_lock_exact_like_cpp(player_guid, 631, 4, 100)
        .expect("reset must retain the exact attached lock");
    assert_eq!(lock.expiry_time, expected_expiry);
}

#[tokio::test]
async fn set_difficulty_id_resets_live_map_without_expiring_lock_or_persisting_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Committed,
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    let reset = send_rx.try_recv().expect("instance reset packet");
    assert_eq!(
        WorldPacket::from_bytes(&reset).server_opcode(),
        Some(ServerOpcodes::InstanceReset)
    );
    assert_eq!(&reset[2..], &[0x77, 0x02, 0x00, 0x00]);
    let difficulty = send_rx.try_recv().expect("dungeon difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&difficulty).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    let locks = fixture.locks.read().unwrap();
    let lock = locks
        .find_active_instance_lock_at(player_guid, &fixture.entries, fixture.now)
        .expect("difficulty reset must not expire the active lock");
    assert_eq!(lock.expiry_time, fixture.expiry_time);
    drop(locks);
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        0
    );
    assert!(port.plans.lock().unwrap().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_difficulty_id_inside_instanceable_map_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_INSTANCE,
    )])));

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 1);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn stand_state_update_uses_cpp_realm_connection_and_values_use_instance() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 9013);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    add_canonical_test_player_on_map_for_misc_test(&canonical, player_guid, position, 571, 0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(canonical);

    session
        .handle_stand_state_change(stand_state_change_packet(UnitStandStateType::Sit as u32))
        .await;

    assert_eq!(
        WorldPacket::from_bytes(&realm_rx.try_recv().expect("realm stand packet")).server_opcode(),
        Some(ServerOpcodes::StandStateUpdate)
    );
    assert_eq!(
        WorldPacket::from_bytes(&instance_rx.try_recv().expect("instance VALUES packet"))
            .server_opcode(),
        Some(ServerOpcodes::UpdateObject)
    );
    assert!(realm_rx.try_recv().is_err());
    assert!(instance_rx.try_recv().is_err());
}

#[tokio::test]
async fn reset_instances_handler_resets_live_map_without_expiring_lock_or_persisting_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Committed,
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::InstanceReset as u16
    );
    assert_eq!(&sent[2..], &[0x77, 0x02, 0x00, 0x00]);
    let locks = fixture.locks.read().unwrap();
    let lock = locks
        .find_active_instance_lock_at(player_guid, &fixture.entries, fixture.now)
        .expect("manual reset must not expire the active lock");
    assert_eq!(lock.expiry_time, fixture.expiry_time);
    drop(locks);
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        0
    );
    assert!(port.plans.lock().unwrap().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn reset_instances_non_new_lock_retains_recent_without_packets_or_persistence_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, false);
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Committed,
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        100
    );
    let locks = fixture.locks.read().unwrap();
    let lock = locks
        .find_active_instance_lock_at(player_guid, &fixture.entries, fixture.now)
        .expect("CannotReset must not expire the active lock");
    assert_eq!(lock.expiry_time, fixture.expiry_time);
    drop(locks);
    assert!(port.plans.lock().unwrap().is_empty());
}

#[tokio::test]
async fn reset_instances_not_empty_notifies_current_occupant_and_retains_recent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let occupant_guid = ObjectGuid::create_player(1, 43);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    add_reset_occupant_like_cpp(&fixture, occupant_guid);
    let occupant_rx = register_reset_occupant_like_cpp(&mut session, occupant_guid);

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    let notify = occupant_rx.try_recv().expect("occupant reset notification");
    assert_eq!(
        WorldPacket::from_bytes(&notify).server_opcode(),
        Some(ServerOpcodes::ResetFailedNotify)
    );
    assert_eq!(notify.len(), 2);
    let failed = send_rx.try_recv().expect("requester reset failure");
    assert_eq!(
        WorldPacket::from_bytes(&failed).server_opcode(),
        Some(ServerOpcodes::InstanceResetFailed)
    );
    assert_eq!(&failed[2..], &[0x77, 0x02, 0x00, 0x00, 0x00]);
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        100
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn reset_instances_group_uses_owned_map_and_forgets_group_recent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let _fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    let (group_registry, group_guid) = install_reset_group_like_cpp(&mut session, player_guid);

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    let sent = send_rx.try_recv().expect("group instance reset packet");
    assert_eq!(
        WorldPacket::from_bytes(&sent).server_opcode(),
        Some(ServerOpcodes::InstanceReset)
    );
    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .recent_instance_id_like_cpp(631),
        0
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_difficulty_id_solo_occupied_forgets_recent_without_reset_failures_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let occupant_guid = ObjectGuid::create_player(1, 43);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    add_reset_occupant_like_cpp(&fixture, occupant_guid);
    let occupant_rx = register_reset_occupant_like_cpp(&mut session, occupant_guid);
    install_reset_difficulty_store_like_cpp(&mut session);

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    let difficulty = send_rx.try_recv().expect("difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&difficulty).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    assert!(send_rx.try_recv().is_err());
    assert!(occupant_rx.try_recv().is_err());
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        0
    );
    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_reset_lock_expiry_unchanged_like_cpp(&fixture, player_guid, fixture.expiry_time);
}

#[tokio::test]
async fn set_difficulty_id_group_occupied_forgets_recent_without_reset_failures_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let occupant_guid = ObjectGuid::create_player(1, 43);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    let (group_registry, group_guid) = install_reset_group_like_cpp(&mut session, player_guid);
    add_reset_occupant_like_cpp(&fixture, occupant_guid);
    let occupant_rx = register_reset_occupant_like_cpp(&mut session, occupant_guid);
    install_reset_difficulty_store_like_cpp(&mut session);
    let port =
        GroupPersistenceFixtureLikeCpp::new(RepresentedGroupPersistenceOutcomeLikeCpp::Applied {
            command_count: 1,
        });
    session.set_represented_group_persistence_port_like_cpp(port.clone());

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    let difficulty = send_rx.try_recv().expect("difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&difficulty).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    assert!(send_rx.try_recv().is_err());
    assert!(occupant_rx.try_recv().is_err());
    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(group.recent_instance_id_like_cpp(631), 0);
    assert_eq!(group.dungeon_difficulty_id, 2);
    drop(group);
    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(port.requests.lock().unwrap().len(), 1);
    assert_reset_lock_expiry_unchanged_like_cpp(&fixture, player_guid, fixture.expiry_time);
}

#[tokio::test]
async fn reset_instances_group_occupied_manual_retains_recent_and_sends_failures_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let occupant_guid = ObjectGuid::create_player(1, 43);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    let (group_registry, group_guid) = install_reset_group_like_cpp(&mut session, player_guid);
    add_reset_occupant_like_cpp(&fixture, occupant_guid);
    let occupant_rx = register_reset_occupant_like_cpp(&mut session, occupant_guid);

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    let notify = occupant_rx.try_recv().expect("occupant notification");
    assert_eq!(
        WorldPacket::from_bytes(&notify).server_opcode(),
        Some(ServerOpcodes::ResetFailedNotify)
    );
    assert_eq!(notify.len(), 2);
    let failed = send_rx.try_recv().expect("requester failure");
    assert_eq!(
        WorldPacket::from_bytes(&failed).server_opcode(),
        Some(ServerOpcodes::InstanceResetFailed)
    );
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .recent_instance_id_like_cpp(631),
        100
    );
}

#[tokio::test]
async fn reset_instances_group_non_new_manual_forgets_recent_without_packets_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, false);
    let (group_registry, group_guid) = install_reset_group_like_cpp(&mut session, player_guid);

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .recent_instance_id_like_cpp(631),
        0
    );
    assert!(
        fixture
            .canonical
            .lock()
            .unwrap()
            .find_map(631, 100)
            .is_some()
    );
    assert_reset_lock_expiry_unchanged_like_cpp(&fixture, player_guid, fixture.expiry_time);
}

#[tokio::test]
async fn set_difficulty_id_group_non_new_forgets_recent_and_applies_difficulty_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, false);
    let (group_registry, group_guid) = install_reset_group_like_cpp(&mut session, player_guid);
    install_reset_difficulty_store_like_cpp(&mut session);
    let port =
        GroupPersistenceFixtureLikeCpp::new(RepresentedGroupPersistenceOutcomeLikeCpp::Applied {
            command_count: 1,
        });
    session.set_represented_group_persistence_port_like_cpp(port.clone());

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    let difficulty = send_rx.try_recv().expect("difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&difficulty).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    assert!(send_rx.try_recv().is_err());
    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(group.recent_instance_id_like_cpp(631), 0);
    assert_eq!(group.dungeon_difficulty_id, 2);
    drop(group);
    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(port.requests.lock().unwrap().len(), 1);
    assert_reset_lock_expiry_unchanged_like_cpp(&fixture, player_guid, fixture.expiry_time);
}

#[tokio::test]
async fn set_difficulty_id_missing_and_non_instance_maps_preserve_solo_recent_like_cpp() {
    for non_instance in [false, true] {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
        {
            let mut manager = fixture.canonical.lock().unwrap();
            assert!(manager.destroy_map(631, 100));
            if non_instance {
                manager.create_world_map(631, 100);
            }
        }
        install_reset_difficulty_store_like_cpp(&mut session);

        session
            .handle_set_difficulty_id(set_difficulty_request(2))
            .await;

        let difficulty = send_rx.try_recv().expect("difficulty packet");
        assert_eq!(
            WorldPacket::from_bytes(&difficulty).server_opcode(),
            Some(ServerOpcodes::SetDungeonDifficulty)
        );
        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            session.represented_player_recent_instance_id_like_cpp(631),
            100
        );
        assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
        assert_reset_lock_expiry_unchanged_like_cpp(&fixture, player_guid, fixture.expiry_time);
    }
}

#[tokio::test]
async fn set_difficulty_id_missing_and_non_instance_maps_preserve_group_recent_like_cpp() {
    for non_instance in [false, true] {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
        {
            let mut manager = fixture.canonical.lock().unwrap();
            assert!(manager.destroy_map(631, 100));
            if non_instance {
                manager.create_world_map(631, 100);
            }
        }
        let (group_registry, group_guid) = install_reset_group_like_cpp(&mut session, player_guid);
        install_reset_difficulty_store_like_cpp(&mut session);
        let port = GroupPersistenceFixtureLikeCpp::new(
            RepresentedGroupPersistenceOutcomeLikeCpp::Applied { command_count: 1 },
        );
        session.set_represented_group_persistence_port_like_cpp(port.clone());

        session
            .handle_set_difficulty_id(set_difficulty_request(2))
            .await;

        let difficulty = send_rx.try_recv().expect("difficulty packet");
        assert_eq!(
            WorldPacket::from_bytes(&difficulty).server_opcode(),
            Some(ServerOpcodes::SetDungeonDifficulty)
        );
        assert!(send_rx.try_recv().is_err());
        let group = group_registry.get(&group_guid).unwrap();
        assert_eq!(group.recent_instance_id_like_cpp(631), 100);
        assert_eq!(group.dungeon_difficulty_id, 2);
        drop(group);
        assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
        assert_eq!(port.requests.lock().unwrap().len(), 1);
        assert_reset_lock_expiry_unchanged_like_cpp(&fixture, player_guid, fixture.expiry_time);
    }
}

#[tokio::test]
async fn reset_instances_group_non_leader_is_silent_and_retains_recent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let leader_guid = ObjectGuid::create_player(1, 43);
    let _fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader_guid);
    group.add_member(player_guid);
    group.link_owned_instance_like_cpp(631, 100);
    group.set_recent_instance_like_cpp(631, leader_guid, 100);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session
        .social
        .set_group_guid_for_test_like_cpp(Some(group_guid));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .recent_instance_id_like_cpp(631),
        100
    );
}

#[tokio::test]
async fn reset_instances_inside_instance_is_silent_and_retains_recent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let _fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    session.set_player_map_position_like_cpp(631, Position::ZERO);

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        100
    );
}

#[tokio::test]
async fn set_difficulty_id_group_persistence_failure_does_not_roll_back_reset_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, true);
    let (group_registry, group_guid) = install_reset_group_like_cpp(&mut session, player_guid);
    install_reset_difficulty_store_like_cpp(&mut session);
    let port = GroupPersistenceFixtureLikeCpp::new(
        RepresentedGroupPersistenceOutcomeLikeCpp::FailedAfterPrefix {
            applied: 0,
            reason: "fixture difficulty persistence failure".to_string(),
        },
    );
    session.set_represented_group_persistence_port_like_cpp(port.clone());

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    let reset = send_rx.try_recv().expect("map reset precedes persistence");
    assert_eq!(
        WorldPacket::from_bytes(&reset).server_opcode(),
        Some(ServerOpcodes::InstanceReset)
    );
    let difficulty = send_rx
        .try_recv()
        .expect("difficulty publication is retained");
    assert_eq!(
        WorldPacket::from_bytes(&difficulty).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    assert!(send_rx.try_recv().is_err());
    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(group.recent_instance_id_like_cpp(631), 0);
    assert_eq!(group.dungeon_difficulty_id, 2);
    drop(group);
    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 2);
    assert_eq!(port.requests.lock().unwrap().len(), 1);
    assert_reset_lock_expiry_unchanged_like_cpp(&fixture, player_guid, fixture.expiry_time);
}

#[tokio::test]
async fn reset_instances_expired_attached_non_new_lock_is_cannot_reset_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_global(wow_core::guid::HighGuid::Player, 0, 42);
    let mut fixture = install_live_reset_fixture_like_cpp(&mut session, player_guid, false);
    let mut expired = wow_instances::InstanceLockMgr::default();
    let entries = fixture.entries.clone();
    let issues = expired.load_from_rows_like_cpp(
        [],
        [wow_instances::CharacterInstanceLockRow {
            player_guid_counter: player_guid.counter() as u64,
            map_id: 631,
            lock_id: 10,
            instance_id: 100,
            difficulty_id: 4,
            data: String::new(),
            completed_encounters_mask: 0,
            entrance_world_safe_loc_id: 0,
            expiry_time: 10,
            extended: false,
        }],
        |_, _| Some(entries),
    );
    assert!(issues.is_empty());
    fixture.locks = Arc::new(std::sync::RwLock::new(expired));
    fixture.expiry_time = 10;
    session.set_instance_lock_mgr(Arc::clone(&fixture.locks));
    let attached = fixture
        .locks
        .read()
        .unwrap()
        .find_instance_lock_exact_like_cpp(player_guid, 631, 4, 100)
        .expect("expired fixture must retain its exact attached lock")
        .clone();
    assert!(!attached.is_new);
    assert!(
        fixture
            .locks
            .read()
            .unwrap()
            .find_active_instance_lock_at(player_guid, &fixture.entries, fixture.now)
            .is_none(),
        "the expired attached lock is not generally active for admission"
    );

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        100
    );
    assert_reset_lock_expiry_unchanged_like_cpp(&fixture, player_guid, 10);
}

#[tokio::test]
async fn set_saved_instance_extend_updates_lock_and_sends_calendar_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let entries = wow_instances::MapDb2Entries {
        map_id: 631,
        difficulty_id: 4,
        lock_id: 10,
        reset_interval: wow_instances::MapDifficultyResetInterval::Weekly,
        max_players: 10,
        is_flex_locking: true,
        is_using_encounter_locks: false,
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut mgr = wow_instances::InstanceLockMgr::default();
    mgr.update_instance_lock_for_player_at(
        player_guid,
        &entries,
        wow_instances::InstanceLockUpdateEvent {
            instance_id: 100,
            new_data: String::new(),
            instance_completed_encounters_mask: 0,
            completed_encounter_bit: None,
            entrance_world_safe_loc_id: None,
        },
        wow_instances::ResetSchedule::default(),
        now,
    );

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(0, Position::ZERO);
    session.set_map_store(Arc::new(MapStore::from_entries([
        MapEntry {
            id: 0,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 631,
            instance_type: 2,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
            flags2: 0,
        },
    ])));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        4,
        2,
        DifficultyFlags::empty(),
    )])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id: 631,
            difficulty_id: 4,
            lock_id: 10,
            reset_interval: 2,
            max_players: 0,
            flags: 0,
        },
    ])));
    let mgr = Arc::new(std::sync::RwLock::new(mgr));
    session.set_instance_lock_mgr(Arc::clone(&mgr));
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Committed,
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());

    session
        .handle_set_saved_instance_extend(SetSavedInstanceExtend {
            map_id: 631,
            difficulty_id: 4,
            extend: true,
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::CalendarRaidLockoutUpdated as u16
    );
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    let _server_time = pkt.read_uint32().unwrap();
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    let old_remaining = pkt.read_int32().unwrap();
    let new_remaining = pkt.read_int32().unwrap();
    assert!(new_remaining > old_remaining);
    assert!(
        mgr.read()
            .unwrap()
            .find_active_instance_lock_at(player_guid, &entries, now)
            .unwrap()
            .extended
    );
    assert!(matches!(
        port.plans.lock().unwrap()[0].mutations.as_slice(),
        [InstanceLockPersistenceMutationLikeCpp::UpdateCharacterLockExtension { .. }]
    ));
}

#[tokio::test]
async fn set_saved_instance_extend_current_map_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_map_position_like_cpp(631, Position::ZERO);

    session
        .handle_set_saved_instance_extend(SetSavedInstanceExtend {
            map_id: 631,
            difficulty_id: 4,
            extend: true,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[test]
fn send_pending_raid_lock_sets_pending_bind_like_cpp_for_stop_prompt() {
    let (mut session, send_rx) = make_session();

    session.send_pending_raid_lock_like_cpp(77, 0xA5, true, false);

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::PendingRaidLock as u16
    );
    assert_eq!(
        session
            .instances
            .represented_pending_bind_for_test_like_cpp(),
        Some(crate::session::RepresentedPendingBind {
            map_id: 0,
            instance_id: 77,
            completed_mask: 0xA5,
            time_until_lock_ms: 60_000,
        })
    );
}

#[test]
fn send_pending_raid_lock_warning_only_does_not_set_pending_bind_like_cpp() {
    let (mut session, _send_rx) = make_session();

    session.send_pending_raid_lock_like_cpp(77, 0xA5, false, true);

    assert!(
        session
            .instances
            .represented_pending_bind_for_test_like_cpp()
            .is_none()
    );
}

#[tokio::test]
async fn instance_lock_response_accept_confirms_and_clears_pending_bind_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let mgr =
        install_pending_bind_instance_context_like_cpp(&mut session, player_guid, 631, 9001, 4, 10);
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Committed,
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());
    session
        .instances
        .set_represented_pending_bind_for_test_like_cpp(Some(
            crate::session::RepresentedPendingBind {
                map_id: 631,
                instance_id: 9001,
                completed_mask: 0xA5,
                time_until_lock_ms: 60_000,
            },
        ));

    session
        .handle_instance_lock_response(WorldPacket::from_bytes(&[0x80]))
        .await;

    assert!(
        session
            .instances
            .represented_pending_bind_for_test_like_cpp()
            .is_none()
    );
    assert_eq!(
        session
            .instances
            .represented_confirmed_pending_binds_for_test_like_cpp(),
        vec![9001]
    );
    assert_eq!(
        session.fixtures.combat.represented_repop_at_graveyard_count,
        0
    );

    let entries = session.create_map_db2_entries_like_cpp(631, 4).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let lock = mgr
        .read()
        .unwrap()
        .find_active_instance_lock_at(player_guid, &entries, now)
        .cloned()
        .expect("accepting a matching pending bind creates the player instance lock");
    assert_eq!(lock.instance_id, 9001);
    assert_eq!(lock.data.completed_encounters_mask, 0xA5);
    assert!(!lock.is_new);

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::InstanceSaveCreated as u16
    );
    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::CalendarRaidLockoutAdded as u16
    );
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    assert_eq!(pkt.read_uint64().unwrap(), 9001);
    let _server_time = pkt.read_uint32().unwrap();
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert!(pkt.read_int32().unwrap() > 0);
    assert!(matches!(
        port.plans.lock().unwrap()[0].mutations.as_slice(),
        [
            InstanceLockPersistenceMutationLikeCpp::DeleteCharacterLock { .. },
            InstanceLockPersistenceMutationLikeCpp::InsertCharacterLock { .. }
        ]
    ));
}

#[tokio::test]
async fn pending_bind_commit_failure_keeps_mutation_but_publishes_nothing_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let mgr =
        install_pending_bind_instance_context_like_cpp(&mut session, player_guid, 631, 9001, 4, 10);
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Failed {
            reason: "fixture commit failure".to_string(),
        },
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());
    session
        .instances
        .set_represented_pending_bind_for_test_like_cpp(Some(
            crate::session::RepresentedPendingBind {
                map_id: 631,
                instance_id: 9001,
                completed_mask: 0xA5,
                time_until_lock_ms: 60_000,
            },
        ));

    session
        .handle_instance_lock_response(WorldPacket::from_bytes(&[0x80]))
        .await;

    assert!(
        session
            .instances
            .represented_pending_bind_for_test_like_cpp()
            .is_none()
    );
    assert!(
        session
            .instances
            .represented_confirmed_pending_binds_for_test_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
    assert_eq!(port.plans.lock().unwrap().len(), 1);

    let entries = session.create_map_db2_entries_like_cpp(631, 4).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        mgr.read()
            .unwrap()
            .find_active_instance_lock_at(player_guid, &entries, now)
            .is_some(),
        "the represented path already mutated memory before the failed commit, matching the pre-port ordering"
    );
}

#[tokio::test]
async fn instance_lock_response_accept_mismatched_instance_only_clears_pending_bind_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let mgr =
        install_pending_bind_instance_context_like_cpp(&mut session, player_guid, 631, 9001, 4, 10);
    session
        .instances
        .set_represented_pending_bind_for_test_like_cpp(Some(
            crate::session::RepresentedPendingBind {
                map_id: 631,
                instance_id: 9002,
                completed_mask: 0xA5,
                time_until_lock_ms: 60_000,
            },
        ));

    session
        .handle_instance_lock_response(WorldPacket::from_bytes(&[0x80]))
        .await;

    assert!(
        session
            .instances
            .represented_pending_bind_for_test_like_cpp()
            .is_none()
    );
    assert!(
        session
            .instances
            .represented_confirmed_pending_binds_for_test_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());

    let entries = session.create_map_db2_entries_like_cpp(631, 4).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        mgr.read()
            .unwrap()
            .find_active_instance_lock_at(player_guid, &entries, now)
            .is_none(),
        "C++ ConfirmPendingBind returns before creating a lock when current InstanceMap id mismatches"
    );
}

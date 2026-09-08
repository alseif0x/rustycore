//! Production-linked ACK, canonical map selection, directory and Session pump.
//! C++: MovementHandler.cpp:808; Player.cpp:23045,23322; Map.cpp:830;
//! GridNotifiers.cpp:30,217. These fixtures prove the represented bridge, not
//! transport/shared-vision, reciprocal visibility or full packet parity.

use super::super::{
    PlayerRegistry, SessionPlayerController, SessionState, SharedCanonicalMapManager, WorldSession,
};
use crate::session::mailbox::{RefreshVisibleWorldCreaturesLikeCppCommand, SessionCommand};
use std::sync::{Arc, Mutex, RwLock};
use wow_constants::ServerOpcodes;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::ObjectNotifyFlags;
use wow_map::{
    DEFAULT_VISIBILITY_NOTIFY_PERIOD, GridStateKind, MapKey, PeriodicTimer,
    PlayerVisibilityRefreshIntentLikeCpp, compute_grid_coord,
};
use wow_packet::{WorldPacket, packets::update::UpdateType};

const KEY: MapKey = MapKey::new(571, 0);

struct Fixture {
    session: WorldSession,
    canonical: SharedCanonicalMapManager,
    registry: Arc<PlayerRegistry>,
    output: flume::Receiver<Vec<u8>>,
    _input: flume::Sender<WorldPacket>,
    creature: ObjectGuid,
    position: Position,
}

impl Fixture {
    fn new() -> Self {
        let (input, incoming) = flume::bounded(1);
        let (outgoing, output) = flume::unbounded();
        let mut session = WorldSession::new(
            1,
            "DeferredVisibility".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".into(),
            incoming,
            outgoing,
        );
        let (command_tx, command_rx) = flume::bounded(1);
        session.session_command_tx = command_tx;
        session.session_command_rx = command_rx;
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        let registry = Arc::new(PlayerRegistry::new());
        let legacy = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
        let position = Position::new(10.0, 10.0, 0.0, 0.0);
        let creature_position = Position::new(12.0, 10.0, 0.0, 0.0);
        let creature = ObjectGuid::create_world_object(
            HighGuid::Creature,
            0,
            1,
            KEY.map_id as u16,
            0,
            901,
            588_902,
        );
        let (grid_x, grid_y) =
            crate::map_manager::world_to_grid_coords(creature_position.x, creature_position.y);
        legacy.write().unwrap().add_creature(
            KEY.map_id as u16,
            KEY.instance_id,
            grid_x,
            grid_y,
            crate::map_manager::WorldCreature::new(
                creature,
                901,
                creature_position,
                100,
                80,
                1,
                2,
                0.0,
                1,
                35,
                0,
                0,
            ),
        );
        session.set_map_manager(legacy);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_player_registry(Arc::clone(&registry));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: KEY.map_id,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            ObjectGuid::create_player(1, 588_901),
            "DeferredViewer".into(),
            position,
            KEY.map_id as u16,
            1,
            1,
            80,
            0,
        ));
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .expect("canonical active viewer");
        session.state = SessionState::LoggedIn;
        session.set_active_player_local_flags_like_cpp(0);
        session.register_in_player_registry();
        assert!(
            registry
                .control_address(session.player_guid().unwrap())
                .is_some()
        );
        session.last_visibility_pos = Some(position);
        // Login setup is outside the action window tested below.
        while output.try_recv().is_ok() {}
        Self {
            session,
            canonical,
            registry,
            output,
            _input: input,
            creature,
            position,
        }
    }

    fn tick(&self) -> Vec<PlayerVisibilityRefreshIntentLikeCpp> {
        let mut manager = self.canonical.lock().unwrap();
        let grid = manager
            .find_map_mut(KEY.map_id, KEY.instance_id)
            .unwrap()
            .map_mut()
            .get_ngrid_mut(compute_grid_coord(self.position.x, self.position.y))
            .unwrap();
        grid.set_state(GridStateKind::Active);
        *grid.info_mut().relocation_timer_mut() =
            PeriodicTimer::new(DEFAULT_VISIBILITY_NOTIFY_PERIOD, 0);
        assert_eq!(manager.update(1), Some(1));
        manager.take_player_visibility_refresh_intents_like_cpp()
    }

    fn acknowledge(&mut self) -> PlayerVisibilityRefreshIntentLikeCpp {
        self.session
            .apply_move_init_active_mover_complete_like_cpp(25);
        let handle = self.session.player_handle_like_cpp.unwrap();
        assert_eq!(
            self.canonical
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |player| {
                    player
                        .unit()
                        .world()
                        .object()
                        .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
                }),
            Some(true)
        );
        // ACK may publish transport-time VALUES, but never CREATE directly.
        for bytes in self.output.try_iter() {
            if let Some((kind, _)) = first_update_block(&bytes) {
                assert_ne!(kind, UpdateType::CreateObject as u8);
                assert_ne!(kind, UpdateType::CreateObject2 as u8);
            }
        }
        let intents = self.tick();
        assert_eq!(intents.len(), 1);
        intents[0]
    }

    async fn pump(&mut self) {
        self.session
            .process_represented_session_commands_like_cpp()
            .await;
    }

    fn assert_created_once(&self) {
        assert!(
            self.session
                .client_visible_guids_like_cpp
                .contains(&self.creature)
        );
        let packets: Vec<_> = self.output.try_iter().collect();
        let creates: Vec<_> = packets
            .iter()
            .filter_map(|bytes| first_update_block(bytes))
            .filter(|(kind, guid)| {
                *kind == UpdateType::CreateObject as u8 && *guid == self.creature
            })
            .collect();
        assert_eq!(
            creates.len(),
            1,
            "exactly one creature CREATE from deferred publication"
        );
        assert_eq!(self.session.last_visibility_pos, Some(self.position));
    }

    fn assert_unpublished(&self) {
        assert!(
            !self
                .session
                .client_visible_guids_like_cpp
                .contains(&self.creature)
        );
        assert!(self.output.try_recv().is_err());
        assert_eq!(self.session.last_visibility_pos, Some(self.position));
    }
}

/// Decode the first actual update block, rather than mistaking transport-time
/// VALUES or the packet's opcode for evidence that a creature was created.
fn first_update_block(bytes: &[u8]) -> Option<(u8, ObjectGuid)> {
    let mut packet = WorldPacket::from_bytes(bytes);
    if packet.read_uint16().unwrap() != ServerOpcodes::UpdateObject as u16 {
        return None;
    }
    let count = packet.read_uint32().unwrap();
    assert_eq!(packet.read_uint16().unwrap(), KEY.map_id as u16);
    if packet.read_bit().unwrap() {
        let _destroy_count = packet.read_uint16().unwrap();
        let total = packet.read_uint32().unwrap();
        for _ in 0..total {
            packet.read_packed_guid().unwrap();
        }
    }
    let size = packet.read_uint32().unwrap();
    if count == 0 {
        return None;
    }
    assert_eq!(
        count, 1,
        "fixture has one visible creature and no other remote objects"
    );
    assert!(size > 0);
    Some((
        packet.read_uint8().unwrap(),
        packet.read_packed_guid().unwrap(),
    ))
}

#[tokio::test]
async fn stationary_ack_map_directory_pump_publishes_creature_only_after_readiness() {
    let mut fixture = Fixture::new();
    fixture.session.force_update_visibility_like_cpp().await;
    for intent in fixture.tick() {
        assert!(
            fixture
                .registry
                .request_deferred_player_visibility_refresh_like_cpp(intent)
        );
    }
    fixture.pump().await;
    fixture.assert_unpublished();
    let intent = fixture.acknowledge();
    fixture.assert_unpublished();
    assert!(
        fixture
            .registry
            .request_deferred_player_visibility_refresh_like_cpp(intent)
    );
    fixture.assert_unpublished();
    fixture.pump().await;
    fixture.assert_created_once();
}

#[tokio::test]
async fn repeated_ack_refreshes_current_ledger_without_duplicate_create() {
    let mut fixture = Fixture::new();
    let first = fixture.acknowledge();
    assert!(
        fixture
            .registry
            .request_deferred_player_visibility_refresh_like_cpp(first)
    );
    fixture.pump().await;
    fixture.assert_created_once();
    let repeated = fixture.acknowledge();
    assert!(
        fixture
            .registry
            .request_deferred_player_visibility_refresh_like_cpp(repeated)
    );
    fixture.pump().await;
    assert!(
        fixture
            .session
            .client_visible_guids_like_cpp
            .contains(&fixture.creature)
    );
    assert!(
        fixture.output.try_recv().is_err(),
        "known creature has no duplicate CREATE"
    );
}

#[tokio::test]
async fn full_general_queue_retains_and_coalesces_deferred_visibility() {
    let mut fixture = Fixture::new();
    fixture
        .session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id: 999,
                instance_id: 0,
            },
        ))
        .unwrap();
    let intent = fixture.acknowledge();
    for _ in 0..3 {
        assert!(
            fixture
                .registry
                .request_deferred_player_visibility_refresh_like_cpp(intent)
        );
    }
    assert_eq!(
        fixture.session.session_command_rx.len(),
        1,
        "bounded queue stays full and untouched"
    );
    fixture.pump().await;
    fixture.assert_created_once();
    fixture.pump().await;
    assert!(fixture.output.try_recv().is_err());
}

#[tokio::test]
async fn consumer_rejects_detached_reattached_retired_and_changed_viewpoint_intents() {
    for transition in [
        "detach",
        "reattach",
        "retire",
        "replacement",
        "viewpoint",
        "missing_handle",
    ] {
        let mut fixture = Fixture::new();
        let intent = fixture.acknowledge();
        {
            let mut manager = fixture.canonical.lock().unwrap();
            match transition {
                "detach" => manager.detach_player_like_cpp(intent.handle()).unwrap(),
                "reattach" => {
                    manager.detach_player_like_cpp(intent.handle()).unwrap();
                    manager
                        .attach_player_like_cpp(intent.handle(), KEY, fixture.position)
                        .unwrap();
                }
                "retire" => {
                    manager.retire_player_like_cpp(intent.handle()).unwrap();
                }
                "replacement" => {
                    let player = manager.retire_player_like_cpp(intent.handle()).unwrap();
                    let new_handle = manager.install_detached_player_like_cpp(player).unwrap();
                    manager
                        .attach_player_like_cpp(new_handle, KEY, fixture.position)
                        .unwrap();
                }
                "viewpoint" => {
                    manager
                        .with_player_mut_like_cpp(intent.handle(), |player| {
                            player.set_farsight_object_like_cpp(fixture.creature);
                        })
                        .unwrap();
                }
                "missing_handle" => fixture.session.player_handle_like_cpp = None,
                _ => unreachable!(),
            }
        }
        let catalogs = fixture.session.creature_spawn_catalogs_for_test_like_cpp();
        fixture
            .session
            .apply_deferred_player_visibility_refresh_like_cpp(&catalogs, intent)
            .await;
        fixture.assert_unpublished();
    }
}

#[tokio::test]
async fn consumer_rejects_non_logged_in_and_disconnecting_sessions() {
    for state in [
        SessionState::Authed,
        SessionState::Transfer,
        SessionState::Disconnecting,
    ] {
        let mut fixture = Fixture::new();
        let intent = fixture.acknowledge();
        fixture.session.state = state;
        let catalogs = fixture.session.creature_spawn_catalogs_for_test_like_cpp();
        fixture
            .session
            .apply_deferred_player_visibility_refresh_like_cpp(&catalogs, intent)
            .await;
        fixture.assert_unpublished();
    }
}

#[tokio::test]
async fn registry_rejects_old_residence_without_replacing_current_retained_request() {
    let mut fixture = Fixture::new();
    let old = fixture.acknowledge();
    assert!(
        fixture
            .registry
            .request_deferred_player_visibility_refresh_like_cpp(old)
    );
    {
        let mut manager = fixture.canonical.lock().unwrap();
        manager.detach_player_like_cpp(old.handle()).unwrap();
        manager
            .attach_player_like_cpp(old.handle(), KEY, fixture.position)
            .unwrap();
    }
    let current = fixture.acknowledge();
    assert_ne!(old.residence_revision(), current.residence_revision());
    assert!(
        fixture
            .registry
            .request_deferred_player_visibility_refresh_like_cpp(current)
    );
    assert!(
        !fixture
            .registry
            .request_deferred_player_visibility_refresh_like_cpp(old)
    );
    fixture.pump().await;
    fixture.assert_created_once();
}

#[tokio::test]
async fn retained_request_is_rechecked_after_transfer_before_pump() {
    let mut fixture = Fixture::new();
    let old = fixture.acknowledge();
    assert!(
        fixture
            .registry
            .request_deferred_player_visibility_refresh_like_cpp(old)
    );
    {
        let mut manager = fixture.canonical.lock().unwrap();
        manager.detach_player_like_cpp(old.handle()).unwrap();
        manager
            .attach_player_like_cpp(old.handle(), KEY, fixture.position)
            .unwrap();
    }
    fixture.pump().await;
    fixture.assert_unpublished();
}

#[test]
fn detached_ack_does_not_mark_map_visibility_notification() {
    let mut fixture = Fixture::new();
    let handle = fixture.session.player_handle_like_cpp.unwrap();
    fixture
        .canonical
        .lock()
        .unwrap()
        .with_player_mut_like_cpp(handle, |player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .reset_all_notifies();
        })
        .unwrap();
    fixture
        .canonical
        .lock()
        .unwrap()
        .detach_player_like_cpp(handle)
        .unwrap();
    fixture
        .session
        .apply_move_init_active_mover_complete_like_cpp(25);
    assert_eq!(
        fixture
            .canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| {
                player
                    .unit()
                    .world()
                    .object()
                    .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
            }),
        Some(false)
    );
    assert!(fixture.tick().is_empty());
}

#[test]
fn retained_visibility_preserves_committed_prefix_before_presentation() {
    use crate::session::mailbox::{CreatureAttackStartLikeCppCommand, SendIfVisibleLikeCppCommand};
    let mut fixture = Fixture::new();
    let intent = fixture.acknowledge();
    {
        let mut rail = fixture
            .session
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .unwrap();
        assert!(
            rail.publish_attack_start_like_cpp(CreatureAttackStartLikeCppCommand {
                attacker_guid: fixture.creature,
                victim_guid: intent.handle().guid(),
                previous_victim_guid: None,
                map_id: KEY.map_id as u16,
                instance_id: KEY.instance_id,
                packet_already_broadcast: true,
            })
        );
        assert!(
            rail.publish_send_if_visible_like_cpp(SendIfVisibleLikeCppCommand {
                queued_at: std::time::Instant::now(),
                source_guid: fixture.creature,
                map_id: KEY.map_id as u16,
                instance_id: KEY.instance_id,
                packet_bytes: vec![1, 2],
            })
        );
    }
    assert!(
        fixture
            .registry
            .request_deferred_player_visibility_refresh_like_cpp(intent)
    );
    fixture
        .session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
        .unwrap();
    let commands = fixture.session.drain_session_commands();
    assert!(matches!(
        commands.as_slice(),
        [
            SessionCommand::CreatureAttackStartLikeCpp(_),
            SessionCommand::RefreshDeferredPlayerVisibilityLikeCpp(_),
            SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp,
            SessionCommand::SendIfVisibleLikeCpp(_),
        ]
    ));
}

#[test]
fn disconnected_session_cannot_retain_new_map_publication() {
    let mut fixture = Fixture::new();
    let intent = fixture.acknowledge();
    drop(fixture.session);
    assert!(
        !fixture
            .registry
            .request_deferred_player_visibility_refresh_like_cpp(intent)
    );
}

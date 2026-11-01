use super::*;
use std::sync::{Arc, Mutex};
use wow_core::ObjectGuid;
use wow_entities::{PlayerDeathOperationContext, PlayerDeathOperationPhase};
use wow_map::MapManager;

fn fixture() -> (SessionCore, Arc<Mutex<MapManager>>, wow_map::PlayerHandle) {
    let guid = ObjectGuid::create_player(1, 70_001);
    let mut player = Box::new(Player::new(Some(70_001), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);

    let mut map_manager = MapManager::default();
    let handle = map_manager
        .install_detached_player_like_cpp(player)
        .expect("Player should install under canonical map ownership");
    let map_manager = Arc::new(Mutex::new(map_manager));

    let (_packet_tx, packet_rx) = flume::bounded(1);
    let (send_tx, _send_rx) = flume::unbounded();
    let connection = wow_session::SessionConnection::new(send_tx, packet_rx);
    let (command_tx, command_rx) = flume::bounded(1);
    let mut core = SessionCore::new_for_world_session_like_cpp(
        1,
        "PlayerSaveOwnerTest".into(),
        0,
        2,
        2,
        54_261,
        "enUS".into(),
        command_tx,
        command_rx,
        connection,
        Vec::new(),
    );
    core.player_guid = Some(guid);
    core.player_handle_like_cpp = Some(handle);
    core.canonical_map_manager = Some(Arc::clone(&map_manager));

    (core, map_manager, handle)
}

fn capture(core: &SessionCore) -> Option<CapturedPlayerSaveLikeCpp> {
    let tutorials_changed = false;
    let tutorials_loaded_coherently = false;
    let tutorials = [0; 8];
    let tutorials_loaded_from_db = false;
    let login_time = None;
    let total_played_time = 0;
    let level_played_time = 0;
    let inputs = PlayerSaveSessionInputsLikeCpp::new_like_cpp(
        1_700_000_000,
        &tutorials_changed,
        &tutorials_loaded_coherently,
        &tutorials,
        &tutorials_loaded_from_db,
        &login_time,
        &total_played_time,
        &level_played_time,
    );
    PlayerSaveOwnerAccessLikeCpp::new_like_cpp(
        core, None, None, None, None, None, None, None, None, None,
    )
    .capture_like_cpp(inputs)
}

#[test]
fn unresolved_native_death_refuses_full_save_capture_until_complete() {
    for phase in [
        PlayerDeathOperationPhase::Ready,
        PlayerDeathOperationPhase::InFlight,
    ] {
        let (core, manager, handle) = fixture();
        let context = PlayerDeathOperationContext::new(ObjectGuid::create_player(1, 70_002), true);
        let id = manager
            .lock()
            .unwrap()
            .with_player_mut_like_cpp(handle, |player| {
                let id = player
                    .begin_native_death_operation(context)
                    .expect("living Player should admit native death operation");
                if phase == PlayerDeathOperationPhase::InFlight {
                    assert_eq!(player.claim_native_death_operation(id), Some(context));
                }
                id
            })
            .expect("canonical Player handle should resolve");

        assert!(
            capture(&core).is_none(),
            "{phase:?} native death operation must block canonical full-save capture"
        );
        let snapshot = manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| player.native_death_operation_snapshot())
            .flatten()
            .expect("refused save must retain the Player-owned operation");
        assert_eq!(snapshot.id(), id);
        assert_eq!(snapshot.context(), context);
        assert_eq!(snapshot.phase(), phase);

        manager
            .lock()
            .unwrap()
            .with_player_mut_like_cpp(handle, |player| {
                if phase == PlayerDeathOperationPhase::Ready {
                    assert_eq!(player.claim_native_death_operation(id), Some(context));
                }
                assert!(player.complete_native_death_operation(id));
                assert!(!player.has_unresolved_native_death_operation());
            })
            .expect("canonical Player handle should remain current");
        assert!(
            capture(&core).is_some(),
            "a completed operation should resume ordinary full-save capture"
        );
    }
}

#[test]
fn ordinary_full_save_capture_without_native_death_operation_is_unchanged() {
    let (core, _, _) = fixture();
    assert!(capture(&core).is_some());
}

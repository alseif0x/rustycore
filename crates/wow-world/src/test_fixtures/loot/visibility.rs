//! The original cast delivery setup over the actual legacy map and connection queue.
use super::*;

pub fn make_loot_cast_delivery_fixture()
-> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let source_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 91_500);
    let manager = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            source_guid,
            777,
            Position::ZERO,
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
    session.set_state(SessionState::LoggedIn);
    session.set_map_manager(manager);
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);
    (session, send_rx, source_guid)
}


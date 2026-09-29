pub fn set_player_position_for_test(
    session: &mut crate::session::WorldSession,
    position: wow_core::Position,
) {
    session.set_player_position_like_cpp(position);
}

pub fn player_position_for_test(
    session: &crate::session::WorldSession,
) -> Option<wow_core::Position> {
    session.player_position_like_cpp()
}

pub fn set_player_health_for_test(
    session: &mut crate::session::WorldSession,
    health: u32,
    max_health: u32,
) {
    session.set_player_health_like_cpp(health, max_health);
}

pub fn handle_under_map_for_test(
    session: &mut crate::session::WorldSession,
    movement_info: &wow_packet::packets::movement::MovementInfo,
) -> bool {
    session.handle_under_map_like_cpp(movement_info).is_some()
}

pub fn player_is_alive_for_test(session: &crate::session::WorldSession) -> bool {
    session.player_is_alive_like_cpp()
}

pub fn unregister_from_player_registry_for_test(session: &crate::session::WorldSession) {
    session.unregister_from_player_registry();
}

pub fn insert_client_visible_guid_for_test(
    session: &mut crate::session::WorldSession,
    guid: wow_core::ObjectGuid,
) {
    session.client_visible_guids_like_cpp.insert(guid);
}

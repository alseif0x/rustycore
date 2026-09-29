pub fn canonical_player_health_snapshot_for_test(
    session: &crate::session::WorldSession,
) -> Option<(u32, u32)> {
    session.canonical_player_health_snapshot_like_cpp()
}

pub fn canonical_player_power_snapshot_for_test(
    session: &crate::session::WorldSession,
    power_type: wow_constants::PowerType,
) -> Option<(i32, i32)> {
    session.canonical_player_power_snapshot_like_cpp(power_type)
}

pub fn get_inventory_item_by_pos_for_test(
    session: &crate::session::WorldSession,
    bag: u8,
    slot: u8,
) -> Option<crate::session::InventoryItem> {
    session.get_inventory_item_by_pos(bag, slot)
}

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

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

pub fn player_interaction_source_guid_for_test(
    session: &crate::session::WorldSession,
) -> Option<wow_core::ObjectGuid> {
    session.player_interaction_source_guid_like_cpp()
}

pub fn set_player_interaction_source_for_test(
    session: &mut crate::session::WorldSession,
    source_guid: wow_core::ObjectGuid,
) -> bool {
    session.set_player_interaction_source_like_cpp(source_guid)
}

pub fn player_interaction_trainer_id_for_test(
    session: &crate::session::WorldSession,
) -> u32 {
    session.player_interaction_trainer_id_like_cpp()
}

pub fn set_player_trainer_interaction_for_test(
    session: &mut crate::session::WorldSession,
    source_guid: wow_core::ObjectGuid,
    trainer_id: u32,
) -> bool {
    session.set_player_trainer_interaction_like_cpp(source_guid, trainer_id)
}

pub fn reset_player_interaction_if_source_for_test(
    session: &mut crate::session::WorldSession,
    source_guid: wow_core::ObjectGuid,
) -> bool {
    session.reset_player_interaction_if_source_like_cpp(source_guid)
}

/// Push one option into the ownerless PlayerMenu fixture, matching the former
/// direct test write without exposing its mutable collection.
pub fn push_player_gossip_option_for_test(
    session: &mut crate::session::WorldSession,
    option: crate::session::GossipOptionInfo,
) {
    session.gossip_options.push(option);
}

/// Snapshot the ownerless PlayerMenu fixture for assertions.
pub fn player_gossip_options_for_test(
    session: &crate::session::WorldSession,
) -> Vec<crate::session::GossipOptionInfo> {
    session.gossip_options.clone()
}

pub fn install_canonical_player_owner_for_test(
    session: &mut crate::session::WorldSession,
    map_id: u32,
    instance_id: u32,
) -> wow_core::ObjectGuid {
    crate::canonical_player_access::install_canonical_player_owner_for_test(
        session,
        map_id,
        instance_id,
    )
}

pub fn tick_player_regeneration_for_test(
    session: &mut crate::session::WorldSession,
    diff_ms: u32,
    power_types: &wow_data::character_progression::PowerTypeStore,
    regen_game_tables: Option<&wow_data::RegenGameTablesLikeCpp>,
    rates: &crate::PlayerRegenerationRatesLikeCpp,
) {
    session.tick_player_regeneration_like_cpp(diff_ms, power_types, regen_game_tables, rates);
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

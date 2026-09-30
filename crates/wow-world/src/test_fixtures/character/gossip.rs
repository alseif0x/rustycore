//! Narrow Gossip operations and observations of the existing fixture rail.

use crate::session::WorldSession;
use std::sync::{Arc, Mutex};
use wow_core::{ObjectGuid, Position};

pub use crate::handlers::character::gossip_fixture_access::{
    GOSSIP_TRAINER_OPTION_ID_FOR_TEST, GOSSIP_TRAINER_OPTION_NPC_FOR_TEST,
};

pub fn enable_gossip_fixture_for_test(session: &mut WorldSession) {
    session.enable_gossip_fixture_for_test();
}

pub fn make_gossip_bank_session_for_test(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>, Arc<Mutex<wow_map::MapManager>>) {
    crate::handlers::character::gossip_fixture_access::make_gossip_bank_session_for_test(capacity)
}

pub fn insert_gossip_binder_creature_for_test(
    manager: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    npc_flags: u32,
) {
    crate::handlers::character::gossip_fixture_access::insert_gossip_binder_creature_for_test(
        manager, guid, npc_flags,
    );
}

pub async fn handle_binder_activate_for_test(
    session: &mut WorldSession,
    hello: wow_packet::packets::gossip::Hello,
) {
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawn_catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    session.handle_binder_activate_with_generator_like_cpp(
        generators.item.as_ref(),
        &creature_spawn_catalogs,
        hello,
    ).await;
}

pub async fn build_gossip_menu_for_test(
    session: &mut WorldSession,
    entry: u32,
    npc_flags: u32,
    npc_guid: ObjectGuid,
) -> Option<wow_packet::packets::gossip::GossipMessage> {
    session.build_gossip_menu(entry, npc_flags, npc_guid).await
}

pub fn gossip_quest_text_for_test(
    session: &WorldSession,
    entry: u32,
) -> Vec<wow_packet::packets::gossip::ClientGossipText> {
    session.represented_creature_gossip_text_like_cpp(entry)
}

pub fn gossip_trainer_interaction_matches_for_test(
    session: &WorldSession,
    source: ObjectGuid,
    trainer_id: i32,
) -> bool {
    session.player_trainer_interaction_matches_like_cpp(source, trainer_id)
}

pub fn gossip_homebind_for_test(
    session: &WorldSession,
) -> Option<wow_entities::PlayerHomebindLikeCpp> {
    session.represented_homebind_like_cpp()
}

pub fn has_gossip_visible_aura_for_test(session: &WorldSession, slot: u8) -> bool {
    session.visible_auras.contains_key(&slot)
}

pub fn insert_gossip_visible_aura_for_test(
    session: &mut WorldSession,
    slot: u8,
    aura: wow_entities::AuraApplicationLikeCpp,
) {
    session.visible_auras.insert(slot, aura);
}

pub fn install_gossip_gameobject_state_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    map_id: Option<u16>,
    position: Option<Position>,
    go_type: Option<u8>,
    icon_name_allows_interaction: Option<bool>,
) {
    session.represented_gameobject_use_states.insert(
        guid,
        crate::session::RepresentedGameObjectUseState {
            map_id,
            position,
            go_type,
            icon_name_allows_interaction_like_cpp: icon_name_allows_interaction,
            ..Default::default()
        },
    );
}

pub fn set_gossip_taxi_flight_for_test(
    session: &mut WorldSession,
    current_node: wow_entities::PlayerTaxiFlightNodeLikeCpp,
    node_after_teleport: Option<wow_entities::PlayerTaxiFlightNodeLikeCpp>,
) {
    session.install_gossip_taxi_flight_for_test(current_node, node_after_teleport);
}

pub fn set_gossip_player_phase_for_test(
    session: &mut WorldSession,
    phase: wow_entities::PhaseShift,
) -> bool {
    session.set_represented_player_phase_shift_like_cpp(phase)
}

pub fn set_gossip_gameobject_phase_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    phase: wow_entities::PhaseShift,
) {
    session.record_represented_gameobject_phase_shift_like_cpp(guid, phase);
}

pub fn adopt_gossip_canonical_player_for_test(session: &mut WorldSession) -> bool {
    session.adopt_registered_canonical_player_fixture_like_cpp()
}

pub fn install_gossip_realm_send_channel_for_test(
    session: &mut WorldSession,
    sender: flume::Sender<Vec<u8>>,
) {
    session.install_realm_send_channel_for_test(sender);
}

pub fn set_gossip_zone_area_for_test(session: &mut WorldSession, zone: u32, area: u32) {
    session.set_player_zone_area_like_cpp(zone, area);
}

pub fn attach_gossip_observer_controller_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    name: String,
    position: Position,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    session.attach_player_controller_for_fixture(crate::session::SessionPlayerController::new(
        guid, name, position, map_id, race, class, level, gender,
    ));
}

pub fn register_gossip_player_for_test(session: &WorldSession) {
    session.register_in_player_registry_with_fixture_hydration();
}

pub async fn process_gossip_session_commands_for_test(session: &mut WorldSession) {
    session.process_represented_session_commands_like_cpp().await;
}

pub fn gossip_taxi_snapshot_for_test(
    session: &WorldSession,
) -> Option<wow_entities::PlayerTaxiState> {
    session.player_taxi_state_snapshot_like_cpp()
}

pub fn gossip_aura_snapshot_for_test(
    session: &WorldSession,
) -> Option<wow_entities::AuraSubsystem> {
    session.player_aura_subsystem_snapshot_like_cpp()
}

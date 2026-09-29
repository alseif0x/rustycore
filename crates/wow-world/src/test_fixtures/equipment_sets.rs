//! Narrow equipment-set fixtures for external integration tests.

use std::sync::Arc;

use crate::session::WorldSession;
use wow_core::EquipmentSetGuidGeneratorLikeCpp;
use wow_entities::PlayerEquipmentSetLikeCpp;
use wow_packet::WorldPacket;

pub fn insert_represented_equipment_set_for_test(
    session: &mut WorldSession,
    guid: u64,
    equipment_set: PlayerEquipmentSetLikeCpp,
) {
    session.insert_represented_equipment_set_like_cpp(guid, equipment_set);
}

pub fn represented_equipment_set_for_test(
    session: &WorldSession,
    guid: u64,
) -> Option<PlayerEquipmentSetLikeCpp> {
    session.represented_equipment_set_like_cpp(guid)
}

pub fn set_equipment_set_guid_generator_for_test(
    session: &mut WorldSession,
    generator: Arc<EquipmentSetGuidGeneratorLikeCpp>,
) {
    session.set_equipment_set_guid_generator_like_cpp(generator);
}

pub async fn handle_save_equipment_set_for_test(session: &mut WorldSession, pkt: WorldPacket) {
    session.handle_save_equipment_set(pkt).await;
}

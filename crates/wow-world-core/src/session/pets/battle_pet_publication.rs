use tracing::warn;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::battle_pet_adapter::{
    RepresentedBattlePetDataLikeCpp, RepresentedBattlePetSaveInfoLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;

impl crate::session::HubMut<'_> {
    /// C++ `BattlePetMgr::SendUpdates`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn send_battle_pet_updates_like_cpp(
        &mut self,
        pet_guids: &[ObjectGuid],
        pet_added: bool,
    ) -> usize {
        let pets: Vec<_> = pet_guids
            .iter()
            .filter_map(|pet_guid| {
                let pet = self
                    .fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pets_like_cpp
                    .get(pet_guid)?;
                if pet.save_info == RepresentedBattlePetSaveInfoLikeCpp::Removed {
                    return None;
                }
                Some(pet.packet_info_like_cpp(*pet_guid))
            })
            .collect();
        let sent_count = pets.len();
        self.core
            .send_packet(&wow_packet::packets::misc::BattlePetUpdates { pets, pet_added });
        sent_count
    }

    /// Publish one durable battle-pet addition exactly like the #160 session
    /// seam (`SMSG_BATTLE_PET_UPDATES` with `pet_added` plus the C++
    /// packet). The issue #161 saga calls this only after the pet and receipt
    /// are durable. Packet recovery may call it again, so it deliberately has
    /// no criteria side effects.
    pub fn publish_battle_pet_trainer_purchase_add_like_cpp(
        &mut self,
        pet: wow_packet::packets::misc::BattlePetJournalPet,
    ) -> bool {
        let packet_enqueued = self
            .core
            .send_tx()
            .send(wow_packet::ServerPacket::to_bytes(
                &wow_packet::packets::misc::BattlePetUpdates {
                    pets: vec![pet],
                    pet_added: true,
                },
            ))
            .is_ok();
        if !packet_enqueued {
            warn!("Send channel closed for account {}", self.core.account_id);
        }
        packet_enqueued
    }

    /// C++ `BattlePetMgr::SendError`.
    pub fn battle_pet_send_error_like_cpp(
        &mut self,
        error: wow_packet::packets::misc::BattlePetErrorCodeLikeCpp,
        creature_id: u32,
    ) {
        self.core
            .send_packet(&wow_packet::packets::misc::BattlePetError::new(
                error,
                i32::try_from(creature_id).unwrap_or(i32::MAX),
            ));
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PetState {
    /// Test/setup seam for represented `BattlePet::PacketInfo` rows already
    /// loaded into `BattlePetMgr::_pets`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn add_represented_battle_pet_packet_info_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        packet_info: RepresentedBattlePetDataLikeCpp,
    ) {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .insert(pet_guid, packet_info);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battle_pet_data_updates_like_cpp(&self) -> &[ObjectGuid] {
        &self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_data_updates_like_cpp
    }
}

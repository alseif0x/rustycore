//! Battle-pet packets and updates published to the client.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Test/setup seam for represented `BattlePet::PacketInfo` rows already
    /// loaded into `BattlePetMgr::_pets`.
    #[cfg(test)]
    pub(crate) fn add_represented_battle_pet_packet_info_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        packet_info: RepresentedBattlePetDataLikeCpp,
    ) {
        self.represented_battle_pets_like_cpp
            .insert(pet_guid, packet_info);
    }
    /// C++ `BattlePetMgr::SendUpdates`.
    #[cfg(test)]
    pub(crate) fn send_battle_pet_updates_like_cpp(
        &mut self,
        pet_guids: &[ObjectGuid],
        pet_added: bool,
    ) -> usize {
        let pets: Vec<_> = pet_guids
            .iter()
            .filter_map(|pet_guid| {
                let pet = self.represented_battle_pets_like_cpp.get(pet_guid)?;
                if pet.save_info == RepresentedBattlePetSaveInfoLikeCpp::Removed {
                    return None;
                }
                Some(pet.packet_info_like_cpp(*pet_guid))
            })
            .collect();
        let sent_count = pets.len();
        self.send_packet(&wow_packet::packets::misc::BattlePetUpdates { pets, pet_added });
        sent_count
    }
    /// Publish one durable battle-pet addition exactly like the #160 session
    /// seam (`SMSG_BATTLE_PET_UPDATES` with `pet_added` plus the C++
    /// packet). The issue #161 saga calls this only after the pet and receipt
    /// are durable. Packet recovery may call it again, so it deliberately has
    /// no criteria side effects.
    pub(crate) fn publish_battle_pet_trainer_purchase_add_like_cpp(
        &mut self,
        pet: wow_packet::packets::misc::BattlePetJournalPet,
    ) -> bool {
        let packet_enqueued = self
            .send_tx()
            .send(wow_packet::ServerPacket::to_bytes(
                &wow_packet::packets::misc::BattlePetUpdates {
                    pets: vec![pet],
                    pet_added: true,
                },
            ))
            .is_ok();
        if !packet_enqueued {
            warn!("Send channel closed for account {}", self.account_id);
        }
        packet_enqueued
    }
    /// C++ `BattlePetMgr::SendError`.
    pub(crate) fn battle_pet_send_error_like_cpp(
        &mut self,
        error: wow_packet::packets::misc::BattlePetErrorCodeLikeCpp,
        creature_id: u32,
    ) {
        self.send_packet(&wow_packet::packets::misc::BattlePetError::new(
            error,
            i32::try_from(creature_id).unwrap_or(i32::MAX),
        ));
    }
    /// C++ `BattlePetMgr::UpdateBattlePetData`, represented at the gate level.
    pub(crate) fn battle_pet_update_notify_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        let Some(pet) = self.represented_battle_pet_like_cpp(pet_guid) else {
            return false;
        };

        if self.represented_summoned_battle_pet_guid_like_cpp() != Some(pet_guid) {
            return false;
        }

        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.set_battle_pet_data_like_cpp(pet_guid, pet.quality, pet.level);
        });
        #[cfg(test)]
        self.represented_battle_pet_data_updates_like_cpp
            .push(pet_guid);
        true
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_data_updates_like_cpp(&self) -> &[ObjectGuid] {
        &self.represented_battle_pet_data_updates_like_cpp
    }
}

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Equipment-set packet handlers and represented set mutations.

use super::*;

impl WorldSession {
    /// Handle CMSG_SAVE_EQUIPMENT_SET.
    ///
    /// C++ validates the equipment/transmog payload, normalizes ignored slots,
    /// then calls `Player::SetEquipmentSet`. Rust mirrors the in-memory dirty
    /// state and new-set `SMSG_EQUIPMENT_SET_ID`; the next full player save
    /// appends `_SaveEquipmentSets`-shaped statements to its transaction.
    pub async fn handle_save_equipment_set_with_generator_like_cpp(
        &mut self,
        generator: &wow_core::EquipmentSetGuidGeneratorLikeCpp,
        mut pkt: WorldPacket,
    ) {
        let request = match SaveEquipmentSet::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad SaveEquipmentSet: {error}");
                return;
            }
        };

        let Some(saved) =
            self.save_represented_equipment_set_with_generator_like_cpp(generator, request.set)
        else {
            return;
        };

        if saved.generated_new_guid {
            self.send_packet(&EquipmentSetId {
                guid: saved.guid,
                set_type: saved.raw_set_type,
                set_id: saved.set_id,
            });
        }
    }

    #[cfg(test)]
    pub async fn handle_save_equipment_set(&mut self, pkt: WorldPacket) {
        let Some(generator) = self.equipment_set_guid_generator_for_test_like_cpp() else {
            return;
        };
        self.handle_save_equipment_set_with_generator_like_cpp(generator.as_ref(), pkt)
            .await;
    }

    /// Handle CMSG_ASSIGN_EQUIPMENT_SET_SPEC.
    ///
    /// C++ `Player::AssignEquipmentSetToSpec` only mutates the first equipment
    /// set whose client SetID matches and does not send an immediate response.
    /// The represented container keeps the same in-memory assignment/state
    /// semantics before the next full player-save transaction persists them.
    pub async fn handle_assign_equipment_set_spec(&mut self, mut pkt: WorldPacket) {
        let request = match AssignEquipmentSetSpec::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad AssignEquipmentSetSpec: {error}");
                return;
            }
        };

        let _assigned = self
            .assign_represented_equipment_set_to_spec_like_cpp(request.set_id, request.spec_index);
    }

    /// Handle CMSG_DELETE_EQUIPMENT_SET.
    ///
    /// C++ marks existing equipment/transmog sets as deleted unless the set was
    /// still new in memory, in which case it removes it immediately. The DB
    /// delete happens later in `_SaveEquipmentSets`.
    pub async fn handle_delete_equipment_set(&mut self, mut pkt: WorldPacket) {
        let request = match DeleteEquipmentSet::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad DeleteEquipmentSet: {error}");
                return;
            }
        };

        let _deleted = self.delete_represented_equipment_set_like_cpp(request.id);
    }

    /// Handle CMSG_USE_EQUIPMENT_SET.
    ///
    /// C++ `HandleUseEquipmentSet` iterates all 19 equipment slots, skips the
    /// ignored GUID sentinel and non-weapon slots in combat, then uses
    /// `GetItemByGuid` + `SwapItem` / `CanStoreItem` to move gear. This slice
    /// mirrors the represented direct-inventory state and the result packet;
    /// full nested-container validation, `CanEquipItem`, DB writes, and item
    /// update fanout remain later inventory-runtime work.
    pub async fn handle_use_equipment_set(&mut self, mut pkt: WorldPacket) {
        let request = match UseEquipmentSet::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad UseEquipmentSet: {error}");
                return;
            }
        };

        let represented_item_mods_changed = self.use_represented_equipment_set_like_cpp(&request);
        if represented_item_mods_changed {
            self.send_represented_item_bonus_player_stat_update_like_cpp();
        }
        self.send_packet(&UseEquipmentSetResult {
            guid: request.guid,
            reason: 0,
        });
    }
}

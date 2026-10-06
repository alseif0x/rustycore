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
        pkt: WorldPacket,
    ) {
        self.build_equipment_sets_save_handler_cx_like_cpp(generator)
            .handle_save_equipment_set(pkt);
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
    /// C++ `Opcodes.cpp:170` marks this opcode `STATUS_UNHANDLED` and dispatches
    /// `Handle_NULL`. This Rust handler currently assigns the first matching
    /// equipment-set ID without a response; the next full player save persists it.
    /// This version difference requires F6 evidence before claiming parity.
    pub async fn handle_assign_equipment_set_spec(&mut self, pkt: WorldPacket) {
        self.build_equipment_sets_handler_cx_like_cpp()
            .handle_assign_equipment_set_spec(pkt);
    }

    /// Handle CMSG_DELETE_EQUIPMENT_SET.
    ///
    /// C++ marks existing equipment/transmog sets as deleted unless the set was
    /// still new in memory, in which case it removes it immediately. The DB
    /// delete happens later in `_SaveEquipmentSets`.
    pub async fn handle_delete_equipment_set(&mut self, pkt: WorldPacket) {
        self.build_equipment_sets_handler_cx_like_cpp()
            .handle_delete_equipment_set(pkt);
    }

    /// Handle CMSG_USE_EQUIPMENT_SET.
    ///
    /// Decode and delegate CMSG_USE_EQUIPMENT_SET to its Application owner.
    pub async fn handle_use_equipment_set(&mut self, pkt: WorldPacket) {
        self.build_equipment_set_use_context_like_cpp()
            .handle_use_equipment_set_like_cpp(pkt);
    }
}

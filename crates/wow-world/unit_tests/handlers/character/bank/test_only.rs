// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `bank` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    /// CMSG_CHANGE_BANK_BAG_SLOT_FLAG — player toggles an ActivePlayer bank bag flag.
    ///
    /// Target Opcodes.cpp:289 registers STATUS_UNHANDLED / Handle_NULL.
    /// F5 preserves the existing Rust operation; this target difference remains F6.
    pub(crate) async fn handle_change_bank_bag_slot_flag(&mut self, packet: ChangeBankBagSlotFlag) {
        self.build_bank_slot_flag_handler_cx_like_cpp()
            .change_bank_bag_slot_flag_like_cpp(packet);
    }
}

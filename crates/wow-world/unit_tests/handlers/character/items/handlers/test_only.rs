// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `handlers` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    /// Handle CMSG_CANCEL_TEMP_ENCHANTMENT.
    ///
    /// C++ ref: `WorldSession::HandleCancelTempEnchantmentOpcode`.
    pub(crate) async fn handle_cancel_temp_enchantment(&mut self, cancel: CancelTempEnchantment) {
        self.build_item_enchantment_handler_cx_like_cpp()
            .cancel_temp_enchantment_like_cpp(cancel);
    }
}

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::SessionState;
use crate::session::state::SessionCore;
use tracing::warn;

impl SessionCore {
    pub fn battlenet_account_id(&self) -> u32 {
        self.account_state.battlenet_account_id
    }

    /// Kick the session (mark as disconnecting).
    pub fn kick(&mut self, reason: &str) {
        warn!(
            "Kicking account {} ({}): {reason}",
            self.account_id, self.account_name
        );
        self.state = SessionState::Disconnecting;
    }
}

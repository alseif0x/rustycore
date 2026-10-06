// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::SessionCore;
use wow_core::ObjectGuid;

/// Borrowed SessionCore roles required by an exclusive player-money transaction.
///
/// Keep this capability for the transaction's lifetime; it resolves identity
/// when requested and exposes neither SessionCore nor canonical Player storage.
pub struct PlayerMoneyTransactionSessionAccessLikeCpp<'a> {
    core: &'a mut SessionCore,
}

impl SessionCore {
    /// Borrow the narrow SessionCore access used by player-money transactions.
    pub fn player_money_transaction_access_like_cpp(
        &mut self,
    ) -> PlayerMoneyTransactionSessionAccessLikeCpp<'_> {
        PlayerMoneyTransactionSessionAccessLikeCpp { core: self }
    }
}

impl PlayerMoneyTransactionSessionAccessLikeCpp<'_> {
    /// Resolve the current session Player GUID at the operation's call site.
    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }

    /// Mark this session disconnecting after an indeterminate money commit.
    pub fn quarantine_like_cpp(&mut self, reason: &str) {
        self.core.kick(reason);
    }
}

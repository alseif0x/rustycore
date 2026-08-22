// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login-side lifecycle: the single-live-session character claim.
//!
//! One character may only be logging in through one session at a time. The
//! claim is taken before the login sequence commits and released on any exit
//! path — completion, a failed ConnectTo, or disconnect — so a dropped
//! instance socket cannot leave a character permanently unloggable.

use std::sync::Arc;

use super::super::{ACTIVE_CHARACTER_LOGIN_CLAIMS_LIKE_CPP, ObjectGuid, WorldSession};

impl WorldSession {
    /// Atomically reserve the only live runtime authority for `guid`.
    /// Re-entry by this same session is idempotent; a live foreign claim is
    /// rejected before either session can load and later save stale rows.
    pub(crate) fn try_claim_character_login_like_cpp(&mut self, guid: ObjectGuid) -> bool {
        if self
            .player_login_claim_like_cpp
            .as_ref()
            .is_some_and(|(claimed_guid, _)| *claimed_guid == guid)
        {
            return true;
        }
        self.release_character_login_claim_like_cpp();

        let claims = ACTIVE_CHARACTER_LOGIN_CLAIMS_LIKE_CPP.get_or_init(Default::default);
        match claims.entry(guid) {
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                let identity = Arc::new(());
                entry.insert(Arc::downgrade(&identity));
                self.player_login_claim_like_cpp = Some((guid, identity));
                true
            }
            dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                if entry.get().upgrade().is_some() {
                    return false;
                }
                let identity = Arc::new(());
                entry.insert(Arc::downgrade(&identity));
                self.player_login_claim_like_cpp = Some((guid, identity));
                true
            }
        }
    }

    pub(crate) fn release_character_login_claim_like_cpp(&mut self) {
        let Some((guid, identity)) = self.player_login_claim_like_cpp.take() else {
            return;
        };
        let Some(claims) = ACTIVE_CHARACTER_LOGIN_CLAIMS_LIKE_CPP.get() else {
            return;
        };
        if let Some(entry) = claims.get(&guid) {
            let owns_claim = entry
                .upgrade()
                .is_some_and(|current| Arc::ptr_eq(&current, &identity));
            drop(entry);
            if owns_claim {
                claims.remove(&guid);
            }
        }
    }
}

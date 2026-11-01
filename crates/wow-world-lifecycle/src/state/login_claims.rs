use std::sync::{Arc, OnceLock, Weak};

use wow_core::ObjectGuid;

use super::SessionLifecycleState;

/// Process-wide ownership of a character's live `Player` runtime.
///
/// C++ has one `Player*` per GUID in `ObjectAccessor`; accepting a second
/// session would create two independent save authorities for the same rows.
/// Reserve the GUID before the asynchronous login pipeline starts. Weak
/// values make an abandoned session claim recoverable without a global sweep.
static ACTIVE_CHARACTER_LOGIN_CLAIMS_LIKE_CPP: OnceLock<dashmap::DashMap<ObjectGuid, Weak<()>>> =
    OnceLock::new();

impl SessionLifecycleState {
    /// Atomically reserve the only live runtime authority for `guid`.
    /// Re-entry by this same session is idempotent; a live foreign claim is
    /// rejected before either session can load and later save stale rows.
    pub fn try_claim_character_login_like_cpp(&mut self, guid: ObjectGuid) -> bool {
        if let Some(operation) = &self.finalization {
            if operation.report().disposition != crate::FinalizationDisposition::Complete {
                return false;
            }
            self.finalization = None;
        }
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

    pub fn release_character_login_claim_like_cpp(&mut self) {
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

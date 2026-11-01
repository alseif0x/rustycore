//! Route a map-selected obligation to the current Session's retained mailbox.

use super::*;
use wow_map::PlayerVisibilityRefreshIntentLikeCpp;

impl PlayerRegistry {
    /// Coalesce and queue a visibility refresh for a current recipient.
    pub fn request_current_visibility_refresh(
        &self,
        registration: PlayerRegistration,
        map_id: u16,
        instance_id: u32,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let pending = Arc::clone(&entry.visibility_refresh_pending_like_cpp);
        let tx = entry.command_tx.clone();
        drop(entry);
        pending.store(true, Ordering::Release);
        let command = SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id,
                instance_id,
            },
        );
        match tx.try_send(command) {
            Ok(()) | Err(flume::TrySendError::Full(_)) => Ok(()),
            Err(flume::TrySendError::Disconnected(_)) => {
                pending.store(false, Ordering::Release);
                Err(PlayerDirectorySendError::Disconnected)
            }
        }
    }

    /// Called by the existing map loop after releasing the canonical manager.
    /// One coalesced slot on the existing durable rail survives a full general
    /// queue without another task, lock, bool fallback or mutable Player mirror.
    pub fn request_deferred_player_visibility_refresh_like_cpp(
        &self,
        intent: PlayerVisibilityRefreshIntentLikeCpp,
    ) -> bool {
        let current = self
            .canonical_map_manager_like_cpp()
            .is_some_and(|manager| {
                manager.lock().is_ok_and(|manager| {
                    manager.player_visibility_refresh_intent_is_current_like_cpp(intent)
                })
            });
        if !current {
            return false;
        }
        let Some(entry) = self.entries.get(&intent.handle().guid()) else {
            return false;
        };
        if entry.command_tx.is_disconnected() {
            return false;
        }
        // Keep registration stable while retaining the obligation. No map guard
        // is held here. PlayerHandle and directory generation are distinct; the
        // consumer verifies its own exact canonical handle before application.
        let Ok(mut pending) = entry.durable_creature_runtime_commands_like_cpp.lock() else {
            return false;
        };
        pending.retain_deferred_visibility_like_cpp(intent);
        true
    }
}

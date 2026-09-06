//! Canonical residence resolution for lifecycle and execution admission.
//!
//! C++ `WorldSession.cpp:64-108` reads the live Player and IsInWorld(), not a
//! Session status label. `Map.cpp:427-462,907-934` owns active insertion/removal.
//! Resolve index, incarnation, container and binding under the caller's existing
//! manager guard. The returned value is an observation, not a lease across await.

use super::{MapKey, MapManager, PlayerHandle, PlayerOwnerError, PlayerResidenceLikeCpp};

impl MapManager {
    /// Resolve a current incarnation's actual residence without guessing defaults.
    ///
    /// No owner, stale generation and inconsistent backing state are distinct.
    /// A detached Player is valid and remains available for non-spatial operations.
    /// Callers must keep admission and synchronous mutation under the same owner
    /// guard; this query alone does not prevent a subsequent detach or replacement.
    pub fn checked_player_residence_like_cpp(
        &self,
        handle: PlayerHandle,
    ) -> Result<PlayerResidenceLikeCpp, PlayerOwnerError> {
        let guid = handle.guid();
        let owner = self
            .player_owners_like_cpp
            .get(&guid)
            .ok_or(PlayerOwnerError::MissingOwner { guid })?;
        if owner.generation != handle.generation() {
            return Err(PlayerOwnerError::StaleHandle);
        }
        let player = match owner.residence {
            PlayerResidenceLikeCpp::Detached => self
                .detached_players_like_cpp
                .get(&guid)
                .map(Box::as_ref)
                .ok_or(PlayerOwnerError::MissingPlayer { guid })?,
            PlayerResidenceLikeCpp::Active(key) => self
                .find_map(key.map_id, key.instance_id)
                .ok_or(PlayerOwnerError::MissingMap { key })?
                .map()
                .get_typed_player(guid)
                .ok_or(PlayerOwnerError::ActivePlayerMissing { guid })?,
        };
        if player.guid() != guid {
            return Err(PlayerOwnerError::PlayerGuidMismatch {
                expected: guid,
                actual: player.guid(),
            });
        }
        let world = player.unit().world();
        let actual_map = world
            .has_current_map()
            .then(|| MapKey::new(world.map_id(), world.instance_id()));
        match owner.residence {
            PlayerResidenceLikeCpp::Detached => {
                if world.object().is_in_world() {
                    return Err(PlayerOwnerError::DetachedPlayerStillInWorld { guid });
                }
                if let Some(key) = actual_map {
                    return Err(PlayerOwnerError::DetachedPlayerStillBound { guid, key });
                }
            }
            PlayerResidenceLikeCpp::Active(expected) => {
                if !world.object().is_in_world() {
                    return Err(PlayerOwnerError::ActivePlayerNotInWorld { guid });
                }
                if actual_map != Some(expected) {
                    return Err(PlayerOwnerError::ActivePlayerMapMismatch {
                        guid,
                        expected,
                        actual: actual_map,
                    });
                }
            }
        }
        Ok(owner.residence)
    }
}

#[cfg(test)]
mod tests;

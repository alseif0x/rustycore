// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::SessionCore;
use wow_core::ObjectGuid;

/// A short-lived view of only the canonical trainer interaction role.
///
/// Hub-backed InteractionState and acquisition contexts share these exact
/// Player reads/writes without weakening acquisition's exclusive Core borrow.
pub struct TrainerInteractionRoleAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    pub fn trainer_interaction_role_access_like_cpp(
        &self,
    ) -> TrainerInteractionRoleAccessLikeCpp<'_> {
        TrainerInteractionRoleAccessLikeCpp { core: self }
    }
}

impl TrainerInteractionRoleAccessLikeCpp<'_> {
    pub fn set_trainer_interaction_like_cpp(
        &self,
        source_guid: ObjectGuid,
        trainer_id: u32,
    ) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_trainer_interaction_like_cpp(source_guid, trainer_id);
            })
            .is_some()
    }

    pub fn trainer_interaction_matches_like_cpp(
        &self,
        source_guid: ObjectGuid,
        trainer_id: i32,
    ) -> Option<bool> {
        self.core.with_owned_player_like_cpp(|player| {
            player
                .interaction_data_like_cpp()
                .trainer_matches(source_guid, trainer_id)
        })
    }

    pub fn player_handle_absent_like_cpp(&self) -> bool {
        self.core
            .owned_spell_acquisition_access_like_cpp()
            .player_handle_absent_like_cpp()
    }
}

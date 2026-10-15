// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter of the far-transfer completion host (#1363 slice 13a). The
//! method returns the deferred save resume's own future unchanged; its body
//! moved to `wow_world_application::session_lifecycle` (#1363 slice 13b).

use std::future::Future;

use wow_world_character::WorldportHostLikeCpp;

use super::PlayerSaveOutcomeLikeCpp;
use crate::session::WorldSession;

impl WorldportHostLikeCpp for WorldSession {
    fn resume_deferred_player_save_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = Option<PlayerSaveOutcomeLikeCpp>> + Send {
        wow_world_application::resume_deferred_player_save_with_generator_like_cpp(
            self,
            item_guid_generator,
        )
    }
}

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter of the lifecycle orchestration host (#1363 slice 13b): the
//! buyback and account-collection saves borrow the `LifecycleCx` owners.

use wow_world_application::{
    PlayerLogoutHookEventLikeCpp, SessionAccountSaveCxLikeCpp, SessionLifecycleHostLikeCpp,
};

use crate::session::WorldSession;

impl SessionLifecycleHostLikeCpp for WorldSession {
    fn session_account_save_cx_like_cpp(&mut self) -> SessionAccountSaveCxLikeCpp<'_> {
        let crate::session::LifecycleCx {
            lifecycle,
            inventory,
            hub,
            ..
        } = crate::session::cx_lifecycle(self);
        SessionAccountSaveCxLikeCpp {
            lifecycle,
            inventory,
            hub,
        }
    }

    /// The `player.logout` hook port, dispatched by the World module adapter.
    fn player_logout_hook_like_cpp(&mut self, event: &PlayerLogoutHookEventLikeCpp) {
        self.module_hooks.player_logout_like_cpp(event);
    }

    fn player_logout_hook_wanted_like_cpp(&self) -> bool {
        self.module_hooks
            .wants_like_cpp(wow_module_api::HookFamily::Player)
    }
}

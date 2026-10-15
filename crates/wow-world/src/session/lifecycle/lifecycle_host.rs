// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter of the lifecycle orchestration host (#1363 slice 13b): the
//! buyback and account-collection saves borrow the `LifecycleCx` owners.

use wow_world_application::{SessionAccountSaveCxLikeCpp, SessionLifecycleHostLikeCpp};

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
}

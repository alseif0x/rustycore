// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter for the pet loading, mode-state and resummon bodies moved to
//! `wow_world_lifecycle::pet_session` (#1363 slice 7a). Every method is a pure
//! forward to the World `PetsCx` split or the same-named inherent method.

use wow_world_core::session::HubMut;
use wow_world_lifecycle::SessionLifecycleState;

use crate::session::WorldSession;

impl wow_world_lifecycle::PetSessionHostLikeCpp for WorldSession {
    fn split_pets_mut(&mut self) -> (&mut SessionLifecycleState, HubMut<'_>) {
        let crate::session::PetsCx { lifecycle, hub } = crate::session::cx_pets(self);
        (lifecycle, hub)
    }
    fn resolved_represented_at_login_flags_like_cpp(&self) -> Option<u16> {
        WorldSession::resolved_represented_at_login_flags_like_cpp(self)
    }
}

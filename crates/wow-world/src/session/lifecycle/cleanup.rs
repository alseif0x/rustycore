// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Disconnect and logout cleanup, in the order it must happen.
//!
//! Publication is torn down before ownership: the session leaves the player
//! directory, tells nearby players their view changed, leaves the canonical
//! map and the object accessor, releases its character login claim and only
//! then drops inventory objects. Reordering these is observable — a session
//! still published while its map entry is gone is exactly the window that
//! produces stale broadcasts. The ordered body moved to
//! `wow_world_application::session_lifecycle` (#1363 slice 13b).

use super::super::WorldSession;
use crate::finalization::FinalizationOutcome;

impl WorldSession {
    pub fn cleanup_shared_runtime_state(&mut self) -> FinalizationOutcome {
        wow_world_application::cleanup_shared_runtime_state(self)
    }
}

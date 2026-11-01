// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only accessors for the ordered Session driver phase trace; the phase enum lives in
//! `session::state::driver_phase`.

pub(crate) use crate::session::state::SessionDriverPhaseLikeCpp;

impl super::super::WorldSession {
    /// The phases recorded since the last reset, in execution order.
    #[cfg(test)]
    pub(crate) fn driver_phase_trace_like_cpp(&self) -> &[SessionDriverPhaseLikeCpp] {
        &self.core.driver_phase_trace_like_cpp
    }

    /// Clear the recorded trace so one test can assert per pass.
    #[cfg(test)]
    pub(crate) fn reset_driver_phase_trace_like_cpp(&mut self) {
        self.core.driver_phase_trace_like_cpp.clear();
    }
}

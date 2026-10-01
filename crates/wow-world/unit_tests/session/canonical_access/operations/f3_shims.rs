// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn mutate_player_unit_presentation_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        self.core.mutate_player_unit_presentation_like_cpp(mutate)
    }
}

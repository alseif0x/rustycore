// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player presentation: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::Player;
#[cfg(any(test, feature = "test-fixtures"))]
use super::RepresentedForceDeselectLikeCpp;
use super::WorldSession;
#[cfg(test)]
use super::debug;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_force_deselects_like_cpp(
        &self,
    ) -> &[RepresentedForceDeselectLikeCpp] {
        self.social.represented_force_deselects_for_test_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/player_presentation/f3_shims.rs"]
mod f3_shims;

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_cuf_profiles_like_cpp(
        &self,
    ) -> &[Option<wow_packet::packets::misc::CufProfile>] {
        self.fixtures
            .presentation
            .represented_cuf_profiles_like_cpp()
    }
}

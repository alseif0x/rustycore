// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn replace_owned_player_mails_like_cpp(
        &self,
        mails: Vec<wow_entities::PlayerMailRecord>,
    ) -> bool {
        crate::session::hub_ref(self).replace_owned_player_mails_like_cpp(mails)
    }
}

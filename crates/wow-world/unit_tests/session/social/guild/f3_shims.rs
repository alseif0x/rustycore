// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_guild_accept_invites_like_cpp(&self) -> &[u64] {
        self.social.represented_guild_accept_invites_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_auto_decline_guild_invites_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_social_ref(self);
        state.represented_auto_decline_guild_invites_like_cpp(hub)
    }
}

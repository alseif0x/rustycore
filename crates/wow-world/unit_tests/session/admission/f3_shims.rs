// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::session) fn kick_packet_spoof_affected_sessions_like_cpp(
        &self,
        affected_account_ids: &[u32],
    ) -> usize {
        self.core
            .kick_packet_spoof_affected_sessions_like_cpp(affected_account_ids)
    }
}

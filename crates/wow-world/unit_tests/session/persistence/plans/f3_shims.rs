// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) async fn persist_player_gold_checked_like_cpp(
        &self,
        money: u64,
    ) -> Result<(), LootMoneyPersistenceErrorLikeCpp> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.persist_player_gold_checked_like_cpp(hub, money).await
    }
}

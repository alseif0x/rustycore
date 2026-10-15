// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the group command receivers, reconciliation and
//! publication tail that moved to `wow_world_instances::group_session` (#1363
//! slice 4d). Each method is the same-named session operation, field or split.

use wow_social::group::GroupDifficultyKindLikeCpp;
use wow_world_core::session::{HubMut, HubRef};
use wow_world_instances::GroupSessionHostLikeCpp;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_social::SessionSocialLimits;

use crate::session::WorldSession;

impl GroupSessionHostLikeCpp for WorldSession {
    fn split_social_ref(&self) -> (&SessionSocialLimits, HubRef<'_>) {
        crate::session::split_social_ref(self)
    }
    fn split_social_mut(&mut self) -> (&mut SessionSocialLimits, HubMut<'_>) {
        crate::session::split_social_mut(self)
    }
    fn lifecycle(&self) -> &SessionLifecycleState {
        &self.lifecycle
    }
    fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize {
        WorldSession::update_visible_gameobjects_or_spell_clicks_like_cpp(self)
    }
    fn apply_group_difficulty_like_cpp(
        &mut self,
        group_guid: u64,
        difficulty_id: u32,
        kind: GroupDifficultyKindLikeCpp,
    ) {
        WorldSession::apply_group_difficulty_like_cpp(self, group_guid, difficulty_id, kind)
    }
}

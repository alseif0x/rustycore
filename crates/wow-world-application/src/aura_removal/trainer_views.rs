// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;
use wow_world_core::session::{AuraNpcAccessBuilderLikeCpp, NpcInteractionAccessLikeCpp};

impl AuraRemovalCxLikeCpp<'_> {
    pub(crate) fn trainer_npc_view_like_cpp<'b>(
        &'b self,
        inputs: &'b AuraNpcAccessBuilderLikeCpp<'_>,
    ) -> NpcInteractionAccessLikeCpp<'b> {
        inputs.reborrow_like_cpp(&self.stats)
    }

    pub(crate) fn trainer_offer_views_like_cpp<'b>(
        &'b self,
        npc: &'b AuraNpcAccessBuilderLikeCpp<'_>,
        conditions: &'b crate::PlayerConditionProjectionInputsLikeCpp<'_>,
    ) -> (
        &'b wow_world_spell::SessionSpellState,
        NpcInteractionAccessLikeCpp<'b>,
        crate::PlayerConditionProjectionCxLikeCpp<'b>,
    ) {
        (
            &*self.spell,
            npc.reborrow_like_cpp(&self.stats),
            conditions.reborrow_like_cpp(
                &self.stats,
                &self.player,
                &*self.inventory,
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.spell,
            ),
        )
    }
}

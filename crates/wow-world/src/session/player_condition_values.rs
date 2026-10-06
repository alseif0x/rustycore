// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player condition values: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;
use wow_data::PlayerConditionContextLikeCpp;

impl WorldSession {
    /// Select the condition-projection participants without reading them.
    /// Callers that own a later evaluation point can pass this inert view
    /// across the boundary and project it there.
    pub(crate) fn player_condition_projection_cx_like_cpp(
        &self,
    ) -> wow_world_application::PlayerConditionProjectionCxLikeCpp<'_> {
        wow_world_application::player_condition_projection_cx_like_cpp(
            super::hub_ref(self),
            &self.inventory,
            &self.social,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.quest_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.instances,
            #[cfg(any(test, feature = "test-fixtures"))]
            cfg!(test),
        )
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RepresentedPlayerConditionContextLikeCpp {
    projected: wow_world_application::RepresentedPlayerConditionContextLikeCpp,
}

/// Represented subset of C++ `Player::m_unitData` item-level cap fields
/// consumed by `Item::GetItemLevel(Player const*)`.
pub(crate) type RepresentedItemLevelCapsLikeCpp = wow_entities::PlayerItemLevelCapsLikeCpp;

impl RepresentedPlayerConditionContextLikeCpp {
    pub(crate) fn as_context<'a>(
        &'a self,
        session: &'a WorldSession,
    ) -> Option<PlayerConditionContextLikeCpp<'a>> {
        session
            .player_condition_projection_cx_like_cpp()
            .condition_context_like_cpp(&self.projected)
    }
}

impl WorldSession {
    pub(crate) fn represented_player_condition_context_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerConditionContextLikeCpp> {
        Some(RepresentedPlayerConditionContextLikeCpp {
            projected: self
                .player_condition_projection_cx_like_cpp()
                .project_like_cpp()?,
        })
    }

    pub(crate) fn represented_meets_player_condition_id_like_cpp(
        &self,
        player_condition_id: u32,
    ) -> bool {
        wow_world_application::meets_player_condition_id_like_cpp(
            &self.player_condition_projection_cx_like_cpp(),
            self.catalogs.player_condition_store.as_deref(),
            player_condition_id,
        )
    }
}

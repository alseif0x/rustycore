// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Stand state adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{RepresentedGameObjectUseEffect, WorldSession};
#[cfg(test)]
use super::{TitanGripPenaltyAction, UnitStandStateType};

#[cfg(test)]
pub(crate) use wow_world_core::session::{
    RepresentedLiveApplicationLikeCpp, RepresentedLiveIntentAppliedLikeCpp,
    RepresentedLiveIntentApplyOutcomeLikeCpp, RepresentedLiveIntentLikeCpp,
    RepresentedStandChannelCancellationBoundary, RepresentedStandStateChangedLikeCpp,
};

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_titan_grip_penalty_actions_like_cpp(
        &self,
    ) -> &[TitanGripPenaltyAction] {
        self.inventory
            .represented_titan_grip_penalty_actions_for_test_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/stand_state_adapter/f3_shims.rs"]
mod f3_shims;

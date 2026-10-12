// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battleground adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::RepresentedWargameInviteAcceptanceLikeCpp;

#[cfg(test)]
pub(crate) use wow_world_core::session::RepresentedBattlegroundQueueTypeIdLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_core::session::{
    RepresentedBattlefieldListLikeCpp, RepresentedBattlefieldPortLikeCpp,
    RepresentedBattlemasterHelloLikeCpp, RepresentedBattlemasterJoinArenaLikeCpp,
    RepresentedBattlemasterJoinLikeCpp, RepresentedBattlemasterJoinSkirmishLikeCpp,
};

#[cfg(test)]
#[path = "../../unit_tests/session/battleground_adapter/f3_shims.rs"]
mod f3_shims;

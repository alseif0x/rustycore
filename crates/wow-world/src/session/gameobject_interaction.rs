// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Gameobject interaction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
pub(crate) use wow_world_entities::{
    BattlegroundFlagDropClickTarget, RepresentedBattlegroundObjectUseRejection,
    RepresentedCapturePointStateLikeCpp, RepresentedGameObjectCriteriaEvent,
};
pub(crate) use wow_world_entities::{
    RepresentedGameObjectUseEffect, RepresentedGameObjectUseState, RepresentedNewFlagStateRequest,
};

#[cfg(test)]
#[path = "../../unit_tests/session/gameobject_interaction/f3_shims.rs"]
mod f3_shims;

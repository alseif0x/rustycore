// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social requests: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::WorldSession;

#[cfg(test)]
pub(crate) use wow_world_core::session::RepresentedWargameInviteAcceptanceLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_social::RepresentedSilencePartyTalkerLikeCpp;
pub(crate) use wow_world_social::{
    RepresentedCalendarAddEventLikeCpp, RepresentedCalendarCommunityInviteLikeCpp,
    RepresentedCalendarRemoveEventLikeCpp, RepresentedDeclinePetitionLikeCpp,
    RepresentedQueryPetitionLikeCpp, RepresentedSignPetitionLikeCpp,
};

#[cfg(test)]
#[path = "../../unit_tests/session/social_requests/f3_shims.rs"]
mod f3_shims;

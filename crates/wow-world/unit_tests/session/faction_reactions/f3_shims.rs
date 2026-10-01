// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn represented_faction_reaction_to_like_cpp(
        &self,
        input: RepresentedFactionReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        crate::session::hub_ref(self).represented_faction_reaction_to_like_cpp(input)
    }
    pub(crate) fn resolved_watched_faction_index_like_cpp(&self) -> Option<i32> {
        crate::session::hub_ref(self).resolved_watched_faction_index_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn watched_faction_index_like_cpp(&self) -> i32 {
        crate::session::hub_ref(self).watched_faction_index_like_cpp()
    }
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Single-owner claims over one admitted creature transition.

use super::*;

/// Which runtime owns one admitted creature transition.
///
/// C++ has one `ObjectUpdater` writer per `Map::Update`. RustyCore still
/// composes three candidates while the cutover is staged: the canonical
/// admitted execution introduced by F6-8D2, the legacy global runtime that
/// production uses today, and the per-session runtime that
/// `RustyCore.LegacyCreatureGlobalRuntime = 0` keeps alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CreatureExecutionOwnerLikeCpp {
    /// The canonical admitted execution path of this module.
    CanonicalAdmitted,
    /// The global legacy runtime (`RuntimeTickOwner::GlobalLegacy`).
    LegacyGlobal,
    /// The per-session runtime (`RuntimeTickOwner::Session`).
    Session,
}

impl CreatureExecutionOwnerLikeCpp {
    /// Stable label for refusal reporting and assertions.
    #[must_use]
    pub const fn label_like_cpp(self) -> &'static str {
        match self {
            Self::CanonicalAdmitted => "canonical-admitted",
            Self::LegacyGlobal => "legacy-global",
            Self::Session => "session",
        }
    }
}

/// One execution owner's claim over one admitted transition.
///
/// The identity is the tuple the admission already froze: the map coordinator,
/// the tick epoch, the map incarnation and the creature GUID. The same
/// transition can therefore be claimed exactly once even when it is attempted
/// from several owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreatureExecutionTransitionLikeCpp {
    pub coordinator_id: u64,
    pub tick_epoch: u64,
    pub map_id: u32,
    pub instance_id: u32,
    pub incarnation: u64,
    pub creature_guid: ObjectGuid,
}

/// The single-owner claim table of the isolated path.
///
/// Claims are retained only for the newest tick epoch of a coordinator, so the
/// table stays bounded across ticks without weakening the exactly-once rule for
/// the transition being executed.
#[derive(Debug, Default)]
pub struct CreatureExecutionLeaseLikeCpp {
    claims: Vec<(
        CreatureExecutionTransitionLikeCpp,
        CreatureExecutionOwnerLikeCpp,
    )>,
}

/// Shared claim table. The isolated path owns it; production never constructs
/// one.
pub type SharedCreatureExecutionLeaseLikeCpp =
    std::sync::Arc<std::sync::Mutex<CreatureExecutionLeaseLikeCpp>>;

impl CreatureExecutionLeaseLikeCpp {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim `transition` for `owner`, or refuse when another owner holds it.
    ///
    /// A repeated claim by the *same* owner is also refused: the contract is
    /// one *execution* per admitted transition, not one claimer per owner.
    pub fn claim_like_cpp(
        &mut self,
        transition: CreatureExecutionTransitionLikeCpp,
        owner: CreatureExecutionOwnerLikeCpp,
    ) -> bool {
        self.retain_current_epoch_like_cpp(transition);
        if self.claims.iter().any(|(held, _)| *held == transition) {
            return false;
        }
        self.claims.push((transition, owner));
        true
    }

    /// Who holds `transition`, if anyone.
    #[must_use]
    pub fn owner_like_cpp(
        &self,
        transition: CreatureExecutionTransitionLikeCpp,
    ) -> Option<CreatureExecutionOwnerLikeCpp> {
        self.claims
            .iter()
            .find(|(held, _)| *held == transition)
            .map(|(_, owner)| *owner)
    }

    fn retain_current_epoch_like_cpp(&mut self, transition: CreatureExecutionTransitionLikeCpp) {
        let (coordinator_id, tick_epoch) = (transition.coordinator_id, transition.tick_epoch);
        self.claims.retain(|(held, _)| {
            held.coordinator_id != coordinator_id || held.tick_epoch == tick_epoch
        });
    }
}

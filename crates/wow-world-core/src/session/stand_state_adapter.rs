// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::UnitStandStateType;

/// Validated session-owned intent whose side effects cross the represented to
/// live boundary. Variants deliberately own their payload so future intents
/// may contain non-`Copy` data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepresentedLiveIntentLikeCpp {
    StandStateChanged(RepresentedStandStateChangedLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedStandStateChangedLikeCpp {
    pub state: UnitStandStateType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedStandChannelCancellationBoundary {
    Interrupted {
        spell_id: u32,
        canonical_spells_interrupted: usize,
        session_cast_interrupted: bool,
    },
    UnknownInterruptMetadata {
        spell_id: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedLiveIntentAppliedLikeCpp {
    StandStateChanged {
        canonical_field_changed: bool,
        canonical_auras_removed: usize,
        represented_auras_removed: usize,
        channel_cancellation_boundary: Option<RepresentedStandChannelCancellationBoundary>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedLiveIntentApplyOutcomeLikeCpp {
    Applied(RepresentedLiveIntentAppliedLikeCpp),
    RejectedMissingPlayer,
    RejectedMissingCanonicalPlayer,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedLiveApplicationLikeCpp {
    pub intent: RepresentedLiveIntentLikeCpp,
    pub outcome: RepresentedLiveIntentApplyOutcomeLikeCpp,
}

impl crate::session::HubRef<'_> {
    pub fn resolved_player_stand_state_like_cpp(
        &self,
    ) -> Option<UnitStandStateType> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().stand_state_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.presentation.player_stand_state_like_cpp);
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_stand_state_like_cpp(&self) -> UnitStandStateType {
        self.resolved_player_stand_state_like_cpp()
            .expect("test Player stand-state owner must resolve")
    }

    pub fn player_is_sit_state_like_cpp(&self) -> bool {
        self.resolved_player_stand_state_like_cpp()
            .is_some_and(|state| {
                matches!(
                    state,
                    UnitStandStateType::Sit
                        | UnitStandStateType::SitChair
                        | UnitStandStateType::SitLowChair
                        | UnitStandStateType::SitMediumChair
                        | UnitStandStateType::SitHighChair
                )
            })
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PlayerPresentationState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_live_applications_like_cpp(
        &self,
    ) -> &[RepresentedLiveApplicationLikeCpp] {
        &self.represented_live_applications_like_cpp
    }
}

impl crate::session::HubMut<'_> {
    pub fn set_player_stand_state_like_cpp(&mut self, state: UnitStandStateType) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_stand_state_like_cpp(state)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if _canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.presentation.player_stand_state_like_cpp = state;
        }
    }
}

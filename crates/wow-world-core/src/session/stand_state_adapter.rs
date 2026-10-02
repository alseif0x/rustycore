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

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `SessionFixtures::auras` sub-state (#1241 F2): moved fields, no logic.

#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::HashMap;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::{
    AuraApplicationLikeCpp as AuraApplication,
    AuraThreatSnapshotLikeCpp as CanonicalThreatAuraSnapshotLikeCpp,
};

/// Player aura fixtures: visible auras, aura authority, the spell-hit tombstone, threat-aura
/// snapshots and the shapeshift form.
pub struct AuraState {
    /// Represented C++ `Player::GetShapeshiftForm()` until shapeshift aura state owns it.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_shapeshift_form_like_cpp: u32,
    /// Permanent fail-closed marker for this C++ Player lifetime after a login
    /// cast closure that Rust did not retain losslessly (currently FIRST).
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_spell_hit_aura_authority_tombstoned_like_cpp: bool,

    // ── Aura system ───────────────────────────────────────────────
    /// Legacy fixture mirror for tests that construct a Session without a
    /// canonical Player owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub visible_auras: HashMap<u8, AuraApplication>,
    /// True only after both persisted aura tables were read successfully for
    /// the active character. Absence is evidence only while this is complete.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_aura_authority_complete_like_cpp: bool,
    /// Difficulty-selected C++ `AuraEffect` identity captured when the aura is applied.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub canonical_threat_aura_snapshots_like_cpp: HashMap<u8, CanonicalThreatAuraSnapshotLikeCpp>,
}

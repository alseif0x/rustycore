// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::auras` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Player aura fixtures: visible auras, aura authority, the spell-hit tombstone, threat-aura
/// snapshots and the shapeshift form.
pub(crate) struct AuraState {
    /// Represented C++ `Player::GetShapeshiftForm()` until shapeshift aura state owns it.
    #[cfg(test)]
    pub(in crate::session) represented_shapeshift_form_like_cpp: u32,
    /// Permanent fail-closed marker for this C++ Player lifetime after a login
    /// cast closure that Rust did not retain losslessly (currently FIRST).
    #[cfg(test)]
    pub(in crate::session) player_spell_hit_aura_authority_tombstoned_like_cpp: bool,

    // ── Aura system ───────────────────────────────────────────────
    /// Legacy fixture mirror for tests that construct a Session without a
    /// canonical Player owner.
    #[cfg(test)]
    pub(crate) visible_auras: HashMap<u8, AuraApplication>,
    /// True only after both persisted aura tables were read successfully for
    /// the active character. Absence is evidence only while this is complete.
    #[cfg(test)]
    pub(in crate::session) player_aura_authority_complete_like_cpp: bool,
    /// Difficulty-selected C++ `AuraEffect` identity captured when the aura is applied.
    #[cfg(test)]
    pub(in crate::session) canonical_threat_aura_snapshots_like_cpp:
        HashMap<u8, CanonicalThreatAuraSnapshotLikeCpp>,
}

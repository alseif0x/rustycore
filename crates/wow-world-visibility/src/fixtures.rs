//! Detached represented visibility inputs used only by fixtures.

#![cfg(any(test, feature = "test-fixtures"))]

use wow_core::ObjectGuid;
use wow_entities::PhaseShift;

/// Detached equivalents of visibility state whose production authority belongs
/// to the canonical map-owned Player.
pub(crate) struct VisibilityTestFixtureLikeCpp {
    /// Test seam for the C++ `Player::m_seer` projection.
    pub(crate) represented_seer_guid_like_cpp: Option<ObjectGuid>,
    /// C++ `Player::GetPhaseShift()` fixture used before a canonical player handle is installed.
    pub(crate) represented_player_phase_shift: PhaseShift,
}

impl Default for VisibilityTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_seer_guid_like_cpp: None,
            represented_player_phase_shift: PhaseShift::default(),
        }
    }
}

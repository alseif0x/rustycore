//! Test-only represented visibility inputs owned by the Session fixture.

use super::super::{ObjectGuid, PhaseShift};

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

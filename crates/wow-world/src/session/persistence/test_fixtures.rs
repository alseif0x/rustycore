//! Test-only loaded Player flag state owned by the Session fixture.

/// Detached values loaded from the character row before a canonical Player exists.
pub(crate) struct LoadedPlayerFlagsTestFixtureLikeCpp {
    /// C++ `characters.playerFlags` loaded by `Player::LoadFromDB`.
    pub(in crate::session) represented_loaded_player_flags_like_cpp: Option<u32>,
    /// C++ `characters.playerFlagsEx` loaded by `Player::LoadFromDB`.
    pub(in crate::session) represented_loaded_player_flags_ex_like_cpp: Option<u32>,
    /// Tracks whether the detached values were applied to a canonical Player.
    pub(in crate::session) represented_loaded_player_flags_applied_like_cpp: bool,
}

impl Default for LoadedPlayerFlagsTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_loaded_player_flags_like_cpp: None,
            represented_loaded_player_flags_ex_like_cpp: None,
            represented_loaded_player_flags_applied_like_cpp: false,
        }
    }
}

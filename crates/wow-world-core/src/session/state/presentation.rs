// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::{UnitFlags, UnitStandStateType};

use crate::session::{
    RepresentedAlterAppearanceLikeCpp, RepresentedConfirmBarbersChoiceLikeCpp,
    RepresentedLiveApplicationLikeCpp,
};

/// Player presentation fixtures: unit flags and scale, stand state and emote, action bars,
/// cinematics, CUF profiles and barber requests.
pub struct PlayerPresentationState {
    /// Handle-less unit-test fallback; production C++ `Player::_CUFProfiles` lives on canonical
    /// `wow_entities::Player`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub cuf_profiles_like_cpp: Vec<Option<wow_packet::packets::misc::CufProfile>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub cuf_profiles_loaded_like_cpp: bool,
    /// Represented stand state used by movement side effects until UnitData owns it.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_stand_state_like_cpp: UnitStandStateType,
    /// Test-only successful represented->live evidence. Production emits
    /// bounded structured telemetry instead of retaining client-controlled
    /// history for the lifetime of the session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_live_applications_like_cpp: Vec<RepresentedLiveApplicationLikeCpp>,
    /// Represented `UnitData::EmoteState`, used to clear stateful emotes on movement like C++.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_emote_state_like_cpp: u32,
    /// Represented `ActivePlayerData::LocalFlags`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub active_player_local_flags_like_cpp: u32,
    /// Represented `ActivePlayerData::TransportServerTime`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub active_player_transport_server_time_like_cpp: i32,
    /// Represented `ActivePlayerData::MultiActionBars`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub active_player_multi_action_bars_like_cpp: u8,
    /// Test-only action-button owner for fixtures without a canonical Player.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_action_buttons_like_cpp: [u32; wow_packet::packets::misc::MAX_ACTION_BUTTONS],
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_action_buttons_loaded_like_cpp: bool,
    /// Represented accepted barber-shop requests until ChrCustomization DB2/cost/update runtime is canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_alter_appearance_requests_like_cpp: Vec<RepresentedAlterAppearanceLikeCpp>,
    /// Represented accepted barber confirmation requests until Player::SetCustomizations is canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_confirm_barbers_choice_requests_like_cpp:
        Vec<RepresentedConfirmBarbersChoiceLikeCpp>,
    /// Handle-less fixture for C++ `Object::GetObjectScale()`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_object_scale_like_cpp: f32,
    /// Handle-less fixture for C++ `UnitData::Flags`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_unit_flags_like_cpp: UnitFlags,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_cinematic_state_like_cpp: wow_entities::PlayerCinematicStateLikeCpp,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_cinematic_next_camera_events_like_cpp: Vec<u16>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_cinematic_end_events_like_cpp: Vec<u32>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_movie_complete_events_like_cpp: Vec<u32>,
}

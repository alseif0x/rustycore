// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The per-family login loaders the World continue-login coordinator calls in
//! C++ `Player::LoadFromDB` order.

mod action_buttons;
mod admission;
mod aura_loading;
mod cuf_profiles;
mod currency_loading;
mod default_skills;
mod glyph_loading;
mod group_loading;
mod reputation_loading;
mod skill_loading;
mod spell_loading;
mod spell_map_finalization;
mod talent_loading;
mod transport_restore;

pub use action_buttons::load_action_buttons_for_login_like_cpp;
pub use admission::{LoginAdmissionDataLikeCpp, load_login_admission_data_like_cpp};
pub use aura_loading::load_character_auras_for_login_like_cpp;
pub use cuf_profiles::load_cuf_profiles_for_login_like_cpp;
pub use currency_loading::load_character_currencies_for_login_like_cpp;
pub use default_skills::apply_default_skills_for_login_like_cpp;
pub use glyph_loading::load_character_glyphs_for_login_like_cpp;
pub use group_loading::load_group_membership_for_login_like_cpp;
pub use reputation_loading::load_character_reputation_for_login_like_cpp;
pub use skill_loading::load_character_skill_rows_for_login_like_cpp;
pub use spell_loading::{
    LoginSpellRowsLikeCpp, load_character_favorite_spells_for_login_like_cpp,
    load_character_spell_rows_for_login_like_cpp,
};
pub use spell_map_finalization::finalize_player_spell_map_for_login_like_cpp;
pub use talent_loading::load_character_talents_for_login_like_cpp;
pub use transport_restore::restore_persisted_transport_for_login_like_cpp;

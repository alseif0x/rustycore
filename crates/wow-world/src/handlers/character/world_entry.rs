// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login, world entry, logout and the client-state handshake. The world
//! entry, logout, login packet bursts, login loaders and login recovery moved
//! to `wow-world-character` (#1363 slice 8d); the continue-login coordinator
//! stays here.

use super::*;
// The moved login loaders and world-entry operations the coordinator calls;
// `login` reaches them through `use super::*`.
use wow_world_character::{
    LoginAdmissionDataLikeCpp, LoginSpellRowsLikeCpp, abort_partial_login_sequence_like_cpp,
    apply_default_skills_for_login_like_cpp, delete_invalid_character_homebind_like_cpp,
    finalize_player_spell_map_for_login_like_cpp, load_action_buttons_for_login_like_cpp,
    load_character_auras_for_login_like_cpp, load_character_currencies_for_login_like_cpp,
    load_character_favorite_spells_for_login_like_cpp, load_character_glyphs_for_login_like_cpp,
    load_character_reputation_for_login_like_cpp, load_character_skill_rows_for_login_like_cpp,
    load_character_spell_history_packets_like_cpp, load_character_spell_rows_for_login_like_cpp,
    load_character_talents_for_login_like_cpp, load_cuf_profiles_for_login_like_cpp,
    load_group_membership_for_login_like_cpp, load_inventory_for_login_like_cpp,
    load_login_admission_data_like_cpp, player_login_combat_stats_like_cpp,
    repair_character_homebind_like_cpp, restore_persisted_transport_for_login_like_cpp,
    retry_login_at_homebind_like_cpp, seed_login_location_zone_area_like_cpp, send_login_sequence,
    skill_rewarded_spell_changes_for_login_like_cpp,
};

mod login;

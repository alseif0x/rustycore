//! Feature-gated forwards to the real Character operation; no replacement logic.

use super::*;

pub fn start_position_for_test(
    race: u8,
) -> (i32, f32, f32, f32, f32) {
    crate::handlers::character::creation_support::start_position(race)
}

pub fn default_display_id_for_test(
    race: u8,
    sex: u8,
) -> u32 {
    crate::handlers::character::creation_support::default_display_id(race, sex)
}

pub fn start_zone_for_test(
    race: u8,
) -> i32 {
    crate::handlers::character::creation_support::start_zone(race)
}

pub fn restored_saved_health_for_test(
    saved_health: Option<u32>,
    max_health: i64,
) -> i64 {
    crate::handlers::character::creation_support::restored_saved_health_like_cpp(saved_health, max_health)
}

pub fn default_character_power1_for_test(
    class: u8,
    mana: u32,
) -> u32 {
    crate::handlers::character::creation_support::default_character_power1_like_cpp(class, mana)
}

pub fn motd_lines_for_test(
    motd: &str,
) -> Vec<String> {
    crate::handlers::character::login_context::motd_lines_like_cpp(motd)
}

pub fn initial_character_rest_state_for_test(
    is_a_recruiter: bool,
    recruiter_id: u32,
) -> u8 {
    crate::handlers::character::login_context::initial_character_rest_state_like_cpp(is_a_recruiter, recruiter_id)
}

//! Character login context: MOTD lines, void-storage context and the initial rest state.
//!
//! Split out of `character/mod.rs` under #584 (B5); items are unchanged.

use super::*;

pub(in crate::handlers::character) fn motd_lines_like_cpp(motd: &str) -> Vec<String> {
    // C++ `World::SetMotd` uses `boost::split` on `@` with token compression
    // disabled, so empty and trailing lines remain part of the login burst.
    motd.split('@').map(ToOwned::to_owned).collect()
}

pub(in crate::handlers::character) fn void_storage_login_context_like_cpp(
    random_properties_id: i32,
    _selected_context_column: u8,
) -> u8 {
    // Audited 3.4.3 `Player::_LoadVoidStorage` constructs ItemContext from
    // fields[5] even though CHAR_SEL_CHAR_VOID_STORAGE selects `context` as
    // fields[7]. Keep that executable C++ behavior; the unused argument makes
    // the query/implementation mismatch explicit instead of hiding column 7.
    random_properties_id as u8
}

pub(in crate::handlers::character) fn initial_character_rest_state_like_cpp(is_a_recruiter: bool, recruiter_id: u32) -> u8 {
    if is_a_recruiter || recruiter_id != 0 {
        REST_STATE_RAF_LINKED_LIKE_CPP
    } else {
        REST_STATE_NORMAL_LIKE_CPP
    }
}

#[cfg(test)]
#[path = "login_context_tests.rs"]
mod rule_tests;

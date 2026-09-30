//! Misc scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;
use wow_constants::character::{
    CHAR_NAME_INVALID_CHARACTER_LIKE_CPP, CHAR_NAME_NO_NAME_LIKE_CPP,
    CHAR_NAME_TOO_LONG_LIKE_CPP, CHAR_NAME_TOO_SHORT_LIKE_CPP, RESPONSE_SUCCESS_LIKE_CPP,
};
use wow_constants::rest::{REST_STATE_NORMAL_LIKE_CPP, REST_STATE_RAF_LINKED_LIKE_CPP};
use wow_data::{PlayerCreateInfoLikeCpp, PlayerCreatePositionLikeCpp};

#[test]
fn character_rename_name_validation_matches_represented_cpp_gates() {
    assert_eq!(
        wow_entities::represented_character_rename_name_result_like_cpp(""),
        CHAR_NAME_NO_NAME_LIKE_CPP
    );
    assert_eq!(
        wow_entities::represented_character_rename_name_result_like_cpp("A"),
        CHAR_NAME_TOO_SHORT_LIKE_CPP
    );
    assert_eq!(
        wow_entities::represented_character_rename_name_result_like_cpp("VeryLongNameX"),
        CHAR_NAME_TOO_LONG_LIKE_CPP
    );
    assert_eq!(
        wow_entities::represented_character_rename_name_result_like_cpp("Bad1"),
        CHAR_NAME_INVALID_CHARACTER_LIKE_CPP
    );
    assert_eq!(
        wow_entities::represented_character_rename_name_result_like_cpp("Newname"),
        RESPONSE_SUCCESS_LIKE_CPP
    );
}

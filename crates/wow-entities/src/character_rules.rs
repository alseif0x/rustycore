// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Receiver-free character admission rules.

use wow_constants::character::{
    CHAR_NAME_INVALID_CHARACTER_LIKE_CPP, CHAR_NAME_NO_NAME_LIKE_CPP, CHAR_NAME_TOO_LONG_LIKE_CPP,
    CHAR_NAME_TOO_SHORT_LIKE_CPP, RESPONSE_SUCCESS_LIKE_CPP,
};

pub fn represented_character_rename_name_result_like_cpp(name: &str) -> u8 {
    if name.is_empty() {
        return CHAR_NAME_NO_NAME_LIKE_CPP;
    }
    if name.len() < 2 {
        return CHAR_NAME_TOO_SHORT_LIKE_CPP;
    }
    if name.len() > 12 {
        return CHAR_NAME_TOO_LONG_LIKE_CPP;
    }
    if !name.chars().all(|c| c.is_ascii_alphabetic()) {
        return CHAR_NAME_INVALID_CHARACTER_LIKE_CPP;
    }

    RESPONSE_SUCCESS_LIKE_CPP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_name_admission_preserves_cpp_response_order() {
        assert_eq!(
            represented_character_rename_name_result_like_cpp(""),
            CHAR_NAME_NO_NAME_LIKE_CPP
        );
        assert_eq!(
            represented_character_rename_name_result_like_cpp("A"),
            CHAR_NAME_TOO_SHORT_LIKE_CPP
        );
        assert_eq!(
            represented_character_rename_name_result_like_cpp("VeryLongNameX"),
            CHAR_NAME_TOO_LONG_LIKE_CPP
        );
        assert_eq!(
            represented_character_rename_name_result_like_cpp("Bad1"),
            CHAR_NAME_INVALID_CHARACTER_LIKE_CPP
        );
        assert_eq!(
            represented_character_rename_name_result_like_cpp("Newname"),
            RESPONSE_SUCCESS_LIKE_CPP
        );
    }
}

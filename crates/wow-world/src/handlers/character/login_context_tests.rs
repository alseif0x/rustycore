use super::*;

#[test]
fn create_character_seeds_valid_cpp_rest_state_for_raf_roles() {
    assert_eq!(
        initial_character_rest_state_like_cpp(false, 0),
        REST_STATE_NORMAL_LIKE_CPP
    );
    assert_eq!(
        initial_character_rest_state_like_cpp(false, 7),
        REST_STATE_RAF_LINKED_LIKE_CPP
    );
    assert_eq!(
        initial_character_rest_state_like_cpp(true, 0),
        REST_STATE_RAF_LINKED_LIKE_CPP
    );
}

#[test]
fn motd_split_preserves_cpp_empty_and_trailing_lines() {
    assert_eq!(
        motd_lines_like_cpp("first@@third@"),
        vec!["first", "", "third", ""]
    );
}

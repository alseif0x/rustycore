#[test]
fn continue_login_has_no_concrete_persistence_after_remaining_writes_move() {
    let handler = concat!(
        include_str!("../src/handlers/character/world_entry/login.rs"),
        include_str!("../src/handlers/character/world_entry/login/action_buttons.rs"),
        include_str!("../src/handlers/character/world_entry/login/aura_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/cuf_profiles.rs"),
        include_str!("../src/handlers/character/world_entry/login/currency_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/default_skills.rs"),
        include_str!("../src/handlers/character/world_entry/login/glyph_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/group_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/mail_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/pet_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/reputation_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/skill_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/spell_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/spell_map_finalization.rs"),
        include_str!("../src/handlers/character/world_entry/login/talent_loading.rs"),
        include_str!("../src/handlers/character/world_entry/login/transport_restore.rs"),
    );

    assert!(handler.contains("reset_login_pet_talents_like_cpp"));
    assert!(handler.contains("mark_player_online_like_cpp"));
    for concrete in [
        "char_db()",
        "CharStatements::",
        "SqlTransaction::",
        ".prepare(",
        ".execute(",
        ".commit_transaction(",
    ] {
        assert!(
            !handler.contains(concrete),
            "continue-login still contains concrete persistence: {concrete}"
        );
    }
}

//! Group scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn continue_login_no_longer_names_location_or_guild_statements() {
    let source = include_str!("../character/world_entry.rs");
    let (_, tail) = source
        .split_once("pub async fn handle_continue_player_login")
        .expect("continue-login handler starts");
    let (handler, _) = tail
        .split_once("pub(super) fn player_login_combat_stats_like_cpp")
        .expect("continue-login handler ends before packet helper");
    assert!(handler.contains("load_login_admission_like_cpp"));
    assert!(handler.contains("PlayerLoginAdmissionLoadedLikeCpp::BattlegroundLocation"));
    assert!(handler.contains("PlayerLoginAdmissionLoadedLikeCpp::HomebindLocation"));
    assert!(handler.contains("PlayerLoginAdmissionLoadedLikeCpp::GuildMembership"));
    for statement in [
        "CharStatements::SEL_CHARACTER_BGDATA",
        "CharStatements::SEL_CHARACTER_HOMEBIND",
        "CharStatements::SEL_GUILD_MEMBER",
    ] {
        assert!(
            !handler.contains(statement),
            "handler still names {statement}"
        );
    }
}
#[tokio::test]
async fn show_trade_skill_is_noop_null_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session.handle_show_trade_skill().await;

    assert!(send_rx.try_recv().is_err());
}

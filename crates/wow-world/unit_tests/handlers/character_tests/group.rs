//! Group scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn continue_login_no_longer_names_location_or_guild_statements() {
    let handler = concat!(
        include_str!("../../../src/handlers/character/world_entry/login.rs"),
        include_str!("../../../src/handlers/character/world_entry/login/admission.rs"),
    );
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
/// 3.4.3 leaves `CMSG_SHOW_TRADE_SKILL` `STATUS_UNHANDLED` / `Handle_NULL`
/// (`Opcodes.cpp:925`), so the 2026-10-07 #1263 F6 decision (D4,
/// `docs/migration/EXISTING-CODE-DEFECTS.md`) removes its registration instead of
/// keeping a logging-only handler the dispatch table reports as handled.
#[test]
fn show_trade_skill_is_not_registered_like_cpp() {
    assert!(
        !crate::session::registry::contains_handler(wow_constants::ClientOpcodes::ShowTradeSkill),
        "ShowTradeSkill is STATUS_UNHANDLED/Handle_NULL in 3.4.3 and must not be registered"
    );
}

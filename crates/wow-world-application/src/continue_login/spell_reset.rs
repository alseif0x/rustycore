// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented login spell reset (`AT_LOGIN_RESET_SPELLS`) and its
//! notification text (#1363 slice 12b4). Moved verbatim from the World
//! `session/spell_state/{effects,spell}.rs`; it re-teaches the quest-rewarded
//! spells, so it sits above spell and quest.

use wow_world_quest::QuestSessionHostLikeCpp;
use wow_world_spell::{AuraHostLikeCpp, TalentHostLikeCpp};

const LANG_RESET_SPELLS_LIKE_CPP: u32 = 215;
const LANG_RESET_SPELLS_TEXT_LIKE_CPP: &str = "Your spells have been reset.";

pub fn apply_represented_login_spell_reset_if_needed_like_cpp<
    H: TalentHostLikeCpp + QuestSessionHostLikeCpp + AuraHostLikeCpp,
>(
    host: &mut H,
) -> bool {
    const AT_LOGIN_RESET_SPELLS_LIKE_CPP: u16 = 0x002;

    if !host
        .resolved_represented_at_login_flags_like_cpp()
        .is_some_and(|flags| (flags & AT_LOGIN_RESET_SPELLS_LIKE_CPP) != 0)
    {
        return false;
    }

    host.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_SPELLS_LIKE_CPP, true);
    let spells = wow_world_spell::known_spells_like_cpp(host).to_vec();
    for spell_id in spells {
        wow_world_spell::remove_known_spell_like_cpp(host, spell_id);
    }
    wow_world_quest::apply_represented_quest_rewarded_spells_like_cpp(host);
    host.core()
        .send_notification_like_cpp(reset_spells_notification_text_like_cpp(host));
    true
}

fn reset_spells_notification_text_like_cpp<H: TalentHostLikeCpp>(host: &H) -> String {
    let text = host.trinity_string_like_cpp(LANG_RESET_SPELLS_LIKE_CPP);
    if text == "<error>" {
        LANG_RESET_SPELLS_TEXT_LIKE_CPP.to_string()
    } else {
        text.to_string()
    }
}

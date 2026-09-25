// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-enumeration projection support.

use super::{
    AT_LOGIN_CHANGE_FACTION_LIKE_CPP, AT_LOGIN_CHANGE_RACE_LIKE_CPP, AT_LOGIN_CUSTOMIZE_LIKE_CPP,
    AT_LOGIN_FIRST_LIKE_CPP, AT_LOGIN_RENAME_LIKE_CPP, AT_LOGIN_RESURRECT_LIKE_CPP,
    CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP, CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP,
    CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP, CHARACTER_FLAG_DECLINED_LIKE_CPP,
    CHARACTER_FLAG_GHOST_LIKE_CPP, CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP,
    CHARACTER_FLAG_RENAME_LIKE_CPP, CLASS_DEATH_KNIGHT_LIKE_CPP, CLASS_HUNTER_LIKE_CPP,
    CLASS_WARLOCK_LIKE_CPP, PLAYER_FLAGS_GHOST_LIKE_CPP,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EnumCharacterFlagsLikeCpp {
    pub(super) flags: u32,
    pub(super) flags2: u32,
    pub(super) first_login: bool,
}

pub(super) fn enum_character_effective_player_flags_like_cpp(
    player_flags: u32,
    at_login_flags: u16,
) -> u32 {
    if (at_login_flags & AT_LOGIN_RESURRECT_LIKE_CPP) != 0 {
        player_flags & !PLAYER_FLAGS_GHOST_LIKE_CPP
    } else {
        player_flags
    }
}

/// C++ anchors:
/// - `Server/Packets/CharacterPackets.cpp:118-145`
/// - `Miscellaneous/SharedDefines.h:1019-1061`
pub(super) fn enum_character_flags_like_cpp(
    player_flags: u32,
    at_login_flags: u16,
    banned_guid: u64,
    declined_genitive: Option<&str>,
    declined_names_used: bool,
) -> EnumCharacterFlagsLikeCpp {
    let player_flags = enum_character_effective_player_flags_like_cpp(player_flags, at_login_flags);
    let mut flags = 0;

    if (player_flags & PLAYER_FLAGS_GHOST_LIKE_CPP) != 0 {
        flags |= CHARACTER_FLAG_GHOST_LIKE_CPP;
    }
    if (at_login_flags & AT_LOGIN_RENAME_LIKE_CPP) != 0 {
        flags |= CHARACTER_FLAG_RENAME_LIKE_CPP;
    }
    if banned_guid != 0 {
        flags |= CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP;
    }
    if declined_names_used && declined_genitive.is_some_and(|name| !name.is_empty()) {
        flags |= CHARACTER_FLAG_DECLINED_LIKE_CPP;
    }

    let flags2 = if (at_login_flags & AT_LOGIN_CUSTOMIZE_LIKE_CPP) != 0 {
        CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP
    } else if (at_login_flags & AT_LOGIN_CHANGE_FACTION_LIKE_CPP) != 0 {
        CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP
    } else if (at_login_flags & AT_LOGIN_CHANGE_RACE_LIKE_CPP) != 0 {
        CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP
    } else {
        0
    };

    EnumCharacterFlagsLikeCpp {
        flags,
        flags2,
        first_login: (at_login_flags & AT_LOGIN_FIRST_LIKE_CPP) != 0,
    }
}

/// C++ anchor: `Server/Packets/CharacterPackets.cpp:147-156`.
pub(super) fn enum_character_pet_data_like_cpp(
    player_flags: u32,
    at_login_flags: u16,
    class_id: u8,
    pet_entry: u32,
    pet_display_id: u32,
    pet_level: u32,
    creature_templates: Option<&wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
) -> (u32, u32, u32) {
    let player_flags = enum_character_effective_player_flags_like_cpp(player_flags, at_login_flags);
    if (player_flags & PLAYER_FLAGS_GHOST_LIKE_CPP) != 0
        || !matches!(
            class_id,
            CLASS_WARLOCK_LIKE_CPP | CLASS_HUNTER_LIKE_CPP | CLASS_DEATH_KNIGHT_LIKE_CPP
        )
    {
        return (0, 0, 0);
    }

    creature_templates
        .and_then(|store| store.get(pet_entry))
        .map(|template| (pet_display_id, pet_level, template.family))
        .unwrap_or((0, 0, 0))
}

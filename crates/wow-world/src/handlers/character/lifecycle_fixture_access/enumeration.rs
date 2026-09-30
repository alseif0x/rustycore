//! Feature-gated forwards to the real Character operation; no replacement logic.

use super::*;

use crate::handlers::character::enumeration_support as original;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumCharacterFlagsForTest {
    pub flags: u32,
    pub flags2: u32,
    pub first_login: bool,
}

impl From<EnumCharacterFlagsForTest> for original::EnumCharacterFlagsLikeCpp {
    fn from(value: EnumCharacterFlagsForTest) -> Self {
        Self {
            flags: value.flags,
            flags2: value.flags2,
            first_login: value.first_login,
        }
    }
}

impl From<original::EnumCharacterFlagsLikeCpp> for EnumCharacterFlagsForTest {
    fn from(value: original::EnumCharacterFlagsLikeCpp) -> Self {
        Self {
            flags: value.flags,
            flags2: value.flags2,
            first_login: value.first_login,
        }
    }
}

pub fn enum_character_flags_for_test(
    player_flags: u32,
    at_login_flags: u16,
    banned_guid: u64,
    declined_genitive: Option<&str>,
    declined_names_used: bool,
) -> EnumCharacterFlagsForTest {
    crate::handlers::character::enumeration_support::enum_character_flags_like_cpp(player_flags, at_login_flags, banned_guid, declined_genitive, declined_names_used).into()
}

pub fn enum_character_pet_data_for_test(
    player_flags: u32,
    at_login_flags: u16,
    class_id: u8,
    pet_entry: u32,
    pet_display_id: u32,
    pet_level: u32,
    creature_templates: Option<&wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
) -> (u32, u32, u32) {
    crate::handlers::character::enumeration_support::enum_character_pet_data_like_cpp(player_flags, at_login_flags, class_id, pet_entry, pet_display_id, pet_level, creature_templates)
}

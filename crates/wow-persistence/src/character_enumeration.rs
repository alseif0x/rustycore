//! Character-select enumeration request, rows and read-only port.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::PersistenceFutureLikeCpp;

/// SQLx-free input for the C++ character-select enumeration query holder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterEnumerationRequestLikeCpp {
    pub account_id: u32,
    pub declined_names_used: bool,
}

/// One Characters-database row consumed by the character-select application
/// layer. Packet flags, pet-template interpretation and GUID construction stay
/// with the gameplay/session owner.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterEnumerationRowLikeCpp {
    pub guid_low: u64,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub level: u8,
    pub zone: i32,
    pub map: i32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub guild_id: u64,
    pub player_flags: u32,
    pub at_login_flags: u16,
    pub pet_entry: u32,
    pub pet_display_id: u32,
    pub pet_level: u32,
    pub equipment_cache: String,
    pub banned_guid: u64,
    pub list_slot: u8,
    pub last_played_time: i64,
    pub active_talent_group: i16,
    pub last_login_build: u32,
    pub declined_genitive: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CharacterEnumerationLoadOutcomeLikeCpp {
    Loaded {
        rows: Vec<CharacterEnumerationRowLikeCpp>,
        /// `CHAR_DEL_EXPIRED_BANS` is best effort in C++; a failure is logged
        /// but does not suppress the subsequent enumeration result.
        expired_ban_cleanup_error: Option<String>,
    },
    Failed {
        reason: String,
        expired_ban_cleanup_error: Option<String>,
    },
}

/// Narrow Characters-database capability for `CMSG_ENUM_CHARACTERS`.
pub trait CharacterEnumerationPersistencePortLikeCpp: Send + Sync {
    fn load_character_enumeration_like_cpp<'a>(
        &'a self,
        request: CharacterEnumerationRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, CharacterEnumerationLoadOutcomeLikeCpp>;
}

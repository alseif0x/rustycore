// Copyright (c) 2026 alseif0x
// Licensed under GPL v3

//! MariaDB adapter for the character-select enumeration query holder.
//!
//! C++ anchors:
//! - `CharacterHandler.cpp:288-423`
//! - `CharacterDatabase.cpp:36,52-65`

use std::sync::Arc;

use wow_persistence::{
    CharacterEnumerationLoadOutcomeLikeCpp, CharacterEnumerationPersistencePortLikeCpp,
    CharacterEnumerationRequestLikeCpp, CharacterEnumerationRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, SqlResult};

fn character_enumeration_select_statement_like_cpp(declined_names_used: bool) -> CharStatements {
    if declined_names_used {
        CharStatements::SEL_ENUM_DECLINED_NAME
    } else {
        CharStatements::SEL_ENUM
    }
}

fn character_enumeration_row_like_cpp(
    result: &SqlResult,
    list_slot_fallback: u8,
    declined_names_used: bool,
) -> CharacterEnumerationRowLikeCpp {
    CharacterEnumerationRowLikeCpp {
        guid_low: result.read(0),
        name: result.read_string(1),
        race: result.read(2),
        class: result.read(3),
        gender: result.read(4),
        level: result.read(5),
        zone: result.try_read::<u16>(6).unwrap_or(0) as i32,
        map: result.try_read::<u16>(7).unwrap_or(0) as i32,
        position_x: result.try_read(8).unwrap_or(0.0),
        position_y: result.try_read(9).unwrap_or(0.0),
        position_z: result.try_read(10).unwrap_or(0.0),
        guild_id: result.try_read(11).unwrap_or(0),
        player_flags: result.try_read(12).unwrap_or(0),
        at_login_flags: result.try_read(13).unwrap_or(0),
        pet_entry: result.try_read(14).unwrap_or(0),
        pet_display_id: result.try_read(15).unwrap_or(0),
        pet_level: result.try_read(16).unwrap_or(0),
        equipment_cache: result.try_read(17).unwrap_or_default(),
        banned_guid: result.try_read(18).unwrap_or(0),
        list_slot: result.try_read(19).unwrap_or(list_slot_fallback),
        last_played_time: result.try_read(20).unwrap_or(0),
        active_talent_group: result.try_read::<u8>(21).unwrap_or(0) as i16,
        last_login_build: result.try_read(22).unwrap_or(54261),
        declined_genitive: declined_names_used
            .then(|| result.try_read::<String>(28).unwrap_or_default())
            .unwrap_or_default(),
    }
}

pub struct MariaDbCharacterEnumerationPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbCharacterEnumerationPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl CharacterEnumerationPersistencePortLikeCpp
    for MariaDbCharacterEnumerationPersistenceAdapterLikeCpp
{
    fn load_character_enumeration_like_cpp<'a>(
        &'a self,
        request: CharacterEnumerationRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, CharacterEnumerationLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let cleanup = self.character_db.prepare(CharStatements::DEL_EXPIRED_BANS);
            let expired_ban_cleanup_error = self
                .character_db
                .execute(&cleanup)
                .await
                .err()
                .map(|error| error.to_string());

            let mut statement =
                self.character_db
                    .prepare(character_enumeration_select_statement_like_cpp(
                        request.declined_names_used,
                    ));
            statement.set_u32(0, request.account_id);

            let mut result = match self.character_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return CharacterEnumerationLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                        expired_ban_cleanup_error,
                    };
                }
            };

            let mut rows = Vec::new();
            if !result.is_empty() {
                loop {
                    rows.push(character_enumeration_row_like_cpp(
                        &result,
                        rows.len() as u8,
                        request.declined_names_used,
                    ));
                    if !result.next_row() {
                        break;
                    }
                }
            }

            CharacterEnumerationLoadOutcomeLikeCpp::Loaded {
                rows,
                expired_ban_cleanup_error,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declined_name_configuration_selects_the_exact_cpp_projection() {
        assert_eq!(
            character_enumeration_select_statement_like_cpp(false),
            CharStatements::SEL_ENUM
        );
        assert_eq!(
            character_enumeration_select_statement_like_cpp(true),
            CharStatements::SEL_ENUM_DECLINED_NAME
        );
    }
}

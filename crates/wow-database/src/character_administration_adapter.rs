//! MariaDB adapter for C++ character-list administration.

use std::sync::Arc;

use wow_persistence::{
    CharacterAdministrationLoadOutcomeLikeCpp as LoadOutcome,
    CharacterAdministrationMutationOutcomeLikeCpp as MutationOutcome,
    CharacterAdministrationPersistencePortLikeCpp, CharacterCreatePersistenceRequestLikeCpp,
    CharacterCustomizationPersistenceLikeCpp, CharacterCustomizeCandidateLikeCpp,
    CharacterRenameCandidateLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, SqlTransaction, WorldDatabase, WorldStatements};

pub struct MariaDbCharacterAdministrationPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
    world_db: Arc<WorldDatabase>,
}

impl MariaDbCharacterAdministrationPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>, world_db: Arc<WorldDatabase>) -> Self {
        Self {
            character_db,
            world_db,
        }
    }
}

impl CharacterAdministrationPersistencePortLikeCpp
    for MariaDbCharacterAdministrationPersistenceAdapterLikeCpp
{
    fn find_character_name_like_cpp(
        &self,
        name: &str,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<()>> {
        let name = name.to_owned();
        Box::pin(async move {
            let mut statement = self.character_db.prepare(CharStatements::SEL_CHECK_NAME);
            statement.set_string(0, &name);
            match self.character_db.query(&statement).await {
                Ok(result) if result.is_empty() => LoadOutcome::NotFound,
                Ok(_) => LoadOutcome::Loaded(()),
                Err(error) => LoadOutcome::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_account_character_count_like_cpp(
        &self,
        account_id: u32,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<u64>> {
        Box::pin(async move {
            let mut statement = self.character_db.prepare(CharStatements::SEL_SUM_CHARS);
            statement.set_u32(0, account_id);
            match self.character_db.query(&statement).await {
                Ok(result) if result.is_empty() => LoadOutcome::NotFound,
                Ok(result) => LoadOutcome::Loaded(result.try_read(0).unwrap_or(0)),
                Err(error) => LoadOutcome::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn create_character_like_cpp(
        &self,
        request: CharacterCreatePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        Box::pin(async move {
            let mut statement = self.character_db.prepare(CharStatements::INS_CHARACTER);
            statement.set_u64(0, request.guid);
            statement.set_u32(1, request.account_id);
            statement.set_string(2, &request.name);
            statement.set_u8(3, request.race);
            statement.set_u8(4, request.class);
            statement.set_u8(5, request.sex);
            statement.set_u8(6, 1);
            statement.set_u64(7, 0);
            statement.set_u64(8, 0);
            statement.set_u32(9, 16);
            statement.set_u32(10, 0);
            statement.set_u8(11, request.rest_state);
            statement.set_u32(12, 0);
            statement.set_u32(13, 0);
            statement.set_i32(14, request.map_id);
            statement.set_u32(15, 0);
            statement.set_u8(16, 0);
            statement.set_u8(17, 0);
            statement.set_u8(18, 0);
            statement.set_f32(19, request.position[0]);
            statement.set_f32(20, request.position[1]);
            statement.set_f32(21, request.position[2]);
            statement.set_f32(22, request.position[3]);
            for index in 23..=26 {
                statement.set_f32(index, 0.0);
            }
            statement.set_u64(27, 0);
            statement.set_string(28, "");
            statement.set_i64(29, request.create_time);
            statement.set_u8(30, 0);
            statement.set_u8(31, 0);
            statement.set_u32(32, 0);
            statement.set_u32(33, 0);
            statement.set_f32(34, 0.0);
            statement.set_u64(35, request.create_time.max(0) as u64);
            statement.set_u8(36, 0);
            statement.set_u32(37, 0);
            statement.set_u32(38, 0);
            statement.set_u8(39, 0);
            statement.set_u8(40, 0);
            statement.set_u32(41, 0);
            statement.set_u32(42, 0);
            statement.set_u32(43, 0x20);
            statement.set_u32(44, 0);
            statement.set_string(45, "");
            for index in 46..=49 {
                statement.set_u32(index, 0);
            }
            statement.set_i32(50, 0);
            statement.set_u8(51, 0);
            statement.set_u32(52, request.health);
            statement.set_u32(53, request.power1);
            for index in 54..=64 {
                statement.set_u32(index, 0);
            }
            statement.set_string(65, "");
            statement.set_string(66, "");
            statement.set_string(67, "");
            statement.set_u8(68, 0);
            statement.set_u32(69, request.last_login_build);

            if let Err(error) = self.character_db.execute(&statement).await {
                return MutationOutcome::Failed {
                    reason: error.to_string(),
                };
            }

            // Preserve the existing best-effort order: character, choices,
            // then initial action buttons. C++-parity atomic creation remains
            // a gameplay gap; this boundary extraction does not alter it.
            for customization in &request.customizations {
                let mut statement = self
                    .character_db
                    .prepare(CharStatements::INS_CHAR_CUSTOMIZATION);
                statement.set_u64(0, request.guid);
                statement.set_i32(1, customization.option_id);
                statement.set_i32(2, customization.choice_id);
                let _ = self.character_db.execute(&statement).await;
            }

            let action_statement = self
                .world_db
                .prepare(WorldStatements::SEL_PLAYER_CREATEINFO_ACTION);
            if let Ok(mut rows) = self.world_db.query(&action_statement).await {
                if !rows.is_empty() {
                    loop {
                        let race: u8 = rows.read(0);
                        let class: u8 = rows.read(1);
                        let action: i32 = rows.try_read(3).unwrap_or(0);
                        if race == request.race && class == request.class && action > 0 {
                            let mut insert = self
                                .character_db
                                .prepare(CharStatements::INS_CHARACTER_ACTION);
                            insert.set_u64(0, request.guid);
                            insert.set_u8(1, rows.read(2));
                            insert.set_i32(2, action);
                            insert.set_u8(3, rows.try_read(4).unwrap_or(0));
                            let _ = self.character_db.execute(&insert).await;
                        }
                        if !rows.next_row() {
                            break;
                        }
                    }
                }
            }
            MutationOutcome::Applied
        })
    }

    fn delete_owned_character_like_cpp(
        &self,
        guid: u64,
        account_id: u32,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        Box::pin(async move {
            let mut check = self
                .character_db
                .prepare(CharStatements::SEL_CHAR_DEL_CHECK);
            check.set_u32(0, guid as u32);
            check.set_u32(1, account_id);
            if let Ok(result) = self.character_db.query(&check).await {
                if result.is_empty() {
                    return MutationOutcome::Failed {
                        reason: "character is not owned by account".into(),
                    };
                }
            }
            let mut statement = self.character_db.prepare(CharStatements::DEL_CHARACTER);
            statement.set_u32(0, guid as u32);
            match self.character_db.execute(&statement).await {
                Ok(_) => MutationOutcome::Applied,
                Err(error) => MutationOutcome::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_rename_candidate_like_cpp(
        &self,
        guid: u64,
        new_name: &str,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<CharacterRenameCandidateLikeCpp>> {
        let new_name = new_name.to_owned();
        Box::pin(async move {
            let mut statement = self.character_db.prepare(CharStatements::SEL_FREE_NAME);
            statement.set_u64(0, guid);
            statement.set_string(1, &new_name);
            match self.character_db.query(&statement).await {
                Ok(result) if result.is_empty() => LoadOutcome::NotFound,
                Ok(result) => LoadOutcome::Loaded(CharacterRenameCandidateLikeCpp {
                    old_name: result.read_string(0),
                    at_login_flags: result.try_read(1).unwrap_or(0),
                }),
                Err(error) => LoadOutcome::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn commit_rename_like_cpp(
        &self,
        guid: u64,
        new_name: &str,
        at_login_flags: u16,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        let new_name = new_name.to_owned();
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            let mut update = self
                .character_db
                .prepare(CharStatements::UPD_CHAR_NAME_AT_LOGIN);
            update.set_string(0, &new_name);
            update.set_u16(1, at_login_flags);
            update.set_u64(2, guid);
            transaction.append(update);
            let mut delete = self
                .character_db
                .prepare(CharStatements::DEL_CHAR_DECLINED_NAME);
            delete.set_u64(0, guid);
            transaction.append(delete);
            match self.character_db.commit_transaction(transaction).await {
                Ok(_) => MutationOutcome::Applied,
                Err(error) => MutationOutcome::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_customize_candidate_like_cpp(
        &self,
        guid: u64,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<CharacterCustomizeCandidateLikeCpp>> {
        Box::pin(async move {
            let mut statement = self
                .character_db
                .prepare(CharStatements::SEL_CHAR_CUSTOMIZE_INFO);
            statement.set_u64(0, guid);
            match self.character_db.query(&statement).await {
                Ok(result) if result.is_empty() => LoadOutcome::NotFound,
                Ok(result) => LoadOutcome::Loaded(CharacterCustomizeCandidateLikeCpp {
                    old_name: result.read_string(0),
                    race: result.try_read(1).unwrap_or(0),
                    class: result.try_read(2).unwrap_or(0),
                    gender: result.try_read(3).unwrap_or(0),
                    at_login_flags: result.try_read(4).unwrap_or(0),
                }),
                Err(error) => LoadOutcome::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn commit_customize_like_cpp(
        &self,
        guid: u64,
        name: &str,
        at_login_flags: u16,
        customizations: Vec<CharacterCustomizationPersistenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        let name = name.to_owned();
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            let mut delete = self
                .character_db
                .prepare(CharStatements::DEL_CHARACTER_CUSTOMIZATIONS);
            delete.set_u64(0, guid);
            transaction.append(delete);
            for customization in customizations {
                let mut insert = self
                    .character_db
                    .prepare(CharStatements::INS_CHAR_CUSTOMIZATION);
                insert.set_u64(0, guid);
                insert.set_i32(1, customization.option_id);
                insert.set_i32(2, customization.choice_id);
                transaction.append(insert);
            }
            let mut update = self
                .character_db
                .prepare(CharStatements::UPD_CHAR_NAME_AT_LOGIN);
            update.set_string(0, &name);
            update.set_u16(1, at_login_flags);
            update.set_u64(2, guid);
            transaction.append(update);
            let mut delete_declined = self
                .character_db
                .prepare(CharStatements::DEL_CHAR_DECLINED_NAME);
            delete_declined.set_u64(0, guid);
            transaction.append(delete_declined);
            match self.character_db.commit_transaction(transaction).await {
                Ok(_) => MutationOutcome::Applied,
                Err(error) => MutationOutcome::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

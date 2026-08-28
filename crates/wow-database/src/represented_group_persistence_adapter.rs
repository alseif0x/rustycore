//! MariaDB adapter for represented C++ `Group` durability commands.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, RepresentedGroupDifficultyKindLikeCpp,
    RepresentedGroupPersistenceCommandLikeCpp, RepresentedGroupPersistenceModeLikeCpp,
    RepresentedGroupPersistenceOutcomeLikeCpp, RepresentedGroupPersistencePortLikeCpp,
    RepresentedGroupPersistenceRequestLikeCpp,
};

use crate::{
    CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction,
    SqlTransactionCommitError, persistence_trace::LogicalDatabase,
};

fn represented_group_statement_like_cpp(
    command: RepresentedGroupPersistenceCommandLikeCpp,
) -> PreparedStatement {
    match command {
        RepresentedGroupPersistenceCommandLikeCpp::InsertGroup {
            db_store_id,
            leader_guid,
            loot_method,
            looter_guid,
            loot_threshold,
            group_flags,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
            master_looter_guid,
        } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::INS_GROUP);
            statement.set_u32(0, db_store_id);
            statement.set_u64(1, leader_guid);
            statement.set_u8(2, loot_method);
            statement.set_u64(3, looter_guid);
            statement.set_u8(4, loot_threshold);
            for index in 0..8 {
                statement.set_bytes(5 + index, vec![0; 16]);
            }
            statement.set_u16(13, group_flags);
            statement.set_u32(14, dungeon_difficulty_id);
            statement.set_u32(15, raid_difficulty_id);
            statement.set_u32(16, legacy_raid_difficulty_id);
            statement.set_u64(17, master_looter_guid);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::InsertMember {
            db_store_id,
            member_guid,
            member_flags,
            subgroup,
            roles,
        } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::INS_GROUP_MEMBER);
            statement.set_u32(0, db_store_id);
            statement.set_u64(1, member_guid);
            statement.set_u8(2, member_flags);
            statement.set_u8(3, subgroup);
            statement.set_u8(4, roles);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::DeleteGroup { db_store_id } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::DEL_GROUP);
            statement.set_u32(0, db_store_id);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::DeleteAllMembers { db_store_id } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::DEL_GROUP_MEMBER_ALL);
            statement.set_u32(0, db_store_id);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::DeleteLfgData { db_store_id } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::DEL_LFG_DATA);
            statement.set_u32(0, db_store_id);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::DeleteMember { member_guid } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::DEL_GROUP_MEMBER);
            statement.set_u64(0, member_guid);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::UpdateLeader {
            db_store_id,
            leader_guid,
        } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::UPD_GROUP_LEADER);
            statement.set_u64(0, leader_guid);
            statement.set_u32(1, db_store_id);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::UpdateGroupType {
            db_store_id,
            group_flags,
        } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::UPD_GROUP_TYPE);
            statement.set_u16(0, group_flags);
            statement.set_u32(1, db_store_id);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::UpdateMemberSubgroup {
            member_guid,
            subgroup,
        } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::UPD_GROUP_MEMBER_SUBGROUP);
            statement.set_u8(0, subgroup);
            statement.set_u64(1, member_guid);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::UpdateMemberFlags { member_guid, flags } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::UPD_GROUP_MEMBER_FLAG);
            statement.set_u8(0, flags);
            statement.set_u64(1, member_guid);
            statement
        }
        RepresentedGroupPersistenceCommandLikeCpp::UpdateDifficulty {
            db_store_id,
            kind,
            difficulty_id,
        } => {
            let statement_kind = match kind {
                RepresentedGroupDifficultyKindLikeCpp::Dungeon => {
                    CharStatements::UPD_GROUP_DIFFICULTY
                }
                RepresentedGroupDifficultyKindLikeCpp::Raid => {
                    CharStatements::UPD_GROUP_RAID_DIFFICULTY
                }
                RepresentedGroupDifficultyKindLikeCpp::LegacyRaid => {
                    CharStatements::UPD_GROUP_LEGACY_RAID_DIFFICULTY
                }
            };
            let mut statement = PreparedStatement::for_statement(statement_kind);
            statement.set_u32(0, difficulty_id);
            statement.set_u32(1, db_store_id);
            statement
        }
    }
}

pub struct MariaDbRepresentedGroupPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbRepresentedGroupPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl RepresentedGroupPersistencePortLikeCpp for MariaDbRepresentedGroupPersistenceAdapterLikeCpp {
    fn persist_group_commands_like_cpp(
        &self,
        request: RepresentedGroupPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, RepresentedGroupPersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let command_count = request.commands.len();
            match request.mode {
                RepresentedGroupPersistenceModeLikeCpp::Sequential => {
                    for (applied, command) in request.commands.into_iter().enumerate() {
                        let statement = represented_group_statement_like_cpp(command);
                        if let Err(error) = self.character_db.execute(&statement).await {
                            return RepresentedGroupPersistenceOutcomeLikeCpp::FailedAfterPrefix {
                                applied,
                                reason: error.to_string(),
                            };
                        }
                    }
                    RepresentedGroupPersistenceOutcomeLikeCpp::Applied { command_count }
                }
                RepresentedGroupPersistenceModeLikeCpp::Atomic => {
                    let mut transaction = SqlTransaction::new();
                    for command in request.commands {
                        transaction.append(represented_group_statement_like_cpp(command));
                    }
                    transaction.attribute_to_like_cpp(LogicalDatabase::Character);
                    match transaction
                        .commit_with_outcome_like_cpp(self.character_db.pool())
                        .await
                    {
                        Ok(()) => {
                            RepresentedGroupPersistenceOutcomeLikeCpp::Applied { command_count }
                        }
                        Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                            RepresentedGroupPersistenceOutcomeLikeCpp::DefinitelyRolledBack {
                                reason: error.to_string(),
                            }
                        }
                        Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
                            RepresentedGroupPersistenceOutcomeLikeCpp::CommitOutcomeUnknown {
                                command_count,
                                reason: error.to_string(),
                            }
                        }
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn represented_group_commands_preserve_cpp_statement_identity_and_bind_order() {
        let cases = [
            (
                RepresentedGroupPersistenceCommandLikeCpp::InsertMember {
                    db_store_id: 7,
                    member_guid: 11,
                    member_flags: 2,
                    subgroup: 3,
                    roles: 4,
                },
                CharStatements::INS_GROUP_MEMBER,
                vec![
                    SqlParam::U32(7),
                    SqlParam::U64(11),
                    SqlParam::U8(2),
                    SqlParam::U8(3),
                    SqlParam::U8(4),
                ],
            ),
            (
                RepresentedGroupPersistenceCommandLikeCpp::UpdateMemberSubgroup {
                    member_guid: 11,
                    subgroup: 5,
                },
                CharStatements::UPD_GROUP_MEMBER_SUBGROUP,
                vec![SqlParam::U8(5), SqlParam::U64(11)],
            ),
            (
                RepresentedGroupPersistenceCommandLikeCpp::UpdateMemberFlags {
                    member_guid: 11,
                    flags: 6,
                },
                CharStatements::UPD_GROUP_MEMBER_FLAG,
                vec![SqlParam::U8(6), SqlParam::U64(11)],
            ),
            (
                RepresentedGroupPersistenceCommandLikeCpp::UpdateDifficulty {
                    db_store_id: 7,
                    kind: RepresentedGroupDifficultyKindLikeCpp::LegacyRaid,
                    difficulty_id: 16,
                },
                CharStatements::UPD_GROUP_LEGACY_RAID_DIFFICULTY,
                vec![SqlParam::U32(16), SqlParam::U32(7)],
            ),
        ];

        for (command, statement_kind, params) in cases {
            let statement = represented_group_statement_like_cpp(command);
            assert_eq!(statement.sql(), statement_kind.sql());
            assert_eq!(statement.params(), params);
        }
    }

    #[test]
    fn group_insert_preserves_icons_and_all_cpp_binds() {
        let statement = represented_group_statement_like_cpp(
            RepresentedGroupPersistenceCommandLikeCpp::InsertGroup {
                db_store_id: 1,
                leader_guid: 2,
                loot_method: 3,
                looter_guid: 4,
                loot_threshold: 5,
                group_flags: 6,
                dungeon_difficulty_id: 7,
                raid_difficulty_id: 8,
                legacy_raid_difficulty_id: 9,
                master_looter_guid: 10,
            },
        );
        assert_eq!(statement.sql(), CharStatements::INS_GROUP.sql());
        assert_eq!(statement.params().len(), 18);
        assert_eq!(statement.params()[0], SqlParam::U32(1));
        assert_eq!(statement.params()[1], SqlParam::U64(2));
        assert_eq!(statement.params()[2], SqlParam::U8(3));
        assert_eq!(statement.params()[3], SqlParam::U64(4));
        assert_eq!(statement.params()[4], SqlParam::U8(5));
        assert!(
            statement.params()[5..13]
                .iter()
                .all(|param| *param == SqlParam::Bytes(vec![0; 16]))
        );
        assert_eq!(statement.params()[13], SqlParam::U16(6));
        assert_eq!(statement.params()[14], SqlParam::U32(7));
        assert_eq!(statement.params()[15], SqlParam::U32(8));
        assert_eq!(statement.params()[16], SqlParam::U32(9));
        assert_eq!(statement.params()[17], SqlParam::U64(10));
    }
}

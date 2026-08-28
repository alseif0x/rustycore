//! MariaDB adapter for the represented Player social-list workflows.

use std::sync::Arc;

use sqlx::Row;
use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, SocialAddCandidateLikeCpp,
    SocialAddCandidateLoadOutcomeLikeCpp, SocialContactListLoadOutcomeLikeCpp,
    SocialContactLoadRowLikeCpp, SocialPersistencePortLikeCpp, SocialRelationshipKindLikeCpp,
    SocialRelationshipStateLikeCpp,
};

use crate::CharacterDatabase;

const LOAD_CONTACTS_SQL: &str = "SELECT CAST(cs.friend AS SIGNED), cs.flags, cs.note, c.class, c.level, c.zone \
     FROM character_social cs \
     JOIN characters c ON c.guid = cs.friend \
     WHERE cs.guid = ? AND (cs.flags & ?) <> 0";
const LOAD_FRIEND_SQL: &str = "SELECT CAST(guid AS SIGNED), account, race, class, level, zone FROM characters WHERE name = ? LIMIT 1";
const LOAD_IGNORE_SQL: &str = "SELECT CAST(guid AS SIGNED) FROM characters WHERE name = ? LIMIT 1";

#[derive(Debug, Clone, PartialEq, Eq)]
enum SocialSqlBindLikeCpp {
    I64(i64),
    U8(u8),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SocialSqlOperationLikeCpp {
    sql: &'static str,
    binds: Vec<SocialSqlBindLikeCpp>,
}

fn add_relationship_operation_like_cpp(
    player_guid: i64,
    target_guid: i64,
    kind: SocialRelationshipKindLikeCpp,
    note: String,
) -> SocialSqlOperationLikeCpp {
    match kind {
        SocialRelationshipKindLikeCpp::Friend => SocialSqlOperationLikeCpp {
            sql: "INSERT INTO character_social (guid, friend, flags, note) VALUES (?, ?, 1, ?) \
                  ON DUPLICATE KEY UPDATE flags = flags | 1, note = VALUES(note)",
            binds: vec![
                SocialSqlBindLikeCpp::I64(player_guid),
                SocialSqlBindLikeCpp::I64(target_guid),
                SocialSqlBindLikeCpp::Text(note),
            ],
        },
        SocialRelationshipKindLikeCpp::Ignored => SocialSqlOperationLikeCpp {
            sql: "INSERT INTO character_social (guid, friend, flags, note) VALUES (?, ?, 2, '') \
                  ON DUPLICATE KEY UPDATE flags = flags | 2",
            binds: vec![
                SocialSqlBindLikeCpp::I64(player_guid),
                SocialSqlBindLikeCpp::I64(target_guid),
            ],
        },
    }
}

fn remove_relationship_operations_like_cpp(
    player_guid: i64,
    target_guid: i64,
    kind: SocialRelationshipKindLikeCpp,
) -> [SocialSqlOperationLikeCpp; 2] {
    let (mask, flag) = match kind {
        SocialRelationshipKindLikeCpp::Friend => (254_u8, 1_u8),
        SocialRelationshipKindLikeCpp::Ignored => (253_u8, 2_u8),
    };
    [
        SocialSqlOperationLikeCpp {
            sql: "UPDATE character_social SET flags = flags & ? \
                  WHERE guid = ? AND friend = ? AND flags & ?",
            binds: vec![
                SocialSqlBindLikeCpp::U8(mask),
                SocialSqlBindLikeCpp::I64(player_guid),
                SocialSqlBindLikeCpp::I64(target_guid),
                SocialSqlBindLikeCpp::U8(flag),
            ],
        },
        SocialSqlOperationLikeCpp {
            sql: "DELETE FROM character_social WHERE guid = ? AND friend = ? AND flags = 0",
            binds: vec![
                SocialSqlBindLikeCpp::I64(player_guid),
                SocialSqlBindLikeCpp::I64(target_guid),
            ],
        },
    ]
}

async fn execute_social_operation_like_cpp(
    character_db: &CharacterDatabase,
    operation: SocialSqlOperationLikeCpp,
) -> Result<u64, String> {
    let mut query = sqlx::query(operation.sql);
    for bind in operation.binds {
        query = match bind {
            SocialSqlBindLikeCpp::I64(value) => query.bind(value),
            SocialSqlBindLikeCpp::U8(value) => query.bind(value),
            SocialSqlBindLikeCpp::Text(value) => query.bind(value),
        };
    }
    query
        .execute(character_db.pool())
        .await
        .map(|result| result.rows_affected())
        .map_err(|error| error.to_string())
}

pub struct MariaDbSocialPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbSocialPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl SocialPersistencePortLikeCpp for MariaDbSocialPersistenceAdapterLikeCpp {
    fn load_contacts_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        flags: u32,
    ) -> PersistenceFutureLikeCpp<'a, SocialContactListLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let rows = match sqlx::query(LOAD_CONTACTS_SQL)
                .bind(player_guid)
                .bind(flags)
                .fetch_all(self.character_db.pool())
                .await
            {
                Ok(rows) => rows,
                Err(error) => {
                    return SocialContactListLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            SocialContactListLoadOutcomeLikeCpp::Loaded(
                rows.into_iter()
                    .map(|row| SocialContactLoadRowLikeCpp {
                        friend_guid: row.try_get(0).unwrap_or(0),
                        type_flags: row.try_get::<u8, _>(1).unwrap_or(0) as u32,
                        note: row.try_get(2).unwrap_or_default(),
                        class_id: row.try_get::<u8, _>(3).unwrap_or(0) as u32,
                        level: row.try_get::<u8, _>(4).unwrap_or(0) as u32,
                        zone_id: row.try_get::<i32, _>(5).unwrap_or(0) as u32,
                    })
                    .collect(),
            )
        })
    }

    fn load_add_candidate_like_cpp<'a>(
        &'a self,
        normalized_name: String,
        kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SocialAddCandidateLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let row = match kind {
                SocialRelationshipKindLikeCpp::Friend => {
                    sqlx::query(LOAD_FRIEND_SQL)
                        .bind(&normalized_name)
                        .fetch_optional(self.character_db.pool())
                        .await
                }
                SocialRelationshipKindLikeCpp::Ignored => {
                    sqlx::query(LOAD_IGNORE_SQL)
                        .bind(&normalized_name)
                        .fetch_optional(self.character_db.pool())
                        .await
                }
            };
            let row = match row {
                Ok(Some(row)) => row,
                Ok(None) => return SocialAddCandidateLoadOutcomeLikeCpp::NotFound,
                Err(error) => {
                    return SocialAddCandidateLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            let guid = row.try_get::<i64, _>(0).unwrap_or(0);
            SocialAddCandidateLoadOutcomeLikeCpp::Found(SocialAddCandidateLikeCpp {
                guid,
                race: if kind == SocialRelationshipKindLikeCpp::Friend {
                    row.try_get(2).unwrap_or(0)
                } else {
                    0
                },
                class_id: if kind == SocialRelationshipKindLikeCpp::Friend {
                    row.try_get::<u8, _>(3).unwrap_or(0) as u32
                } else {
                    0
                },
                level: if kind == SocialRelationshipKindLikeCpp::Friend {
                    row.try_get::<u8, _>(4).unwrap_or(0) as i32
                } else {
                    0
                },
                zone_id: if kind == SocialRelationshipKindLikeCpp::Friend {
                    row.try_get::<i32, _>(5).unwrap_or(0)
                } else {
                    0
                },
            })
        })
    }

    fn load_relationship_state_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        target_guid: i64,
        kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SocialRelationshipStateLikeCpp> {
        Box::pin(async move {
            let flag = match kind {
                SocialRelationshipKindLikeCpp::Friend => 1_u8,
                SocialRelationshipKindLikeCpp::Ignored => 2_u8,
            };
            let already_present = sqlx::query(
                "SELECT COUNT(*) FROM character_social WHERE guid = ? AND friend = ? AND flags & ?",
            )
            .bind(player_guid)
            .bind(target_guid)
            .bind(flag)
            .fetch_one(self.character_db.pool())
            .await
            .ok()
            .and_then(|row| row.try_get::<i64, _>(0).ok())
            .unwrap_or(0)
                > 0;
            if already_present {
                return SocialRelationshipStateLikeCpp {
                    already_present: true,
                    relationship_count: 0,
                };
            }
            let relationship_count =
                sqlx::query("SELECT COUNT(*) FROM character_social WHERE guid = ? AND flags & ?")
                    .bind(player_guid)
                    .bind(flag)
                    .fetch_one(self.character_db.pool())
                    .await
                    .ok()
                    .and_then(|row| row.try_get::<i64, _>(0).ok())
                    .unwrap_or(0);
            SocialRelationshipStateLikeCpp {
                already_present: false,
                relationship_count,
            }
        })
    }

    fn add_relationship_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        target_guid: i64,
        kind: SocialRelationshipKindLikeCpp,
        note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            match execute_social_operation_like_cpp(
                &self.character_db,
                add_relationship_operation_like_cpp(player_guid, target_guid, kind, note),
            )
            .await
            {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(reason) => PersistenceOutcomeLikeCpp::Failed { reason },
            }
        })
    }

    fn remove_relationship_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        target_guid: i64,
        kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let [update, cleanup] =
                remove_relationship_operations_like_cpp(player_guid, target_guid, kind);
            let update_rows =
                match execute_social_operation_like_cpp(&self.character_db, update).await {
                    Ok(rows) => rows,
                    Err(reason) => {
                        return PersistenceOutcomeLikeCpp::Failed { reason };
                    }
                };
            match execute_social_operation_like_cpp(&self.character_db, cleanup).await {
                Ok(cleanup_rows) => PersistenceOutcomeLikeCpp::Applied {
                    rows: update_rows + cleanup_rows,
                },
                Err(reason) => PersistenceOutcomeLikeCpp::Failed { reason },
            }
        })
    }

    fn set_contact_note_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        target_guid: i64,
        note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            match sqlx::query("UPDATE character_social SET note = ? WHERE guid = ? AND friend = ?")
                .bind(note)
                .bind(player_guid)
                .bind(target_guid)
                .execute(self.character_db.pool())
                .await
            {
                Ok(result) => PersistenceOutcomeLikeCpp::Applied {
                    rows: result.rows_affected(),
                },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn social_queries_keep_the_current_read_shapes_inside_the_adapter() {
        assert!(LOAD_CONTACTS_SQL.contains("(cs.flags & ?) <> 0"));
        assert!(LOAD_FRIEND_SQL.contains("account, race, class, level, zone"));
        assert_eq!(LOAD_IGNORE_SQL.matches('?').count(), 1);
    }

    #[test]
    fn friend_upsert_plan_preserves_sql_and_bind_order() {
        let operation = add_relationship_operation_like_cpp(
            42,
            77,
            SocialRelationshipKindLikeCpp::Friend,
            "raid".into(),
        );
        assert!(operation.sql.contains("flags = flags | 1"));
        assert_eq!(
            operation.binds,
            vec![
                SocialSqlBindLikeCpp::I64(42),
                SocialSqlBindLikeCpp::I64(77),
                SocialSqlBindLikeCpp::Text("raid".into()),
            ]
        );
    }

    #[test]
    fn ignored_remove_plan_clears_then_cleans_up_with_exact_binds() {
        let [clear, cleanup] =
            remove_relationship_operations_like_cpp(42, 77, SocialRelationshipKindLikeCpp::Ignored);
        assert!(clear.sql.starts_with("UPDATE character_social"));
        assert_eq!(
            clear.binds,
            vec![
                SocialSqlBindLikeCpp::U8(253),
                SocialSqlBindLikeCpp::I64(42),
                SocialSqlBindLikeCpp::I64(77),
                SocialSqlBindLikeCpp::U8(2),
            ]
        );
        assert!(cleanup.sql.starts_with("DELETE FROM character_social"));
        assert_eq!(
            cleanup.binds,
            vec![SocialSqlBindLikeCpp::I64(42), SocialSqlBindLikeCpp::I64(77)]
        );
    }
}

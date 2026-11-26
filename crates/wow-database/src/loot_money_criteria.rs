//! Transactional Characters-database updates for criterion 4093.

use sqlx::{MySql, Transaction};
use wow_persistence::{LOOT_MONEY_ACHIEVEMENTS, LOOT_MONEY_CRITERION, LootMoneyCriteriaProgress};

use crate::{
    DatabaseError,
    persistence_trace::{ExplicitTransactionTrace, TracedParam},
};

const SELECT_PROGRESS: &str = "SELECT counter, date FROM character_achievement_progress WHERE guid = ? AND criteria = ? FOR UPDATE";
const SELECT_ACHIEVEMENTS: &str = "SELECT achievement FROM character_achievement WHERE guid = ? AND achievement IN (1176,1177,1178,1180,1181) FOR UPDATE";
const SELECT_NOW: &str = "SELECT CAST(UNIX_TIMESTAMP() AS UNSIGNED)";
const INSERT_PROGRESS: &str = "INSERT INTO character_achievement_progress (guid, criteria, counter, date) VALUES (?, ?, ?, ?)";
const UPDATE_PROGRESS: &str = "UPDATE character_achievement_progress SET counter = ?, date = ? WHERE guid = ? AND criteria = ?";
const INSERT_ACHIEVEMENT: &str =
    "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)";

/// Advance eligible creature-money progress in the caller's claim transaction.
/// The character money row is locked first by each caller, serializing this
/// operation with other loot-money claims for that character.
pub(crate) async fn advance(
    transaction: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    player_guid: u64,
    base_amount: u64,
) -> Result<Option<LootMoneyCriteriaProgress>, DatabaseError> {
    if base_amount == 0 {
        return Ok(None);
    }

    trace.statement(|| {
        (
            SELECT_PROGRESS.into(),
            vec![
                TracedParam::Uint {
                    value: player_guid,
                    width_bits: 64,
                },
                TracedParam::Uint {
                    value: u64::from(LOOT_MONEY_CRITERION),
                    width_bits: 32,
                },
            ],
        )
    });
    let previous = sqlx::query_as::<_, (u64, i64)>(SELECT_PROGRESS)
        .bind(player_guid)
        .bind(LOOT_MONEY_CRITERION)
        .fetch_optional(&mut **transaction)
        .await?;
    let previous_counter = previous.map(|(counter, _)| counter).unwrap_or(0);
    let counter = previous_counter.saturating_add(base_amount);
    let progress_changed = counter != previous_counter;
    let previous_date = previous
        .map(|(_, date)| u64::try_from(date).unwrap_or_default())
        .unwrap_or_default();

    trace.statement(|| {
        (
            SELECT_ACHIEVEMENTS.into(),
            vec![TracedParam::Uint {
                value: player_guid,
                width_bits: 64,
            }],
        )
    });
    let existing = sqlx::query_scalar::<_, u32>(SELECT_ACHIEVEMENTS)
        .bind(player_guid)
        .fetch_all(&mut **transaction)
        .await?;

    let achievements_to_add: Vec<u32> = LOOT_MONEY_ACHIEVEMENTS
        .iter()
        .filter(|(achievement_id, threshold)| {
            counter >= *threshold && !existing.contains(achievement_id)
        })
        .map(|(achievement_id, _)| *achievement_id)
        .collect();
    if !progress_changed && achievements_to_add.is_empty() {
        // C++ keeps the progress timestamp unchanged at u64 saturation. Its
        // caller still checks criteria trees after SetCriteriaProgress returns.
        return Ok(None);
    }

    trace.statement(|| (SELECT_NOW.into(), Vec::<TracedParam>::new()));
    let now = sqlx::query_scalar::<_, u64>(SELECT_NOW)
        .fetch_one(&mut **transaction)
        .await?;
    let progress_date = if progress_changed { now } else { previous_date };

    if progress_changed {
        match previous {
            Some(_) => {
                trace.statement_expecting(
                    || {
                        (
                            UPDATE_PROGRESS.into(),
                            vec![
                                TracedParam::Uint {
                                    value: counter,
                                    width_bits: 64,
                                },
                                TracedParam::Uint {
                                    value: progress_date,
                                    width_bits: 64,
                                },
                                TracedParam::Uint {
                                    value: player_guid,
                                    width_bits: 64,
                                },
                                TracedParam::Uint {
                                    value: u64::from(LOOT_MONEY_CRITERION),
                                    width_bits: 32,
                                },
                            ],
                        )
                    },
                    1,
                );
                let updated = sqlx::query(UPDATE_PROGRESS)
                    .bind(counter)
                    .bind(progress_date)
                    .bind(player_guid)
                    .bind(LOOT_MONEY_CRITERION)
                    .execute(&mut **transaction)
                    .await?;
                if updated.rows_affected() != 1 {
                    return Err(DatabaseError::Transaction(format!(
                        "loot-money criterion progress update for character {player_guid} affected {}; expected exactly 1",
                        updated.rows_affected()
                    )));
                }
            }
            None => {
                trace.statement_expecting(
                    || {
                        (
                            INSERT_PROGRESS.into(),
                            vec![
                                TracedParam::Uint {
                                    value: player_guid,
                                    width_bits: 64,
                                },
                                TracedParam::Uint {
                                    value: u64::from(LOOT_MONEY_CRITERION),
                                    width_bits: 32,
                                },
                                TracedParam::Uint {
                                    value: counter,
                                    width_bits: 64,
                                },
                                TracedParam::Uint {
                                    value: progress_date,
                                    width_bits: 64,
                                },
                            ],
                        )
                    },
                    1,
                );
                let inserted = sqlx::query(INSERT_PROGRESS)
                    .bind(player_guid)
                    .bind(LOOT_MONEY_CRITERION)
                    .bind(counter)
                    .bind(progress_date)
                    .execute(&mut **transaction)
                    .await?;
                if inserted.rows_affected() != 1 {
                    return Err(DatabaseError::Transaction(format!(
                        "loot-money criterion progress insert for character {player_guid} affected {}; expected exactly 1",
                        inserted.rows_affected()
                    )));
                }
            }
        }
    }

    let mut new_achievements = Vec::new();
    for achievement_id in achievements_to_add {
        trace.statement_expecting(
            || {
                (
                    INSERT_ACHIEVEMENT.into(),
                    vec![
                        TracedParam::Uint {
                            value: player_guid,
                            width_bits: 64,
                        },
                        TracedParam::Uint {
                            value: u64::from(achievement_id),
                            width_bits: 32,
                        },
                        TracedParam::Uint {
                            value: now,
                            width_bits: 64,
                        },
                    ],
                )
            },
            1,
        );
        let inserted = sqlx::query(INSERT_ACHIEVEMENT)
            .bind(player_guid)
            .bind(achievement_id)
            .bind(now)
            .execute(&mut **transaction)
            .await?;
        if inserted.rows_affected() != 1 {
            return Err(DatabaseError::Transaction(format!(
                "loot-money achievement {achievement_id} insert for character {player_guid} affected {}; expected exactly 1",
                inserted.rows_affected()
            )));
        }
        new_achievements.push(achievement_id);
    }

    let achievement_date = (!new_achievements.is_empty()).then_some(now);
    Ok(Some(LootMoneyCriteriaProgress {
        counter,
        date: progress_date,
        new_achievements,
        achievement_date,
    }))
}

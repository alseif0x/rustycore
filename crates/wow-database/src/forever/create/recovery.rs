//! Projection-only recovery of a committed creation, never Player reconstruction.
use super::{
    CreateError, ForeverSessionRepository, PreparedStatement, SqlTransaction, field, journal,
    writer,
};
use journal::Entry;

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_recovery.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreationRecoveryReport {
    pub completed: u64,
}

// Names are private; neither row type is Debug.
struct CanonicalCharacter {
    guid: u64,
    account: u32,
    name: String,
    logout_time: i64,
    deleted: bool,
}

struct LastPlayed {
    realm: u32,
    name: String,
    guid: u64,
    time: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastPlayedWrite {
    Keep,
    Insert,
    Replace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecoveryPlan {
    count: u8,
    last_played: LastPlayedWrite,
}

fn plan(
    entry: Entry,
    character: &CanonicalCharacter,
    count: u64,
    last: Option<&LastPlayed>,
) -> Result<RecoveryPlan, CreateError> {
    if character.guid != entry.guid
        || character.account != entry.account
        || character.name.is_empty()
        || character.deleted
        || character.logout_time != i64::from(entry.save_time)
        || count == 0
    {
        return Err(CreateError::RecoveryRequired);
    }
    let count = u8::try_from(count).map_err(|_| CreateError::InvalidRow)?;
    let last_played = match last {
        None => LastPlayedWrite::Insert,
        Some(last) if last.realm == 0 || last.guid == 0 || last.name.is_empty() => {
            return Err(CreateError::RecoveryRequired);
        }
        Some(last) if last.time > entry.save_time => LastPlayedWrite::Keep,
        Some(last) if last.time < entry.save_time => LastPlayedWrite::Replace,
        Some(last)
            if last.guid == entry.guid
                && last.realm == (entry.realm & 0xFFFF)
                && last.name == character.name =>
        {
            LastPlayedWrite::Keep
        }
        Some(_) => return Err(CreateError::RecoveryRequired),
    };
    Ok(RecoveryPlan { count, last_played })
}

fn projection(entry: Entry, character: CanonicalCharacter, plan: RecoveryPlan) -> SqlTransaction {
    let mut transaction = SqlTransaction::new();
    if plan.last_played == LastPlayedWrite::Replace {
        transaction.append_expect_rows_affected(writer::delete_last_played(entry), 1);
    }
    if plan.last_played != LastPlayedWrite::Keep {
        transaction
            .append_expect_rows_affected(writer::insert_last_played(entry, character.name), 1);
    }
    transaction.append(writer::realm_count(entry, plan.count));
    transaction
}

impl ForeverSessionRepository {
    /// Explicit startup recovery, before publishing account/character state.
    /// This does not apply schema or enable a creation/gameplay writer.
    pub async fn reconcile_character_creations(
        &self,
        realm: u32,
    ) -> Result<CreationRecoveryReport, CreateError> {
        if realm & 0xFFFF == 0 {
            return Err(CreateError::InvalidRow);
        }
        let mut ids = self.creation_ids.lock().await;
        journal::ensure_schema(&self.characters, &self.auth).await?;
        let mut writer = writer::CreationWriter::acquire(&self.characters).await?;
        // Recovery may partially finish before error/cancellation. Keep the
        // local fence until the entire durable journal is demonstrably empty.
        ids.recovery_required = true;
        let mut completed = 0_u64;
        loop {
            let mut rows = writer.query(&PreparedStatement::new(
                "SELECT player_guid,account_id,realm_address,save_time,format_version FROM rustycore_forever_creation_journal ORDER BY player_guid LIMIT 64",
            )).await?;
            if rows.is_empty() {
                break;
            }
            let mut entries = Vec::new();
            loop {
                let entry = Entry {
                    guid: field(&rows, 0).map_err(|_| CreateError::InvalidRow)?,
                    account: field(&rows, 1).map_err(|_| CreateError::InvalidRow)?,
                    realm: field(&rows, 2).map_err(|_| CreateError::InvalidRow)?,
                    save_time: field(&rows, 3).map_err(|_| CreateError::InvalidRow)?,
                };
                let version: u8 = field(&rows, 4).map_err(|_| CreateError::InvalidRow)?;
                if version != 1
                    || entry.realm != realm
                    || entry.account == 0
                    || entry.guid == 0
                    || entry.guid >= 0xFFFFFFFFFF - 1
                {
                    return Err(CreateError::RecoveryRequired);
                }
                entries.push(entry);
                if !rows.next_row() {
                    break;
                }
            }
            for entry in entries {
                // The predecessor acquired these Auth locks before writing
                // Characters. Its Auth COMMIT/rollback must settle first.
                let mut auth = writer::lock_account(&self.auth, entry.account, entry.realm).await?;
                let mut statement = PreparedStatement::new(
                    "SELECT guid,account,name,logout_time,deleteDate FROM characters WHERE guid=?",
                );
                statement.set_u64(0, entry.guid);
                let row = writer.query(&statement).await?;
                if row.count() != 1 {
                    return Err(CreateError::RecoveryRequired);
                }
                let character = CanonicalCharacter {
                    guid: field(&row, 0).map_err(|_| CreateError::InvalidRow)?,
                    account: field(&row, 1).map_err(|_| CreateError::InvalidRow)?,
                    name: field(&row, 2).map_err(|_| CreateError::InvalidRow)?,
                    logout_time: field(&row, 3).map_err(|_| CreateError::InvalidRow)?,
                    deleted: field::<Option<i64>>(&row, 4)
                        .map_err(|_| CreateError::InvalidRow)?
                        .is_some(),
                };
                let mut statement = PreparedStatement::new(
                    "SELECT CAST(COUNT(guid) AS UNSIGNED) FROM characters WHERE account=? AND deleteDate IS NULL",
                );
                statement.set_u32(0, entry.account);
                let count: u64 = field(&writer.query(&statement).await?, 0)
                    .map_err(|_| CreateError::InvalidRow)?;
                let mut statement = PreparedStatement::new(
                    "SELECT realmId,characterName,characterGUID,lastPlayedTime FROM account_last_played_character WHERE accountId=? AND region=? AND battlegroup=? FOR UPDATE",
                );
                statement.set_u32(0, entry.account);
                statement.set_u8(1, (entry.realm >> 24) as u8);
                statement.set_u8(2, (entry.realm >> 16) as u8);
                let row = writer::auth_query(&mut auth, &statement).await?;
                if row.count() > 1 {
                    return Err(CreateError::InvalidRow);
                }
                let last = if row.is_empty() {
                    None
                } else {
                    Some(LastPlayed {
                        realm: field::<Option<u32>>(&row, 0)
                            .map_err(|_| CreateError::InvalidRow)?
                            .ok_or(CreateError::InvalidRow)?,
                        name: field::<Option<String>>(&row, 1)
                            .map_err(|_| CreateError::InvalidRow)?
                            .ok_or(CreateError::InvalidRow)?,
                        guid: field::<Option<u64>>(&row, 2)
                            .map_err(|_| CreateError::InvalidRow)?
                            .ok_or(CreateError::InvalidRow)?,
                        time: field::<Option<u32>>(&row, 3)
                            .map_err(|_| CreateError::InvalidRow)?
                            .ok_or(CreateError::InvalidRow)?,
                    })
                };
                let plan = plan(entry, &character, count, last.as_ref())?;
                writer::commit_auth(projection(entry, character, plan), auth)
                    .await
                    .map_err(|_| CreateError::LoginCommitUnknown)?;
                writer
                    .commit(entry.cleanup())
                    .await
                    .map_err(|_| CreateError::RecoveryRequired)?;
                completed = completed.checked_add(1).ok_or(CreateError::InvalidRow)?;
            }
        }
        ids.recovery_required = false;
        Ok(CreationRecoveryReport { completed })
    }
}

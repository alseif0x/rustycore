//! Restore the `characters` columns the quest smoke rewrites before login.
//!
//! `--quest-relocate`, `--quest-set-level`, `--quest-set-race` and
//! `--quest-set-class` mutate the bot character directly. The snapshot is
//! taken before those writes and put back after the run once the server has
//! finished its logout save. Server-derived state of the mutated session
//! (learned spells/skills, reputation) is not reverted; a quest status row the
//! run created is removed only when none existed before.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(crate) struct QuestCharacterSnapshot {
    pub(crate) race: u8,
    pub(crate) class: u8,
    pub(crate) level: u8,
    pub(crate) map_id: u32,
    pub(crate) zone_id: u32,
    pub(crate) instance_id: u32,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) orientation: f32,
    /// Whether `character_queststatus` held the expected quest before setup.
    pub(crate) quest_status_present: Option<bool>,
}

pub(crate) fn quest_smoke_mutates_character(quest_options: &QuestSmokeOptions) -> bool {
    quest_options.relocate_before_login
        || quest_options.set_level_before_login.is_some()
        || quest_options.set_race_before_login.is_some()
        || quest_options.set_class_before_login.is_some()
}

pub(crate) fn snapshot_quest_character(
    conn: &mut mysql::Conn,
    character_guid: u64,
    quest_id: Option<u32>,
) -> Result<QuestCharacterSnapshot> {
    use mysql::prelude::Queryable;

    type Row = (u8, u8, u8, u32, u32, u32, f64, f64, f64, f32);
    let row: Option<Row> = conn
        .exec_first(
            "SELECT race, class, level, map, zone, instance_id, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (character_guid,),
        )
        .map_err(|e| anyhow!("Snapshot quest bot character: {e}"))?;
    let (race, class, level, map_id, zone_id, instance_id, x, y, z, orientation) =
        row.ok_or_else(|| anyhow!("No characters row for guid {character_guid}"))?;
    let quest_status_present = match quest_id {
        Some(quest_id) => Some(
            conn.exec_first::<u32, _, _>(
                "SELECT quest FROM character_queststatus WHERE guid = ? AND quest = ?",
                (character_guid, quest_id),
            )
            .map_err(|e| anyhow!("Snapshot quest status: {e}"))?
            .is_some(),
        ),
        None => None,
    };
    Ok(QuestCharacterSnapshot {
        race,
        class,
        level,
        map_id,
        zone_id,
        instance_id,
        x,
        y,
        z,
        orientation,
        quest_status_present,
    })
}

pub(crate) fn restore_quest_character(
    bot: &config::BotConfig,
    snapshot: &QuestCharacterSnapshot,
    quest_id: Option<u32>,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    // A late logout save would overwrite the restored row; wait for it.
    let offline_deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|e| anyhow!("Check quest bot offline state: {e}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < offline_deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => bail!(
                "character {} remained online; refusing to restore it before the logout save",
                bot.character_guid
            ),
            None => bail!("No characters row for guid {}", bot.character_guid),
        }
    }
    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start quest restore transaction: {e}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET race = ?, class = ?, level = ?, map = ?, zone = ?, instance_id = ?, \
             position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ?",
            (
                snapshot.race,
                snapshot.class,
                snapshot.level,
                snapshot.map_id,
                snapshot.zone_id,
                snapshot.instance_id,
                snapshot.x,
                snapshot.y,
                snapshot.z,
                snapshot.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|e| anyhow!("Restore quest bot character: {e}"))?;
    if let (Some(quest_id), Some(false)) = (quest_id, snapshot.quest_status_present) {
        for table in ["character_queststatus_objectives", "character_queststatus"] {
            transaction
                .exec_drop(
                    format!("DELETE FROM {table} WHERE guid = ? AND quest = ?"),
                    (bot.character_guid, quest_id),
                )
                .map_err(|e| anyhow!("Remove quest smoke row from {table}: {e}"))?;
        }
    }
    transaction
        .commit()
        .map_err(|e| anyhow!("Commit quest restore transaction: {e}"))?;
    Ok(())
}

/// Log out (unless the objective check already did), then restore the
/// snapshot. A restore failure is reported, never hidden; it does not change
/// the quest verdict, which was measured before the restore.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn restore_quest_character_after_run(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    quest_options: &QuestSmokeOptions,
    snapshot: QuestCharacterSnapshot,
    result: &mut BotRunResult,
) {
    if !quest_options.objective_persist {
        if let Err(error) = logout_and_wait(bot_index, stream, crypt, server_inflater, result).await
        {
            warn!(
                "[Bot {bot_index}] quest restore logout: {error}; waiting for the disconnect save"
            );
        }
    }
    let bot_for_db = bot.clone();
    let quest_id = quest_options.expected_quest_id;
    let restored = tokio::task::spawn_blocking(move || {
        restore_quest_character(&bot_for_db, &snapshot, quest_id)
    })
    .await
    .map_err(|e| anyhow!("Quest restore DB worker join failed: {e}"))
    .and_then(|restored| restored);
    match restored {
        Ok(()) => result.quest_character_restored = Some(true),
        Err(error) => {
            result.quest_character_restored = Some(false);
            result.quest_restore_failure = Some(error.to_string());
        }
    }
}

//! Misc operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn load_rested_xp_db_state(bot: &config::BotConfig) -> Result<RestedXpDbState> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let row: Option<(u8, u32, u8, f32, u8)> = conn
        .exec_first(
            "SELECT level, xp, restState, rest_bonus, online FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load rested-XP persistence state: {error}"))?;
    row.map(
        |(level, xp, rest_state, rest_bonus, online)| RestedXpDbState {
            level,
            xp,
            rest_state,
            rest_bonus,
            online,
        },
    )
    .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))
}
pub(crate) fn rested_xp_restore_params(
    restore_point: &RestedXpCharacterRestorePoint,
    character_guid: u64,
) -> Vec<mysql::Value> {
    let mut values = vec![
        restore_point.level.into(),
        restore_point.xp.into(),
        restore_point.rest_state.into(),
        restore_point.player_flags.into(),
        restore_point.rest_bonus.into(),
        restore_point.logout_time.into(),
        restore_point.is_logout_resting.into(),
        restore_point.map_id.into(),
        restore_point.zone_id.into(),
        restore_point.instance_id.into(),
        restore_point.x.into(),
        restore_point.y.into(),
        restore_point.z.into(),
        restore_point.orientation.into(),
        restore_point.health.into(),
    ];
    values.extend(restore_point.powers.iter().copied().map(mysql::Value::from));
    values.extend([
        restore_point.total_kills.into(),
        restore_point.today_kills.into(),
        restore_point.yesterday_kills.into(),
        restore_point.total_time.into(),
        restore_point.level_time.into(),
        restore_point.latency.into(),
        restore_point.last_login_build.into(),
        character_guid.into(),
    ]);
    values
}
pub(crate) fn wait_for_rested_xp_game_account_offline(
    bot: &config::BotConfig,
    fixture: &RestedXpSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let auth_url = auth_db_url()?;
    let opts =
        mysql::Opts::from_url(&auth_url).map_err(|error| anyhow!("Bad auth DB URL: {error}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|error| anyhow!("Connect to auth DB failed: {error}"))?;
    let deadline = std::time::Instant::now()
        + Duration::from_secs(
            fixture
                .options
                .timeout_secs
                .clamp(10, RESTED_XP_DISCONNECT_SAVE_MAX_WAIT_SECS),
        );
    loop {
        let row: Option<(Option<u32>, u8, Option<String>)> = conn
            .exec_first(
                "SELECT a.battlenet_account, a.online, ba.email \
                 FROM account a LEFT JOIN battlenet_accounts ba ON ba.id = a.battlenet_account \
                 WHERE a.id = ?",
                (bot.account_id,),
            )
            .map_err(|error| anyhow!("Wait for rested-XP game account offline: {error}"))?;
        let (battlenet_account_id, online, email) =
            row.ok_or_else(|| anyhow!("No auth.account row for id {}", bot.account_id))?;
        if battlenet_account_id != Some(fixture.battlenet_account_id)
            || !email
                .as_deref()
                .is_some_and(|email| email.eq_ignore_ascii_case(&bot.account))
        {
            bail!(
                "rested-XP fixture identity changed while waiting for account {} to disconnect",
                bot.account_id
            );
        }
        if online == 0 {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "game account {} remained online; refusing rested-XP cleanup mutation",
                bot.account_id
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
pub(crate) fn wait_for_rested_xp_target_respawn_cleanup(
    conn: &mut mysql::Conn,
    fixture: &RestedXpSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let target = &fixture.options.target;
    let wait_secs = rested_xp_respawn_cleanup_wait_secs(
        fixture.options.timeout_secs,
        fixture.target_respawn_secs,
    );
    let mut deadline = std::time::Instant::now() + Duration::from_secs(wait_secs);
    let mut absent_since = None;
    let mut last_respawn_time = None;
    let mut saw_persisted_respawn = false;
    loop {
        let respawn_time: Option<u64> = conn
            .exec_first(
                "SELECT respawnTime FROM respawn WHERE type = 0 AND spawnId = ? AND mapId = ? AND instanceId = 0",
                (target.spawn_guid, u32::from(target.map_id)),
            )
            .map_err(|error| anyhow!("Wait for rested-XP target respawn cleanup: {error}"))?;
        if let Some(respawn_time) = respawn_time {
            last_respawn_time = Some(respawn_time);
            absent_since = None;
            if !saw_persisted_respawn {
                info!(
                    "Rested-XP target persisted respawn row observed for spawn {} map {} at {}",
                    target.spawn_guid, target.map_id, respawn_time
                );
            }
            saw_persisted_respawn = true;
            let remaining =
                rested_xp_observed_respawn_remaining_secs(respawn_time, current_epoch_secs())?;
            deadline = deadline.max(std::time::Instant::now() + Duration::from_secs(remaining));
        } else {
            if saw_persisted_respawn {
                let absent_since = absent_since.get_or_insert_with(std::time::Instant::now);
                if absent_since.elapsed() >= Duration::from_secs(1) {
                    info!(
                        "Rested-XP target respawn row cleared naturally for spawn {} map {} after the persisted timer was observed",
                        target.spawn_guid, target.map_id
                    );
                    return Ok(());
                }
            }
        }

        if std::time::Instant::now() >= deadline {
            bail!(
                "rested-XP target respawn transition was not observed within the bounded wait (initial_wait={wait_secs}s, spawn={}, map={}, saw_persisted_respawn={}, last_respawnTime={:?}); the harness did not delete it, so wait for the runtime respawn before retrying",
                target.spawn_guid,
                target.map_id,
                saw_persisted_respawn,
                last_respawn_time
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
pub(crate) fn rested_xp_observed_respawn_remaining_secs(
    respawn_time: u64,
    now: u64,
) -> Result<u64> {
    let remaining = respawn_time
        .saturating_sub(now)
        .saturating_add(RESTED_XP_RESPAWN_GRACE_SECS);
    if remaining > MAX_RESTED_XP_RESPAWN_CLEANUP_WAIT_SECS {
        bail!(
            "observed rested-XP respawn timer requires {remaining}s, exceeding the {MAX_RESTED_XP_RESPAWN_CLEANUP_WAIT_SECS}s safety bound"
        );
    }
    Ok(remaining)
}
pub(crate) fn rested_xp_respawn_cleanup_wait_secs(
    protocol_timeout_secs: u64,
    respawn_secs: u32,
) -> u64 {
    protocol_timeout_secs
        .max(u64::from(respawn_secs).saturating_add(RESTED_XP_RESPAWN_GRACE_SECS))
        .clamp(
            10,
            u64::from(MAX_RESTED_XP_TARGET_RESPAWN_SECS)
                .saturating_add(RESTED_XP_RESPAWN_GRACE_SECS),
        )
}
pub(crate) fn cleanup_rested_xp_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &RestedXpSmokeFixture,
    verify_target_respawn: bool,
) -> Result<()> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing rested-XP fixture cleanup for non-local account {}",
            bot.account
        );
    }
    wait_for_rested_xp_character_offline_and_stable(bot, fixture.options.timeout_secs)?;
    wait_for_rested_xp_game_account_offline(bot, fixture)?;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let mut character_tx = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start rested-XP character cleanup transaction: {error}"))?;
    let (owner, online): (u32, u8) = character_tx
        .exec_first(
            "SELECT account, online FROM characters WHERE guid = ? FOR UPDATE",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Lock rested-XP cleanup character: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if owner != bot.account_id || online != 0 {
        bail!(
            "rested-XP cleanup character ownership/online state changed (owner={owner}, online={online})"
        );
    }
    let characters_on_game_account: u64 = character_tx
        .exec_first(
            "SELECT COUNT(*) FROM characters WHERE account = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Recheck rested-XP character exclusivity: {error}"))?
        .unwrap_or(0);
    if characters_on_game_account != 1 {
        bail!(
            "rested-XP cleanup requires the game account to remain exclusive; found {characters_on_game_account} characters"
        );
    }

    let auth_url = auth_db_url()?;
    let auth_opts =
        mysql::Opts::from_url(&auth_url).map_err(|error| anyhow!("Bad auth DB URL: {error}"))?;
    let mut auth = mysql::Conn::new(auth_opts)
        .map_err(|error| anyhow!("Connect to auth DB failed: {error}"))?;
    let mut auth_tx = auth
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start rested-XP auth cleanup transaction: {error}"))?;
    let (battlenet_account_id, game_account_online, battlenet_email): (
        Option<u32>,
        u8,
        Option<String>,
    ) = auth_tx
        .exec_first(
            "SELECT a.battlenet_account, a.online, ba.email \
             FROM account a LEFT JOIN battlenet_accounts ba ON ba.id = a.battlenet_account \
             WHERE a.id = ? FOR UPDATE",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Lock rested-XP cleanup game account: {error}"))?
        .ok_or_else(|| anyhow!("No auth.account row for id {}", bot.account_id))?;
    if battlenet_account_id != Some(fixture.battlenet_account_id)
        || game_account_online != 0
        || !battlenet_email
            .as_deref()
            .is_some_and(|email| email.eq_ignore_ascii_case(&bot.account))
    {
        bail!("rested-XP cleanup Battle.net identity or online state changed");
    }
    let game_accounts_on_bnet: u64 = auth_tx
        .exec_first(
            "SELECT COUNT(*) FROM account WHERE battlenet_account = ?",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Recheck rested-XP Battle.net exclusivity: {error}"))?
        .unwrap_or(0);
    if game_accounts_on_bnet != 1 {
        bail!(
            "rested-XP cleanup requires the Battle.net identity to remain exclusive; found {game_accounts_on_bnet} game accounts"
        );
    }

    character_tx
        .exec_drop(
            RESTED_XP_RESTORE_CHARACTER_SQL,
            mysql::Params::Positional(rested_xp_restore_params(
                &fixture.original,
                bot.character_guid,
            )),
        )
        .map_err(|error| anyhow!("Restore rested-XP selected character fields: {error}"))?;
    // A real C++ kill can update both achievement tables. Restore their exact
    // pre-smoke snapshots rather than assuming this disposable character had
    // no existing criteria or completed achievements.
    character_tx
        .exec_drop(
            "DELETE FROM character_achievement WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character achievements: {error}"))?;
    for (achievement, date) in &fixture.original_achievements {
        character_tx
            .exec_drop(
                "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)",
                (bot.character_guid, achievement, date),
            )
            .map_err(|error| anyhow!("Restore rested-XP character achievement: {error}"))?;
    }
    character_tx
        .exec_drop(
            "DELETE FROM character_achievement_progress WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character achievement progress: {error}"))?;
    for (criteria, counter, date) in &fixture.original_achievement_progress {
        character_tx
            .exec_drop(
                "INSERT INTO character_achievement_progress (guid, criteria, counter, date) VALUES (?, ?, ?, ?)",
                (bot.character_guid, criteria, counter, date),
            )
            .map_err(|error| anyhow!("Restore rested-XP achievement progress: {error}"))?;
    }
    // C++ Player::_LoadTraits may create missing per-specialization configs,
    // and SaveToDB persists them. Restore the exact pre-smoke snapshot so both
    // pre-existing builds and newly materialized defaults are handled safely.
    character_tx
        .exec_drop(
            "DELETE FROM character_trait_entry WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character trait entries: {error}"))?;
    character_tx
        .exec_drop(
            "DELETE FROM character_trait_config WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character trait configs: {error}"))?;
    for (
        trait_config_id,
        config_type,
        chr_specialization_id,
        combat_config_flags,
        local_identifier,
        skill_line_id,
        trait_system_id,
        name,
    ) in &fixture.original_trait_configs
    {
        character_tx
            .exec_drop(
                "INSERT INTO character_trait_config \
                 (guid, traitConfigId, type, chrSpecializationId, combatConfigFlags, \
                  localIdentifier, skillLineId, traitSystemId, name) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    bot.character_guid,
                    trait_config_id,
                    config_type,
                    chr_specialization_id,
                    combat_config_flags,
                    local_identifier,
                    skill_line_id,
                    trait_system_id,
                    name,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP character trait config: {error}"))?;
    }
    for (trait_config_id, trait_node_id, trait_node_entry_id, rank, granted_ranks) in
        &fixture.original_trait_entries
    {
        character_tx
            .exec_drop(
                "INSERT INTO character_trait_entry \
                 (guid, traitConfigId, traitNodeId, traitNodeEntryId, rank, grantedRanks) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                (
                    bot.character_guid,
                    trait_config_id,
                    trait_node_id,
                    trait_node_entry_id,
                    rank,
                    granted_ranks,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP character trait entry: {error}"))?;
    }
    character_tx
        .exec_drop(
            "DELETE FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character homebind: {error}"))?;
    if let Some((map_id, zone_id, x, y, z, orientation)) = fixture.original_homebind {
        character_tx
            .exec_drop(
                "INSERT INTO character_homebind \
                 (guid, mapId, zoneId, posX, posY, posZ, orientation) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                (bot.character_guid, map_id, zone_id, x, y, z, orientation),
            )
            .map_err(|error| anyhow!("Restore rested-XP character homebind: {error}"))?;
    }
    character_tx
        .exec_drop(
            "DELETE FROM character_fishingsteps WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character fishing steps: {error}"))?;
    if let Some(fishing_steps) = fixture.original_fishing_steps {
        character_tx
            .exec_drop(
                "INSERT INTO character_fishingsteps (guid, fishingSteps) VALUES (?, ?)",
                (bot.character_guid, fishing_steps),
            )
            .map_err(|error| anyhow!("Restore rested-XP character fishing steps: {error}"))?;
    }
    character_tx
        .exec_drop(
            "DELETE FROM character_battleground_data WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character battleground data: {error}"))?;
    if let Some((
        instance_id,
        team,
        join_x,
        join_y,
        join_z,
        join_o,
        join_map_id,
        taxi_start,
        taxi_end,
        mount_spell,
        queue_id,
    )) = fixture.original_battleground_data
    {
        character_tx
            .exec_drop(
                "INSERT INTO character_battleground_data \
                 (guid, instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, \
                  taxiStart, taxiEnd, mountSpell, queueId) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    bot.character_guid,
                    instance_id,
                    team,
                    join_x,
                    join_y,
                    join_z,
                    join_o,
                    join_map_id,
                    taxi_start,
                    taxi_end,
                    mount_spell,
                    queue_id,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP character battleground data: {error}"))?;
    }
    // Preflight proved these tables were empty. C++ login/save deterministically
    // creates them, so remove only the rows scoped to this disposable fixture.
    for (label, delete_sql, _) in RESTED_XP_CPP_GENERATED_CHARACTER_ROWS {
        character_tx
            .exec_drop(*delete_sql, (bot.character_guid,))
            .map_err(|error| anyhow!("Remove rested-XP fixture {label} rows: {error}"))?;
    }
    for (label, _, count_sql) in RESTED_XP_CPP_GENERATED_CHARACTER_ROWS {
        let rows: u64 = character_tx
            .exec_first(*count_sql, (bot.character_guid,))
            .map_err(|error| anyhow!("Verify rested-XP cleanup {label}: {error}"))?
            .unwrap_or(0);
        if rows != 0 {
            bail!("rested-XP cleanup left {rows} rows in {label}");
        }
    }
    let restored_row: mysql::Row = character_tx
        .exec_first(
            "SELECT level, xp, restState, playerFlags, rest_bonus, logout_time, is_logout_resting, \
             map, zone, instance_id, position_x, position_y, position_z, orientation, health, \
             power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, \
             totalKills, todayKills, yesterdayKills, totaltime, leveltime, latency, lastLoginBuild \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Reload restored rested-XP selected fields: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    let restored = rested_xp_character_restore_point_from_row(&restored_row)?;
    if restored != fixture.original {
        bail!("rested-XP cleanup verification did not reproduce the selected character fields");
    }
    let restored_achievements: Vec<(u32, i64)> = character_tx
        .exec(
            "SELECT achievement, date FROM character_achievement WHERE guid = ? ORDER BY achievement",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP character achievements: {error}"))?;
    if restored_achievements != fixture.original_achievements {
        bail!("rested-XP cleanup verification did not restore character achievements");
    }
    let restored_achievement_progress: Vec<(u32, u64, i64)> = character_tx
        .exec(
            "SELECT criteria, counter, date FROM character_achievement_progress WHERE guid = ? ORDER BY criteria",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP achievement progress: {error}"))?;
    if restored_achievement_progress != fixture.original_achievement_progress {
        bail!("rested-XP cleanup verification did not restore achievement progress");
    }
    let restored_trait_configs: Vec<RestedXpTraitConfigSnapshot> = character_tx
        .exec(RESTED_XP_SELECT_TRAIT_CONFIGS_SQL, (bot.character_guid,))
        .map_err(|error| anyhow!("Verify rested-XP character trait configs: {error}"))?;
    if restored_trait_configs != fixture.original_trait_configs {
        bail!("rested-XP cleanup verification did not restore character trait configs");
    }
    let restored_trait_entries: Vec<RestedXpTraitEntrySnapshot> = character_tx
        .exec(RESTED_XP_SELECT_TRAIT_ENTRIES_SQL, (bot.character_guid,))
        .map_err(|error| anyhow!("Verify rested-XP character trait entries: {error}"))?;
    if restored_trait_entries != fixture.original_trait_entries {
        bail!("rested-XP cleanup verification did not restore character trait entries");
    }
    let restored_homebind: Option<RestedXpHomebindSnapshot> = character_tx
        .exec_first(
            "SELECT mapId, zoneId, posX, posY, posZ, orientation \
             FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP character homebind: {error}"))?;
    if restored_homebind != fixture.original_homebind {
        bail!("rested-XP cleanup verification did not restore character homebind");
    }
    let restored_fishing_steps: Option<u8> = character_tx
        .exec_first(
            "SELECT fishingSteps FROM character_fishingsteps WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP character fishing steps: {error}"))?;
    if restored_fishing_steps != fixture.original_fishing_steps {
        bail!("rested-XP cleanup verification did not restore character fishing steps");
    }
    let restored_battleground_data: Option<RestedXpBattlegroundDataSnapshot> = character_tx
        .exec_first(
            "SELECT instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, \
                    taxiStart, taxiEnd, mountSpell, queueId \
             FROM character_battleground_data WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP character battleground data: {error}"))?;
    if restored_battleground_data != fixture.original_battleground_data {
        bail!("rested-XP cleanup verification did not restore character battleground data");
    }

    auth_tx
        .exec_drop(
            "DELETE FROM account_last_played_character WHERE accountId = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Clear rested-XP last-played character rows: {error}"))?;
    for (region, battlegroup, realm_id, name, character_guid, last_played_time) in
        &fixture.original_last_played_characters
    {
        auth_tx
            .exec_drop(
                "INSERT INTO account_last_played_character \
                 (accountId, region, battlegroup, realmId, characterName, characterGUID, lastPlayedTime) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                (
                    bot.account_id,
                    region,
                    battlegroup,
                    realm_id,
                    name,
                    character_guid,
                    last_played_time,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP last-played character row: {error}"))?;
    }
    auth_tx
        .exec_drop(
            "DELETE FROM battle_pet_slots WHERE battlenetAccountId = ?",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Clear rested-XP Battle.net pet slots: {error}"))?;
    for (slot_id, battle_pet_guid, locked) in &fixture.original_battle_pet_slots {
        auth_tx
            .exec_drop(
                "INSERT INTO battle_pet_slots \
                 (id, battlenetAccountId, battlePetGuid, locked) VALUES (?, ?, ?, ?)",
                (
                    slot_id,
                    fixture.battlenet_account_id,
                    battle_pet_guid,
                    locked,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP Battle.net pet slot: {error}"))?;
    }
    auth_tx
        .exec_drop(
            "DELETE FROM battlenet_account_transmog_illusions WHERE battlenetAccountId = ?",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Remove rested-XP fixture illusion rows: {error}"))?;
    let illusion_rows: u64 = auth_tx
        .exec_first(
            "SELECT COUNT(*) FROM battlenet_account_transmog_illusions WHERE battlenetAccountId = ?",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Verify rested-XP illusion cleanup: {error}"))?
        .unwrap_or(0);
    if illusion_rows != 0 {
        bail!("rested-XP cleanup left {illusion_rows} transmog illusion rows");
    }
    let restored_last_played_characters: Vec<RestedXpLastPlayedCharacterSnapshot> = auth_tx
        .exec(
            "SELECT region, battlegroup, realmId, characterName, characterGUID, lastPlayedTime \
             FROM account_last_played_character WHERE accountId = ? \
             ORDER BY region, battlegroup",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Verify rested-XP last-played character rows: {error}"))?;
    if restored_last_played_characters != fixture.original_last_played_characters {
        bail!("rested-XP cleanup verification did not restore last-played character rows");
    }
    let restored_battle_pet_slots: Vec<RestedXpBattlePetSlotSnapshot> = auth_tx
        .exec(
            "SELECT id, battlePetGuid, locked FROM battle_pet_slots \
             WHERE battlenetAccountId = ? ORDER BY id",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Verify rested-XP Battle.net pet slots: {error}"))?;
    if restored_battle_pet_slots != fixture.original_battle_pet_slots {
        bail!("rested-XP cleanup verification did not restore Battle.net pet slots");
    }

    character_tx
        .commit()
        .map_err(|error| anyhow!("Commit rested-XP character cleanup: {error}"))?;
    auth_tx
        .commit()
        .map_err(|error| anyhow!("Commit rested-XP auth cleanup: {error}"))?;
    info!(
        "Rested-XP fixture restored character/account snapshots and deterministic glyph/reputation/skill/illusion save rows for character {}",
        bot.character_guid,
    );
    if verify_target_respawn {
        wait_for_rested_xp_target_respawn_cleanup(&mut conn, fixture)?;
    } else {
        info!(
            "Rested-XP workflow did not prove a target kill; skipped the inapplicable respawn-transition wait"
        );
    }
    Ok(())
}
pub(crate) fn issue20_expected_enchantment_ids() -> [i32; ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT] {
    let mut enchantments = [0; ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT];
    enchantments[0] = ISSUE20_ITEM_PERMANENT_ENCHANT_ID;
    enchantments[ISSUE20_ITEM_RANDOM_PROPERTY_SLOT] = ISSUE20_ITEM_RANDOM_PROPERTY_ENCHANT_ID;
    enchantments
}
pub(crate) fn issue20_zero_enchantments_db_string() -> String {
    vec!["0"; ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT * 3].join(" ")
}

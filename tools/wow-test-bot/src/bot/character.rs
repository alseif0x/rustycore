//! Character operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn characters_db_url() -> Result<String> {
    database_url("WOW_BOT_CHAR_DB_URL", "CharacterDatabaseInfo")
}
pub(crate) fn validate_local_bot_character_owner(
    conn: &mut mysql::Conn,
    bot: &config::BotConfig,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let (owner, online, at_login) = conn
        .exec_first::<(u32, u8, u16), _, _>(
            "SELECT account, online, at_login FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Lookup character {}: {e}", bot.character_guid))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;

    if owner != bot.account_id {
        bail!(
            "Refusing to reassign character GUID {} from account {} to configured test account {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 || at_login != 0 {
        bail!(
            "Character GUID {} is not an offline clean fixture (online={online}, at_login={at_login})",
            bot.character_guid
        );
    }

    Ok(())
}
pub(crate) fn rested_xp_character_restore_point_from_row(
    row: &mysql::Row,
) -> Result<RestedXpCharacterRestorePoint> {
    Ok(RestedXpCharacterRestorePoint {
        level: required_row_value(row, "level")?,
        xp: required_row_value(row, "xp")?,
        rest_state: required_row_value(row, "restState")?,
        player_flags: required_row_value(row, "playerFlags")?,
        rest_bonus: required_row_value(row, "rest_bonus")?,
        logout_time: required_row_value(row, "logout_time")?,
        is_logout_resting: required_row_value(row, "is_logout_resting")?,
        map_id: required_row_value(row, "map")?,
        zone_id: required_row_value(row, "zone")?,
        instance_id: required_row_value(row, "instance_id")?,
        x: required_row_value(row, "position_x")?,
        y: required_row_value(row, "position_y")?,
        z: required_row_value(row, "position_z")?,
        orientation: required_row_value(row, "orientation")?,
        health: required_row_value(row, "health")?,
        powers: [
            required_row_value(row, "power1")?,
            required_row_value(row, "power2")?,
            required_row_value(row, "power3")?,
            required_row_value(row, "power4")?,
            required_row_value(row, "power5")?,
            required_row_value(row, "power6")?,
            required_row_value(row, "power7")?,
            required_row_value(row, "power8")?,
            required_row_value(row, "power9")?,
            required_row_value(row, "power10")?,
        ],
        total_kills: required_row_value(row, "totalKills")?,
        today_kills: required_row_value(row, "todayKills")?,
        yesterday_kills: required_row_value(row, "yesterdayKills")?,
        total_time: required_row_value(row, "totaltime")?,
        level_time: required_row_value(row, "leveltime")?,
        latency: required_row_value(row, "latency")?,
        last_login_build: required_row_value(row, "lastLoginBuild")?,
    })
}
pub(crate) fn prepare_rested_xp_character_phase(
    bot: &config::BotConfig,
    fixture: &RestedXpSmokeFixture,
    phase: RestedXpSmokePhase,
) -> Result<()> {
    use mysql::prelude::Queryable;

    if phase == RestedXpSmokePhase::VerifyRelog {
        return wait_for_rested_xp_character_offline_and_stable(bot, fixture.options.timeout_secs);
    }
    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start rested-XP phase transaction: {error}"))?;
    let online: u8 = transaction
        .exec_first(
            "SELECT online FROM characters WHERE guid = ? FOR UPDATE",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Lock rested-XP character row: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if online != 0 {
        bail!(
            "character {} remained online before {:?}; refusing DB mutation",
            bot.character_guid,
            phase
        );
    }
    let (rest_state, rest_bonus, logout_time, is_logout_resting) = match phase {
        RestedXpSmokePhase::OfflineWilderness => (
            REST_STATE_NORMAL,
            0.0,
            current_epoch_secs().saturating_sub(fixture.offline_secs),
            0u8,
        ),
        RestedXpSmokePhase::OfflineResting => (
            REST_STATE_NORMAL,
            0.0,
            current_epoch_secs().saturating_sub(fixture.offline_secs),
            1u8,
        ),
        RestedXpSmokePhase::ConsumeKill => (
            REST_STATE_RESTED,
            fixture.options.seeded_rest_bonus,
            current_epoch_secs(),
            0,
        ),
        RestedXpSmokePhase::VerifyRelog => unreachable!(),
    };
    let player_flags =
        fixture.original.player_flags & !(PLAYER_FLAGS_RESTING | PLAYER_FLAGS_NO_XP_GAIN);
    let player_x = fixture.options.target.x + 1.0;
    let player_y = fixture.options.target.y;
    let player_z = fixture.options.target.z;
    let player_orientation =
        (fixture.options.target.y - player_y).atan2(fixture.options.target.x - player_x) as f32;
    transaction
        .exec_drop(
            "UPDATE characters SET level = ?, xp = 0, restState = ?, playerFlags = ?, rest_bonus = ?, \
             logout_time = ?, is_logout_resting = ?, map = ?, zone = 0, instance_id = 0, \
             position_x = ?, position_y = ?, position_z = ?, orientation = ?, health = ? \
             WHERE guid = ? AND online = 0",
            mysql::Params::Positional(vec![
                fixture.test_level.into(),
                rest_state.into(),
                player_flags.into(),
                rest_bonus.into(),
                logout_time.into(),
                is_logout_resting.into(),
                u32::from(fixture.options.target.map_id).into(),
                player_x.into(),
                player_y.into(),
                player_z.into(),
                player_orientation.into(),
                u32::MAX.into(),
                bot.character_guid.into(),
            ]),
        )
        .map_err(|error| anyhow!("Prepare rested-XP character phase {phase:?}: {error}"))?;
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit rested-XP phase {phase:?}: {error}"))?;
    Ok(())
}
pub(crate) fn wait_for_rested_xp_character_offline_and_stable(
    bot: &config::BotConfig,
    timeout_secs: u64,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let deadline = std::time::Instant::now()
        + Duration::from_secs(timeout_secs.clamp(10, RESTED_XP_DISCONNECT_SAVE_MAX_WAIT_SECS));
    let mut previous_offline_marker: Option<(u32, u32, u64, u8)> = None;
    loop {
        let row: Option<(u8, u32, f32, u64, u8)> = conn
            .exec_first(
                "SELECT online, xp, rest_bonus, logout_time, is_logout_resting FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Wait for rested-XP disconnect save: {error}"))?;
        let (online, xp, rest_bonus, logout_time, is_logout_resting) =
            row.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
        if online == 0 {
            let marker = (xp, rest_bonus.to_bits(), logout_time, is_logout_resting);
            if previous_offline_marker == Some(marker) {
                return Ok(());
            }
            previous_offline_marker = Some(marker);
        } else {
            previous_offline_marker = None;
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "character {} did not reach a stable offline DB state; refusing selected-field restore",
                bot.character_guid
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
pub(crate) fn set_bot_character_level(
    conn: &mut mysql::Conn,
    character_guid: u64,
    level: u8,
) -> Result<()> {
    use mysql::prelude::Queryable;

    conn.exec_drop(
        "UPDATE characters SET level = ? WHERE guid = ?",
        (level, character_guid),
    )
    .map_err(|e| anyhow!("UPDATE characters.level: {}", e))?;
    Ok(())
}
pub(crate) fn set_bot_character_race_class(
    conn: &mut mysql::Conn,
    character_guid: u64,
    race: Option<u8>,
    class: Option<u8>,
) -> Result<()> {
    use mysql::prelude::Queryable;

    match (race, class) {
        (Some(race), Some(class)) => conn
            .exec_drop(
                "UPDATE characters SET race = ?, class = ? WHERE guid = ?",
                (race, class, character_guid),
            )
            .map_err(|e| anyhow!("UPDATE characters.race/class: {}", e))?,
        (Some(race), None) => conn
            .exec_drop(
                "UPDATE characters SET race = ? WHERE guid = ?",
                (race, character_guid),
            )
            .map_err(|e| anyhow!("UPDATE characters.race: {}", e))?,
        (None, Some(class)) => conn
            .exec_drop(
                "UPDATE characters SET class = ? WHERE guid = ?",
                (class, character_guid),
            )
            .map_err(|e| anyhow!("UPDATE characters.class: {}", e))?,
        (None, None) => {}
    }
    Ok(())
}

//! Sql fixtures operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn qa_mysql_opts(url: &str, label: &str) -> Result<mysql::Opts> {
    let opts = mysql::Opts::from_url(url).map_err(|e| anyhow!("Bad {label} DB URL: {e}"))?;
    Ok(mysql::OptsBuilder::from_opts(opts)
        .tcp_connect_timeout(Some(Duration::from_secs(10)))
        .read_timeout(Some(Duration::from_secs(30)))
        .write_timeout(Some(Duration::from_secs(30)))
        .into())
}
pub(crate) fn equipment_set_db_row_from_mysql(row: &mysql::Row) -> Result<EquipmentSetDbRow> {
    Ok(EquipmentSetDbRow {
        set_guid: required_row_value(row, "setguid")?,
        set_index: required_row_value(row, "setindex")?,
        name: required_row_value(row, "name")?,
        icon_name: required_row_value(row, "iconname")?,
        ignore_mask: required_row_value(row, "ignore_mask")?,
        assigned_spec_index: required_row_value(row, "AssignedSpecIndex")?,
        items: required_indexed_row_values(row, "item")?,
    })
}
pub(crate) fn transmog_outfit_db_row_from_mysql(row: &mysql::Row) -> Result<TransmogOutfitDbRow> {
    Ok(TransmogOutfitDbRow {
        set_guid: required_row_value(row, "setguid")?,
        set_index: required_row_value(row, "setindex")?,
        name: required_row_value(row, "name")?,
        icon_name: required_row_value(row, "iconname")?,
        ignore_mask: required_row_value(row, "ignore_mask")?,
        appearances: required_indexed_row_values(row, "appearance")?,
        main_hand_enchant: required_row_value(row, "mainHandEnchant")?,
        off_hand_enchant: required_row_value(row, "offHandEnchant")?,
    })
}
pub(crate) const RESTED_XP_RESTORE_CHARACTER_SQL: &str =
    "UPDATE characters SET level = ?, xp = ?, restState = ?, playerFlags = ?, rest_bonus = ?, \
     logout_time = ?, is_logout_resting = ?, map = ?, zone = ?, instance_id = ?, \
     position_x = ?, position_y = ?, position_z = ?, orientation = ?, health = ?, \
     power1 = ?, power2 = ?, power3 = ?, power4 = ?, power5 = ?, power6 = ?, power7 = ?, \
     power8 = ?, power9 = ?, power10 = ?, totalKills = ?, todayKills = ?, yesterdayKills = ?, \
     totaltime = ?, leveltime = ?, latency = ?, lastLoginBuild = ? \
     WHERE guid = ? AND online = 0";

pub(crate) const RESTED_XP_SELECT_TRAIT_CONFIGS_SQL: &str =
    "SELECT traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, \
     skillLineId, traitSystemId, name FROM character_trait_config \
     WHERE guid = ? ORDER BY traitConfigId";
pub(crate) const RESTED_XP_SELECT_TRAIT_ENTRIES_SQL: &str =
    "SELECT traitConfigId, traitNodeId, traitNodeEntryId, rank, grantedRanks \
     FROM character_trait_entry WHERE guid = ? \
     ORDER BY traitConfigId, traitNodeId, traitNodeEntryId";

// Stock C++ login/save materializes these defaults for an otherwise clean
// character. The rested-XP fixture requires them empty before the smoke, so
// cleanup must remove the same bounded, character-owned rows afterwards.
pub(crate) const RESTED_XP_CPP_GENERATED_CHARACTER_ROWS: &[(&str, &str, &str)] = &[
    (
        "character_glyphs",
        "DELETE FROM character_glyphs WHERE guid = ?",
        "SELECT COUNT(*) FROM character_glyphs WHERE guid = ?",
    ),
    (
        "character_reputation",
        "DELETE FROM character_reputation WHERE guid = ?",
        "SELECT COUNT(*) FROM character_reputation WHERE guid = ?",
    ),
    (
        "character_skills",
        "DELETE FROM character_skills WHERE guid = ?",
        "SELECT COUNT(*) FROM character_skills WHERE guid = ?",
    ),
];

pub(crate) fn prepare_rested_xp_smoke_fixture(
    bot: &config::BotConfig,
    creature_entry: u32,
    creature_spawn_guid: Option<u64>,
    runtime_counter: Option<u64>,
    offline_secs: u64,
    timeout_secs: u64,
) -> Result<RestedXpSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing rested-XP fixture setup for non-local account {}",
            bot.account
        );
    }
    if let Some(counter) = runtime_counter {
        if counter == 0 || counter > OBJECT_GUID_COUNTER_MASK {
            bail!(
                "rested-XP runtime counter override {counter} must fit the nonzero 40-bit ObjectGuid counter field"
            );
        }
    }
    let stat_save_min_level = worldserver_config_u32("PlayerSave.Stats.MinLevel", 0)?;
    if stat_save_min_level != 0 {
        bail!(
            "rested-XP fixture requires PlayerSave.Stats.MinLevel=0, got {stat_save_min_level}; stock C++ otherwise destructively rewrites character_stats on logout"
        );
    }
    let start_all_spells = worldserver_config_u32("PlayerStart.AllSpells", 0)?;
    if start_all_spells != 0 {
        bail!(
            "rested-XP fixture requires PlayerStart.AllSpells=0, got {start_all_spells}; otherwise stock C++ can populate character_spell during login"
        );
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let character_row: mysql::Row = characters
        .exec_first(
            "SELECT account, online, at_login, level, xp, restState, playerFlags, rest_bonus, logout_time, \
             is_logout_resting, map, zone, instance_id, position_x, position_y, position_z, \
             orientation, health, power1, power2, power3, power4, power5, power6, power7, power8, \
             power9, power10, totalKills, todayKills, yesterdayKills, totaltime, leveltime, \
             latency, lastLoginBuild \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load rested-XP bot character: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    let owner: u32 = required_row_value(&character_row, "account")?;
    let online: u8 = required_row_value(&character_row, "online")?;
    let at_login: u16 = required_row_value(&character_row, "at_login")?;
    if owner != bot.account_id {
        bail!(
            "character {} belongs to account {}, expected {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 {
        bail!(
            "character {} is online; refusing rested-XP fixture setup",
            bot.character_guid
        );
    }
    let characters_on_game_account = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM characters WHERE account = ?",
        u64::from(bot.account_id),
        "characters on game account",
    )?;
    let mut safety_state = RestedXpFixtureSafetyState {
        at_login,
        characters_on_game_account,
        ..RestedXpFixtureSafetyState::default()
    };
    for (table, sql) in [
        (
            "character_inventory",
            "SELECT COUNT(*) FROM character_inventory WHERE guid = ?",
        ),
        (
            "character_pet",
            "SELECT COUNT(*) FROM character_pet WHERE owner = ?",
        ),
        (
            "character_aura",
            "SELECT COUNT(*) FROM character_aura WHERE guid = ?",
        ),
        (
            "character_aura_effect",
            "SELECT COUNT(*) FROM character_aura_effect WHERE guid = ?",
        ),
        (
            "character_spell_cooldown",
            "SELECT COUNT(*) FROM character_spell_cooldown WHERE guid = ?",
        ),
        (
            "character_spell_charges",
            "SELECT COUNT(*) FROM character_spell_charges WHERE guid = ?",
        ),
        (
            "character_skills",
            "SELECT COUNT(*) FROM character_skills WHERE guid = ?",
        ),
        (
            "character_glyphs",
            "SELECT COUNT(*) FROM character_glyphs WHERE guid = ?",
        ),
        (
            "character_talent",
            "SELECT COUNT(*) FROM character_talent WHERE guid = ?",
        ),
        (
            "character_spell",
            "SELECT COUNT(*) FROM character_spell WHERE guid = ?",
        ),
        (
            "character_spell_favorite",
            "SELECT COUNT(*) FROM character_spell_favorite WHERE guid = ?",
        ),
        (
            "character_action",
            "SELECT COUNT(*) FROM character_action WHERE guid = ?",
        ),
        (
            "character_reputation",
            "SELECT COUNT(*) FROM character_reputation WHERE guid = ?",
        ),
        (
            "character_equipmentsets",
            "SELECT COUNT(*) FROM character_equipmentsets WHERE guid = ?",
        ),
        (
            "character_transmog_outfits",
            "SELECT COUNT(*) FROM character_transmog_outfits WHERE guid = ?",
        ),
        (
            "character_cuf_profiles",
            "SELECT COUNT(*) FROM character_cuf_profiles WHERE guid = ?",
        ),
        (
            "character_void_storage",
            "SELECT COUNT(*) FROM character_void_storage WHERE playerGuid = ?",
        ),
        (
            "guild_member",
            "SELECT COUNT(*) FROM guild_member WHERE guid = ?",
        ),
        ("corpse", "SELECT COUNT(*) FROM corpse WHERE guid = ?"),
    ] {
        let rows = rested_xp_count_rows(&mut characters, sql, bot.character_guid, table)?;
        if rows != 0 {
            safety_state
                .nonempty_side_state
                .push((table.to_string(), rows));
        }
    }
    let instance_lock_rows = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM account_instance_times WHERE accountId = ?",
        u64::from(bot.account_id),
        "account_instance_times",
    )?;
    if instance_lock_rows != 0 {
        safety_state
            .nonempty_side_state
            .push(("account_instance_times".to_string(), instance_lock_rows));
    }
    let tutorial_rows = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM account_tutorial WHERE accountId = ?",
        u64::from(bot.account_id),
        "account_tutorial",
    )?;
    if tutorial_rows != 0 {
        safety_state
            .nonempty_side_state
            .push(("account_tutorial".to_string(), tutorial_rows));
    }
    let original = rested_xp_character_restore_point_from_row(&character_row)?;
    let original_achievements: Vec<(u32, i64)> = characters
        .exec(
            "SELECT achievement, date FROM character_achievement WHERE guid = ? ORDER BY achievement",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character achievements: {error}"))?;
    let original_achievement_progress: Vec<(u32, u64, i64)> = characters
        .exec(
            "SELECT criteria, counter, date FROM character_achievement_progress WHERE guid = ? ORDER BY criteria",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character achievement progress: {error}"))?;
    // Stock C++ can materialize missing specialization trait configs during
    // login/save. Preserve these tables exactly instead of assuming the
    // disposable fixture started with no trait state.
    let original_trait_configs: Vec<RestedXpTraitConfigSnapshot> = characters
        .exec(RESTED_XP_SELECT_TRAIT_CONFIGS_SQL, (bot.character_guid,))
        .map_err(|error| anyhow!("Snapshot rested-XP character trait configs: {error}"))?;
    let original_trait_entries: Vec<RestedXpTraitEntrySnapshot> = characters
        .exec(RESTED_XP_SELECT_TRAIT_ENTRIES_SQL, (bot.character_guid,))
        .map_err(|error| anyhow!("Snapshot rested-XP character trait entries: {error}"))?;
    let original_homebind: Option<RestedXpHomebindSnapshot> = characters
        .exec_first(
            "SELECT mapId, zoneId, posX, posY, posZ, orientation \
             FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character homebind: {error}"))?;
    let original_fishing_steps: Option<u8> = characters
        .exec_first(
            "SELECT fishingSteps FROM character_fishingsteps WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character fishing steps: {error}"))?;
    let original_battleground_data: Option<RestedXpBattlegroundDataSnapshot> = characters
        .exec_first(
            "SELECT instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, \
                    taxiStart, taxiEnd, mountSpell, queueId \
             FROM character_battleground_data WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character battleground data: {error}"))?;

    let active_quests: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_queststatus WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Check rested-XP active quests: {error}"))?
        .unwrap_or(0);
    let active_objectives: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_queststatus_objectives WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Check rested-XP quest objectives: {error}"))?
        .unwrap_or(0);
    let active_criteria = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM character_queststatus_objectives_criteria WHERE guid = ?",
        bot.character_guid,
        "character_queststatus_objectives_criteria",
    )?;
    let active_criteria_progress = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM character_queststatus_objectives_criteria_progress WHERE guid = ?",
        bot.character_guid,
        "character_queststatus_objectives_criteria_progress",
    )?;
    if active_quests != 0
        || active_objectives != 0
        || active_criteria != 0
        || active_criteria_progress != 0
    {
        bail!(
            "character {} has active quest state ({active_quests} quests/{active_objectives} objectives/{active_criteria} criteria/{active_criteria_progress} criteria progress); use a clean @bot.local character so the kill cannot mutate quest progress",
            bot.character_guid
        );
    }
    let group_rows: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM group_member WHERE memberGuid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Check rested-XP group membership: {error}"))?
        .unwrap_or(0);
    if group_rows != 0 {
        bail!(
            "character {} is in a persisted group; refusing ambiguous rested/RAF XP QA",
            bot.character_guid
        );
    }

    let auth_url = auth_db_url()?;
    let auth_opts =
        mysql::Opts::from_url(&auth_url).map_err(|error| anyhow!("Bad auth DB URL: {error}"))?;
    let mut auth = mysql::Conn::new(auth_opts)
        .map_err(|error| anyhow!("Connect to auth DB failed: {error}"))?;
    let (recruiter, battlenet_account_id, game_account_online, battlenet_email): (
        u32,
        Option<u32>,
        u8,
        Option<String>,
    ) = auth
        .exec_first(
            "SELECT a.recruiter, a.battlenet_account, a.online, ba.email \
             FROM account a LEFT JOIN battlenet_accounts ba ON ba.id = a.battlenet_account \
             WHERE a.id = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Check rested-XP account scope: {error}"))?
        .ok_or_else(|| anyhow!("No auth.account row for id {}", bot.account_id))?;
    let battlenet_account_id = battlenet_account_id.ok_or_else(|| {
        anyhow!(
            "auth.account {} has no Battle.net identity; refusing disposable rested-XP fixture",
            bot.account_id
        )
    })?;
    let original_last_played_characters: Vec<RestedXpLastPlayedCharacterSnapshot> = auth
        .exec(
            "SELECT region, battlegroup, realmId, characterName, characterGUID, lastPlayedTime \
             FROM account_last_played_character WHERE accountId = ? \
             ORDER BY region, battlegroup",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP last-played character rows: {error}"))?;
    let original_battle_pet_slots: Vec<RestedXpBattlePetSlotSnapshot> = auth
        .exec(
            "SELECT id, battlePetGuid, locked FROM battle_pet_slots \
             WHERE battlenetAccountId = ? ORDER BY id",
            (battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP Battle.net pet slots: {error}"))?;
    safety_state.game_account_online = game_account_online;
    safety_state.bnet_email_matches_configured_account = battlenet_email
        .as_deref()
        .is_some_and(|email| email.eq_ignore_ascii_case(&bot.account));
    safety_state.game_accounts_on_bnet_account = rested_xp_count_rows(
        &mut auth,
        "SELECT COUNT(*) FROM account WHERE battlenet_account = ?",
        u64::from(battlenet_account_id),
        "game accounts on Battle.net identity",
    )?;
    for (table, sql) in [
        (
            "battlenet_account_mounts",
            "SELECT COUNT(*) FROM battlenet_account_mounts WHERE battlenetAccountId = ?",
        ),
        (
            "battlenet_account_toys",
            "SELECT COUNT(*) FROM battlenet_account_toys WHERE accountId = ?",
        ),
        (
            "battlenet_account_heirlooms",
            "SELECT COUNT(*) FROM battlenet_account_heirlooms WHERE accountId = ?",
        ),
        (
            "battlenet_item_appearances",
            "SELECT COUNT(*) FROM battlenet_item_appearances WHERE battlenetAccountId = ?",
        ),
        (
            "battlenet_item_favorite_appearances",
            "SELECT COUNT(*) FROM battlenet_item_favorite_appearances WHERE battlenetAccountId = ?",
        ),
        (
            "battlenet_account_transmog_illusions",
            "SELECT COUNT(*) FROM battlenet_account_transmog_illusions WHERE battlenetAccountId = ?",
        ),
        (
            "battle_pets",
            "SELECT COUNT(*) FROM battle_pets WHERE battlenetAccountId = ?",
        ),
    ] {
        let rows = rested_xp_count_rows(
            &mut auth,
            sql,
            u64::from(battlenet_account_id),
            table,
        )?;
        if rows != 0 {
            safety_state
                .nonempty_side_state
                .push((table.to_string(), rows));
        }
    }
    let recruited_accounts: u64 = auth
        .exec_first(
            "SELECT COUNT(*) FROM account WHERE recruiter = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Check rested-XP recruited accounts: {error}"))?
        .unwrap_or(0);
    if recruiter != 0 || recruited_accounts != 0 {
        bail!(
            "account {} participates in Recruit-A-Friend; refusing a rested-XP test that could award 300% XP",
            bot.account_id
        );
    }
    validate_rested_xp_fixture_safety_state(&safety_state)?;

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|error| anyhow!("Bad world DB URL: {error}"))?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let target_row: mysql::Row = if let Some(spawn_guid) = creature_spawn_guid {
        world
            .exec_first(
                "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation, \
                 c.wander_distance, c.spawntimesecs AS SpawnTimeSecs, COALESCE(d.MinLevel, 1) AS MinLevel, \
                 COALESCE(d.MaxLevel, 1) AS MaxLevel, ct.type AS CreatureType, \
                 ct.VehicleId, ct.flags_extra, \
                 COALESCE(d.StaticFlags1, 0) AS StaticFlags1 \
                 FROM creature c JOIN creature_template ct ON ct.entry = c.id \
                 LEFT JOIN creature_template_difficulty d ON d.Entry = c.id AND d.DifficultyID = 0 \
                 WHERE c.guid = ? AND c.id = ?",
                (spawn_guid, creature_entry),
            )
            .map_err(|error| anyhow!("Resolve rested-XP target spawn: {error}"))?
    } else {
        world
            .exec_first(
                "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation, \
                 c.wander_distance, c.spawntimesecs AS SpawnTimeSecs, COALESCE(d.MinLevel, 1) AS MinLevel, \
                 COALESCE(d.MaxLevel, 1) AS MaxLevel, ct.type AS CreatureType, \
                 ct.VehicleId, ct.flags_extra, \
                 COALESCE(d.StaticFlags1, 0) AS StaticFlags1 \
                 FROM creature c JOIN creature_template ct ON ct.entry = c.id \
                 LEFT JOIN creature_template_difficulty d ON d.Entry = c.id AND d.DifficultyID = 0 \
                 WHERE c.id = ? ORDER BY c.guid LIMIT 1",
                (creature_entry,),
            )
            .map_err(|error| anyhow!("Resolve rested-XP target entry: {error}"))?
    }
    .ok_or_else(|| {
        anyhow!(
            "No world.creature spawn for rested-XP entry {}{}",
            creature_entry,
            creature_spawn_guid
                .map(|guid| format!(" and guid {guid}"))
                .unwrap_or_default()
        )
    })?;
    let spawn_guid: u64 = required_row_value(&target_row, "guid")?;
    let entry: u32 = required_row_value(&target_row, "id")?;
    let map_id_u32: u32 = required_row_value(&target_row, "map")?;
    let map_id = u16::try_from(map_id_u32)
        .map_err(|_| anyhow!("rested-XP target map {map_id_u32} does not fit protocol u16"))?;
    let x: f64 = required_row_value(&target_row, "position_x")?;
    let y: f64 = required_row_value(&target_row, "position_y")?;
    let z: f64 = required_row_value(&target_row, "position_z")?;
    let orientation: f32 = required_row_value(&target_row, "orientation")?;
    let wander_distance: f32 = required_row_value(&target_row, "wander_distance")?;
    let target_match_radius = wander_distance.max(0.0) + 2.0;
    let target_respawn_secs: u32 = required_row_value(&target_row, "SpawnTimeSecs")?;
    let min_level: u8 = required_row_value(&target_row, "MinLevel")?;
    let max_level: u8 = required_row_value(&target_row, "MaxLevel")?;
    let creature_type: u8 = required_row_value(&target_row, "CreatureType")?;
    let vehicle_id: u32 = required_row_value(&target_row, "VehicleId")?;
    let flags_extra: u32 = required_row_value(&target_row, "flags_extra")?;
    let static_flags_1: u32 = required_row_value(&target_row, "StaticFlags1")?;
    if runtime_counter.is_none() {
        let overlapping_spawn: Option<u64> = world
            .exec_first(
                "SELECT guid FROM creature \
                 WHERE id = ? AND map = ? AND guid <> ? \
                   AND SQRT(POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) \
                       <= ? + GREATEST(wander_distance, 0) \
                 ORDER BY guid LIMIT 1",
                (entry, map_id_u32, spawn_guid, x, y, z, target_match_radius),
            )
            .map_err(|error| anyhow!("Check rested-XP target spawn ambiguity: {error}"))?;
        if let Some(overlapping_spawn) = overlapping_spawn {
            bail!(
                "rested-XP SQL spawn {spawn_guid} has an overlapping same-entry movement radius with spawn {overlapping_spawn}; set --rested-xp-runtime-counter from a trusted live discovery or choose an isolated spawn"
            );
        }
    }
    validate_rested_xp_target_template(entry, creature_type, vehicle_id)?;
    if !(MIN_RESTED_XP_TARGET_RESPAWN_SECS..=MAX_RESTED_XP_TARGET_RESPAWN_SECS)
        .contains(&target_respawn_secs)
    {
        bail!(
            "rested-XP target entry {entry} has an unsuitable {target_respawn_secs}s respawn; choose a disposable target in {MIN_RESTED_XP_TARGET_RESPAWN_SECS}..={MAX_RESTED_XP_TARGET_RESPAWN_SECS}s so the harness can observe the persisted timer before it clears"
        );
    }
    if min_level == 0 || max_level == 0 || min_level > max_level || max_level > 6 {
        bail!(
            "rested-XP target {} has nondeterministic/unsafe level range {}..={}; choose a level 1..=6 creature",
            entry,
            min_level,
            max_level
        );
    }
    if flags_extra & CREATURE_FLAG_EXTRA_NO_XP != 0
        || static_flags_1 & CREATURE_STATIC_FLAG_NO_XP != 0
    {
        bail!("rested-XP target entry {entry} is marked NO_XP");
    }
    let reputation_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM creature_onkill_reputation WHERE creature_id = ?",
            (entry,),
        )
        .map_err(|error| anyhow!("Check rested-XP target reputation side effects: {error}"))?
        .unwrap_or(0);
    if reputation_rows != 0 {
        bail!("rested-XP target entry {entry} has on-kill reputation; choose an isolated target");
    }

    let pending_respawn: Option<u64> = characters
        .exec_first(
            "SELECT respawnTime FROM respawn WHERE type = 0 AND spawnId = ? AND mapId = ? AND instanceId = 0",
            (spawn_guid, map_id_u32),
        )
        .map_err(|error| anyhow!("Check rested-XP target respawn timer: {error}"))?;
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    if let Some(respawn_time) = pending_respawn {
        bail!(
            "rested-XP target spawn {spawn_guid} already has persisted respawn row {respawn_time} (now={now}); refusing to overwrite world state"
        );
    }

    let test_level = max_level;
    let next_level_xp: u32 = world
        .exec_first(
            "SELECT Experience FROM player_xp_for_level WHERE Level = ?",
            (test_level,),
        )
        .map_err(|error| anyhow!("Load rested-XP next-level threshold: {error}"))?
        .ok_or_else(|| anyhow!("No player_xp_for_level row for level {test_level}"))?;
    if next_level_xp == 0 {
        bail!("level {test_level} has zero next-level XP; cannot test rested XP");
    }
    let wilderness_rate = worldserver_config_f32("Rate.Rest.Offline.InWilderness", 1.0)?;
    let resting_rate = worldserver_config_f32("Rate.Rest.Offline.InTavernOrCity", 1.0)?;
    let expected_wilderness = offline_rest_bonus_like_cpp(
        next_level_xp,
        offline_secs,
        REST_OFFLINE_WILDERNESS_BUBBLE,
        wilderness_rate,
    );
    let expected_resting = offline_rest_bonus_like_cpp(
        next_level_xp,
        offline_secs,
        REST_OFFLINE_TAVERN_OR_CITY_BUBBLE,
        resting_rate,
    );
    if expected_wilderness <= 0.0 || expected_resting <= expected_wilderness {
        bail!(
            "configured rest rates/interval cannot prove resting>wilderness: wilderness={expected_wilderness:.4}, resting={expected_resting:.4}"
        );
    }

    let guid_counter = runtime_counter.unwrap_or(0);
    let packed_guid = if guid_counter == 0 {
        Vec::new()
    } else {
        let (low, high) = create_creature_guid_raw(map_id, entry, guid_counter);
        build_packed_guid(low, high)
    };
    let target = ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id,
        x,
        y,
        z,
        orientation,
        packed_guid,
    };
    let seeded_rest_bonus = next_level_xp as f32 * REST_BONUS_CAP_NEXT_LEVEL_FACTOR;
    info!(
        "Rested-XP fixture ready: character={} target={}/{} map={} level={} nextXP={} rates={}/{}",
        bot.character_guid,
        entry,
        spawn_guid,
        map_id,
        test_level,
        next_level_xp,
        wilderness_rate,
        resting_rate
    );
    Ok(RestedXpSmokeFixture {
        options: RestedXpSmokeOptions {
            phase: RestedXpSmokePhase::OfflineWilderness,
            target,
            target_match_radius,
            test_level,
            next_level_xp,
            seeded_rest_bonus,
            expected_xp: None,
            expected_rest_bonus: None,
            timeout_secs,
        },
        original,
        original_achievements,
        original_achievement_progress,
        original_trait_configs,
        original_trait_entries,
        original_homebind,
        original_fishing_steps,
        original_battleground_data,
        original_last_played_characters,
        original_battle_pet_slots,
        battlenet_account_id,
        target_respawn_secs,
        test_level,
        offline_secs,
        wilderness_rate,
        resting_rate,
    })
}

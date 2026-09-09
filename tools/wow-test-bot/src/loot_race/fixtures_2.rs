//! Loot-race fixtures operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

pub(crate) fn snapshot_loot_fixture_characters(
    character_db: &mut mysql::Conn,
    bots: &[config::BotConfig],
    item_entry: u32,
    target_max_level: u8,
    maximum_money_gain: u32,
    required_empty_top_level_slot: Option<(u64, u8)>,
) -> Result<([CharacterFixture; 2], CharacterProgressSnapshot)> {
    let mut fixtures = Vec::with_capacity(2);
    for bot in bots {
        let character: mysql::Row = character_db
            .exec_first(
                "SELECT account, name, race, level, xp, health, \
                        power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, \
                        restState, rest_bonus, exploredZones, knownTitles, chosenTitle, \
                        online, at_login, money, map, zone, instance_id, \
                        position_x, position_y, position_z, orientation \
                 FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Load loot-race character {}: {error}", bot.character_guid))?
            .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
        let account: u32 = required_row_value(&character, "account")?;
        let name: String = required_row_value(&character, "name")?;
        let race: u8 = required_row_value(&character, "race")?;
        let level: u8 = required_row_value(&character, "level")?;
        let xp: u32 = required_row_value(&character, "xp")?;
        let health: u32 = required_row_value(&character, "health")?;
        let powers = [
            required_row_value(&character, "power1")?,
            required_row_value(&character, "power2")?,
            required_row_value(&character, "power3")?,
            required_row_value(&character, "power4")?,
            required_row_value(&character, "power5")?,
            required_row_value(&character, "power6")?,
            required_row_value(&character, "power7")?,
            required_row_value(&character, "power8")?,
            required_row_value(&character, "power9")?,
            required_row_value(&character, "power10")?,
        ];
        let rest_state: u8 = required_row_value(&character, "restState")?;
        let rest_bonus: f32 = required_row_value(&character, "rest_bonus")?;
        let explored_zones: Option<String> = required_row_value(&character, "exploredZones")?;
        let known_titles: Option<String> = required_row_value(&character, "knownTitles")?;
        let chosen_title: u32 = required_row_value(&character, "chosenTitle")?;
        let online: u8 = required_row_value(&character, "online")?;
        let at_login: u16 = required_row_value(&character, "at_login")?;
        let money: u64 = required_row_value(&character, "money")?;
        let old_map: u32 = required_row_value(&character, "map")?;
        let zone: u32 = required_row_value(&character, "zone")?;
        let instance_id: u32 = required_row_value(&character, "instance_id")?;
        let old_x: f64 = required_row_value(&character, "position_x")?;
        let old_y: f64 = required_row_value(&character, "position_y")?;
        let old_z: f64 = required_row_value(&character, "position_z")?;
        let orientation: f32 = required_row_value(&character, "orientation")?;
        if account != bot.account_id || online != 0 || at_login != 0 {
            bail!(
                "loot-race character {} owner/online/at_login safety check failed",
                bot.character_guid
            );
        }
        if health == 0 || level <= target_max_level {
            bail!(
                "loot-race character {} must be alive and above target level {}",
                bot.character_guid,
                target_max_level
            );
        }
        let maximum_safe_start = MAX_PLAYER_MONEY_LIKE_CPP
            .checked_sub(u64::from(maximum_money_gain))
            .ok_or_else(|| anyhow!("loot-race creature maximum gold exceeds the player cap"))?;
        if money > maximum_safe_start {
            bail!(
                "loot-race character {} lacks headroom for the fixture's maximum money roll",
                bot.character_guid
            );
        }
        let account_chars: u64 = character_db
            .exec_first(
                "SELECT COUNT(*) FROM characters WHERE account = ?",
                (bot.account_id,),
            )
            .map_err(|error| anyhow!("Count dedicated loot-race characters: {error}"))?
            .unwrap_or(0);
        if account_chars != 1 {
            bail!(
                "loot-race requires one dedicated character per game account; {} has {account_chars}",
                bot.account
            );
        }
        let group_rows: u64 = character_db
            .exec_first(
                "SELECT COUNT(*) FROM (\
                    SELECT guid FROM group_member WHERE memberGuid = ? \
                    UNION ALL \
                    SELECT guid FROM `groups` WHERE leaderGuid = ?\
                 ) AS persisted_group_state",
                (bot.character_guid, bot.character_guid),
            )
            .map_err(|error| anyhow!("Check loot-race group state: {error}"))?
            .unwrap_or(0);
        if group_rows != 0 {
            bail!(
                "loot-race character {} is already in a persisted group",
                bot.character_guid
            );
        }
        let persistent_auras: u64 = character_db
            .exec_first(
                "SELECT COUNT(*) FROM character_aura WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Check loot-race persistent auras: {error}"))?
            .unwrap_or(0);
        if persistent_auras != 0 {
            bail!(
                "loot-race character {} has {persistent_auras} persistent aura row(s); money modifiers must be absent",
                bot.character_guid
            );
        }
        let owned_expected: u64 = character_db
            .exec_first(
                "SELECT COUNT(*) FROM item_instance WHERE owner_guid = ? AND itemEntry = ?",
                (bot.character_guid, item_entry),
            )
            .map_err(|error| anyhow!("Check existing loot-race item: {error}"))?
            .unwrap_or(0);
        if owned_expected != 0 {
            bail!(
                "loot-race character {} already owns item entry {}",
                bot.character_guid,
                item_entry
            );
        }
        let occupied: Vec<u8> = character_db
            .exec_map(
                "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
                (bot.character_guid,),
                |slot: u8| slot,
            )
            .map_err(|error| anyhow!("Load loot-race backpack slots: {error}"))?;
        if !(INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
            .any(|slot| !occupied.contains(&slot))
        {
            bail!(
                "loot-race character {} has no empty backpack slot",
                bot.character_guid
            );
        }
        validate_required_empty_top_level_slot(
            bot.character_guid,
            &occupied,
            required_empty_top_level_slot,
        )?;
        fixtures.push(CharacterFixture {
            bot: bot.clone(),
            name,
            race,
            money,
            core: CharacterCoreSnapshot {
                level,
                xp,
                health,
                powers,
                rest_state,
                rest_bonus,
                explored_zones,
                known_titles,
                chosen_title,
            },
            position: CharacterPositionSnapshot {
                map_id: old_map,
                zone_id: zone,
                instance_id,
                x: old_x,
                y: old_y,
                z: old_z,
                orientation,
            },
        });
    }
    if faction_for_race(fixtures[0].race) != faction_for_race(fixtures[1].race) {
        bail!(
            "loot-race characters must be the same faction so C++ party invite rules permit grouping"
        );
    }
    let fixture_characters: [CharacterFixture; 2] = fixtures
        .try_into()
        .map_err(|_| anyhow!("internal loot-race character count mismatch"))?;
    let guid_a = fixture_characters[0].bot.character_guid;
    let guid_b = fixture_characters[1].bot.character_guid;
    let guild_members: u64 = character_db
        .exec_first(
            "SELECT COUNT(*) FROM guild_member WHERE guid IN (?, ?)",
            (guid_a, guid_b),
        )
        .map_err(|error| anyhow!("Check loot-race guild criteria isolation: {error}"))?
        .unwrap_or(0);
    if guild_members != 0 {
        bail!(
            "loot-race disposable characters have {guild_members} guild membership row(s); a kill/item criteria update could mutate guild-wide state"
        );
    }
    let progress = load_character_progress_snapshot(character_db, guid_a, guid_b)?;
    Ok((fixture_characters, progress))
}
pub(crate) fn relocate_loot_fixture_characters(
    character_db: &mut mysql::Conn,
    fixture_characters: &[CharacterFixture; 2],
    map_id: u16,
    x: f64,
    y: f64,
    z: f64,
) -> Result<()> {
    let mut tx = character_db
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start loot-race relocation transaction: {error}"))?;
    // C++ Player::LoadFromDB and Rust `restored_saved_health_like_cpp` both
    // clamp this sentinel to the recomputed max health on login. The original
    // health remains in CharacterCoreSnapshot and cleanup restores it exactly;
    // powers are intentionally untouched.
    for (index, fixture) in fixture_characters.iter().enumerate() {
        let player_x = x + 1.0 + index as f64;
        let player_orientation = 0.0_f64.atan2(x - player_x);
        tx.exec_drop(
            "UPDATE characters SET map = :new_map, zone = 0, instance_id = 0, \
                    position_x = :new_x, position_y = :new_y, position_z = :new_z, \
                    orientation = :new_orientation, health = :new_health \
             WHERE guid = :guid AND account = :account AND online = 0 AND at_login = 0 \
               AND map = :old_map AND zone = :old_zone AND instance_id = :old_instance \
               AND position_x = :old_x AND position_y = :old_y AND position_z = :old_z \
               AND orientation = :old_orientation AND health = :old_health",
            mysql::params! {
                "new_map" => u32::from(map_id),
                "new_x" => player_x,
                "new_y" => y,
                "new_z" => z,
                "new_orientation" => player_orientation,
                "new_health" => u32::MAX,
                "guid" => fixture.bot.character_guid,
                "account" => fixture.bot.account_id,
                "old_map" => fixture.position.map_id,
                "old_zone" => fixture.position.zone_id,
                "old_instance" => fixture.position.instance_id,
                "old_x" => fixture.position.x,
                "old_y" => fixture.position.y,
                "old_z" => fixture.position.z,
                "old_orientation" => fixture.position.orientation,
                "old_health" => fixture.core.health,
            },
        )
        .map_err(|error| anyhow!("Relocate loot-race character: {error}"))?;
        if tx.affected_rows() != 1 {
            bail!(
                "loot-race character {} drifted after its durable snapshot; relocation applied to {} rows",
                fixture.bot.character_guid,
                tx.affected_rows()
            );
        }
    }
    tx.commit()
        .map_err(|error| anyhow!("Commit loot-race relocation: {error}"))?;
    Ok(())
}
pub(crate) fn cleanup_fixture(fixture: &LootRaceFixture) -> Result<()> {
    let url = characters_db_url()?;
    let opts = loot_db_opts(&url, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    wait_both_offline(&mut conn, fixture)?;
    let guid_a = fixture.characters[0].bot.character_guid;
    let guid_b = fixture.characters[1].bot.character_guid;
    let gained_items: Vec<u64> = conn
        .exec_map(
            "SELECT guid FROM item_instance WHERE itemEntry = ? AND owner_guid IN (?, ?)",
            (fixture.target.item_entry, guid_a, guid_b),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Load gained loot-race items for cleanup: {error}"))?;
    let group_ids: Vec<u32> = conn
        .exec_map(
            "SELECT DISTINCT guid FROM group_member WHERE memberGuid IN (?, ?) \
             UNION SELECT guid FROM `groups` WHERE leaderGuid IN (?, ?)",
            (guid_a, guid_b, guid_a, guid_b),
            |guid: u32| guid,
        )
        .map_err(|error| anyhow!("Load loot-race groups for cleanup: {error}"))?;
    for group_id in &group_ids {
        let unrelated_members: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM group_member \
                 WHERE guid = ? AND memberGuid NOT IN (?, ?)",
                (*group_id, guid_a, guid_b),
            )
            .map_err(|error| anyhow!("Check loot-race group cleanup scope: {error}"))?
            .unwrap_or(0);
        if unrelated_members != 0 {
            bail!(
                "refusing to delete loot-race group {group_id}: it gained {unrelated_members} unrelated members"
            );
        }
    }
    if let Some(expected_state) = fixture.gameobject_state {
        let world_url = world_db_url()?;
        let world_opts = loot_db_opts(&world_url, "world")?;
        let mut world = mysql::Conn::new(world_opts).map_err(|error| {
            anyhow!("Connect to world DB during GameObject cleanup failed: {error}")
        })?;
        let observed: Option<u8> = world
            .exec_first(
                "SELECT state FROM gameobject WHERE guid = ? AND id = ?",
                (fixture.target.spawn_guid, fixture.target.entry),
            )
            .map_err(|error| anyhow!("Verify GameObject fixture SQL state: {error}"))?;
        if observed != Some(expected_state) {
            bail!(
                "GameObject fixture SQL state drifted: expected {expected_state}, got {observed:?}; refusing to hide drift or restore normal PM2"
            );
        }
    }
    let observed_respawn: Vec<(i64, u16, u32)> = conn
        .exec(
            "SELECT respawnTime, mapId, instanceId FROM respawn \
             WHERE type = ? AND spawnId = ? ORDER BY mapId, instanceId, respawnTime",
            (fixture.respawn_type, fixture.target.spawn_guid),
        )
        .map_err(|error| anyhow!("Inspect loot-fixture respawn rows before cleanup: {error}"))?;
    let baseline_respawn = fixture
        .respawn
        .iter()
        .map(|row| (row.respawn_time, row.map_id, row.instance_id))
        .collect::<Vec<_>>();
    let generated_respawn = validate_respawn_cleanup_scope(
        fixture.target.map_id,
        &baseline_respawn,
        &observed_respawn,
    )?;
    let mut tx = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start loot-race cleanup transaction: {error}"))?;
    for item_guid in gained_items {
        tx.exec_drop(
            "DELETE FROM character_inventory WHERE item = ?",
            (item_guid,),
        )
        .map_err(|error| anyhow!("Delete loot-race inventory row: {error}"))?;
        tx.exec_drop(
            "DELETE FROM item_instance_gems WHERE itemGuid = ?",
            (item_guid,),
        )
        .map_err(|error| anyhow!("Delete loot-race gem row: {error}"))?;
        tx.exec_drop("DELETE FROM item_instance WHERE guid = ?", (item_guid,))
            .map_err(|error| anyhow!("Delete loot-race item row: {error}"))?;
    }
    for group_id in group_ids {
        tx.exec_drop("DELETE FROM group_member WHERE guid = ?", (group_id,))
            .map_err(|error| anyhow!("Delete loot-race group members: {error}"))?;
        tx.exec_drop("DELETE FROM `groups` WHERE guid = ?", (group_id,))
            .map_err(|error| anyhow!("Delete loot-race group row: {error}"))?;
    }
    restore_character_progress_snapshot(&mut tx, guid_a, guid_b, &fixture.progress)?;
    for character in &fixture.characters {
        tx.exec_drop(
            "UPDATE characters SET \
                    money = :money, level = :level, xp = :xp, health = :health, \
                    power1 = :power1, power2 = :power2, power3 = :power3, \
                    power4 = :power4, power5 = :power5, power6 = :power6, \
                    power7 = :power7, power8 = :power8, power9 = :power9, power10 = :power10, \
                    restState = :rest_state, rest_bonus = :rest_bonus, \
                    exploredZones = :explored_zones, knownTitles = :known_titles, \
                    chosenTitle = :chosen_title, \
                    map = :map, zone = :zone, instance_id = :instance_id, \
                    position_x = :position_x, position_y = :position_y, \
                    position_z = :position_z, orientation = :orientation \
             WHERE guid = :guid AND online = 0",
            mysql::params! {
                "money" => character.money,
                "level" => character.core.level,
                "xp" => character.core.xp,
                "health" => character.core.health,
                "power1" => character.core.powers[0],
                "power2" => character.core.powers[1],
                "power3" => character.core.powers[2],
                "power4" => character.core.powers[3],
                "power5" => character.core.powers[4],
                "power6" => character.core.powers[5],
                "power7" => character.core.powers[6],
                "power8" => character.core.powers[7],
                "power9" => character.core.powers[8],
                "power10" => character.core.powers[9],
                "rest_state" => character.core.rest_state,
                "rest_bonus" => character.core.rest_bonus,
                "explored_zones" => character.core.explored_zones.clone(),
                "known_titles" => character.core.known_titles.clone(),
                "chosen_title" => character.core.chosen_title,
                "map" => character.position.map_id,
                "zone" => character.position.zone_id,
                "instance_id" => character.position.instance_id,
                "position_x" => character.position.x,
                "position_y" => character.position.y,
                "position_z" => character.position.z,
                "orientation" => character.position.orientation,
                "guid" => character.bot.character_guid,
            },
        )
        .map_err(|error| anyhow!("Restore loot-race character snapshot: {error}"))?;
        if tx.affected_rows() != 1 {
            bail!(
                "loot-race character {} became online or disappeared during cleanup",
                character.bot.character_guid
            );
        }
    }
    if fixture.target.kind == LootRaceTargetKind::Creature {
        if let Some((respawn_time, map_id, instance_id)) = generated_respawn {
            tx.exec_drop(
                "DELETE FROM respawn \
                 WHERE type = ? AND spawnId = ? AND respawnTime = ? AND mapId = ? AND instanceId = ?",
                (
                    fixture.respawn_type,
                    fixture.target.spawn_guid,
                    respawn_time,
                    map_id,
                    instance_id,
                ),
            )
            .map_err(|error| anyhow!("Delete exact generated loot respawn row: {error}"))?;
            if tx.affected_rows() != 1 {
                bail!(
                    "exact generated respawn row drifted before cleanup; deleted {} rows",
                    tx.affected_rows()
                );
            }
        }
    }
    tx.commit()
        .map_err(|error| anyhow!("Commit loot-race cleanup: {error}"))?;

    ensure_no_online_characters(&mut conn, "loot fixture cleanup")?;
    let remaining_expected_items: u64 = conn
        .exec_first(
            "SELECT COUNT(*) FROM item_instance WHERE itemEntry = ? AND owner_guid IN (?, ?)",
            (fixture.target.item_entry, guid_a, guid_b),
        )
        .map_err(|error| anyhow!("Verify loot-fixture item cleanup: {error}"))?
        .unwrap_or(0);
    if remaining_expected_items != 0 {
        bail!(
            "loot fixture cleanup left {remaining_expected_items} expected-item instance row(s) behind"
        );
    }
    let remaining_groups: u64 = conn
        .exec_first(
            "SELECT COUNT(*) FROM (\
                SELECT guid FROM group_member WHERE memberGuid IN (?, ?) \
                UNION ALL \
                SELECT guid FROM `groups` WHERE leaderGuid IN (?, ?)\
             ) AS persisted_group_state",
            (guid_a, guid_b, guid_a, guid_b),
        )
        .map_err(|error| anyhow!("Verify loot-fixture group cleanup: {error}"))?
        .unwrap_or(0);
    if remaining_groups != 0 {
        bail!("loot fixture cleanup left {remaining_groups} persisted group row(s) behind");
    }
    if fixture.target.kind == LootRaceTargetKind::Creature {
        let occupied_capture_slot: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM character_inventory WHERE guid = ? AND bag = 0 AND slot = ?",
                (guid_a, LOOT_ITEM_CAPTURE_KEYRING_SLOT),
            )
            .map_err(|error| anyhow!("Verify loot-item keyring-slot cleanup: {error}"))?
            .unwrap_or(0);
        if occupied_capture_slot != 0 {
            bail!(
                "loot fixture cleanup did not restore exact empty keyring slot 0/{LOOT_ITEM_CAPTURE_KEYRING_SLOT}"
            );
        }
    }
    let restored_respawn: Vec<(i64, u16, u32)> = conn
        .exec(
            "SELECT respawnTime, mapId, instanceId FROM respawn \
             WHERE type = ? AND spawnId = ? ORDER BY mapId, instanceId",
            (fixture.respawn_type, fixture.target.spawn_guid),
        )
        .map_err(|error| anyhow!("Verify loot-fixture respawn cleanup: {error}"))?;
    let expected_respawn = if fixture.target.kind == LootRaceTargetKind::GameObject {
        observed_respawn
    } else {
        baseline_respawn
    };
    if restored_respawn != expected_respawn {
        bail!("loot fixture target respawn rows did not restore exactly");
    }
    let restored_progress = load_character_progress_snapshot(&mut conn, guid_a, guid_b)?;
    if restored_progress != fixture.progress {
        bail!("loot fixture quest/achievement/criteria/reputation rows did not restore exactly");
    }
    for character in &fixture.characters {
        let restored_core = load_character_core_snapshot(&mut conn, character.bot.character_guid)?;
        if restored_core != character.core {
            bail!(
                "loot fixture character {} XP/level/health/rest/exploration/title state did not restore exactly",
                character.bot.character_guid
            );
        }
        let restored_position_and_money: (u64, u32, u32, u32, f64, f64, f64, f32) = conn
            .exec_first(
                "SELECT money, map, zone, instance_id, position_x, position_y, position_z, orientation \
                 FROM characters WHERE guid = ?",
                (character.bot.character_guid,),
            )
            .map_err(|error| anyhow!("Reload loot-fixture money/position state: {error}"))?
            .ok_or_else(|| {
                anyhow!(
                    "Loot-fixture character {} disappeared during cleanup verification",
                    character.bot.character_guid
                )
            })?;
        let expected_position_and_money = (
            character.money,
            character.position.map_id,
            character.position.zone_id,
            character.position.instance_id,
            character.position.x,
            character.position.y,
            character.position.z,
            character.position.orientation,
        );
        if restored_position_and_money != expected_position_and_money {
            bail!(
                "loot fixture character {} money/position state did not restore exactly",
                character.bot.character_guid
            );
        }
    }

    fixture.journal.complete()?;
    Ok(())
}
pub(crate) fn validate_respawn_cleanup_scope(
    target_map: u16,
    baseline: &[(i64, u16, u32)],
    observed: &[(i64, u16, u32)],
) -> Result<Option<(i64, u16, u32)>> {
    if observed == baseline {
        return Ok(None);
    }
    if !baseline.is_empty() || observed.len() != 1 {
        bail!(
            "loot fixture respawn drift: baseline={baseline:?}, observed={observed:?}; refusing broad cleanup"
        );
    }
    let row = observed[0];
    if row.0 <= 0 || row.1 != target_map || row.2 != 0 {
        bail!(
            "loot fixture generated respawn has wrong time/map/instance: {row:?}, expected positive/{target_map}/0"
        );
    }
    Ok(Some(row))
}
pub(crate) fn load_group_capacity_fixture(
    bots: &[config::BotConfig],
    cli: &GroupCapacityRaceCli,
) -> Result<GroupCapacityFixture> {
    if cli.group_db_store_id == 0 {
        bail!("group-capacity race requires a nonzero preloaded --group-capacity-group-id");
    }
    if cli.timeout_secs == 0 {
        bail!("--group-capacity-timeout must be greater than zero");
    }

    let find_bot = |account: &str| {
        bots.iter()
            .find(|bot| bot.account.eq_ignore_ascii_case(account))
            .cloned()
            .ok_or_else(|| anyhow!("group-capacity account {account} was not selected"))
    };
    let leader = find_bot(&cli.leader_account)?;
    let candidate_a = find_bot(&cli.candidate_a_account)?;
    let candidate_b = find_bot(&cli.candidate_b_account)?;
    let selected_guids = [
        leader.character_guid,
        candidate_a.character_guid,
        candidate_b.character_guid,
    ];
    if selected_guids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != 3
    {
        bail!("group-capacity leader and candidates must be three distinct characters");
    }

    let opts = qa_mysql_opts(&characters_db_url()?, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let (
        loaded_leader,
        group_type,
        loot_method,
        loot_threshold,
        dungeon_difficulty_id,
        raid_difficulty_id,
        legacy_raid_difficulty_id,
        master_looter_guid,
    ): (u64, u16, u8, u8, u32, u32, u32, u64) = conn
        .exec_first(
            "SELECT leaderGuid, groupType, lootMethod, lootThreshold, difficulty, raidDifficulty, legacyRaidDifficulty, masterLooterGuid FROM `groups` WHERE guid = ?",
            (cli.group_db_store_id,),
        )
        .map_err(|error| anyhow!("Load preseeded group-capacity group: {error}"))?
        .ok_or_else(|| {
            anyhow!(
                "preseeded group-capacity group {} does not exist; seed it before starting world-server",
                cli.group_db_store_id
            )
        })?;
    if loaded_leader != leader.character_guid || group_type != 0 {
        bail!(
            "group-capacity fixture must be a normal party led by GUID {}; found leader={loaded_leader} groupType={group_type}",
            leader.character_guid
        );
    }

    let initial_members: Vec<u64> = conn
        .exec(
            "SELECT memberGuid FROM group_member WHERE guid = ? ORDER BY memberGuid",
            (cli.group_db_store_id,),
        )
        .map_err(|error| anyhow!("Load preseeded group-capacity members: {error}"))?;
    let initial_member_guids: [u64; 4] =
        initial_members.try_into().map_err(|members: Vec<u64>| {
            anyhow!(
                "group-capacity fixture must start with exactly four members, found {:?}",
                members
            )
        })?;
    if !initial_member_guids.contains(&leader.character_guid)
        || initial_member_guids.contains(&candidate_a.character_guid)
        || initial_member_guids.contains(&candidate_b.character_guid)
    {
        bail!(
            "group-capacity fixture must contain the leader and exclude both candidates; members={initial_member_guids:?}"
        );
    }

    let load_character = |conn: &mut mysql::Conn, guid: u64| -> Result<(String, u8, u8)> {
        conn.exec_first(
            "SELECT name, race, online FROM characters WHERE guid = ?",
            (guid,),
        )
        .map_err(|error| anyhow!("Load group-capacity character {guid}: {error}"))?
        .ok_or_else(|| anyhow!("group-capacity character {guid} does not exist"))
    };
    for member_guid in initial_member_guids {
        let (_, _, online) = load_character(&mut conn, member_guid)?;
        validate_group_capacity_initial_member_offline(member_guid, online)?;
    }

    let leader_character = load_character(&mut conn, leader.character_guid)?;
    let candidate_a_character = load_character(&mut conn, candidate_a.character_guid)?;
    let candidate_b_character = load_character(&mut conn, candidate_b.character_guid)?;
    if leader_character.2 != 0 || candidate_a_character.2 != 0 || candidate_b_character.2 != 0 {
        bail!("group-capacity leader and both candidates must be offline before the race");
    }
    let leader_faction = faction_for_race(leader_character.1);
    if faction_for_race(candidate_a_character.1) != leader_faction
        || faction_for_race(candidate_b_character.1) != leader_faction
    {
        bail!(
            "group-capacity leader and candidates must share a faction for deterministic invite acceptance; races were {}/{}/{}",
            leader_character.1,
            candidate_a_character.1,
            candidate_b_character.1
        );
    }

    Ok(GroupCapacityFixture {
        leader_guid: leader.character_guid,
        candidate_names: [candidate_a_character.0, candidate_b_character.0],
        candidate_guids: [candidate_a.character_guid, candidate_b.character_guid],
        initial_member_guids,
        party_settings: GroupCapacityPartySettings {
            loot_method,
            loot_threshold,
            master_looter_guid,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
        },
    })
}
pub(crate) fn verify_group_capacity_fixture(
    cli: &GroupCapacityRaceCli,
    fixture: &GroupCapacityFixture,
) -> Result<GroupCapacityPersistenceEvidence> {
    let opts = qa_mysql_opts(&characters_db_url()?, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let group_identity: Option<(u64, u16)> = conn
        .exec_first(
            "SELECT leaderGuid, groupType FROM `groups` WHERE guid = ?",
            (cli.group_db_store_id,),
        )
        .map_err(|error| anyhow!("Verify group-capacity group row: {error}"))?;
    if group_identity != Some((fixture.leader_guid, 0)) {
        bail!(
            "group-capacity group row changed during the race: {group_identity:?}; expected leader {} and normal groupType 0",
            fixture.leader_guid
        );
    }
    let final_members: Vec<u64> = conn
        .exec(
            "SELECT memberGuid FROM group_member WHERE guid = ? ORDER BY memberGuid",
            (cli.group_db_store_id,),
        )
        .map_err(|error| anyhow!("Verify group-capacity member rows: {error}"))?;
    validate_group_capacity_persisted_members(fixture, &final_members)
}

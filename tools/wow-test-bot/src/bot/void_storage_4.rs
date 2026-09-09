//! Void storage operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn prepare_void_storage_smoke_fixture(
    bot: &config::BotConfig,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<VoidStorageSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive void-storage fixture setup for non-local account {}",
            bot.account
        );
    }
    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let character: Option<(u32, u8, u64, u32, u32, u32, u32, f64, f64, f64, f32)> = characters
        .exec_first(
            "SELECT account, online, money, playerFlags, map, zone, instance_id, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load void-storage bot character: {error}"))?;
    let (
        owner,
        online,
        original_money,
        original_player_flags,
        map_id,
        zone_id,
        instance_id,
        x,
        y,
        z,
        orientation,
    ) = character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
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
            "character {} is online; log it out before void-storage smoke setup",
            bot.character_guid
        );
    }
    let existing_void_items: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_void_storage WHERE playerGuid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Check existing void-storage rows: {error}"))?
        .unwrap_or(0);
    if existing_void_items != 0 {
        bail!(
            "character {} already has {existing_void_items} void-storage rows; use an empty disposable bot",
            bot.character_guid
        );
    }
    let same_entry_count: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, item_entry),
        )
        .map_err(|error| anyhow!("Check existing void-storage fixture item entry: {error}"))?
        .unwrap_or(0);
    if same_entry_count != 0 {
        bail!(
            "bot character already owns item entry {item_entry}; choose another --void-storage-item-entry"
        );
    }
    let occupied_slots: Vec<u8> = characters
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (bot.character_guid,),
            |slot: u8| slot,
        )
        .map_err(|error| anyhow!("Load occupied void-storage bot slots: {error}"))?;
    let inventory_slot = (INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
        .find(|slot| !occupied_slots.contains(slot))
        .ok_or_else(|| anyhow!("No empty default backpack slot for void-storage smoke"))?;
    let max_item_guid: u64 = characters
        .query_first("SELECT COALESCE(MAX(guid), 0) FROM item_instance")
        .map_err(|error| anyhow!("Load max item guid: {error}"))?
        .unwrap_or(0);
    let item_guid = max_item_guid
        .checked_add(10_000)
        .ok_or_else(|| anyhow!("item guid overflow while reserving void-storage fixture"))?;

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|error| anyhow!("Bad world DB URL: {error}"))?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let neutral: Option<(u64, u32, u32, f64, f64, f64, f32)> = world
        .exec_first(
            "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
             FROM creature c JOIN creature_template ct ON ct.entry = c.id \
             WHERE ct.faction = 35 \
               AND ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
               AND c.phaseid = 0 AND c.phasegroup = 0 \
               AND FIND_IN_SET('0', c.spawnDifficulties) > 0 \
               AND ct.VehicleId = 0 ORDER BY c.guid LIMIT 1",
            (NPC_FLAG_VAULT_KEEPER,),
        )
        .map_err(|error| anyhow!("Resolve neutral vault keeper: {error}"))?;
    let vault_row = match neutral {
        Some(row) => row,
        None => world
            .exec_first(
                "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
                 FROM creature c JOIN creature_template ct ON ct.entry = c.id \
                 WHERE ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
                 ORDER BY c.guid LIMIT 1",
                (NPC_FLAG_VAULT_KEEPER,),
            )
            .map_err(|error| anyhow!("Resolve fallback vault keeper: {error}"))?
            .ok_or_else(|| anyhow!("No vault-keeper creature spawn exists in world DB"))?,
    };
    let (spawn_guid, entry, vault_map, vault_x, vault_y, vault_z, vault_orientation) = vault_row;
    let vault_map = u16::try_from(vault_map)
        .map_err(|_| anyhow!("vault-keeper map id does not fit protocol: {vault_map}"))?;
    let runtime_realm_id = void_storage_runtime_realm_id()?;
    let discover_runtime_guid = runtime_counter.is_none();
    let guid_counter = runtime_counter.unwrap_or(spawn_guid);
    let (low, high) =
        create_void_storage_creature_guid_raw(vault_map, entry, guid_counter, runtime_realm_id);
    let vault_keeper = ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id: vault_map,
        x: vault_x,
        y: vault_y,
        z: vault_z,
        orientation: vault_orientation,
        packed_guid: build_packed_guid(low, high),
    };

    let fixture_money = original_money.max(
        VOID_STORAGE_UNLOCK_COST
            .saturating_add(VOID_STORAGE_STORE_ITEM_COST)
            .saturating_add(10_000),
    );
    let fixture_flags = original_player_flags & !PLAYER_FLAGS_VOID_UNLOCKED;
    let mut transaction = characters
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start void-storage fixture transaction: {error}"))?;
    transaction
        .exec_drop(
            "INSERT INTO item_instance \
             (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, durability, \
              enchantments, charges, flags, randomPropertiesId, randomPropertiesSeed, context) \
             VALUES (?, ?, ?, 0, 0, 1, 0, '', '', 0, 0, 0, 0)",
            (item_guid, item_entry, bot.character_guid),
        )
        .map_err(|error| anyhow!("Insert void-storage fixture item: {error}"))?;
    transaction
        .exec_drop(
            "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)",
            (bot.character_guid, inventory_slot, item_guid),
        )
        .map_err(|error| anyhow!("Insert void-storage fixture inventory row: {error}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET money = ?, playerFlags = ?, map = ?, zone = 0, instance_id = 0, \
             position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
                fixture_money,
                fixture_flags,
                u32::from(vault_map),
                vault_x + 2.0,
                vault_y,
                vault_z,
                vault_orientation,
                bot.character_guid,
            ),
        )
        .map_err(|error| anyhow!("Relocate and seed void-storage bot: {error}"))?;
    if transaction.affected_rows() != 1 {
        bail!("void-storage fixture lost its offline character guard");
    }
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit void-storage fixture transaction: {error}"))?;

    info!(
        "Void-storage fixture: character={} item={}/entry={} slot={} vault={}/{} runtime_counter={}",
        bot.character_guid,
        item_guid,
        item_entry,
        inventory_slot,
        entry,
        spawn_guid,
        guid_counter
    );
    Ok(VoidStorageSmokeFixture {
        options: VoidStorageSmokeOptions {
            phase: VoidStorageSmokePhase::UnlockDeposit,
            vault_keeper,
            runtime_realm_id,
            discover_runtime_guid,
            fixture_item_guid: item_guid,
            item_entry,
            inventory_slot,
            expected_void_item_id: None,
            expected_void_slot: 0,
            timeout_secs,
        },
        original_position: CharacterPositionSnapshot {
            map_id,
            zone_id,
            instance_id,
            x,
            y,
            z,
            orientation,
        },
        original_money,
        original_player_flags,
    })
}
pub(crate) fn prepare_void_storage_query_capture_fixture(
    bot: &config::BotConfig,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<VoidStorageSmokeFixture> {
    use mysql::prelude::Queryable;

    let mut fixture =
        prepare_void_storage_smoke_fixture(bot, item_entry, runtime_counter, timeout_secs)?;
    let setup = (|| {
        let characters_url = characters_db_url()?;
        let opts = mysql::Opts::from_url(&characters_url)
            .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
        let mut conn = mysql::Conn::new(opts)
            .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
        let max_void_item_id: u64 = conn
            .query_first("SELECT COALESCE(MAX(itemId), 0) FROM character_void_storage")
            .map_err(|error| anyhow!("Load max void item ID for query capture: {error}"))?
            .unwrap_or(0);
        let void_item_id = max_void_item_id
            .checked_add(10_000)
            .ok_or_else(|| anyhow!("void item ID overflow while reserving query fixture"))?;
        let mut transaction = conn
            .start_transaction(mysql::TxOpts::default())
            .map_err(|error| anyhow!("Start void-storage query fixture transaction: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
                (bot.character_guid, fixture.options.fixture_item_guid),
            )
            .map_err(|error| anyhow!("Delete query fixture inventory row: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ? AND itemEntry = ?",
                (
                    fixture.options.fixture_item_guid,
                    bot.character_guid,
                    item_entry,
                ),
            )
            .map_err(|error| anyhow!("Delete query fixture item instance: {error}"))?;
        transaction
            .exec_drop(
                "INSERT INTO character_void_storage \
                 (itemId, playerGuid, itemEntry, slot, creatorGuid, fixedScalingLevel, \
                  randomPropertiesId, randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, 0, 0, 0, 0)",
                (void_item_id, bot.character_guid, item_entry),
            )
            .map_err(|error| anyhow!("Insert void-storage query fixture row: {error}"))?;
        transaction
            .exec_drop(
                "UPDATE characters SET playerFlags = playerFlags | ? WHERE guid = ? AND online = 0",
                (PLAYER_FLAGS_VOID_UNLOCKED, bot.character_guid),
            )
            .map_err(|error| anyhow!("Unlock void-storage query fixture: {error}"))?;
        if transaction.affected_rows() != 1 {
            bail!("void-storage query fixture lost its offline character guard");
        }
        transaction
            .commit()
            .map_err(|error| anyhow!("Commit void-storage query fixture: {error}"))?;
        Ok::<u64, anyhow::Error>(void_item_id)
    })();

    let void_item_id = match setup {
        Ok(void_item_id) => void_item_id,
        Err(error) => {
            return match cleanup_void_storage_smoke_fixture(bot, &fixture) {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(anyhow!(
                    "Void-storage query fixture setup failed: {error}; cleanup failed: {cleanup_error}"
                )),
            };
        }
    };
    fixture.options.phase = VoidStorageSmokePhase::QueryCapture;
    fixture.options.expected_void_item_id = Some(void_item_id);
    fixture.options.expected_void_slot = 0;
    Ok(fixture)
}
pub(crate) fn cleanup_void_storage_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &VoidStorageSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let offline_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Check void-storage bot offline state: {error}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < offline_deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => bail!(
                "character {} remained online; refusing void-storage fixture cleanup",
                bot.character_guid
            ),
            None => bail!(
                "No characters row for guid {} during void-storage cleanup",
                bot.character_guid
            ),
        }
    }

    let item_guids: Vec<u64> = conn
        .exec_map(
            "SELECT ii.guid FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, fixture.options.item_entry),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Resolve void-storage cleanup items: {error}"))?;
    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start void-storage cleanup transaction: {error}"))?;
    for item_guid in item_guids {
        transaction
            .exec_drop(
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
                (bot.character_guid, item_guid),
            )
            .map_err(|error| anyhow!("Delete void-storage fixture inventory row: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ? AND itemEntry = ?",
                (item_guid, bot.character_guid, fixture.options.item_entry),
            )
            .map_err(|error| anyhow!("Delete void-storage fixture item: {error}"))?;
    }
    transaction
        .exec_drop(
            "DELETE FROM character_void_storage WHERE playerGuid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Delete void-storage fixture rows: {error}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET money = ?, playerFlags = ?, map = ?, zone = ?, instance_id = ?, \
             position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
                fixture.original_money,
                fixture.original_player_flags,
                fixture.original_position.map_id,
                fixture.original_position.zone_id,
                fixture.original_position.instance_id,
                fixture.original_position.x,
                fixture.original_position.y,
                fixture.original_position.z,
                fixture.original_position.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|error| anyhow!("Restore void-storage bot character: {error}"))?;
    if transaction.affected_rows() != 1 {
        bail!("void-storage cleanup lost its offline character guard");
    }
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit void-storage fixture cleanup: {error}"))?;
    let restored = load_void_storage_db_state(bot, fixture.options.item_entry)?;
    if restored.money != fixture.original_money
        || restored.player_flags != fixture.original_player_flags
        || !restored.void_items.is_empty()
        || !restored.inventory_items.is_empty()
    {
        bail!("void-storage cleanup verification failed: {restored:?}");
    }
    Ok(())
}
pub(crate) fn create_void_storage_creature_guid_raw(
    map_id: u16,
    entry: u32,
    counter: u64,
    runtime_realm_id: u16,
) -> (u64, u64) {
    let (low, high) = create_creature_guid_raw(map_id, entry, counter);
    (low, high | (u64::from(runtime_realm_id) << 42))
}
pub(crate) fn void_storage_runtime_realm_id() -> Result<u16> {
    let configured = std::env::var("WOW_BOT_VOID_STORAGE_RUNTIME_REALM_ID")
        .ok()
        .map(|value| value.parse::<u16>())
        .transpose()
        .map_err(|error| anyhow!("Invalid WOW_BOT_VOID_STORAGE_RUNTIME_REALM_ID: {error}"))?
        .unwrap_or_else(|| u16::try_from(realm_id()).unwrap_or(u16::MAX));
    if configured > 0x1FFF {
        bail!("void-storage runtime realm ID {configured} exceeds the 13-bit ObjectGuid field");
    }
    Ok(configured)
}

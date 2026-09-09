//! Misc operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_vendor_smoke_fixture(
    bot: &config::BotConfig,
    vendor_entry: u32,
    vendor_spawn_guid: u64,
    runtime_counter: Option<u64>,
    item_entry: u32,
    extended_cost: u32,
    currency_id: u32,
    currency_cost: u32,
    currency_quantity: u32,
    timeout_secs: u64,
) -> Result<VendorSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive vendor fixture setup for non-local account {}",
            bot.account
        );
    }
    if item_entry > i32::MAX as u32
        || extended_cost > i32::MAX as u32
        || currency_id > u16::MAX as u32
        || currency_quantity <= currency_cost
    {
        bail!("vendor fixture identifiers/quantity do not fit the 3.4.3 wire/database shape");
    }
    if runtime_counter.is_some_and(|counter| counter == 0 || counter > OBJECT_GUID_COUNTER_MASK) {
        bail!("vendor runtime counter override must fit the nonzero 40-bit ObjectGuid counter");
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let character: Option<(u32, u8, u32, u32, u32, f64, f64, f64, f32)> = characters
        .exec_first(
            "SELECT account, online, map, zone, instance_id, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load vendor bot character: {error}"))?;
    let (owner, online, map_id, zone_id, instance_id, x, y, z, orientation) =
        character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
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
            "character {} is online; log it out before vendor smoke setup",
            bot.character_guid
        );
    }
    let original_position = CharacterPositionSnapshot {
        map_id,
        zone_id,
        instance_id,
        x,
        y,
        z,
        orientation,
    };
    let original_currency: Option<VendorCurrencyRowSnapshot> = characters
        .exec_first::<(u32, u32, u32, u32, u32, u8), _, _>(
            "SELECT Quantity, WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags \
             FROM character_currency WHERE CharacterGuid = ? AND Currency = ?",
            (bot.character_guid, currency_id),
        )
        .map_err(|error| anyhow!("Load original vendor currency row: {error}"))?
        .map(
            |(
                quantity,
                weekly_quantity,
                tracked_quantity,
                increased_cap_quantity,
                earned_quantity,
                flags,
            )| VendorCurrencyRowSnapshot {
                quantity,
                weekly_quantity,
                tracked_quantity,
                increased_cap_quantity,
                earned_quantity,
                flags,
            },
        );
    let existing_item_total: u64 = characters
        .exec_first(
            "SELECT COALESCE(SUM(ii.count), 0) FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, item_entry),
        )
        .map_err(|error| anyhow!("Check existing vendor fixture item: {error}"))?
        .unwrap_or(0);
    if existing_item_total != 0 {
        bail!(
            "bot character already owns {} of vendor item {}; choose an isolated fixture item",
            existing_item_total,
            item_entry
        );
    }
    let occupied_slots: Vec<u8> = characters
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (bot.character_guid,),
            |slot: u8| slot,
        )
        .map_err(|error| anyhow!("Load vendor bot occupied backpack slots: {error}"))?;
    if !(INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
        .any(|slot| !occupied_slots.contains(&slot))
    {
        bail!("No empty default backpack slot for vendor smoke");
    }

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|error| anyhow!("Bad world DB URL: {error}"))?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let spawn: Option<(u32, u32, f64, f64, f64, f32, f32, u32, u32, String)> = world
        .exec_first(
            "SELECT c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation, \
                    c.wander_distance, c.phaseId, c.phaseGroup, c.spawnDifficulties \
             FROM creature c WHERE c.guid = ?",
            (vendor_spawn_guid,),
        )
        .map_err(|error| anyhow!("Load exact vendor spawn: {error}"))?;
    let (
        spawn_entry,
        vendor_map,
        vendor_x,
        vendor_y,
        vendor_z,
        vendor_o,
        wander_distance,
        phase_id,
        phase_group,
        spawn_difficulties,
    ) = spawn.ok_or_else(|| anyhow!("No world.creature row for guid {vendor_spawn_guid}"))?;
    if spawn_entry != vendor_entry {
        bail!(
            "vendor spawn {} has entry {}, expected {}",
            vendor_spawn_guid,
            spawn_entry,
            vendor_entry
        );
    }
    if phase_id != 0 || phase_group != 0 || !spawn_difficulties.split(',').any(|id| id == "0") {
        bail!(
            "vendor spawn {} is not a deterministic base-phase difficulty-0 fixture",
            vendor_spawn_guid
        );
    }
    let target_match_radius = wander_distance.max(0.0) + 2.0;
    if runtime_counter.is_none() {
        let overlapping_spawn: Option<u64> = world
            .exec_first(
                "SELECT guid FROM creature \
                 WHERE id = ? AND map = ? AND guid <> ? \
                   AND SQRT(POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) \
                       <= ? + GREATEST(wander_distance, 0) \
                 ORDER BY guid LIMIT 1",
                (
                    vendor_entry,
                    vendor_map,
                    vendor_spawn_guid,
                    vendor_x,
                    vendor_y,
                    vendor_z,
                    target_match_radius,
                ),
            )
            .map_err(|error| anyhow!("Check vendor spawn ambiguity: {error}"))?;
        if let Some(overlapping_spawn) = overlapping_spawn {
            bail!(
                "vendor SQL spawn {vendor_spawn_guid} overlaps same-entry spawn {overlapping_spawn}; supply a trusted live runtime counter or choose an isolated spawn"
            );
        }
    }
    let vendor_row_count: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM npc_vendor \
             WHERE entry = ? AND item = ? AND ExtendedCost = ? AND type = 1",
            (vendor_entry, item_entry, extended_cost),
        )
        .map_err(|error| anyhow!("Validate exact npc_vendor row: {error}"))?
        .unwrap_or(0);
    if vendor_row_count != 1 {
        bail!(
            "expected one npc_vendor row for vendor/item/extended-cost {vendor_entry}/{item_entry}/{extended_cost}, found {vendor_row_count}"
        );
    }
    let vendor_map = u16::try_from(vendor_map)
        .map_err(|_| anyhow!("vendor map id does not fit protocol: {vendor_map}"))?;
    let guid_counter = runtime_counter.unwrap_or(0);
    let packed_guid = if guid_counter == 0 {
        Vec::new()
    } else {
        let (low, high) = create_creature_guid_raw(vendor_map, vendor_entry, guid_counter);
        build_packed_guid(low, high)
    };
    let vendor = ResolvedCreatureTarget {
        entry: vendor_entry,
        spawn_guid: vendor_spawn_guid,
        guid_counter,
        map_id: vendor_map,
        x: vendor_x,
        y: vendor_y,
        z: vendor_z,
        orientation: vendor_o,
        packed_guid,
    };

    let mut transaction = characters
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start vendor fixture transaction: {error}"))?;
    transaction
        .exec_drop(
            "INSERT INTO character_currency \
             (CharacterGuid, Currency, Quantity, WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags) \
             VALUES (?, ?, ?, 0, 0, 0, 0, 0) \
             ON DUPLICATE KEY UPDATE Quantity = VALUES(Quantity), WeeklyQuantity = 0, \
                 TrackedQuantity = 0, IncreasedCapQuantity = 0, EarnedQuantity = 0, Flags = 0",
            (bot.character_guid, currency_id, currency_quantity),
        )
        .map_err(|error| anyhow!("Seed vendor currency fixture: {error}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET map = ?, zone = 0, instance_id = 0, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
                u32::from(vendor_map),
                vendor_x + 2.0,
                vendor_y,
                vendor_z,
                vendor_o,
                bot.character_guid,
            ),
        )
        .map_err(|error| anyhow!("Relocate vendor bot near vendor: {error}"))?;
    if transaction.affected_rows() != 1 {
        bail!("vendor character relocation lost its offline ownership guard");
    }
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit vendor fixture transaction: {error}"))?;

    info!(
        "Vendor fixture: character={} vendor={}/{} counter={} item={} extended_cost={} currency={} quantity/cost={}/{}",
        bot.character_guid,
        vendor_entry,
        vendor_spawn_guid,
        guid_counter,
        item_entry,
        extended_cost,
        currency_id,
        currency_quantity,
        currency_cost
    );
    Ok(VendorSmokeFixture {
        options: VendorSmokeOptions {
            phase: VendorSmokePhase::Purchase,
            vendor,
            target_match_radius,
            item_entry,
            extended_cost,
            currency_id,
            currency_before: currency_quantity,
            currency_cost,
            expected_item_total: 1,
            timeout_secs,
        },
        original_position,
        original_currency,
    })
}
pub(crate) fn load_vendor_smoke_db_state(
    bot: &config::BotConfig,
    currency_id: u32,
    item_entry: u32,
) -> Result<(u32, u64)> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let currency = conn
        .exec_first(
            "SELECT Quantity FROM character_currency WHERE CharacterGuid = ? AND Currency = ?",
            (bot.character_guid, currency_id),
        )
        .map_err(|error| anyhow!("Load vendor currency DB state: {error}"))?
        .unwrap_or(0);
    let item_total = conn
        .exec_first(
            "SELECT COALESCE(SUM(ii.count), 0) FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, item_entry),
        )
        .map_err(|error| anyhow!("Load vendor item DB state: {error}"))?
        .unwrap_or(0);
    Ok((currency, item_total))
}
pub(crate) fn cleanup_vendor_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &VendorSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    // Stock C++ may defer its disconnected-session save/offline transition
    // substantially longer than Rust. A failed phase already attempts a
    // graceful logout, but retain a bounded disconnect fallback as well.
    let deadline = std::time::Instant::now() + Duration::from_secs(90);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Check vendor bot offline before cleanup: {error}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => bail!(
                "character {} remained online; refusing vendor cleanup",
                bot.character_guid
            ),
            None => bail!(
                "No characters row for guid {} during vendor cleanup",
                bot.character_guid
            ),
        }
    }

    let purchased_guids: Vec<u64> = conn
        .exec_map(
            "SELECT ii.guid FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ? ORDER BY ii.guid",
            (bot.character_guid, fixture.options.item_entry),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Load purchased vendor fixture item GUIDs: {error}"))?;
    if purchased_guids.len() > fixture.options.expected_item_total as usize {
        bail!(
            "vendor cleanup found {} fixture item stacks, expected at most {}",
            purchased_guids.len(),
            fixture.options.expected_item_total
        );
    }

    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start vendor cleanup transaction: {error}"))?;
    for item_guid in purchased_guids {
        transaction
            .exec_drop(
                "DELETE FROM item_refund_instance WHERE item_guid = ?",
                (item_guid,),
            )
            .map_err(|error| anyhow!("Delete vendor refund metadata: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
                (bot.character_guid, item_guid),
            )
            .map_err(|error| anyhow!("Delete vendor inventory row: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?",
                (item_guid, bot.character_guid),
            )
            .map_err(|error| anyhow!("Delete vendor item instance: {error}"))?;
    }
    transaction
        .exec_drop(
            "DELETE FROM character_currency WHERE CharacterGuid = ? AND Currency = ?",
            (bot.character_guid, fixture.options.currency_id),
        )
        .map_err(|error| anyhow!("Clear vendor fixture currency row: {error}"))?;
    if let Some(currency) = fixture.original_currency {
        transaction
            .exec_drop(
                "INSERT INTO character_currency \
                 (CharacterGuid, Currency, Quantity, WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    bot.character_guid,
                    fixture.options.currency_id,
                    currency.quantity,
                    currency.weekly_quantity,
                    currency.tracked_quantity,
                    currency.increased_cap_quantity,
                    currency.earned_quantity,
                    currency.flags,
                ),
            )
            .map_err(|error| anyhow!("Restore vendor currency snapshot: {error}"))?;
    }
    transaction
        .exec_drop(
            "UPDATE characters SET map = ?, zone = ?, instance_id = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
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
        .map_err(|error| anyhow!("Restore vendor bot position: {error}"))?;
    if transaction.affected_rows() != 1 {
        bail!("vendor cleanup lost its offline character guard");
    }
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit vendor fixture cleanup: {error}"))?;

    let (restored_currency, restored_item_total) =
        load_vendor_smoke_db_state(bot, fixture.options.currency_id, fixture.options.item_entry)?;
    let expected_currency = fixture
        .original_currency
        .map(|currency| currency.quantity)
        .unwrap_or(0);
    if restored_currency != expected_currency || restored_item_total != 0 {
        bail!(
            "vendor cleanup verification found currency/item {restored_currency}/{restored_item_total}, expected {expected_currency}/0"
        );
    }
    Ok(())
}
pub(crate) fn prepare_bank_smoke_fixture(
    bot: &config::BotConfig,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<BankSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive bank fixture setup for non-local account {}",
            bot.account
        );
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    let character: Option<(u32, u8, u32, u32, u32, f64, f64, f64, f32)> = characters
        .exec_first(
            "SELECT account, online, map, zone, instance_id, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load bank bot character: {e}"))?;
    let (owner, online, map_id, zone_id, instance_id, x, y, z, orientation) =
        character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
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
            "character {} is online; log it out before bank smoke setup",
            bot.character_guid
        );
    }
    let original_position = CharacterPositionSnapshot {
        map_id,
        zone_id,
        instance_id,
        x,
        y,
        z,
        orientation,
    };

    let occupied_slots: Vec<u8> = characters
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (bot.character_guid,),
            |slot: u8| slot,
        )
        .map_err(|e| anyhow!("Load occupied bank bot slots: {e}"))?;
    let inventory_slot = (INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
        .find(|slot| !occupied_slots.contains(slot))
        .ok_or_else(|| anyhow!("No empty default backpack slot for bank smoke"))?;
    let bank_slot = (BANK_SLOT_ITEM_START..BANK_SLOT_ITEM_END)
        .find(|slot| !occupied_slots.contains(slot))
        .ok_or_else(|| anyhow!("No empty personal bank slot for bank smoke"))?;

    let same_entry_count: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, item_entry),
        )
        .map_err(|e| anyhow!("Check existing fixture item entry: {e}"))?
        .unwrap_or(0);
    if same_entry_count != 0 {
        bail!(
            "bot character already owns item entry {}; choose another --bank-item-entry to keep the fixture isolated",
            item_entry
        );
    }

    let max_item_guid: u64 = characters
        .query_first("SELECT COALESCE(MAX(guid), 0) FROM item_instance")
        .map_err(|e| anyhow!("Load max item guid: {e}"))?
        .unwrap_or(0);
    let item_guid = max_item_guid
        .checked_add(10_000)
        .ok_or_else(|| anyhow!("item guid overflow while reserving bank fixture"))?;

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|e| anyhow!("Bad world DB URL: {e}"))?;
    let mut world =
        mysql::Conn::new(world_opts).map_err(|e| anyhow!("Connect to world DB failed: {e}"))?;
    // Use a neutral banker fixture. Picking the geometrically nearest banker can
    // cross faction boundaries on continent maps (for example Exodar vs.
    // Silvermoon), and C++ `GetNPCIfCanInteractWith` correctly rejects hostile
    // NPCs even when their BANKER flag and distance are valid.
    let neutral: Option<(u64, u32, u32, f64, f64, f64, f32)> = world
        .exec_first(
            "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
             FROM creature c JOIN creature_template ct ON ct.entry = c.id \
             WHERE ct.faction = 35 \
               AND ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
               AND c.phaseid = 0 AND c.phasegroup = 0 \
               AND FIND_IN_SET('0', c.spawnDifficulties) > 0 \
               AND ct.VehicleId = 0 \
             ORDER BY c.guid LIMIT 1",
            (NPC_FLAG_BANKER,),
        )
        .map_err(|e| anyhow!("Resolve neutral banker: {e}"))?;
    let banker_row = match neutral {
        Some(row) => row,
        None => world
            .exec_first(
                "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
                 FROM creature c JOIN creature_template ct ON ct.entry = c.id \
                 WHERE ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
                 ORDER BY c.guid LIMIT 1",
                (NPC_FLAG_BANKER,),
            )
            .map_err(|e| anyhow!("Resolve fallback banker: {e}"))?
            .ok_or_else(|| anyhow!("No banker creature spawn exists in world DB"))?,
    };
    let (spawn_guid, entry, banker_map, banker_x, banker_y, banker_z, banker_orientation) =
        banker_row;
    let banker_map = u16::try_from(banker_map)
        .map_err(|_| anyhow!("banker map id does not fit protocol: {banker_map}"))?;
    let guid_counter = runtime_counter.ok_or_else(|| {
        anyhow!(
            "banker entry {entry} resolved world.creature guid {spawn_guid}, but needs the live ObjectGuid low counter"
        )
    })?;
    let (low, high) = create_creature_guid_raw(banker_map, entry, guid_counter);
    let banker = ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id: banker_map,
        x: banker_x,
        y: banker_y,
        z: banker_z,
        orientation: banker_orientation,
        packed_guid: build_packed_guid(low, high),
    };

    let mut transaction = characters
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start bank fixture transaction: {e}"))?;
    transaction
        .exec_drop(
            "INSERT INTO item_instance \
             (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, durability, \
              enchantments, charges, flags, randomPropertiesId, randomPropertiesSeed, context) \
             VALUES (?, ?, ?, 0, 0, 1, 0, '', '', 0, 0, 0, 0)",
            (item_guid, item_entry, bot.character_guid),
        )
        .map_err(|e| anyhow!("Insert bank fixture item: {e}"))?;
    transaction
        .exec_drop(
            "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)",
            (bot.character_guid, inventory_slot, item_guid),
        )
        .map_err(|e| anyhow!("Insert bank fixture inventory row: {e}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET map = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ?",
            (
                u32::from(banker_map),
                banker_x + 2.0,
                banker_y,
                banker_z,
                banker_orientation,
                bot.character_guid,
            ),
        )
        .map_err(|e| anyhow!("Relocate bank bot near banker: {e}"))?;
    transaction
        .commit()
        .map_err(|e| anyhow!("Commit bank fixture transaction: {e}"))?;

    info!(
        "Bank smoke fixture: character={} item={}/entry={} inventory_slot={} bank_slot={} banker={}/{} runtime_counter={}",
        bot.character_guid,
        item_guid,
        item_entry,
        inventory_slot,
        bank_slot,
        entry,
        spawn_guid,
        guid_counter
    );
    Ok(BankSmokeFixture {
        options: BankSmokeOptions {
            phase: BankSmokePhase::Deposit,
            banker,
            item_guid,
            item_entry,
            inventory_slot,
            bank_slot,
            timeout_secs,
        },
        original_position,
    })
}
pub(crate) fn prepare_homebind_smoke_fixture(
    bot: &config::BotConfig,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<HomebindSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive homebind fixture setup for non-local account {}",
            bot.account
        );
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    let character: Option<(u32, u8, u8, u32, u32, u32, f64, f64, f64, f32)> = characters
        .exec_first(
            "SELECT account, online, race, map, zone, instance_id, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load homebind bot character: {e}"))?;
    let (owner, online, race, map_id, zone_id, instance_id, x, y, z, orientation) =
        character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
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
            "character {} is online; log it out before homebind smoke setup",
            bot.character_guid
        );
    }
    let original_position = CharacterPositionSnapshot {
        map_id,
        zone_id,
        instance_id,
        x,
        y,
        z,
        orientation,
    };
    let original_homebind = characters
        .exec_first(
            "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load original character_homebind: {e}"))?
        .map(|(map_id, zone_id, x, y, z, orientation)| HomebindRowSnapshot {
            map_id,
            zone_id,
            x,
            y,
            z,
            orientation,
        });

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|e| anyhow!("Bad world DB URL: {e}"))?;
    let mut world =
        mysql::Conn::new(world_opts).map_err(|e| anyhow!("Connect to world DB failed: {e}"))?;
    let preferred_faction = if matches!(race, 2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 35 | 36) {
        29u32
    } else {
        12u32
    };
    let innkeeper_row: (u64, u32, u32, f64, f64, f64, f32) = world
        .exec_first(
            "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
             FROM creature c JOIN creature_template ct ON ct.entry = c.id \
             WHERE c.map IN (0, 1) \
               AND ct.faction = ? \
               AND ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
               AND c.phaseid = 0 AND c.phasegroup = 0 \
               AND FIND_IN_SET('0', c.spawnDifficulties) > 0 \
               AND ct.VehicleId = 0 \
               AND NOT EXISTS (SELECT 1 FROM creature duplicate \
                               WHERE duplicate.map = c.map AND duplicate.id = c.id \
                                 AND duplicate.guid <> c.guid) \
             ORDER BY c.guid LIMIT 1",
            (preferred_faction, NPC_FLAG_INNKEEPER),
        )
        .map_err(|e| anyhow!("Resolve unique faction-friendly innkeeper: {e}"))?
        .ok_or_else(|| anyhow!("No unique faction-friendly continent innkeeper exists"))?;
    let (spawn_guid, entry, innkeeper_map, innkeeper_x, innkeeper_y, innkeeper_z, innkeeper_o) =
        innkeeper_row;
    let innkeeper_map = u16::try_from(innkeeper_map)
        .map_err(|_| anyhow!("innkeeper map id does not fit protocol: {innkeeper_map}"))?;
    let discover_runtime_guid = runtime_counter.is_none();
    // The placeholder is replaced from the login UpdateObject stream. An
    // explicit override remains useful for narrow packet captures.
    let guid_counter = runtime_counter.unwrap_or(spawn_guid);
    let (low, high) = create_creature_guid_raw(innkeeper_map, entry, guid_counter);
    let innkeeper = ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id: innkeeper_map,
        x: innkeeper_x,
        y: innkeeper_y,
        z: innkeeper_z,
        orientation: innkeeper_o,
        packed_guid: build_packed_guid(low, high),
    };

    characters
        .exec_drop(
            "UPDATE characters SET map = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ?",
            (
                u32::from(innkeeper_map),
                innkeeper_x + 2.0,
                innkeeper_y,
                innkeeper_z,
                innkeeper_o,
                bot.character_guid,
            ),
        )
        .map_err(|e| anyhow!("Relocate homebind bot near innkeeper: {e}"))?;

    Ok(HomebindSmokeFixture {
        options: HomebindSmokeOptions {
            phase: HomebindSmokePhase::Bind,
            innkeeper,
            discover_runtime_guid,
            expected_homebind: None,
            timeout_secs,
        },
        original_position,
        original_homebind,
    })
}
pub(crate) fn load_homebind_row(bot: &config::BotConfig) -> Result<Option<HomebindRowSnapshot>> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    let row: Option<(u16, u16, f32, f32, f32, f32)> = conn
        .exec_first(
            "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load bound character_homebind: {e}"))?;
    Ok(row.map(
        |(map_id, zone_id, x, y, z, orientation)| HomebindRowSnapshot {
            map_id,
            zone_id,
            x,
            y,
            z,
            orientation,
        },
    ))
}
pub(crate) fn wait_for_homebind_row(
    bot: &config::BotConfig,
    expected: &HomebindRowSnapshot,
    timeout: Duration,
) -> Result<bool> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if load_homebind_row(bot)?.as_ref() == Some(expected) {
            return Ok(true);
        }
        if std::time::Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

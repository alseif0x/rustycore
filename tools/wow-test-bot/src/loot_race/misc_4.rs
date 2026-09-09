//! Loot-race misc operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

pub(crate) fn parse_loot_response(payload: &[u8]) -> Result<ParsedLootResponse> {
    let (owner_used, owner_low, owner_high) =
        parse_packed_guid(payload).ok_or_else(|| anyhow!("malformed loot response owner guid"))?;
    let (loot_used, loot_low, loot_high) = parse_packed_guid(&payload[owner_used..])
        .ok_or_else(|| anyhow!("malformed loot response loot guid"))?;
    let mut offset = owner_used + loot_used;
    let failure_reason = take_u8(payload, &mut offset)?;
    let _acquire_reason = take_u8(payload, &mut offset)?;
    let loot_method = take_u8(payload, &mut offset)?;
    let _threshold = take_u8(payload, &mut offset)?;
    let coins = take_u32(payload, &mut offset)?;
    let item_count = take_u32(payload, &mut offset)? as usize;
    let currency_count = take_u32(payload, &mut offset)? as usize;
    let bits = take_u8(payload, &mut offset)?;
    let acquired = bits & 0x80 != 0;
    let mut items = Vec::with_capacity(item_count);
    for _ in 0..item_count {
        let _item_bits = take_u8(payload, &mut offset)?;
        let item_entry = take_i32(payload, &mut offset)?;
        if item_entry <= 0 {
            bail!("loot response has nonpositive item entry {item_entry}");
        }
        let _random_seed = take_i32(payload, &mut offset)?;
        let _random_property = take_i32(payload, &mut offset)?;
        let has_bonus = take_u8(payload, &mut offset)? & 0x80 != 0;
        let mod_count = usize::from(take_u8(payload, &mut offset)? >> 2);
        offset = offset
            .checked_add(mod_count * 5)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| anyhow!("loot response item modifications are truncated"))?;
        if has_bonus {
            let _context = take_u8(payload, &mut offset)?;
            let bonus_count = take_u32(payload, &mut offset)? as usize;
            offset = offset
                .checked_add(bonus_count * 4)
                .filter(|end| *end <= payload.len())
                .ok_or_else(|| anyhow!("loot response item bonuses are truncated"))?;
        }
        let quantity = take_u32(payload, &mut offset)?;
        let _loot_item_type = take_u8(payload, &mut offset)?;
        let loot_list_id = take_u8(payload, &mut offset)?;
        items.push(ParsedLootItem {
            item_entry: item_entry as u32,
            quantity,
            loot_list_id,
        });
    }
    // The fixture forbids currencies; parsing their variable bit tail is not
    // needed and accepting it would make the preflight weaker.
    if currency_count != 0 {
        bail!("loot-race fixture unexpectedly produced {currency_count} currencies");
    }
    if offset != payload.len() {
        bail!(
            "loot response has {} unexpected trailing bytes",
            payload.len() - offset
        );
    }
    Ok(ParsedLootResponse {
        owner_low,
        owner_high,
        loot_low,
        loot_high,
        failure_reason,
        loot_method,
        coins,
        acquired,
        items,
    })
}
pub(crate) fn parse_item_push(payload: &[u8]) -> Result<ItemPush> {
    let (player_used, player_low, player_high) = parse_packed_guid(payload)
        .ok_or_else(|| anyhow!("malformed SMSG_ITEM_PUSH_RESULT player guid"))?;
    let mut offset = player_used;
    let slot = take_u8(payload, &mut offset)?;
    let slot_in_bag = take_i32(payload, &mut offset)?;
    let quest_log_item_id = take_i32(payload, &mut offset)?;
    let quantity = take_i32(payload, &mut offset)?;
    let quantity_in_inventory = take_i32(payload, &mut offset)?;
    let dungeon_encounter_id = take_i32(payload, &mut offset)?;
    // Battle-pet metadata is part of the 3.4.3 layout but is unrelated to
    // this ordinary item fixture. Consume it so the following GUID/bit fields
    // remain aligned.
    for _ in 0..4 {
        let _ = take_i32(payload, &mut offset)?;
    }
    let (item_guid_used, item_guid_low, item_guid_high) = parse_packed_guid(&payload[offset..])
        .ok_or_else(|| anyhow!("malformed SMSG_ITEM_PUSH_RESULT item guid"))?;
    offset += item_guid_used;
    let flags = take_u8(payload, &mut offset)?;
    if flags & 0x01 != 0 {
        bail!("SMSG_ITEM_PUSH_RESULT has a nonzero reserved padding bit");
    }
    let pushed = flags & 0x80 != 0;
    let created = flags & 0x40 != 0;
    let display_text = (flags >> 3) & 0x07;
    let is_bonus_roll = flags & 0x04 != 0;
    let is_encounter_loot = flags & 0x02 != 0;
    let item_entry = take_i32(payload, &mut offset)?;
    if item_entry <= 0 {
        bail!("item push has nonpositive item entry {item_entry}");
    }
    let _random_properties_seed = take_i32(payload, &mut offset)?;
    let _random_properties_id = take_i32(payload, &mut offset)?;
    let item_bonus_bits = take_u8(payload, &mut offset)?;
    if item_bonus_bits & 0x7F != 0 {
        bail!("SMSG_ITEM_PUSH_RESULT ItemBonus has nonzero padding bits");
    }
    let has_item_bonus = item_bonus_bits & 0x80 != 0;
    let modification_bits = take_u8(payload, &mut offset)?;
    if modification_bits & 0x03 != 0 {
        bail!("SMSG_ITEM_PUSH_RESULT ItemModList has nonzero padding bits");
    }
    let modification_count = usize::from(modification_bits >> 2);
    for _ in 0..modification_count {
        let _value = take_i32(payload, &mut offset)?;
        let _modifier_type = take_u8(payload, &mut offset)?;
    }
    if has_item_bonus {
        let _context = take_u8(payload, &mut offset)?;
        let bonus_count = usize::try_from(take_u32(payload, &mut offset)?)?;
        let bonus_bytes = bonus_count
            .checked_mul(std::mem::size_of::<u32>())
            .ok_or_else(|| anyhow!("SMSG_ITEM_PUSH_RESULT bonus-list length overflow"))?;
        offset = offset
            .checked_add(bonus_bytes)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| anyhow!("SMSG_ITEM_PUSH_RESULT item bonuses are truncated"))?;
    }
    if offset != payload.len() {
        bail!(
            "SMSG_ITEM_PUSH_RESULT has {} unexpected trailing bytes",
            payload.len() - offset
        );
    }
    Ok(ItemPush {
        player_low,
        player_high,
        slot,
        slot_in_bag,
        quest_log_item_id,
        quantity,
        quantity_in_inventory,
        dungeon_encounter_id,
        item_guid_low,
        item_guid_high,
        pushed,
        created,
        display_text,
        is_bonus_roll,
        is_encounter_loot,
        item_entry: item_entry as u32,
    })
}
pub(crate) fn validate_vendor_item_push_result_like_cpp(
    payload: &[u8],
    expected_character_guid: u64,
    expected_item_entry: u32,
    expected_quantity: u32,
    expected_realm_id: u32,
) -> Result<()> {
    let push = parse_item_push(payload)?;
    let expected_quantity = i32::try_from(expected_quantity)
        .map_err(|_| anyhow!("vendor item quantity exceeds i32"))?;
    let expected_player = create_player_guid_raw(expected_character_guid, expected_realm_id);
    if (push.player_low, push.player_high) != expected_player {
        bail!(
            "vendor ItemPush player {:#018X}/{:#018X} did not match character {}",
            push.player_low,
            push.player_high,
            expected_character_guid
        );
    }
    if push.item_entry != expected_item_entry
        || push.quantity != expected_quantity
        || push.quantity_in_inventory != expected_quantity
    {
        bail!(
            "vendor ItemPush entry/quantity/inventory {:?} did not match {expected_item_entry}/{expected_quantity}/{expected_quantity}",
            (push.item_entry, push.quantity, push.quantity_in_inventory)
        );
    }
    if push.slot != INVENTORY_SLOT_BAG_0
        || !(i32::from(INVENTORY_SLOT_ITEM_START)..i32::from(INVENTORY_SLOT_ITEM_START + 16))
            .contains(&push.slot_in_bag)
    {
        bail!(
            "vendor ItemPush slot {}/{} was not a base-backpack destination",
            push.slot,
            push.slot_in_bag
        );
    }
    if push.quest_log_item_id != 0
        || !push.pushed
        || push.created
        || push.display_text != 1
        || push.is_bonus_roll
        || push.is_encounter_loot
        || push.dungeon_encounter_id != 0
    {
        bail!("vendor ItemPush flags were not the ordinary C++ purchase shape: {push:?}");
    }
    let expected_item_high =
        (HIGH_GUID_ITEM << 58) | ((u64::from(expected_realm_id) & GUID_REALM_SPECIFIC_MASK) << 42);
    if push.item_guid_low == 0
        || push.item_guid_low & !GUID_COUNTER_MASK != 0
        || push.item_guid_high != expected_item_high
    {
        bail!(
            "vendor ItemPush GUID {:#018X}/{:#018X} was not a nonempty C++ Item GUID for realm {}",
            push.item_guid_low,
            push.item_guid_high,
            expected_realm_id
        );
    }
    Ok(())
}
pub(crate) fn parse_inventory_failure(payload: &[u8]) -> Result<InventoryFailure> {
    if payload.len() < 4 {
        bail!("malformed SMSG_INVENTORY_CHANGE_FAILURE result");
    }
    let result = i32::from_le_bytes(payload[0..4].try_into()?);
    let (item_0_used, item_0_low, item_0_high) = parse_packed_guid(&payload[4..])
        .ok_or_else(|| anyhow!("malformed SMSG_INVENTORY_CHANGE_FAILURE item 0"))?;
    let item_1_offset = 4 + item_0_used;
    let (item_1_used, item_1_low, item_1_high) = parse_packed_guid(&payload[item_1_offset..])
        .ok_or_else(|| anyhow!("malformed SMSG_INVENTORY_CHANGE_FAILURE item 1"))?;
    let end = item_1_offset + item_1_used;
    let container_b_slot = *payload
        .get(end)
        .ok_or_else(|| anyhow!("malformed SMSG_INVENTORY_CHANGE_FAILURE container slot"))?;
    // EQUIP_ERR_LOOT_GONE has no result-specific tail in C++.
    if result == 50 && end + 1 != payload.len() {
        bail!("EQUIP_ERR_LOOT_GONE has unexpected trailing bytes");
    }
    Ok(InventoryFailure {
        result,
        item_0_low,
        item_0_high,
        item_1_low,
        item_1_high,
        container_b_slot,
    })
}
pub(crate) fn take_u8(payload: &[u8], offset: &mut usize) -> Result<u8> {
    let value = *payload
        .get(*offset)
        .ok_or_else(|| anyhow!("packet truncated at byte {}", *offset))?;
    *offset += 1;
    Ok(value)
}
pub(crate) fn take_u32(payload: &[u8], offset: &mut usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| anyhow!("packet offset overflow"))?;
    let bytes: [u8; 4] = payload
        .get(*offset..end)
        .ok_or_else(|| anyhow!("packet truncated at byte {}", *offset))?
        .try_into()?;
    *offset = end;
    Ok(u32::from_le_bytes(bytes))
}
pub(crate) fn take_i32(payload: &[u8], offset: &mut usize) -> Result<i32> {
    Ok(take_u32(payload, offset)? as i32)
}
pub(crate) fn validate_required_empty_top_level_slot(
    character_guid: u64,
    occupied: &[u8],
    required: Option<(u64, u8)>,
) -> Result<()> {
    if let Some((required_guid, required_slot)) = required {
        if character_guid == required_guid && occupied.contains(&required_slot) {
            bail!(
                "loot-item capture character {character_guid} requires exact top-level keyring slot {required_slot} empty"
            );
        }
    }
    Ok(())
}
pub(crate) fn ensure_no_online_characters(conn: &mut mysql::Conn, stage: &str) -> Result<()> {
    let online: u64 = conn
        .query_first("SELECT COUNT(*) FROM characters WHERE online <> 0")
        .map_err(|error| anyhow!("Check global online-character isolation at {stage}: {error}"))?
        .unwrap_or(0);
    validate_online_character_count(online, stage)
}
pub(crate) fn validate_online_character_count(online: u64, stage: &str) -> Result<()> {
    if online != 0 {
        bail!(
            "loot capture requires exclusive world access at {stage}, but {online} character(s) are marked online"
        );
    }
    Ok(())
}
pub(crate) fn load_character_progress_snapshot(
    conn: &mut mysql::Conn,
    guid_a: u64,
    guid_b: u64,
) -> Result<CharacterProgressSnapshot> {
    let guids = (guid_a, guid_b);
    Ok(CharacterProgressSnapshot {
        achievements: conn
            .exec(
                "SELECT guid, achievement, `date` FROM character_achievement \
                 WHERE guid IN (?, ?) ORDER BY guid, achievement",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character achievements: {error}"))?,
        achievement_progress: conn
            .exec(
                "SELECT guid, criteria, counter, `date` FROM character_achievement_progress \
                 WHERE guid IN (?, ?) ORDER BY guid, criteria",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character achievement criteria: {error}"))?,
        quest_status: conn
            .exec(
                "SELECT guid, quest, status, explored, acceptTime, endTime \
                 FROM character_queststatus WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character quest status: {error}"))?,
        quest_daily: conn
            .exec(
                "SELECT guid, quest, `time` FROM character_queststatus_daily \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character daily quests: {error}"))?,
        quest_monthly: conn
            .exec(
                "SELECT guid, quest FROM character_queststatus_monthly \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character monthly quests: {error}"))?,
        quest_objectives: conn
            .exec(
                "SELECT guid, quest, objective, data FROM character_queststatus_objectives \
                 WHERE guid IN (?, ?) ORDER BY guid, quest, objective",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character quest objectives: {error}"))?,
        quest_objective_criteria: conn
            .exec(
                "SELECT guid, questObjectiveId FROM character_queststatus_objectives_criteria \
                 WHERE guid IN (?, ?) ORDER BY guid, questObjectiveId",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character quest objective criteria: {error}"))?,
        quest_objective_criteria_progress: conn
            .exec(
                "SELECT guid, criteriaId, counter, `date` \
                 FROM character_queststatus_objectives_criteria_progress \
                 WHERE guid IN (?, ?) ORDER BY guid, criteriaId",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character quest criteria progress: {error}"))?,
        quest_rewarded: conn
            .exec(
                "SELECT guid, quest, active FROM character_queststatus_rewarded \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character rewarded quests: {error}"))?,
        quest_seasonal: conn
            .exec(
                "SELECT guid, quest, event, completedTime FROM character_queststatus_seasonal \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character seasonal quests: {error}"))?,
        quest_weekly: conn
            .exec(
                "SELECT guid, quest FROM character_queststatus_weekly \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character weekly quests: {error}"))?,
        reputation: conn
            .exec(
                "SELECT guid, faction, standing, flags FROM character_reputation \
                 WHERE guid IN (?, ?) ORDER BY guid, faction",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character reputation: {error}"))?,
    })
}
pub(crate) fn restore_character_progress_snapshot(
    tx: &mut mysql::Transaction<'_>,
    guid_a: u64,
    guid_b: u64,
    snapshot: &CharacterProgressSnapshot,
) -> Result<()> {
    for table in CHARACTER_PROGRESS_TABLES {
        tx.exec_drop(
            format!("DELETE FROM `{table}` WHERE guid IN (?, ?)"),
            (guid_a, guid_b),
        )
        .map_err(|error| anyhow!("Clear loot-fixture progress table {table}: {error}"))?;
    }

    for &(guid, achievement, date) in &snapshot.achievements {
        tx.exec_drop(
            "INSERT INTO character_achievement (guid, achievement, `date`) VALUES (?, ?, ?)",
            (guid, achievement, date),
        )
        .map_err(|error| anyhow!("Restore character achievements: {error}"))?;
    }
    for &(guid, criteria, counter, date) in &snapshot.achievement_progress {
        tx.exec_drop(
            "INSERT INTO character_achievement_progress (guid, criteria, counter, `date`) \
             VALUES (?, ?, ?, ?)",
            (guid, criteria, counter, date),
        )
        .map_err(|error| anyhow!("Restore character achievement criteria: {error}"))?;
    }
    for &(guid, quest, status, explored, accept_time, end_time) in &snapshot.quest_status {
        tx.exec_drop(
            "INSERT INTO character_queststatus \
             (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)",
            (guid, quest, status, explored, accept_time, end_time),
        )
        .map_err(|error| anyhow!("Restore character quest status: {error}"))?;
    }
    for &(guid, quest, time) in &snapshot.quest_daily {
        tx.exec_drop(
            "INSERT INTO character_queststatus_daily (guid, quest, `time`) VALUES (?, ?, ?)",
            (guid, quest, time),
        )
        .map_err(|error| anyhow!("Restore character daily quests: {error}"))?;
    }
    for &(guid, quest) in &snapshot.quest_monthly {
        tx.exec_drop(
            "INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)",
            (guid, quest),
        )
        .map_err(|error| anyhow!("Restore character monthly quests: {error}"))?;
    }
    for &(guid, quest, objective, data) in &snapshot.quest_objectives {
        tx.exec_drop(
            "INSERT INTO character_queststatus_objectives (guid, quest, objective, data) \
             VALUES (?, ?, ?, ?)",
            (guid, quest, objective, data),
        )
        .map_err(|error| anyhow!("Restore character quest objectives: {error}"))?;
    }
    for &(guid, objective) in &snapshot.quest_objective_criteria {
        tx.exec_drop(
            "INSERT INTO character_queststatus_objectives_criteria (guid, questObjectiveId) \
             VALUES (?, ?)",
            (guid, objective),
        )
        .map_err(|error| anyhow!("Restore character quest objective criteria: {error}"))?;
    }
    for &(guid, criteria, counter, date) in &snapshot.quest_objective_criteria_progress {
        tx.exec_drop(
            "INSERT INTO character_queststatus_objectives_criteria_progress \
             (guid, criteriaId, counter, `date`) VALUES (?, ?, ?, ?)",
            (guid, criteria, counter, date),
        )
        .map_err(|error| anyhow!("Restore character quest criteria progress: {error}"))?;
    }
    for &(guid, quest, active) in &snapshot.quest_rewarded {
        tx.exec_drop(
            "INSERT INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, ?)",
            (guid, quest, active),
        )
        .map_err(|error| anyhow!("Restore character rewarded quests: {error}"))?;
    }
    for &(guid, quest, event, completed_time) in &snapshot.quest_seasonal {
        tx.exec_drop(
            "INSERT INTO character_queststatus_seasonal (guid, quest, event, completedTime) \
             VALUES (?, ?, ?, ?)",
            (guid, quest, event, completed_time),
        )
        .map_err(|error| anyhow!("Restore character seasonal quests: {error}"))?;
    }
    for &(guid, quest) in &snapshot.quest_weekly {
        tx.exec_drop(
            "INSERT INTO character_queststatus_weekly (guid, quest) VALUES (?, ?)",
            (guid, quest),
        )
        .map_err(|error| anyhow!("Restore character weekly quests: {error}"))?;
    }
    for &(guid, faction, standing, flags) in &snapshot.reputation {
        tx.exec_drop(
            "INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ?, ?)",
            (guid, faction, standing, flags),
        )
        .map_err(|error| anyhow!("Restore character reputation: {error}"))?;
    }
    Ok(())
}
pub(crate) fn load_character_core_snapshot(
    conn: &mut mysql::Conn,
    character_guid: u64,
) -> Result<CharacterCoreSnapshot> {
    let row: mysql::Row = conn
        .exec_first(
            "SELECT level, xp, health, \
                    power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, \
                    restState, rest_bonus, exploredZones, knownTitles, chosenTitle \
             FROM characters WHERE guid = ?",
            (character_guid,),
        )
        .map_err(|error| anyhow!("Reload loot-fixture character core state: {error}"))?
        .ok_or_else(|| anyhow!("Loot-fixture character {character_guid} disappeared"))?;
    Ok(CharacterCoreSnapshot {
        level: required_row_value(&row, "level")?,
        xp: required_row_value(&row, "xp")?,
        health: required_row_value(&row, "health")?,
        powers: [
            required_row_value(&row, "power1")?,
            required_row_value(&row, "power2")?,
            required_row_value(&row, "power3")?,
            required_row_value(&row, "power4")?,
            required_row_value(&row, "power5")?,
            required_row_value(&row, "power6")?,
            required_row_value(&row, "power7")?,
            required_row_value(&row, "power8")?,
            required_row_value(&row, "power9")?,
            required_row_value(&row, "power10")?,
        ],
        rest_state: required_row_value(&row, "restState")?,
        rest_bonus: required_row_value(&row, "rest_bonus")?,
        explored_zones: required_row_value(&row, "exploredZones")?,
        known_titles: required_row_value(&row, "knownTitles")?,
        chosen_title: required_row_value(&row, "chosenTitle")?,
    })
}
pub(crate) async fn expected_persisted_item_grant(
    fixture: &LootRaceFixture,
    sync: &LootRaceSync,
) -> Result<ExpectedPersistedItemGrant> {
    let window = sync.windows.lock().await[0]
        .clone()
        .ok_or_else(|| anyhow!("loot-race passed without a retained item window"))?;
    let (owner_low, owner_high) = sync
        .runtime_guid
        .lock()
        .map_err(|_| anyhow!("loot-race runtime GUID state was poisoned"))?
        .as_ref()
        .copied()
        .ok_or_else(|| anyhow!("loot-race passed without a retained world-object ObjectGuid"))?;
    let expected_removal = LootRemovedEvidence {
        owner_low,
        owner_high,
        loot_low: window.loot_low,
        loot_high: window.loot_high,
        loot_list_id: window.loot_list_id,
    };
    let evidence = sync.evidence.lock().await;
    validate_atomic_item_wire_outcome_like_cpp(
        &evidence,
        [
            fixture.characters[0].bot.character_guid,
            fixture.characters[1].bot.character_guid,
        ],
        fixture.target.item_entry,
        window.quantity,
        expected_removal,
        realm_id(),
    )
}
pub(crate) async fn expected_persisted_money_grant(
    fixture: &LootRaceFixture,
    sync: &LootRaceSync,
    source_coins: u64,
) -> Result<ExpectedPersistedMoneyGrant> {
    if source_coins != u64::from(RACE_GAMEOBJECT_MONEY) {
        bail!(
            "shared chest source money was {source_coins}, expected exact guarded pool {RACE_GAMEOBJECT_MONEY}"
        );
    }
    let window = sync.windows.lock().await[0]
        .clone()
        .ok_or_else(|| anyhow!("loot-race passed without a retained money window"))?;
    let evidence = sync.evidence.lock().await;
    let winner = validate_serialized_gameobject_money_wire_outcome_like_cpp(
        &evidence,
        (window.loot_low, window.loot_high),
        source_coins,
    )?;
    Ok(ExpectedPersistedMoneyGrant {
        owner_guid: fixture.characters[winner].bot.character_guid,
        amount: source_coins,
    })
}
pub(crate) fn validate_persisted_item_grant_like_cpp(
    expected: ExpectedPersistedItemGrant,
    persisted: PersistedItemGrantRow,
) -> Result<()> {
    let quantity = u32::try_from(expected.push.quantity)
        .map_err(|_| anyhow!("wire item quantity was negative"))?;
    let quantity_in_inventory = u32::try_from(expected.push.quantity_in_inventory)
        .map_err(|_| anyhow!("wire inventory quantity was negative"))?;
    let slot_in_bag = u8::try_from(expected.push.slot_in_bag)
        .map_err(|_| anyhow!("wire SlotInBag did not fit a persisted inventory slot"))?;

    if persisted.item_guid != expected.push.item_guid_low
        || persisted.owner_guid != expected.owner_guid
        || persisted.owner_guid != expected.push.player_low
        || persisted.item_entry != expected.push.item_entry
        || persisted.count != quantity
        || persisted.count != quantity_in_inventory
    {
        bail!(
            "wire-keyed persisted item {:?} did not match grant owner/item/count {:?}",
            persisted,
            expected
        );
    }
    // For this clean fixture CanStoreNewItem chooses a free top-level
    // backpack slot. C++ serializes that as wire Slot=255 but persists bag=0;
    // those values are intentionally not compared numerically.
    if expected.push.slot != INVENTORY_SLOT_BAG_0
        || persisted.inventory_owner != Some(expected.owner_guid)
        || persisted.bag_guid != Some(0)
        || persisted.slot != Some(slot_in_bag)
        || persisted.bag_slot.is_some()
    {
        bail!(
            "wire slot {}/{} did not bind to the expected top-level character_inventory row {:?}",
            expected.push.slot,
            expected.push.slot_in_bag,
            persisted
        );
    }
    Ok(())
}
pub(crate) fn verify_persisted_grants(
    fixture: &LootRaceFixture,
    expected_money_grant: ExpectedPersistedMoneyGrant,
    expected_item_grant: ExpectedPersistedItemGrant,
) -> Result<(
    u64,
    u64,
    ExpectedPersistedItemGrant,
    ExpectedPersistedMoneyGrant,
)> {
    let url = characters_db_url()?;
    let opts = loot_db_opts(&url, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    wait_both_offline(&mut conn, fixture)?;
    ensure_no_online_characters(&mut conn, "loot-race persistence verification")?;
    let guids = (
        fixture.characters[0].bot.character_guid,
        fixture.characters[1].bot.character_guid,
    );
    let (item_total, item_rows, inventory_rows, persisted_item_owner): (u64, u64, u64, u64) = conn
        .exec_first(
            "SELECT COALESCE(SUM(ii.count), 0), COUNT(DISTINCT ii.guid), \
                    COUNT(DISTINCT ci.item), COALESCE(MIN(ii.owner_guid), 0) \
             FROM item_instance ii \
             LEFT JOIN character_inventory ci \
               ON ci.item = ii.guid AND ci.guid = ii.owner_guid \
             WHERE ii.itemEntry = ? AND ii.owner_guid IN (?, ?)",
            (fixture.target.item_entry, guids.0, guids.1),
        )
        .map_err(|error| anyhow!("Verify loot-race item persistence: {error}"))?
        .ok_or_else(|| anyhow!("loot-race item aggregate query returned no row"))?;
    let current_money: Vec<(u64, u64)> = conn
        .exec(
            "SELECT guid, money FROM characters WHERE guid IN (?, ?) ORDER BY guid",
            guids,
        )
        .map_err(|error| anyhow!("Verify loot-race money: {error}"))?;
    if current_money.len() != 2 {
        bail!("loot-race character rows disappeared during verification");
    }
    if item_total != 1 || item_rows != 1 || inventory_rows != 1 {
        bail!(
            "atomic ITEM race persisted quantity/instances/inventory rows {item_total}/{item_rows}/{inventory_rows}; expected 1/1/1"
        );
    }
    if persisted_item_owner != expected_item_grant.owner_guid {
        bail!(
            "atomic ITEM race persisted owner {persisted_item_owner}, but wire winner was character {}",
            expected_item_grant.owner_guid
        );
    }
    let persisted_tuple: Option<(
        u64,
        u64,
        u32,
        u32,
        Option<u64>,
        Option<u64>,
        Option<u8>,
        Option<u8>,
    )> = conn
        .exec_first(
            "SELECT ii.guid, ii.owner_guid, ii.itemEntry, ii.count, \
                    ci.guid, ci.bag, ci.slot, bag_ci.slot \
             FROM item_instance ii \
             LEFT JOIN character_inventory ci \
               ON ci.item = ii.guid \
             LEFT JOIN character_inventory bag_ci \
               ON ci.bag <> 0 AND bag_ci.item = ci.bag AND bag_ci.guid = ci.guid \
             WHERE ii.guid = ?",
            (expected_item_grant.push.item_guid_low,),
        )
        .map_err(|error| anyhow!("Verify wire-keyed loot-race item persistence: {error}"))?;
    let persisted = persisted_tuple
        .map(
            |(
                item_guid,
                owner_guid,
                item_entry,
                count,
                inventory_owner,
                bag_guid,
                slot,
                bag_slot,
            )| PersistedItemGrantRow {
                item_guid,
                owner_guid,
                item_entry,
                count,
                inventory_owner,
                bag_guid,
                slot,
                bag_slot,
            },
        )
        .ok_or_else(|| {
            anyhow!(
                "wire ItemGUID counter {} did not identify a persisted item_instance row",
                expected_item_grant.push.item_guid_low
            )
        })?;
    validate_persisted_item_grant_like_cpp(expected_item_grant, persisted)?;
    let mut money_delta = 0u64;
    for character in &fixture.characters {
        let current = current_money
            .iter()
            .find_map(|(guid, money)| (*guid == character.bot.character_guid).then_some(*money))
            .ok_or_else(|| {
                anyhow!(
                    "loot-race money verification omitted character {}",
                    character.bot.character_guid
                )
            })?;
        let character_delta = if character.bot.character_guid == expected_money_grant.owner_guid {
            expected_money_grant.amount
        } else {
            0
        };
        let expected = character
            .money
            .checked_add(character_delta)
            .ok_or_else(|| anyhow!("loot-race expected money overflow"))?;
        if current != expected {
            bail!(
                "atomic GAMEOBJECT MONEY race persisted character {} money {}; expected {} from wire winner {} and whole-pool amount {}",
                character.bot.character_guid,
                current,
                expected,
                expected_money_grant.owner_guid,
                expected_money_grant.amount
            );
        }
        money_delta = money_delta
            .checked_add(current - character.money)
            .ok_or_else(|| anyhow!("loot-race persisted money delta overflow"))?;
    }
    if money_delta != expected_money_grant.amount {
        bail!(
            "atomic GAMEOBJECT MONEY race persisted total delta {money_delta}; expected one exact C++ whole-pool grant {}",
            expected_money_grant.amount
        );
    }
    Ok((
        item_total,
        money_delta,
        expected_item_grant,
        expected_money_grant,
    ))
}
pub(crate) fn verify_single_item_capture_persistence(fixture: &LootRaceFixture) -> Result<u64> {
    let url = characters_db_url()?;
    let opts = loot_db_opts(&url, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    wait_both_offline(&mut conn, fixture)?;
    ensure_no_online_characters(&mut conn, "loot-item capture persistence verification")?;

    let owner = fixture.characters[0].bot.character_guid;
    let offline_peer = fixture.characters[1].bot.character_guid;
    let (item_total, item_rows, inventory_rows, persisted_owner): (u64, u64, u64, u64) = conn
        .exec_first(
            "SELECT COALESCE(SUM(ii.count), 0), COUNT(DISTINCT ii.guid), \
                    COUNT(DISTINCT ci.item), COALESCE(MIN(ii.owner_guid), 0) \
             FROM item_instance ii \
             LEFT JOIN character_inventory ci \
               ON ci.item = ii.guid AND ci.guid = ii.owner_guid \
             WHERE ii.itemEntry = ? AND ii.owner_guid IN (?, ?)",
            (fixture.target.item_entry, owner, offline_peer),
        )
        .map_err(|error| anyhow!("Verify loot-item capture persistence: {error}"))?
        .ok_or_else(|| anyhow!("loot-item capture aggregate query returned no row"))?;
    if (item_total, item_rows, inventory_rows, persisted_owner) != (1, 1, 1, owner) {
        bail!(
            "single-session item capture persisted quantity/instances/inventory/owner {item_total}/{item_rows}/{inventory_rows}/{persisted_owner}; expected 1/1/1/{owner}"
        );
    }
    let persisted_slots: Vec<(u64, u8)> = conn
        .exec(
            "SELECT ci.bag, ci.slot FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ii.itemEntry = ? AND ii.owner_guid = ? AND ci.guid = ?",
            (fixture.target.item_entry, owner, owner),
        )
        .map_err(|error| anyhow!("Verify loot-item capture keyring slot: {error}"))?;
    if persisted_slots.as_slice() != [(0, LOOT_ITEM_CAPTURE_KEYRING_SLOT)] {
        bail!(
            "single-session item capture persisted item in slots {persisted_slots:?}; expected exact top-level keyring slot 0/{LOOT_ITEM_CAPTURE_KEYRING_SLOT}"
        );
    }

    let money_rows: Vec<(u64, u64)> = conn
        .exec(
            "SELECT guid, money FROM characters WHERE guid IN (?, ?) ORDER BY guid",
            (owner, offline_peer),
        )
        .map_err(|error| anyhow!("Verify loot-item capture money isolation: {error}"))?;
    if money_rows.len() != 2 {
        bail!("loot-item capture character rows disappeared during verification");
    }
    for character in &fixture.characters {
        let current = money_rows
            .iter()
            .find_map(|(guid, money)| (*guid == character.bot.character_guid).then_some(*money))
            .ok_or_else(|| {
                anyhow!(
                    "loot-item capture money verification omitted character {}",
                    character.bot.character_guid
                )
            })?;
        if current != character.money {
            bail!(
                "item-only capture changed character {} money from {} to {} without CMSG_LOOT_MONEY",
                character.bot.character_guid,
                character.money,
                current
            );
        }
    }

    Ok(item_total)
}
pub(crate) fn wait_both_offline(conn: &mut mysql::Conn, fixture: &LootRaceFixture) -> Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(LOOT_FIXTURE_OFFLINE_WAIT_SECS);
    loop {
        let online: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM characters WHERE guid IN (?, ?) AND online <> 0",
                (
                    fixture.characters[0].bot.character_guid,
                    fixture.characters[1].bot.character_guid,
                ),
            )
            .map_err(|error| anyhow!("Check loot-race offline state: {error}"))?
            .unwrap_or(2);
        if online == 0 {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "loot-race characters remained online; refusing fixture restoration before disconnect saves"
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

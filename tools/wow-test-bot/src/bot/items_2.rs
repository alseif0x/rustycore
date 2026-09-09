//! Items operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) async fn run_inventory_swap_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    item_entry_a: u32,
    item_entry_b: u32,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_inventory_swap_smoke_fixture(
            &bot_for_setup,
            item_entry_a,
            item_entry_b,
            timeout_secs,
        )
    })
    .await
    .map_err(|e| anyhow!("Inventory swap smoke setup DB worker join failed: {e}"))??;

    let mut forward_options = fixture.options.clone();
    forward_options.phase = InventorySwapSmokePhase::Forward;
    let first = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        Some(forward_options),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;

    let mut combined = match first {
        Ok(result) => result,
        Err(error) => {
            let bot_for_cleanup = bot.clone();
            let fixture_for_cleanup = fixture.clone();
            let _ = tokio::task::spawn_blocking(move || {
                cleanup_inventory_swap_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
            })
            .await;
            return Err(error.context("Inventory swap forward login/phase failed"));
        }
    };

    if combined.inventory_swap_smoke_passed.unwrap_or(false) {
        let mut reverse_options = fixture.options.clone();
        reverse_options.phase = InventorySwapSmokePhase::Reverse;
        match run_bot(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            None,
            None,
            Some(reverse_options),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        {
            Ok(second) => {
                let expected_item_create_sha256 =
                    combined.inventory_swap_item_create_sha256.clone();
                let forward_relogin_hash_verified = expected_item_create_sha256.is_some()
                    && expected_item_create_sha256 == second.inventory_swap_item_create_sha256;
                let reverse_phase_passed = second.inventory_swap_smoke_passed.unwrap_or(false);
                combined.world_auth &= second.world_auth;
                combined.enum_characters &= second.enum_characters;
                combined.player_login_verified &= second.player_login_verified;
                combined.inventory_swap_relogin_after_forward =
                    second.inventory_swap_relogin_after_forward;
                combined.inventory_swap_reverse_persisted = second.inventory_swap_reverse_persisted;
                combined.inventory_swap_item_metadata_persisted &=
                    second.inventory_swap_item_metadata_persisted;
                combined.seen_opcodes.extend(second.seen_opcodes);
                combined.inventory_swap_failure = second.inventory_swap_failure;
                combined.inventory_swap_smoke_passed = Some(false);

                if combined.inventory_swap_forward_persisted
                    && combined.inventory_swap_relogin_after_forward
                    && combined.inventory_swap_reverse_persisted
                    && combined.inventory_swap_item_metadata_persisted
                    && forward_relogin_hash_verified
                    && reverse_phase_passed
                {
                    let mut verify_reverse_options = fixture.options.clone();
                    verify_reverse_options.phase = InventorySwapSmokePhase::VerifyReverseRelog;
                    match run_bot(
                        bot.clone(),
                        dungeon_id,
                        lfg_secs,
                        auto_teleport,
                        false,
                        None,
                        None,
                        None,
                        Some(verify_reverse_options),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                    .await
                    {
                        Ok(third) => {
                            let reverse_relogin_hash_verified = expected_item_create_sha256
                                .is_some()
                                && expected_item_create_sha256
                                    == third.inventory_swap_item_create_sha256;
                            combined.world_auth &= third.world_auth;
                            combined.enum_characters &= third.enum_characters;
                            combined.player_login_verified &= third.player_login_verified;
                            combined.inventory_swap_relogin_after_reverse =
                                third.inventory_swap_relogin_after_reverse;
                            combined.inventory_swap_item_create_relogin_verified =
                                forward_relogin_hash_verified && reverse_relogin_hash_verified;
                            combined.inventory_swap_item_metadata_persisted &=
                                third.inventory_swap_item_metadata_persisted;
                            combined.seen_opcodes.extend(third.seen_opcodes);
                            combined.inventory_swap_failure = third.inventory_swap_failure;
                            combined.inventory_swap_smoke_passed = Some(
                                combined.inventory_swap_relogin_after_reverse
                                    && combined.inventory_swap_item_create_relogin_verified
                                    && combined.inventory_swap_item_metadata_persisted
                                    && third.inventory_swap_smoke_passed.unwrap_or(false),
                            );
                        }
                        Err(error) => {
                            combined.inventory_swap_failure = Some(format!(
                                "Inventory swap post-reverse relog verification failed: {error}"
                            ));
                        }
                    }
                } else if combined.inventory_swap_failure.is_none() {
                    combined.inventory_swap_failure = Some(
                        "Inventory swap reverse phase did not satisfy its relog/hash/persistence proof"
                            .to_string(),
                    );
                }
            }
            Err(error) => {
                combined.inventory_swap_failure = Some(format!(
                    "Inventory swap reverse relog/phase failed: {error}"
                ));
                combined.inventory_swap_smoke_passed = Some(false);
            }
        }
    }

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_inventory_swap_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|e| anyhow!("Inventory swap cleanup DB worker join failed: {e}"))?;
    if let Err(error) = cleanup {
        combined.inventory_swap_failure = Some(format!("Inventory swap cleanup failed: {error}"));
        combined.inventory_swap_smoke_passed = Some(false);
    }

    Ok(combined)
}
pub(crate) async fn run_inventory_swap_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &InventorySwapSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let (expected_before_a, expected_before_b, expected_after_a, expected_after_b) =
        match options.phase {
            InventorySwapSmokePhase::Forward => (
                options.slot_a,
                options.slot_b,
                options.slot_b,
                options.slot_a,
            ),
            InventorySwapSmokePhase::Reverse => {
                result.inventory_swap_relogin_after_forward = true;
                (
                    options.slot_b,
                    options.slot_a,
                    options.slot_a,
                    options.slot_b,
                )
            }
            InventorySwapSmokePhase::VerifyReverseRelog => (
                options.slot_a,
                options.slot_b,
                options.slot_a,
                options.slot_b,
            ),
        };

    let bot_for_before = bot.clone();
    let options_for_before = options.clone();
    let before = tokio::task::spawn_blocking(move || {
        verify_inventory_swap_fixture_locations(
            &bot_for_before,
            &options_for_before,
            expected_before_a,
            expected_before_b,
        )
    })
    .await
    .map_err(|e| anyhow!("Inventory swap pre-phase DB worker join failed: {e}"))??;
    if !before {
        bail!(
            "inventory swap fixture was not in expected slots {expected_before_a}/{expected_before_b} before {:?}",
            options.phase
        );
    }

    if options.phase == InventorySwapSmokePhase::VerifyReverseRelog {
        let logout_complete_seen = loot_race::logout_and_wait_routed_like_cpp(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection.as_mut(),
            bot.character_guid,
            result,
        )
        .await?;
        if !logout_complete_seen {
            bail!(
                "inventory swap post-reverse relog did not observe SMSG_LOGOUT_COMPLETE on either socket"
            );
        }
        result.inventory_swap_relogin_after_reverse = true;
        result.inventory_swap_item_metadata_persisted = true;
        result.inventory_swap_smoke_passed = Some(true);
        return Ok(());
    }

    if options.phase == InventorySwapSmokePhase::Forward {
        verify_inventory_swap_invalid_position_gate(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection,
            options.slot_a,
            options.timeout_secs,
            result,
        )
        .await?;
        result.inventory_swap_validation_gate_seen = true;
    }

    let payload = build_swap_inv_item_payload(options.slot_a, options.slot_b);
    send_encrypted_packet(stream, crypt, CMSG_SWAP_INV_ITEM, &payload).await?;
    info!(
        "[Bot {}] ✅ CMSG_SWAP_INV_ITEM sent slot {} -> {} ({:?})",
        bot_index, options.slot_a, options.slot_b, options.phase
    );

    let logout_complete_seen = loot_race::logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection.as_mut(),
        bot.character_guid,
        result,
    )
    .await?;
    if !logout_complete_seen {
        bail!(
            "inventory swap {:?} phase did not observe SMSG_LOGOUT_COMPLETE on either socket",
            options.phase
        );
    }

    // C++ marks both items changed in memory and persists them during the
    // logout save transaction. Polling CharacterDB before logout would test a
    // Rust-only implementation detail rather than the reference lifecycle.
    wait_for_inventory_swap_locations(
        bot,
        options,
        expected_after_a,
        expected_after_b,
        options.timeout_secs,
    )
    .await?;
    result.inventory_swap_item_metadata_persisted = true;

    match options.phase {
        InventorySwapSmokePhase::Forward => result.inventory_swap_forward_persisted = true,
        InventorySwapSmokePhase::Reverse => result.inventory_swap_reverse_persisted = true,
        InventorySwapSmokePhase::VerifyReverseRelog => {
            unreachable!("post-reverse relog returns before sending a swap")
        }
    }
    result.inventory_swap_smoke_passed = Some(true);
    Ok(())
}
pub(crate) async fn wait_for_inventory_swap_locations(
    bot: &config::BotConfig,
    options: &InventorySwapSmokeOptions,
    expected_slot_a: u8,
    expected_slot_b: u8,
    timeout_secs: u64,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        let bot_for_check = bot.clone();
        let options_for_check = options.clone();
        let matches = tokio::task::spawn_blocking(move || {
            verify_inventory_swap_fixture_locations(
                &bot_for_check,
                &options_for_check,
                expected_slot_a,
                expected_slot_b,
            )
        })
        .await
        .map_err(|e| anyhow!("Inventory swap polling DB worker join failed: {e}"))??;
        if matches {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("timed out waiting for inventory swap DB slots {expected_slot_a}/{expected_slot_b}")
}
pub(crate) async fn wait_for_bank_item_location(
    bot_index: usize,
    bot: &config::BotConfig,
    item_guid: u64,
    expected_slot: u8,
    timeout_secs: u64,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        let bot_for_db = bot.clone();
        let located = tokio::task::spawn_blocking(move || {
            verify_bank_fixture_location(&bot_for_db, item_guid, expected_slot)
        })
        .await
        .map_err(|e| anyhow!("Bank location DB worker join failed: {e}"))??;
        if located {
            info!(
                "[Bot {}] ✅ fixture item {} reached persisted slot {}",
                bot_index, item_guid, expected_slot
            );
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!(
        "timed out waiting for fixture item {} in slot {}",
        item_guid,
        expected_slot
    )
}
pub(crate) async fn wait_for_vendor_inventory_item(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    expected_vendor_guid: &[u8],
    options: &VendorSmokeOptions,
    result: &mut BotRunResult,
) -> Result<VendorInventoryItemWire> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for SMSG_VENDOR_INVENTORY");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for SMSG_VENDOR_INVENTORY"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 vendor-list {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            continue;
        }
        if opcode != SMSG_VENDOR_INVENTORY {
            continue;
        }

        let items = parse_vendor_inventory(&payload, expected_vendor_guid)?;
        let item = items
            .into_iter()
            .find(|item| {
                item.item_id == options.item_entry as i32
                    && item.extended_cost == options.extended_cost as i32
            })
            .ok_or_else(|| {
                anyhow!(
                    "vendor inventory omitted expected item/cost {}/{}",
                    options.item_entry,
                    options.extended_cost
                )
            })?;
        if item.item_type != 1 || item.muid <= 0 || item.price != 0 || item.stack_count != 1 {
            bail!(
                "vendor item {} wire row is not the deterministic fixture shape: {item:?}",
                options.item_entry
            );
        }
        result.vendor_inventory_seen = true;
        return Ok(item);
    }
}
pub(crate) fn parse_vendor_inventory(
    payload: &[u8],
    expected_vendor_guid: &[u8],
) -> Result<Vec<VendorInventoryItemWire>> {
    let (guid_len, low, high) = parse_packed_guid(payload)
        .ok_or_else(|| anyhow!("SMSG_VENDOR_INVENTORY has an invalid packed vendor GUID"))?;
    let (expected_len, expected_low, expected_high) = parse_packed_guid(expected_vendor_guid)
        .ok_or_else(|| anyhow!("fixture has an invalid packed vendor GUID"))?;
    if guid_len != expected_len || low != expected_low || high != expected_high {
        bail!("SMSG_VENDOR_INVENTORY names a different vendor GUID");
    }
    let mut cursor = guid_len;
    let reason = *payload
        .get(cursor)
        .ok_or_else(|| anyhow!("SMSG_VENDOR_INVENTORY omitted reason"))?;
    cursor += 1;
    if reason != 0 {
        bail!("SMSG_VENDOR_INVENTORY returned reason {reason}");
    }
    let count = take_vendor_u32(payload, &mut cursor)?;
    let count = usize::try_from(count).map_err(|_| anyhow!("vendor item count overflow"))?;
    if count > 300 {
        bail!("SMSG_VENDOR_INVENTORY item count {count} exceeds C++ vendor bound");
    }
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let price = take_vendor_u64(payload, &mut cursor)?;
        let muid = take_vendor_i32(payload, &mut cursor)?;
        let item_type = take_vendor_i32(payload, &mut cursor)?;
        let _durability = take_vendor_i32(payload, &mut cursor)?;
        let stack_count = take_vendor_i32(payload, &mut cursor)?;
        let _quantity = take_vendor_i32(payload, &mut cursor)?;
        let extended_cost = take_vendor_i32(payload, &mut cursor)?;
        let _player_condition_failed = take_vendor_i32(payload, &mut cursor)?;
        cursor = cursor
            .checked_add(1)
            .filter(|next| *next <= payload.len())
            .ok_or_else(|| anyhow!("SMSG_VENDOR_INVENTORY omitted vendor flags"))?;
        let item_id = take_vendor_i32(payload, &mut cursor)?;
        let _random_seed = take_vendor_i32(payload, &mut cursor)?;
        let _random_property = take_vendor_i32(payload, &mut cursor)?;
        let has_bonus_bits = *payload
            .get(cursor)
            .ok_or_else(|| anyhow!("vendor ItemInstance omitted bonus bit"))?;
        cursor += 1;
        let mod_count_bits = *payload
            .get(cursor)
            .ok_or_else(|| anyhow!("vendor ItemInstance omitted modifier count"))?;
        cursor += 1;
        if has_bonus_bits != 0 || mod_count_bits != 0 {
            bail!(
                "vendor ItemInstance for item {item_id} has unsupported bonus/modifier bits 0x{has_bonus_bits:02X}/0x{mod_count_bits:02X}"
            );
        }
        items.push(VendorInventoryItemWire {
            muid,
            item_id,
            item_type,
            price,
            stack_count,
            extended_cost,
        });
    }
    if cursor != payload.len() {
        bail!(
            "SMSG_VENDOR_INVENTORY has {} trailing bytes after {} items",
            payload.len() - cursor,
            count
        );
    }
    Ok(items)
}
pub(crate) fn build_vendor_buy_item_payload(
    vendor_guid: &[u8],
    character_guid: u64,
    muid: i32,
    item_entry: u32,
) -> Vec<u8> {
    let (player_low, player_high) = create_player_guid_raw(character_guid, realm_id());
    let mut payload = Vec::with_capacity(vendor_guid.len() + 48);
    payload.extend_from_slice(vendor_guid);
    payload.extend(build_packed_guid(player_low, player_high));
    payload.extend_from_slice(&1i32.to_le_bytes());
    payload.extend_from_slice(&muid.to_le_bytes());
    payload.extend_from_slice(&i32::from(u8::MAX).to_le_bytes());
    payload.extend_from_slice(&1i32.to_le_bytes());
    payload.extend_from_slice(&(item_entry as i32).to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes());
    payload.push(0);
    payload.push(0);
    payload
}
pub(crate) fn build_auto_bank_item_payload(slot: u8) -> [u8; 5] {
    // C++ InvUpdate count=1 is two MSB-first bits `01`, followed by the
    // affected position and then the packet's source bag/slot.
    [0x40, INVENTORY_SLOT_BAG_0, slot, INVENTORY_SLOT_BAG_0, slot]
}
pub(crate) fn build_swap_inv_item_payload(src_slot: u8, dst_slot: u8) -> [u8; 7] {
    // C++ InvUpdate count=2 is two MSB-first bits `10`. The real 3.4.3
    // client lists destination then source, followed by Slot2/Slot1.
    [
        0x80,
        INVENTORY_SLOT_BAG_0,
        dst_slot,
        INVENTORY_SLOT_BAG_0,
        src_slot,
        dst_slot,
        src_slot,
    ]
}
pub(crate) fn build_swap_item_invalid_source_payload(valid_destination_slot: u8) -> [u8; 9] {
    // InvUpdate contains the destination then source. The packet body follows
    // C++ order ContainerSlotB/A, SlotB/A; container 200 is never a valid
    // player bag position and must produce EQUIP_ERR_ITEM_NOT_FOUND.
    [
        0x80,
        INVENTORY_SLOT_BAG_0,
        valid_destination_slot,
        200,
        0,
        INVENTORY_SLOT_BAG_0,
        200,
        valid_destination_slot,
        0,
    ]
}
pub(crate) fn quest_details_or_request_items_seen(result: &BotRunResult) -> bool {
    result.quest_details_seen || result.quest_request_items_seen
}
pub(crate) fn issue20_item_enchantments_db_string() -> String {
    let mut fields = Vec::with_capacity(ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT * 3);
    for id in issue20_expected_enchantment_ids() {
        fields.extend([id.to_string(), "0".to_string(), "0".to_string()]);
    }
    fields.join(" ")
}
pub(crate) fn issue20_item_metadata_matches_db_like_cpp(
    enchantments: &str,
    random_properties_id: i32,
    random_properties_seed: i32,
) -> bool {
    let parsed = enchantments
        .split_whitespace()
        .map(str::parse::<i32>)
        .collect::<std::result::Result<Vec<_>, _>>();
    let Ok(parsed) = parsed else {
        return false;
    };
    parsed
        == issue20_item_enchantments_db_string()
            .split_whitespace()
            .map(|value| value.parse::<i32>().expect("static issue #20 enchantment"))
            .collect::<Vec<_>>()
        && random_properties_id == ISSUE20_ITEM_RANDOM_PROPERTY_ID
        && random_properties_seed == 0
}
pub(crate) fn issue20_item_has_zero_metadata_db_like_cpp(
    enchantments: &str,
    random_properties_id: i32,
    random_properties_seed: i32,
) -> bool {
    let parsed = enchantments
        .split_whitespace()
        .map(str::parse::<i32>)
        .collect::<std::result::Result<Vec<_>, _>>();
    matches!(
        parsed,
        Ok(values)
            if values.len() == ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT * 3
                && values.iter().all(|value| *value == 0)
                && random_properties_id == 0
                && random_properties_seed == 0
    )
}
pub(crate) fn prepare_inventory_swap_smoke_fixture(
    bot: &config::BotConfig,
    item_entry_a: u32,
    item_entry_b: u32,
    timeout_secs: u64,
) -> Result<InventorySwapSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive inventory swap fixture setup for non-local account {}",
            bot.account
        );
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    let character: Option<(u32, u8)> = characters
        .exec_first(
            "SELECT account, online FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load inventory swap bot character: {e}"))?;
    let (owner, online) =
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
            "character {} is online; log it out before inventory swap smoke setup",
            bot.character_guid
        );
    }

    let occupied_slots: Vec<u8> = characters
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (bot.character_guid,),
            |slot: u8| slot,
        )
        .map_err(|e| anyhow!("Load occupied inventory swap slots: {e}"))?;
    let free_slots: Vec<u8> = (INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
        .filter(|slot| !occupied_slots.contains(slot))
        .take(2)
        .collect();
    if free_slots.len() != 2 {
        bail!("Two empty default backpack slots are required for inventory swap smoke");
    }
    let slot_a = free_slots[0];
    let slot_b = free_slots[1];

    for item_entry in [item_entry_a, item_entry_b] {
        let owned_count: u64 = characters
            .exec_first(
                "SELECT COUNT(*) FROM character_inventory ci \
                 JOIN item_instance ii ON ii.guid = ci.item \
                 WHERE ci.guid = ? AND ii.itemEntry = ?",
                (bot.character_guid, item_entry),
            )
            .map_err(|e| anyhow!("Check existing inventory swap item entry: {e}"))?
            .unwrap_or(0);
        if owned_count != 0 {
            bail!(
                "bot character already owns item entry {item_entry}; choose isolated inventory-swap item entries"
            );
        }
    }

    let max_item_guid: u64 = characters
        .query_first("SELECT COALESCE(MAX(guid), 0) FROM item_instance")
        .map_err(|e| anyhow!("Load max item guid: {e}"))?
        .unwrap_or(0);
    let item_guid_a = max_item_guid
        .checked_add(20_000)
        .ok_or_else(|| anyhow!("item guid overflow while reserving inventory swap fixture"))?;
    let item_guid_b = item_guid_a
        .checked_add(1)
        .ok_or_else(|| anyhow!("item guid overflow while reserving inventory swap fixture"))?;

    let mut transaction = characters
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start inventory swap fixture transaction: {e}"))?;
    for (item_guid, item_entry, slot) in [
        (item_guid_a, item_entry_a, slot_a),
        (item_guid_b, item_entry_b, slot_b),
    ] {
        let (enchantments, random_properties_id) = if item_guid == item_guid_a {
            (
                issue20_item_enchantments_db_string(),
                ISSUE20_ITEM_RANDOM_PROPERTY_ID,
            )
        } else {
            (issue20_zero_enchantments_db_string(), 0)
        };
        transaction
            .exec_drop(
                "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, durability, \
                  enchantments, charges, flags, randomPropertiesId, randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, 1, 0, ?, '', 0, ?, 0, 0)",
                (
                    item_guid,
                    item_entry,
                    bot.character_guid,
                    enchantments,
                    random_properties_id,
                ),
            )
            .map_err(|e| anyhow!("Insert inventory swap fixture item: {e}"))?;
        transaction
            .exec_drop(
                "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)",
                (bot.character_guid, slot, item_guid),
            )
            .map_err(|e| anyhow!("Insert inventory swap fixture inventory row: {e}"))?;
    }
    transaction
        .commit()
        .map_err(|e| anyhow!("Commit inventory swap fixture transaction: {e}"))?;

    info!(
        "Inventory swap fixture: character={} items={}/{} entries={}/{} slots={}/{}",
        bot.character_guid, item_guid_a, item_guid_b, item_entry_a, item_entry_b, slot_a, slot_b
    );
    Ok(InventorySwapSmokeFixture {
        options: InventorySwapSmokeOptions {
            phase: InventorySwapSmokePhase::Forward,
            item_guid_a,
            item_guid_b,
            item_entry_a,
            item_entry_b,
            slot_a,
            slot_b,
            timeout_secs,
        },
    })
}
pub(crate) fn verify_inventory_swap_fixture_locations(
    bot: &config::BotConfig,
    options: &InventorySwapSmokeOptions,
    expected_slot_a: u8,
    expected_slot_b: u8,
) -> Result<bool> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    let load = |conn: &mut mysql::Conn,
                item_guid: u64|
     -> Result<Option<(u64, u8, u32, u64, String, i32, i32)>> {
        conn.exec_first(
            "SELECT ci.bag, ci.slot, ii.itemEntry, ii.owner_guid, ii.enchantments, \
                    ii.randomPropertiesId, ii.randomPropertiesSeed \
             FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ci.item = ? AND ii.count = 1",
            (bot.character_guid, item_guid),
        )
        .map_err(|e| anyhow!("Load inventory swap fixture location: {e}"))
    };
    let row_a = load(&mut conn, options.item_guid_a)?;
    let row_b = load(&mut conn, options.item_guid_b)?;
    Ok(matches!(
        row_a,
        Some((0, slot, entry, owner, ref enchantments, random_properties_id, random_properties_seed))
            if slot == expected_slot_a
                && entry == options.item_entry_a
                && owner == bot.character_guid
                && issue20_item_metadata_matches_db_like_cpp(
                    enchantments,
                    random_properties_id,
                    random_properties_seed,
                )
    ) && matches!(
        row_b,
        Some((0, slot, entry, owner, ref enchantments, random_properties_id, random_properties_seed))
            if slot == expected_slot_b
                && entry == options.item_entry_b
                && owner == bot.character_guid
                && issue20_item_has_zero_metadata_db_like_cpp(
                    enchantments,
                    random_properties_id,
                    random_properties_seed,
                )
    ))
}
pub(crate) fn cleanup_inventory_swap_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &InventorySwapSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    let offline_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|e| anyhow!("Check inventory swap bot offline state before cleanup: {e}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < offline_deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => {
                bail!(
                    "character {} remained online; refusing inventory swap cleanup before disconnect save",
                    bot.character_guid
                );
            }
            None => bail!(
                "No characters row for guid {} during inventory swap cleanup",
                bot.character_guid
            ),
        }
    }

    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start inventory swap cleanup transaction: {e}"))?;
    for item_guid in [fixture.options.item_guid_a, fixture.options.item_guid_b] {
        transaction
            .exec_drop(
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
                (bot.character_guid, item_guid),
            )
            .map_err(|e| anyhow!("Delete inventory swap fixture inventory row: {e}"))?;
        transaction
            .exec_drop(
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?",
                (item_guid, bot.character_guid),
            )
            .map_err(|e| anyhow!("Delete inventory swap fixture item: {e}"))?;
    }
    transaction
        .commit()
        .map_err(|e| anyhow!("Commit inventory swap cleanup transaction: {e}"))?;
    Ok(())
}

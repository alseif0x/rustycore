//! Void storage operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) async fn run_void_storage_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_void_storage_smoke_fixture(
            &bot_for_setup,
            item_entry,
            runtime_counter,
            timeout_secs,
        )
    })
    .await
    .map_err(|error| anyhow!("Void-storage fixture setup worker failed: {error}"))??;

    let workflow = async {
        let mut unlock_deposit = fixture.options.clone();
        unlock_deposit.phase = VoidStorageSmokePhase::UnlockDeposit;
        let mut combined = run_bot_with_void_storage(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(unlock_deposit),
            None,
            None,
            None,
        )
        .await?;
        if !combined.void_storage_smoke_passed.unwrap_or(false) {
            return Ok(combined);
        }
        let void_item_id = combined
            .void_storage_item_id
            .context("void-storage deposit response omitted its generated item ID")?;

        let phases = [
            VoidStorageSmokePhase::VerifyDepositSwap,
            VoidStorageSmokePhase::VerifySwapWithdraw,
            VoidStorageSmokePhase::VerifyWithdraw,
        ];
        for phase in phases {
            let mut options = fixture.options.clone();
            options.phase = phase;
            options.expected_void_item_id = Some(void_item_id);
            options.expected_void_slot = match phase {
                VoidStorageSmokePhase::VerifyDepositSwap => 0,
                VoidStorageSmokePhase::VerifySwapWithdraw => 5,
                VoidStorageSmokePhase::VerifyWithdraw => 0,
                VoidStorageSmokePhase::UnlockDeposit | VoidStorageSmokePhase::QueryCapture => {
                    unreachable!()
                }
            };
            let next = run_bot_with_void_storage(
                bot.clone(),
                dungeon_id,
                lfg_secs,
                auto_teleport,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(options),
                None,
                None,
                None,
            )
            .await?;
            combined.world_auth &= next.world_auth;
            combined.enum_characters &= next.enum_characters;
            combined.player_login_verified &= next.player_login_verified;
            combined.void_storage_deposit_relogin_verified |=
                next.void_storage_deposit_relogin_verified;
            combined.void_storage_swap_persisted |= next.void_storage_swap_persisted;
            combined.void_storage_swap_relogin_verified |= next.void_storage_swap_relogin_verified;
            combined.void_storage_withdraw_persisted |= next.void_storage_withdraw_persisted;
            combined.void_storage_withdraw_relogin_verified |=
                next.void_storage_withdraw_relogin_verified;
            combined.seen_opcodes.extend(next.seen_opcodes);
            if !next.void_storage_smoke_passed.unwrap_or(false) {
                combined.void_storage_failure = next.void_storage_failure;
                combined.void_storage_smoke_passed = Some(false);
                break;
            }
        }
        combined.void_storage_smoke_passed = Some(
            combined.void_storage_unlock_persisted
                && combined.void_storage_deposit_persisted
                && combined.void_storage_deposit_relogin_verified
                && combined.void_storage_swap_persisted
                && combined.void_storage_swap_relogin_verified
                && combined.void_storage_withdraw_persisted
                && combined.void_storage_withdraw_relogin_verified,
        );
        Ok::<_, anyhow::Error>(combined)
    }
    .await;

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_void_storage_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|error| anyhow!("Void-storage cleanup worker failed: {error}"))?;

    match (workflow, cleanup) {
        (Ok(result), Ok(())) => Ok(result),
        (Ok(mut result), Err(error)) => {
            result.void_storage_smoke_passed = Some(false);
            result.void_storage_failure = Some(format!("fixture cleanup failed: {error}"));
            Ok(result)
        }
        (Err(error), Ok(())) => Err(error),
        (Err(workflow_error), Err(cleanup_error)) => Err(anyhow!(
            "Void-storage workflow failed: {workflow_error}; cleanup failed: {cleanup_error}"
        )),
    }
}
pub(crate) async fn run_void_storage_query_capture_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_void_storage_query_capture_fixture(
            &bot_for_setup,
            item_entry,
            runtime_counter,
            timeout_secs,
        )
    })
    .await
    .map_err(|error| anyhow!("Void-storage query fixture setup worker failed: {error}"))??;

    let workflow = run_bot_with_void_storage(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(fixture.options.clone()),
        None,
        None,
        None,
    )
    .await;

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_void_storage_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|error| anyhow!("Void-storage query cleanup worker failed: {error}"))?;

    match (workflow, cleanup) {
        (Ok(result), Ok(())) => Ok(result),
        (Ok(mut result), Err(error)) => {
            result.void_storage_query_capture_passed = Some(false);
            result.void_storage_failure = Some(format!("fixture cleanup failed: {error}"));
            Ok(result)
        }
        (Err(error), Ok(())) => Err(error),
        (Err(workflow_error), Err(cleanup_error)) => Err(anyhow!(
            "Void-storage query workflow failed: {workflow_error}; cleanup failed: {cleanup_error}"
        )),
    }
}
pub(crate) fn build_void_storage_transfer_payload(
    target: &ResolvedCreatureTarget,
    runtime_realm_id: u16,
    deposits: &[(u64, u64)],
    withdrawals: &[(u64, u64)],
) -> Vec<u8> {
    let mut payload = vault_keeper_packed_guid(target, runtime_realm_id);
    payload.extend_from_slice(&(deposits.len() as u32).to_le_bytes());
    payload.extend_from_slice(&(withdrawals.len() as u32).to_le_bytes());
    for &(low, high) in deposits.iter().chain(withdrawals) {
        payload.extend(build_packed_guid(low, high));
    }
    payload
}
pub(crate) fn build_void_storage_swap_payload(
    target: &ResolvedCreatureTarget,
    runtime_realm_id: u16,
    void_item_id: u64,
    dst_slot: u32,
) -> Vec<u8> {
    let mut payload = vault_keeper_packed_guid(target, runtime_realm_id);
    let (low, high) = item_guid_raw(void_item_id, runtime_realm_id);
    payload.extend(build_packed_guid(low, high));
    payload.extend_from_slice(&dst_slot.to_le_bytes());
    payload
}
pub(crate) fn parse_void_storage_contents(payload: &[u8]) -> Result<Vec<VoidStorageItemWire>> {
    let count = usize::from(
        *payload
            .first()
            .ok_or_else(|| anyhow!("empty SMSG_VOID_STORAGE_CONTENTS"))?,
    );
    let mut cursor = 1;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(parse_void_item_wire(payload, &mut cursor)?);
    }
    if cursor != payload.len() {
        bail!(
            "SMSG_VOID_STORAGE_CONTENTS left {} trailing bytes",
            payload.len() - cursor
        );
    }
    Ok(items)
}
pub(crate) async fn read_void_storage_packet(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    deadline: tokio::time::Instant,
    result: &mut BotRunResult,
) -> Result<(u16, Vec<u8>)> {
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for void-storage response");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for void-storage response"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 void-storage {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            continue;
        }
        return Ok((opcode, payload));
    }
}
pub(crate) async fn query_void_storage_contents(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    target: &ResolvedCreatureTarget,
    runtime_realm_id: u16,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<Vec<VoidStorageItemWire>> {
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_QUERY_VOID_STORAGE,
        &vault_keeper_packed_guid(target, runtime_realm_id),
    )
    .await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let (opcode, payload) =
            read_void_storage_packet(bot_index, stream, crypt, server_inflater, deadline, result)
                .await?;
        match opcode {
            SMSG_VOID_STORAGE_CONTENTS => return parse_void_storage_contents(&payload),
            SMSG_VOID_STORAGE_FAILED => bail!("void-storage query returned failure"),
            _ => {}
        }
    }
}
pub(crate) async fn wait_for_void_storage_transfer(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    expect_added: bool,
    expected_item_id: Option<u64>,
    result: &mut BotRunResult,
) -> Result<Option<VoidStorageItemWire>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut changed_item = None;
    let mut changes_seen = false;
    let mut success_seen = false;
    while !changes_seen || !success_seen {
        let (opcode, payload) =
            read_void_storage_packet(bot_index, stream, crypt, server_inflater, deadline, result)
                .await?;
        match opcode {
            SMSG_VOID_STORAGE_TRANSFER_CHANGES => {
                let counts = *payload
                    .first()
                    .ok_or_else(|| anyhow!("empty void-storage transfer changes"))?;
                let added_count = usize::from(counts >> 4);
                let removed_count = usize::from(counts & 0x0F);
                let expected_counts = if expect_added { (1, 0) } else { (0, 1) };
                if (added_count, removed_count) != expected_counts {
                    bail!(
                        "unexpected void-storage change counts {added_count}/{removed_count}, expected {}/{}",
                        expected_counts.0,
                        expected_counts.1
                    );
                }
                let mut cursor = 1;
                if expect_added {
                    changed_item = Some(parse_void_item_wire(&payload, &mut cursor)?);
                } else {
                    let (guid_len, removed_id, _) = parse_packed_guid(&payload[cursor..])
                        .ok_or_else(|| anyhow!("withdrawal change packet has invalid GUID"))?;
                    if Some(removed_id) != expected_item_id {
                        bail!(
                            "withdrawal removed void item {removed_id}, expected {:?}",
                            expected_item_id
                        );
                    }
                    cursor += guid_len;
                }
                if cursor != payload.len() {
                    bail!("void-storage transfer changes left trailing bytes");
                }
                changes_seen = true;
            }
            SMSG_VOID_TRANSFER_RESULT => {
                if payload.len() != 4 || i32::from_le_bytes(payload[..4].try_into()?) != 0 {
                    bail!("void-storage transfer returned nonzero or malformed result");
                }
                success_seen = true;
            }
            _ => {}
        }
    }
    Ok(changed_item)
}
pub(crate) async fn wait_for_void_storage_swap(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    expected_item_id: u64,
    expected_slot: u32,
    result: &mut BotRunResult,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let (opcode, payload) =
            read_void_storage_packet(bot_index, stream, crypt, server_inflater, deadline, result)
                .await?;
        if opcode == SMSG_VOID_STORAGE_FAILED || opcode == SMSG_VOID_TRANSFER_RESULT {
            bail!("void-storage swap returned failure opcode 0x{opcode:04X}");
        }
        if opcode != SMSG_VOID_ITEM_SWAP_RESPONSE {
            continue;
        }
        let mut cursor = 0;
        let (item_guid_len, item_id, _) = parse_packed_guid(&payload[cursor..])
            .ok_or_else(|| anyhow!("void-storage swap response has invalid item GUID"))?;
        cursor += item_guid_len;
        let slot = payload
            .get(cursor..cursor + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| anyhow!("void-storage swap response omitted item slot"))?;
        cursor += 4;
        let (destination_guid_len, destination_low, _) = parse_packed_guid(&payload[cursor..])
            .ok_or_else(|| anyhow!("void-storage swap response has invalid destination GUID"))?;
        cursor += destination_guid_len;
        let destination_slot = payload
            .get(cursor..cursor + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| anyhow!("void-storage swap response omitted destination slot"))?;
        cursor += 4;
        if item_id != expected_item_id
            || slot != expected_slot
            || destination_low != 0
            || destination_slot != 0
            || cursor != payload.len()
        {
            bail!(
                "unexpected void-storage swap response item/slot/destination {item_id}/{slot}/{destination_low}/{destination_slot}"
            );
        }
        return Ok(());
    }
}
pub(crate) async fn wait_for_void_storage_db_state<F>(
    bot: &config::BotConfig,
    item_entry: u32,
    timeout_secs: u64,
    description: &str,
    predicate: F,
) -> Result<VoidStorageDbState>
where
    F: Fn(&VoidStorageDbState) -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let bot_for_db = bot.clone();
        let state = tokio::task::spawn_blocking(move || {
            load_void_storage_db_state(&bot_for_db, item_entry)
        })
        .await
        .map_err(|error| anyhow!("Void-storage DB worker failed: {error}"))??;
        if predicate(&state) {
            return Ok(state);
        }
        if tokio::time::Instant::now() >= deadline {
            bail!("timed out waiting for {description}; last state: {state:?}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
pub(crate) async fn run_void_storage_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &VoidStorageSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    match options.phase {
        VoidStorageSmokePhase::UnlockDeposit => {
            let before = wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "seeded void-storage fixture",
                |state| {
                    state.player_flags & PLAYER_FLAGS_VOID_UNLOCKED == 0
                        && state.void_items.is_empty()
                        && state.inventory_items
                            == vec![(options.fixture_item_guid, options.inventory_slot, 0)]
                },
            )
            .await?;
            send_encrypted_packet(
                stream,
                crypt,
                CMSG_UNLOCK_VOID_STORAGE,
                &vault_keeper_packed_guid(&options.vault_keeper, options.runtime_realm_id),
            )
            .await?;
            let after_unlock = wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "atomic void-storage unlock",
                |state| {
                    state.player_flags & PLAYER_FLAGS_VOID_UNLOCKED != 0
                        && state.money == before.money.saturating_sub(VOID_STORAGE_UNLOCK_COST)
                },
            )
            .await?;
            result.void_storage_unlock_persisted = true;

            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            if !contents.is_empty() {
                bail!("freshly unlocked void storage was not empty: {contents:?}");
            }

            let deposit_guid = item_guid_raw(options.fixture_item_guid, options.runtime_realm_id);
            let payload = build_void_storage_transfer_payload(
                &options.vault_keeper,
                options.runtime_realm_id,
                &[deposit_guid],
                &[],
            );
            send_encrypted_packet(stream, crypt, CMSG_VOID_STORAGE_TRANSFER, &payload).await?;
            let added = wait_for_void_storage_transfer(
                bot_index,
                stream,
                crypt,
                server_inflater,
                options.timeout_secs,
                true,
                None,
                result,
            )
            .await?
            .context("deposit transfer omitted added void item")?;
            if added.item_entry != options.item_entry || added.slot != 0 || added.item_id == 0 {
                bail!("unexpected deposited void item: {added:?}");
            }
            let expected_money = after_unlock
                .money
                .saturating_sub(VOID_STORAGE_STORE_ITEM_COST);
            wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "atomic void-storage deposit",
                |state| {
                    state.money == expected_money
                        && state.void_items == vec![(added.item_id, options.item_entry, 0)]
                        && state.inventory_items.is_empty()
                },
            )
            .await?;
            result.void_storage_item_id = Some(added.item_id);
            result.void_storage_deposit_persisted = true;
        }
        VoidStorageSmokePhase::VerifyDepositSwap => {
            let expected_id = options
                .expected_void_item_id
                .context("deposit-relog phase omitted void item ID")?;
            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            if contents
                != vec![VoidStorageItemWire {
                    item_id: expected_id,
                    slot: u32::from(options.expected_void_slot),
                    item_entry: options.item_entry,
                }]
            {
                bail!("deposit relog query mismatch: {contents:?}");
            }
            result.void_storage_deposit_relogin_verified = true;
            let payload = build_void_storage_swap_payload(
                &options.vault_keeper,
                options.runtime_realm_id,
                expected_id,
                5,
            );
            send_encrypted_packet(stream, crypt, CMSG_SWAP_VOID_ITEM, &payload).await?;
            wait_for_void_storage_swap(
                bot_index,
                stream,
                crypt,
                server_inflater,
                options.timeout_secs,
                expected_id,
                5,
                result,
            )
            .await?;
            wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "atomic void-storage slot swap",
                |state| state.void_items == vec![(expected_id, options.item_entry, 5)],
            )
            .await?;
            result.void_storage_swap_persisted = true;
        }
        VoidStorageSmokePhase::VerifySwapWithdraw => {
            let expected_id = options
                .expected_void_item_id
                .context("swap-relog phase omitted void item ID")?;
            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            if contents
                != vec![VoidStorageItemWire {
                    item_id: expected_id,
                    slot: u32::from(options.expected_void_slot),
                    item_entry: options.item_entry,
                }]
            {
                bail!("swap relog query mismatch: {contents:?}");
            }
            result.void_storage_swap_relogin_verified = true;
            let withdrawal_guid = item_guid_raw(expected_id, options.runtime_realm_id);
            let payload = build_void_storage_transfer_payload(
                &options.vault_keeper,
                options.runtime_realm_id,
                &[],
                &[withdrawal_guid],
            );
            send_encrypted_packet(stream, crypt, CMSG_VOID_STORAGE_TRANSFER, &payload).await?;
            wait_for_void_storage_transfer(
                bot_index,
                stream,
                crypt,
                server_inflater,
                options.timeout_secs,
                false,
                Some(expected_id),
                result,
            )
            .await?;
            wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "atomic void-storage withdrawal",
                |state| {
                    state.void_items.is_empty()
                        && state.inventory_items.len() == 1
                        && state.inventory_items[0].2 & 1 != 0
                },
            )
            .await?;
            result.void_storage_withdraw_persisted = true;
        }
        VoidStorageSmokePhase::VerifyWithdraw => {
            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            if !contents.is_empty() {
                bail!("withdraw relog query was not empty: {contents:?}");
            }
            wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "withdrawn item after relog",
                |state| {
                    state.void_items.is_empty()
                        && state.inventory_items.len() == 1
                        && state.inventory_items[0].2 & 1 != 0
                },
            )
            .await?;
            result.void_storage_withdraw_relogin_verified = true;
        }
        VoidStorageSmokePhase::QueryCapture => {
            let expected_id = options
                .expected_void_item_id
                .context("void-storage query capture omitted seeded item ID")?;
            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            let expected = vec![VoidStorageItemWire {
                item_id: expected_id,
                slot: u32::from(options.expected_void_slot),
                item_entry: options.item_entry,
            }];
            if contents != expected {
                bail!("void-storage query capture mismatch: {contents:?}, expected {expected:?}");
            }
            result.void_storage_item_id = Some(expected_id);
            result.void_storage_query_capture_passed = Some(true);
        }
    }
    loot_race::logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection.as_mut(),
        bot.character_guid,
        result,
    )
    .await?;
    if options.phase != VoidStorageSmokePhase::QueryCapture {
        result.void_storage_smoke_passed = Some(true);
    }
    Ok(())
}
pub(crate) fn load_void_storage_db_state(
    bot: &config::BotConfig,
    item_entry: u32,
) -> Result<VoidStorageDbState> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let (money, player_flags): (u64, u32) = conn
        .exec_first(
            "SELECT money, playerFlags FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load void-storage character state: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    let void_items = conn
        .exec_map(
            "SELECT itemId, itemEntry, slot FROM character_void_storage WHERE playerGuid = ? ORDER BY slot",
            (bot.character_guid,),
            |(item_id, entry, slot): (u64, u32, u8)| (item_id, entry, slot),
        )
        .map_err(|error| anyhow!("Load character_void_storage state: {error}"))?;
    let inventory_items = conn
        .exec_map(
            "SELECT ii.guid, ci.slot, ii.flags FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ? ORDER BY ii.guid",
            (bot.character_guid, item_entry),
            |(guid, slot, flags): (u64, u8, u32)| (guid, slot, flags),
        )
        .map_err(|error| anyhow!("Load void-storage fixture inventory state: {error}"))?;
    Ok(VoidStorageDbState {
        money,
        player_flags,
        void_items,
        inventory_items,
    })
}

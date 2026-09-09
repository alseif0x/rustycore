//! Runtime operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) async fn disconnect_rested_xp_and_wait(
    bot_index: usize,
    bot: &config::BotConfig,
    instance_stream: &mut TcpStream,
    instance_crypt: &mut WorldCrypt,
    instance_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    // A bare socket loss keeps a stock C++ WorldSession alive for 60 seconds
    // (`expireTime` in WorldSession). Exercise the real logout opcode instead:
    // wilderness logout completes after C++'s 20-second countdown, while Rust
    // may complete immediately. The DB-stability wait below remains the final
    // persistence proof for both runtimes.
    send_encrypted_packet(instance_stream, instance_crypt, CMSG_LOGOUT_REQUEST, &[0])
        .await
        .context("send rested-XP CMSG_LOGOUT_REQUEST")?;
    info!("[Bot {}] ✅ rested-XP CMSG_LOGOUT_REQUEST sent", bot_index);
    let logout_wait_secs = timeout_secs.min(NORMAL_LOGOUT_COMPLETE_WAIT_SECS);
    let logout_deadline = tokio::time::Instant::now() + Duration::from_secs(logout_wait_secs);
    let client_clock_origin = tokio::time::Instant::now();
    let mut logout_complete = false;
    let mut instance_open = true;
    let mut realm_open = realm_connection.is_some();
    enum RestedXpLogoutReady {
        Instance,
        Realm,
        InstanceClosed,
        RealmClosed,
    }
    while tokio::time::Instant::now() < logout_deadline {
        let remaining = logout_deadline.saturating_duration_since(tokio::time::Instant::now());
        if !instance_open && !realm_open {
            break;
        }
        let ready = match (instance_open, realm_open) {
            (true, true) => {
                let realm = realm_connection
                    .as_mut()
                    .context("rested-XP logout lost its realm connection")?;
                tokio::time::timeout(remaining, async {
                    let mut instance_peek = [0u8; 1];
                    let mut realm_peek = [0u8; 1];
                    tokio::select! {
                        ready = instance_stream.peek(&mut instance_peek) => {
                            if ready.context("rested-XP logout instance peek failed")? == 0 {
                                Ok(RestedXpLogoutReady::InstanceClosed)
                            } else {
                                Ok(RestedXpLogoutReady::Instance)
                            }
                        }
                        ready = realm.stream.peek(&mut realm_peek) => {
                            if ready.context("rested-XP logout realm peek failed")? == 0 {
                                Ok(RestedXpLogoutReady::RealmClosed)
                            } else {
                                Ok(RestedXpLogoutReady::Realm)
                            }
                        }
                    }
                })
                .await
            }
            (true, false) => {
                tokio::time::timeout(remaining, async {
                    let mut peek = [0u8; 1];
                    if instance_stream
                        .peek(&mut peek)
                        .await
                        .context("rested-XP logout instance peek failed")?
                        == 0
                    {
                        Ok(RestedXpLogoutReady::InstanceClosed)
                    } else {
                        Ok(RestedXpLogoutReady::Instance)
                    }
                })
                .await
            }
            (false, true) => {
                let realm = realm_connection
                    .as_mut()
                    .context("rested-XP logout lost its realm connection")?;
                tokio::time::timeout(remaining, async {
                    let mut peek = [0u8; 1];
                    if realm
                        .stream
                        .peek(&mut peek)
                        .await
                        .context("rested-XP logout realm peek failed")?
                        == 0
                    {
                        Ok(RestedXpLogoutReady::RealmClosed)
                    } else {
                        Ok(RestedXpLogoutReady::Realm)
                    }
                })
                .await
            }
            (false, false) => unreachable!(),
        };
        let ready = match ready {
            Ok(Ok(ready)) => ready,
            Ok(Err(error)) => return Err(error),
            Err(_) => break,
        };
        match ready {
            RestedXpLogoutReady::InstanceClosed => {
                instance_open = false;
                continue;
            }
            RestedXpLogoutReady::RealmClosed => {
                realm_open = false;
                continue;
            }
            RestedXpLogoutReady::Instance | RestedXpLogoutReady::Realm => {}
        }
        let (connection, opcode, payload) = match ready {
            RestedXpLogoutReady::Instance => {
                let (opcode, payload) = tokio::time::timeout(
                    logout_deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(instance_stream, instance_crypt, instance_inflater),
                )
                .await
                .map_err(|_| anyhow!("rested-XP logout instance packet read timed out"))??;
                ("instance", opcode, payload)
            }
            RestedXpLogoutReady::Realm => {
                let realm = realm_connection
                    .as_mut()
                    .context("rested-XP logout lost its realm connection")?;
                let (opcode, payload) = tokio::time::timeout(
                    logout_deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater),
                )
                .await
                .map_err(|_| anyhow!("rested-XP logout realm packet read timed out"))??;
                ("realm", opcode, payload)
            }
            RestedXpLogoutReady::InstanceClosed | RestedXpLogoutReady::RealmClosed => {
                unreachable!()
            }
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 {} rested-XP logout {}",
            bot_index,
            connection,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            if connection != "instance" {
                bail!("SMSG_TIME_SYNC_REQUEST arrived on realm during rested-XP logout");
            }
            let sequence_index = parse_time_sync_request_sequence(&payload)?;
            let client_time = client_clock_origin.elapsed().as_millis() as u32;
            let response = build_time_sync_response_payload(sequence_index, client_time);
            send_encrypted_packet(
                instance_stream,
                instance_crypt,
                CMSG_TIME_SYNC_RESPONSE,
                &response,
            )
            .await?;
            continue;
        }
        if opcode == SMSG_LOGOUT_COMPLETE {
            if connection != "realm" {
                warn!(
                    "[Bot {}] SMSG_LOGOUT_COMPLETE arrived on instance; stock C++ routes it on realm",
                    bot_index
                );
            }
            logout_complete = true;
            break;
        }
    }
    if !logout_complete {
        warn!(
            "[Bot {}] rested-XP graceful logout did not emit SMSG_LOGOUT_COMPLETE within {}s; closing both sockets and relying on the bounded DB-state proof",
            bot_index, logout_wait_secs
        );
    }

    instance_stream
        .shutdown()
        .await
        .context("shut down rested-XP instance socket")?;
    if let Some(mut realm) = realm_connection.take() {
        realm
            .stream
            .shutdown()
            .await
            .context("shut down rested-XP realm socket")?;
    }

    let bot_for_wait = bot.clone();
    tokio::task::spawn_blocking(move || {
        wait_for_rested_xp_character_offline_and_stable(&bot_for_wait, timeout_secs)
    })
    .await
    .map_err(|error| anyhow!("Rested-XP disconnect-save worker failed: {error}"))??;
    info!(
        "[Bot {}] ✅ rested-XP sockets disconnected and character save reached a stable offline row",
        bot_index
    );
    Ok(())
}
pub(crate) async fn run_bank_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    options: &BankSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let expected_before = match options.phase {
        BankSmokePhase::Deposit => options.inventory_slot,
        BankSmokePhase::Withdraw => options.bank_slot,
    };
    let bot_for_before = bot.clone();
    let item_guid = options.item_guid;
    let before = tokio::task::spawn_blocking(move || {
        verify_bank_fixture_location(&bot_for_before, item_guid, expected_before)
    })
    .await
    .map_err(|e| anyhow!("Bank pre-phase DB worker join failed: {e}"))??;
    if !before {
        bail!(
            "fixture item {} was not in expected slot {} before {:?}",
            options.item_guid,
            expected_before,
            options.phase
        );
    }
    if options.phase == BankSmokePhase::Withdraw {
        result.bank_relogin_after_deposit = true;
    }

    send_encrypted_packet(
        stream,
        crypt,
        CMSG_BANKER_ACTIVATE,
        &options.banker.packed_guid,
    )
    .await?;
    info!(
        "[Bot {}] ✅ CMSG_BANKER_ACTIVATE sent to entry={} spawn={}",
        bot_index, options.banker.entry, options.banker.spawn_guid
    );

    wait_for_bank_open(
        bot_index,
        stream,
        crypt,
        server_inflater,
        options.timeout_secs,
        result,
    )
    .await?;

    let (opcode, source_slot, expected_after) = match options.phase {
        BankSmokePhase::Deposit => (
            CMSG_AUTOBANK_ITEM,
            options.inventory_slot,
            options.bank_slot,
        ),
        BankSmokePhase::Withdraw => (
            CMSG_AUTOSTORE_BANK_ITEM,
            options.bank_slot,
            options.inventory_slot,
        ),
    };
    let payload = build_auto_bank_item_payload(source_slot);
    send_encrypted_packet(stream, crypt, opcode, &payload).await?;
    info!(
        "[Bot {}] ✅ {} sent from bag={} slot={}",
        bot_index,
        if options.phase == BankSmokePhase::Deposit {
            "CMSG_AUTOBANK_ITEM"
        } else {
            "CMSG_AUTOSTORE_BANK_ITEM"
        },
        INVENTORY_SLOT_BAG_0,
        source_slot
    );

    wait_for_bank_item_location(
        bot_index,
        bot,
        options.item_guid,
        expected_after,
        options.timeout_secs,
    )
    .await?;
    logout_and_wait(bot_index, stream, crypt, server_inflater, result).await?;

    let bot_for_after = bot.clone();
    let item_guid = options.item_guid;
    let persisted = tokio::task::spawn_blocking(move || {
        verify_bank_fixture_location(&bot_for_after, item_guid, expected_after)
    })
    .await
    .map_err(|e| anyhow!("Bank post-logout DB worker join failed: {e}"))??;
    if !persisted {
        bail!(
            "fixture item {} did not persist in slot {} after logout",
            options.item_guid,
            expected_after
        );
    }

    match options.phase {
        BankSmokePhase::Deposit => result.bank_deposit_persisted = true,
        BankSmokePhase::Withdraw => result.bank_withdraw_persisted = true,
    }
    result.bank_smoke_passed = Some(true);
    Ok(())
}
pub(crate) async fn run_vendor_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    vendor_entry: u32,
    vendor_spawn_guid: u64,
    runtime_counter: Option<u64>,
    item_entry: u32,
    extended_cost: u32,
    currency_id: u32,
    currency_cost: u32,
    currency_quantity: u32,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_vendor_smoke_fixture(
            &bot_for_setup,
            vendor_entry,
            vendor_spawn_guid,
            runtime_counter,
            item_entry,
            extended_cost,
            currency_id,
            currency_cost,
            currency_quantity,
            timeout_secs,
        )
    })
    .await
    .map_err(|error| anyhow!("Vendor smoke setup DB worker join failed: {error}"))??;

    let first = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        None,
        Some(fixture.options.clone()),
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
            let cleanup = tokio::task::spawn_blocking(move || {
                cleanup_vendor_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
            })
            .await
            .map_err(|join_error| {
                anyhow!(
                    "Vendor smoke purchase login/phase failed: {error}; cleanup worker failed: {join_error}"
                )
            })?;
            if let Err(cleanup_error) = cleanup {
                bail!(
                    "Vendor smoke purchase login/phase failed: {error}; fixture cleanup failed: {cleanup_error}"
                );
            }
            return Err(error.context("Vendor smoke purchase login/phase failed"));
        }
    };

    if combined.vendor_smoke_passed.unwrap_or(false) {
        let mut relog_options = fixture.options.clone();
        relog_options.phase = VendorSmokePhase::VerifyRelog;
        match run_bot(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            None,
            None,
            None,
            Some(relog_options),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        {
            Ok(second) => {
                combined.world_auth &= second.world_auth;
                combined.enum_characters &= second.enum_characters;
                combined.player_login_verified &= second.player_login_verified;
                combined.vendor_relogin_verified = second.vendor_relogin_verified;
                combined.vendor_currency_after = second.vendor_currency_after;
                combined.vendor_item_total_after = second.vendor_item_total_after;
                combined.seen_opcodes.extend(second.seen_opcodes);
                combined.vendor_failure = second.vendor_failure;
                combined.vendor_smoke_passed = Some(
                    combined.vendor_inventory_seen
                        && combined.vendor_buy_succeeded_seen
                        && combined.vendor_set_currency_seen
                        && combined.vendor_item_push_seen
                        && combined.vendor_relogin_verified
                        && second.vendor_smoke_passed.unwrap_or(false),
                );
            }
            Err(error) => {
                combined.vendor_failure =
                    Some(format!("Vendor persistence relog/phase failed: {error}"));
                combined.vendor_smoke_passed = Some(false);
            }
        }
    }

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_vendor_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|error| anyhow!("Vendor smoke cleanup DB worker join failed: {error}"))?;
    if let Err(error) = cleanup {
        let cleanup_failure = format!("Vendor fixture cleanup failed: {error}");
        combined.vendor_failure = Some(match combined.vendor_failure.take() {
            Some(previous) => format!("{previous}; {cleanup_failure}"),
            None => cleanup_failure,
        });
        combined.vendor_smoke_passed = Some(false);
    }

    Ok(combined)
}
pub(crate) async fn run_vendor_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &VendorSmokeOptions,
    login_discovered_target: Option<DiscoveredCreatureGuid>,
    result: &mut BotRunResult,
) -> Result<()> {
    let expected_currency_after = options
        .currency_before
        .checked_sub(options.currency_cost)
        .ok_or_else(|| anyhow!("Vendor currency fixture underflow"))?;

    if options.phase == VendorSmokePhase::VerifyRelog {
        let bot_for_db = bot.clone();
        let currency_id = options.currency_id;
        let item_entry = options.item_entry;
        let (currency, item_total) = tokio::task::spawn_blocking(move || {
            load_vendor_smoke_db_state(&bot_for_db, currency_id, item_entry)
        })
        .await
        .map_err(|error| anyhow!("Vendor relog DB worker join failed: {error}"))??;
        result.vendor_currency_after = Some(currency);
        result.vendor_item_total_after = Some(item_total);
        if currency != expected_currency_after || item_total != options.expected_item_total {
            bail!(
                "vendor state after relog is currency/item {currency}/{item_total}, expected {expected_currency_after}/{}",
                options.expected_item_total
            );
        }
        result.vendor_relogin_verified = true;
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
        result.vendor_smoke_passed = Some(true);
        return Ok(());
    }

    let bot_for_before = bot.clone();
    let currency_id = options.currency_id;
    let item_entry = options.item_entry;
    let (currency_before, item_before) = tokio::task::spawn_blocking(move || {
        load_vendor_smoke_db_state(&bot_for_before, currency_id, item_entry)
    })
    .await
    .map_err(|error| anyhow!("Vendor pre-purchase DB worker join failed: {error}"))??;
    if currency_before != options.currency_before || item_before != 0 {
        bail!(
            "vendor fixture drifted before purchase: currency/item {currency_before}/{item_before}, expected {}/0",
            options.currency_before
        );
    }

    // C++ Player::CanNeverSee keeps nearby world objects hidden until the
    // client acknowledges that its active mover is initialized. Rust may have
    // queued the vendor CREATE earlier, so send the canonical ACK before the
    // cross-server discovery window in both cases.
    let active_mover_complete = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
        &active_mover_complete,
    )
    .await?;
    info!(
        "[Bot {}] ✅ CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE sent before vendor discovery",
        bot_index
    );

    let expected_runtime_counter = (options.vendor.guid_counter != 0)
        .then_some(options.vendor.guid_counter & OBJECT_GUID_COUNTER_MASK);
    let mut discovered = login_discovered_target;
    let discovery_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while discovered.is_none() && tokio::time::Instant::now() < discovery_deadline {
        let remaining = discovery_deadline.saturating_duration_since(tokio::time::Instant::now());
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            Duration::from_millis(250).min(remaining),
            Duration::from_secs(5),
            "vendor instance login discovery",
        )
        .await?
        else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
        } else if opcode == SMSG_UPDATE_OBJECT {
            discovered = find_creature_guid_near_position_in_update_object(
                &payload,
                options.vendor.map_id,
                options.vendor.entry,
                options.vendor.x as f32,
                options.vendor.y as f32,
                options.vendor.z as f32,
                options.target_match_radius,
                expected_runtime_counter,
            );
        }
    }
    let runtime_target = resolve_vendor_runtime_target(&options.vendor, discovered)?;
    let runtime_counter = runtime_target.low & OBJECT_GUID_COUNTER_MASK;
    let runtime_vendor_guid = build_packed_guid(runtime_target.low, runtime_target.high);
    result.vendor_runtime_counter = Some(runtime_counter);

    send_encrypted_packet(stream, crypt, CMSG_LIST_INVENTORY, &runtime_vendor_guid).await?;
    info!(
        "[Bot {}] ✅ CMSG_LIST_INVENTORY sent to entry={} spawn={} counter={}",
        bot_index, options.vendor.entry, options.vendor.spawn_guid, runtime_counter
    );
    let vendor_item = wait_for_vendor_inventory_item(
        bot_index,
        stream,
        crypt,
        server_inflater,
        &runtime_vendor_guid,
        options,
        result,
    )
    .await?;

    let buy_payload = build_vendor_buy_item_payload(
        &runtime_vendor_guid,
        bot.character_guid,
        vendor_item.muid,
        options.item_entry,
    );
    send_encrypted_packet(stream, crypt, CMSG_BUY_ITEM, &buy_payload).await?;
    info!(
        "[Bot {}] ✅ CMSG_BUY_ITEM sent item={} muid={} cost={}/currency={}",
        bot_index, options.item_entry, vendor_item.muid, options.currency_cost, options.currency_id
    );
    wait_for_vendor_purchase_result(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        &runtime_vendor_guid,
        vendor_item.muid,
        options,
        expected_currency_after,
        result,
    )
    .await?;

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
    let bot_for_after = bot.clone();
    let currency_id = options.currency_id;
    let item_entry = options.item_entry;
    let (currency_after, item_after) = tokio::task::spawn_blocking(move || {
        load_vendor_smoke_db_state(&bot_for_after, currency_id, item_entry)
    })
    .await
    .map_err(|error| anyhow!("Vendor post-purchase DB worker join failed: {error}"))??;
    result.vendor_currency_after = Some(currency_after);
    result.vendor_item_total_after = Some(item_after);
    if currency_after != expected_currency_after || item_after != options.expected_item_total {
        bail!(
            "vendor purchase persisted currency/item {currency_after}/{item_after}, expected {expected_currency_after}/{}",
            options.expected_item_total
        );
    }
    result.vendor_smoke_passed = Some(true);
    Ok(())
}
/// Expand 32-byte K (from SRP6) to the 64-byte session_key_bnet expected by the
/// worldserver: K || SHA256(K). Mirrors main_srp6_complete.rs::expand_session_key.
pub(crate) fn expand_session_key(k: &[u8]) -> [u8; 64] {
    use sha2::{Digest, Sha256};
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(k);
    out[32..].copy_from_slice(&Sha256::digest(k));
    out
}
pub(crate) fn parse_connect_to(payload: &[u8]) -> Option<ConnectToTarget> {
    // RustyCore ConnectTo payload:
    // signature[256], address_type u8, address (4/16), port u16, serial u32,
    // connection_type u8, key i64.
    let address_type = *payload.get(256)?;
    let address_offset = 257;
    let (address, address_len) = match address_type {
        1 => {
            let bytes: [u8; 4] = payload
                .get(address_offset..address_offset + 4)?
                .try_into()
                .ok()?;
            (IpAddr::from(bytes), 4)
        }
        2 => {
            let bytes: [u8; 16] = payload
                .get(address_offset..address_offset + 16)?
                .try_into()
                .ok()?;
            (IpAddr::from(bytes), 16)
        }
        _ => return None,
    };
    let serial_offset = 256 + 1 + address_len + 2;
    let port_offset = 256 + 1 + address_len;
    let port = u16::from_le_bytes(payload.get(port_offset..port_offset + 2)?.try_into().ok()?);
    let serial = u32::from_le_bytes(
        payload
            .get(serial_offset..serial_offset + 4)?
            .try_into()
            .ok()?,
    );
    let connection_type_offset = serial_offset + 4;
    let connection_type = *payload.get(connection_type_offset)?;
    let key_offset = connection_type_offset + 1;
    let key = i64::from_le_bytes(payload.get(key_offset..key_offset + 8)?.try_into().ok()?);

    Some(ConnectToTarget {
        address,
        port,
        serial,
        connection_type,
        key,
    })
}
pub(crate) async fn connect_to_instance(
    bot_index: usize,
    connect_to: &ConnectToTarget,
    session_key: &[u8],
) -> Result<(TcpStream, WorldCrypt)> {
    if connect_to.connection_type != 1 {
        bail!(
            "SMSG_CONNECT_TO requested unsupported connection type {}",
            connect_to.connection_type
        );
    }

    let expected_port = std::env::var("INSTANCE_PORT").ok();
    validate_pinned_instance_port(connect_to.port, expected_port.as_deref())?;

    let target_host =
        std::env::var("INSTANCE_HOST").unwrap_or_else(|_| connect_to.address.to_string());
    let addr = format!("{}:{}", target_host, connect_to.port);
    info!(
        "[Bot {}] Connecting to instance socket {}...",
        bot_index, addr
    );
    let mut stream = tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, TcpStream::connect(&addr))
        .await
        .map_err(|_| anyhow!("Timed out connecting to instance socket {addr}"))?
        .map_err(|e| anyhow!("Failed to connect to instance socket {}: {}", addr, e))?;

    let mut init_buf = vec![0u8; 256];
    let n = tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, stream.read(&mut init_buf))
        .await
        .map_err(|_| anyhow!("Timed out reading instance SERVER_INIT"))??;
    if !init_buf[..n].starts_with(&SERVER_INIT[..SERVER_INIT.len().min(n)]) {
        bail!(
            "Unexpected instance server init: {:?}",
            String::from_utf8_lossy(&init_buf[..n])
        );
    }

    tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, async {
        stream.write_all(CLIENT_INIT).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| anyhow!("Timed out writing instance CLIENT_INIT"))??;

    let (opcode, challenge_data) = tokio::time::timeout(
        INITIAL_NETWORK_IO_TIMEOUT,
        read_unencrypted_packet(&mut stream),
    )
    .await
    .map_err(|_| anyhow!("Timed out reading instance SMSG_AUTH_CHALLENGE"))??;
    if opcode != 0x3048 {
        bail!(
            "Expected instance SMSG_AUTH_CHALLENGE (0x3048), got 0x{:04X}",
            opcode
        );
    }
    if challenge_data.len() < 48 {
        bail!(
            "Instance SMSG_AUTH_CHALLENGE too short: {} bytes",
            challenge_data.len()
        );
    }

    let server_challenge: [u8; 16] = challenge_data[32..48].try_into()?;
    let local_challenge: [u8; 16] = rand::random();
    let digest = compute_continued_auth_digest(
        connect_to.key,
        &local_challenge,
        &server_challenge,
        session_key,
    );
    let auth_data = build_cmsg_auth_continued_session(connect_to.key, &local_challenge, &digest);
    send_unencrypted_packet(&mut stream, 0x3766, &auth_data).await?;

    let mut got_encryption = false;
    for _ in 0..10 {
        let (op, _payload) =
            tokio::time::timeout(Duration::from_secs(5), read_unencrypted_packet(&mut stream))
                .await
                .map_err(|_| anyhow!("Timeout waiting for instance encrypted mode"))??;

        if op == 0x3049 {
            got_encryption = true;
            break;
        }

        info!(
            "[Bot {}] Instance pre-encryption packet 0x{:04X}",
            bot_index, op
        );
    }

    if !got_encryption {
        bail!("Instance socket did not send SMSG_ENTER_ENCRYPTED_MODE");
    }

    let enc_key = derive_instance_encryption_key(session_key, &local_challenge, &server_challenge);
    send_unencrypted_packet(&mut stream, 0x3767, &[]).await?;

    Ok((stream, WorldCrypt::new_with_counters(&enc_key, 2, 2)))
}

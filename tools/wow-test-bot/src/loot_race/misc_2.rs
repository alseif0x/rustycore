//! Loot-race misc operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

pub(crate) fn find_loot_target_guid_in_update_object(
    payload: &[u8],
    target: &LootRaceTarget,
) -> Result<Option<(u64, u64)>> {
    let expected_high_type = match target.kind {
        LootRaceTargetKind::Creature => HIGH_GUID_CREATURE,
        LootRaceTargetKind::GameObject => HIGH_GUID_GAMEOBJECT,
    };
    let mut candidates = std::collections::BTreeSet::new();
    for offset in 0..payload.len().saturating_sub(2) {
        if !matches!(payload[offset], 1 | 2) {
            continue;
        }
        let Some((_, low, high)) = parse_packed_guid(&payload[offset + 1..]) else {
            continue;
        };
        if ((high >> 58) & GUID_HIGH_TYPE_MASK) != expected_high_type
            || ((high >> 29) & GUID_MAP_MASK) != u64::from(target.map_id)
            || ((high >> 6) & GUID_ENTRY_MASK) != u64::from(target.entry)
        {
            continue;
        }
        candidates.insert((low, high));
    }

    if candidates.len() > 1 {
        bail!(
            "loot-race update contained {} distinct live ObjectGuid candidates for {:?} entry {} map {}: {candidates:?}",
            candidates.len(),
            target.kind,
            target.entry,
            target.map_id
        );
    }
    Ok(candidates.into_iter().next())
}
pub(crate) async fn run_phase(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &LootRaceOptions,
    target_seen: bool,
    result: &mut BotRunResult,
) -> Result<()> {
    let run = run_phase_inner(
        bot_index,
        stream,
        crypt,
        inflater,
        realm_connection,
        options,
        target_seen,
        result,
    )
    .await;
    if let Err(error) = &run {
        options.sync.cancel(format!(
            "participant {} phase {:?} failed: {error:#}",
            options.participant, options.phase
        ));
    }
    run
}
pub(crate) async fn run_phase_inner(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &LootRaceOptions,
    mut target_seen: bool,
    result: &mut BotRunResult,
) -> Result<()> {
    if options.phase == LootRacePhase::VerifyRelog {
        result.loot_race_target_runtime_counter = Some(options.resolved_runtime_counter()?);
        logout_and_wait_routed_like_cpp(
            bot_index,
            stream,
            crypt,
            inflater,
            realm_connection.as_mut(),
            options.character_guid,
            result,
        )
        .await?;
        result.loot_race_relog_verified = true;
        result.loot_race_smoke_passed = Some(true);
        return Ok(());
    }
    let realm = realm_connection
        .as_mut()
        .ok_or_else(|| anyhow!("loot-race requires separate realm and instance sockets"))?;

    // C++ can defer nearby-object visibility until the client confirms that its
    // active mover is initialized. Reuse the same real movement handshake as
    // the rested-XP combat smoke before proving the exact creature is live.
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
        &build_move_init_active_mover_complete_payload(0),
    )
    .await?;
    target_seen |=
        drain_until_target_or_quiet(bot_index, stream, crypt, inflater, realm, options, result)
            .await?;
    if !target_seen {
        let override_detail = if options.target.runtime_counter_override == 0 {
            "auto-discovery".to_string()
        } else {
            format!("override {}", options.target.runtime_counter_override)
        };
        bail!(
            "world-loot target entry {} spawn {} ({override_detail}) was not present in SMSG_UPDATE_OBJECT; require a fresh world/runtime and matching override",
            options.target.entry,
            options.target.spawn_guid
        );
    }
    result.loot_race_target_runtime_counter = Some(options.resolved_runtime_counter()?);
    result.loot_race_target_discovered = true;
    if options.phase == LootRacePhase::CaptureItem {
        return run_single_item_capture_phase(
            bot_index, stream, crypt, inflater, realm, options, result,
        )
        .await;
    }
    options.sync.cancellation_error()?;
    wait_phase(options, &options.sync.logged_in, "both bots logged in").await?;

    form_party(bot_index, stream, crypt, inflater, realm, options, result).await?;
    result.loot_race_party_confirmed = true;
    wait_phase(options, &options.sync.party_ready, "PERSONAL party formed").await?;

    let (player_low, player_high) = create_player_guid_raw(options.character_guid, realm_id());
    let player_x = options.target.x + 1.0 + options.participant as f64;
    let player_y = options.target.y;
    let player_z = options.target.z;
    let player_orientation =
        (options.target.y - player_y).atan2(options.target.x - player_x) as f32;
    let movement = build_move_heartbeat_payload(
        player_low,
        player_high,
        player_x as f32,
        player_y as f32,
        player_z as f32,
        player_orientation,
    );
    send_encrypted_packet(stream, crypt, CMSG_MOVE_HEARTBEAT, &movement).await?;
    wait_phase(
        options,
        &options.sync.positioned,
        "both bots positioned at the shared chest",
    )
    .await?;

    if options.target.kind != LootRaceTargetKind::GameObject {
        bail!("two-client Race must use the guarded shared GameObject fixture");
    }
    wait_phase(
        options,
        &options.sync.use_ready,
        "simultaneous CMSG_GAME_OBJ_USE",
    )
    .await?;
    // C++ `WorldPackets::GameObject::GameObjUse::Read` consumes one packed
    // GameObject ObjectGuid; both clients deliberately send the same wire GUID.
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_GAME_OBJ_USE,
        &options.resolved_packed_guid()?,
    )
    .await?;
    info!(
        "[Bot {}] loot-race phase: CMSG_GAME_OBJ_USE sent for shared spawn {}",
        bot_index, options.target.spawn_guid
    );
    let window =
        wait_for_loot_window(bot_index, stream, crypt, inflater, realm, options, result).await?;
    result.loot_race_loot_opened = true;
    result.loot_race_loot_list_id = Some(window.loot_list_id);
    result.loot_race_loot_coins = Some(window.coins);
    info!(
        "[Bot {}] loot-race phase: SMSG_LOOT_RESPONSE received (loot_list_id={}, coins={})",
        bot_index, window.loot_list_id, window.coins
    );
    options.sync.windows.lock().await[options.participant] = Some(window.clone());
    wait_phase(
        options,
        &options.sync.response_received,
        "both SMSG_LOOT_RESPONSE packets received",
    )
    .await?;
    wait_phase(
        options,
        &options.sync.windows_ready,
        "shared loot windows recorded",
    )
    .await?;
    validate_shared_windows(options).await?;

    wait_phase(options, &options.sync.item_claim, "simultaneous item claim").await?;
    let item_claim = build_loot_item_claim(&window);
    send_encrypted_packet(stream, crypt, CMSG_LOOT_ITEM, &item_claim).await?;
    let expected_loot_owner = options.resolved_runtime_guid()?;
    let mut evidence = collect_evidence(
        bot_index,
        stream,
        crypt,
        inflater,
        realm,
        expected_loot_owner,
        RESPONSE_SETTLE,
        options,
        result,
    )
    .await?;
    options.sync.evidence.lock().await[options.participant] = evidence.clone();
    wait_phase(
        options,
        &options.sync.item_observed,
        "item outcomes observed",
    )
    .await?;
    validate_item_outcome(options, result).await?;

    wait_phase(
        options,
        &options.sync.money_claim,
        "simultaneous money claim",
    )
    .await?;
    send_encrypted_packet(stream, crypt, CMSG_LOOT_MONEY, &[0]).await?;
    evidence.merge(
        collect_evidence(
            bot_index,
            stream,
            crypt,
            inflater,
            realm,
            expected_loot_owner,
            RESPONSE_SETTLE,
            options,
            result,
        )
        .await?,
    );
    options.sync.evidence.lock().await[options.participant] = evidence;
    wait_phase(
        options,
        &options.sync.money_observed,
        "money outcomes observed",
    )
    .await?;
    // The money settle window can also surface a delayed item response. Merge
    // first, then re-run the exact item fanout proof so a late duplicate,
    // foreign push/failure, or extra removal cannot escape the earlier fence.
    validate_item_outcome(options, result).await?;
    validate_money_outcome(options, result).await?;

    wait_phase(options, &options.sync.before_leave, "party cleanup").await?;
    send_encrypted_packet(&mut realm.stream, &mut realm.crypt, CMSG_LEAVE_GROUP, &[0]).await?;
    logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        inflater,
        Some(realm),
        options.character_guid,
        result,
    )
    .await?;
    result.loot_race_smoke_passed = Some(true);
    Ok(())
}
pub(crate) async fn run_single_item_capture_phase(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if options.participant != 0 || options.character_guid != options.killer_character_guid {
        bail!("loot-item capture requires account A as its sole connected killer");
    }

    let (player_low, player_high) = create_player_guid_raw(options.character_guid, realm_id());
    let player_x = options.target.x + 1.0;
    let player_y = options.target.y;
    let player_z = options.target.z;
    let player_orientation =
        (options.target.y - player_y).atan2(options.target.x - player_x) as f32;
    let movement = build_move_heartbeat_payload(
        player_low,
        player_high,
        player_x as f32,
        player_y as f32,
        player_z as f32,
        player_orientation,
    );
    send_encrypted_packet(stream, crypt, CMSG_MOVE_HEARTBEAT, &movement).await?;

    kill_target_once(bot_index, stream, crypt, inflater, realm, options, result).await?;
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_LOOT_UNIT,
        &options.resolved_packed_guid()?,
    )
    .await?;
    let window =
        wait_for_loot_window(bot_index, stream, crypt, inflater, realm, options, result).await?;
    result.loot_race_loot_opened = true;
    result.loot_race_loot_list_id = Some(window.loot_list_id);
    result.loot_race_loot_coins = Some(window.coins);

    // Capture-diff starts at this exact CMSG. Opening the corpse (including its
    // random coin roll) is deliberately outside the item-only window.
    let item_claim = build_loot_item_claim(&window);
    send_encrypted_packet(stream, crypt, CMSG_LOOT_ITEM, &item_claim).await?;
    let evidence = collect_single_item_capture_evidence(
        bot_index,
        stream,
        crypt,
        inflater,
        realm,
        options.resolved_runtime_guid()?,
        options.timeout_secs,
        result,
    )
    .await?;
    let expected_player = create_player_guid_raw(options.character_guid, realm_id());
    validate_single_item_capture_evidence(
        &evidence,
        expected_player,
        options.target.item_entry,
        &window,
    )?;
    result.loot_race_item_push_seen = true;
    result.loot_race_loot_removed_seen = true;

    send_and_verify_loot_item_capture_fence(
        bot_index,
        stream,
        crypt,
        inflater,
        options.timeout_secs,
        result,
    )
    .await?;
    logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        inflater,
        Some(realm),
        options.character_guid,
        result,
    )
    .await?;
    result.loot_race_smoke_passed = Some(true);
    Ok(())
}
/// Read only until the two C++-anchored item-claim responses have arrived.
///
/// The two-client race deliberately keeps a settle window so it can prove the
/// absence/presence of competing outcomes on both sockets.  A capture golden
/// has a different requirement: put the fixed ping fence immediately after
/// the one `LootRemoved` and one `ItemPushResult`, rather than admitting an
/// arbitrary two seconds of periodic traffic into the strict diff window.
pub(crate) async fn collect_single_item_capture_evidence(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    expected_loot_owner: (u64, u64),
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<WireEvidence> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut evidence = WireEvidence::default();

    loop {
        if evidence.item_pushes.len() == 1 && evidence.loot_removed.len() == 1 {
            return Ok(evidence);
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for the capture item responses (item pushes={}, loot removals={})",
                evidence.item_pushes.len(),
                evidence.loot_removed.len()
            );
        }

        // Do not `select!` two in-progress encrypted reads: cancelling the
        // losing `read_exact` could consume part of a framed packet.  Poll
        // socket readiness briefly, then finish any selected frame without
        // cancellation.
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(5)),
            remaining,
            "loot-item capture realm response",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            record_evidence(opcode, &payload, expected_loot_owner, &mut evidence)?;
            validate_single_item_capture_candidate(&evidence)?;
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            continue;
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "loot-item capture instance response",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            record_evidence(opcode, &payload, expected_loot_owner, &mut evidence)?;
            validate_single_item_capture_candidate(&evidence)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
}
pub(crate) fn validate_single_item_capture_candidate(evidence: &WireEvidence) -> Result<()> {
    if evidence.item_pushes.len() > 1 || evidence.loot_removed.len() > 1 {
        bail!(
            "loot-item capture observed duplicate functional responses before its fence (item pushes={}, loot removals={})",
            evidence.item_pushes.len(),
            evidence.loot_removed.len()
        );
    }
    if !evidence.inventory_failures.is_empty() {
        bail!("loot-item capture observed an inventory failure before its success fence");
    }
    if !evidence.money_notifies.is_empty() || !evidence.coin_removed.is_empty() {
        bail!("loot-item capture observed money-claim traffic before its item-only fence");
    }
    Ok(())
}
pub(crate) fn validate_single_item_capture_evidence(
    evidence: &WireEvidence,
    expected_player: (u64, u64),
    expected_item_entry: u32,
    window: &LootWindow,
) -> Result<()> {
    if evidence.item_pushes.len() != 1 {
        bail!(
            "single-session item capture observed item pushes {:?}; expected exactly one",
            evidence.item_pushes
        );
    }
    let push = &evidence.item_pushes[0];
    if push.item_entry != expected_item_entry
        || push.quantity != 1
        || push.quantity_in_inventory != 1
        || push.slot != INVENTORY_SLOT_BAG_0
        || push.slot_in_bag != i32::from(LOOT_ITEM_CAPTURE_KEYRING_SLOT)
        || (push.player_low, push.player_high) != expected_player
    {
        bail!(
            "single-session item capture observed item pushes {:?}; expected exactly item {} quantity 1 in keyring slot {}/{} for the sole character",
            evidence.item_pushes,
            expected_item_entry,
            INVENTORY_SLOT_BAG_0,
            LOOT_ITEM_CAPTURE_KEYRING_SLOT
        );
    }
    let expected_removal = LootRemovedEvidence {
        owner_low: window.owner_low,
        owner_high: window.owner_high,
        loot_low: window.loot_low,
        loot_high: window.loot_high,
        loot_list_id: window.loot_list_id,
    };
    if evidence.loot_removed != [expected_removal] {
        bail!(
            "single-session item capture observed removals {:?}; expected exactly {:?}",
            evidence.loot_removed,
            expected_removal
        );
    }
    if !evidence.inventory_failures.is_empty() {
        bail!("single-session item capture unexpectedly failed inventory storage");
    }
    if !evidence.money_notifies.is_empty() || !evidence.coin_removed.is_empty() {
        bail!("item-only capture unexpectedly emitted money-claim packets");
    }
    Ok(())
}
pub(crate) async fn send_and_verify_loot_item_capture_fence(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    let payload = build_ping_payload(LOOT_ITEM_CAPTURE_FENCE_SERIAL);
    send_encrypted_packet(stream, crypt, CMSG_PING, &payload).await?;
    info!(
        "[Bot {}] deterministic loot-item CMSG_PING fence sent (serial=0x{:08X})",
        bot_index, LOOT_ITEM_CAPTURE_FENCE_SERIAL
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for loot-item capture-fence SMSG_PONG");
        }
        let (opcode, payload) =
            tokio::time::timeout(remaining, read_encrypted_packet(stream, crypt, inflater))
                .await
                .map_err(|_| {
                    anyhow!("timed out waiting for loot-item capture-fence SMSG_PONG")
                })??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_PONG {
            if payload != LOOT_ITEM_CAPTURE_FENCE_SERIAL.to_le_bytes() {
                bail!(
                    "loot-item capture-fence SMSG_PONG mismatch: expected 0x{:08X}, got {:02X?}",
                    LOOT_ITEM_CAPTURE_FENCE_SERIAL,
                    payload
                );
            }
            return Ok(());
        }
        handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
    }
}
pub(crate) fn logout_completion_route(
    opcode: u16,
    payload: &[u8],
    route: LogoutCompletionRoute,
) -> Result<Option<LogoutCompletionRoute>> {
    if opcode != SMSG_LOGOUT_COMPLETE {
        return Ok(None);
    }
    if !payload.is_empty() {
        bail!(
            "SMSG_LOGOUT_COMPLETE carried {} bytes; C++ 3.4.3 writes an empty body",
            payload.len()
        );
    }
    Ok(Some(route))
}
pub(crate) fn wait_for_loot_character_offline(
    character_guid: u64,
    timeout: Duration,
) -> Result<()> {
    let url = characters_db_url()?;
    let opts = loot_db_opts(&url, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let online: u8 = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (character_guid,),
            )
            .map_err(|error| anyhow!("Check loot logout offline state: {error}"))?
            .ok_or_else(|| anyhow!("Loot character {character_guid} disappeared during logout"))?;
        if online == 0 {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "loot character {character_guid} remained online after its bounded logout DB proof"
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
/// Finish a loot workflow across the real 3.4.3 socket topology.
///
/// C++ routes `SMSG_LOGOUT_RESPONSE` on instance and
/// `SMSG_LOGOUT_COMPLETE` on realm. Rust currently may complete on instance,
/// so both sockets are drained safely and the observed route is reported. In
/// every case, success additionally requires the exact character row to be
/// offline; a closed socket by itself is never accepted as logout proof.
pub(crate) async fn logout_and_wait_routed_like_cpp(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    mut realm_connection: Option<&mut EncryptedWorldConnection>,
    character_guid: u64,
    result: &mut BotRunResult,
) -> Result<bool> {
    send_encrypted_packet(stream, crypt, CMSG_LOGOUT_REQUEST, &[0]).await?;
    info!("[Bot {}] ✅ CMSG_LOGOUT_REQUEST sent", bot_index);

    let deadline =
        tokio::time::Instant::now() + Duration::from_secs(NORMAL_LOGOUT_COMPLETE_WAIT_SECS);
    let mut instance_open = true;
    let mut realm_open = realm_connection.is_some();
    let mut completion_route = None;
    let mut transport_errors = Vec::new();

    while tokio::time::Instant::now() < deadline && completion_route.is_none() {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        if !realm_open && !instance_open {
            tokio::time::sleep(remaining).await;
            break;
        }

        if realm_open {
            let realm = realm_connection
                .as_deref_mut()
                .expect("realm-open state requires the preserved realm connection");
            match read_encrypted_packet_if_ready(
                &mut realm.stream,
                &mut realm.crypt,
                &mut realm.inflater,
                remaining.min(Duration::from_millis(50)),
                remaining,
                "loot logout realm packet",
            )
            .await
            {
                Ok(Some((opcode, payload))) => {
                    result.seen_opcodes.push(format!("0x{opcode:04X}"));
                    info!(
                        "[Bot {}] 📦 realm loot logout {}",
                        bot_index,
                        parse_packet(opcode, &payload)
                    );
                    completion_route =
                        logout_completion_route(opcode, &payload, LogoutCompletionRoute::Realm)?;
                }
                Ok(None) => {}
                Err(error) => {
                    realm_open = false;
                    transport_errors.push(format!("realm: {error}"));
                }
            }
        }

        if completion_route.is_some() {
            break;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        if instance_open {
            let readiness_wait = if realm_open {
                Duration::from_millis(1)
            } else {
                remaining.min(Duration::from_millis(50))
            };
            match read_encrypted_packet_if_ready(
                stream,
                crypt,
                inflater,
                readiness_wait,
                remaining,
                "loot logout instance packet",
            )
            .await
            {
                Ok(Some((opcode, payload))) => {
                    result.seen_opcodes.push(format!("0x{opcode:04X}"));
                    info!(
                        "[Bot {}] 📦 instance loot logout {}",
                        bot_index,
                        parse_packet(opcode, &payload)
                    );
                    completion_route =
                        logout_completion_route(opcode, &payload, LogoutCompletionRoute::Instance)?;
                    if completion_route.is_none() {
                        handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload)
                            .await?;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    instance_open = false;
                    transport_errors.push(format!("instance: {error}"));
                }
            }
        }
    }

    let offline = tokio::task::spawn_blocking(move || {
        wait_for_loot_character_offline(
            character_guid,
            Duration::from_secs(LOOT_LOGOUT_DB_CONFIRM_WAIT_SECS),
        )
    })
    .await
    .map_err(|error| anyhow!("loot logout DB worker join failed: {error}"))?;
    if let Err(error) = offline {
        let route = completion_route
            .map(|route| format!("{route:?}"))
            .unwrap_or_else(|| "none".to_string());
        let transport = if transport_errors.is_empty() {
            "none".to_string()
        } else {
            transport_errors.join("; ")
        };
        bail!(
            "loot logout was not proven for character {character_guid} (LogoutComplete route={route}, transport errors={transport}): {error}"
        );
    }

    match completion_route {
        Some(LogoutCompletionRoute::Realm) => info!(
            "[Bot {}] ✅ realm SMSG_LOGOUT_COMPLETE and exact offline DB state confirmed",
            bot_index
        ),
        Some(LogoutCompletionRoute::Instance) => warn!(
            "[Bot {}] SMSG_LOGOUT_COMPLETE arrived on instance instead of the C++ realm route; exact offline DB state confirmed",
            bot_index
        ),
        None => warn!(
            "[Bot {}] no SMSG_LOGOUT_COMPLETE arrived within {}s; exact offline DB state confirmed as the bounded fallback{}",
            bot_index,
            NORMAL_LOGOUT_COMPLETE_WAIT_SECS,
            if transport_errors.is_empty() {
                String::new()
            } else {
                format!(" ({})", transport_errors.join("; "))
            }
        ),
    }
    Ok(completion_route.is_some())
}
pub(crate) async fn best_effort_close(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    character_guid: u64,
    result: &mut BotRunResult,
) {
    if let Some(realm) = realm_connection.as_mut() {
        let _ = send_encrypted_packet(&mut realm.stream, &mut realm.crypt, CMSG_LEAVE_GROUP, &[0])
            .await;
    }
    let _ = logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        inflater,
        realm_connection.as_mut(),
        character_guid,
        result,
    )
    .await;
}
pub(crate) async fn best_effort_logout_preserving_group(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    character_guid: u64,
    result: &mut BotRunResult,
) {
    let _ = logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        inflater,
        realm_connection.as_mut(),
        character_guid,
        result,
    )
    .await;
}
impl WireEvidence {
    pub(crate) fn merge(&mut self, other: Self) {
        self.item_pushes.extend(other.item_pushes);
        self.loot_removed.extend(other.loot_removed);
        self.money_notifies.extend(other.money_notifies);
        self.coin_removed.extend(other.coin_removed);
        self.inventory_failures.extend(other.inventory_failures);
    }
}
pub(crate) async fn wait_phase(
    options: &LootRaceOptions,
    barrier: &Barrier,
    label: &str,
) -> Result<()> {
    options.sync.cancellation_error()?;
    info!(
        "[Bot {}] loot-race phase: waiting for {label}",
        options.participant + 1
    );
    tokio::time::timeout(Duration::from_secs(options.timeout_secs), async {
        tokio::select! {
            _ = barrier.wait() => Ok(()),
            _ = options.sync.cancelled() => options.sync.cancellation_error(),
        }
    })
    .await
    .map_err(|_| anyhow!("loot-race timed out waiting for {label}"))??;
    info!(
        "[Bot {}] loot-race phase: {label} complete",
        options.participant + 1
    );
    Ok(())
}
pub(crate) struct PartyUpdateCursor<'a> {
    pub(crate) payload: &'a [u8],
    pub(crate) offset: usize,
}
impl<'a> PartyUpdateCursor<'a> {
    pub(crate) fn new(payload: &'a [u8]) -> Self {
        Self { payload, offset: 0 }
    }

    pub(crate) fn take(&mut self, len: usize, field: &str) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| anyhow!("PartyUpdate {field} offset overflow"))?;
        let bytes = self.payload.get(self.offset..end).ok_or_else(|| {
            anyhow!(
                "malformed PartyUpdate: {field} needs {len} byte(s) at offset {}, payload has {}",
                self.offset,
                self.payload.len()
            )
        })?;
        self.offset = end;
        Ok(bytes)
    }

    pub(crate) fn read_u8(&mut self, field: &str) -> Result<u8> {
        Ok(self.take(1, field)?[0])
    }

    pub(crate) fn read_u16(&mut self, field: &str) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.take(2, field)?.try_into().expect("exact u16 slice"),
        ))
    }

    pub(crate) fn read_u32(&mut self, field: &str) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4, field)?.try_into().expect("exact u32 slice"),
        ))
    }

    pub(crate) fn read_i32(&mut self, field: &str) -> Result<i32> {
        Ok(i32::from_le_bytes(
            self.take(4, field)?.try_into().expect("exact i32 slice"),
        ))
    }

    pub(crate) fn read_packed_guid(&mut self, field: &str) -> Result<(u64, u64)> {
        let (consumed, low, high) = parse_packed_guid(&self.payload[self.offset..])
            .ok_or_else(|| anyhow!("malformed PartyUpdate {field} packed ObjectGuid"))?;
        self.offset = self
            .offset
            .checked_add(consumed)
            .ok_or_else(|| anyhow!("PartyUpdate {field} offset overflow"))?;
        Ok((low, high))
    }
}

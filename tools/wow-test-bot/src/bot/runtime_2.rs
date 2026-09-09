//! Runtime operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) async fn run_rested_xp_smoke_workflow_inner(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    fixture: &RestedXpSmokeFixture,
) -> Result<BotRunResult> {
    let mut wilderness_options = fixture.options.clone();
    wilderness_options.phase = RestedXpSmokePhase::OfflineWilderness;
    prepare_rested_xp_phase_async(
        bot.clone(),
        fixture.clone(),
        RestedXpSmokePhase::OfflineWilderness,
    )
    .await?;
    let mut combined = run_bot(
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
        Some(wilderness_options),
        None,
        None,
        None,
        None,
    )
    .await?;
    if !combined.rested_xp_smoke_passed.unwrap_or(false) {
        return Ok(combined);
    }
    let wilderness = load_rested_xp_db_state_async(bot.clone()).await?;
    let expected_wilderness = offline_rest_bonus_like_cpp(
        fixture.options.next_level_xp,
        fixture.offline_secs,
        REST_OFFLINE_WILDERNESS_BUBBLE,
        fixture.wilderness_rate,
    );
    if let Err(error) = validate_rested_xp_saved_state_shape(
        wilderness,
        fixture.test_level,
        0,
        0,
        "wilderness offline accrual",
    ) {
        set_rested_xp_failure(&mut combined, error.to_string());
        return Ok(combined);
    }
    if !offline_rest_bonus_matches_like_cpp(
        wilderness.rest_bonus,
        expected_wilderness,
        fixture.options.next_level_xp,
        REST_OFFLINE_WILDERNESS_BUBBLE,
        fixture.wilderness_rate,
        fixture.options.timeout_secs,
    ) {
        set_rested_xp_failure(
            &mut combined,
            format!(
                "wilderness offline bonus mismatch: expected approximately {expected_wilderness:.4}, got {:.4}",
                wilderness.rest_bonus
            ),
        );
        return Ok(combined);
    }
    combined.rested_xp_offline_wilderness_bonus = Some(wilderness.rest_bonus);

    let mut resting_options = fixture.options.clone();
    resting_options.phase = RestedXpSmokePhase::OfflineResting;
    prepare_rested_xp_phase_async(
        bot.clone(),
        fixture.clone(),
        RestedXpSmokePhase::OfflineResting,
    )
    .await?;
    let resting_result = run_bot(
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
        Some(resting_options),
        None,
        None,
        None,
        None,
    )
    .await?;
    merge_rested_xp_results(&mut combined, resting_result);
    if !combined.rested_xp_smoke_passed.unwrap_or(false) {
        return Ok(combined);
    }
    let resting = load_rested_xp_db_state_async(bot.clone()).await?;
    let expected_resting = offline_rest_bonus_like_cpp(
        fixture.options.next_level_xp,
        fixture.offline_secs,
        REST_OFFLINE_TAVERN_OR_CITY_BUBBLE,
        fixture.resting_rate,
    );
    if let Err(error) = validate_rested_xp_saved_state_shape(
        resting,
        fixture.test_level,
        0,
        0,
        "resting offline accrual",
    ) {
        set_rested_xp_failure(&mut combined, error.to_string());
        return Ok(combined);
    }
    if !offline_rest_bonus_matches_like_cpp(
        resting.rest_bonus,
        expected_resting,
        fixture.options.next_level_xp,
        REST_OFFLINE_TAVERN_OR_CITY_BUBBLE,
        fixture.resting_rate,
        fixture.options.timeout_secs,
    ) {
        set_rested_xp_failure(
            &mut combined,
            format!(
                "resting offline bonus mismatch: expected approximately {expected_resting:.4}, got {:.4}",
                resting.rest_bonus
            ),
        );
        return Ok(combined);
    }
    combined.rested_xp_offline_resting_bonus = Some(resting.rest_bonus);
    if resting.rest_bonus <= wilderness.rest_bonus {
        set_rested_xp_failure(
            &mut combined,
            format!(
                "resting offline bonus {:.4} was not greater than wilderness {:.4}",
                resting.rest_bonus, wilderness.rest_bonus
            ),
        );
        return Ok(combined);
    }

    let mut consume_options = fixture.options.clone();
    consume_options.phase = RestedXpSmokePhase::ConsumeKill;
    prepare_rested_xp_phase_async(
        bot.clone(),
        fixture.clone(),
        RestedXpSmokePhase::ConsumeKill,
    )
    .await?;
    let consume_result = run_bot(
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
        Some(consume_options),
        None,
        None,
        None,
        None,
    )
    .await?;
    merge_rested_xp_results(&mut combined, consume_result);
    if !combined.rested_xp_smoke_passed.unwrap_or(false) {
        return Ok(combined);
    }

    let expected_xp = combined
        .rested_xp_db_xp_after
        .context("rested-XP consume phase omitted persisted XP")?;
    let expected_rest_bonus = combined
        .rested_xp_db_rest_after
        .context("rested-XP consume phase omitted persisted rest bonus")?;
    let mut verify_options = fixture.options.clone();
    verify_options.phase = RestedXpSmokePhase::VerifyRelog;
    verify_options.expected_xp = Some(expected_xp);
    verify_options.expected_rest_bonus = Some(expected_rest_bonus);
    let verify_result = run_bot(
        bot,
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        None,
        None,
        Some(verify_options),
        None,
        None,
        None,
        None,
    )
    .await?;
    merge_rested_xp_results(&mut combined, verify_result);
    combined.rested_xp_smoke_passed =
        Some(combined.rested_xp_smoke_passed.unwrap_or(false) && combined.rested_xp_relog_verified);
    Ok(combined)
}
pub(crate) async fn run_bank_smoke_workflow(
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
        prepare_bank_smoke_fixture(&bot_for_setup, item_entry, runtime_counter, timeout_secs)
    })
    .await
    .map_err(|e| anyhow!("Bank smoke setup DB worker join failed: {e}"))??;

    let mut deposit_options = fixture.options.clone();
    deposit_options.phase = BankSmokePhase::Deposit;
    let first = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        Some(deposit_options),
        None,
        None,
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
                cleanup_bank_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
            })
            .await;
            return Err(error.context("Bank smoke deposit login/phase failed"));
        }
    };

    if combined.bank_smoke_passed.unwrap_or(false) {
        let mut withdraw_options = fixture.options.clone();
        withdraw_options.phase = BankSmokePhase::Withdraw;
        match run_bot(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            Some(withdraw_options),
            None,
            None,
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
                combined.world_auth &= second.world_auth;
                combined.enum_characters &= second.enum_characters;
                combined.player_login_verified &= second.player_login_verified;
                combined.bank_open_confirmed &= second.bank_open_confirmed;
                combined.bank_relogin_after_deposit = second.bank_relogin_after_deposit;
                combined.bank_withdraw_persisted = second.bank_withdraw_persisted;
                combined.seen_opcodes.extend(second.seen_opcodes);
                combined.bank_failure = second.bank_failure;
                combined.bank_smoke_passed = Some(
                    combined.bank_deposit_persisted
                        && combined.bank_relogin_after_deposit
                        && combined.bank_withdraw_persisted
                        && second.bank_smoke_passed.unwrap_or(false),
                );
            }
            Err(error) => {
                combined.bank_failure = Some(format!("Withdrawal relog/phase failed: {error}"));
                combined.bank_smoke_passed = Some(false);
            }
        }
    }

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_bank_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|e| anyhow!("Bank smoke cleanup DB worker join failed: {e}"))?;
    if let Err(error) = cleanup {
        combined.bank_failure = Some(format!("Bank fixture cleanup failed: {error}"));
        combined.bank_smoke_passed = Some(false);
    }

    Ok(combined)
}
pub(crate) async fn run_homebind_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_homebind_smoke_fixture(&bot_for_setup, runtime_counter, timeout_secs)
    })
    .await
    .map_err(|e| anyhow!("Homebind smoke setup DB worker join failed: {e}"))??;

    let first = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        Some(fixture.options.clone()),
        None,
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
            let cleanup = tokio::task::spawn_blocking(move || {
                cleanup_homebind_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
            })
            .await
            .map_err(|join_error| {
                anyhow!(
                    "Homebind smoke bind login/phase failed: {error}; cleanup worker failed: {join_error}"
                )
            })?;
            if let Err(cleanup_error) = cleanup {
                bail!(
                    "Homebind smoke bind login/phase failed: {error}; fixture cleanup failed: {cleanup_error}"
                );
            }
            return Err(error.context("Homebind smoke bind login/phase failed"));
        }
    };

    if combined.homebind_smoke_passed.unwrap_or(false) {
        let mut relog_options = fixture.options.clone();
        relog_options.phase = HomebindSmokePhase::VerifyRelog;
        let bot_for_db = bot.clone();
        relog_options.expected_homebind =
            match tokio::task::spawn_blocking(move || load_homebind_row(&bot_for_db)).await {
                Ok(Ok(Some(row))) => Some(row),
                Ok(Ok(None)) => {
                    combined.homebind_failure =
                        Some("character_homebind disappeared before relog".to_string());
                    combined.homebind_smoke_passed = Some(false);
                    None
                }
                Ok(Err(error)) => {
                    combined.homebind_failure =
                        Some(format!("Homebind expected-row query failed: {error}"));
                    combined.homebind_smoke_passed = Some(false);
                    None
                }
                Err(error) => {
                    combined.homebind_failure =
                        Some(format!("Homebind expected-row worker join failed: {error}"));
                    combined.homebind_smoke_passed = Some(false);
                    None
                }
            };
        if relog_options.expected_homebind.is_some() {
            match run_bot(
                bot.clone(),
                dungeon_id,
                lfg_secs,
                auto_teleport,
                false,
                None,
                None,
                Some(relog_options),
                None,
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
                    combined.world_auth &= second.world_auth;
                    combined.enum_characters &= second.enum_characters;
                    combined.player_login_verified &= second.player_login_verified;
                    combined.homebind_relogin_verified = second.homebind_relogin_verified;
                    combined.seen_opcodes.extend(second.seen_opcodes);
                    combined.homebind_failure = second.homebind_failure;
                    combined.homebind_smoke_passed = Some(
                        combined.homebind_spell_go_seen
                            && combined.homebind_bind_point_update_seen
                            && combined.homebind_player_bound_seen
                            && combined.homebind_gossip_complete_seen
                            && combined.homebind_db_persisted
                            && combined.homebind_relogin_verified
                            && second.homebind_smoke_passed.unwrap_or(false),
                    );
                }
                Err(error) => {
                    combined.homebind_failure = Some(format!("Homebind relog failed: {error}"));
                    combined.homebind_smoke_passed = Some(false);
                }
            }
        }
    }

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_homebind_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|e| anyhow!("Homebind smoke cleanup DB worker join failed: {e}"))?;
    if let Err(error) = cleanup {
        combined.homebind_failure = Some(format!("Homebind fixture cleanup failed: {error}"));
        combined.homebind_smoke_passed = Some(false);
    }

    Ok(combined)
}
pub(crate) async fn run_homebind_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &HomebindSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if options.phase == HomebindSmokePhase::Bind {
        let mut runtime_components = (!options.discover_runtime_guid).then(|| {
            let (_, low, high) = parse_packed_guid(&options.innkeeper.packed_guid)
                .expect("fixture packed GUID was built locally");
            (low, high)
        });
        let drain_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::time::Instant::now() < drain_deadline {
            match tokio::time::timeout(
                Duration::from_millis(250),
                read_encrypted_packet(stream, crypt, server_inflater),
            )
            .await
            {
                Ok(Ok((opcode, payload))) => {
                    result.seen_opcodes.push(format!("0x{opcode:04X}"));
                    if opcode == SMSG_UPDATE_OBJECT {
                        if let Some((low, high)) = find_creature_guid_in_update_object(
                            &payload,
                            options.innkeeper.map_id,
                            options.innkeeper.entry,
                        ) {
                            runtime_components = Some((low, high));
                            result.homebind_innkeeper_guid_counter = Some(low & 0xFF_FFFF_FFFF);
                        }
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => break,
            }
        }
        let (runtime_low, runtime_high) = runtime_components.ok_or_else(|| {
            anyhow!(
                "innkeeper entry {} was not discovered in login SMSG_UPDATE_OBJECT packets",
                options.innkeeper.entry
            )
        })?;
        let realm = realm_connection.as_mut().context(
            "homebind smoke requires distinct realm/instance sockets to validate C++ routing",
        )?;
        loop {
            match tokio::time::timeout(
                Duration::from_millis(250),
                read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater),
            )
            .await
            {
                Ok(Ok((opcode, payload))) => {
                    result.seen_opcodes.push(format!("0x{opcode:04X}"));
                    info!(
                        "[Bot {}] 📦 realm login drain {}",
                        bot_index,
                        parse_packet(opcode, &payload)
                    );
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => break,
            }
        }
        let runtime_guid = build_packed_guid(runtime_low, runtime_high);
        send_encrypted_packet(stream, crypt, CMSG_BINDER_ACTIVATE, &runtime_guid).await?;
        info!(
            "[Bot {}] ✅ CMSG_BINDER_ACTIVATE sent to entry={} spawn={}",
            bot_index, options.innkeeper.entry, options.innkeeper.spawn_guid
        );

        let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
        let mut bind_packet_homebind = None;
        let mut pending_player_bound = None;
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            enum HomebindReady {
                Instance,
                Realm,
            }
            let mut instance_peek = [0u8; 1];
            let mut realm_peek = [0u8; 1];
            let ready = tokio::time::timeout(remaining, async {
                tokio::select! {
                    result = stream.peek(&mut instance_peek) => {
                        if result.context("homebind instance peek failed")? == 0 {
                            bail!("instance connection closed during homebind smoke");
                        }
                        Ok(HomebindReady::Instance)
                    }
                    result = realm.stream.peek(&mut realm_peek) => {
                        if result.context("homebind realm peek failed")? == 0 {
                            bail!("realm connection closed during homebind smoke");
                        }
                        Ok(HomebindReady::Realm)
                    }
                }
            })
            .await;
            let ready = match ready {
                Ok(result) => result?,
                Err(_) => break,
            };
            let (connection, opcode, payload) = match ready {
                HomebindReady::Instance => {
                    let (opcode, payload) = tokio::time::timeout(
                        deadline.saturating_duration_since(tokio::time::Instant::now()),
                        read_encrypted_packet(stream, crypt, server_inflater),
                    )
                    .await
                    .map_err(|_| anyhow!("homebind instance packet read timed out"))??;
                    ("instance", opcode, payload)
                }
                HomebindReady::Realm => {
                    let (opcode, payload) = tokio::time::timeout(
                        deadline.saturating_duration_since(tokio::time::Instant::now()),
                        read_encrypted_packet(
                            &mut realm.stream,
                            &mut realm.crypt,
                            &mut realm.inflater,
                        ),
                    )
                    .await
                    .map_err(|_| anyhow!("homebind realm packet read timed out"))??;
                    ("realm", opcode, payload)
                }
            };
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            info!(
                "[Bot {}] 📦 {} {}",
                bot_index,
                connection,
                parse_packet(opcode, &payload)
            );
            match (connection, opcode) {
                ("instance", SMSG_SPELL_GO) => {
                    let player_high = (2u64 << 58) | ((u64::from(realm_id()) & 0x1FFF) << 42);
                    result.homebind_spell_go_seen = homebind_spell_go_seen_after_packet(
                        result.homebind_spell_go_seen,
                        &payload,
                        runtime_low,
                        runtime_high,
                        bot.character_guid,
                        player_high,
                    );
                }
                ("instance", SMSG_BIND_POINT_UPDATE) => {
                    if let Some(homebind) =
                        parse_bind_point_update(&payload, options.innkeeper.orientation)
                    {
                        result.homebind_bind_point_update_seen = true;
                        if let Some(player_bound) = pending_player_bound.as_deref() {
                            result.homebind_player_bound_seen = player_bound_matches(
                                player_bound,
                                runtime_low,
                                runtime_high,
                                u32::from(homebind.zone_id),
                            );
                        }
                        bind_packet_homebind = Some(homebind);
                    }
                }
                ("realm", SMSG_PLAYER_BOUND) => {
                    if let Some(expected) = bind_packet_homebind.as_ref() {
                        result.homebind_player_bound_seen = player_bound_matches(
                            &payload,
                            runtime_low,
                            runtime_high,
                            u32::from(expected.zone_id),
                        );
                    }
                    pending_player_bound = Some(payload);
                }
                ("realm", SMSG_GOSSIP_COMPLETE) => result.homebind_gossip_complete_seen = true,
                ("instance", SMSG_PLAYER_BOUND | SMSG_GOSSIP_COMPLETE) => {
                    bail!(
                        "{} arrived on instance; C++ routes it on realm",
                        parse_packet(opcode, &payload)
                    );
                }
                ("realm", SMSG_SPELL_GO | SMSG_BIND_POINT_UPDATE) => {
                    bail!(
                        "{} arrived on realm; C++ routes it on instance",
                        parse_packet(opcode, &payload)
                    );
                }
                _ => {}
            }
            if result.homebind_spell_go_seen
                && result.homebind_bind_point_update_seen
                && result.homebind_player_bound_seen
                && result.homebind_gossip_complete_seen
            {
                break;
            }
        }
        if !(result.homebind_spell_go_seen
            && result.homebind_bind_point_update_seen
            && result.homebind_player_bound_seen
            && result.homebind_gossip_complete_seen)
        {
            bail!(
                "missing bind responses: spell_go={} bind_update={} player_bound={} gossip_complete={}",
                result.homebind_spell_go_seen,
                result.homebind_bind_point_update_seen,
                result.homebind_player_bound_seen,
                result.homebind_gossip_complete_seen
            );
        }
        let expected_homebind = bind_packet_homebind
            .ok_or_else(|| anyhow!("BindPointUpdate payload could not be decoded"))?;
        let bot_for_db = bot.clone();
        let persistence_timeout = Duration::from_secs(options.timeout_secs.clamp(1, 10));
        result.homebind_db_persisted = tokio::task::spawn_blocking(move || {
            wait_for_homebind_row(&bot_for_db, &expected_homebind, persistence_timeout)
        })
        .await
        .map_err(|e| anyhow!("Homebind DB verification worker join failed: {e}"))??;
        if !result.homebind_db_persisted {
            bail!("character_homebind did not persist the live bind location");
        }
    } else {
        let expected_homebind = options
            .expected_homebind
            .clone()
            .ok_or_else(|| anyhow!("Homebind relog phase missing expected complete row"))?;
        let bot_for_db = bot.clone();
        result.homebind_relogin_verified = tokio::task::spawn_blocking(move || {
            Ok::<_, anyhow::Error>(
                load_homebind_row(&bot_for_db)?.as_ref() == Some(&expected_homebind),
            )
        })
        .await
        .map_err(|e| anyhow!("Homebind relog DB worker join failed: {e}"))??;
        if !result.homebind_relogin_verified {
            bail!("character_homebind changed before the verification relog completed");
        }
    }

    logout_and_wait(bot_index, stream, crypt, server_inflater, result).await?;
    result.homebind_smoke_passed = Some(true);
    Ok(())
}

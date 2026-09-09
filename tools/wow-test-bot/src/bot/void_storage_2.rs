//! Void storage operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) async fn run_bot_with_void_storage(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    login_only: bool,
    stand_state_options: Option<StandStateSmokeOptions>,
    bank_options: Option<BankSmokeOptions>,
    homebind_options: Option<HomebindSmokeOptions>,
    inventory_swap_options: Option<InventorySwapSmokeOptions>,
    vendor_options: Option<VendorSmokeOptions>,
    rested_xp_options: Option<RestedXpSmokeOptions>,
    loot_race_options: Option<loot_race::LootRaceOptions>,
    group_capacity_options: Option<loot_race::GroupCapacityRaceOptions>,
    equipment_set_options: Option<EquipmentSetSmokeOptions>,
    quest_options: Option<QuestSmokeOptions>,
    mut void_storage_options: Option<VoidStorageSmokeOptions>,
    detour_chase_options: Option<DetourChaseCaptureOptions>,
    creature_spell_options: Option<CreatureSpellCaptureOptions>,
    cast_lifecycle_options: Option<cast_lifecycle::Options>,
) -> Result<BotRunResult> {
    let bot_index = bot.account_id as usize;
    let void_storage_query_capture = void_storage_options
        .as_ref()
        .is_some_and(|options| options.phase == VoidStorageSmokePhase::QueryCapture);
    let mut result = BotRunResult {
        account: bot.account.clone(),
        account_id: bot.account_id,
        character_guid: bot.character_guid,
        dungeon_id,
        role: bot.lfg_role,
        join_result: None,
        join_detail: None,
        got_proposal: false,
        accepted_proposal: false,
        got_ready_check: false,
        group_formed: false,
        teleport_denied_reason: None,
        entered_world: false,
        world_auth: false,
        enum_characters: false,
        player_login_verified: false,
        login_instance_object_update_seen: false,
        login_stream_drained: false,
        login_save: None,
        spell_acquisition: None,
        cast_lifecycle: cast_lifecycle_options
            .as_ref()
            .map(|_| cast_lifecycle::Evidence::default()),
        login_only,
        stand_state_smoke: stand_state_options.is_some(),
        stand_state_smoke_passed: None,
        stand_states_requested: stand_state_options
            .as_ref()
            .map(|options| options.states.clone())
            .unwrap_or_default(),
        stand_states_confirmed: Vec::new(),
        stand_state_failure: None,
        bank_smoke: bank_options.is_some(),
        bank_smoke_passed: None,
        bank_banker_entry: bank_options.as_ref().map(|options| options.banker.entry),
        bank_banker_spawn_guid: bank_options
            .as_ref()
            .map(|options| options.banker.spawn_guid),
        bank_banker_guid_counter: bank_options
            .as_ref()
            .map(|options| options.banker.guid_counter),
        bank_item_guid: bank_options.as_ref().map(|options| options.item_guid),
        bank_item_entry: bank_options.as_ref().map(|options| options.item_entry),
        bank_inventory_slot: bank_options.as_ref().map(|options| options.inventory_slot),
        bank_bank_slot: bank_options.as_ref().map(|options| options.bank_slot),
        bank_open_confirmed: false,
        bank_deposit_persisted: false,
        bank_relogin_after_deposit: false,
        bank_withdraw_persisted: false,
        bank_failure: None,
        void_storage_smoke: void_storage_options.is_some() && !void_storage_query_capture,
        void_storage_smoke_passed: None,
        void_storage_query_capture,
        void_storage_query_capture_passed: None,
        void_storage_unlock_persisted: false,
        void_storage_deposit_persisted: false,
        void_storage_deposit_relogin_verified: false,
        void_storage_swap_persisted: false,
        void_storage_swap_relogin_verified: false,
        void_storage_withdraw_persisted: false,
        void_storage_withdraw_relogin_verified: false,
        void_storage_item_id: void_storage_options
            .as_ref()
            .and_then(|options| options.expected_void_item_id),
        void_storage_failure: None,
        homebind_smoke: homebind_options.is_some(),
        homebind_smoke_passed: None,
        homebind_innkeeper_entry: homebind_options
            .as_ref()
            .map(|options| options.innkeeper.entry),
        homebind_innkeeper_spawn_guid: homebind_options
            .as_ref()
            .map(|options| options.innkeeper.spawn_guid),
        homebind_innkeeper_guid_counter: homebind_options
            .as_ref()
            .map(|options| options.innkeeper.guid_counter),
        homebind_spell_go_seen: false,
        homebind_bind_point_update_seen: false,
        homebind_player_bound_seen: false,
        homebind_gossip_complete_seen: false,
        homebind_db_persisted: false,
        homebind_relogin_verified: false,
        homebind_failure: None,
        inventory_swap_smoke: inventory_swap_options.is_some(),
        inventory_swap_smoke_passed: None,
        inventory_swap_item_guid_a: inventory_swap_options
            .as_ref()
            .map(|options| options.item_guid_a),
        inventory_swap_item_guid_b: inventory_swap_options
            .as_ref()
            .map(|options| options.item_guid_b),
        inventory_swap_item_entry_a: inventory_swap_options
            .as_ref()
            .map(|options| options.item_entry_a),
        inventory_swap_item_entry_b: inventory_swap_options
            .as_ref()
            .map(|options| options.item_entry_b),
        inventory_swap_slot_a: inventory_swap_options
            .as_ref()
            .map(|options| options.slot_a),
        inventory_swap_slot_b: inventory_swap_options
            .as_ref()
            .map(|options| options.slot_b),
        inventory_swap_forward_persisted: false,
        inventory_swap_relogin_after_forward: false,
        inventory_swap_reverse_persisted: false,
        inventory_swap_relogin_after_reverse: false,
        inventory_swap_item_create_sha256: None,
        inventory_swap_item_create_relogin_verified: false,
        inventory_swap_item_metadata_persisted: false,
        inventory_swap_validation_gate_seen: false,
        inventory_swap_failure: None,
        vendor_smoke: vendor_options.is_some(),
        vendor_smoke_passed: None,
        vendor_entry: vendor_options.as_ref().map(|options| options.vendor.entry),
        vendor_spawn_guid: vendor_options
            .as_ref()
            .map(|options| options.vendor.spawn_guid),
        vendor_runtime_counter: vendor_options.as_ref().and_then(|options| {
            (options.vendor.guid_counter != 0).then_some(options.vendor.guid_counter)
        }),
        vendor_item_entry: vendor_options.as_ref().map(|options| options.item_entry),
        vendor_extended_cost: vendor_options.as_ref().map(|options| options.extended_cost),
        vendor_currency_id: vendor_options.as_ref().map(|options| options.currency_id),
        vendor_currency_before: vendor_options
            .as_ref()
            .map(|options| options.currency_before),
        vendor_currency_after: None,
        vendor_item_total_after: None,
        vendor_inventory_seen: false,
        vendor_buy_succeeded_seen: false,
        vendor_set_currency_seen: false,
        vendor_item_push_seen: false,
        vendor_relogin_verified: false,
        vendor_failure: None,
        equipment_set_smoke: equipment_set_options.is_some(),
        equipment_set_smoke_passed: None,
        equipment_set_type: equipment_set_options
            .as_ref()
            .map(|options| options.set_type),
        equipment_set_id: equipment_set_options.as_ref().map(|options| options.set_id),
        equipment_set_generated_guid: equipment_set_options
            .as_ref()
            .and_then(|options| options.expected_guid),
        equipment_set_login_count: None,
        equipment_set_load_seen: false,
        equipment_set_db_persisted: false,
        equipment_set_relogin_verified: false,
        equipment_set_failure: None,
        rested_xp_smoke: rested_xp_options.is_some(),
        rested_xp_smoke_passed: None,
        rested_xp_offline_wilderness_bonus: None,
        rested_xp_offline_resting_bonus: None,
        rested_xp_target_entry: rested_xp_options
            .as_ref()
            .map(|options| options.target.entry),
        rested_xp_target_spawn_guid: rested_xp_options
            .as_ref()
            .map(|options| options.target.spawn_guid),
        rested_xp_target_guid_counter: rested_xp_options.as_ref().and_then(|options| {
            (options.target.guid_counter != 0).then_some(options.target.guid_counter)
        }),
        rested_xp_packet_amount: None,
        rested_xp_packet_original: None,
        rested_xp_db_xp_before: None,
        rested_xp_db_xp_after: None,
        rested_xp_db_rest_before: None,
        rested_xp_db_rest_after: None,
        rested_xp_relog_verified: false,
        rested_xp_failure: None,
        detour_chase_capture: detour_chase_options.is_some(),
        detour_chase_capture_passed: None,
        detour_chase_target_entry: detour_chase_options
            .as_ref()
            .map(|options| options.target_entry),
        detour_chase_target_spawn_guid: detour_chase_options
            .as_ref()
            .map(|options| options.target_spawn_guid),
        detour_chase_target_runtime_counter: None,
        detour_chase_target_discovered: false,
        detour_chase_active_mover_ack_sent: false,
        detour_chase_attack_start_confirmed: false,
        detour_chase_first_swing_confirmed: false,
        detour_chase_prewindow_target_moves: 0,
        detour_chase_heartbeat_sent: false,
        detour_chase_heartbeat_sha256: None,
        detour_chase_window_target_moves: 0,
        detour_chase_monster_move_sha256: None,
        detour_chase_monster_move_bytes: None,
        detour_chase_ping_serial: None,
        detour_chase_pong_confirmed: false,
        detour_chase_time_sync_before_window: 0,
        detour_chase_time_sync_during_window: 0,
        detour_chase_time_sync_after_fence: 0,
        detour_chase_logout_confirmed: false,
        detour_chase_failure: None,
        creature_spell_capture: creature_spell_options.is_some(),
        creature_spell_capture_passed: None,
        creature_spell_fixture_manifest_sha256: creature_spell_options
            .as_ref()
            .map(|options| options.fixture_manifest_sha256.clone()),
        creature_spell_target_entry: creature_spell_options
            .as_ref()
            .map(|_| CREATURE_SPELL_FIXTURE_ENTRY),
        creature_spell_target_spawn_guid: creature_spell_options
            .as_ref()
            .map(|_| CREATURE_SPELL_FIXTURE_SPAWN_GUID),
        creature_spell_target_runtime_counter: None,
        creature_spell_target_discovered: false,
        creature_spell_heartbeat_sent: false,
        creature_spell_heartbeat_sha256: None,
        creature_spell_start_opcode: None,
        creature_spell_start_body_sha256: None,
        creature_spell_start_body_bytes: None,
        creature_spell_go_opcode: None,
        creature_spell_go_body_sha256: None,
        creature_spell_go_body_bytes: None,
        creature_spell_cast_id_low: None,
        creature_spell_cast_id_high: None,
        creature_spell_caster_guid_low: None,
        creature_spell_caster_guid_high: None,
        creature_spell_victim_guid_low: None,
        creature_spell_victim_guid_high: None,
        creature_spell_spell_id: None,
        creature_spell_start_cast_flags: None,
        creature_spell_go_cast_flags: None,
        creature_spell_cast_flags_ex: None,
        creature_spell_go_hit_target_count: None,
        creature_spell_go_miss_target_count: None,
        creature_spell_full_combat_log: None,
        creature_spell_advanced_logging_sent: false,
        creature_spell_adjacent_start_go: false,
        creature_spell_disconnect_confirmed: false,
        creature_spell_logout_confirmed: false,
        creature_spell_failure: None,
        loot_race_smoke: loot_race_options.is_some(),
        loot_race_smoke_passed: None,
        loot_race_target_entry: loot_race_options
            .as_ref()
            .map(|options| options.target.entry),
        loot_race_target_spawn_guid: loot_race_options
            .as_ref()
            .map(|options| options.target.spawn_guid),
        loot_race_target_runtime_counter: loot_race_options
            .as_ref()
            .and_then(|options| options.resolved_runtime_counter().ok()),
        loot_race_party_confirmed: false,
        loot_race_target_discovered: false,
        loot_race_loot_opened: false,
        loot_race_loot_list_id: None,
        loot_race_loot_coins: None,
        loot_race_item_push_seen: false,
        loot_race_loot_removed_seen: false,
        loot_race_money_notify_amount: None,
        loot_race_coin_removed_seen: false,
        loot_race_db_item_total: None,
        loot_race_db_money_delta: None,
        loot_race_relog_verified: false,
        loot_race_failure: None,
        group_capacity_race_smoke: group_capacity_options.is_some(),
        group_capacity_race_smoke_passed: None,
        group_capacity_group_id: group_capacity_options
            .as_ref()
            .map(|options| options.group_db_store_id),
        group_capacity_outcome: None,
        group_capacity_final_member_count: None,
        group_capacity_failure: None,
        quest_smoke: quest_options.is_some(),
        quest_smoke_passed: None,
        quest_target_entry: None,
        quest_target_spawn_guid: None,
        quest_target_guid_counter: None,
        quest_target_map_id: None,
        quest_gossip_hello_sent: false,
        quest_questgiver_hello_sent: false,
        quest_gossip_id_seen: None,
        quest_gossip_select_sent: false,
        quest_gossip_message_seen: false,
        quest_quest_list_seen: false,
        quest_details_seen: false,
        quest_request_items_seen: false,
        trainer_list_seen: false,
        trainer_id_seen: None,
        trainer_spell_count_seen: None,
        quest_accept_sent: false,
        quest_accept_confirm_seen: false,
        quest_db_verified: false,
        quest_db_status: None,
        quest_objective_persist: quest_options
            .as_ref()
            .is_some_and(|options| options.objective_persist),
        quest_objective_seeded: quest_options
            .as_ref()
            .map(|options| options.objective_seed.clone())
            .unwrap_or_default(),
        quest_objective_db_before: Vec::new(),
        quest_objective_db_after: Vec::new(),
        quest_objective_db_verified: false,
        quest_objective_update_seen: false,
        quest_objective_update_has_expected: false,
        quest_ids_seen: Vec::new(),
        quest_titles_seen: Vec::new(),
        quest_failure: None,
        seen_opcodes: Vec::new(),
    };

    let mut acquisition_plan = spell_acquisition::load()?;
    let login_save_before = if login_save::enabled() {
        if !login_only {
            bail!("login save check requires login-only mode");
        }
        let selected = bot.clone();
        Some(tokio::task::spawn_blocking(move || login_save::preflight(&selected)).await??)
    } else {
        None
    };

    // ── Step 1: Prepare the World session key ────────────────────────────────
    // The guarded equipment-set QA writes a fresh 64-byte fixture key for each
    // verified disposable account. Group-capacity QA may instead reuse a
    // configured 64-byte fixture key. Otherwise live BNet SRP6 computes
    // (login_ticket, K_32), where
    // K = SHA256(broken_evidence_le(S)), and expands it to K || SHA256(K).
    // Either path writes account.session_key_bnet before CMSG_AUTH_SESSION.
    info!(
        "[Bot {}] Step 1: Preparing World session key (configured group fixture or live SRP6 via {}:{})",
        bot_index,
        bnet_host(),
        bnet_port()
    );

    let configured_group_session_key = group_capacity_options
        .as_ref()
        .filter(|_| !bot.session_key_bnet.trim().is_empty())
        .map(|_| {
            hex::decode(bot.session_key_bnet.trim()).map_err(|error| {
                anyhow!(
                    "Configured group-capacity session_key_bnet for {} is not hex: {error}",
                    bot.account
                )
            })
        })
        .transpose()?;
    let generated_equipment_session_key = equipment_set_options.as_ref().map(|_| {
        let mut session_key = vec![0u8; 64];
        rand::thread_rng().fill_bytes(&mut session_key);
        session_key
    });
    let (session_key, used_fixture_session_key) =
        if let Some(session_key) = generated_equipment_session_key {
            (session_key, true)
        } else if let Some(session_key) = configured_group_session_key {
            if session_key.len() != 64 {
                bail!(
                    "Configured group-capacity session_key_bnet for {} has {} bytes, expected 64",
                    bot.account,
                    session_key.len()
                );
            }
            (session_key, true)
        } else {
            // Rusty's current BNet bot endpoint keeps challenge state that
            // cannot safely serve multiple fallback logins at once. Serialize
            // only this authentication exchange; all World connections and
            // the actual group accept race remain concurrent.
            let group_capacity_auth_guard = if let Some(options) = group_capacity_options.as_ref() {
                Some(options.auth_serial.lock().await)
            } else {
                None
            };
            let bnet_url = format!("https://{}:{}", bnet_host(), bnet_port());
            let (login_ticket, session_key_32) =
                bot_srp6::authenticate_bot(&bnet_url, &bot.account, &bot.password)
                    .await
                    .map_err(|e| anyhow!("Bot SRP6 failed: {}", e))?;
            drop(group_capacity_auth_guard);
            let _ = login_ticket; // BNet proof only; World auth uses the derived key.
            if session_key_32.len() != 32 {
                bail!(
                    "Bot SRP6 returned K of unexpected length: {}",
                    session_key_32.len()
                );
            }
            (expand_session_key(&session_key_32).to_vec(), false)
        };

    let account_for_db = bot.account.clone();
    let session_key_for_db = session_key.clone();
    let realm_id_for_db = realm_id();
    let world_auth_context = tokio::task::spawn_blocking(move || {
        prepare_world_auth_context(&account_for_db, &session_key_for_db, realm_id_for_db)
    })
    .await
    .map_err(|e| anyhow!("DB worker join failed for {}: {}", bot.account, e))?
    .map_err(|e| {
        anyhow!(
            "Failed to prepare world auth context for {}: {}",
            bot.account,
            e
        )
    })?;
    let wow_username = world_auth_context.username.clone();

    if used_fixture_session_key {
        info!(
            "[Bot {}] ✅ guarded fixture session key prepared (64B)",
            bot_index
        );
    } else {
        info!("[Bot {}] ✅ LoginTicket received", bot_index);
        info!("[Bot {}] ✅ K (live, 32B) received", bot_index);
    }
    info!(
        "[Bot {}] ✅ session_key_bnet (64B) written to account `{}`; realm build {} auth seed loaded",
        bot_index, wow_username, world_auth_context.realm_build
    );

    if let Some(quest_options) = &quest_options {
        if quest_smoke_needs_prelogin_db_setup(quest_options) {
            let bot_for_db = bot.clone();
            let options_for_db = quest_options.clone();
            tokio::task::spawn_blocking(move || {
                prepare_quest_smoke_before_login(&bot_for_db, &options_for_db)
            })
            .await
            .map_err(|e| anyhow!("Quest smoke setup DB worker join failed: {}", e))??;
        }
        if quest_options.objective_persist {
            let bot_for_db = bot.clone();
            let quest_id = quest_options
                .expected_quest_id
                .ok_or_else(|| anyhow!("Objective persistence requested without quest id"))?;
            result.quest_objective_db_before = tokio::task::spawn_blocking(move || {
                load_bot_quest_objectives(&bot_for_db, quest_id)
            })
            .await
            .map_err(|e| anyhow!("Quest objective before-load worker join failed: {}", e))??;
        }
    }

    // ── Step 2: Connect to World Server ─────────────────────────────────────
    info!(
        "[Bot {}] Step 2: Connecting to World Server {}:{}",
        bot_index,
        world_host(),
        world_port()
    );
    let world_addr = format!("{}:{}", world_host(), world_port());
    let mut stream =
        tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, TcpStream::connect(&world_addr))
            .await
            .map_err(|_| anyhow!("Timed out connecting to world server {world_addr}"))?
            .map_err(|e| anyhow!("Failed to connect to world server: {}", e))?;
    info!("[Bot {}] ✅ TCP connected", bot_index);

    // ── Step 3: World Server Handshake ──────────────────────────────────────
    info!("[Bot {}] Step 3: Handshake...", bot_index);
    let mut init_buf = vec![0u8; 256];
    let n = tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, stream.read(&mut init_buf))
        .await
        .map_err(|_| anyhow!("Timed out reading SERVER_INIT"))??;
    if !init_buf[..n].starts_with(&SERVER_INIT[..SERVER_INIT.len().min(n)]) {
        bail!(
            "Unexpected server init: {:?}",
            String::from_utf8_lossy(&init_buf[..n])
        );
    }
    info!("[Bot {}] ✅ SERVER_INIT received", bot_index);

    tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, async {
        stream.write_all(CLIENT_INIT).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| anyhow!("Timed out writing CLIENT_INIT"))??;
    info!("[Bot {}] ✅ CLIENT_INIT sent", bot_index);

    // ── Step 4: Read SMSG_AUTH_CHALLENGE ────────────────────────────────────
    info!("[Bot {}] Step 4: Reading SMSG_AUTH_CHALLENGE...", bot_index);
    let (opcode, challenge_data) = tokio::time::timeout(
        INITIAL_NETWORK_IO_TIMEOUT,
        read_unencrypted_packet(&mut stream),
    )
    .await
    .map_err(|_| anyhow!("Timed out reading SMSG_AUTH_CHALLENGE"))??;
    if opcode != 0x3048 {
        bail!(
            "Expected SMSG_AUTH_CHALLENGE (0x3048), got 0x{:04X}",
            opcode
        );
    }
    if challenge_data.len() < 48 {
        bail!(
            "SMSG_AUTH_CHALLENGE too short: {} bytes",
            challenge_data.len()
        );
    }
    let server_challenge: [u8; 16] = challenge_data[32..48].try_into()?;
    info!("[Bot {}] ✅ SMSG_AUTH_CHALLENGE received", bot_index);

    // ── Step 5: Send CMSG_AUTH_SESSION ──────────────────────────────────────
    info!(
        "[Bot {}] Step 5: Sending CMSG_AUTH_SESSION (build={})...",
        bot_index,
        client_build()
    );
    let local_challenge: [u8; 16] = rand::random();
    let digest = compute_auth_digest(
        &local_challenge,
        &server_challenge,
        &session_key,
        &world_auth_context.win64_auth_seed,
    );
    let derived_session_key =
        derive_realm_session_key(&session_key, &local_challenge, &server_challenge);

    // RealmJoinTicket on the worldserver side is the WoW account name (account.username),
    // World auth uses the game-account username and `session_key_bnet`, not a
    // BNet login ticket (sending that ticket yields "unknown account").
    let auth_data = build_cmsg_auth_session(realm_id(), &local_challenge, &digest, &wow_username);
    send_unencrypted_packet(&mut stream, 0x3765, &auth_data).await?;
    info!("[Bot {}] ✅ CMSG_AUTH_SESSION sent", bot_index);

    // ── Step 6: Wait for SMSG_AUTH_RESPONSE & encryption activation ─────────
    info!(
        "[Bot {}] Step 6: Waiting for auth response & encryption...",
        bot_index
    );
    let mut world_crypt: Option<WorldCrypt> = None;
    let mut encrypted = false;

    for _ in 0..10 {
        match tokio::time::timeout(Duration::from_secs(5), read_unencrypted_packet(&mut stream))
            .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                let parsed = parse_packet(op, &payload);
                info!("[Bot {}] 📦 {}", bot_index, parsed);

                if op == 0x256D {
                    // SMSG_AUTH_RESPONSE
                    info!("[Bot {}] ✅ SMSG_AUTH_RESPONSE received", bot_index);
                } else if op == 0x3049 {
                    // SMSG_ENTER_ENCRYPTED_MODE
                    info!("[Bot {}] ✅ SMSG_ENTER_ENCRYPTED_MODE received", bot_index);

                    let enc_key =
                        derive_encryption_key(&session_key, &local_challenge, &server_challenge);
                    info!(
                        "[Bot {}] Encryption key derived: {:02x}{:02x}...",
                        bot_index, enc_key[0], enc_key[1]
                    );

                    // Server's WorldPacketCrypt increments _clientCounter / _serverCounter
                    // on every packet — including the unencrypted SMSG_AUTH_CHALLENGE,
                    // SMSG_ENTER_ENCRYPTED_MODE, CMSG_AUTH_SESSION, and CMSG_ENTER_ENCRYPTED_MODE_ACK
                    // exchanges that happen before _authCrypt.Init() is called. By the
                    // time the first AES-GCM packet flies, both counters are at 2.
                    world_crypt = Some(WorldCrypt::new_with_counters(&enc_key, 2, 2));
                    encrypted = true;

                    // Send ACK
                    send_unencrypted_packet(&mut stream, 0x3767, &[]).await?;
                    info!("[Bot {}] ✅ CMSG_ENTER_ENCRYPTED_MODE_ACK sent", bot_index);
                    break;
                } else if op == 0x256E {
                    // SMSG_AUTH_RESPONSE (error variant)
                    warn!(
                        "[Bot {}] ⚠️ Auth response error code: {:?}",
                        bot_index,
                        payload.get(0)
                    );
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Error reading packet: {}", bot_index, e);
                break;
            }
            Err(_) => {
                warn!("[Bot {}] Timeout waiting for encryption", bot_index);
                break;
            }
        }
    }

    if !encrypted {
        bail!("Encryption not established");
    }
    result.world_auth = true;
    let mut crypt = world_crypt.take().unwrap();
    let mut server_inflater = ServerPacketInflater::default();
    let mut realm_connection: Option<EncryptedWorldConnection> = None;

    // ── Step 7a: Enumerate Characters ──────────────────────────────────────
    // The worldserver gates HandlePlayerLoginOpcode on `_legitCharacters` being
    // populated, which only happens after CMSG_ENUM_CHARACTERS is processed.
    // Skipping this step results in "Trying to login with a character of another account".
    info!(
        "[Bot {}] Step 7a: Sending CMSG_ENUM_CHARACTERS...",
        bot_index
    );
    send_encrypted_packet(&mut stream, &mut crypt, 0x35E9, &[]).await?;

    let mut enum_ok = false;
    for _ in 0..20 {
        match tokio::time::timeout(
            Duration::from_secs(3),
            read_encrypted_packet(&mut stream, &mut crypt, &mut server_inflater),
        )
        .await
        {
            Ok(Ok((op, _payload))) => {
                if op == 0x2583 {
                    // SMSG_ENUM_CHARACTERS_RESULT
                    info!(
                        "[Bot {}] ✅ SMSG_ENUM_CHARACTERS_RESULT received",
                        bot_index
                    );
                    enum_ok = true;
                    break;
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Enum read error: {}", bot_index, e);
                break;
            }
            Err(_) => { /* fall through to retry */ }
        }
    }
    if !enum_ok {
        bail!("Did not receive SMSG_ENUM_CHARACTERS_RESULT");
    }
    result.enum_characters = true;

    // ── Step 7b: Player Login ──────────────────────────────────────────────
    info!(
        "[Bot {}] Step 7b: Sending CMSG_PLAYER_LOGIN (guid={})...",
        bot_index, bot.character_guid
    );
    let login_data = build_player_login(bot.character_guid, realm_id(), 500.0);
    send_encrypted_packet(&mut stream, &mut crypt, 0x35EB, &login_data).await?;
    info!("[Bot {}] ✅ CMSG_PLAYER_LOGIN sent", bot_index);

    let mut login_ok = false;
    let expected_known_spells = std::env::var("WOW_BOT_LOGIN_EXPECT_KNOWN_SPELLS")
        .ok()
        .map(|raw| parse_login_known_spells_expectation(&raw))
        .transpose()?;
    if expected_known_spells.is_some() && !login_only {
        bail!("WOW_BOT_LOGIN_EXPECT_KNOWN_SPELLS requires login-only mode");
    }
    let require_known_spells = login_only
        && (login_save_before.is_some()
            || expected_known_spells.is_some()
            || std::env::var("WOW_BOT_LOGIN_REQUIRE_KNOWN_SPELLS")
                .is_ok_and(|value| is_truthy(&value)));
    let mut known_spells_seen = false;
    let mut saved_known_spells = None;
    let mut loot_race_target_seen = false;
    let mut vendor_target_seen: Option<DiscoveredCreatureGuid> = None;
    let mut void_storage_target_seen: Option<DiscoveredCreatureGuid> = None;
    let mut detour_chase_target_seen: Option<DiscoveredCreatureGuid> = None;
    let mut creature_spell_target_seen: Option<DiscoveredCreatureGuid> = None;
    let login_budget = LoginVerifyBudget::new(LOGIN_VERIFY_TIMEOUT);
    while let Some(read_timeout) = login_budget.next_read_timeout() {
        match tokio::time::timeout(
            read_timeout,
            read_encrypted_packet(&mut stream, &mut crypt, &mut server_inflater),
        )
        .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                login_stream::observe_login_packet(realm_connection.is_some(), op, &mut result);
                if op == SMSG_SEND_KNOWN_SPELLS {
                    let decoded = decode_login_known_spells_like_cpp(&payload)?;
                    if !decoded.initial_login {
                        bail!("login SMSG_SEND_KNOWN_SPELLS has InitialLogin=false");
                    }
                    if let Some(expected) = expected_known_spells.as_ref() {
                        if decoded.known_spells != *expected {
                            bail!(
                                "SMSG_SEND_KNOWN_SPELLS mismatch: expected {:?}, received {:?}",
                                expected,
                                decoded.known_spells
                            );
                        }
                    }
                    known_spells_seen = true;
                    saved_known_spells = Some(decoded.clone());
                    info!(
                        "[Bot {}] ✅ SMSG_SEND_KNOWN_SPELLS received (known={}, favorites={})",
                        bot_index,
                        decoded.known_spells.len(),
                        decoded.favorite_spells.len()
                    );
                }
                if let Some(options) = loot_race_options.as_ref() {
                    if let Some(counter) = loot_race::target_seen_in_update(options, op, &payload)?
                    {
                        loot_race_target_seen = true;
                        result.loot_race_target_runtime_counter = Some(counter);
                    }
                }
                if let Some(options) = vendor_options.as_ref() {
                    let candidate = (op == SMSG_UPDATE_OBJECT)
                        .then(|| {
                            find_creature_guid_near_position_in_update_object(
                                &payload,
                                options.vendor.map_id,
                                options.vendor.entry,
                                options.vendor.x as f32,
                                options.vendor.y as f32,
                                options.vendor.z as f32,
                                options.target_match_radius,
                                (options.vendor.guid_counter != 0).then_some(
                                    options.vendor.guid_counter & OBJECT_GUID_COUNTER_MASK,
                                ),
                            )
                        })
                        .flatten();
                    if let Some(candidate) = candidate {
                        match vendor_target_seen {
                            Some(previous)
                                if (previous.low, previous.high)
                                    != (candidate.low, candidate.high) =>
                            {
                                bail!(
                                    "vendor login discovery produced two different live candidates near SQL spawn {}",
                                    options.vendor.spawn_guid
                                );
                            }
                            _ => vendor_target_seen = Some(candidate),
                        }
                    }
                }
                if let Some(options) = void_storage_options.as_ref() {
                    let expected_counter = (!options.discover_runtime_guid)
                        .then_some(options.vault_keeper.guid_counter & OBJECT_GUID_COUNTER_MASK);
                    let candidate = (op == SMSG_UPDATE_OBJECT)
                        .then(|| {
                            find_creature_guid_near_position_in_update_object(
                                &payload,
                                options.vault_keeper.map_id,
                                options.vault_keeper.entry,
                                options.vault_keeper.x as f32,
                                options.vault_keeper.y as f32,
                                options.vault_keeper.z as f32,
                                10.0,
                                expected_counter,
                            )
                        })
                        .flatten();
                    if let Some(candidate) = candidate {
                        match void_storage_target_seen {
                            Some(previous)
                                if (previous.low, previous.high)
                                    != (candidate.low, candidate.high) =>
                            {
                                bail!(
                                    "void-storage login discovery produced two different live candidates near SQL spawn {}",
                                    options.vault_keeper.spawn_guid
                                );
                            }
                            _ => void_storage_target_seen = Some(candidate),
                        }
                    }
                }
                if let Some(options) = detour_chase_options.as_ref() {
                    let candidates = (op == SMSG_UPDATE_OBJECT)
                        .then(|| {
                            find_creature_guids_near_position_in_update_object(
                                &payload,
                                options.map_id,
                                options.target_entry,
                                options.target_x,
                                options.target_y,
                                options.target_z,
                                DETOUR_CHASE_TARGET_MATCH_RADIUS,
                                None,
                            )
                        })
                        .unwrap_or_default();
                    if candidates.len() > 1 {
                        bail!(
                            "detour login discovery found {} same-entry live candidates inside the pinned spawn radius",
                            candidates.len()
                        );
                    }
                    if let Some(candidate) = candidates.into_iter().next() {
                        match detour_chase_target_seen {
                            Some(previous)
                                if (previous.low, previous.high)
                                    != (candidate.low, candidate.high) =>
                            {
                                bail!(
                                    "detour login discovery produced two different live candidates for pinned spawn {}",
                                    options.target_spawn_guid
                                );
                            }
                            _ => detour_chase_target_seen = Some(candidate),
                        }
                    }
                }
                if creature_spell_options.is_some() {
                    if op == SMSG_SPELL_START {
                        bail!(
                            "creature began casting during login before the bot's body-pull heartbeat; the pre-login position is not isolated"
                        );
                    }
                    let candidates = (op == SMSG_UPDATE_OBJECT)
                        .then(|| {
                            find_creature_guids_near_position_in_update_object(
                                &payload,
                                CREATURE_SPELL_FIXTURE_MAP_ID,
                                CREATURE_SPELL_FIXTURE_ENTRY,
                                CREATURE_SPELL_FIXTURE_X,
                                CREATURE_SPELL_FIXTURE_Y,
                                CREATURE_SPELL_FIXTURE_Z,
                                CREATURE_SPELL_TARGET_MATCH_RADIUS,
                                None,
                            )
                        })
                        .unwrap_or_default();
                    if candidates.len() > 1 {
                        bail!(
                            "creature spell login discovery found {} same-entry live candidates inside the pinned spawn radius",
                            candidates.len()
                        );
                    }
                    if let Some(candidate) = candidates.into_iter().next() {
                        match creature_spell_target_seen {
                            Some(previous)
                                if (previous.low, previous.high)
                                    != (candidate.low, candidate.high) =>
                            {
                                bail!(
                                    "creature spell login discovery produced two different live candidates for pinned spawn {}",
                                    CREATURE_SPELL_FIXTURE_SPAWN_GUID
                                );
                            }
                            _ => creature_spell_target_seen = Some(candidate),
                        }
                    }
                }
                if let Some(options) = quest_options.as_ref() {
                    record_quest_objective_login_signal(op, &payload, options, &mut result);
                }
                if let Some(options) = equipment_set_options.as_ref() {
                    record_equipment_set_login_signal(op, &payload, options, &mut result)?;
                }
                if let Some(options) = inventory_swap_options.as_ref() {
                    if op == SMSG_UPDATE_OBJECT {
                        record_issue20_item_create_evidence(
                            &payload,
                            options,
                            bot.character_guid,
                            &mut result,
                        )?;
                    }
                }
                if let Some(plan) = acquisition_plan.as_mut() {
                    plan.observe_login(op, &payload, &mut stream, &mut crypt)
                        .await?;
                }
                let acquisition_ready = acquisition_plan.as_ref().is_none_or(|p| p.login_ready());
                if op == 0x2597 {
                    // SMSG_LOGIN_VERIFY_WORLD
                    info!("[Bot {}] ✅ SMSG_LOGIN_VERIFY_WORLD received", bot_index);
                    login_ok = true;
                    let equipment_set_login_ready = equipment_set_options
                        .as_ref()
                        .is_none_or(|_| result.equipment_set_load_seen);
                    let void_storage_login_ready =
                        void_storage_options.as_ref().is_none_or(|options| {
                            void_storage_login_target_ready(
                                options.discover_runtime_guid,
                                void_storage_target_seen.is_some(),
                            )
                        });
                    let inventory_swap_login_ready = inventory_swap_options
                        .as_ref()
                        .is_none_or(|_| result.inventory_swap_item_create_sha256.is_some());
                    if login_known_spells_ready(login_ok, require_known_spells, known_spells_seen)
                        && realm_connection.is_some()
                        && equipment_set_login_ready
                        && void_storage_login_ready
                        && inventory_swap_login_ready
                        && acquisition_ready
                    {
                        break;
                    }
                    // Routing-sensitive captures validate both connections, so
                    // keep reading the realm socket until SMSG_CONNECT_TO has
                    // created and authenticated a distinct instance socket.
                    continue;
                }

                if op == 0x304D {
                    let connect_to = parse_connect_to(&payload)
                        .ok_or_else(|| anyhow!("Unable to parse SMSG_CONNECT_TO payload"))?;
                    info!(
                        "[Bot {}] SMSG_CONNECT_TO: {}:{} serial={} con={} key={}",
                        bot_index,
                        connect_to.address,
                        connect_to.port,
                        connect_to.serial,
                        connect_to.connection_type,
                        connect_to.key
                    );
                    let (instance_stream, instance_crypt) =
                        connect_to_instance(bot_index, &connect_to, &derived_session_key).await?;
                    // The server keeps cross-socket ordering fences alive after
                    // SMSG_CONNECT_TO, so the realm socket must stay open on every
                    // mode: closing it makes the realm writer disappear before it
                    // acknowledges the fence and the session is kicked with
                    // "login packet sequence failed".
                    if realm_connection.is_some() {
                        bail!("Routing smoke received more than one SMSG_CONNECT_TO");
                    }
                    let realm_stream = std::mem::replace(&mut stream, instance_stream);
                    let realm_crypt = std::mem::replace(&mut crypt, instance_crypt);
                    let realm_inflater = std::mem::take(&mut server_inflater);
                    realm_connection = Some(EncryptedWorldConnection {
                        stream: realm_stream,
                        crypt: realm_crypt,
                        inflater: realm_inflater,
                    });
                    info!("[Bot {}] ✅ Instance socket authenticated", bot_index);
                    let void_storage_login_ready =
                        void_storage_options.as_ref().is_none_or(|options| {
                            void_storage_login_target_ready(
                                options.discover_runtime_guid,
                                void_storage_target_seen.is_some(),
                            )
                        });
                    let inventory_swap_login_ready = inventory_swap_options
                        .as_ref()
                        .is_none_or(|_| result.inventory_swap_item_create_sha256.is_some());
                    if login_ok
                        && void_storage_login_ready
                        && inventory_swap_login_ready
                        && acquisition_ready
                    {
                        break;
                    }
                } else if op == 0x304B {
                    // SMSG_RESUME_COMMS
                    info!("[Bot {}] ✅ SMSG_RESUME_COMMS received", bot_index);
                }
                if equipment_set_options.is_some()
                    && login_ok
                    && result.equipment_set_load_seen
                    && realm_connection.is_some()
                {
                    break;
                }
                if inventory_swap_options.is_some()
                    && login_ok
                    && result.inventory_swap_item_create_sha256.is_some()
                    && realm_connection.is_some()
                {
                    break;
                }
                if require_known_spells
                    && login_known_spells_ready(login_ok, true, known_spells_seen)
                    && acquisition_ready
                {
                    break;
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Login read error: {}", bot_index, e);
                break;
            }
            // A quiet five-second slice does not invalidate an otherwise
            // healthy login. The absolute deadline above remains the guard.
            Err(_) => continue,
        }
    }
    if !login_ok {
        bail!("Login verification failed");
    }
    if require_known_spells && !known_spells_seen {
        bail!("Login verification did not reach SMSG_SEND_KNOWN_SPELLS");
    }
    if inventory_swap_options.is_some() && result.inventory_swap_item_create_sha256.is_none() {
        bail!("issue #20 item CREATE_OBJECT was not observed during the login window");
    }
    result.player_login_verified = true;

    if let Some(cast_lifecycle_options) = cast_lifecycle_options {
        cast_lifecycle::run_after_login(
            cast_lifecycle_options,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &mut result,
        )
        .await;
        return Ok(result);
    }

    if let Some(stand_state_options) = stand_state_options {
        run_stand_state_smoke(
            bot_index,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &stand_state_options,
            &mut result,
        )
        .await;
        return Ok(result);
    }

    if let Some(quest_options) = quest_options {
        run_quest_smoke(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &quest_options,
            &mut result,
        )
        .await;
        if quest_options.objective_persist {
            if let Err(e) = logout_and_verify_quest_objectives(
                bot_index,
                &bot,
                &mut stream,
                &mut crypt,
                &mut server_inflater,
                &quest_options,
                &mut result,
            )
            .await
            {
                result.quest_failure = Some(format!("Quest objective persist QA failed: {e}"));
                result.quest_smoke_passed = Some(false);
            } else {
                result.quest_smoke_passed = Some(quest_smoke_passes(&quest_options, &mut result));
            }
        }
        return Ok(result);
    }

    if let Some(bank_options) = bank_options {
        if let Err(error) = run_bank_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &bank_options,
            &mut result,
        )
        .await
        {
            result.bank_failure = Some(error.to_string());
            result.bank_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(mut void_storage_options) = void_storage_options.take() {
        if void_storage_options.discover_runtime_guid {
            let discovered = void_storage_target_seen.ok_or_else(|| {
                anyhow!(
                    "void-storage vault keeper entry {} spawn {} was not discovered in login object updates",
                    void_storage_options.vault_keeper.entry,
                    void_storage_options.vault_keeper.spawn_guid
                )
            })?;
            void_storage_options.vault_keeper.guid_counter = discovered.low;
            void_storage_options.vault_keeper.packed_guid =
                build_packed_guid(discovered.low, discovered.high);
        }
        if let Err(error) = run_void_storage_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &void_storage_options,
            &mut result,
        )
        .await
        {
            result.void_storage_failure = Some(error.to_string());
            if result.void_storage_query_capture {
                result.void_storage_query_capture_passed = Some(false);
            } else {
                result.void_storage_smoke_passed = Some(false);
            }
        }
        return Ok(result);
    }

    if let Some(homebind_options) = homebind_options {
        if let Err(error) = run_homebind_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &homebind_options,
            &mut result,
        )
        .await
        {
            result.homebind_failure = Some(error.to_string());
            result.homebind_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(inventory_swap_options) = inventory_swap_options {
        if let Err(error) = run_inventory_swap_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &inventory_swap_options,
            &mut result,
        )
        .await
        {
            result.inventory_swap_failure = Some(error.to_string());
            result.inventory_swap_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(vendor_options) = vendor_options {
        if let Err(error) = run_vendor_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &vendor_options,
            vendor_target_seen,
            &mut result,
        )
        .await
        {
            let mut failure = error.to_string();
            if let Err(logout_error) = loot_race::logout_and_wait_routed_like_cpp(
                bot_index,
                &mut stream,
                &mut crypt,
                &mut server_inflater,
                realm_connection.as_mut(),
                bot.character_guid,
                &mut result,
            )
            .await
            {
                failure.push_str(&format!(
                    "; graceful logout after failure also failed: {logout_error}"
                ));
            }
            result.vendor_failure = Some(failure);
            result.vendor_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(rested_xp_options) = rested_xp_options {
        if let Err(error) = run_rested_xp_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &rested_xp_options,
            &mut result,
        )
        .await
        {
            result.rested_xp_failure = Some(error.to_string());
            result.rested_xp_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(detour_chase_options) = detour_chase_options {
        if let Err(error) = run_detour_chase_capture_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &detour_chase_options,
            detour_chase_target_seen,
            &mut result,
        )
        .await
        {
            result.detour_chase_failure = Some(error.to_string());
            result.detour_chase_capture_passed = Some(false);
            if !result.detour_chase_logout_confirmed {
                match loot_race::logout_and_wait_routed_like_cpp(
                    bot_index,
                    &mut stream,
                    &mut crypt,
                    &mut server_inflater,
                    realm_connection.as_mut(),
                    bot.character_guid,
                    &mut result,
                )
                .await
                {
                    Ok(_) => result.detour_chase_logout_confirmed = true,
                    Err(logout_error) => {
                        let failure = result.detour_chase_failure.get_or_insert_with(String::new);
                        failure.push_str(&format!(
                            "; graceful logout after failure also failed: {logout_error}"
                        ));
                    }
                }
            }
        }
        return Ok(result);
    }

    if let Some(creature_spell_options) = creature_spell_options {
        let capture_error = run_creature_spell_capture_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &creature_spell_options,
            creature_spell_target_seen,
            &mut result,
        )
        .await
        .err();

        let disconnect_result =
            disconnect_creature_spell_sockets(bot_index, &mut stream, &mut realm_connection).await;
        if disconnect_result.is_ok() {
            result.creature_spell_disconnect_confirmed = true;
        }

        match (capture_error, disconnect_result) {
            (None, Ok(())) => result.creature_spell_capture_passed = Some(true),
            (Some(error), Ok(())) => {
                result.creature_spell_failure = Some(error.to_string());
                result.creature_spell_capture_passed = Some(false);
            }
            (None, Err(disconnect_error)) => {
                result.creature_spell_failure = Some(format!(
                    "creature spell capture did not confirm both socket shutdowns: {disconnect_error}"
                ));
                result.creature_spell_capture_passed = Some(false);
            }
            (Some(error), Err(disconnect_error)) => {
                result.creature_spell_failure = Some(format!(
                    "{error}; best-effort socket disconnect after failure also failed: {disconnect_error}"
                ));
                result.creature_spell_capture_passed = Some(false);
            }
        }
        return Ok(result);
    }

    if let Some(loot_race_options) = loot_race_options {
        if let Err(error) = loot_race::run_phase(
            bot_index,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &loot_race_options,
            loot_race_target_seen,
            &mut result,
        )
        .await
        {
            result.loot_race_failure = Some(error.to_string());
            result.loot_race_smoke_passed = Some(false);
            loot_race::best_effort_close(
                bot_index,
                &mut stream,
                &mut crypt,
                &mut server_inflater,
                &mut realm_connection,
                loot_race_options.character_guid,
                &mut result,
            )
            .await;
        }
        return Ok(result);
    }

    if let Some(group_capacity_options) = group_capacity_options {
        if let Err(error) = loot_race::run_group_capacity_phase(
            bot_index,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &group_capacity_options,
            &mut result,
        )
        .await
        {
            result.group_capacity_failure = Some(error.to_string());
            result.group_capacity_race_smoke_passed = Some(false);
            loot_race::best_effort_logout_preserving_group(
                bot_index,
                &mut stream,
                &mut crypt,
                &mut server_inflater,
                &mut realm_connection,
                group_capacity_options.character_guid,
                &mut result,
            )
            .await;
        }
        return Ok(result);
    }

    if let Some(equipment_set_options) = equipment_set_options {
        if let Err(error) = run_equipment_set_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            realm_connection.as_mut(),
            &equipment_set_options,
            &mut result,
        )
        .await
        {
            result.equipment_set_failure = Some(error.to_string());
            result.equipment_set_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if login_only {
        login_save::finish_login(
            acquisition_plan.as_ref(),
            bot_index,
            &bot,
            login_save_before,
            saved_known_spells,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &mut result,
        )
        .await?;
        info!(
            "[Bot {}] ✅ Login-only smoke passed: world_auth=true enum_characters=true player_login=true",
            bot_index
        );
        return Ok(result);
    }

    // ── Step 8: LFG Setup ───────────────────────────────────────────────────
    // Opcodes verified against src/server/game/Server/Protocol/Opcodes.h:
    //   CMSG_DF_SET_ROLES = 0x3617    (was incorrectly 0x35EE)
    //   CMSG_DF_JOIN      = 0x360B
    //   CMSG_DF_READY_CHECK_RESPONSE = 0x361C  (was 0x360D)
    //   SMSG_LFG_JOIN_RESULT         = 0x2A1C  (was 0x2F0C)
    //   SMSG_LFG_READY_CHECK_UPDATE  = 0x2A22  (was 0x2F0E)
    info!(
        "[Bot {}] Step 8: Setting up LFG (role={})...",
        bot_index, bot.lfg_role
    );
    tokio::time::sleep(Duration::from_secs(1)).await;

    // CMSG_DF_SET_ROLES wire format (per LFGPackets.cpp::DFSetRoles::Read):
    //   bit:  hasPartyIndex (false here → 0)
    //   u8:   RolesDesired
    // ByteBuffer aligns to byte after bits, so the wire is [bit_byte=0x00, role].
    let role_data = vec![0x00, bot.lfg_role];
    send_encrypted_packet(&mut stream, &mut crypt, 0x3617, &role_data).await?;
    info!("[Bot {}] ✅ CMSG_DF_SET_ROLES sent", bot_index);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // ── Step 9: Join Defense Protocol Alpha (259) ───────────────────────────
    info!(
        "[Bot {}] Step 9: Joining LFG dungeon {}...",
        bot_index, dungeon_id
    );
    let join_data = build_lfg_join(dungeon_id, bot.lfg_role);
    send_encrypted_packet(&mut stream, &mut crypt, 0x360B, &join_data).await?;
    info!("[Bot {}] ✅ CMSG_DF_JOIN sent", bot_index);

    // ── Step 10: Wait for LFG result ────────────────────────────────────────
    let mut result_code = 255u8;
    let mut detail_code = 255u8;

    // Read until we hit the configured overall budget. We stay in the queue past the initial
    // SMSG_LFG_JOIN_RESULT so LFGMgr has time to match all 5 bots, send a proposal,
    // collect accepts, and run the ready check — which is what proves group formation.
    // the queue and needs time to log in, set roles, and accept the proposal.
    info!("[Bot {}] LFG read window: {}s", bot_index, lfg_secs);
    let lfg_deadline = tokio::time::Instant::now() + Duration::from_secs(lfg_secs);
    let mut got_proposal = false;
    let mut group_formed = false;
    while tokio::time::Instant::now() < lfg_deadline {
        let remaining = lfg_deadline - tokio::time::Instant::now();
        match tokio::time::timeout(
            remaining,
            read_encrypted_packet(&mut stream, &mut crypt, &mut server_inflater),
        )
        .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                let parsed = parse_packet(op, &payload);
                info!("[Bot {}] 📦 {}", bot_index, parsed);

                if op == 0x2A1C {
                    // SMSG_LFG_JOIN_RESULT
                    // Use the proper parser — the result/detail bytes live AFTER the
                    // RideTicket prefix (PackedGuid + 12 bytes + 1 bit pad), not at offset 0.
                    if let Some(r) = packet_parser::parse_lfg_join_result(&payload) {
                        result_code = r.result;
                        detail_code = r.result_detail;
                        result.join_result = Some(r.result);
                        result.join_detail = Some(r.result_detail);
                        info!(
                            "[Bot {}] 🎯 LFG RESULT: result={}, detail={}",
                            bot_index, result_code, detail_code
                        );
                    }
                }

                if op == 0x2A2D {
                    // SMSG_LFG_PROPOSAL_UPDATE
                    info!(
                        "[Bot {}] 📜 SMSG_LFG_PROPOSAL_UPDATE ({} bytes)",
                        bot_index,
                        payload.len()
                    );
                    if let Some(resp) = build_proposal_response(&payload) {
                        send_encrypted_packet(&mut stream, &mut crypt, 0x3609, &resp).await?;
                        info!(
                            "[Bot {}] ✅ CMSG_DF_PROPOSAL_RESPONSE sent (Accepted=true)",
                            bot_index
                        );
                        got_proposal = true;
                        result.got_proposal = true;
                        result.accepted_proposal = true;
                    } else {
                        warn!(
                            "[Bot {}] Proposal payload too short to parse Ticket prefix",
                            bot_index
                        );
                    }
                }

                if op == 0x2A22 {
                    // SMSG_LFG_READY_CHECK_UPDATE
                    info!("[Bot {}] 📢 LFG Ready Check received", bot_index);
                    result.got_ready_check = true;
                    let ready_data = vec![1u8];
                    send_encrypted_packet(&mut stream, &mut crypt, 0x361C, &ready_data).await?;
                    info!(
                        "[Bot {}] ✅ CMSG_DF_READY_CHECK_RESPONSE sent (Ready=true)",
                        bot_index
                    );
                }

                if op == 0x2A36 {
                    // SMSG_LFG_PARTY_INFO — sent when group is committed
                    info!("[Bot {}] 🎉 SMSG_LFG_PARTY_INFO — group formed!", bot_index);
                    group_formed = true;
                    result.group_formed = true;
                    if auto_teleport {
                        send_encrypted_packet(&mut stream, &mut crypt, 0x3619, &[1u8]).await?;
                        info!(
                            "[Bot {}] ✅ CMSG_DF_TELEPORT sent (teleport_out=false)",
                            bot_index
                        );
                    }
                }

                if op == 0x2594 {
                    // SMSG_NEW_WORLD
                    info!("[Bot {}] 🌍 SMSG_NEW_WORLD ({} bytes) — replying with CMSG_WORLD_PORT_RESPONSE", bot_index, payload.len());
                    send_encrypted_packet(&mut stream, &mut crypt, 0x35FA, &[]).await?;
                    info!(
                        "[Bot {}] ✅ CMSG_WORLD_PORT_RESPONSE sent — should now be inside map",
                        bot_index
                    );
                    result.entered_world = true;
                }

                if op == 0x2A32 {
                    // SMSG_LFG_TELEPORT_DENIED (1 byte body, reason code high nibble)
                    let reason_byte = payload.first().copied().unwrap_or(0);
                    let reason = reason_byte >> 4;
                    warn!(
                        "[Bot {}] ⛔ SMSG_LFG_TELEPORT_DENIED reason={} (raw=0x{:02X})",
                        bot_index, reason, reason_byte
                    );
                    result.teleport_denied_reason = Some(reason);
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Error: {}", bot_index, e);
                break;
            }
            Err(_) => {
                debug!("[Bot {}] LFG read budget exhausted", bot_index);
                break;
            }
        }
    }
    let _ = (got_proposal, group_formed); // mirrored in result; keep locals for readable logs while debugging

    tokio::time::sleep(Duration::from_secs(3)).await;
    info!(
        "[Bot {}] 🏁 Completed: result={}, detail={}",
        bot_index, result_code, detail_code
    );
    Ok(result)
}

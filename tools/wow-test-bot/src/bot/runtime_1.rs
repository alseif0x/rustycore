//! Runtime operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn add_pinned_detour_fixture_bot_if_missing(
    bots: &mut Vec<config::BotConfig>,
    detour_chase_capture: bool,
) {
    if !detour_chase_capture || !bots.is_empty() {
        return;
    }

    // The committed bot config intentionally need not contain the disposable
    // TESTBOT2 identity or a credential. The explicit acknowledgement, exact
    // --single gate, read-only DB preflight, and pinned manifest select this
    // bounded identity. Its password still comes only from the environment.
    bots.push(config::BotConfig {
        account: DETOUR_CHASE_FIXTURE_ACCOUNT.to_string(),
        password: String::new(),
        character_guid: DETOUR_CHASE_FIXTURE_CHARACTER_GUID,
        account_id: DETOUR_CHASE_FIXTURE_ACCOUNT_ID,
        lfg_role: 4,
        class: "priest".to_string(),
        enabled: true,
        session_key_bnet: String::new(),
    });
}
pub(crate) fn provision_local_bot_account_create_only(
    conn: &mut mysql::Conn,
    bot: &config::BotConfig,
    character_count: u64,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let (email, bnet_salt, bnet_verifier) =
        bot_srp6::bnet_v1_registration_material_like_cpp(&bot.account, &bot.password);
    let allow_nonlocal =
        std::env::var("WOW_BOT_ALLOW_NONLOCAL_ACCOUNT_BOOTSTRAP").is_ok_and(|v| is_truthy(&v));
    if !email.ends_with("@BOT.LOCAL") && !allow_nonlocal {
        bail!(
            "Refusing to bootstrap non-local bot account {email}; set WOW_BOT_ALLOW_NONLOCAL_ACCOUNT_BOOTSTRAP=1 if this is intentional"
        );
    }
    let game_username = game_account_username(&bot.account)?;
    let bnet_rows: Vec<(u32, String)> = conn
        .exec(
            "SELECT id, email FROM battlenet_accounts WHERE email = ? ORDER BY id",
            (&email,),
        )
        .map_err(|e| anyhow!("Lookup BNet account {email}: {e}"))?;
    if bnet_rows.len() > 1 {
        bail!(
            "Refusing account provisioning: BNet email {email} has {} rows",
            bnet_rows.len()
        );
    }
    let game_account_exists = conn
        .exec_first::<u32, _, _>("SELECT id FROM account WHERE id = ?", (bot.account_id,))
        .map_err(|e| anyhow!("Lookup game account id {}: {e}", bot.account_id))?
        .is_some();

    // Existing identities are validation-only. This makes repeated
    // provisioning idempotent while forbidding credential rewrites.
    match create_only_provisioning_plan(!bnet_rows.is_empty(), game_account_exists)? {
        CreateOnlyProvisioningPlan::ValidateExisting => {
            validate_exact_bot_identity(conn, None, bot)?;
            validate_realm_character_count(conn, bot, character_count)?;
            info!(
                "[Bot {}] existing local auth fixture validated without mutation",
                bot.account_id
            );
            return Ok(());
        }
        CreateOnlyProvisioningPlan::CreateBoth => {}
    }

    let colliding_accounts: u64 = conn
        .exec_first(
            "SELECT COUNT(*) FROM account WHERE username = ? OR email = ? OR reg_mail = ?",
            (&game_username, &email, &email),
        )
        .map_err(|e| anyhow!("Check game-account identity collisions: {e}"))?
        .unwrap_or(0);
    if !game_account_exists && colliding_accounts != 0 {
        bail!(
            "Refusing account provisioning: username/email for {} already belongs to another game account",
            bot.account
        );
    }

    let expected_numchars = u8::try_from(character_count).map_err(|_| {
        anyhow!("Character count {character_count} exceeds realmcharacters capacity")
    })?;
    let mut tx = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start create-only account transaction: {e}"))?;
    let bnet_id = if let Some((id, _)) = bnet_rows.first() {
        *id
    } else {
        tx.exec_drop(
            "INSERT INTO battlenet_accounts (email, srp_version, salt, verifier) VALUES (?, 1, ?, ?)",
            (&email, bnet_salt.to_vec(), bnet_verifier),
        )
        .map_err(|e| anyhow!("Insert BNet account {email}: {e}"))?;
        u32::try_from(
            tx.last_insert_id()
                .ok_or_else(|| anyhow!("BNet account insert returned no id"))?,
        )
        .map_err(|_| anyhow!("BNet account id overflow"))?
    };

    if !game_account_exists {
        // The 3.4.3 world login path authenticates through
        // account.session_key_bnet. Legacy Grunt fields remain NOT NULL.
        tx.exec_drop(
            "INSERT INTO account \
             (id, username, salt, verifier, reg_mail, email, joindate, battlenet_account, battlenet_index, expansion) \
             VALUES (?, ?, ?, ?, ?, ?, NOW(), ?, 1, 9)",
            (
                bot.account_id,
                &game_username,
                random_32().to_vec(),
                fixed_le_32(Vec::new()),
                &email,
                &email,
                bnet_id,
            ),
        )
        .map_err(|e| anyhow!("Insert game account {}: {e}", bot.account_id))?;
    }

    let existing_realm_count: Option<u8> = tx
        .exec_first(
            "SELECT numchars FROM realmcharacters WHERE acctid = ? AND realmid = ?",
            (bot.account_id, realm_id()),
        )
        .map_err(|e| anyhow!("Load realmcharacters for account {}: {e}", bot.account_id))?;
    match existing_realm_count {
        Some(actual) if actual != expected_numchars => bail!(
            "Refusing to rewrite realmcharacters for account {}: expected {}, found {}",
            bot.account_id,
            expected_numchars,
            actual
        ),
        Some(_) => {}
        None => tx
            .exec_drop(
                "INSERT INTO realmcharacters (numchars, acctid, realmid) VALUES (?, ?, ?)",
                (expected_numchars, bot.account_id, realm_id()),
            )
            .map_err(|e| anyhow!("Insert realmcharacters for account {}: {e}", bot.account_id))?,
    }
    tx.commit()
        .map_err(|e| anyhow!("Commit create-only account transaction: {e}"))?;
    validate_exact_bot_identity(conn, None, bot)?;
    validate_realm_character_count(conn, bot, character_count)?;
    info!(
        "[Bot {}] created missing local auth rows without rewriting existing identities",
        bot.account_id
    );
    Ok(())
}
pub(crate) fn validate_exact_bot_identity(
    auth_conn: &mut mysql::Conn,
    mut character_conn: Option<&mut mysql::Conn>,
    bot: &config::BotConfig,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let expected_email = bot_srp6::utf8_to_upper_only_latin_like_cpp(&bot.account);
    let expected_username = game_account_username(&bot.account)?;
    let bnet_rows: Vec<(u32, String, i8, Vec<u8>, Vec<u8>, u32, u8, u8)> = auth_conn
        .exec(
            "SELECT id, email, srp_version, salt, verifier, failed_logins, locked, online \
             FROM battlenet_accounts WHERE email = ? ORDER BY id",
            (&expected_email,),
        )
        .map_err(|e| anyhow!("Load exact BNet fixture {}: {e}", bot.account))?;
    if bnet_rows.len() != 1 {
        bail!(
            "Expected exactly one BNet row for {}, found {}",
            bot.account,
            bnet_rows.len()
        );
    }
    let (bnet_id, email, srp_version, salt, verifier, failed_logins, locked, bnet_online) =
        &bnet_rows[0];
    let (_, expected_verifier) =
        bot_srp6::bnet_v1_verifier_for_salt_like_cpp(&bot.account, &bot.password, salt);
    if !email.eq_ignore_ascii_case(&expected_email)
        || *srp_version != 1
        || salt.len() != 32
        || *verifier != expected_verifier
        || *failed_logins != 0
        || *locked != 0
        || *bnet_online != 0
    {
        bail!(
            "BNet fixture {} does not exactly match configured credentials/offline state",
            bot.account
        );
    }

    let account = auth_conn
        .exec_first::<(
            String,
            String,
            String,
            Option<u32>,
            Option<u8>,
            u8,
            u32,
            u8,
            u8,
        ), _, _>(
            "SELECT username, reg_mail, email, battlenet_account, battlenet_index, expansion, \
                    failed_logins, locked, online FROM account WHERE id = ?",
            (bot.account_id,),
        )
        .map_err(|e| anyhow!("Load exact game-account fixture {}: {e}", bot.account_id))?
        .ok_or_else(|| anyhow!("No game-account row for id {}", bot.account_id))?;
    if !account.0.eq_ignore_ascii_case(&expected_username)
        || !account.1.eq_ignore_ascii_case(&expected_email)
        || !account.2.eq_ignore_ascii_case(&expected_email)
        || account.3 != Some(*bnet_id)
        || account.4 != Some(1)
        || account.5 != 9
        || account.6 != 0
        || account.7 != 0
        || account.8 != 0
    {
        bail!(
            "Game account {} does not exactly match username/email/BNet/offline fixture contract",
            bot.account_id
        );
    }
    let game_accounts_on_bnet: u64 = auth_conn
        .exec_first(
            "SELECT COUNT(*) FROM account WHERE battlenet_account = ?",
            (*bnet_id,),
        )
        .map_err(|e| anyhow!("Count game accounts on BNet fixture: {e}"))?
        .unwrap_or(0);
    if game_accounts_on_bnet != 1 {
        bail!(
            "BNet fixture {} must own exactly one game account, found {game_accounts_on_bnet}",
            bot.account
        );
    }
    let bnet_bans: u64 = auth_conn
        .exec_first(
            "SELECT COUNT(*) FROM battlenet_account_bans WHERE id = ?",
            (*bnet_id,),
        )
        .map_err(|e| anyhow!("Check BNet fixture bans: {e}"))?
        .unwrap_or(0);
    let game_bans: u64 = auth_conn
        .exec_first(
            "SELECT COUNT(*) FROM account_banned WHERE id = ? AND active <> 0",
            (bot.account_id,),
        )
        .map_err(|e| anyhow!("Check game-account fixture bans: {e}"))?
        .unwrap_or(0);
    if bnet_bans != 0 || game_bans != 0 {
        bail!(
            "Configured bot fixture is banned (bnet rows={bnet_bans}, active game rows={game_bans}); provisioning will not clear bans"
        );
    }

    if let Some(char_conn) = character_conn.as_mut() {
        validate_local_bot_character_owner(char_conn, bot)?;
        let count: u64 = char_conn
            .exec_first(
                "SELECT COUNT(*) FROM characters WHERE account = ?",
                (bot.account_id,),
            )
            .map_err(|e| anyhow!("Count dedicated fixture characters: {e}"))?
            .unwrap_or(0);
        if count != 1 {
            bail!(
                "Loot fixture account {} must own exactly one character, found {count}",
                bot.account_id
            );
        }
        let realm_count: Option<u8> = auth_conn
            .exec_first(
                "SELECT numchars FROM realmcharacters WHERE acctid = ? AND realmid = ?",
                (bot.account_id, realm_id()),
            )
            .map_err(|e| anyhow!("Load realmcharacters fixture count: {e}"))?;
        if realm_count != Some(1) {
            bail!(
                "realmcharacters fixture count for account {} must be exactly 1, found {realm_count:?}",
                bot.account_id
            );
        }
    }
    Ok(())
}
/// Run a single bot through the full SRP6 → World → LFG flow
pub(crate) async fn run_bot(
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
) -> Result<BotRunResult> {
    run_bot_with_void_storage(
        bot,
        dungeon_id,
        lfg_secs,
        auto_teleport,
        login_only,
        stand_state_options,
        bank_options,
        homebind_options,
        inventory_swap_options,
        vendor_options,
        rested_xp_options,
        loot_race_options,
        group_capacity_options,
        equipment_set_options,
        quest_options,
        None,
        None,
        None,
        None,
    )
    .await
}
pub(crate) async fn run_bot_with_detour_chase(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    detour_chase_options: DetourChaseCaptureOptions,
) -> Result<BotRunResult> {
    let bot_for_preflight = bot.clone();
    let options_for_preflight = detour_chase_options.clone();
    tokio::task::spawn_blocking(move || {
        validate_detour_live_fixture_before_login(&bot_for_preflight, &options_for_preflight)
    })
    .await
    .map_err(|error| anyhow!("Detour fixture DB preflight worker failed: {error}"))??;
    info!("Pinned detour account/character/spawn fixture passed read-only DB preflight");

    run_bot_with_void_storage(
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
        None,
        None,
        None,
        None,
        None,
        None,
        Some(detour_chase_options),
        None,
        None,
    )
    .await
}
pub(crate) async fn run_stand_state_smoke(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &StandStateSmokeOptions,
    result: &mut BotRunResult,
) {
    match run_stand_state_smoke_inner(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        options,
        result,
    )
    .await
    {
        Ok(()) => {
            result.stand_state_smoke_passed = Some(true);
            info!(
                "[Bot {}] ✅ Stand-state smoke passed: {:?}",
                bot_index, result.stand_states_confirmed
            );
        }
        Err(error) => {
            result.stand_state_smoke_passed = Some(false);
            result.stand_state_failure = Some(error.to_string());
            warn!("[Bot {}] Stand-state smoke failed: {}", bot_index, error);
        }
    }
}
pub(crate) async fn run_stand_state_smoke_inner(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &StandStateSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    validate_stand_state_socket_topology(realm_connection.is_some())?;

    let _ = drain_connections_until_quiet_for_stand_state_smoke(
        bot_index,
        "login burst",
        STAND_STATE_LOGIN_QUIET_PERIOD,
        STAND_STATE_LOGIN_DRAIN_LIMIT,
        false,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        result,
    )
    .await?;

    let mut ack_side_effects = StandStateDrainSummary::default();
    for &expected_state in &options.states {
        let request = build_stand_state_change(expected_state);
        send_encrypted_packet(stream, crypt, CMSG_STAND_STATE_CHANGE, &request).await?;
        info!(
            "[Bot {}] ✅ CMSG_STAND_STATE_CHANGE sent on instance (state={})",
            bot_index, expected_state
        );

        let realm = realm_connection
            .as_mut()
            .context("stand-state realm socket disappeared after topology validation")?;
        let summary = wait_for_stand_state_update(
            bot_index,
            "realm",
            true,
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            options.timeout_secs,
            expected_state,
            result,
        )
        .await?;
        ack_side_effects.active_update_objects += summary.active_update_objects;
        ack_side_effects.active_aura_updates += summary.active_aura_updates;
    }

    // Keep both sockets alive until deferred UpdateObject/aura fanout has been
    // observed, then write a deterministic CMSG_PING fence. Capture-diff trims
    // at that CMSG, so the isolated action cannot end at the earlier realm ACK
    // and silently omit instance-side state deltas.
    let post_action = drain_connections_until_quiet_for_stand_state_smoke(
        bot_index,
        "stand-state side effects",
        STAND_STATE_POST_ACTION_QUIET_PERIOD,
        STAND_STATE_POST_ACTION_DRAIN_LIMIT,
        true,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        result,
    )
    .await?;
    info!(
        "[Bot {}] ✅ stand side effects on active connection: UpdateObject={}, AuraUpdate={}",
        bot_index,
        ack_side_effects.active_update_objects + post_action.active_update_objects,
        ack_side_effects.active_aura_updates + post_action.active_aura_updates
    );
    if options
        .states
        .iter()
        .any(|state| *state != UNIT_STAND_STATE_STAND)
        && ack_side_effects.active_update_objects + post_action.active_update_objects == 0
    {
        bail!(
            "changed stand-state smoke received no instance SMSG_UPDATE_OBJECT before capture fence"
        );
    }
    send_and_verify_stand_state_capture_fence(
        bot_index,
        stream,
        crypt,
        server_inflater,
        options.timeout_secs,
        result,
    )
    .await?;

    Ok(())
}
pub(crate) fn validate_stand_state_socket_topology(
    has_separate_realm_connection: bool,
) -> Result<()> {
    if !has_separate_realm_connection {
        bail!(
            "stand-state smoke requires SMSG_CONNECT_TO and distinct realm/instance sockets; single-socket login cannot validate opcode routing"
        );
    }
    Ok(())
}
pub(crate) async fn drain_connections_until_quiet_for_stand_state_smoke(
    bot_index: usize,
    phase: &str,
    quiet_period: Duration,
    drain_limit: Duration,
    enforce_instance_side_effect_routing: bool,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) -> Result<StandStateDrainSummary> {
    enum DrainReady {
        Active,
        Realm,
        Quiet,
    }

    let deadline = tokio::time::Instant::now() + drain_limit;
    let mut quiet_deadline = tokio::time::Instant::now() + quiet_period;
    let mut drained = 0usize;
    let mut summary = StandStateDrainSummary::default();

    loop {
        let now = tokio::time::Instant::now();
        let remaining = deadline.saturating_duration_since(now);
        if remaining.is_zero() {
            bail!(
                "{} did not become quiet within {} ms",
                phase,
                drain_limit.as_millis()
            );
        }
        let quiet_remaining = quiet_deadline.saturating_duration_since(now);
        if quiet_remaining.is_zero() {
            info!(
                "[Bot {}] ✅ {} drained across realm/instance: {} packet(s), {} ms relevant quiet",
                bot_index,
                phase,
                drained,
                quiet_period.as_millis()
            );
            return Ok(summary);
        }
        let quiet_wait = remaining.min(quiet_remaining);

        let quiet = tokio::time::sleep(quiet_wait);
        tokio::pin!(quiet);
        // `peek` waits for real bytes without consuming them. It is safe to
        // cancel when the other socket or quiet timer wins, unlike selecting
        // directly on the `read_exact`-based packet reader; it also avoids a
        // stale Tokio readiness bit causing a full read to wait past quiet.
        let mut active_peek = [0u8; 1];
        let mut realm_peek = [0u8; 1];
        let ready = if let Some(realm) = realm_connection.as_ref() {
            tokio::select! {
                result = stream.peek(&mut active_peek) => {
                    if result.with_context(|| format!("active {phase} drain peek failed"))? == 0 {
                        bail!("active connection closed during {phase} drain");
                    }
                    DrainReady::Active
                }
                result = realm.stream.peek(&mut realm_peek) => {
                    if result.with_context(|| format!("realm {phase} drain peek failed"))? == 0 {
                        bail!("realm connection closed during {phase} drain");
                    }
                    DrainReady::Realm
                }
                _ = &mut quiet => DrainReady::Quiet,
            }
        } else {
            tokio::select! {
                result = stream.peek(&mut active_peek) => {
                    if result.with_context(|| format!("primary realm {phase} drain peek failed"))? == 0 {
                        bail!("primary realm connection closed during {phase} drain");
                    }
                    DrainReady::Active
                }
                _ = &mut quiet => DrainReady::Quiet,
            }
        };

        match ready {
            DrainReady::Quiet => {
                if tokio::time::Instant::now() >= quiet_deadline {
                    info!(
                        "[Bot {}] ✅ {} drained across realm/instance: {} packet(s), {} ms relevant quiet",
                        bot_index,
                        phase,
                        drained,
                        quiet_period.as_millis()
                    );
                    return Ok(summary);
                }
                bail!(
                    "{} did not become quiet within {} ms",
                    phase,
                    drain_limit.as_millis()
                );
            }
            DrainReady::Active => {
                let read_remaining =
                    deadline.saturating_duration_since(tokio::time::Instant::now());
                let packet = tokio::time::timeout(
                    read_remaining,
                    read_encrypted_packet(stream, crypt, server_inflater),
                )
                .await
                .map_err(|_| anyhow!("active {phase} drain packet read timed out"))?;
                let (opcode, payload) = packet
                    .map_err(|error| anyhow!("active {phase} drain packet read failed: {error}"))?;
                drained += 1;
                if !stand_state_quiet_drain_ambient_opcode(opcode) {
                    quiet_deadline = tokio::time::Instant::now() + quiet_period;
                }
                if enforce_instance_side_effect_routing {
                    match opcode {
                        SMSG_UPDATE_OBJECT => summary.active_update_objects += 1,
                        SMSG_AURA_UPDATE => summary.active_aura_updates += 1,
                        _ => {}
                    }
                }
                result.seen_opcodes.push(format!("0x{:04X}", opcode));
                let connection_name = if realm_connection.is_some() {
                    "instance"
                } else {
                    "primary realm"
                };
                info!(
                    "[Bot {}] 📦 {} {} drain {}",
                    bot_index,
                    connection_name,
                    phase,
                    parse_packet(opcode, &payload)
                );
            }
            DrainReady::Realm => {
                let read_remaining =
                    deadline.saturating_duration_since(tokio::time::Instant::now());
                let realm = realm_connection.as_mut().with_context(|| {
                    format!("realm connection disappeared during {phase} drain")
                })?;
                let packet = tokio::time::timeout(
                    read_remaining,
                    read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater),
                )
                .await
                .map_err(|_| anyhow!("realm {phase} drain packet read timed out"))?;
                let (opcode, payload) = packet
                    .map_err(|error| anyhow!("realm {phase} drain packet read failed: {error}"))?;
                drained += 1;
                if !stand_state_quiet_drain_ambient_opcode(opcode) {
                    quiet_deadline = tokio::time::Instant::now() + quiet_period;
                }
                if enforce_instance_side_effect_routing
                    && matches!(opcode, SMSG_UPDATE_OBJECT | SMSG_AURA_UPDATE)
                {
                    bail!(
                        "{} arrived on realm during {} drain; C++ routes stand side effects on instance",
                        parse_packet(opcode, &payload),
                        phase
                    );
                }
                result.seen_opcodes.push(format!("0x{:04X}", opcode));
                info!(
                    "[Bot {}] 📦 realm {} drain {}",
                    bot_index,
                    phase,
                    parse_packet(opcode, &payload)
                );
            }
        }
    }
}
pub(crate) async fn run_rested_xp_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    creature_entry: u32,
    creature_spawn_guid: Option<u64>,
    runtime_counter: Option<u64>,
    offline_secs: u64,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_rested_xp_smoke_fixture(
            &bot_for_setup,
            creature_entry,
            creature_spawn_guid,
            runtime_counter,
            offline_secs,
            timeout_secs,
        )
    })
    .await
    .map_err(|error| anyhow!("Rested-XP fixture setup worker failed: {error}"))??;

    let workflow = run_rested_xp_smoke_workflow_inner(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        &fixture,
    )
    .await;

    // A successful XP packet proves this workflow killed the selected target.
    // On earlier protocol/discovery failures no respawn transition is expected;
    // skipping that wait avoids masking the real error. If an ambiguous failed
    // attack did create a timer, the next preflight still rejects it safely.
    let verify_target_respawn = workflow
        .as_ref()
        .is_ok_and(|result| result.rested_xp_packet_amount.is_some());

    // Cleanup is deliberately outside the workflow result so every login,
    // protocol, assertion, and DB error path attempts the bounded selected-field
    // restore after the server has completed its disconnect save.
    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_rested_xp_smoke_fixture(
            &bot_for_cleanup,
            &fixture_for_cleanup,
            verify_target_respawn,
        )
    })
    .await
    .map_err(|error| anyhow!("Rested-XP cleanup worker failed: {error}"))?;

    match (workflow, cleanup) {
        (Ok(result), Ok(())) => Ok(result),
        (Ok(mut result), Err(error)) => {
            result.rested_xp_smoke_passed = Some(false);
            result.rested_xp_failure = Some(format!("fixture cleanup failed: {error}"));
            Ok(result)
        }
        (Err(error), Ok(())) => Err(error),
        (Err(workflow_error), Err(cleanup_error)) => Err(anyhow!(
            "Rested-XP workflow failed: {workflow_error}; fixture cleanup failed: {cleanup_error}"
        )),
    }
}

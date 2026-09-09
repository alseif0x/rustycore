//! Spell operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn validate_creature_spell_capture_cli_values(
    enabled: bool,
    single_account: Option<&str>,
    timeout_secs: u64,
    manifest: Option<&str>,
) -> Result<()> {
    if !enabled {
        return Ok(());
    }
    if !single_account
        .is_some_and(|account| account.eq_ignore_ascii_case(CREATURE_SPELL_FIXTURE_ACCOUNT))
    {
        bail!("--creature-spell-capture requires --single {CREATURE_SPELL_FIXTURE_ACCOUNT}");
    }
    if timeout_secs == 0 {
        bail!("--creature-spell-timeout must be greater than zero");
    }
    if !manifest.is_some_and(|path| !path.trim().is_empty()) {
        bail!("--creature-spell-capture requires --creature-spell-fixture-manifest <path>");
    }
    Ok(())
}
pub(crate) fn validate_creature_spell_fixture_manifest(path: &Path) -> Result<String> {
    let metadata = std::fs::symlink_metadata(path).with_context(|| {
        format!(
            "Read creature spell fixture manifest metadata {}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "Creature spell fixture manifest must be a regular file and not a symlink: {}",
            path.display()
        );
    }
    let canonical = path.canonicalize().with_context(|| {
        format!(
            "Canonicalize creature spell fixture manifest {}",
            path.display()
        )
    })?;
    let bytes = std::fs::read(&canonical).with_context(|| {
        format!(
            "Read creature spell fixture manifest {}",
            canonical.display()
        )
    })?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != CREATURE_SPELL_FIXTURE_MANIFEST_SHA256 {
        bail!(
            "Creature spell fixture manifest SHA-256 mismatch: got {digest}, expected {CREATURE_SPELL_FIXTURE_MANIFEST_SHA256}"
        );
    }
    let manifest: serde_json::Value = serde_json::from_slice(&bytes).with_context(|| {
        format!(
            "Decode creature spell fixture manifest {}",
            canonical.display()
        )
    })?;
    let expected_difficulty = serde_json::json!({
        "entry": 22_378,
        "difficulty_id": 0,
        "min_level": 64,
        "max_level": 65,
        "health_scaling_expansion": 0,
        "health_modifier": 1.0,
        "mana_modifier": 1.0,
        "armor_modifier": 1.0,
        "damage_modifier": 1.0,
        "creature_difficulty_id": 18_203,
        "type_flags": 0,
        "type_flags_2": 0,
        "loot_id": 22_378,
        "pickpocket_loot_id": 22_378,
        "skin_loot_id": 0,
        "gold_min": 153,
        "gold_max": 205,
        "original_static_flags_1": 0,
        "temporary_static_flags_1": 0x0010_0000,
        "static_flags_2": 0,
        "static_flags_3": 0,
        "static_flags_4": 0,
        "static_flags_5": 0,
        "static_flags_6": 0,
        "static_flags_7": 0,
        "static_flags_8": 0
    });
    if manifest
        .get("schema_version")
        .and_then(|value| value.as_u64())
        != Some(2)
        || manifest.get("flow").and_then(|value| value.as_str())
            != Some(CREATURE_SPELL_FIXTURE_FLOW)
        || manifest.get("contract").and_then(|value| value.as_str())
            != Some(CREATURE_SPELL_FIXTURE_CONTRACT)
        || manifest
            .pointer("/creature_template/entry")
            .and_then(|value| value.as_u64())
            != Some(u64::from(CREATURE_SPELL_FIXTURE_ENTRY))
        || manifest
            .pointer("/creature_template/original_ai_name")
            .and_then(|value| value.as_str())
            != Some("SmartAI")
        || manifest
            .pointer("/creature_template/temporary_ai_name")
            .and_then(|value| value.as_str())
            != Some("CombatAI")
        || manifest.get("creature_template_difficulty") != Some(&expected_difficulty)
        || manifest
            .pointer("/spawn/guid")
            .and_then(|value| value.as_u64())
            != Some(CREATURE_SPELL_FIXTURE_SPAWN_GUID)
        || manifest
            .pointer("/template_spell/spell_id")
            .and_then(|value| value.as_u64())
            != Some(u64::from(CREATURE_SPELL_FIXTURE_SPELL_ID))
        || manifest
            .pointer("/spell_shape/required_go_hit_targets")
            .and_then(|value| value.as_u64())
            != Some(1)
        || manifest
            .pointer("/spell_shape/required_go_miss_targets")
            .and_then(|value| value.as_u64())
            != Some(0)
    {
        bail!(
            "Creature spell fixture manifest differs from the pinned issue #26 contract: {}",
            canonical.display()
        );
    }
    Ok(digest)
}
pub(crate) fn validate_creature_spell_no_persisted_ghost_state(
    ghost_aura_count: u64,
    ghost_effect_count: u64,
) -> Result<()> {
    if ghost_aura_count != 0 || ghost_effect_count != 0 {
        bail!(
            "creature spell fixture character {} has persisted ghost spell {} state (auras={ghost_aura_count}, effects={ghost_effect_count})",
            CREATURE_SPELL_FIXTURE_CHARACTER_GUID,
            CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID
        );
    }
    Ok(())
}
pub(crate) fn add_pinned_creature_spell_fixture_bot_if_missing(
    bots: &mut Vec<config::BotConfig>,
    creature_spell_capture: bool,
) {
    if !creature_spell_capture || !bots.is_empty() {
        return;
    }
    bots.push(config::BotConfig {
        account: CREATURE_SPELL_FIXTURE_ACCOUNT.to_string(),
        password: String::new(),
        character_guid: CREATURE_SPELL_FIXTURE_CHARACTER_GUID,
        account_id: CREATURE_SPELL_FIXTURE_ACCOUNT_ID,
        lfg_role: 4,
        class: "paladin".to_string(),
        enabled: true,
        session_key_bnet: String::new(),
    });
}
pub(crate) async fn run_bot_with_creature_spell_capture(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    creature_spell_options: CreatureSpellCaptureOptions,
) -> Result<BotRunResult> {
    let bot_for_preflight = bot.clone();
    let options_for_preflight = creature_spell_options.clone();
    tokio::task::spawn_blocking(move || {
        validate_creature_spell_live_fixture_before_login(
            &bot_for_preflight,
            &options_for_preflight,
        )
    })
    .await
    .map_err(|error| anyhow!("Creature spell fixture DB preflight worker failed: {error}"))??;
    info!("Pinned creature spell account/character/spawn fixture passed read-only DB preflight");

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
        None,
        Some(creature_spell_options),
        None,
    )
    .await
}
pub(crate) fn creature_spell_failure_result(
    bot: &config::BotConfig,
    dungeon_id: u32,
    options: &CreatureSpellCaptureOptions,
    failure: String,
) -> BotRunResult {
    BotRunResult {
        account: bot.account.clone(),
        account_id: bot.account_id,
        character_guid: bot.character_guid,
        dungeon_id,
        role: bot.lfg_role,
        creature_spell_capture: true,
        creature_spell_capture_passed: Some(false),
        creature_spell_fixture_manifest_sha256: Some(options.fixture_manifest_sha256.clone()),
        creature_spell_target_entry: Some(CREATURE_SPELL_FIXTURE_ENTRY),
        creature_spell_target_spawn_guid: Some(CREATURE_SPELL_FIXTURE_SPAWN_GUID),
        creature_spell_failure: Some(failure),
        ..BotRunResult::default()
    }
}
pub(crate) async fn run_creature_spell_capture_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &CreatureSpellCaptureOptions,
    login_target: Option<DiscoveredCreatureGuid>,
    result: &mut BotRunResult,
) -> Result<()> {
    if realm_connection.is_none() {
        bail!("creature spell capture requires distinct authenticated realm/instance sockets");
    }
    if !bot
        .account
        .eq_ignore_ascii_case(CREATURE_SPELL_FIXTURE_ACCOUNT)
        || bot.account_id != CREATURE_SPELL_FIXTURE_ACCOUNT_ID
        || bot.character_guid != CREATURE_SPELL_FIXTURE_CHARACTER_GUID
        || options.fixture_manifest_sha256 != CREATURE_SPELL_FIXTURE_MANIFEST_SHA256
    {
        bail!("creature spell capture lost its pinned account/character/manifest identity");
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs.max(1));
    let clock_origin = tokio::time::Instant::now();
    let active_mover_complete = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
        &active_mover_complete,
    )
    .await?;

    let mut target = login_target;
    while target.is_none() && tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out discovering creature spell fixture target"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        match opcode {
            SMSG_TIME_SYNC_REQUEST => {
                respond_to_detour_time_sync_like_cpp(
                    bot_index,
                    stream,
                    crypt,
                    &payload,
                    clock_origin,
                    "creature-spell-discovery",
                )
                .await?;
            }
            SMSG_SPELL_START => {
                bail!(
                    "creature began casting before the bot's body-pull heartbeat; the pre-login position is not isolated"
                );
            }
            SMSG_UPDATE_OBJECT => {
                let candidates = find_creature_guids_near_position_in_update_object(
                    &payload,
                    CREATURE_SPELL_FIXTURE_MAP_ID,
                    CREATURE_SPELL_FIXTURE_ENTRY,
                    CREATURE_SPELL_FIXTURE_X,
                    CREATURE_SPELL_FIXTURE_Y,
                    CREATURE_SPELL_FIXTURE_Z,
                    CREATURE_SPELL_TARGET_MATCH_RADIUS,
                    None,
                );
                if candidates.len() > 1 {
                    bail!(
                        "creature spell discovery found {} live candidates inside the pinned spawn radius",
                        candidates.len()
                    );
                }
                target = candidates.into_iter().next();
            }
            _ => {}
        }
    }
    let target = target.ok_or_else(|| {
        anyhow!(
            "did not discover creature spell fixture entry={} spawn={} near ({:.3},{:.3},{:.3})",
            CREATURE_SPELL_FIXTURE_ENTRY,
            CREATURE_SPELL_FIXTURE_SPAWN_GUID,
            CREATURE_SPELL_FIXTURE_X,
            CREATURE_SPELL_FIXTURE_Y,
            CREATURE_SPELL_FIXTURE_Z
        )
    })?;
    result.creature_spell_target_runtime_counter = Some(target.low);
    result.creature_spell_target_discovered = true;
    let target_guid = (target.low, target.high);
    let player_guid = create_player_guid_raw(bot.character_guid, realm_id());

    let heartbeat = build_move_heartbeat_payload(
        player_guid.0,
        player_guid.1,
        CREATURE_SPELL_CHARACTER_PULL_X,
        CREATURE_SPELL_FIXTURE_Y,
        CREATURE_SPELL_FIXTURE_Z,
        CREATURE_SPELL_CHARACTER_ORIENTATION,
    );
    send_encrypted_packet(stream, crypt, CMSG_MOVE_HEARTBEAT, &heartbeat).await?;
    result.creature_spell_heartbeat_sent = true;
    result.creature_spell_heartbeat_sha256 = Some(format!("{:x}", Sha256::digest(&heartbeat)));
    info!(
        "[Bot {}] ✅ creature spell body-pull heartbeat sent without CMSG_ATTACK_SWING; player faces away from spawn",
        bot_index
    );

    let start_summary = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for fixture SMSG_SPELL_START 15691");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for fixture SMSG_SPELL_START 15691"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_TIME_SYNC_REQUEST {
            respond_to_detour_time_sync_like_cpp(
                bot_index,
                stream,
                crypt,
                &payload,
                clock_origin,
                "creature-spell-start-wait",
            )
            .await?;
            continue;
        }
        if opcode != SMSG_SPELL_START {
            continue;
        }
        let summary = parse_creature_spell_cast_data(&payload, false)
            .context("malformed creature spell SMSG_SPELL_START")?;
        if summary.caster != target_guid || summary.spell_id != CREATURE_SPELL_FIXTURE_SPELL_ID {
            continue;
        }
        if summary.caster_unit != target_guid
            || summary.cast_id == (0, 0)
            || summary.original_cast_id != (0, 0)
            || summary.target != player_guid
            || summary.cast_flags != 0x0000_0002
            || summary.cast_flags_ex != 0
            || !summary.hit_targets.is_empty()
            || !summary.miss_targets.is_empty()
            || summary.miss_status_count != 0
            || summary.full_combat_log.is_some()
        {
            bail!("fixture SMSG_SPELL_START differs from the exact basic cast contract");
        }
        result.creature_spell_start_opcode = Some(SMSG_SPELL_START);
        result.creature_spell_start_body_sha256 = Some(format!("{:x}", Sha256::digest(&payload)));
        result.creature_spell_start_body_bytes = Some(payload.len());
        result.creature_spell_cast_id_low = Some(summary.cast_id.0);
        result.creature_spell_cast_id_high = Some(summary.cast_id.1);
        result.creature_spell_caster_guid_low = Some(summary.caster.0);
        result.creature_spell_caster_guid_high = Some(summary.caster.1);
        result.creature_spell_victim_guid_low = Some(summary.target.0);
        result.creature_spell_victim_guid_high = Some(summary.target.1);
        result.creature_spell_spell_id = Some(summary.spell_id);
        result.creature_spell_start_cast_flags = Some(summary.cast_flags);
        result.creature_spell_cast_flags_ex = Some(summary.cast_flags_ex);
        break summary;
    };

    // The flow has no ignored S2C opcodes. The very next instance packet must
    // be the GO for the same CastID; this binds the bot report to the exact RAW
    // pair selected by capture-diff.
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    let (go_opcode, go_payload) = tokio::time::timeout(
        remaining,
        read_encrypted_packet(stream, crypt, server_inflater),
    )
    .await
    .map_err(|_| anyhow!("timed out waiting for adjacent fixture SMSG_SPELL_GO"))??;
    result.seen_opcodes.push(format!("0x{go_opcode:04X}"));
    if go_opcode != SMSG_SPELL_GO {
        bail!("fixture START was followed by opcode 0x{go_opcode:04X}, not adjacent SMSG_SPELL_GO");
    }
    let go_summary = parse_creature_spell_cast_data(&go_payload, true)
        .context("malformed creature spell SMSG_SPELL_GO")?;
    if go_summary.caster != target_guid
        || go_summary.caster_unit != target_guid
        || go_summary.cast_id != start_summary.cast_id
        || go_summary.original_cast_id != (0, 0)
        || go_summary.spell_id != CREATURE_SPELL_FIXTURE_SPELL_ID
        || go_summary.target != player_guid
        || go_summary.cast_flags != 0x0000_0100
        || go_summary.cast_flags_ex != 0
        || go_summary.hit_targets != [player_guid]
        || !go_summary.miss_targets.is_empty()
        || go_summary.miss_status_count != 0
        || go_summary.full_combat_log != Some(false)
    {
        bail!(
            "fixture SMSG_SPELL_GO missed/avoided or differs from the exact basic hit contract; discard and repeat the live attempt"
        );
    }
    result.creature_spell_go_opcode = Some(SMSG_SPELL_GO);
    result.creature_spell_go_body_sha256 = Some(format!("{:x}", Sha256::digest(&go_payload)));
    result.creature_spell_go_body_bytes = Some(go_payload.len());
    result.creature_spell_go_cast_flags = Some(go_summary.cast_flags);
    result.creature_spell_go_hit_target_count =
        Some(u16::try_from(go_summary.hit_targets.len()).unwrap_or(u16::MAX));
    result.creature_spell_go_miss_target_count =
        Some(u16::try_from(go_summary.miss_targets.len()).unwrap_or(u16::MAX));
    result.creature_spell_full_combat_log = go_summary.full_combat_log;
    result.creature_spell_adjacent_start_go = true;
    Ok(())
}
pub(crate) async fn disconnect_creature_spell_sockets(
    bot_index: usize,
    instance_stream: &mut TcpStream,
    realm_connection: &mut Option<EncryptedWorldConnection>,
) -> Result<()> {
    let mut shutdown_errors = Vec::new();
    if let Err(error) = instance_stream.shutdown().await {
        shutdown_errors.push(format!("instance: {error}"));
    }
    match realm_connection.take() {
        Some(mut realm) => {
            if let Err(error) = realm.stream.shutdown().await {
                shutdown_errors.push(format!("realm: {error}"));
            }
        }
        None => shutdown_errors.push("realm: authenticated socket was already absent".to_string()),
    }
    if !shutdown_errors.is_empty() {
        bail!(
            "creature-spell best-effort socket shutdown failed: {}",
            shutdown_errors.join("; ")
        );
    }
    info!(
        "[Bot {}] ✅ creature-spell instance/realm socket shutdowns confirmed without CMSG_LOGOUT_REQUEST",
        bot_index
    );
    Ok(())
}
pub(crate) fn parse_creature_spell_cast_data(
    payload: &[u8],
    is_spell_go: bool,
) -> Result<CreatureSpellCastSummary> {
    let mut position = 0usize;
    let caster = take_packed_guid(payload, &mut position)
        .context("creature spell cast omitted CasterGUID")?;
    let caster_unit = take_packed_guid(payload, &mut position)
        .context("creature spell cast omitted CasterUnit")?;
    let cast_id =
        take_packed_guid(payload, &mut position).context("creature spell cast omitted CastID")?;
    let original_cast_id = take_packed_guid(payload, &mut position)
        .context("creature spell cast omitted OriginalCastID")?;
    let spell_id =
        take_u32(payload, &mut position).context("creature spell cast omitted SpellID")?;
    let visual =
        take_u32(payload, &mut position).context("creature spell cast omitted SpellCastVisual")?;
    let cast_flags =
        take_u32(payload, &mut position).context("creature spell cast omitted CastFlags")?;
    let cast_flags_ex =
        take_u32(payload, &mut position).context("creature spell cast omitted CastFlagsEx")?;
    let cast_time =
        take_u32(payload, &mut position).context("creature spell cast omitted CastTime")?;
    let trajectory_time =
        take_u32(payload, &mut position).context("creature spell cast omitted trajectory time")?;
    let trajectory_pitch =
        take_u32(payload, &mut position).context("creature spell cast omitted trajectory pitch")?;
    let destination_index =
        take_u8(payload, &mut position).context("creature spell cast omitted destination index")?;
    let immunity_school =
        take_u32(payload, &mut position).context("creature spell cast omitted immunity school")?;
    let immunity_value =
        take_u32(payload, &mut position).context("creature spell cast omitted immunity value")?;
    let prediction_points = take_u32(payload, &mut position)
        .context("creature spell cast omitted heal prediction points")?;
    let prediction_type = take_u8(payload, &mut position)
        .context("creature spell cast omitted heal prediction type")?;
    let prediction_beacon = take_packed_guid(payload, &mut position)
        .context("creature spell cast omitted prediction beacon")?;
    if visual != CREATURE_SPELL_FIXTURE_SPELL_X_VISUAL_ID
        || (!is_spell_go && cast_time != 0)
        || trajectory_time != 0
        || trajectory_pitch != 0
        || destination_index != 0
        || immunity_school != 0
        || immunity_value != 0
        || prediction_points != 0
        || prediction_type != 0
        || prediction_beacon != (0, 0)
    {
        bail!("creature spell cast contains an unexpected fixed optional payload");
    }

    let counts = payload
        .get(position..position.saturating_add(10))
        .context("creature spell cast omitted vector counts")?;
    let hit_count = read_msb_bits(counts, 0, 16).context("invalid hit target count")? as u16;
    let miss_count = read_msb_bits(counts, 16, 16).context("invalid miss target count")? as u16;
    let miss_status_count =
        read_msb_bits(counts, 32, 16).context("invalid miss status count")? as u16;
    let remaining_power = read_msb_bits(counts, 48, 9).context("invalid remaining-power count")?;
    let has_runes = read_msb_bits(counts, 57, 1).context("invalid rune-present bit")?;
    let target_points = read_msb_bits(counts, 58, 16).context("invalid target-point count")?;
    let has_ammo_display = read_msb_bits(counts, 74, 1).context("invalid ammo-display bit")?;
    let has_ammo_inventory = read_msb_bits(counts, 75, 1).context("invalid ammo-inventory bit")?;
    let count_padding = read_msb_bits(counts, 76, 4).context("invalid vector-count padding")?;
    if remaining_power != 0
        || has_runes != 0
        || target_points != 0
        || has_ammo_display != 0
        || has_ammo_inventory != 0
        || count_padding != 0
    {
        bail!("creature spell cast contains an unexpected vector optional");
    }
    position += 10;

    let target_header = payload
        .get(position..position.saturating_add(5))
        .context("creature spell cast omitted target header")?;
    let target_flags =
        read_msb_bits(target_header, 0, 28).context("invalid creature spell target flags")?;
    let target_optionals =
        read_msb_bits(target_header, 28, 4).context("invalid target optional bits")?;
    let target_name_length =
        read_msb_bits(target_header, 32, 7).context("invalid target name length")?;
    let target_padding =
        read_msb_bits(target_header, 39, 1).context("invalid target-header padding")?;
    if target_flags != 0x2
        || target_optionals != 0
        || target_name_length != 0
        || target_padding != 0
    {
        bail!("creature spell cast target is not the exact unit-only target shape");
    }
    position += 5;
    let target = take_packed_guid(payload, &mut position)
        .context("creature spell cast omitted unit target")?;
    let item = take_packed_guid(payload, &mut position)
        .context("creature spell cast omitted item target")?;
    if item != (0, 0) {
        bail!("creature spell cast unexpectedly targets an item");
    }

    let mut hit_targets = Vec::with_capacity(usize::from(hit_count));
    for _ in 0..hit_count {
        hit_targets.push(
            take_packed_guid(payload, &mut position)
                .context("creature spell cast truncated hit target")?,
        );
    }
    let mut miss_targets = Vec::with_capacity(usize::from(miss_count));
    for _ in 0..miss_count {
        miss_targets.push(
            take_packed_guid(payload, &mut position)
                .context("creature spell cast truncated miss target")?,
        );
    }
    // Success evidence requires no misses. Refuse to guess variable-length
    // SpellMissStatus records in a failed attempt; the operator must repeat it.
    if miss_status_count != 0 {
        bail!(
            "creature spell cast contains {miss_status_count} miss status record(s); discard and repeat the live attempt"
        );
    }

    let full_combat_log = if is_spell_go {
        let byte = take_u8(payload, &mut position)
            .context("creature spell GO omitted FullCombatLog bit")?;
        if byte != 0 {
            bail!("creature spell GO enabled FullCombatLog or has trailing log data");
        }
        Some(false)
    } else {
        None
    };
    if position != payload.len() {
        bail!(
            "creature spell cast has {} unexpected trailing byte(s)",
            payload.len().saturating_sub(position)
        );
    }

    Ok(CreatureSpellCastSummary {
        caster,
        caster_unit,
        cast_id,
        original_cast_id,
        spell_id,
        cast_flags,
        cast_flags_ex,
        target,
        hit_targets,
        miss_targets,
        miss_status_count,
        full_combat_log,
        consumed: position,
    })
}
pub(crate) fn spell_go_matches_bind(
    payload: &[u8],
    expected_caster_low: u64,
    expected_caster_high: u64,
    expected_player_low: u64,
    expected_player_high: u64,
) -> bool {
    let mut position = 0usize;
    let Some(caster) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    let Some(caster_unit) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    if caster != (expected_caster_low, expected_caster_high) || caster_unit != caster {
        return false;
    }
    if take_packed_guid(payload, &mut position).is_none()
        || take_packed_guid(payload, &mut position).is_none()
        || take_u32(payload, &mut position) != Some(3286)
    {
        return false;
    }

    // Visual, CastFlags, CastFlagsEx, CastTime, trajectory, destination index,
    // immunities, heal prediction and empty prediction beacon GUID.
    if take_u32(payload, &mut position).is_none()
        || take_u32(payload, &mut position) != Some(0x0004_0101)
        || take_u32(payload, &mut position) != Some(0)
        || take_u32(payload, &mut position).is_none()
    {
        return false;
    }
    position = match position.checked_add(8 + 1 + 8 + 4 + 1) {
        Some(end) if end <= payload.len() => end,
        _ => return false,
    };
    let Some(beacon) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    if beacon != (0, 0) {
        return false;
    }

    let Some(counts) = payload.get(position..position.saturating_add(10)) else {
        return false;
    };
    if read_msb_bits(counts, 0, 16) != Some(1)
        || read_msb_bits(counts, 16, 16) != Some(0)
        || read_msb_bits(counts, 32, 16) != Some(0)
        || read_msb_bits(counts, 48, 9) != Some(0)
        || read_msb_bits(counts, 57, 1) != Some(0)
        || read_msb_bits(counts, 58, 16) != Some(0)
        || read_msb_bits(counts, 74, 1) != Some(0)
        || read_msb_bits(counts, 75, 1) != Some(0)
    {
        return false;
    }
    position += 10;

    let Some(target_header) = payload.get(position..position.saturating_add(5)) else {
        return false;
    };
    if read_msb_bits(target_header, 0, 28) != Some(0x2)
        || read_msb_bits(target_header, 28, 4) != Some(0)
        || read_msb_bits(target_header, 32, 7) != Some(0)
    {
        return false;
    }
    position += 5;
    let Some(target) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    let Some(item) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    let Some(hit_target) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    target == (expected_player_low, expected_player_high) && hit_target == target && item == (0, 0)
}

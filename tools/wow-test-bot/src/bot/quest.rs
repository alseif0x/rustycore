//! Quest operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn parse_quest_objective_rows(value: &str) -> Result<Vec<QuestObjectiveDbRow>> {
    let mut rows = Vec::new();
    for raw in value.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (objective, data) = raw
            .split_once(':')
            .or_else(|| raw.split_once('='))
            .ok_or_else(|| anyhow!("Quest objective row `{raw}` must use storage:data"))?;
        rows.push(QuestObjectiveDbRow {
            objective: objective
                .trim()
                .parse()
                .map_err(|e| anyhow!("Invalid objective storage index `{objective}`: {e}"))?,
            data: data
                .trim()
                .parse()
                .map_err(|e| anyhow!("Invalid objective data `{data}`: {e}"))?,
        });
    }
    if rows.is_empty() {
        bail!("Quest objective rows cannot be empty");
    }
    rows.sort();
    rows.dedup_by_key(|row| row.objective);
    Ok(rows)
}
pub(crate) fn quest_smoke_options_from_cli(cli: &CliOptions) -> Result<QuestSmokeOptions> {
    let creature_entry = cli.quest_creature_entry.ok_or_else(|| {
        anyhow!("--quest-smoke requires --quest-creature-entry or WOW_BOT_QUEST_CREATURE_ENTRY")
    })?;
    if (cli.quest_accept || cli.quest_reset) && cli.quest_expected_id.is_none() {
        bail!("--quest-accept/--quest-reset require --expect-quest or WOW_BOT_QUEST_EXPECT_ID");
    }
    if cli.quest_objective_persist {
        if cli.quest_expected_id.is_none() {
            bail!("--quest-objective-persist requires --expect-quest or WOW_BOT_QUEST_EXPECT_ID");
        }
        if cli.quest_objectives.is_empty() {
            bail!("--quest-objective-persist requires --quest-objectives storage:data");
        }
    }
    if let Some(level) = cli.quest_set_level {
        if !(1..=80).contains(&level) {
            bail!("--quest-set-level must be in the 1..=80 player level range");
        }
    }
    if matches!(cli.quest_set_race, Some(0)) {
        bail!("--quest-set-race must be nonzero");
    }
    if matches!(cli.quest_set_class, Some(0)) {
        bail!("--quest-set-class must be nonzero");
    }

    Ok(QuestSmokeOptions {
        creature_entry,
        creature_spawn_guid: cli.quest_creature_guid,
        creature_guid_counter: cli.quest_guid_counter,
        map_id: cli.quest_map_id,
        expected_quest_id: cli.quest_expected_id,
        forbidden_quest_id: cli.quest_forbidden_id,
        forbidden_title_contains: cli.quest_forbidden_title.clone(),
        query_details: cli.quest_query_details || cli.quest_accept,
        accept: cli.quest_accept,
        reset_before_run: cli.quest_reset,
        relocate_before_login: cli.quest_relocate,
        set_level_before_login: cli.quest_set_level,
        set_race_before_login: cli.quest_set_race,
        set_class_before_login: cli.quest_set_class,
        objective_persist: cli.quest_objective_persist,
        objective_seed: cli.quest_objectives.clone(),
        objective_status: cli.quest_objective_status,
        gossip_select_option_id: cli.gossip_select_option_id,
        expect_trainer_list: cli.expect_trainer_list,
        expect_trainer_id: cli.expect_trainer_id,
        timeout_secs: cli.quest_timeout_secs,
    })
}
pub(crate) fn parse_time_sync_request_sequence(payload: &[u8]) -> Result<u32> {
    let bytes: [u8; 4] = payload
        .try_into()
        .map_err(|_| anyhow!("SMSG_TIME_SYNC_REQUEST payload must contain exactly 4 bytes"))?;
    Ok(u32::from_le_bytes(bytes))
}
pub(crate) async fn run_quest_smoke(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    quest_options: &QuestSmokeOptions,
    result: &mut BotRunResult,
) {
    match run_quest_smoke_inner(
        bot_index,
        bot,
        stream,
        crypt,
        server_inflater,
        quest_options,
        result,
    )
    .await
    {
        Ok(()) => {
            let pass = quest_smoke_passes(quest_options, result);
            result.quest_smoke_passed = Some(pass);
            if pass {
                info!(
                    "[Bot {}] ✅ Quest smoke passed: ids={:?} titles={:?}",
                    bot_index, result.quest_ids_seen, result.quest_titles_seen
                );
            } else if result.quest_failure.is_none() {
                result.quest_failure = Some("Quest response expectations were not met".to_string());
            }
        }
        Err(e) => {
            result.quest_smoke_passed = Some(false);
            result.quest_failure = Some(e.to_string());
            warn!("[Bot {}] Quest smoke failed: {}", bot_index, e);
        }
    }
}
pub(crate) fn sorted_quest_objective_rows(
    mut rows: Vec<QuestObjectiveDbRow>,
) -> Vec<QuestObjectiveDbRow> {
    rows.sort();
    rows
}
pub(crate) async fn logout_and_verify_quest_objectives(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    quest_options: &QuestSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let quest_id = quest_options
        .expected_quest_id
        .ok_or_else(|| anyhow!("Objective persistence requested without quest id"))?;

    send_encrypted_packet(stream, crypt, 0x34D6, &[0]).await?;
    info!("[Bot {}] ✅ CMSG_LOGOUT_REQUEST sent", bot_index);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                let parsed = parse_packet(op, &payload);
                info!("[Bot {}] 📦 {}", bot_index, parsed);
                if op == 0x2684 {
                    info!("[Bot {}] ✅ SMSG_LOGOUT_COMPLETE received", bot_index);
                    break;
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Logout read error: {}", bot_index, e);
                break;
            }
            Err(_) => break,
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    let bot_for_db = bot.clone();
    let after =
        tokio::task::spawn_blocking(move || load_bot_quest_objectives(&bot_for_db, quest_id))
            .await
            .map_err(|e| anyhow!("Quest objective after-load worker join failed: {}", e))??;
    let expected = sorted_quest_objective_rows(quest_options.objective_seed.clone());
    let actual = sorted_quest_objective_rows(after);
    result.quest_objective_db_after = actual.clone();
    result.quest_objective_db_verified = actual == expected;
    if !result.quest_objective_db_verified {
        bail!(
            "objective rows changed across logout: expected {:?}, got {:?}",
            expected,
            actual
        );
    }

    Ok(())
}
pub(crate) async fn run_quest_smoke_inner(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    quest_options: &QuestSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let options_for_db = quest_options.clone();
    let bot_for_db = bot.clone();
    let target = tokio::task::spawn_blocking(move || {
        resolve_quest_target_for_bot(&bot_for_db, &options_for_db)
    })
    .await
    .map_err(|e| anyhow!("Quest target DB worker join failed: {}", e))??;

    result.quest_target_entry = Some(target.entry);
    result.quest_target_spawn_guid = Some(target.spawn_guid);
    result.quest_target_guid_counter = Some(target.guid_counter);
    result.quest_target_map_id = Some(target.map_id);

    info!(
        "[Bot {}] Quest smoke target: entry={} spawn_guid={} guid_counter={} map={}",
        bot_index, target.entry, target.spawn_guid, target.guid_counter, target.map_id
    );

    send_encrypted_packet(stream, crypt, 0x349C, &target.packed_guid).await?;
    send_encrypted_packet(stream, crypt, 0x3492, &target.packed_guid).await?;
    result.quest_gossip_hello_sent = true;
    info!("[Bot {}] ✅ CMSG_GOSSIP_HELLO sent", bot_index);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(quest_options.timeout_secs);
    let mut questgiver_hello_sent = false;
    let mut details_query_sent_for: Option<u32> = None;
    let mut accept_sent_for: Option<u32> = None;

    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let slice = remaining.min(Duration::from_millis(500));
        match tokio::time::timeout(slice, read_encrypted_packet(stream, crypt, server_inflater))
            .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                let parsed = parse_packet(op, &payload);
                info!("[Bot {}] 📦 {}", bot_index, parsed);
                record_quest_objective_login_signal(op, &payload, quest_options, result);
                handle_quest_smoke_packet(op, &payload, result);

                if let Some(gossip_option_id) = quest_options.gossip_select_option_id {
                    if !result.quest_gossip_select_sent {
                        if let Some(gossip_id) = result.quest_gossip_id_seen {
                            let select = build_gossip_select_option(
                                &target.packed_guid,
                                gossip_id,
                                gossip_option_id,
                            );
                            send_encrypted_packet(stream, crypt, 0x3494, &select).await?;
                            result.quest_gossip_select_sent = true;
                            info!(
                                "[Bot {}] ✅ CMSG_GOSSIP_SELECT_OPTION sent: gossip_id={} option_id={}",
                                bot_index, gossip_id, gossip_option_id
                            );
                        }
                    }
                }

                if quest_options.query_details
                    && details_query_sent_for.is_none()
                    && !quest_details_or_request_items_seen(result)
                {
                    if let Some(quest_id) = select_quest_to_query(quest_options, result) {
                        let query = build_quest_giver_query_quest(&target.packed_guid, quest_id);
                        send_encrypted_packet(stream, crypt, 0x3497, &query).await?;
                        details_query_sent_for = Some(quest_id);
                        info!(
                            "[Bot {}] ✅ CMSG_QUEST_GIVER_QUERY_QUEST sent for quest {}",
                            bot_index, quest_id
                        );
                    }
                }

                if quest_options.accept && result.quest_details_seen && accept_sent_for.is_none() {
                    if let Some(quest_id) = select_quest_to_query(quest_options, result) {
                        let accept = build_quest_giver_accept_quest(&target.packed_guid, quest_id);
                        send_encrypted_packet(stream, crypt, 0x3498, &accept).await?;
                        result.quest_accept_sent = true;
                        accept_sent_for = Some(quest_id);
                        info!(
                            "[Bot {}] ✅ CMSG_QUEST_GIVER_ACCEPT_QUEST sent for quest {}",
                            bot_index, quest_id
                        );
                    }
                }

                if quest_smoke_has_enough_signal(quest_options, result) {
                    break;
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Quest smoke read error: {}", bot_index, e);
                break;
            }
            Err(_) => {
                if !questgiver_hello_sent {
                    send_encrypted_packet(stream, crypt, 0x3496, &target.packed_guid).await?;
                    result.quest_questgiver_hello_sent = true;
                    questgiver_hello_sent = true;
                    info!("[Bot {}] ✅ CMSG_QUEST_GIVER_HELLO sent", bot_index);
                }
            }
        }
    }

    if !quest_options.objective_persist
        && !result.quest_gossip_message_seen
        && !result.quest_quest_list_seen
        && !result.quest_details_seen
        && !result.quest_request_items_seen
        && !result.trainer_list_seen
    {
        bail!("No GossipMessage, QuestList, QuestDetails, RequestItems, or TrainerList response received");
    }

    if quest_options.accept {
        let quest_id = quest_options
            .expected_quest_id
            .or(accept_sent_for)
            .ok_or_else(|| {
                anyhow!("Quest accept requested but no selected quest id was available")
            })?;
        let bot_for_db = bot.clone();
        let (verified, status) =
            tokio::task::spawn_blocking(move || verify_quest_accepted_in_db(&bot_for_db, quest_id))
                .await
                .map_err(|e| anyhow!("Quest DB verification worker join failed: {}", e))??;
        result.quest_db_verified = verified;
        result.quest_db_status = status;
    }

    Ok(())
}
pub(crate) fn record_quest_offers(
    offers: Vec<packet_parser::QuestOffer>,
    result: &mut BotRunResult,
) {
    for offer in offers {
        record_quest_id(offer.quest_id, result);
        if !offer.title.is_empty() && !result.quest_titles_seen.contains(&offer.title) {
            result.quest_titles_seen.push(offer.title);
        }
    }
}
pub(crate) fn record_quest_id(quest_id: u32, result: &mut BotRunResult) {
    if quest_id != 0 && !result.quest_ids_seen.contains(&quest_id) {
        result.quest_ids_seen.push(quest_id);
    }
}
pub(crate) fn select_quest_to_query(
    quest_options: &QuestSmokeOptions,
    result: &BotRunResult,
) -> Option<u32> {
    if let Some(expected) = quest_options.expected_quest_id {
        if result.quest_ids_seen.contains(&expected) {
            return Some(expected);
        }
    }
    result.quest_ids_seen.first().copied()
}
pub(crate) fn quest_smoke_has_enough_signal(
    quest_options: &QuestSmokeOptions,
    result: &BotRunResult,
) -> bool {
    if quest_options.expect_trainer_list {
        return result.trainer_list_seen;
    }
    if quest_options.accept {
        return result.quest_accept_sent
            && (result.quest_accept_confirm_seen || result.quest_db_verified);
    }
    if quest_options.query_details {
        quest_details_or_request_items_seen(result)
    } else {
        result.quest_gossip_message_seen
            || result.quest_quest_list_seen
            || result.quest_details_seen
            || result.quest_request_items_seen
    }
}
pub(crate) fn quest_smoke_passes(
    quest_options: &QuestSmokeOptions,
    result: &mut BotRunResult,
) -> bool {
    result.quest_failure = None;

    if !quest_options.objective_persist
        && !result.quest_gossip_message_seen
        && !result.quest_quest_list_seen
        && !result.quest_details_seen
        && !result.quest_request_items_seen
        && !result.trainer_list_seen
    {
        result.quest_failure = Some("No questgiver or trainer response was received".to_string());
        return false;
    }

    if quest_options.expect_trainer_list && !result.trainer_list_seen {
        result.quest_failure = Some("TrainerList was not received".to_string());
        return false;
    }

    if let Some(expected) = quest_options.expect_trainer_id {
        if result.trainer_id_seen != Some(expected) {
            result.quest_failure = Some(format!(
                "Expected trainer id {}, got {:?}",
                expected, result.trainer_id_seen
            ));
            return false;
        }
    }

    if quest_options.query_details
        && !quest_options.objective_persist
        && !quest_details_or_request_items_seen(result)
    {
        result.quest_failure = Some("QuestDetails or RequestItems was not received".to_string());
        return false;
    }

    if quest_options.accept {
        if !result.quest_accept_sent {
            result.quest_failure = Some("Quest accept packet was not sent".to_string());
            return false;
        }
        if !result.quest_db_verified {
            result.quest_failure = Some(format!(
                "Accepted quest was not verified in DB (status={:?})",
                result.quest_db_status
            ));
            return false;
        }
    }

    if quest_options.objective_persist && !result.quest_objective_db_verified {
        result.quest_failure = Some(format!(
            "Quest objective DB rows did not survive logout (before={:?}, after={:?})",
            result.quest_objective_db_before, result.quest_objective_db_after
        ));
        return false;
    }

    if !quest_options.objective_persist {
        if let Some(expected) = quest_options.expected_quest_id {
            if !result.quest_ids_seen.contains(&expected) {
                result.quest_failure = Some(format!("Expected quest {} was not seen", expected));
                return false;
            }
        }
    }

    if let Some(forbidden) = quest_options.forbidden_quest_id {
        if result.quest_ids_seen.contains(&forbidden) {
            result.quest_failure = Some(format!("Forbidden quest {} was offered", forbidden));
            return false;
        }
    }

    if let Some(forbidden_title) = &quest_options.forbidden_title_contains {
        let needle = forbidden_title.to_ascii_lowercase();
        if result
            .quest_titles_seen
            .iter()
            .any(|title| title.to_ascii_lowercase().contains(&needle))
        {
            result.quest_failure = Some(format!(
                "Forbidden quest title fragment `{}` was offered",
                forbidden_title
            ));
            return false;
        }
    }

    true
}
pub(crate) fn resolve_quest_target_for_bot(
    bot: &config::BotConfig,
    quest_options: &QuestSmokeOptions,
) -> Result<ResolvedCreatureTarget> {
    use mysql::prelude::Queryable;

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|e| anyhow!("Bad world DB URL: {}", e))?;
    let mut world =
        mysql::Conn::new(world_opts).map_err(|e| anyhow!("Connect to world DB failed: {}", e))?;

    let (spawn_guid, entry, map_id, x, y, z, orientation) = if let Some(spawn_guid) =
        quest_options.creature_spawn_guid
    {
        let row: Option<(u64, u32, u32, f64, f64, f64, f32)> = world
            .exec_first(
                "SELECT guid, id, map, position_x, position_y, position_z, orientation \
                 FROM creature WHERE guid = ?",
                (spawn_guid,),
            )
            .map_err(|e| anyhow!("Lookup creature spawn {}: {}", spawn_guid, e))?;
        let (guid, entry, map_id, x, y, z, orientation) =
            row.ok_or_else(|| anyhow!("No world.creature row for guid {}", spawn_guid))?;
        if entry != quest_options.creature_entry {
            bail!(
                "Creature guid {} has entry {}, expected {}",
                guid,
                entry,
                quest_options.creature_entry
            );
        }
        (
            guid,
            entry,
            quest_options.map_id.unwrap_or(map_id as u16),
            x,
            y,
            z,
            orientation,
        )
    } else {
        let player_position = load_bot_position(bot.character_guid).ok();
        let map_id = quest_options
            .map_id
            .or_else(|| player_position.as_ref().map(|p| p.0))
            .ok_or_else(|| {
                anyhow!("Set WOW_BOT_QUEST_MAP_ID or use a bot character with a saved map position")
            })?;

        let row: Option<(u64, u32, u32, f64, f64, f64, f32)> = if let Some((player_map, x, y, z)) =
            player_position
        {
            let query_map = if quest_options.map_id.is_some() {
                map_id
            } else {
                player_map
            };
            world
                .exec_first(
                    "SELECT guid, id, map, position_x, position_y, position_z, orientation FROM creature \
                     WHERE id = ? AND map = ? \
                     ORDER BY POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2) \
                     LIMIT 1",
                    (quest_options.creature_entry, query_map, x, y, z),
                )
                .map_err(|e| anyhow!("Lookup nearest creature target: {}", e))?
        } else {
            world
                .exec_first(
                    "SELECT guid, id, map, position_x, position_y, position_z, orientation FROM creature \
                     WHERE id = ? AND map = ? \
                     ORDER BY guid LIMIT 1",
                    (quest_options.creature_entry, map_id),
                )
                .map_err(|e| anyhow!("Lookup creature target by entry/map: {}", e))?
        };

        row.ok_or_else(|| {
            anyhow!(
                "No world.creature row for entry {} on map {}",
                quest_options.creature_entry,
                map_id
            )
        })
        .map(|(guid, entry, row_map, x, y, z, orientation)| {
            (guid, entry, row_map as u16, x, y, z, orientation)
        })?
    };

    let guid_counter = resolve_quest_runtime_counter(
        quest_options.creature_guid_counter,
        spawn_guid,
        quest_options.creature_entry,
    )?;
    let (low, high) = create_creature_guid_raw(map_id, entry, guid_counter);
    Ok(ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id,
        x,
        y,
        z,
        orientation,
        packed_guid: build_packed_guid(low, high),
    })
}
pub(crate) fn reset_bot_quest_state(
    conn: &mut mysql::Conn,
    character_guid: u64,
    quest_id: u32,
) -> Result<()> {
    use mysql::prelude::Queryable;

    conn.exec_drop(
        "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?",
        (character_guid, quest_id),
    )
    .map_err(|e| anyhow!("DELETE character_queststatus: {}", e))?;
    conn.exec_drop(
        "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?",
        (character_guid, quest_id),
    )
    .map_err(|e| anyhow!("DELETE character_queststatus_objectives: {}", e))?;
    conn.exec_drop(
        "DELETE FROM character_queststatus_rewarded WHERE guid = ? AND quest = ?",
        (character_guid, quest_id),
    )
    .map_err(|e| anyhow!("DELETE character_queststatus_rewarded: {}", e))?;
    Ok(())
}
pub(crate) fn seed_bot_quest_objective_state(
    conn: &mut mysql::Conn,
    character_guid: u64,
    quest_id: u32,
    status: u8,
    rows: &[QuestObjectiveDbRow],
) -> Result<()> {
    use mysql::prelude::Queryable;

    let accept_time = chrono::Utc::now().timestamp();
    conn.exec_drop(
        "REPLACE INTO character_queststatus \
         (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, 0, ?, 0)",
        (character_guid, quest_id, status, accept_time),
    )
    .map_err(|e| anyhow!("REPLACE character_queststatus: {}", e))?;
    conn.exec_drop(
        "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?",
        (character_guid, quest_id),
    )
    .map_err(|e| anyhow!("DELETE character_queststatus_objectives: {}", e))?;

    for row in rows.iter().filter(|row| row.data != 0) {
        conn.exec_drop(
            "REPLACE INTO character_queststatus_objectives \
             (guid, quest, objective, data) VALUES (?, ?, ?, ?)",
            (character_guid, quest_id, row.objective, row.data),
        )
        .map_err(|e| anyhow!("REPLACE character_queststatus_objectives: {}", e))?;
    }
    Ok(())
}
pub(crate) fn load_bot_quest_objectives(
    bot: &config::BotConfig,
    quest_id: u32,
) -> Result<Vec<QuestObjectiveDbRow>> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;
    let mut rows: Vec<QuestObjectiveDbRow> = conn
        .exec_map(
            "SELECT objective, data FROM character_queststatus_objectives \
             WHERE guid = ? AND quest = ? ORDER BY objective",
            (bot.character_guid, quest_id),
            |(objective, data)| QuestObjectiveDbRow { objective, data },
        )
        .map_err(|e| anyhow!("SELECT character_queststatus_objectives: {}", e))?;
    rows.sort();
    Ok(rows)
}
pub(crate) fn verify_quest_accepted_in_db(
    bot: &config::BotConfig,
    quest_id: u32,
) -> Result<(bool, Option<u8>)> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut last_status = None;
    loop {
        let row: Option<(u8,)> = conn
            .exec_first(
                "SELECT status FROM character_queststatus WHERE guid = ? AND quest = ?",
                (bot.character_guid, quest_id),
            )
            .map_err(|e| anyhow!("SELECT accepted quest status: {}", e))?;
        if let Some((status,)) = row {
            last_status = Some(status);
            if status != 0 {
                return Ok((true, last_status));
            }
        }

        if std::time::Instant::now() >= deadline {
            return Ok((false, last_status));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
pub(crate) fn resolve_quest_runtime_counter(
    runtime_counter: Option<u64>,
    spawn_guid: u64,
    entry: u32,
) -> Result<u64> {
    runtime_counter.ok_or_else(|| {
        anyhow!(
            "Quest smoke for entry {entry} resolved world.creature guid {spawn_guid}, but needs the live ObjectGuid low counter. Set WOW_BOT_QUEST_RUNTIME_COUNTER or pass --quest-runtime-counter."
        )
    })
}
pub(crate) fn build_quest_giver_query_quest(packed_guid: &[u8], quest_id: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity(packed_guid.len() + 5);
    data.extend_from_slice(packed_guid);
    data.extend_from_slice(&quest_id.to_le_bytes());
    data.push(0x80); // RespondToGiver=true, MSB-first WriteBit/ReadBit.
    data
}
pub(crate) fn build_quest_giver_accept_quest(packed_guid: &[u8], quest_id: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity(packed_guid.len() + 5);
    data.extend_from_slice(packed_guid);
    data.extend_from_slice(&quest_id.to_le_bytes());
    data.push(0x00); // StartCheat=false, padded to one bit byte.
    data
}

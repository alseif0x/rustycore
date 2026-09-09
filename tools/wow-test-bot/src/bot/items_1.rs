//! Items operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn prepare_equipment_set_smoke_fixture(
    bots: &[config::BotConfig],
) -> Result<EquipmentSetSmokeFixture> {
    use mysql::prelude::Queryable;

    if bots.len() != 2 {
        bail!("equipment-set fixture requires exactly two bots");
    }
    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;

    for bot in bots {
        if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
            bail!(
                "refusing equipment-set fixture setup for non-local account {}",
                bot.account
            );
        }
        let row: Option<(u32, u8)> = conn
            .exec_first(
                "SELECT account, online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Load equipment-set bot character: {error}"))?;
        let (owner, online) = row.ok_or_else(|| {
            anyhow!(
                "No characters row for equipment-set bot guid {}",
                bot.character_guid
            )
        })?;
        if owner != bot.account_id || online != 0 {
            bail!(
                "equipment-set bot {} ownership/online mismatch: owner={owner}, expected={}, online={online}",
                bot.character_guid,
                bot.account_id
            );
        }
        let equipment_rows: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM character_equipmentsets WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Count equipment-set fixture rows: {error}"))?
            .unwrap_or(0);
        let transmog_rows: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM character_transmog_outfits WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Count transmog-set fixture rows: {error}"))?
            .unwrap_or(0);
        if equipment_rows != 0 || transmog_rows != 0 {
            bail!(
                "equipment-set bot {} is not an empty disposable fixture (equipment={equipment_rows}, transmog={transmog_rows})",
                bot.character_guid
            );
        }
    }

    let initial_max_guid: Option<Option<u64>> = conn
        .query_first(SHARED_EQUIPMENT_SET_GUID_MAX_QUERY)
        .map_err(|error| anyhow!("Load shared equipment/transmog maximum: {error}"))?;
    Ok(EquipmentSetSmokeFixture {
        initial_max_guid: initial_max_guid.flatten().unwrap_or(0),
    })
}
pub(crate) fn verify_equipment_set_db_row(
    bot: &config::BotConfig,
    options: &EquipmentSetSmokeOptions,
    expected_guid: u64,
) -> Result<bool> {
    use mysql::prelude::Queryable;

    let opts = mysql::Opts::from_url(&characters_db_url()?)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let equipment_rows: Vec<mysql::Row> = conn
        .exec(
            "SELECT CAST(setguid AS UNSIGNED) AS setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0, item1, item2, item3, item4, item5, item6, item7, item8, item9, item10, item11, item12, item13, item14, item15, item16, item17, item18 FROM character_equipmentsets WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load persisted equipment sets: {error}"))?;
    let equipment_rows = equipment_rows
        .iter()
        .map(equipment_set_db_row_from_mysql)
        .collect::<Result<Vec<_>>>()?;
    let transmog_rows: Vec<mysql::Row> = conn
        .exec(
            "SELECT CAST(setguid AS UNSIGNED) AS setguid, setindex, name, iconname, ignore_mask, appearance0, appearance1, appearance2, appearance3, appearance4, appearance5, appearance6, appearance7, appearance8, appearance9, appearance10, appearance11, appearance12, appearance13, appearance14, appearance15, appearance16, appearance17, appearance18, mainHandEnchant, offHandEnchant FROM character_transmog_outfits WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load persisted transmog outfits: {error}"))?;
    let transmog_rows = transmog_rows
        .iter()
        .map(transmog_outfit_db_row_from_mysql)
        .collect::<Result<Vec<_>>>()?;

    Ok(equipment_set_db_rows_match(
        options,
        expected_guid,
        &equipment_rows,
        &transmog_rows,
    ))
}
pub(crate) fn equipment_set_db_rows_match(
    options: &EquipmentSetSmokeOptions,
    expected_guid: u64,
    equipment_rows: &[EquipmentSetDbRow],
    transmog_rows: &[TransmogOutfitDbRow],
) -> bool {
    match options.set_type {
        0 => {
            equipment_rows == [expected_equipment_set_db_row(options, expected_guid)]
                && transmog_rows.is_empty()
        }
        1 => {
            equipment_rows.is_empty()
                && transmog_rows == [expected_transmog_outfit_db_row(options, expected_guid)]
        }
        _ => false,
    }
}
pub(crate) fn expected_equipment_set_db_row(
    options: &EquipmentSetSmokeOptions,
    expected_guid: u64,
) -> EquipmentSetDbRow {
    EquipmentSetDbRow {
        set_guid: expected_guid,
        set_index: options.set_id,
        name: options.set_name.clone(),
        icon_name: options.set_icon.clone(),
        ignore_mask: EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP,
        assigned_spec_index: -1,
        items: [0; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    }
}
pub(crate) fn cleanup_equipment_set_smoke_fixture(bots: &[config::BotConfig]) -> Result<()> {
    use mysql::prelude::Queryable;

    let opts = mysql::Opts::from_url(&characters_db_url()?)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    for bot in bots {
        let offline_deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            let online: u8 = conn
                .exec_first(
                    "SELECT online FROM characters WHERE guid = ?",
                    (bot.character_guid,),
                )
                .map_err(|error| anyhow!("Check equipment-set bot offline state: {error}"))?
                .ok_or_else(|| anyhow!("Equipment-set bot character disappeared"))?;
            if online == 0 {
                break;
            }
            if std::time::Instant::now() >= offline_deadline {
                bail!(
                    "equipment-set bot {} remained online before cleanup",
                    bot.character_guid
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        conn.exec_drop(
            "DELETE FROM character_equipmentsets WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clean equipment-set fixture rows: {error}"))?;
        conn.exec_drop(
            "DELETE FROM character_transmog_outfits WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clean transmog-set fixture rows: {error}"))?;
        let remaining: u64 = conn
            .exec_first(
                "SELECT (SELECT COUNT(*) FROM character_equipmentsets WHERE guid = ?) + (SELECT COUNT(*) FROM character_transmog_outfits WHERE guid = ?)",
                (bot.character_guid, bot.character_guid),
            )
            .map_err(|error| anyhow!("Verify equipment-set fixture cleanup: {error}"))?
            .unwrap_or(u64::MAX);
        if remaining != 0 {
            bail!(
                "equipment-set fixture cleanup left {remaining} rows for character {}",
                bot.character_guid
            );
        }
    }
    Ok(())
}
pub(crate) fn build_save_equipment_set_payload(
    options: &EquipmentSetSmokeOptions,
) -> Result<Vec<u8>> {
    let name_len = u32::try_from(options.set_name.len())?;
    let icon_len = u32::try_from(options.set_icon.len())?;
    if name_len > u8::MAX.into() || icon_len >= (1 << 9) {
        bail!("equipment-set fixture name/icon exceeds the packet bit width");
    }
    let mut data = Vec::with_capacity(512);
    data.extend_from_slice(&options.set_type.to_le_bytes());
    data.extend_from_slice(&0_u64.to_le_bytes());
    data.extend_from_slice(&options.set_id.to_le_bytes());
    data.extend_from_slice(&EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP.to_le_bytes());
    for _ in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
        data.extend_from_slice(&[0; 16]);
        data.extend_from_slice(&0_i32.to_le_bytes());
    }
    for _ in 0..6 {
        data.extend_from_slice(&0_i32.to_le_bytes());
    }
    let mut bit_offset = 0;
    push_msb_bits(&mut data, &mut bit_offset, 0, 1);
    push_msb_bits(&mut data, &mut bit_offset, name_len, 8);
    push_msb_bits(&mut data, &mut bit_offset, icon_len, 9);
    data.extend_from_slice(options.set_name.as_bytes());
    data.extend_from_slice(options.set_icon.as_bytes());
    Ok(data)
}
pub(crate) fn take_equipment_bytes<'a>(
    payload: &'a [u8],
    offset: &mut usize,
    count: usize,
) -> Result<&'a [u8]> {
    let end = offset
        .checked_add(count)
        .ok_or_else(|| anyhow!("equipment-set packet offset overflow"))?;
    let bytes = payload
        .get(*offset..end)
        .ok_or_else(|| anyhow!("equipment-set packet truncated at byte {}", *offset))?;
    *offset = end;
    Ok(bytes)
}
pub(crate) fn parse_load_equipment_sets(payload: &[u8]) -> Result<Vec<EquipmentSetWire>> {
    let mut offset = 0;
    let count = read_equipment_u32(payload, &mut offset)?;
    let mut sets = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let set_type = read_equipment_i32(payload, &mut offset)?;
        let guid = read_equipment_u64(payload, &mut offset)?;
        let set_id = read_equipment_u32(payload, &mut offset)?;
        let ignore_mask = read_equipment_u32(payload, &mut offset)?;
        let mut pieces = [[0_u8; 16]; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        let mut appearances = [0_i32; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        for index in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
            pieces[index] = take_equipment_bytes(payload, &mut offset, 16)?.try_into()?;
            appearances[index] = read_equipment_i32(payload, &mut offset)?;
        }
        let enchants = [
            read_equipment_i32(payload, &mut offset)?,
            read_equipment_i32(payload, &mut offset)?,
        ];
        let secondary_appearances_and_slots = [
            read_equipment_i32(payload, &mut offset)?,
            read_equipment_i32(payload, &mut offset)?,
            read_equipment_i32(payload, &mut offset)?,
            read_equipment_i32(payload, &mut offset)?,
        ];
        let mut bit_offset = offset * 8;
        let has_spec = read_equipment_msb_bits(payload, &mut bit_offset, 1)? != 0;
        let name_len = read_equipment_msb_bits(payload, &mut bit_offset, 8)? as usize;
        let icon_len = read_equipment_msb_bits(payload, &mut bit_offset, 9)? as usize;
        offset = bit_offset.div_ceil(8);
        let assigned_spec_index = if has_spec {
            read_equipment_i32(payload, &mut offset)?
        } else {
            -1
        };
        let set_name =
            std::str::from_utf8(take_equipment_bytes(payload, &mut offset, name_len)?)?.to_string();
        let set_icon =
            std::str::from_utf8(take_equipment_bytes(payload, &mut offset, icon_len)?)?.to_string();
        sets.push(EquipmentSetWire {
            set_type,
            guid,
            set_id,
            ignore_mask,
            pieces,
            appearances,
            enchants,
            secondary_appearances_and_slots,
            assigned_spec_index,
            set_name,
            set_icon,
        });
    }
    if offset != payload.len() {
        bail!(
            "SMSG_LOAD_EQUIPMENT_SET left {} trailing bytes",
            payload.len() - offset
        );
    }
    Ok(sets)
}
pub(crate) fn parse_equipment_set_id(payload: &[u8]) -> Result<(u64, i32, u32)> {
    if payload.len() != 16 {
        bail!(
            "SMSG_EQUIPMENT_SET_ID payload length mismatch: expected 16, got {}",
            payload.len()
        );
    }
    let mut offset = 0;
    Ok((
        read_equipment_u64(payload, &mut offset)?,
        read_equipment_i32(payload, &mut offset)?,
        read_equipment_u32(payload, &mut offset)?,
    ))
}
pub(crate) fn validate_equipment_set_id_response(
    on_realm: bool,
    payload: &[u8],
    options: &EquipmentSetSmokeOptions,
) -> Result<u64> {
    if on_realm {
        bail!("SMSG_EQUIPMENT_SET_ID arrived on realm instead of instance");
    }
    let (guid, set_type, set_id) = parse_equipment_set_id(payload)?;
    if guid == 0 || set_type != options.set_type || set_id != options.set_id {
        bail!(
            "SMSG_EQUIPMENT_SET_ID mismatch: got {guid}/{set_type}/{set_id}, expected nonzero/{}/{}",
            options.set_type,
            options.set_id
        );
    }
    Ok(guid)
}
pub(crate) async fn wait_for_equipment_set_id_routed(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    mut realm: Option<&mut EncryptedWorldConnection>,
    options: &EquipmentSetSmokeOptions,
    result: &mut BotRunResult,
) -> Result<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for SMSG_EQUIPMENT_SET_ID");
        }
        let routed = if let Some(realm_connection) = realm.as_deref_mut() {
            read_encrypted_packet_if_ready(
                &mut realm_connection.stream,
                &mut realm_connection.crypt,
                &mut realm_connection.inflater,
                remaining.min(Duration::from_millis(5)),
                remaining,
                "equipment-set realm response",
            )
            .await?
            .map(|(opcode, payload)| (true, opcode, payload))
        } else {
            None
        };
        let routed = if routed.is_some() {
            routed
        } else {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            read_encrypted_packet_if_ready(
                stream,
                crypt,
                inflater,
                remaining.min(Duration::from_millis(5)),
                remaining,
                "equipment-set instance response",
            )
            .await?
            .map(|(opcode, payload)| (false, opcode, payload))
        };
        let Some((on_realm, opcode, payload)) = routed else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_TIME_SYNC_REQUEST {
            if on_realm {
                bail!("SMSG_TIME_SYNC_REQUEST arrived on realm during equipment-set save");
            }
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            continue;
        }
        if opcode != SMSG_EQUIPMENT_SET_ID {
            continue;
        }
        let guid = validate_equipment_set_id_response(on_realm, &payload, options)?;
        info!(
            "[Bot {}] ✅ SMSG_EQUIPMENT_SET_ID guid={} type={} set_id={} route=instance",
            bot_index, guid, options.set_type, options.set_id
        );
        return Ok(guid);
    }
}
pub(crate) async fn run_equipment_set_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    mut realm: Option<&mut EncryptedWorldConnection>,
    options: &EquipmentSetSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if !result.equipment_set_load_seen {
        bail!("login omitted SMSG_LOAD_EQUIPMENT_SET");
    }
    match options.phase {
        EquipmentSetSmokePhase::Save => {
            if result.equipment_set_login_count != Some(0) {
                bail!(
                    "disposable equipment-set fixture loaded {:?} pre-existing sets",
                    result.equipment_set_login_count
                );
            }
            let barrier = options
                .save_barrier
                .as_ref()
                .context("equipment-set save phase missing race barrier")?;
            tokio::time::timeout(Duration::from_secs(options.timeout_secs), barrier.wait())
                .await
                .map_err(|_| anyhow!("timed out at equipment-set concurrent save barrier"))?;
            let payload = build_save_equipment_set_payload(options)?;
            send_encrypted_packet(stream, crypt, CMSG_SAVE_EQUIPMENT_SET, &payload).await?;
            info!(
                "[Bot {}] ✅ CMSG_SAVE_EQUIPMENT_SET sent type={} set_id={}",
                bot_index, options.set_type, options.set_id
            );
            let guid = wait_for_equipment_set_id_routed(
                bot_index,
                stream,
                crypt,
                inflater,
                realm.as_deref_mut(),
                options,
                result,
            )
            .await?;
            result.equipment_set_generated_guid = Some(guid);
            loot_race::logout_and_wait_routed_like_cpp(
                bot_index,
                stream,
                crypt,
                inflater,
                realm.as_deref_mut(),
                bot.character_guid,
                result,
            )
            .await?;
            let bot_for_db = bot.clone();
            let options_for_db = options.clone();
            result.equipment_set_db_persisted = tokio::task::spawn_blocking(move || {
                verify_equipment_set_db_row(&bot_for_db, &options_for_db, guid)
            })
            .await
            .map_err(|error| anyhow!("Equipment-set persistence worker failed: {error}"))??;
            if !result.equipment_set_db_persisted {
                bail!("equipment/transmog set did not persist exactly after logout");
            }
            result.equipment_set_smoke_passed = Some(true);
        }
        EquipmentSetSmokePhase::VerifyRelog => {
            if result.equipment_set_login_count != Some(1) || !result.equipment_set_relogin_verified
            {
                bail!(
                    "fresh relog did not load exactly the expected set (count={:?}, matched={})",
                    result.equipment_set_login_count,
                    result.equipment_set_relogin_verified
                );
            }
            let guid = options
                .expected_guid
                .context("equipment-set relog phase missing expected GUID")?;
            loot_race::logout_and_wait_routed_like_cpp(
                bot_index,
                stream,
                crypt,
                inflater,
                realm.as_deref_mut(),
                bot.character_guid,
                result,
            )
            .await?;
            let bot_for_db = bot.clone();
            let options_for_db = options.clone();
            result.equipment_set_db_persisted = tokio::task::spawn_blocking(move || {
                verify_equipment_set_db_row(&bot_for_db, &options_for_db, guid)
            })
            .await
            .map_err(|error| anyhow!("Equipment-set relog DB worker failed: {error}"))??;
            result.equipment_set_smoke_passed = Some(result.equipment_set_db_persisted);
        }
    }
    Ok(())
}
pub(crate) async fn run_equipment_set_race_workflow(
    mut bots: Vec<config::BotConfig>,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    account_a: String,
    account_b: String,
    timeout_secs: u64,
) -> Result<Vec<BotRunResult>> {
    bots.sort_by_key(|bot| {
        if bot.account.eq_ignore_ascii_case(&account_a) {
            0
        } else if bot.account.eq_ignore_ascii_case(&account_b) {
            1
        } else {
            2
        }
    });
    if bots.len() != 2
        || !bots[0].account.eq_ignore_ascii_case(&account_a)
        || !bots[1].account.eq_ignore_ascii_case(&account_b)
    {
        bail!("configured equipment-set race accounts were not both found exactly once");
    }
    let bots_for_setup = bots.clone();
    let fixture =
        tokio::task::spawn_blocking(move || prepare_equipment_set_smoke_fixture(&bots_for_setup))
            .await
            .map_err(|error| anyhow!("Equipment-set fixture setup worker failed: {error}"))??;

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let base = [
        EquipmentSetSmokeOptions {
            phase: EquipmentSetSmokePhase::Save,
            set_type: 0,
            set_id: 7,
            set_name: "QA Equipment".to_string(),
            set_icon: "INV_Sword_01".to_string(),
            expected_guid: None,
            save_barrier: Some(std::sync::Arc::clone(&barrier)),
            timeout_secs,
        },
        EquipmentSetSmokeOptions {
            phase: EquipmentSetSmokePhase::Save,
            set_type: 1,
            set_id: 8,
            set_name: "QA Transmog".to_string(),
            set_icon: "INV_Chest_Cloth_01".to_string(),
            expected_guid: None,
            save_barrier: Some(barrier),
            timeout_secs,
        },
    ];

    let first = tokio::join!(
        run_bot(
            bots[0].clone(),
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
            Some(base[0].clone()),
            None,
        ),
        run_bot(
            bots[1].clone(),
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
            Some(base[1].clone()),
            None,
        )
    );

    let workflow = async {
        let mut saved = vec![first.0?, first.1?];
        if saved
            .iter()
            .any(|result| !result.equipment_set_smoke_passed.unwrap_or(false))
        {
            let failures = saved
                .iter()
                .filter_map(|result| {
                    (!result.equipment_set_smoke_passed.unwrap_or(false)).then(|| {
                        format!(
                            "{}: {}",
                            result.account,
                            result
                                .equipment_set_failure
                                .as_deref()
                                .unwrap_or("missing success verdict")
                        )
                    })
                })
                .collect::<Vec<_>>()
                .join("; ");
            bail!("concurrent equipment-set save phase failed: {failures}");
        }
        let guids = [
            saved[0]
                .equipment_set_generated_guid
                .context("equipment-set bot omitted generated GUID")?,
            saved[1]
                .equipment_set_generated_guid
                .context("transmog-set bot omitted generated GUID")?,
        ];
        if guids[0] == guids[1] || guids.iter().any(|guid| *guid <= fixture.initial_max_guid) {
            bail!(
                "shared allocator proof failed: initial max={}, generated={guids:?}",
                fixture.initial_max_guid
            );
        }

        let mut verify = base.clone();
        for index in 0..2 {
            verify[index].phase = EquipmentSetSmokePhase::VerifyRelog;
            verify[index].expected_guid = Some(guids[index]);
            verify[index].save_barrier = None;
        }
        let relog = tokio::join!(
            run_bot(
                bots[0].clone(),
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
                Some(verify[0].clone()),
                None,
            ),
            run_bot(
                bots[1].clone(),
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
                Some(verify[1].clone()),
                None,
            )
        );
        let reloaded = [relog.0?, relog.1?];
        for index in 0..2 {
            saved[index].world_auth &= reloaded[index].world_auth;
            saved[index].enum_characters &= reloaded[index].enum_characters;
            saved[index].player_login_verified &= reloaded[index].player_login_verified;
            saved[index].equipment_set_login_count = reloaded[index].equipment_set_login_count;
            saved[index].equipment_set_load_seen = reloaded[index].equipment_set_load_seen;
            saved[index].equipment_set_relogin_verified =
                reloaded[index].equipment_set_relogin_verified;
            saved[index].equipment_set_db_persisted &= reloaded[index].equipment_set_db_persisted;
            saved[index]
                .seen_opcodes
                .extend(reloaded[index].seen_opcodes.clone());
            saved[index].equipment_set_failure = reloaded[index].equipment_set_failure.clone();
            saved[index].equipment_set_smoke_passed = Some(
                saved[index].equipment_set_db_persisted
                    && saved[index].equipment_set_relogin_verified
                    && reloaded[index].equipment_set_smoke_passed.unwrap_or(false),
            );
        }
        Ok::<_, anyhow::Error>(saved)
    }
    .await;

    let bots_for_cleanup = bots.clone();
    let cleanup =
        tokio::task::spawn_blocking(move || cleanup_equipment_set_smoke_fixture(&bots_for_cleanup))
            .await
            .map_err(|error| anyhow!("Equipment-set cleanup worker failed: {error}"))?;
    match (workflow, cleanup) {
        (Ok(results), Ok(())) => Ok(results),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(workflow_error), Ok(())) => Err(workflow_error),
        (Err(workflow_error), Err(cleanup_error)) => Err(anyhow!(
            "equipment-set workflow failed: {workflow_error}; cleanup failed: {cleanup_error}"
        )),
    }
}
pub(crate) fn item_guid_raw(db_guid: u64, runtime_realm_id: u16) -> (u64, u64) {
    let high = (3u64 << 58) | ((u64::from(runtime_realm_id) & 0x1FFF) << 42);
    (db_guid & OBJECT_GUID_COUNTER_MASK, high)
}
pub(crate) fn parse_void_item_wire(
    payload: &[u8],
    cursor: &mut usize,
) -> Result<VoidStorageItemWire> {
    let (guid_len, item_id, _) = parse_packed_guid(
        payload
            .get(*cursor..)
            .ok_or_else(|| anyhow!("truncated void-storage item GUID"))?,
    )
    .ok_or_else(|| anyhow!("truncated void-storage item GUID"))?;
    *cursor += guid_len;
    let (creator_len, _, _) = parse_packed_guid(
        payload
            .get(*cursor..)
            .ok_or_else(|| anyhow!("truncated void-storage creator GUID"))?,
    )
    .ok_or_else(|| anyhow!("truncated void-storage creator GUID"))?;
    *cursor += creator_len;
    let fixed_end = cursor
        .checked_add(18)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| anyhow!("truncated void-storage item"))?;
    let slot = u32::from_le_bytes(payload[*cursor..*cursor + 4].try_into()?);
    *cursor += 4;
    let item_entry = i32::from_le_bytes(payload[*cursor..*cursor + 4].try_into()?);
    if item_entry <= 0 {
        bail!("void-storage item carried invalid entry {item_entry}");
    }
    *cursor += 12; // item id + random seed + random property id
    let has_bonus = payload[*cursor] & 0x80 != 0;
    *cursor += 1;
    let modifier_count = usize::from(payload[*cursor] >> 2);
    *cursor += 1;
    let modifier_bytes = modifier_count
        .checked_mul(5)
        .ok_or_else(|| anyhow!("void-storage modifier length overflow"))?;
    *cursor = cursor
        .checked_add(modifier_bytes)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| anyhow!("truncated void-storage item modifiers"))?;
    if has_bonus {
        if *cursor + 5 > payload.len() {
            bail!("truncated void-storage item bonuses");
        }
        *cursor += 1;
        let bonus_count = u32::from_le_bytes(payload[*cursor..*cursor + 4].try_into()?) as usize;
        *cursor += 4;
        let bonus_bytes = bonus_count
            .checked_mul(4)
            .ok_or_else(|| anyhow!("void-storage bonus length overflow"))?;
        *cursor = cursor
            .checked_add(bonus_bytes)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| anyhow!("truncated void-storage bonus list"))?;
    }
    debug_assert!(*cursor >= fixed_end);
    Ok(VoidStorageItemWire {
        item_id,
        slot,
        item_entry: item_entry as u32,
    })
}

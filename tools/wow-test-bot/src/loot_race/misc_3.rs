//! Loot-race misc operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::misc_1::SMSG_ITEM_PUSH_RESULT;
use super::*;

/// Decode the C++ `PartyPackets.cpp::PartyUpdate::Write` prefix and each
/// `PartyPlayerInfo`. The race must prove an actual two-player HOME party; a
/// stray, destroyed, raid, LFG, or malformed PartyUpdate is not coordination.
pub(crate) fn validate_party_update_like_cpp(
    payload: &[u8],
    receiver_guid: (u64, u64),
    peer_guid: (u64, u64),
    expected_leader_guid: (u64, u64),
) -> Result<()> {
    const GROUP_CATEGORY_HOME_LIKE_CPP: u8 = 0;
    const GROUP_TYPE_NORMAL_LIKE_CPP: u8 = 1;
    const EXPECTED_PARTY_MEMBERS: u32 = 2;

    let mut cursor = PartyUpdateCursor::new(payload);
    let party_flags = cursor.read_u16("PartyFlags")?;
    let party_index = cursor.read_u8("PartyIndex")?;
    let party_type = cursor.read_u8("PartyType")?;
    let my_index = cursor.read_i32("MyIndex")?;
    let party_guid = cursor.read_packed_guid("PartyGUID")?;
    let _sequence_num = cursor.read_u32("SequenceNum")?;
    let leader_guid = cursor.read_packed_guid("LeaderGUID")?;
    let _leader_faction_group = cursor.read_u8("LeaderFactionGroup")?;
    let player_count = cursor.read_u32("PlayerList.size")?;
    let optional_bits = cursor.read_u8("optional-value bits")?;
    let has_lfg_info = optional_bits & 0x80 != 0;
    let has_loot_settings = optional_bits & 0x40 != 0;
    let has_difficulty_settings = optional_bits & 0x20 != 0;

    if party_flags != 0
        || party_index != GROUP_CATEGORY_HOME_LIKE_CPP
        || party_type != GROUP_TYPE_NORMAL_LIKE_CPP
    {
        bail!(
            "loot-race PartyUpdate was not a normal HOME party: flags={party_flags:#06X} index={party_index} type={party_type}"
        );
    }
    if party_guid == (0, 0) {
        bail!("loot-race PartyUpdate carried an empty PartyGUID");
    }
    if leader_guid != expected_leader_guid {
        bail!(
            "loot-race PartyUpdate leader {:#018X}/{:#018X} did not match inviter {:#018X}/{:#018X}",
            leader_guid.0,
            leader_guid.1,
            expected_leader_guid.0,
            expected_leader_guid.1
        );
    }
    if player_count != EXPECTED_PARTY_MEMBERS {
        bail!(
            "loot-race PartyUpdate carried {player_count} player(s), expected {EXPECTED_PARTY_MEMBERS}"
        );
    }
    if optional_bits & 0x1F != 0 {
        bail!(
            "malformed PartyUpdate optional-value bit padding {:#04X}",
            optional_bits & 0x1F
        );
    }
    // C++ `Group::SendUpdateToPlayer` includes loot and difficulty settings
    // for a non-LFG group with more than one member.
    if has_lfg_info || !has_loot_settings || !has_difficulty_settings {
        bail!(
            "loot-race PartyUpdate optional values did not describe a normal two-player group: lfg={has_lfg_info} loot={has_loot_settings} difficulty={has_difficulty_settings}"
        );
    }

    let mut roster = Vec::with_capacity(EXPECTED_PARTY_MEMBERS as usize);
    for player_index in 0..player_count {
        // `PartyPlayerInfo::operator<<` writes 15 MSB-first bits, then the
        // following packed GUID byte-write aligns to the next byte.
        let info_bits = u16::from_be_bytes(
            cursor
                .take(2, "PartyPlayerInfo bit fields")?
                .try_into()
                .expect("exact bit-field slice"),
        );
        if info_bits & 1 != 0 {
            bail!("malformed PartyUpdate player {player_index}: nonzero aligned bit padding");
        }
        let info_bits = info_bits >> 1;
        let name_len = usize::from((info_bits >> 9) & 0x3F);
        let voice_len_plus_one = usize::from((info_bits >> 3) & 0x3F);
        let connected = info_bits & 0x04 != 0;
        if name_len == 0 || voice_len_plus_one == 0 {
            bail!(
                "malformed PartyUpdate player {player_index}: name_len={name_len} voice_len_plus_one={voice_len_plus_one}"
            );
        }

        let guid = cursor.read_packed_guid("PartyPlayerInfo.GUID")?;
        let subgroup = cursor.read_u8("PartyPlayerInfo.Subgroup")?;
        let _flags = cursor.read_u8("PartyPlayerInfo.Flags")?;
        let _roles = cursor.read_u8("PartyPlayerInfo.RolesAssigned")?;
        let _class = cursor.read_u8("PartyPlayerInfo.Class")?;
        let _faction = cursor.read_u8("PartyPlayerInfo.FactionGroup")?;
        let _name = cursor.take(name_len, "PartyPlayerInfo.Name")?;
        let _voice_state = cursor.take(voice_len_plus_one - 1, "PartyPlayerInfo.VoiceStateID")?;

        if !connected || subgroup != 0 {
            bail!(
                "loot-race PartyUpdate player {player_index} was not a connected HOME subgroup member: connected={connected} subgroup={subgroup}"
            );
        }
        roster.push(guid);
    }

    let receiver_index = usize::try_from(my_index)
        .ok()
        .filter(|index| *index < roster.len())
        .ok_or_else(|| {
            anyhow!("loot-race PartyUpdate MyIndex {my_index} was outside the roster")
        })?;
    if roster[receiver_index] != receiver_guid {
        bail!(
            "loot-race PartyUpdate MyIndex {my_index} identified {:#018X}/{:#018X}, expected receiver {:#018X}/{:#018X}",
            roster[receiver_index].0,
            roster[receiver_index].1,
            receiver_guid.0,
            receiver_guid.1
        );
    }
    let mut expected_roster = [receiver_guid, peer_guid];
    expected_roster.sort_unstable();
    roster.sort_unstable();
    if roster.as_slice() != expected_roster {
        bail!(
            "loot-race PartyUpdate roster {roster:?} did not contain exactly receiver/peer {expected_roster:?}"
        );
    }

    let loot_method = cursor.read_u8("PartyLootSettings.Method")?;
    let _loot_master = cursor.read_packed_guid("PartyLootSettings.LootMaster")?;
    let _loot_threshold = cursor.read_u8("PartyLootSettings.Threshold")?;
    if loot_method != PERSONAL_LOOT_METHOD_LIKE_CPP {
        bail!(
            "loot-race PartyUpdate loot method {loot_method} was not C++ PERSONAL_LOOT ({PERSONAL_LOOT_METHOD_LIKE_CPP})"
        );
    }
    let _difficulty_settings = cursor.take(12, "PartyDifficultySettings")?;
    if cursor.offset != payload.len() {
        bail!(
            "malformed PartyUpdate left {} trailing byte(s)",
            payload.len() - cursor.offset
        );
    }
    Ok(())
}
pub(crate) async fn form_party(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if options.participant == 0 {
        let (peer_low, peer_high) = create_player_guid_raw(options.peer_character_guid, realm_id());
        let payload = build_party_invite(&options.peer_name, peer_low, peer_high)?;
        send_encrypted_packet(
            &mut realm.stream,
            &mut realm.crypt,
            CMSG_PARTY_INVITE,
            &payload,
        )
        .await?;
        wait_for_realm_opcode(
            bot_index,
            stream,
            crypt,
            inflater,
            realm,
            options,
            SMSG_PARTY_UPDATE,
            result,
        )
        .await?;
    } else {
        wait_for_realm_opcode(
            bot_index,
            stream,
            crypt,
            inflater,
            realm,
            options,
            SMSG_PARTY_INVITE,
            result,
        )
        .await?;
        send_encrypted_packet(
            &mut realm.stream,
            &mut realm.crypt,
            CMSG_PARTY_INVITE_RESPONSE,
            &[0x40],
        )
        .await?;
        wait_for_realm_opcode(
            bot_index,
            stream,
            crypt,
            inflater,
            realm,
            options,
            SMSG_PARTY_UPDATE,
            result,
        )
        .await?;
    }
    Ok(())
}
pub(crate) async fn drain_until_target_or_quiet(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<bool> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut seen = false;
    while tokio::time::Instant::now() < deadline {
        options.sync.cancellation_error()?;
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(100),
            Duration::from_secs(2),
            "loot-race target discovery",
        )
        .await?
        else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if let Some(counter) = target_seen_in_update(options, opcode, &payload)? {
            result.loot_race_target_runtime_counter = Some(counter);
            seen = true;
        }
        handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        if seen {
            break;
        }
    }
    // Do not let unrelated queued realm traffic grow without bound.
    let _ = read_encrypted_packet_if_ready(
        &mut realm.stream,
        &mut realm.crypt,
        &mut realm.inflater,
        Duration::from_millis(1),
        Duration::from_secs(1),
        "loot-race realm discovery drain",
    )
    .await?;
    Ok(seen)
}
pub(crate) async fn kill_target_once(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let (target_low, target_high) = options.resolved_runtime_guid()?;
    let (killer_low, killer_high) =
        create_player_guid_raw(options.killer_character_guid, realm_id());
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_ATTACK_SWING,
        &build_packed_guid(target_low, target_high),
    )
    .await?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    let mut positive_damage = 0i64;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for the disposable loot-race creature to die after {positive_damage} observed damage"
            );
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "loot-race killer combat packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
            if opcode == SMSG_ATTACKER_STATE_UPDATE {
                let update = parse_attacker_state_update_summary(&payload)
                    .context("malformed loot-race SMSG_ATTACKER_STATE_UPDATE")?;
                if (update.attacker_guid_low, update.attacker_guid_high)
                    == (killer_low, killer_high)
                    && (update.victim_guid_low, update.victim_guid_high)
                        == (target_low, target_high)
                {
                    if update.damage < 0 {
                        bail!(
                            "loot-race killer reported negative damage {}",
                            update.damage
                        );
                    }
                    positive_damage += i64::from(update.damage);
                    if update.over_damage >= 0 {
                        if positive_damage == 0 {
                            bail!("loot-race target death had no positive killer damage evidence");
                        }
                        return Ok(());
                    }
                }
            } else if opcode == SMSG_ATTACK_STOP {
                let stop = parse_attack_stop_summary(&payload)
                    .context("malformed loot-race SMSG_ATTACK_STOP")?;
                if (stop.attacker_guid_low, stop.attacker_guid_high) == (killer_low, killer_high)
                    && (stop.victim_guid_low, stop.victim_guid_high) == (target_low, target_high)
                {
                    if !stop.now_dead {
                        bail!("server stopped the loot-race attack before the target died");
                    }
                    if positive_damage == 0 {
                        bail!("loot-race target death had no positive killer damage evidence");
                    }
                    return Ok(());
                }
            }
        }
        if let Some((opcode, _payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            Duration::from_millis(1),
            remaining,
            "loot-race killer realm combat packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
        }
    }
}
pub(crate) async fn wait_for_loot_window(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<LootWindow> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for SMSG_LOOT_RESPONSE");
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "loot-race loot response",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            if opcode == SMSG_LOOT_RESPONSE {
                let response = parse_loot_response(&payload)?;
                let expected_items = response
                    .items
                    .iter()
                    .filter(|item| {
                        item.item_entry == options.target.item_entry && item.quantity == 1
                    })
                    .collect::<Vec<_>>();
                let wrong_item_shape = expected_items.len() != 1
                    || (options.phase == LootRacePhase::CaptureItem && response.items.len() != 1);
                if wrong_item_shape {
                    bail!(
                        "loot response contained {} total rows and {} matching single-item rows for expected entry {}; strict capture requires one of each and race requires exactly one expected row",
                        response.items.len(),
                        expected_items.len(),
                        options.target.item_entry,
                    );
                }
                let item = expected_items[0];
                let (owner_low, owner_high) = options.resolved_runtime_guid()?;
                if (response.owner_low, response.owner_high) != (owner_low, owner_high) {
                    bail!("loot response owner did not match the exact acknowledged world spawn");
                }
                validate_loot_object_guid_like_cpp(
                    response.loot_low,
                    response.loot_high,
                    options.target.map_id,
                    realm_id(),
                )?;
                let wrong_money_shape = match options.phase {
                    LootRacePhase::CaptureItem => response.coins != 0,
                    LootRacePhase::Race => response.coins != RACE_GAMEOBJECT_MONEY,
                    LootRacePhase::VerifyRelog => true,
                };
                let wrong_method = options.phase == LootRacePhase::Race
                    && response.loot_method != PERSONAL_LOOT_METHOD_LIKE_CPP;
                if !response.acquired
                    || response.failure_reason != 17
                    || wrong_money_shape
                    || wrong_method
                {
                    bail!(
                        "loot response did not match the acquired item/money/method contract for {:?} (failure={}, acquired={}, coins={}, method={})",
                        options.phase,
                        response.failure_reason,
                        response.acquired,
                        response.coins,
                        response.loot_method
                    );
                }
                return Ok(LootWindow {
                    owner_low: response.owner_low,
                    owner_high: response.owner_high,
                    loot_low: response.loot_low,
                    loot_high: response.loot_high,
                    coins: response.coins,
                    item_entry: item.item_entry,
                    quantity: item.quantity,
                    loot_list_id: item.loot_list_id,
                    loot_method: response.loot_method,
                });
            }
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
        let _ = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            Duration::from_millis(1),
            remaining,
            "loot-race realm while opening",
        )
        .await?;
    }
}
pub(crate) async fn collect_evidence(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    expected_loot_owner: (u64, u64),
    window: Duration,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<WireEvidence> {
    let deadline = tokio::time::Instant::now() + window;
    let mut evidence = WireEvidence::default();
    while tokio::time::Instant::now() < deadline {
        options.sync.cancellation_error()?;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(20)),
            remaining,
            "loot-race realm evidence",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            record_evidence(opcode, &payload, expected_loot_owner, &mut evidence)?;
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "loot-race instance evidence",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            record_evidence(opcode, &payload, expected_loot_owner, &mut evidence)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
    Ok(evidence)
}
pub(crate) fn record_evidence(
    opcode: u16,
    payload: &[u8],
    expected_loot_owner: (u64, u64),
    evidence: &mut WireEvidence,
) -> Result<()> {
    match opcode {
        SMSG_ITEM_PUSH_RESULT => evidence.item_pushes.push(parse_item_push(payload)?),
        SMSG_LOOT_REMOVED => {
            let (owner_used, owner_low, owner_high) = parse_packed_guid(payload)
                .ok_or_else(|| anyhow!("malformed SMSG_LOOT_REMOVED owner guid"))?;
            if (owner_low, owner_high) != expected_loot_owner {
                bail!(
                    "SMSG_LOOT_REMOVED owner ({owner_low:#x}, {owner_high:#x}) did not match discovered world-object GUID ({:#x}, {:#x})",
                    expected_loot_owner.0,
                    expected_loot_owner.1
                );
            }
            let (loot_used, loot_low, loot_high) = parse_packed_guid(&payload[owner_used..])
                .ok_or_else(|| anyhow!("malformed SMSG_LOOT_REMOVED loot guid"))?;
            let list_id = *payload
                .get(owner_used + loot_used)
                .ok_or_else(|| anyhow!("malformed SMSG_LOOT_REMOVED list id"))?;
            if owner_used + loot_used + 1 != payload.len() {
                bail!("SMSG_LOOT_REMOVED has unexpected trailing bytes");
            }
            evidence.loot_removed.push(LootRemovedEvidence {
                owner_low,
                owner_high,
                loot_low,
                loot_high,
                loot_list_id: list_id,
            });
        }
        SMSG_LOOT_MONEY_NOTIFY => {
            if payload.len() != 17 {
                bail!("malformed SMSG_LOOT_MONEY_NOTIFY");
            }
            evidence.money_notifies.push(MoneyNotify {
                money: u64::from_le_bytes(payload[0..8].try_into()?),
                money_mod: u64::from_le_bytes(payload[8..16].try_into()?),
                sole_looter: payload[16] & 0x80 != 0,
            });
        }
        SMSG_COIN_REMOVED => {
            let (used, low, high) =
                parse_packed_guid(payload).ok_or_else(|| anyhow!("malformed SMSG_COIN_REMOVED"))?;
            if used != payload.len() {
                bail!("SMSG_COIN_REMOVED has unexpected trailing bytes");
            }
            evidence.coin_removed.push((low, high));
        }
        SMSG_INVENTORY_CHANGE_FAILURE => evidence
            .inventory_failures
            .push(parse_inventory_failure(payload)?),
        _ => {}
    }
    Ok(())
}
pub(crate) async fn validate_shared_windows(options: &LootRaceOptions) -> Result<()> {
    let windows = options.sync.windows.lock().await;
    let left = windows[0]
        .as_ref()
        .ok_or_else(|| anyhow!("bot A did not record a loot window"))?;
    let right = windows[1]
        .as_ref()
        .ok_or_else(|| anyhow!("bot B did not record a loot window"))?;
    if left != right {
        bail!("two bots did not receive the same shared loot authority: {left:?} vs {right:?}");
    }
    Ok(())
}
/// Validate the exact `ObjectGuid::Create<HighGuid::LootObject>` wire shape.
///
/// C++ `ObjectGuidFactory::CreateWorldObject` substitutes the active realm
/// when its realm argument is zero, then encodes a map-local LootObject with
/// zero subtype/server/entry and a nonzero map sequence counter.
pub(crate) fn validate_loot_object_guid_like_cpp(
    low: u64,
    high: u64,
    expected_map_id: u16,
    expected_realm_id: u32,
) -> Result<()> {
    let high_type = (high >> 58) & GUID_HIGH_TYPE_MASK;
    let realm = (high >> 42) & GUID_REALM_MASK;
    let map = (high >> 29) & GUID_MAP_MASK;
    let entry = (high >> 6) & GUID_ENTRY_MASK;
    let subtype = high & GUID_SUBTYPE_MASK;
    let server = (low >> 40) & GUID_SERVER_MASK;
    let counter = low & GUID_COUNTER_MASK;
    let expected_realm = u64::from(expected_realm_id) & GUID_REALM_MASK;
    let expected_map = u64::from(expected_map_id) & GUID_MAP_MASK;

    if high_type != HIGH_GUID_LOOT_OBJECT
        || realm != expected_realm
        || map != expected_map
        || entry != 0
        || subtype != 0
        || server != 0
        || counter == 0
    {
        bail!(
            "loot response LootObject GUID has invalid C++ structure: type={high_type}, realm={realm}, map={map}, entry={entry}, subtype={subtype}, server={server}, counter={counter}; expected type={HIGH_GUID_LOOT_OBJECT}, realm={expected_realm}, map={expected_map}, entry/subtype/server=0 and nonzero counter"
        );
    }

    Ok(())
}
pub(crate) async fn validate_item_outcome(
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let window = options.sync.windows.lock().await[options.participant]
        .clone()
        .ok_or_else(|| anyhow!("loot-race participant has no recorded loot window"))?;
    let (owner_low, owner_high) = options.resolved_runtime_guid()?;
    let expected_removal = LootRemovedEvidence {
        owner_low,
        owner_high,
        loot_low: window.loot_low,
        loot_high: window.loot_high,
        loot_list_id: window.loot_list_id,
    };
    let evidence = options.sync.evidence.lock().await;
    let character_guids = if options.participant == 0 {
        [options.character_guid, options.peer_character_guid]
    } else {
        [options.peer_character_guid, options.character_guid]
    };
    let grant = validate_atomic_item_wire_outcome_like_cpp(
        &evidence,
        character_guids,
        options.target.item_entry,
        window.quantity,
        expected_removal,
        realm_id(),
    )?;
    // `ItemPushResult::PlayerGUID`, rather than the receiving socket, names
    // the winner. This remains correct when #55 adds C++'s group broadcast.
    result.loot_race_item_push_seen = grant.owner_guid == options.character_guid;
    result.loot_race_loot_removed_seen =
        evidence[options.participant].loot_removed == [expected_removal];
    Ok(())
}
/// Prove one logical item grant without mistaking C++ party fanout for a
/// duplicate grant.
///
/// Issue #106 owns atomic claim authority, while the complete
/// `StoreLootItem` side-effect cascade remains #55. The current Rust path may
/// send only to the winner; C++ broadcasts the same packet to both group
/// members for the default item. Both shapes represent one grant, but any
/// second packet on one socket or any divergent packet fails closed.
pub(crate) fn validate_atomic_item_wire_outcome_like_cpp(
    evidence: &[WireEvidence; 2],
    character_guids: [u64; 2],
    expected_item_entry: u32,
    expected_quantity: u32,
    expected_removal: LootRemovedEvidence,
    expected_realm_id: u32,
) -> Result<ExpectedPersistedItemGrant> {
    for (participant, entry) in evidence.iter().enumerate() {
        if entry.loot_removed.as_slice() != [expected_removal] {
            bail!(
                "participant {participant} observed LootRemoved fanout {:?}; expected exactly {:?}",
                entry.loot_removed,
                expected_removal
            );
        }
        if entry.item_pushes.len() > 1 {
            bail!(
                "participant {participant} observed {} ItemPush packets; one logical grant permits at most one observation per socket",
                entry.item_pushes.len()
            );
        }
    }

    // Deliberately inspect every ItemPush before checking the expected entry.
    // Filtering by entry would hide a foreign or late duplicate grant.
    let wire_grants = evidence
        .iter()
        .enumerate()
        .flat_map(|(participant, entry)| {
            entry
                .item_pushes
                .iter()
                .copied()
                .map(move |push| WireItemGrant { participant, push })
        })
        .collect::<Vec<_>>();
    let first = wire_grants
        .first()
        .copied()
        .ok_or_else(|| anyhow!("atomic ITEM race emitted no ItemPush result"))?;
    if wire_grants.iter().any(|grant| grant.push != first.push) {
        bail!("atomic ITEM race emitted divergent ItemPush observations: {wire_grants:?}");
    }

    let push = first.push;
    let expected_quantity = i32::try_from(expected_quantity)
        .map_err(|_| anyhow!("loot-race expected item quantity exceeds i32"))?;
    if push.item_entry != expected_item_entry
        || push.quantity != expected_quantity
        || push.quantity_in_inventory != expected_quantity
    {
        bail!(
            "atomic ITEM push entry/quantity/inventory {:?} did not match expected {expected_item_entry}/{expected_quantity}/{expected_quantity}",
            (push.item_entry, push.quantity, push.quantity_in_inventory)
        );
    }
    if push.slot != INVENTORY_SLOT_BAG_0
        || !(i32::from(INVENTORY_SLOT_ITEM_START)..i32::from(INVENTORY_SLOT_ITEM_START + 16))
            .contains(&push.slot_in_bag)
    {
        bail!(
            "atomic ITEM push slot {}/{} was not one of the preflight-guaranteed base-backpack destinations",
            push.slot,
            push.slot_in_bag
        );
    }
    if push.pushed
        || push.created
        || push.display_text != 1
        || push.is_bonus_roll
        || push.is_encounter_loot
        || push.dungeon_encounter_id != 0
    {
        bail!(
            "atomic ITEM push flags were not the ordinary overworld StoreLootItem shape: {push:?}"
        );
    }
    let expected_item_high =
        (HIGH_GUID_ITEM << 58) | ((u64::from(expected_realm_id) & GUID_REALM_SPECIFIC_MASK) << 42);
    if push.item_guid_low == 0
        || push.item_guid_low & !GUID_COUNTER_MASK != 0
        || push.item_guid_high != expected_item_high
    {
        bail!(
            "atomic ITEM push GUID {:#018X}/{:#018X} was not a nonempty C++ Item GUID for realm {}",
            push.item_guid_low,
            push.item_guid_high,
            expected_realm_id
        );
    }

    let winner = character_guids
        .iter()
        .position(|guid| {
            create_player_guid_raw(*guid, expected_realm_id) == (push.player_low, push.player_high)
        })
        .ok_or_else(|| {
            anyhow!(
                "atomic ITEM push winner {:#018X}/{:#018X} was neither disposable character",
                push.player_low,
                push.player_high
            )
        })?;
    let loser = 1 - winner;
    if evidence[winner].item_pushes.as_slice() != [push] {
        bail!("atomic ITEM winner socket did not receive exactly its direct ItemPush");
    }
    if !evidence[loser].item_pushes.is_empty() && evidence[loser].item_pushes.as_slice() != [push] {
        bail!("atomic ITEM loser observed a non-identical group ItemPush");
    }

    let loot_gone = InventoryFailure {
        result: 50,
        item_0_low: 0,
        item_0_high: 0,
        item_1_low: 0,
        item_1_high: 0,
        container_b_slot: 0,
    };
    if !evidence[winner].inventory_failures.is_empty()
        || evidence[loser].inventory_failures.as_slice() != [loot_gone]
    {
        bail!(
            "atomic ITEM failure fanout was winner={:?}, loser={:?}; expected no winner failure and one exact EQUIP_ERR_LOOT_GONE (50)",
            evidence[winner].inventory_failures,
            evidence[loser].inventory_failures
        );
    }

    Ok(ExpectedPersistedItemGrant {
        owner_guid: character_guids[winner],
        push,
    })
}
pub(crate) async fn validate_money_outcome(
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let window = options.sync.windows.lock().await[options.participant]
        .clone()
        .ok_or_else(|| anyhow!("loot-race participant has no recorded loot window"))?;
    let evidence = options.sync.evidence.lock().await;
    let source_coins = u64::from(window.coins);
    let expected_source = (window.loot_low, window.loot_high);
    let winner = validate_serialized_gameobject_money_wire_outcome_like_cpp(
        &evidence,
        expected_source,
        source_coins,
    )?;
    result.loot_race_money_notify_amount = Some(if winner == options.participant {
        source_coins
    } else {
        0
    });
    result.loot_race_coin_removed_seen = evidence[options.participant]
        .coin_removed
        .contains(&expected_source);
    Ok(())
}
pub(crate) async fn handle_instance_housekeeping(
    _bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    opcode: u16,
    payload: &[u8],
) -> Result<()> {
    if opcode == SMSG_TIME_SYNC_REQUEST {
        let sequence = parse_time_sync_request_sequence(payload)?;
        let response = build_time_sync_response_payload(sequence, current_millis_u32());
        send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
    }
    Ok(())
}
pub(crate) fn build_party_invite(name: &str, target_low: u64, target_high: u64) -> Result<Vec<u8>> {
    if name.len() > 0x1FF {
        bail!("party invite name is too long");
    }
    // C++ reads HasPartyIndex as one bit and then ResetBitPos(), so the two
    // string lengths begin at the next byte rather than sharing that bit byte.
    let mut payload = vec![0];
    payload.extend_from_slice(&pack_msb_fields(&[(name.len() as u32, 9), (0, 9)]));
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&build_packed_guid(target_low, target_high));
    payload.extend_from_slice(name.as_bytes());
    Ok(payload)
}
pub(crate) fn build_loot_item_claim(window: &LootWindow) -> Vec<u8> {
    let mut payload = 1u32.to_le_bytes().to_vec();
    payload.extend_from_slice(&build_packed_guid(window.loot_low, window.loot_high));
    payload.push(window.loot_list_id);
    payload.push(0); // IsSoftInteract=false, flushed to a complete byte.
    payload
}
pub(crate) fn pack_msb_fields(fields: &[(u32, usize)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut current = 0u8;
    let mut used = 0usize;
    for &(value, width) in fields {
        for bit in (0..width).rev() {
            current |= (((value >> bit) & 1) as u8) << (7 - used);
            used += 1;
            if used == 8 {
                bytes.push(current);
                current = 0;
                used = 0;
            }
        }
    }
    if used != 0 {
        bytes.push(current);
    }
    bytes
}
#[derive(Debug)]
pub(crate) struct ParsedLootItem {
    pub(crate) item_entry: u32,
    pub(crate) quantity: u32,
    pub(crate) loot_list_id: u8,
}
#[derive(Debug)]
pub(crate) struct ParsedLootResponse {
    pub(crate) owner_low: u64,
    pub(crate) owner_high: u64,
    pub(crate) loot_low: u64,
    pub(crate) loot_high: u64,
    pub(crate) failure_reason: u8,
    pub(crate) loot_method: u8,
    pub(crate) coins: u32,
    pub(crate) acquired: bool,
    pub(crate) items: Vec<ParsedLootItem>,
}

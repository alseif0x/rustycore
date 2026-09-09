//! Loot-race misc operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

pub(crate) fn faction_for_race(race: u8) -> u8 {
    match race {
        1 | 3 | 4 | 7 | 11 | 22 | 25 => 0,
        _ => 1,
    }
}
pub(crate) fn current_millis_u32() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}
#[derive(Debug, Clone)]
pub(crate) struct GroupCapacityFixture {
    pub(crate) leader_guid: u64,
    pub(crate) candidate_names: [String; 2],
    pub(crate) candidate_guids: [u64; 2],
    pub(crate) initial_member_guids: [u64; 4],
    pub(crate) party_settings: GroupCapacityPartySettings,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GroupCapacityPartySettings {
    pub(crate) loot_method: u8,
    pub(crate) loot_threshold: u8,
    pub(crate) master_looter_guid: u64,
    pub(crate) dungeon_difficulty_id: u32,
    pub(crate) raid_difficulty_id: u32,
    pub(crate) legacy_raid_difficulty_id: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GroupCapacityPersistenceEvidence {
    pub(crate) final_member_count: u64,
    pub(crate) winning_candidate_guid: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupCapacityOutcome {
    Added,
    Full,
}
pub(crate) fn validate_group_capacity_initial_member_offline(
    member_guid: u64,
    online: u8,
) -> Result<()> {
    if online != 0 {
        bail!(
            "group-capacity initial member {member_guid} must be offline before the race; online={online}"
        );
    }
    Ok(())
}
pub(crate) fn validate_group_capacity_persisted_members(
    fixture: &GroupCapacityFixture,
    final_members: &[u64],
) -> Result<GroupCapacityPersistenceEvidence> {
    if final_members.len() != 5
        || !fixture
            .initial_member_guids
            .iter()
            .all(|guid| final_members.contains(guid))
    {
        bail!(
            "group-capacity race persisted {:?}; expected four initial members plus one candidate",
            final_members
        );
    }
    let persisted_candidates: Vec<_> = fixture
        .candidate_guids
        .iter()
        .copied()
        .filter(|guid| final_members.contains(guid))
        .collect();
    let [winning_candidate_guid] = persisted_candidates.as_slice() else {
        bail!(
            "group-capacity race persisted {} candidates instead of exactly one: {:?}",
            persisted_candidates.len(),
            final_members
        );
    };
    Ok(GroupCapacityPersistenceEvidence {
        final_member_count: final_members.len() as u64,
        winning_candidate_guid: *winning_candidate_guid,
    })
}
pub(crate) fn validate_group_capacity_winner_consistency(
    wire_winner_guid: u64,
    persisted_winner_guid: u64,
) -> Result<()> {
    if wire_winner_guid != persisted_winner_guid {
        bail!(
            "group-capacity winner mismatch: wire added GUID {wire_winner_guid}, CharacterDB persisted GUID {persisted_winner_guid}"
        );
    }
    Ok(())
}
pub(crate) async fn wait_group_capacity_barrier(
    options: &GroupCapacityRaceOptions,
    barrier: &Barrier,
    label: &str,
) -> Result<()> {
    options.sync.cancellation_error()?;
    tokio::time::timeout(Duration::from_secs(options.timeout_secs), async {
        tokio::select! {
            _ = barrier.wait() => Ok(()),
            _ = options.sync.cancelled.cancelled() => options.sync.cancellation_error(),
        }
    })
    .await
    .map_err(|_| anyhow!("group-capacity race timed out waiting for {label}"))??;
    Ok(())
}
pub(crate) fn validate_group_capacity_invite(payload: &[u8]) -> Result<()> {
    if read_msb_bits(payload, 0, 1) != Some(1) {
        bail!("group-capacity candidate received a PartyInvite that could not be accepted");
    }
    Ok(())
}
pub(crate) fn validate_group_full_result(payload: &[u8]) -> Result<()> {
    let name_len = read_msb_bits(payload, 0, 9)
        .ok_or_else(|| anyhow!("malformed group-capacity PartyCommandResult name length"))?;
    let operation = read_msb_bits(payload, 9, 4)
        .ok_or_else(|| anyhow!("malformed group-capacity PartyCommandResult operation"))?;
    let result = read_msb_bits(payload, 13, 6)
        .ok_or_else(|| anyhow!("malformed group-capacity PartyCommandResult result"))?;
    if name_len != 0 || operation != 0 || result != 4 {
        bail!(
            "group-capacity loser received PartyCommandResult name_len={name_len} operation={operation} result={result}; expected empty Invite/GROUP_FULL"
        );
    }
    validate_party_command_result_tail(payload, 0, "")?;
    Ok(())
}
pub(crate) fn validate_group_invite_ok_result(payload: &[u8], expected_name: &str) -> Result<()> {
    let name_len = read_msb_bits(payload, 0, 9)
        .ok_or_else(|| anyhow!("malformed group-capacity invite result name length"))?;
    let operation = read_msb_bits(payload, 9, 4)
        .ok_or_else(|| anyhow!("malformed group-capacity invite result operation"))?;
    let result = read_msb_bits(payload, 13, 6)
        .ok_or_else(|| anyhow!("malformed group-capacity invite result code"))?;
    if name_len != expected_name.len() as u32 || operation != 0 || result != 0 {
        bail!(
            "group-capacity invite for {expected_name} returned name_len={name_len} operation={operation} result={result}; expected Invite/OK"
        );
    }
    validate_party_command_result_tail(payload, name_len as usize, expected_name)?;
    Ok(())
}
pub(crate) fn validate_party_command_result_tail(
    payload: &[u8],
    name_len: usize,
    expected_name: &str,
) -> Result<()> {
    const BIT_HEADER_BYTES: usize = 3;
    const RESULT_DATA_END: usize = BIT_HEADER_BYTES + 4;
    if payload.get(2).is_none_or(|byte| byte & 0x1F != 0) {
        bail!("group-capacity PartyCommandResult had malformed bit padding");
    }
    let result_data = u32::from_le_bytes(
        payload
            .get(BIT_HEADER_BYTES..RESULT_DATA_END)
            .ok_or_else(|| anyhow!("group-capacity PartyCommandResult omitted ResultData"))?
            .try_into()
            .expect("exact PartyCommandResult ResultData slice"),
    );
    let (guid_len, guid_low, guid_high) = parse_packed_guid(
        payload
            .get(RESULT_DATA_END..)
            .ok_or_else(|| anyhow!("group-capacity PartyCommandResult omitted ResultGUID"))?,
    )
    .ok_or_else(|| anyhow!("group-capacity PartyCommandResult had malformed ResultGUID"))?;
    let name_start = RESULT_DATA_END + guid_len;
    let name_end = name_start
        .checked_add(name_len)
        .ok_or_else(|| anyhow!("group-capacity PartyCommandResult name length overflow"))?;
    let name = payload
        .get(name_start..name_end)
        .ok_or_else(|| anyhow!("group-capacity PartyCommandResult omitted its declared name"))?;
    if result_data != 0
        || (guid_low, guid_high) != (0, 0)
        || name != expected_name.as_bytes()
        || name_end != payload.len()
    {
        bail!(
            "group-capacity PartyCommandResult tail differed: ResultData={result_data} ResultGUID=({guid_low}, {guid_high}) name={name:?} trailing={} byte(s)",
            payload.len().saturating_sub(name_end)
        );
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupCapacityPartyUpdateEvidence {
    CompleteRoster,
    ConnectedOnlyRoster,
}
pub(crate) fn validate_group_capacity_party_update(
    payload: &[u8],
    options: &GroupCapacityRaceOptions,
) -> Result<GroupCapacityPartyUpdateEvidence> {
    const EXPECTED_MEMBERS: u32 = 5;
    let mut cursor = PartyUpdateCursor::new(payload);
    let party_flags = cursor.read_u16("PartyFlags")?;
    let party_index = cursor.read_u8("PartyIndex")?;
    let party_type = cursor.read_u8("PartyType")?;
    let my_index = cursor.read_i32("MyIndex")?;
    let party_guid = cursor.read_packed_guid("PartyGUID")?;
    let _sequence_num = cursor.read_u32("SequenceNum")?;
    let leader_guid = cursor.read_packed_guid("LeaderGUID")?;
    let _leader_faction = cursor.read_u8("LeaderFactionGroup")?;
    let member_count = cursor.read_u32("PlayerList.size")?;
    let optional_bits = cursor.read_u8("optional-value bits")?;
    let has_lfg_info = optional_bits & 0x80 != 0;
    let has_loot_settings = optional_bits & 0x40 != 0;
    let has_difficulty_settings = optional_bits & 0x20 != 0;
    if party_flags != 0
        || party_index != 0
        || party_type != 1
        || party_guid == (0, 0)
        || leader_guid != create_player_guid_raw(options.leader_guid, realm_id())
        || !matches!(member_count, 2 | EXPECTED_MEMBERS)
        || optional_bits & 0x1F != 0
    {
        bail!(
            "group-capacity winner received invalid normal HOME PartyUpdate: flags={party_flags:#06X} index={party_index} type={party_type} leader={leader_guid:?} members={member_count} optional={optional_bits:#04X}"
        );
    }
    if has_lfg_info || !has_loot_settings || !has_difficulty_settings {
        bail!(
            "group-capacity PartyUpdate optional values differed from a normal non-LFG group: lfg={has_lfg_info} loot={has_loot_settings} difficulty={has_difficulty_settings}"
        );
    }

    let mut roster = Vec::with_capacity(EXPECTED_MEMBERS as usize);
    for member_index in 0..member_count {
        let info_bits = u16::from_be_bytes(
            cursor
                .take(2, "PartyPlayerInfo bit fields")?
                .try_into()
                .expect("exact party-player bit field"),
        );
        if info_bits & 1 != 0 {
            bail!("group-capacity PartyUpdate member {member_index} had nonzero bit padding");
        }
        let info_bits = info_bits >> 1;
        let name_len = usize::from((info_bits >> 9) & 0x3F);
        let voice_len_plus_one = usize::from((info_bits >> 3) & 0x3F);
        if name_len == 0 || voice_len_plus_one == 0 {
            bail!(
                "group-capacity PartyUpdate member {member_index} had invalid name/voice lengths"
            );
        }
        let guid = cursor.read_packed_guid("PartyPlayerInfo.GUID")?;
        let subgroup = cursor.read_u8("PartyPlayerInfo.Subgroup")?;
        let _flags = cursor.read_u8("PartyPlayerInfo.Flags")?;
        let _roles = cursor.read_u8("PartyPlayerInfo.RolesAssigned")?;
        let _class = cursor.read_u8("PartyPlayerInfo.Class")?;
        let _faction = cursor.read_u8("PartyPlayerInfo.FactionGroup")?;
        let _name = cursor.take(name_len, "PartyPlayerInfo.Name")?;
        let _voice = cursor.take(voice_len_plus_one - 1, "PartyPlayerInfo.VoiceStateID")?;
        if subgroup != 0 {
            bail!("group-capacity normal party member {member_index} had subgroup {subgroup}");
        }
        roster.push(guid);
    }

    let receiver = create_player_guid_raw(options.character_guid, realm_id());
    let wire_roster = roster.clone();
    let mut expected: Vec<_> = options
        .initial_member_guids
        .iter()
        .copied()
        .chain(std::iter::once(options.character_guid))
        .map(|guid| create_player_guid_raw(guid, realm_id()))
        .collect();
    expected.sort_unstable();
    roster.sort_unstable();

    let evidence = if member_count == EXPECTED_MEMBERS {
        let receiver_index = usize::try_from(my_index)
            .ok()
            .filter(|index| *index < expected.len())
            .ok_or_else(|| anyhow!("group-capacity PartyUpdate MyIndex {my_index} was invalid"))?;
        if wire_roster[receiver_index] != receiver {
            bail!("group-capacity PartyUpdate MyIndex did not identify the winning candidate");
        }
        if roster != expected {
            bail!(
                "group-capacity PartyUpdate roster {roster:?} did not match initial members plus winner {expected:?}"
            );
        }
        GroupCapacityPartyUpdateEvidence::CompleteRoster
    } else {
        // C++ Group::SendUpdateToPlayer serializes every MemberSlot, including
        // offline players. The current Rust send_party_update path filters its
        // PlayerList through PlayerRegistry, but keeps MyIndex from the complete
        // group. Keep the #110 runtime race scoped to atomic admission by pinning
        // that existing divergence exactly; the post-race DB assertion below is
        // still the authority for the complete five-member persisted roster.
        let mut expected_connected = vec![
            create_player_guid_raw(options.leader_guid, realm_id()),
            receiver,
        ];
        expected_connected.sort_unstable();
        let expected_complete_index = i32::try_from(options.initial_member_guids.len())
            .expect("normal group fixture length fits i32");
        if my_index != expected_complete_index || roster != expected_connected {
            bail!(
                "group-capacity PartyUpdate connected-only roster {roster:?} with MyIndex {my_index} did not match leader plus winner {expected_connected:?} and complete index {expected_complete_index}"
            );
        }
        GroupCapacityPartyUpdateEvidence::ConnectedOnlyRoster
    };

    let loot_method = cursor.read_u8("PartyLootSettings.Method")?;
    let loot_master = cursor.read_packed_guid("PartyLootSettings.LootMaster")?;
    let loot_threshold = cursor.read_u8("PartyLootSettings.Threshold")?;
    let dungeon_difficulty_id = cursor.read_u32("PartyDifficultySettings.DungeonDifficultyID")?;
    let raid_difficulty_id = cursor.read_u32("PartyDifficultySettings.RaidDifficultyID")?;
    let legacy_raid_difficulty_id =
        cursor.read_u32("PartyDifficultySettings.LegacyRaidDifficultyID")?;
    let expected_loot_master = if options.party_settings.loot_method == 2 {
        create_player_guid_raw(options.party_settings.master_looter_guid, realm_id())
    } else {
        (0, 0)
    };
    if loot_method != options.party_settings.loot_method
        || loot_master != expected_loot_master
        || loot_threshold != options.party_settings.loot_threshold
        || dungeon_difficulty_id != options.party_settings.dungeon_difficulty_id
        || raid_difficulty_id != options.party_settings.raid_difficulty_id
        || legacy_raid_difficulty_id != options.party_settings.legacy_raid_difficulty_id
    {
        bail!(
            "group-capacity PartyUpdate settings differed from the preloaded group: loot={loot_method}/{loot_master:?}/{loot_threshold} difficulty={dungeon_difficulty_id}/{raid_difficulty_id}/{legacy_raid_difficulty_id}; expected loot={}/{expected_loot_master:?}/{} difficulty={}/{}/{}",
            options.party_settings.loot_method,
            options.party_settings.loot_threshold,
            options.party_settings.dungeon_difficulty_id,
            options.party_settings.raid_difficulty_id,
            options.party_settings.legacy_raid_difficulty_id,
        );
    }
    if cursor.offset != payload.len() {
        bail!(
            "group-capacity PartyUpdate left {} trailing byte(s)",
            payload.len() - cursor.offset
        );
    }
    Ok(evidence)
}
pub(crate) async fn wait_for_group_capacity_outcome(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &GroupCapacityRaceOptions,
    result: &mut BotRunResult,
) -> Result<GroupCapacityOutcome> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        options.sync.cancellation_error()?;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for group-capacity Added/GROUP_FULL outcome");
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "group-capacity outcome realm packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Realm)?;
            match opcode {
                SMSG_PARTY_UPDATE => {
                    let evidence = validate_group_capacity_party_update(&payload, options)?;
                    if evidence == GroupCapacityPartyUpdateEvidence::ConnectedOnlyRoster {
                        warn!(
                            "[Bot {}] group-capacity PartyUpdate exposed the known Rust connected-only roster boundary; exact five-member persistence will be checked in the DB",
                            bot_index
                        );
                    }
                    return Ok(GroupCapacityOutcome::Added);
                }
                SMSG_PARTY_COMMAND_RESULT => {
                    validate_group_full_result(&payload)?;
                    return Ok(GroupCapacityOutcome::Full);
                }
                _ => {}
            }
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "group-capacity outcome instance packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Instance)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
}
pub(crate) async fn run_group_capacity_phase(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &GroupCapacityRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let run = run_group_capacity_phase_inner(
        bot_index,
        stream,
        crypt,
        inflater,
        realm_connection,
        options,
        result,
    )
    .await;
    if let Err(error) = &run {
        options
            .sync
            .cancel(format!("participant {:?} failed: {error:#}", options.role));
    }
    run
}
pub(crate) async fn run_group_capacity_phase_inner(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &GroupCapacityRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let realm = realm_connection.as_mut().ok_or_else(|| {
        anyhow!("group-capacity race requires separate realm and instance sockets")
    })?;
    wait_group_capacity_barrier(options, &options.sync.logged_in, "all three logins").await?;

    match options.role {
        GroupCapacityRaceRole::Leader => {
            for index in 0..2 {
                let (low, high) =
                    create_player_guid_raw(options.candidate_guids[index], realm_id());
                let payload = build_party_invite(&options.candidate_names[index], low, high)?;
                send_encrypted_packet(
                    &mut realm.stream,
                    &mut realm.crypt,
                    CMSG_PARTY_INVITE,
                    &payload,
                )
                .await?;
                let invite_result = wait_for_group_capacity_realm_opcode(
                    bot_index,
                    stream,
                    crypt,
                    inflater,
                    realm,
                    options,
                    SMSG_PARTY_COMMAND_RESULT,
                    result,
                )
                .await?;
                validate_group_invite_ok_result(&invite_result, &options.candidate_names[index])?;
            }
            wait_group_capacity_barrier(
                options,
                &options.sync.invitations_sent,
                "both invitations sent",
            )
            .await?;
            result.group_capacity_outcome = Some("leader-observer".to_string());
        }
        GroupCapacityRaceRole::CandidateA | GroupCapacityRaceRole::CandidateB => {
            wait_group_capacity_barrier(
                options,
                &options.sync.invitations_sent,
                "both invitations sent",
            )
            .await?;
            let invite = wait_for_group_capacity_realm_opcode(
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
            validate_group_capacity_invite(&invite)?;
            wait_group_capacity_barrier(
                options,
                &options.sync.accepts_ready,
                "both candidates ready to accept",
            )
            .await?;
            send_encrypted_packet(
                &mut realm.stream,
                &mut realm.crypt,
                CMSG_PARTY_INVITE_RESPONSE,
                &[0x40],
            )
            .await?;
            let outcome = wait_for_group_capacity_outcome(
                bot_index, stream, crypt, inflater, realm, options, result,
            )
            .await?;
            result.group_capacity_outcome = Some(
                match outcome {
                    GroupCapacityOutcome::Added => "added",
                    GroupCapacityOutcome::Full => "full",
                }
                .to_string(),
            );
        }
    }

    wait_group_capacity_barrier(
        options,
        &options.sync.outcomes_observed,
        "both candidate outcomes observed",
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
    Ok(())
}

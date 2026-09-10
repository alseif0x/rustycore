//! Packets packets.
//!
//! Separated from mod.rs under #709.

use super::*;

pub(super) fn set_difficulty_request(difficulty_id: u32) -> WorldPacket {
    let mut request = WorldPacket::new_empty();
    request.write_uint32(difficulty_id);
    request.reset_read();
    request
}

pub(super) fn set_dungeon_difficulty_request(difficulty_id: u32) -> WorldPacket {
    let mut request = WorldPacket::new_empty();
    request.write_uint32(difficulty_id);
    request.reset_read();
    request
}

pub(super) fn set_raid_difficulty_request(difficulty_id: i32, legacy: u8) -> WorldPacket {
    let mut request = WorldPacket::new_empty();
    request.write_int32(difficulty_id);
    request.write_uint8(legacy);
    request.reset_read();
    request
}

pub(super) fn request_cemetery_list_packet(extra_payload: Option<u8>) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    if let Some(byte) = extra_payload {
        packet.write_uint8(byte);
    }
    packet.reset_read();
    packet
}

pub(super) fn read_cemetery_list_response(bytes: &[u8]) -> (bool, Vec<u32>) {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::RequestCemeteryListResponse)
    );
    assert_eq!(
        packet.read_uint16().unwrap(),
        ServerOpcodes::RequestCemeteryListResponse as u16
    );
    let is_gossip_triggered = packet.read_bit().unwrap();
    let count = packet.read_uint32().unwrap();
    let cemetery_ids = (0..count)
        .map(|_| packet.read_uint32().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(packet.remaining(), 0);
    (is_gossip_triggered, cemetery_ids)
}

pub(super) fn auto_guild_bank_item_packet(
    banker: ObjectGuid,
    bank_tab: u8,
    bank_slot: u8,
    container_item_slot: u8,
    container_slot: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(bank_tab);
    pkt.write_uint8(bank_slot);
    pkt.write_uint8(container_item_slot);
    pkt.write_bit(container_slot.is_some());
    pkt.flush_bits();
    if let Some(container_slot) = container_slot {
        pkt.write_uint8(container_slot);
    }
    pkt.reset_read();
    pkt
}

pub(super) fn guild_bank_activate_packet(banker: ObjectGuid, full_update: bool) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_bit(full_update);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

pub(super) fn guild_bank_query_tab_packet(
    banker: ObjectGuid,
    tab: u8,
    full_update: bool,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(tab);
    pkt.write_bit(full_update);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

pub(super) fn guild_bank_money_packet(banker: ObjectGuid, money: u64) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint64(money);
    pkt.reset_read();
    pkt
}

pub(super) fn guild_bank_buy_tab_packet(banker: ObjectGuid, tab: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(tab);
    pkt.reset_read();
    pkt
}

pub(super) fn guild_bank_update_tab_packet(
    banker: ObjectGuid,
    tab: u8,
    name: &str,
    icon: &str,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(tab);
    pkt.write_bits(name.len() as u32, 7);
    pkt.write_bits(icon.len() as u32, 9);
    pkt.flush_bits();
    pkt.write_string(name);
    pkt.write_string(icon);
    pkt.reset_read();
    pkt
}

pub(super) fn guild_bank_tab_query_packet(tab: i32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(tab);
    pkt.reset_read();
    pkt
}

pub(super) fn guild_bank_set_tab_text_packet(tab: i32, text: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(tab);
    pkt.write_bits(text.len() as u32, 14);
    pkt.flush_bits();
    pkt.write_string(text);
    pkt.reset_read();
    pkt
}

pub(super) fn auto_store_guild_bank_item_packet(
    banker: ObjectGuid,
    bank_tab: u8,
    bank_slot: u8,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(bank_tab);
    pkt.write_uint8(bank_slot);
    pkt.reset_read();
    pkt
}

pub(super) fn battlemaster_hello_packet(unit: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&unit);
    pkt.reset_read();
    pkt
}

pub(super) fn battlefield_list_packet(list_id: i32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(list_id);
    pkt.reset_read();
    pkt
}

pub(super) fn battlemaster_join_packet(
    queue_ids: &[u64],
    roles: u8,
    blacklist_map: [i32; 2],
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(queue_ids.len() as u32);
    pkt.write_uint8(roles);
    pkt.write_int32(blacklist_map[0]);
    pkt.write_int32(blacklist_map[1]);
    for queue_id in queue_ids {
        pkt.write_uint64(*queue_id);
    }
    pkt.reset_read();
    pkt
}

pub(super) fn battlemaster_join_arena_packet(team_size_index: u8, roles: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(team_size_index);
    pkt.write_uint8(roles);
    pkt.reset_read();
    pkt
}

pub(super) fn battlemaster_join_skirmish_packet(
    bg_type_id: u32,
    bracket_id: u32,
    as_group: u8,
    is_rated: u8,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(bg_type_id);
    pkt.write_uint32(bracket_id);
    pkt.write_uint8(as_group);
    pkt.write_uint8(is_rated);
    pkt.reset_read();
    pkt
}

pub(super) fn battlefield_port_packet(
    requester_guid: ObjectGuid,
    slot: u32,
    ride_type: u32,
    time: i64,
    unknown925: bool,
    accepted_invite: bool,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&requester_guid);
    pkt.write_uint32(slot);
    pkt.write_uint32(ride_type);
    pkt.write_int64(time);
    pkt.write_bit(unknown925);
    pkt.flush_bits();
    pkt.write_bit(accepted_invite);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

pub(super) fn resurrect_response_packet(resurrecter: ObjectGuid, response: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&resurrecter);
    pkt.write_uint32(response);
    pkt
}

pub(super) fn repop_request_packet(check_instance: bool) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(check_instance);
    pkt.flush_bits();
    pkt
}

pub(super) fn port_graveyard_packet() -> WorldPacket {
    WorldPacket::new_empty()
}

pub(super) fn reclaim_corpse_packet(corpse_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&corpse_guid.to_raw_bytes());
    pkt
}

pub(super) fn update_account_data_packet(
    player_guid: ObjectGuid,
    data_type: u8,
    time: i64,
    data: &str,
) -> WorldPacket {
    let compressed_data = compress_account_data_like_cpp(data).unwrap();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&player_guid);
    pkt.write_int64(time);
    pkt.write_uint32(data.len() as u32);
    pkt.write_bits(u32::from(data_type), 4);
    pkt.write_uint32(compressed_data.len() as u32);
    pkt.write_bytes(&compressed_data);
    pkt
}

pub(super) fn request_account_data_packet(player_guid: ObjectGuid, data_type: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&player_guid);
    pkt.write_bits(u32::from(data_type), 4);
    pkt.flush_bits();
    pkt
}

pub(super) fn activate_taxi_packet(
    vendor: ObjectGuid,
    node: u32,
    ground_mount_id: u32,
    flying_mount_id: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&vendor);
    pkt.write_uint32(node);
    pkt.write_uint32(ground_mount_id);
    pkt.write_uint32(flying_mount_id);
    pkt
}

pub(super) fn bug_report_packet(report_type: bool, diag_info: &str, text: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(report_type);
    pkt.write_bits(diag_info.len() as u32, 12);
    pkt.write_bits(text.len() as u32, 10);
    pkt.flush_bits();
    pkt.write_string(diag_info);
    pkt.write_string(text);
    pkt.reset_read();
    pkt
}

pub(super) fn submit_user_feedback_packet(is_suggestion: bool, note: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_bits((note.len() + 1) as u32, 24);
    pkt.write_bit(is_suggestion);
    pkt.write_string(note);
    pkt.write_uint8(0);
    pkt.reset_read();
    pkt
}

pub(super) fn support_ticket_submit_bug_packet(message: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_bits(message.len() as u32, 10);
    pkt.write_string(message);
    pkt
}

pub(super) fn support_ticket_submit_complaint_packet(note: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    let target = ObjectGuid::create_player(1, 42);
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_packed_guid(&target);
    pkt.write_int32(1);
    pkt.write_int32(2);
    pkt.write_int32(4);
    pkt.write_uint32(0); // ChatLog.Lines.Count
    pkt.write_bit(false); // ReportLineIndex.HasValue
    pkt.flush_bits();
    pkt.write_bits(note.len() as u32, 10);
    pkt.write_bit(false); // MailInfo
    pkt.write_bit(false); // CalendarInfo
    pkt.write_bit(false); // PetInfo
    pkt.write_bit(false); // GuildInfo
    pkt.write_bit(false); // LFGListSearchResult
    pkt.write_bit(false); // LFGListApplicant
    pkt.write_bit(false); // ClubMessage
    pkt.write_bit(false); // ClubFinderResult
    pkt.write_bit(false); // Unused910
    pkt.flush_bits();
    pkt.write_uint32(0); // HorusChatLog.Lines.Count
    pkt.write_string(note);
    pkt
}

pub(super) fn support_ticket_submit_suggestion_packet(message: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(message.len() as u32, 10);
    pkt.write_string(message);
    pkt
}

pub(super) fn object_update_recovery_packet(guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.reset_read();
    pkt
}

pub(super) fn stand_state_change_packet(state: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(state);
    pkt.reset_read();
    pkt
}

pub(super) fn can_duel_packet(target_guid: ObjectGuid, to_the_death: bool) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_bytes(&target_guid.to_raw_bytes());
    packet.write_bit(to_the_death);
    packet.flush_bits();
    packet.reset_read();
    packet
}

pub(super) fn duel_response_packet(
    arbiter_guid: ObjectGuid,
    accepted: bool,
    forfeited: bool,
) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_bytes(&arbiter_guid.to_raw_bytes());
    packet.write_bit(accepted);
    packet.write_bit(forfeited);
    packet.flush_bits();
    packet.reset_read();
    packet
}

pub(super) fn accept_trade_packet(state_index: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(state_index);
    pkt.reset_read();
    pkt
}

pub(super) fn clear_trade_item_packet(trade_slot: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(trade_slot);
    pkt.reset_read();
    pkt
}

pub(super) fn set_trade_item_packet(
    trade_slot: u8,
    pack_slot: u8,
    item_slot_in_pack: u8,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(trade_slot);
    pkt.write_uint8(pack_slot);
    pkt.write_uint8(item_slot_in_pack);
    pkt.reset_read();
    pkt
}

pub(super) fn set_trade_gold_packet(coinage: u64) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(coinage);
    pkt.reset_read();
    pkt
}

pub(super) fn set_trade_spell_packet(
    spell_id: u32,
    pack_slot: u8,
    item_slot_in_pack: u8,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(spell_id);
    pkt.write_uint8(pack_slot);
    pkt.write_uint8(item_slot_in_pack);
    pkt.reset_read();
    pkt
}

pub(super) fn sign_petition_packet(petition_guid: ObjectGuid, choice: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&petition_guid.to_raw_bytes());
    pkt.write_uint8(choice);
    pkt.reset_read();
    pkt
}

pub(super) fn decline_petition_packet(petition_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&petition_guid.to_raw_bytes());
    pkt.reset_read();
    pkt
}

pub(super) fn query_petition_packet(petition_id: u32, item_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(petition_id);
    pkt.write_bytes(&item_guid.to_raw_bytes());
    pkt.reset_read();
    pkt
}

pub(super) fn save_cuf_profiles_packet(
    profiles: impl IntoIterator<Item = wow_packet::packets::misc::CufProfile>,
) -> WorldPacket {
    let profiles: Vec<_> = profiles.into_iter().collect();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::SaveCufProfiles as u16);
    pkt.write_uint32(profiles.len() as u32);
    for profile in profiles {
        pkt.write_bits(profile.profile_name.len() as u32, 7);
        for option in 0..wow_packet::packets::misc::CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
            pkt.write_bit(profile.bool_options & (1 << option) != 0);
        }
        pkt.write_uint16(profile.frame_height);
        pkt.write_uint16(profile.frame_width);
        pkt.write_uint8(profile.sort_by);
        pkt.write_uint8(profile.health_text);
        pkt.write_uint8(profile.top_point);
        pkt.write_uint8(profile.bottom_point);
        pkt.write_uint8(profile.left_point);
        pkt.write_uint16(profile.top_offset);
        pkt.write_uint16(profile.bottom_offset);
        pkt.write_uint16(profile.left_offset);
        pkt.write_string(&profile.profile_name);
    }
    WorldPacket::from_bytes(pkt.data())
}

pub(super) fn collection_item_set_favorite_packet(
    collection_type: u32,
    id: u32,
    favorite: bool,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::CollectionItemSetFavorite as u16);
    pkt.write_uint32(collection_type);
    pkt.write_uint32(id);
    pkt.write_bit(favorite);
    pkt.flush_bits();
    pkt
}

pub(super) fn battle_pet_clear_fanfare_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetClearFanfare as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

pub(super) fn battle_pet_delete_pet_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

pub(super) fn cage_battle_pet_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

pub(super) fn battle_pet_modify_name_packet(
    pet_guid: ObjectGuid,
    name: &str,
    declined_names: Option<[&str; 5]>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_bits(name.len() as u32, 7);
    pkt.write_bit(declined_names.is_some());
    if let Some(declined_names) = declined_names {
        for declined_name in declined_names {
            pkt.write_bits(declined_name.len() as u32, 7);
        }
        for declined_name in declined_names {
            pkt.write_string(declined_name);
        }
    }
    pkt.write_string(name);
    pkt
}

pub(super) fn battle_pet_set_flags_packet(
    pet_guid: ObjectGuid,
    flags: u16,
    control_type: u8,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSetFlags as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_uint16(flags);
    pkt.write_bits(u32::from(control_type), 2);
    pkt.flush_bits();
    pkt
}

pub(super) fn battle_pet_set_battle_slot_packet(pet_guid: ObjectGuid, slot: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSetBattleSlot as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_uint8(slot);
    pkt
}

pub(super) fn battle_pet_summon_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSummon as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

pub(super) fn battle_pet_update_notify_packet(pet_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetUpdateNotify as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt
}

pub(super) fn battle_pet_update_display_notify_packet() -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetUpdateDisplayNotify as u16);
    pkt
}

pub(super) fn dismiss_critter_packet(critter_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&critter_guid);
    pkt
}

pub(super) fn query_battle_pet_name_packet(
    battle_pet_id: ObjectGuid,
    unit_guid: ObjectGuid,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::QueryBattlePetName as u16);
    pkt.write_packed_guid(&battle_pet_id);
    pkt.write_packed_guid(&unit_guid);
    pkt
}

pub(super) fn battle_pet_request_journal_lock_packet() -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetRequestJournalLock as u16);
    pkt
}

pub(super) fn battle_pet_request_journal_packet() -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetRequestJournal as u16);
    pkt
}

pub(super) fn accept_wargame_invite_packet(inviter_name: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_string(inviter_name);
    pkt.write_uint8(0);
    pkt.reset_read();
    pkt
}

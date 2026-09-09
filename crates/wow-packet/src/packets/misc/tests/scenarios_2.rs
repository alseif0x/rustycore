//! Miscellaneous packet regressions, part 2 of 6.
//!
//! Moved out of the tests.rs root under #640; every test is unchanged.

use super::*;

#[test]
fn gm_ticket_system_status_matches_cpp_int32_shape() {
    let bytes = GmTicketSystemStatus::from_support_enabled_like_cpp(true).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::GmTicketSystemStatus as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_int32().unwrap(), GmTicketSystemStatus::ENABLED);
    assert_eq!(pkt.remaining(), 0);

    let bytes = GmTicketSystemStatus::from_support_enabled_like_cpp(false).to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_int32().unwrap(), GmTicketSystemStatus::DISABLED);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn gm_ticket_acknowledge_survey_reads_case_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(42);

    let survey = GmTicketAcknowledgeSurvey::read(&mut pkt).unwrap();
    assert_eq!(survey.case_id, 42);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn complaint_reads_chat_variant_like_cpp() {
    let offender_guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP);
    pkt.write_packed_guid(&offender_guid);
    pkt.write_uint32(0x0102_0304);
    pkt.write_uint32(55);
    pkt.write_uint32(7);
    pkt.write_uint32(9);
    pkt.write_bits(11, 12);
    pkt.write_string("hello world");

    let complaint = Complaint::read(&mut pkt).unwrap();

    assert_eq!(complaint.complaint_type, SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP);
    assert_eq!(complaint.offender.player_guid, offender_guid);
    assert_eq!(complaint.offender.realm_address, 0x0102_0304);
    assert_eq!(complaint.offender.time_since_offence, 55);
    assert!(complaint.mail_id.is_none());
    let chat = complaint.chat.expect("chat complaint payload");
    assert_eq!(chat.command, 7);
    assert_eq!(chat.channel_id, 9);
    assert_eq!(chat.message_log, "hello world");
    assert!(complaint.calendar_event_guid.is_none());
    assert!(complaint.calendar_invite_guid.is_none());
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn submit_user_feedback_reads_header_note_and_suggestion_bit_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_bits(6, 24); // "hello" plus null terminator
    pkt.write_bit(true);
    pkt.write_string("hello");
    pkt.write_uint8(0);

    let feedback = SubmitUserFeedback::read(&mut pkt).unwrap();

    assert_eq!(feedback.header.map_id, 571);
    assert_eq!(feedback.header.position, Position::xyz(1.25, 2.5, 3.75));
    assert_eq!(feedback.header.facing, 4.0);
    assert_eq!(feedback.header.program, 9);
    assert!(feedback.is_suggestion);
    assert_eq!(feedback.note, "hello");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn support_ticket_submit_suggestion_reads_10_bit_message_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    let message = "future idea text";
    pkt.write_bits(message.len() as u32, 10);
    pkt.write_string(message);

    let suggestion = SupportTicketSubmitSuggestion::read(&mut pkt).unwrap();

    assert_eq!(suggestion.message, message);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn support_ticket_submit_bug_reads_header_and_10_bit_message_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    let message = "broken thing";
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_bits(message.len() as u32, 10);
    pkt.write_string(message);

    let bug = SupportTicketSubmitBug::read(&mut pkt).unwrap();

    assert_eq!(bug.header.map_id, 571);
    assert_eq!(bug.header.position, Position::xyz(1.25, 2.5, 3.75));
    assert_eq!(bug.header.facing, 4.0);
    assert_eq!(bug.header.program, 9);
    assert_eq!(bug.message, message);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn support_ticket_submit_complaint_reads_chatlog_note_and_mail_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    let target = ObjectGuid::create_player(1, 42);
    let note = "report note";
    let chat_text = "bad text";
    let mail_body = "mail body";
    let mail_subject = "subject";

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
    pkt.write_uint32(1); // ChatLog.Lines.Count
    pkt.write_bit(true); // ReportLineIndex.HasValue
    pkt.write_int64(12345);
    pkt.write_bits(chat_text.len() as u32, 12);
    pkt.write_string(chat_text);
    pkt.write_uint32(0);
    pkt.write_bits(note.len() as u32, 10);
    pkt.write_bit(true); // MailInfo
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
    pkt.write_int64(77);
    pkt.write_bits(mail_body.len() as u32, 13);
    pkt.write_bits(mail_subject.len() as u32, 9);
    pkt.write_string(mail_body);
    pkt.write_string(mail_subject);

    let complaint = SupportTicketSubmitComplaint::read(&mut pkt).unwrap();

    assert_eq!(complaint.header.map_id, 571);
    assert_eq!(complaint.target_character_guid, target);
    assert_eq!(complaint.report_type, 1);
    assert_eq!(complaint.major_category, 2);
    assert_eq!(complaint.minor_category_flags, 4);
    assert_eq!(complaint.chat_log.lines.len(), 1);
    assert_eq!(complaint.chat_log.lines[0].timestamp, 12345);
    assert_eq!(complaint.chat_log.lines[0].text, chat_text);
    assert_eq!(complaint.chat_log.report_line_index, Some(0));
    assert!(complaint.horus_chat_log.lines.is_empty());
    assert_eq!(complaint.note, note);
    let mail = complaint.mail_info.expect("mail info");
    assert_eq!(mail.mail_id, 77);
    assert_eq!(mail.mail_body, mail_body);
    assert_eq!(mail.mail_subject, mail_subject);
    assert!(complaint.calendar_info.is_none());
    assert!(complaint.pet_info.is_none());
    assert!(complaint.guild_info.is_none());
    assert!(complaint.lfg_list_search_result.is_none());
    assert!(complaint.lfg_list_applicant.is_none());
    assert!(complaint.community_message.is_none());
    assert!(complaint.club_finder_result.is_none());
    assert!(complaint.unused910.is_none());
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn lfg_player_info_empty_matches_cpp_shape() {
    let bytes = LfgPlayerInfo::empty().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LfgPlayerInfo as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Dungeon.Count
    assert!(!pkt.has_bit().unwrap()); // BlackList.PlayerGuid.HasValue
    assert_eq!(pkt.read_uint32().unwrap(), 0); // BlackList.Slot.Count
}

#[test]
fn lfg_party_info_empty_matches_cpp_shape() {
    let bytes = LfgPartyInfo::empty().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LfgPartyInfo as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[test]
fn gm_ticket_case_status_empty_matches_cpp_todo_handler_shape() {
    let bytes = GmTicketCaseStatus::empty().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::GmTicketCaseStatus as u16
    );
    assert_eq!(bytes.len(), 2 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[test]
fn complaint_result_matches_cpp_shape() {
    let bytes = ComplaintResult {
        complaint_type: SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP as u32,
        result: ComplaintResult::OK_LIKE_CPP,
    }
    .to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::ComplaintResult as u16
    );
    assert_eq!(bytes.len(), 2 + 5);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        pkt.read_uint32().unwrap(),
        SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP as u32
    );
    assert_eq!(pkt.read_uint8().unwrap(), ComplaintResult::OK_LIKE_CPP);
}

#[test]
fn calendar_send_num_pending_matches_cpp_shape() {
    let bytes = CalendarSendNumPending { num_pending: 3 }.to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarSendNumPending as u16
    );
    assert_eq!(bytes.len(), 2 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 3);
}

#[test]
fn calendar_send_calendar_empty_matches_cpp_header_shape() {
    let bytes = CalendarSendCalendar::empty_at_unix(946_684_800).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarSendCalendar as u16
    );
    assert_eq!(bytes.len(), 2 + 4 + 4 + 4 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0x0000_3000); // 2000-01-01 00:00 UTC
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Invites.Count
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Events.Count
    assert_eq!(pkt.read_uint32().unwrap(), 0); // RaidLockouts.Count
}

#[test]
fn calendar_raid_lockout_added_matches_cpp_field_order() {
    let bytes = CalendarRaidLockoutAdded::new_at_unix(9001, 946_684_800, 631, 4, 86_400).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarRaidLockoutAdded as u16
    );
    assert_eq!(bytes.len(), 2 + 8 + 4 + 4 + 4 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint64().unwrap(), 9001);
    assert_eq!(pkt.read_uint32().unwrap(), 0x0000_3000); // 2000-01-01 00:00 UTC
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert_eq!(pkt.read_int32().unwrap(), 86_400);
}

#[test]
fn calendar_raid_lockout_removed_matches_cpp_field_order() {
    let bytes = CalendarRaidLockoutRemoved {
        instance_id: 9001,
        map_id: 631,
        difficulty_id: 4,
    }
    .to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarRaidLockoutRemoved as u16
    );
    assert_eq!(bytes.len(), 2 + 8 + 4 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint64().unwrap(), 9001);
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
}

#[test]
fn calendar_raid_lockout_updated_matches_cpp_field_order() {
    let bytes =
        CalendarRaidLockoutUpdated::new_at_unix(946_684_800, 631, 4, 3_600, 86_400).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarRaidLockoutUpdated as u16
    );
    assert_eq!(bytes.len(), 2 + 4 + 4 + 4 + 4 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0x0000_3000); // 2000-01-01 00:00 UTC
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert_eq!(pkt.read_int32().unwrap(), 3_600);
    assert_eq!(pkt.read_int32().unwrap(), 86_400);
}

#[test]
fn set_saved_instance_extend_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(631);
    pkt.write_uint32(4);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let query = SetSavedInstanceExtend::read(&mut pkt).unwrap();
    assert_eq!(query.map_id, 631);
    assert_eq!(query.difficulty_id, 4);
    assert!(query.extend);
}

#[test]
fn calendar_community_invite_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x0102_0304_0506_0708);
    pkt.write_uint8(10);
    pkt.write_uint8(70);
    pkt.write_uint8(3);

    let query = CalendarCommunityInvite::read(&mut pkt).unwrap();
    assert_eq!(query.club_id, 0x0102_0304_0506_0708);
    assert_eq!(query.min_level, 10);
    assert_eq!(query.max_level, 70);
    assert_eq!(query.max_rank_order, 3);
}

#[test]
fn calendar_get_event_reads_cpp_event_id() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x0102_0304_0506_0708);

    let query = CalendarGetEvent::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x0102_0304_0506_0708);
}

#[test]
fn calendar_remove_event_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint32(0xDEAD_BEEF);

    let query = CalendarRemoveEvent::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.moderator_id, 0x5555_6666_7777_8888);
    assert_eq!(query.club_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.flags, 0xDEAD_BEEF);
}

#[test]
fn calendar_copy_event_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint32(0xDEAD_BEEF);

    let query = CalendarCopyEvent::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.moderator_id, 0x5555_6666_7777_8888);
    assert_eq!(query.event_club_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.date, 0xDEAD_BEEF);
}

#[test]
fn calendar_remove_invite_reads_cpp_field_order() {
    let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint64(0xDEAD_BEEF_CAFE_BABE);

    let query = CalendarRemoveInvite::read(&mut pkt).unwrap();
    assert_eq!(query.guid, guid);
    assert_eq!(query.invite_id, 0x5555_6666_7777_8888);
    assert_eq!(query.moderator_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.event_id, 0xDEAD_BEEF_CAFE_BABE);
}

#[test]
fn calendar_event_sign_up_reads_cpp_field_order_and_tentative_bit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_bit(true);
    pkt.flush_bits();

    let query = CalendarEventSignUp::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.club_id, 0x5555_6666_7777_8888);
    assert!(query.tentative);
}

#[test]
fn calendar_invite_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_bits(4, 9);
    pkt.write_bit(false);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.write_string("Test");

    let query = CalendarInvite::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.moderator_id, 0x5555_6666_7777_8888);
    assert_eq!(query.club_id, 0x9999_AAAA_BBBB_CCCC);
    assert!(!query.creating);
    assert!(query.is_sign_up);
    assert_eq!(query.name, "Test");
}

#[test]
fn calendar_add_event_reads_cpp_field_order_with_invite_optionals() {
    let mut pkt = WorldPacket::new_empty();
    let invite_guid = ObjectGuid::new(0x0102_0304_0506_0708_i64, 0x1112_1314_1516_1718_i64);
    let optional_guid = ObjectGuid::new(0x2122_2324_2526_2728_i64, 0x3132_3334_3536_3738_i64);

    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint8(7);
    pkt.write_int32(-1234);
    pkt.write_uint32(0x0102_0304);
    pkt.write_uint32(0x0000_0440);
    pkt.write_uint32(1);
    pkt.write_bits(5, 8);
    pkt.write_bits(4, 11);
    pkt.write_guid(&invite_guid);
    pkt.write_uint8(3);
    pkt.write_uint8(2);
    pkt.write_bit(true);
    pkt.write_bit(true);
    pkt.write_bit(true);
    pkt.write_guid(&optional_guid);
    pkt.write_uint64(0x4142_4344_4546_4748);
    pkt.write_uint64(0x5152_5354_5556_5758);
    pkt.write_string("Title");
    pkt.write_string("Desc");
    pkt.write_uint32(99);

    let query = CalendarAddEvent::read(&mut pkt).unwrap();
    assert_eq!(query.club_id, 0x1111_2222_3333_4444);
    assert_eq!(query.event_type, 7);
    assert_eq!(query.texture_id, -1234);
    assert_eq!(query.time_packed, 0x0102_0304);
    assert_eq!(query.flags, 0x0000_0440);
    assert_eq!(query.title, "Title");
    assert_eq!(query.description, "Desc");
    assert_eq!(query.max_size, 99);
    assert_eq!(query.invites.len(), 1);
    assert_eq!(query.invites[0].guid, invite_guid);
    assert_eq!(query.invites[0].status, 3);
    assert_eq!(query.invites[0].moderator, 2);
    assert_eq!(query.invites[0].unused_801_1, Some(optional_guid));
    assert_eq!(query.invites[0].unused_801_2, Some(0x4142_4344_4546_4748));
    assert_eq!(query.invites[0].unused_801_3, Some(0x5152_5354_5556_5758));
}

#[test]
fn calendar_update_event_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint8(7);
    pkt.write_uint32(0x0102_0304);
    pkt.write_uint32(0x0506_0708);
    pkt.write_uint32(0x090A_0B0C);
    pkt.write_bits(5, 8);
    pkt.write_bits(4, 11);
    pkt.flush_bits();
    pkt.write_string("Title");
    pkt.write_string("Desc");
    pkt.write_uint32(99);

    let query = CalendarUpdateEvent::read(&mut pkt).unwrap();
    assert_eq!(query.club_id, 0x1111_2222_3333_4444);
    assert_eq!(query.event_id, 0x5555_6666_7777_8888);
    assert_eq!(query.moderator_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.event_type, 7);
    assert_eq!(query.texture_id, 0x0102_0304);
    assert_eq!(query.time_packed, 0x0506_0708);
    assert_eq!(query.flags, 0x090A_0B0C);
    assert_eq!(query.title, "Title");
    assert_eq!(query.description, "Desc");
    assert_eq!(query.max_size, 99);
}

#[test]
fn calendar_rsvp_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint8(9);

    let query = CalendarRsvp::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.invite_id, 0x5555_6666_7777_8888);
    assert_eq!(query.status, 9);
}

#[test]
fn calendar_status_reads_cpp_field_order() {
    let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint64(0xDEAD_BEEF_CAFE_BABE);
    pkt.write_uint8(9);

    let query = CalendarStatus::read(&mut pkt).unwrap();
    assert_eq!(query.guid, guid);
    assert_eq!(query.event_id, 0x5555_6666_7777_8888);
    assert_eq!(query.invite_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.moderator_id, 0xDEAD_BEEF_CAFE_BABE);
    assert_eq!(query.status, 9);
}

#[test]
fn calendar_moderator_status_query_reads_cpp_field_order() {
    let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint64(0xDEAD_BEEF_CAFE_BABE);
    pkt.write_uint8(9);

    let query = CalendarModeratorStatusQuery::read(&mut pkt).unwrap();
    assert_eq!(query.guid, guid);
    assert_eq!(query.event_id, 0x5555_6666_7777_8888);
    assert_eq!(query.invite_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.moderator_id, 0xDEAD_BEEF_CAFE_BABE);
    assert_eq!(query.status, 9);
}

#[test]
fn calendar_command_result_event_invalid_matches_cpp_shape() {
    let bytes = CalendarCommandResult::event_invalid_like_cpp().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );
    assert_eq!(bytes.len(), 2 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[test]
fn calendar_command_result_no_invite_matches_cpp_shape() {
    let bytes = CalendarCommandResult::no_invite_like_cpp().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );
    assert_eq!(bytes.len(), 2 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 29);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[test]
fn calendar_complain_reads_cpp_guid_event_invite_order() {
    let invited_by_guid = ObjectGuid::create_player(1, 0xAABB_CCDD);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&invited_by_guid);
    pkt.write_uint64(0x0102_0304_0506_0708);
    pkt.write_uint64(0x1112_1314_1516_1718);

    let complain = CalendarComplain::read(&mut pkt).unwrap();
    assert_eq!(complain.invited_by_guid, invited_by_guid);
    assert_eq!(complain.event_id, 0x0102_0304_0506_0708);
    assert_eq!(complain.invite_id, 0x1112_1314_1516_1718);
}

#[test]
fn arena_team_roster_reads_cpp_team_id() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x0102_0304);

    let request = ArenaTeamRoster::read(&mut pkt).unwrap();
    assert_eq!(request.team_id, 0x0102_0304);
}

#[test]
fn arena_team_decline_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    ArenaTeamDecline::read(&mut pkt).unwrap();
}

#[test]
fn arena_team_accept_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    ArenaTeamAccept::read(&mut pkt).unwrap();
}

#[test]
fn arena_team_leave_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    ArenaTeamLeave::read(&mut pkt).unwrap();
}

#[test]
fn arena_team_remove_reads_team_id_and_9bit_target_name_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x0102_0304);
    pkt.write_bits(7, 9);
    pkt.write_string("Playerx");
    pkt.reset_read();

    let request = ArenaTeamRemove::read(&mut pkt).unwrap();

    assert_eq!(request.team_id, 0x0102_0304);
    assert_eq!(request.target_name, "Playerx");
}

#[test]
fn arena_team_disband_reads_team_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x1122_3344);
    pkt.reset_read();

    let request = ArenaTeamDisband::read(&mut pkt).unwrap();

    assert_eq!(request.team_id, 0x1122_3344);
}

#[test]
fn arena_team_leader_reads_team_id_and_9bit_target_name_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x5566_7788);
    pkt.write_bits(6, 9);
    pkt.write_string("Leader");
    pkt.reset_read();

    let request = ArenaTeamLeader::read(&mut pkt).unwrap();

    assert_eq!(request.team_id, 0x5566_7788);
    assert_eq!(request.target_name, "Leader");
}

#[test]
fn query_arena_team_reads_team_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0xAABB_CCDD);
    pkt.reset_read();

    let request = QueryArenaTeam::read(&mut pkt).unwrap();

    assert_eq!(request.team_id, 0xAABB_CCDD);
}

#[test]
fn busy_trade_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    BusyTrade::read(&mut pkt).unwrap();
}

#[test]
fn begin_trade_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    BeginTrade::read(&mut pkt).unwrap();
}

#[test]
fn accept_trade_reads_state_index_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x1122_3344);
    pkt.reset_read();

    let packet = AcceptTrade::read(&mut pkt).unwrap();

    assert_eq!(packet.state_index, 0x1122_3344);
}

#[test]
fn clear_trade_item_reads_trade_slot_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(5);
    pkt.reset_read();

    let packet = ClearTradeItem::read(&mut pkt).unwrap();

    assert_eq!(packet.trade_slot, 5);
}

#[test]
fn set_trade_item_reads_slots_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(2);
    pkt.write_uint8(255);
    pkt.write_uint8(18);
    pkt.reset_read();

    let packet = SetTradeItem::read(&mut pkt).unwrap();

    assert_eq!(packet.trade_slot, 2);
    assert_eq!(packet.pack_slot, 255);
    assert_eq!(packet.item_slot_in_pack, 18);
}

#[test]
fn set_trade_spell_reads_spell_and_slots_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(7418);
    pkt.write_uint8(255);
    pkt.write_uint8(23);
    pkt.reset_read();

    let packet = SetTradeSpell::read(&mut pkt).unwrap();

    assert_eq!(packet.spell_id, 7418);
    assert_eq!(packet.pack_slot, 255);
    assert_eq!(packet.item_slot_in_pack, 23);
}

#[test]
fn sign_petition_reads_guid_and_choice_like_cpp() {
    let petition_guid = ObjectGuid::create_item(1, 0x0102_0304_0506_0708);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&petition_guid.to_raw_bytes());
    pkt.write_uint8(1);
    pkt.reset_read();

    let packet = SignPetition::read(&mut pkt).unwrap();

    assert_eq!(packet.petition_guid, petition_guid);
    assert_eq!(packet.choice, 1);
}

#[test]
fn decline_petition_reads_guid_like_cpp() {
    let petition_guid = ObjectGuid::create_item(1, 0x1112_1314_1516_1718);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&petition_guid.to_raw_bytes());
    pkt.reset_read();

    let packet = DeclinePetition::read(&mut pkt).unwrap();

    assert_eq!(packet.petition_guid, petition_guid);
}

#[test]
fn query_petition_reads_id_then_guid_like_cpp() {
    let item_guid = ObjectGuid::create_item(1, 0x2122_2324_2526_2728);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x1122_3344);
    pkt.write_bytes(&item_guid.to_raw_bytes());
    pkt.reset_read();

    let packet = QueryPetition::read(&mut pkt).unwrap();

    assert_eq!(packet.petition_id, 0x1122_3344);
    assert_eq!(packet.item_guid, item_guid);
}

#[test]
fn query_petition_not_found_response_writes_id_and_allow_false_like_cpp() {
    let item_guid = ObjectGuid::create_item(1, 0x3132_3334_3536_3738);
    let bytes = QueryPetitionResponse::not_found_like_cpp(item_guid).to_bytes();
    let mut body = WorldPacket::from_bytes(&bytes);

    assert_eq!(
        body.server_opcode(),
        Some(ServerOpcodes::QueryPetitionResponse)
    );
    assert_eq!(
        body.read_uint16().unwrap(),
        ServerOpcodes::QueryPetitionResponse as u16
    );
    assert_eq!(body.read_uint32().unwrap(), item_guid.counter() as u32);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[test]
fn set_trade_gold_reads_coinage_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1122_3344_5566_7788);
    pkt.reset_read();

    let packet = SetTradeGold::read(&mut pkt).unwrap();

    assert_eq!(packet.coinage, 0x1122_3344_5566_7788);
}

#[test]
fn unaccept_trade_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    UnacceptTrade::read(&mut pkt).unwrap();
}

#[test]
fn ignore_trade_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    IgnoreTrade::read(&mut pkt).unwrap();
}

#[test]
fn trade_status_player_busy_writes_cancel_status_bits_like_cpp() {
    let bytes = TradeStatus::cancel_like_cpp(TRADE_STATUS_PLAYER_BUSY_LIKE_CPP).to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes.len(), 3);
    assert_eq!(bytes[2], TRADE_STATUS_PLAYER_BUSY_LIKE_CPP << 1);
}

#[test]
fn trade_status_initiated_writes_id_payload_like_cpp() {
    let bytes = TradeStatus::initiated_like_cpp(0x1122_3344).to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes.len(), 7);
    assert_eq!(bytes[2], TRADE_STATUS_INITIATED_LIKE_CPP << 2);
    assert_eq!(
        u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        0x1122_3344
    );
}

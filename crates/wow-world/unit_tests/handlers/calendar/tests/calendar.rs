// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Calendar handler behavior tests.

use super::*;

#[tokio::test]
async fn calendar_get_num_pending_sends_zero_pending_like_cpp_without_calendar_mgr() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_get_num_pending(WorldPacket::new_empty())
        .await;

    let bytes = send_rx.try_recv().expect("calendar pending packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarSendNumPending as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[tokio::test]
async fn calendar_complain_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let invited_by_guid = ObjectGuid::create_player(1, 0xAABB_CCDD);

    session
        .handle_calendar_complain(CalendarComplain {
            invited_by_guid,
            event_id: 0x0102_0304_0506_0708,
            invite_id: 0x1112_1314_1516_1718,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn calendar_get_sends_empty_calendar_like_cpp_without_calendar_mgr() {
    let (mut session, send_rx) = make_session();

    session.handle_calendar_get(WorldPacket::new_empty()).await;

    let bytes = send_rx.try_recv().expect("calendar send calendar packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarSendCalendar as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let _server_time = pkt.read_uint32().unwrap();
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Invites.Count
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Events.Count
    assert_eq!(pkt.read_uint32().unwrap(), 0); // RaidLockouts.Count
}

#[tokio::test]
async fn calendar_get_event_without_calendar_mgr_sends_event_invalid_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_get_event(CalendarGetEvent {
            event_id: 0x0102_0304_0506_0708,
        })
        .await;

    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_copy_event_without_calendar_mgr_sends_event_invalid_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_copy_event(CalendarCopyEvent {
            event_id: 0x1111_2222_3333_4444,
            moderator_id: 0x5555_6666_7777_8888,
            event_club_id: 0x9999_AAAA_BBBB_CCCC,
            date: 0xDEAD_BEEF,
        })
        .await;

    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_event_sign_up_without_calendar_mgr_sends_event_invalid_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_event_sign_up(CalendarEventSignUp {
            event_id: 0x1111_2222_3333_4444,
            club_id: 0x5555_6666_7777_8888,
            tentative: true,
        })
        .await;

    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_invite_existing_event_without_calendar_mgr_sends_event_invalid_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_invite(CalendarInvite {
            event_id: 0x1111_2222_3333_4444,
            moderator_id: 0x5555_6666_7777_8888,
            club_id: 0x9999_AAAA_BBBB_CCCC,
            creating: false,
            is_sign_up: true,
            name: "Invitee".to_string(),
        })
        .await;

    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_update_event_without_calendar_mgr_sends_event_invalid_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_update_event(CalendarUpdateEvent {
            club_id: 0x1111_2222_3333_4444,
            event_id: 0x5555_6666_7777_8888,
            moderator_id: 0x9999_AAAA_BBBB_CCCC,
            event_type: 7,
            texture_id: 0x0102_0304,
            time_packed: 0x0506_0708,
            flags: 0x090A_0B0C,
            title: "Title".to_string(),
            description: "Desc".to_string(),
            max_size: 99,
        })
        .await;

    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_remove_invite_without_calendar_mgr_sends_no_invite_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_remove_invite(CalendarRemoveInvite {
            guid: ObjectGuid::create_player(1, 0x1111_2222),
            invite_id: 0x3333_4444_5555_6666,
            moderator_id: 0x7777_8888_9999_AAAA,
            event_id: 0xBBBB_CCCC_DDDD_EEEE,
        })
        .await;

    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 29);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_rsvp_without_calendar_mgr_sends_event_invalid_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_rsvp(CalendarRsvp {
            event_id: 0x1111_2222_3333_4444,
            invite_id: 0x5555_6666_7777_8888,
            status: 9,
        })
        .await;

    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_moderator_status_without_calendar_mgr_sends_event_invalid_like_cpp() {
    let (mut session, send_rx) = make_session();
    let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);

    session
        .handle_calendar_moderator_status(CalendarModeratorStatusQuery {
            guid,
            event_id: 0x5555_6666_7777_8888,
            invite_id: 0x9999_AAAA_BBBB_CCCC,
            moderator_id: 0xDEAD_BEEF_CAFE_BABE,
            status: 9,
        })
        .await;

    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_status_without_calendar_mgr_sends_event_invalid_like_cpp() {
    let (mut session, send_rx) = make_session();
    let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);

    session
        .handle_calendar_status(CalendarStatus {
            guid,
            event_id: 0x5555_6666_7777_8888,
            invite_id: 0x9999_AAAA_BBBB_CCCC,
            moderator_id: 0xDEAD_BEEF_CAFE_BABE,
            status: 9,
        })
        .await;

    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_community_invite_without_guild_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_guild_id_like_cpp(0);

    session
        .handle_calendar_community_invite(CalendarCommunityInvite {
            club_id: 0x0102_0304_0506_0708,
            min_level: 10,
            max_level: 70,
            max_rank_order: 3,
        })
        .await;

    assert!(
        session
            .represented_calendar_community_invites_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn calendar_community_invite_records_represented_guild_mass_invite_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_guild_id_like_cpp(42);

    session
        .handle_calendar_community_invite(CalendarCommunityInvite {
            club_id: 0x0102_0304_0506_0708,
            min_level: 10,
            max_level: 70,
            max_rank_order: 3,
        })
        .await;

    assert_eq!(
        session.represented_calendar_community_invites_like_cpp(),
        &[crate::session::RepresentedCalendarCommunityInviteLikeCpp {
            guild_id: 42,
            min_level: 10,
            max_level: 70,
            max_rank_order: 3,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn calendar_add_event_guild_scoped_without_guild_sends_not_in_guild_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_guild_id_like_cpp(0);

    session
        .handle_calendar_add_event(CalendarAddEvent {
            club_id: 0x1111_2222_3333_4444,
            event_type: 7,
            texture_id: -1234,
            time_packed: 0x0102_0304,
            flags: 0x0000_0400,
            invites: Vec::new(),
            title: "Title".to_string(),
            description: "Desc".to_string(),
            max_size: 99,
        })
        .await;

    assert!(
        session
            .represented_calendar_add_events_like_cpp()
            .is_empty()
    );
    let bytes = send_rx.try_recv().expect("calendar command result");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 9);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[tokio::test]
async fn calendar_add_event_records_represented_creation_intent_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_add_event(CalendarAddEvent {
            club_id: 0x1111_2222_3333_4444,
            event_type: 7,
            texture_id: -1234,
            time_packed: 0x0102_0304,
            flags: 0,
            invites: Vec::new(),
            title: "Title".to_string(),
            description: "Desc".to_string(),
            max_size: 99,
        })
        .await;

    assert_eq!(
        session.represented_calendar_add_events_like_cpp(),
        &[crate::session::RepresentedCalendarAddEventLikeCpp {
            guild_id: None,
            club_id: 0x1111_2222_3333_4444,
            event_type: 7,
            texture_id: -1234,
            time_packed: 0x0102_0304,
            flags: 0,
            invite_count: 0,
            title: "Title".to_string(),
            description: "Desc".to_string(),
            max_size: 99,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn calendar_remove_event_records_represented_remove_request_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_calendar_remove_event(CalendarRemoveEvent {
            event_id: 0x1111_2222_3333_4444,
            moderator_id: 0x5555_6666_7777_8888,
            club_id: 0x9999_AAAA_BBBB_CCCC,
            flags: 0xDEAD_BEEF,
        })
        .await;

    assert_eq!(
        session.represented_calendar_remove_events_like_cpp(),
        &[crate::session::RepresentedCalendarRemoveEventLikeCpp {
            event_id: 0x1111_2222_3333_4444,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

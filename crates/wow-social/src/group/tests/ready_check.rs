// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

use super::*;

#[test]
fn ready_check_start_marks_offline_starter_and_preserves_cpp_event_order() {
    let leader = ObjectGuid::create_player(1, 42);
    let offline = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(offline);

    let events = group.start_ready_check_like_cpp(leader, [leader]);

    assert_eq!(group.ready_check_timer_ms, 0);
    assert!(!group.ready_check_started);
    assert!(group.member_slots.iter().all(|slot| !slot.ready_checked));
    assert_eq!(
        events,
        vec![
            ReadyCheckEventLikeCpp::Response {
                party_guid: group.group_guid,
                player: offline,
                is_ready: false,
            },
            ReadyCheckEventLikeCpp::Completed {
                party_index: 0,
                party_guid: group.group_guid,
            },
            ReadyCheckEventLikeCpp::Started {
                party_index: 0,
                party_guid: group.group_guid,
                initiator_guid: leader,
                duration_ms: READYCHECK_DURATION_MS_LIKE_CPP,
            },
        ]
    );
}

#[test]
fn ready_check_response_before_started_is_cpp_noop() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);

    let events = group.set_member_ready_check_like_cpp(member, true);

    assert!(events.is_empty());
    assert!(!group.member_slot_like_cpp(member).unwrap().ready_checked);
    assert!(!group.ready_check_started);
}

#[test]
fn ready_check_member_response_broadcasts_and_completes_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let start_events = group.start_ready_check_like_cpp(leader, [leader, member]);

    assert_eq!(start_events.len(), 1);
    assert!(group.ready_check_started);
    assert!(group.member_slot_like_cpp(leader).unwrap().ready_checked);
    assert!(!group.member_slot_like_cpp(member).unwrap().ready_checked);

    let events = group.set_member_ready_check_like_cpp(member, true);

    assert_eq!(
        events,
        vec![
            ReadyCheckEventLikeCpp::Response {
                party_guid: group.group_guid,
                player: member,
                is_ready: true,
            },
            ReadyCheckEventLikeCpp::Completed {
                party_index: 0,
                party_guid: group.group_guid,
            },
        ]
    );
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
    assert!(group.member_slots.iter().all(|slot| !slot.ready_checked));
}

// ── Ready-check tick tests ──────────────────────────────────────────

#[test]
fn update_ready_check_noop_when_not_started() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::new(leader);
    assert!(!group.ready_check_started);

    let events = group.update_ready_check_like_cpp(500);
    assert!(events.is_empty());
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
}

#[test]
fn update_ready_check_decrements_without_completing_when_time_remains() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::new(leader);
    group.ready_check_started = true;
    group.ready_check_timer_ms = READYCHECK_DURATION_MS_LIKE_CPP;

    // Tick 1000ms — timer should go from 35000 to 34000, no events.
    let events = group.update_ready_check_like_cpp(1_000);
    assert!(events.is_empty());
    assert!(group.ready_check_started);
    assert_eq!(
        group.ready_check_timer_ms,
        READYCHECK_DURATION_MS_LIKE_CPP - 1_000
    );

    // Tick another 1000ms
    let events = group.update_ready_check_like_cpp(1_000);
    assert!(events.is_empty());
    assert!(group.ready_check_started);
    assert_eq!(
        group.ready_check_timer_ms,
        READYCHECK_DURATION_MS_LIKE_CPP - 2_000
    );
}

#[test]
fn update_ready_check_expires_and_resets_all_state() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 99);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.ready_check_started = true;
    group.ready_check_timer_ms = READYCHECK_DURATION_MS_LIKE_CPP;
    // Simulate some members already responded.
    for slot in &mut group.member_slots {
        slot.ready_checked = true;
    }

    // Tick more than remaining — should expire.
    let events = group.update_ready_check_like_cpp(36_000);
    assert_eq!(events.len(), 1);
    match events[0] {
        ReadyCheckEventLikeCpp::Completed {
            party_index,
            party_guid,
        } => {
            assert_eq!(party_index, 0);
            assert_eq!(party_guid, group.group_guid);
        }
        _ => panic!("expected Completed event"),
    }
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
    // All members should have been reset.
    assert!(group.member_slots.iter().all(|s| !s.ready_checked));
}

#[test]
fn update_ready_check_exact_zero_expires() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::new(leader);
    group.ready_check_started = true;
    group.ready_check_timer_ms = 500;

    let events = group.update_ready_check_like_cpp(500);
    assert_eq!(events.len(), 1);
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
}

#[test]
fn registry_stale_ready_response_cannot_reopen_completed_check() {
    let registry = GroupRegistry::new();
    let leader = ObjectGuid::create_player(1, 91);
    let member = ObjectGuid::create_player(1, 92);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    registry.register_group_like_cpp(group_guid, group);

    registry
        .start_ready_check_transition_like_cpp(group_guid, leader, [leader, member])
        .expect("leader starts ready check");
    registry
        .respond_ready_check_transition_like_cpp(group_guid, member, true)
        .expect("final member completes ready check");
    let stale = registry.respond_ready_check_transition_like_cpp(group_guid, member, false);

    assert!(matches!(stale, Err(GroupAuthorityErrorLikeCpp::NoChange)));
    let group = registry.get(&group_guid).unwrap();
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
    assert!(group.member_slots.iter().all(|slot| !slot.ready_checked));
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group ready-check state machine.

use wow_core::ObjectGuid;

use super::*;

pub const READYCHECK_DURATION_MS_LIKE_CPP: i64 = 35_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadyCheckEventLikeCpp {
    Started {
        party_index: u8,
        party_guid: u64,
        initiator_guid: ObjectGuid,
        duration_ms: i64,
    },
    Response {
        party_guid: u64,
        player: ObjectGuid,
        is_ready: bool,
    },
    Completed {
        party_index: u8,
        party_guid: u64,
    },
}

/// C++ `Group::UpdateReadyCheck` fanout: ticks every active group's
/// ready-check timer and collects expired `Completed` events without
/// holding any lock during packet fanout.
///
/// Returns `(group_guid, events)` for groups whose ready check expired this
/// tick. Caller is responsible for broadcasting the events to connected
/// players via `PlayerRegistry`.
pub fn tick_all_group_ready_checks_like_cpp(
    registry: &GroupRegistry,
    diff_ms: u32,
) -> Vec<(u64, Vec<ReadyCheckEventLikeCpp>)> {
    registry.tick_ready_checks_like_cpp(diff_ms)
}

impl GroupInfo {
    pub fn reset_member_ready_checked_like_cpp(&mut self) {
        for slot in &mut self.member_slots {
            slot.ready_checked = false;
        }
    }

    pub fn is_ready_check_completed_like_cpp(&self) -> bool {
        self.member_slots.iter().all(|slot| slot.ready_checked)
    }

    fn end_ready_check_like_cpp(&mut self, events: &mut Vec<ReadyCheckEventLikeCpp>) {
        if !self.ready_check_started {
            return;
        }

        self.ready_check_started = false;
        self.ready_check_timer_ms = 0;
        self.reset_member_ready_checked_like_cpp();
        events.push(ReadyCheckEventLikeCpp::Completed {
            party_index: 0,
            party_guid: self.group_guid,
        });
    }

    /// C++ `Group::UpdateReadyCheck(uint32 diff)` at Group.cpp:1445-1453.
    ///
    /// NOOP when no ready check is active. Otherwise subtracts `diff_ms` from
    /// the timer and, if it has expired (<= 0), calls `end_ready_check_like_cpp`
    /// which resets all state and emits exactly one `Completed` event.
    pub fn update_ready_check_like_cpp(&mut self, diff_ms: u32) -> Vec<ReadyCheckEventLikeCpp> {
        if !self.ready_check_started {
            return Vec::new();
        }

        self.ready_check_timer_ms -= i64::from(diff_ms);
        if self.ready_check_timer_ms <= 0 {
            let mut events = Vec::new();
            self.end_ready_check_like_cpp(&mut events);
            events
        } else {
            Vec::new()
        }
    }

    fn set_member_ready_checked_like_cpp(
        &mut self,
        slot_index: usize,
        events: &mut Vec<ReadyCheckEventLikeCpp>,
    ) {
        self.member_slots[slot_index].ready_checked = true;
        if self.is_ready_check_completed_like_cpp() {
            self.end_ready_check_like_cpp(events);
        }
    }

    fn set_member_ready_check_slot_like_cpp(
        &mut self,
        slot_index: usize,
        ready: bool,
        events: &mut Vec<ReadyCheckEventLikeCpp>,
    ) {
        let player = self.member_slots[slot_index].guid;
        events.push(ReadyCheckEventLikeCpp::Response {
            party_guid: self.group_guid,
            player,
            is_ready: ready,
        });
        self.set_member_ready_checked_like_cpp(slot_index, events);
    }

    pub fn start_ready_check_like_cpp(
        &mut self,
        starter_guid: ObjectGuid,
        connected_members: impl IntoIterator<Item = ObjectGuid>,
    ) -> Vec<ReadyCheckEventLikeCpp> {
        let mut events = Vec::new();
        if self.ready_check_started {
            return events;
        }

        let Some(starter_index) = self
            .member_slots
            .iter()
            .position(|slot| slot.guid == starter_guid)
        else {
            return events;
        };

        self.ready_check_started = true;
        self.ready_check_timer_ms = READYCHECK_DURATION_MS_LIKE_CPP;

        let connected: Vec<ObjectGuid> = connected_members.into_iter().collect();
        let offline_indices: Vec<usize> = self
            .member_slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| (!connected.contains(&slot.guid)).then_some(index))
            .collect();
        for index in offline_indices {
            if self.ready_check_started {
                self.set_member_ready_check_slot_like_cpp(index, false, &mut events);
            }
        }

        if self.ready_check_started {
            self.set_member_ready_checked_like_cpp(starter_index, &mut events);
        }

        events.push(ReadyCheckEventLikeCpp::Started {
            party_index: 0,
            party_guid: self.group_guid,
            initiator_guid: starter_guid,
            duration_ms: READYCHECK_DURATION_MS_LIKE_CPP,
        });
        events
    }

    pub fn set_member_ready_check_like_cpp(
        &mut self,
        guid: ObjectGuid,
        ready: bool,
    ) -> Vec<ReadyCheckEventLikeCpp> {
        let mut events = Vec::new();
        if !self.ready_check_started {
            return events;
        }

        if let Some(slot_index) = self.member_slots.iter().position(|slot| slot.guid == guid) {
            self.set_member_ready_check_slot_like_cpp(slot_index, ready, &mut events);
        }

        events
    }
}

impl GroupRegistry {
    pub fn start_ready_check_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        connected_members: impl IntoIterator<Item = ObjectGuid>,
    ) -> Result<
        GroupTransitionOutcomeLikeCpp<Vec<ReadyCheckEventLikeCpp>>,
        GroupAuthorityErrorLikeCpp,
    > {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) && !group.is_assistant_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant);
        }
        let facts = group.start_ready_check_like_cpp(actor_guid, connected_members);
        if facts.is_empty() {
            return Err(GroupAuthorityErrorLikeCpp::NoChange);
        }
        Ok(GroupTransitionOutcomeLikeCpp {
            persistence: Vec::new(),
            group: group.clone(),
            facts,
        })
    }

    pub fn respond_ready_check_transition_like_cpp(
        &self,
        group_guid: u64,
        member_guid: ObjectGuid,
        ready: bool,
    ) -> Result<
        GroupTransitionOutcomeLikeCpp<Vec<ReadyCheckEventLikeCpp>>,
        GroupAuthorityErrorLikeCpp,
    > {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.members.contains(&member_guid) {
            return Err(GroupAuthorityErrorLikeCpp::MissingMember);
        }
        let facts = group.set_member_ready_check_like_cpp(member_guid, ready);
        if facts.is_empty() {
            return Err(GroupAuthorityErrorLikeCpp::NoChange);
        }
        Ok(GroupTransitionOutcomeLikeCpp {
            persistence: Vec::new(),
            group: group.clone(),
            facts,
        })
    }

    pub fn tick_ready_checks_like_cpp(
        &self,
        diff_ms: u32,
    ) -> Vec<(u64, Vec<ReadyCheckEventLikeCpp>)> {
        let active_keys: Vec<u64> = self
            .groups
            .iter()
            .filter(|entry| entry.value().ready_check_started)
            .map(|entry| *entry.key())
            .collect();
        let mut results = Vec::new();
        for group_guid in active_keys {
            if let Some(mut group) = self.groups.get_mut(&group_guid) {
                let events = group.update_ready_check_like_cpp(diff_ms);
                if !events.is_empty() {
                    results.push((group_guid, events));
                }
            }
        }
        results
    }
}

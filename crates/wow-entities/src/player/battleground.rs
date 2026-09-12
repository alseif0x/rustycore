// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player battleground state.
//!
//! C++ keeps this in `Player::m_bgData` (`Player.h:2821`, the `BGData` struct
//! at `:976`): the instance the Player was teleported into (`bgInstanceID`),
//! the type (`bgTypeID`), the side they joined (`bgTeam`) and the queue
//! (`queueId`), read back through `InBattleground` (`:2335`),
//! `GetBattlegroundId` (`:2337`) and `GetBattlegroundTypeId` (`:2338`);
//! `Player::m_ArenaTeamIdInvited` sits beside it with `SetArenaTeamIdInvited`
//! (`:1956`).
//!
//! Separated from `player/mod.rs` under #783, which also closed the members.
//! The queue policy, the packets and the battlemaster catalog stay in
//! `wow-world`.
//!
//! Two things here are represented state, recorded rather than presented as C++
//! members: the battleground status is RustyCore's stand-in for
//! `Battleground::GetStatus()` until a live `Battleground` owner exists, and the
//! map id is the entry gate's copy of the battleground map. Both retire with
//! that live ownership, not here.

use super::{
    PlayerBattlegroundQueueRecord, PlayerBattlegroundQueueSlotLikeCpp,
    PlayerRandomBattlegroundState,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerBattlegroundState {
    pub(super) queues: Vec<PlayerBattlegroundQueueRecord>,
    pub(super) current_bg_instance_id: Option<u32>,
    pub(super) current_bg_team: Option<u32>,
    pub(super) random: PlayerRandomBattlegroundState,
    /// Represented C++ `Player::m_bgData.bgTypeID`.
    pub(super) represented_type_id: Option<u32>,
    /// Represented current battleground map/instance map used by teleport leave gates.
    pub(super) represented_map_id: Option<u32>,
    /// Represented `Battleground::GetStatus()` until live Battleground ownership exists.
    pub(super) represented_status: Option<u8>,
    /// C++ `Player::m_bgData.bgBattlegroundQueueID` slots.
    pub(super) represented_queue_slots: Vec<PlayerBattlegroundQueueSlotLikeCpp>,
    /// C++ `Player::m_ArenaTeamIdInvited`.
    pub(super) arena_team_id_invited: u32,
}

impl PlayerBattlegroundState {
    // ---- reads -------------------------------------------------------------

    /// C++ `Player::GetBattlegroundTypeId` (`Player.h:2338`) over
    /// `m_bgData.bgTypeID`; `None` is `BATTLEGROUND_TYPE_NONE`.
    #[must_use]
    pub fn battleground_type_id_like_cpp(&self) -> Option<u32> {
        self.represented_type_id
    }

    /// C++ `Player::InBattleground` (`Player.h:2335`), which asks whether the
    /// Player is in one at all.
    #[must_use]
    pub fn in_battleground_like_cpp(&self) -> bool {
        self.represented_type_id.is_some()
    }

    /// The battleground map the entry gate recorded.
    #[must_use]
    pub fn battleground_map_id_like_cpp(&self) -> Option<u32> {
        self.represented_map_id
    }

    /// The represented `Battleground::GetStatus()` value.
    #[must_use]
    pub fn battleground_status_like_cpp(&self) -> Option<u8> {
        self.represented_status
    }

    /// The queue slots C++ keeps in `m_bgData.bgBattlegroundQueueID`.
    #[must_use]
    pub fn queue_slots_like_cpp(&self) -> &[PlayerBattlegroundQueueSlotLikeCpp] {
        &self.represented_queue_slots
    }

    /// Whether the Player holds no queue slot at all.
    #[must_use]
    pub fn has_no_queue_slot_like_cpp(&self) -> bool {
        self.represented_queue_slots.is_empty()
    }

    /// C++ `Player::m_ArenaTeamIdInvited`.
    #[must_use]
    pub fn arena_team_id_invited_like_cpp(&self) -> u32 {
        self.arena_team_id_invited
    }

    /// Rebuild the battleground state from one represented payload, for the
    /// handle-less mirror that has no Player to borrow. The type, map and
    /// status describe one battleground and travel together, as do the queue
    /// slots and the arena invitation beside them.
    #[must_use]
    pub fn from_represented_parts_like_cpp(
        represented_type_id: Option<u32>,
        represented_map_id: Option<u32>,
        represented_status: Option<u8>,
        represented_queue_slots: Vec<PlayerBattlegroundQueueSlotLikeCpp>,
        arena_team_id_invited: u32,
    ) -> Self {
        Self {
            represented_type_id,
            represented_map_id,
            represented_status,
            represented_queue_slots,
            arena_team_id_invited,
            ..Self::default()
        }
    }

    // ---- transitions -------------------------------------------------------

    /// Record the battleground type the Player belongs to, as C++ assigns
    /// `m_bgData.bgTypeID`. A zero type is `BATTLEGROUND_TYPE_NONE` and clears
    /// it.
    pub fn set_battleground_type_id_like_cpp(&mut self, bg_type_id: u32) {
        self.represented_type_id = (bg_type_id != 0).then_some(bg_type_id);
    }

    /// Record the battleground type and its map together, as the entry gate
    /// establishes both at once.
    pub fn set_battleground_context_like_cpp(&mut self, bg_type_id: u32, bg_map_id: u32) {
        self.set_battleground_type_id_like_cpp(bg_type_id);
        self.represented_map_id = (bg_map_id != 0).then_some(bg_map_id);
    }

    /// Record the represented `Battleground::GetStatus()` value.
    pub fn set_battleground_status_like_cpp(&mut self, status: Option<u8>) {
        self.represented_status = status;
    }

    /// Install one queue slot, replacing the slot of the same index rather than
    /// adding a second row for it: C++ indexes
    /// `m_bgData.bgBattlegroundQueueID` by slot, so one slot holds one queue.
    pub fn install_queue_slot_like_cpp(&mut self, slot: PlayerBattlegroundQueueSlotLikeCpp) {
        if let Some(existing) = self
            .represented_queue_slots
            .iter_mut()
            .find(|queued| queued.slot == slot.slot)
        {
            *existing = slot;
        } else {
            self.represented_queue_slots.push(slot);
        }
    }

    /// C++ `Player::SetArenaTeamIdInvited` (`Player.h:1956`).
    pub fn set_arena_team_id_invited_like_cpp(&mut self, arena_team_id: u32) {
        self.arena_team_id_invited = arena_team_id;
    }
}

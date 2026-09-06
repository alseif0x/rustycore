// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Calendar packet registrations and the represented CalendarMgr boundary.

use super::{
    CalendarAddEvent, CalendarCommandResult, CalendarCommunityInvite, CalendarComplain,
    CalendarCopyEvent, CalendarEventSignUp, CalendarGetEvent, CalendarInvite,
    CalendarModeratorStatusQuery, CalendarRemoveEvent, CalendarRemoveInvite, CalendarRsvp,
    CalendarSendCalendar, CalendarSendNumPending, CalendarStatus, CalendarUpdateEvent,
    ClientOpcodes, PacketHandlerEntry, PacketProcessing, SessionStatus,
};
use wow_packet::ClientPacket;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarGetNumPending,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_get_num_pending",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_calendar_get_num_pending(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarComplain,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_complain",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarComplain::read(&mut pkt) {
                    Ok(complain) => session.handle_calendar_complain(complain).await,
                    Err(e) => tracing::warn!("Failed to read CalendarComplain: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarCommunityInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_community_invite",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarCommunityInvite::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_community_invite(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarCommunityInvite: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarAddEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_add_event",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarAddEvent::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_add_event(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarAddEvent: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarGet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_get",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_calendar_get(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarGetEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_get_event",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarGetEvent::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_get_event(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarGetEvent: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarCopyEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_copy_event",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarCopyEvent::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_copy_event(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarCopyEvent: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarEventSignUp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_event_sign_up",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarEventSignUp::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_event_sign_up(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarEventSignUp: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_invite",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarInvite::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_invite(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarInvite: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarUpdateEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_update_event",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarUpdateEvent::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_update_event(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarUpdateEvent: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarRemoveEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_remove_event",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarRemoveEvent::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_remove_event(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarRemoveEvent: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarRemoveInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_remove_invite",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarRemoveInvite::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_remove_invite(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarRemoveInvite: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarRsvp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_rsvp",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarRsvp::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_rsvp(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarRsvp: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarModeratorStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_moderator_status",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarModeratorStatusQuery::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_moderator_status(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarModeratorStatusQuery: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_status",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::CalendarStatus::read(&mut pkt) {
                    Ok(query) => session.handle_calendar_status(query).await,
                    Err(e) => tracing::warn!("Failed to read CalendarStatus: {e}"),
                }
            })
        },
    }
}

impl crate::session::WorldSession {
    pub async fn handle_calendar_get_num_pending(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ reads `sCalendarMgr->GetPlayerNumPending(playerGuid)` and sends
        // CalendarSendNumPending. Calendar manager state is not ported yet, so
        // represent the empty pending-invite count.
        self.send_packet_realm(&CalendarSendNumPending { num_pending: 0 });
    }
    pub async fn handle_calendar_complain(&mut self, _complain: CalendarComplain) {
        // C++ only parses/logs this packet and has no gameplay side effect.
    }
    pub async fn handle_calendar_get(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ fills CalendarSendCalendar from sCalendarMgr and instance locks.
        // Those live managers are not ported here yet, so represent the
        // well-defined empty calendar/lockout lists with current server time.
        self.send_packet(&CalendarSendCalendar::empty_now());
    }

    pub async fn handle_calendar_community_invite(&mut self, query: CalendarCommunityInvite) {
        // C++ reads ClubID but does not use it in this handler. It only calls
        // Guild::MassInviteToEvent if the player's guild resolves.
        self.calendar_community_invite_like_cpp(
            query.min_level,
            query.max_level,
            query.max_rank_order,
        );
    }

    pub async fn handle_calendar_add_event(&mut self, query: CalendarAddEvent) {
        // C++ rejects guild-scoped events before allocating CalendarMgr state.
        // Rust only has represented guild membership here, so this captures that
        // observable branch and records otherwise-accepted creation intent.
        let accepted = self.calendar_add_event_like_cpp(
            query.club_id,
            query.event_type,
            query.texture_id,
            query.time_packed,
            query.flags,
            query.invites.len(),
            query.title,
            query.description,
            query.max_size,
        );
        if !accepted {
            self.send_packet(&CalendarCommandResult::with_result_like_cpp(
                CalendarCommandResult::ERROR_GUILD_PLAYER_NOT_IN_GUILD_LIKE_CPP,
            ));
        }
    }

    pub async fn handle_calendar_get_event(&mut self, _query: CalendarGetEvent) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_copy_event(&mut self, _query: CalendarCopyEvent) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no source event for the requested id. Rust does not have CalendarMgr
        // wired yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_event_sign_up(&mut self, _query: CalendarEventSignUp) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_invite(&mut self, query: CalendarInvite) {
        // C++ only consults CalendarMgr for an existing event when Creating is
        // false. Rust does not have CalendarMgr wired yet, so this captures the
        // observable no-event branch without inventing name/cache/guild logic.
        if !query.creating {
            self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
        }
    }

    pub async fn handle_calendar_update_event(&mut self, _query: CalendarUpdateEvent) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_remove_event(&mut self, query: CalendarRemoveEvent) {
        // C++ delegates only EventID and the player GUID to CalendarMgr.
        // CalendarMgr is not live here yet, so capture the represented request.
        self.calendar_remove_event_like_cpp(query.event_id);
    }

    pub async fn handle_calendar_remove_invite(&mut self, _query: CalendarRemoveInvite) {
        // C++ sends CalendarCommandResult(NO_INVITE) when sCalendarMgr has no
        // event for the requested id. Rust does not have CalendarMgr wired yet,
        // so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::no_invite_like_cpp());
    }

    pub async fn handle_calendar_rsvp(&mut self, _query: CalendarRsvp) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_moderator_status(&mut self, _query: CalendarModeratorStatusQuery) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_status(&mut self, _query: CalendarStatus) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }
}

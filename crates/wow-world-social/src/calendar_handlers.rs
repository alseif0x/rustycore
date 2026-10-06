// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Calendar command packet handlers.
//!
//! C++ source of truth: `WorldSession::HandleCalendar*` in
//! `src/server/game/Handlers/CalendarHandler.cpp` and the represented
//! `CalendarMgr` boundary those handlers currently observe. The family owns the
//! packet bodies, the represented guild-scope checks and the fixture recording;
//! the World session only builds the borrowed context from its social state and
//! the mutable hub (#1263 F5).

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::misc::{
    CalendarAddEvent, CalendarCommandResult, CalendarCommunityInvite, CalendarComplain,
    CalendarCopyEvent, CalendarEventSignUp, CalendarGetEvent, CalendarInvite,
    CalendarModeratorStatusQuery, CalendarRemoveEvent, CalendarRemoveInvite, CalendarRsvp,
    CalendarSendCalendar, CalendarSendNumPending, CalendarStatus, CalendarUpdateEvent,
};
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

use crate::SessionSocialLimits;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::{RepresentedCalendarAddEventLikeCpp, RepresentedCalendarCommunityInviteLikeCpp};

/// Borrowed inputs of one calendar handler invocation.
pub struct CalendarHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    social: &'a mut SessionSocialLimits,
}

impl<'a> CalendarHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>, social: &'a mut SessionSocialLimits) -> Self {
        Self { hub, social }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    fn calendar_community_invite_like_cpp(
        &mut self,
        min_level: u8,
        max_level: u8,
        max_rank_order: u8,
    ) -> bool {
        let Some(guild_id) =
            crate::resolved_represented_guild_id_like_cpp(&self.hub.shared(), self.social)
        else {
            return false;
        };
        if guild_id == 0 {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        self.social
            .record_calendar_community_invite_for_test_like_cpp(
                RepresentedCalendarCommunityInviteLikeCpp {
                    guild_id,
                    min_level,
                    max_level,
                    max_rank_order,
                },
            );
        true
    }

    fn calendar_add_event_like_cpp(
        &mut self,
        club_id: u64,
        event_type: u8,
        texture_id: i32,
        time_packed: u32,
        flags: u32,
        invite_count: usize,
        title: String,
        description: String,
        max_size: u32,
    ) -> bool {
        const CALENDAR_FLAG_WITHOUT_INVITES_LIKE_CPP: u32 = 0x040;
        const CALENDAR_FLAG_GUILD_EVENT_LIKE_CPP: u32 = 0x400;

        let guild_scoped = (flags
            & (CALENDAR_FLAG_GUILD_EVENT_LIKE_CPP | CALENDAR_FLAG_WITHOUT_INVITES_LIKE_CPP))
            != 0;
        let guild_id = if guild_scoped {
            let Some(resolved_guild_id) =
                crate::resolved_represented_guild_id_like_cpp(&self.hub.shared(), self.social)
            else {
                return false;
            };
            if resolved_guild_id == 0 {
                return false;
            }
            Some(resolved_guild_id)
        } else {
            None
        };

        #[cfg(any(test, feature = "test-fixtures"))]
        self.social.record_calendar_add_event_for_test_like_cpp(
            RepresentedCalendarAddEventLikeCpp {
                guild_id,
                club_id,
                event_type,
                texture_id,
                time_packed,
                flags,
                invite_count,
                title,
                description,
                max_size,
            },
        );
        true
    }

    fn calendar_remove_event_like_cpp(&mut self, event_id: u64) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.social
            .record_calendar_remove_event_for_test_like_cpp(event_id);
    }

    pub async fn handle_calendar_get_num_pending(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ reads `sCalendarMgr->GetPlayerNumPending(playerGuid)` and sends
        // CalendarSendNumPending. Calendar manager state is not ported yet, so
        // represent the empty pending-invite count.
        self.publication_like_cpp()
            .send_packet_realm(&CalendarSendNumPending { num_pending: 0 });
    }

    pub async fn handle_calendar_complain(&mut self, _complain: CalendarComplain) {
        // C++ only parses/logs this packet and has no gameplay side effect.
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
            self.publication_like_cpp()
                .send_packet(&CalendarCommandResult::with_result_like_cpp(
                    CalendarCommandResult::ERROR_GUILD_PLAYER_NOT_IN_GUILD_LIKE_CPP,
                ));
        }
    }

    pub async fn handle_calendar_get(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ fills CalendarSendCalendar from sCalendarMgr and instance locks.
        // Those live managers are not ported here yet, so represent the
        // well-defined empty calendar/lockout lists with current server time.
        self.publication_like_cpp()
            .send_packet(&CalendarSendCalendar::empty_now());
    }

    pub async fn handle_calendar_get_event(&mut self, _query: CalendarGetEvent) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.publication_like_cpp()
            .send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_copy_event(&mut self, _query: CalendarCopyEvent) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no source event for the requested id. Rust does not have CalendarMgr
        // wired yet, so this represents the observable miss branch.
        self.publication_like_cpp()
            .send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_event_sign_up(&mut self, _query: CalendarEventSignUp) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.publication_like_cpp()
            .send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_invite(&mut self, query: CalendarInvite) {
        // C++ only consults CalendarMgr for an existing event when Creating is
        // false. Rust does not have CalendarMgr wired yet, so this captures the
        // observable no-event branch without inventing name/cache/guild logic.
        if !query.creating {
            self.publication_like_cpp()
                .send_packet(&CalendarCommandResult::event_invalid_like_cpp());
        }
    }

    pub async fn handle_calendar_update_event(&mut self, _query: CalendarUpdateEvent) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.publication_like_cpp()
            .send_packet(&CalendarCommandResult::event_invalid_like_cpp());
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
        self.publication_like_cpp()
            .send_packet(&CalendarCommandResult::no_invite_like_cpp());
    }

    pub async fn handle_calendar_rsvp(&mut self, _query: CalendarRsvp) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.publication_like_cpp()
            .send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_moderator_status(&mut self, _query: CalendarModeratorStatusQuery) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.publication_like_cpp()
            .send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_status(&mut self, _query: CalendarStatus) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.publication_like_cpp()
            .send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }
}

/// Builds a calendar handler context from a host's social state and hub.
pub trait CalendarHandlerHostLikeCpp<C> {
    fn calendar_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> CalendarHandlerCxLikeCpp<'a>;
}

fn handle_calendar_get_num_pending_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .calendar_handler_cx_like_cpp(catalogs)
            .handle_calendar_get_num_pending(pkt)
            .await;
    })
}

fn handle_calendar_complain_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarComplain::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_complain(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarComplain: {e}"),
        }
    })
}

fn handle_calendar_community_invite_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarCommunityInvite::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_community_invite(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarCommunityInvite: {e}"),
        }
    })
}

fn handle_calendar_add_event_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarAddEvent::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_add_event(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarAddEvent: {e}"),
        }
    })
}

fn handle_calendar_get_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .calendar_handler_cx_like_cpp(catalogs)
            .handle_calendar_get(pkt)
            .await;
    })
}

fn handle_calendar_get_event_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarGetEvent::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_get_event(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarGetEvent: {e}"),
        }
    })
}

fn handle_calendar_copy_event_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarCopyEvent::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_copy_event(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarCopyEvent: {e}"),
        }
    })
}

fn handle_calendar_event_sign_up_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarEventSignUp::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_event_sign_up(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarEventSignUp: {e}"),
        }
    })
}

fn handle_calendar_invite_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarInvite::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_invite(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarInvite: {e}"),
        }
    })
}

fn handle_calendar_update_event_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarUpdateEvent::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_update_event(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarUpdateEvent: {e}"),
        }
    })
}

fn handle_calendar_remove_event_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarRemoveEvent::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_remove_event(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarRemoveEvent: {e}"),
        }
    })
}

fn handle_calendar_remove_invite_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarRemoveInvite::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_remove_invite(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarRemoveInvite: {e}"),
        }
    })
}

fn handle_calendar_rsvp_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarRsvp::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_rsvp(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarRsvp: {e}"),
        }
    })
}

fn handle_calendar_moderator_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarModeratorStatusQuery::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_moderator_status(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarModeratorStatusQuery: {e}"),
        }
    })
}

fn handle_calendar_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match CalendarStatus::read(&mut pkt) {
            Ok(value) => {
                session
                    .calendar_handler_cx_like_cpp(catalogs)
                    .handle_calendar_status(value)
                    .await;
            }
            Err(e) => warn!("Failed to read CalendarStatus: {e}"),
        }
    })
}

/// Register the calendar packet entries through their social owner.
pub fn register_calendar_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: CalendarHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarGetNumPending,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_get_num_pending",
        handler: handle_calendar_get_num_pending_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarComplain,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_complain",
        handler: handle_calendar_complain_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarCommunityInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_community_invite",
        handler: handle_calendar_community_invite_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarAddEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_add_event",
        handler: handle_calendar_add_event_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarGet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_get",
        handler: handle_calendar_get_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarGetEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_get_event",
        handler: handle_calendar_get_event_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarCopyEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_copy_event",
        handler: handle_calendar_copy_event_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarEventSignUp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_event_sign_up",
        handler: handle_calendar_event_sign_up_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_invite",
        handler: handle_calendar_invite_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarUpdateEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_update_event",
        handler: handle_calendar_update_event_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarRemoveEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_remove_event",
        handler: handle_calendar_remove_event_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarRemoveInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_remove_invite",
        handler: handle_calendar_remove_invite_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarRsvp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_rsvp",
        handler: handle_calendar_rsvp_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarModeratorStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_moderator_status",
        handler: handle_calendar_moderator_status_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_status",
        handler: handle_calendar_status_thunk::<S, C>,
    })?;
    Ok(())
}

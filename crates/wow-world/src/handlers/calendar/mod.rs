// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Calendar packet handlers.
//!
//! The calendar handlers and their registration moved to `wow-world-social`
//! under #1263 F5. The session keeps only these test-only entry points so the
//! existing scenario module can drive one handler without composing a dispatch
//! table; the represented fixture accessors stay in the session modules that
//! own the social state.

// The scenario module drives the moved handlers with the typed packets, so the
// packet types stay visible through the parent module's glob import.
use wow_packet::packets::misc::{
    CalendarAddEvent, CalendarCommandResult, CalendarCommunityInvite, CalendarComplain,
    CalendarCopyEvent, CalendarEventSignUp, CalendarGetEvent, CalendarInvite,
    CalendarModeratorStatusQuery, CalendarRemoveEvent, CalendarRemoveInvite, CalendarRsvp,
    CalendarSendCalendar, CalendarSendNumPending, CalendarStatus, CalendarUpdateEvent,
};

#[cfg(test)]
mod session_shims {
    use wow_packet::WorldPacket;
    use wow_packet::packets::misc::{
        CalendarAddEvent, CalendarCommunityInvite, CalendarComplain, CalendarCopyEvent,
        CalendarEventSignUp, CalendarGetEvent, CalendarInvite, CalendarModeratorStatusQuery,
        CalendarRemoveEvent, CalendarRemoveInvite, CalendarRsvp, CalendarStatus,
        CalendarUpdateEvent,
    };
    use wow_world_social::CalendarHandlerCxLikeCpp;

    use crate::session::WorldSession;

    impl WorldSession {
        fn calendar_test_cx_like_cpp(&mut self) -> CalendarHandlerCxLikeCpp<'_> {
            self.build_calendar_handler_cx_like_cpp()
        }

        pub(crate) async fn handle_calendar_get_num_pending(
            &mut self,
            _pkt: wow_packet::WorldPacket,
        ) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_get_num_pending(_pkt)
                .await;
        }
        pub(crate) async fn handle_calendar_complain(&mut self, _complain: CalendarComplain) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_complain(_complain)
                .await;
        }
        pub(crate) async fn handle_calendar_community_invite(
            &mut self,
            query: CalendarCommunityInvite,
        ) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_community_invite(query)
                .await;
        }
        pub(crate) async fn handle_calendar_add_event(&mut self, query: CalendarAddEvent) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_add_event(query)
                .await;
        }
        pub(crate) async fn handle_calendar_get(&mut self, _pkt: wow_packet::WorldPacket) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_get(_pkt)
                .await;
        }
        pub(crate) async fn handle_calendar_get_event(&mut self, _query: CalendarGetEvent) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_get_event(_query)
                .await;
        }
        pub(crate) async fn handle_calendar_copy_event(&mut self, _query: CalendarCopyEvent) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_copy_event(_query)
                .await;
        }
        pub(crate) async fn handle_calendar_event_sign_up(&mut self, _query: CalendarEventSignUp) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_event_sign_up(_query)
                .await;
        }
        pub(crate) async fn handle_calendar_invite(&mut self, query: CalendarInvite) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_invite(query)
                .await;
        }
        pub(crate) async fn handle_calendar_update_event(&mut self, _query: CalendarUpdateEvent) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_update_event(_query)
                .await;
        }
        pub(crate) async fn handle_calendar_remove_event(&mut self, query: CalendarRemoveEvent) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_remove_event(query)
                .await;
        }
        pub(crate) async fn handle_calendar_remove_invite(&mut self, _query: CalendarRemoveInvite) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_remove_invite(_query)
                .await;
        }
        pub(crate) async fn handle_calendar_rsvp(&mut self, _query: CalendarRsvp) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_rsvp(_query)
                .await;
        }
        pub(crate) async fn handle_calendar_moderator_status(
            &mut self,
            _query: CalendarModeratorStatusQuery,
        ) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_moderator_status(_query)
                .await;
        }
        pub(crate) async fn handle_calendar_status(&mut self, _query: CalendarStatus) {
            self.calendar_test_cx_like_cpp()
                .handle_calendar_status(_query)
                .await;
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/calendar/tests/mod.rs"]
mod tests;

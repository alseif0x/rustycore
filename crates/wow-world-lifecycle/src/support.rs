// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Support, GM-ticket, feedback and client object-update handlers.
//!
//! C++ source of truth: `WorldSession::HandleGMTicketGetCaseStatusOpcode`,
//! `HandleGMTicketGetSystemStatusOpcode`, `HandleGMTicketAcknowledgeSurveyOpcode`,
//! `HandleComplaintOpcode`, `HandleSubmitUserFeedback`, `HandleSupportTicketSubmitBug`,
//! `HandleSupportTicketSubmitComplaint`, `HandleSupportTicketSubmitSuggestion`,
//! `HandleBugReportOpcode`, `HandleObjectUpdateFailedOpcode` and
//! `HandleObjectUpdateRescuedOpcode` (`src/server/game/Handlers/MiscHandler.cpp` and
//! `SupportHandler.cpp`). The family owns the packet bodies and the support policy it reads;
//! the World session only builds the borrowed context from its disjoint owners (#1263 F5).

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::misc::{
    BugReport, Complaint, ComplaintResult, GmTicketAcknowledgeSurvey, GmTicketCaseStatus,
    GmTicketSystemStatus, ObjectUpdateFailed, ObjectUpdateRescued, SubmitUserFeedback,
    SupportTicketSubmitBug, SupportTicketSubmitComplaint, SupportTicketSubmitSuggestion,
};
use wow_world_core::session::{
    HubMut, PacketPublicationAccessLikeCpp, SupportFeaturePolicyLikeCpp,
};

use crate::SessionLifecycleState;

/// Borrowed inputs of one support handler invocation.
pub struct SupportHandlerCxLikeCpp<'a> {
    lifecycle: &'a mut SessionLifecycleState,
    hub: HubMut<'a>,
    policy: &'a SupportFeaturePolicyLikeCpp,
}

impl<'a> SupportHandlerCxLikeCpp<'a> {
    pub fn new(
        lifecycle: &'a mut SessionLifecycleState,
        hub: HubMut<'a>,
        policy: &'a SupportFeaturePolicyLikeCpp,
    ) -> Self {
        Self {
            lifecycle,
            hub,
            policy,
        }
    }

    fn account_id_like_cpp(&self) -> u32 {
        self.hub.shared().core.account_id
    }

    fn player_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.hub.shared().core.player_guid()
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    pub async fn handle_gm_ticket_get_case_status(&mut self, _pkt: WorldPacket) {
        // C++ `HandleGMTicketGetCaseStatusOpcode` is still a TODO and sends a
        // default `GMTicketCaseStatus`, i.e. an empty case list.
        self.publication_like_cpp()
            .send_packet_realm(&GmTicketCaseStatus::empty());
    }

    pub async fn handle_gm_ticket_get_system_status(&mut self, _pkt: WorldPacket) {
        // C++ uses `sSupportMgr->GetSupportSystemStatus()` here, not
        // `GetTicketSystemStatus()`: this disables the whole customer-support UI.
        self.publication_like_cpp().send_packet(
            &GmTicketSystemStatus::from_support_enabled_like_cpp(self.policy.support_enabled),
        );
    }

    pub async fn handle_gm_ticket_acknowledge_survey(&mut self, mut pkt: WorldPacket) {
        // C++ logs the CaseID and otherwise has only a TODO for future survey persistence.
        if let Err(error) = GmTicketAcknowledgeSurvey::read(&mut pkt) {
            warn!(
                account = self.account_id_like_cpp(),
                "GmTicketAcknowledgeSurvey parse failed: {error}"
            );
        }
    }

    pub async fn handle_complaint(&mut self, mut pkt: WorldPacket) {
        let complaint = match Complaint::read(&mut pkt) {
            Ok(complaint) => complaint,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "Complaint parse failed: {error}"
                );
                return;
            }
        };

        self.publication_like_cpp().send_packet(&ComplaintResult {
            complaint_type: u32::from(complaint.complaint_type),
            result: ComplaintResult::OK_LIKE_CPP,
        });
    }

    pub async fn handle_submit_user_feedback(&mut self, mut pkt: WorldPacket) {
        let feedback = match SubmitUserFeedback::read(&mut pkt) {
            Ok(feedback) => feedback,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "SubmitUserFeedback parse failed: {error}"
                );
                return;
            }
        };

        if feedback.is_suggestion {
            if !self.policy.suggestion_system_enabled_like_cpp() {
                return;
            }
        } else if !self.policy.bug_system_enabled_like_cpp() {
            return;
        }

        // C++ creates a SuggestionTicket/BugTicket and adds it to SupportMgr.
        // Rust has no live SupportMgr ticket runtime yet; the packet has no
        // direct response, so the represented enabled branch remains silent.
    }

    pub async fn handle_support_ticket_submit_bug(&mut self, mut pkt: WorldPacket) {
        let bug = match SupportTicketSubmitBug::read(&mut pkt) {
            Ok(bug) => bug,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "SupportTicketSubmitBug parse failed: {error}"
                );
                return;
            }
        };

        if !self.policy.bug_system_enabled_like_cpp() {
            return;
        }

        let _header = bug.header;
        let _message = bug.message;
        // C++ creates a BugTicket from the packet header/message, then adds it
        // to SupportMgr. Rust has no live SupportMgr ticket runtime yet; the
        // packet has no direct response.
    }

    pub async fn handle_support_ticket_submit_complaint(&mut self, mut pkt: WorldPacket) {
        let complaint = match SupportTicketSubmitComplaint::read(&mut pkt) {
            Ok(complaint) => complaint,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "SupportTicketSubmitComplaint parse failed: {error}"
                );
                return;
            }
        };

        if !self.policy.complaint_system_enabled_like_cpp() {
            return;
        }

        let _complaint = complaint;
        // C++ creates a ComplaintTicket, copies header/chat/category/note
        // fields, then adds it to SupportMgr. Rust has no live SupportMgr
        // ticket runtime yet; the packet has no direct response.
    }

    pub async fn handle_support_ticket_submit_suggestion(&mut self, mut pkt: WorldPacket) {
        let suggestion = match SupportTicketSubmitSuggestion::read(&mut pkt) {
            Ok(suggestion) => suggestion,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "SupportTicketSubmitSuggestion parse failed: {error}"
                );
                return;
            }
        };

        if !self.policy.suggestion_system_enabled_like_cpp() {
            return;
        }

        let _message = suggestion.message;
        // C++ creates a SuggestionTicket with the player's current map and
        // position, then adds it to SupportMgr. Rust has no live SupportMgr
        // ticket runtime yet; the packet has no direct response.
    }

    pub async fn handle_bug_report(&mut self, mut pkt: WorldPacket) {
        let report = match BugReport::read(&mut pkt) {
            Ok(report) => report,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "BugReport parse failed: {error}"
                );
                return;
            }
        };

        if !self.policy.bug_system_enabled_like_cpp() {
            return;
        }

        let Some(port) = self
            .lifecycle
            .support_bug_report_persistence_port_like_cpp()
        else {
            return;
        };
        let request = wow_persistence::SupportBugReportWriteRequestLikeCpp {
            text: report.text,
            diagnostic_info: report.diag_info,
        };
        match port.persist_bug_report_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = self.account_id_like_cpp(),
                    error = %reason,
                    "failed to persist represented CMSG_BUG_REPORT"
                );
            }
        }
    }

    pub async fn handle_object_update_failed(&mut self, mut pkt: WorldPacket) {
        let packet = match ObjectUpdateFailed::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "ObjectUpdateFailed parse failed: {error}"
                );
                return;
            }
        };

        if self.player_guid_like_cpp() == Some(packet.object_guid) {
            self.lifecycle.set_player_logout_like_cpp(true);
            return;
        }

        self.hub
            .shared()
            .core
            .client_visible_guids_like_cpp
            .remove(&packet.object_guid);
    }

    pub async fn handle_object_update_rescued(&mut self, mut pkt: WorldPacket) {
        let packet = match ObjectUpdateRescued::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id_like_cpp(),
                    "ObjectUpdateRescued parse failed: {error}"
                );
                return;
            }
        };

        self.hub
            .shared()
            .core
            .client_visible_guids_like_cpp
            .insert(packet.object_guid);
    }
}

/// Builds a support handler context from a host's lifecycle state, hub and support policy.
pub trait SupportHandlerHostLikeCpp<C> {
    fn support_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> SupportHandlerCxLikeCpp<'a>;
}

fn handle_gm_ticket_get_case_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_gm_ticket_get_case_status(pkt)
            .await;
    })
}

fn handle_gm_ticket_get_system_status_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_gm_ticket_get_system_status(pkt)
            .await;
    })
}

fn handle_gm_ticket_acknowledge_survey_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_gm_ticket_acknowledge_survey(pkt)
            .await;
    })
}

fn handle_complaint_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_complaint(pkt)
            .await;
    })
}

fn handle_submit_user_feedback_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_submit_user_feedback(pkt)
            .await;
    })
}

fn handle_support_ticket_submit_bug_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_support_ticket_submit_bug(pkt)
            .await;
    })
}

fn handle_support_ticket_submit_complaint_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_support_ticket_submit_complaint(pkt)
            .await;
    })
}

fn handle_support_ticket_submit_suggestion_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_support_ticket_submit_suggestion(pkt)
            .await;
    })
}

fn handle_bug_report_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_bug_report(pkt)
            .await;
    })
}

fn handle_object_update_failed_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_object_update_failed(pkt)
            .await;
    })
}

fn handle_object_update_rescued_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .support_handler_cx_like_cpp(catalogs)
            .handle_object_update_rescued(pkt)
            .await;
    })
}

/// Register the support packet entries through their lifecycle owner.
pub fn register_support_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: SupportHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GmTicketGetCaseStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gm_ticket_get_case_status",
        handler: handle_gm_ticket_get_case_status_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GmTicketGetSystemStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gm_ticket_get_system_status",
        handler: handle_gm_ticket_get_system_status_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GmTicketAcknowledgeSurvey,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gm_ticket_acknowledge_survey",
        handler: handle_gm_ticket_acknowledge_survey_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::Complaint,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_complaint",
        handler: handle_complaint_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SubmitUserFeedback,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_submit_user_feedback",
        handler: handle_submit_user_feedback_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SupportTicketSubmitBug,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_support_ticket_submit_bug",
        handler: handle_support_ticket_submit_bug_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SupportTicketSubmitComplaint,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_support_ticket_submit_complaint",
        handler: handle_support_ticket_submit_complaint_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SupportTicketSubmitSuggestion,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_support_ticket_submit_suggestion",
        handler: handle_support_ticket_submit_suggestion_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BugReport,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_bug_report",
        handler: handle_bug_report_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ObjectUpdateFailed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_object_update_failed",
        handler: handle_object_update_failed_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ObjectUpdateRescued,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_object_update_rescued",
        handler: handle_object_update_rescued_thunk::<S, C>,
    })?;
    Ok(())
}

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private support capability handlers extracted from the legacy misc owner.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    BugReport, Complaint, ComplaintResult, GmTicketAcknowledgeSurvey, GmTicketCaseStatus,
    GmTicketSystemStatus, ObjectUpdateFailed, ObjectUpdateRescued, SubmitUserFeedback,
    SupportTicketSubmitBug, SupportTicketSubmitComplaint, SupportTicketSubmitSuggestion,
};

use super::bug_report_insert_statement_like_cpp;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GmTicketGetCaseStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gm_ticket_get_case_status",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_gm_ticket_get_case_status(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GmTicketGetSystemStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gm_ticket_get_system_status",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_gm_ticket_get_system_status(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GmTicketAcknowledgeSurvey,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gm_ticket_acknowledge_survey",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_gm_ticket_acknowledge_survey(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Complaint,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_complaint",
        handler: |session, pkt| Box::pin(async move { session.handle_complaint(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SubmitUserFeedback,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_submit_user_feedback",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_submit_user_feedback(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SupportTicketSubmitBug,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_support_ticket_submit_bug",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_support_ticket_submit_bug(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SupportTicketSubmitComplaint,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_support_ticket_submit_complaint",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_support_ticket_submit_complaint(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SupportTicketSubmitSuggestion,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_support_ticket_submit_suggestion",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_support_ticket_submit_suggestion(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BugReport,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_bug_report",
        handler: |session, pkt| Box::pin(async move { session.handle_bug_report(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ObjectUpdateFailed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_object_update_failed",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_object_update_failed(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ObjectUpdateRescued,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_object_update_rescued",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_object_update_rescued(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    pub async fn handle_gm_ticket_get_case_status(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleGMTicketGetCaseStatusOpcode` is still a TODO and sends a
        // default `GMTicketCaseStatus`, i.e. an empty case list.
        self.send_packet_realm(&GmTicketCaseStatus::empty());
    }

    pub async fn handle_gm_ticket_get_system_status(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ uses `sSupportMgr->GetSupportSystemStatus()` here, not
        // `GetTicketSystemStatus()`: this disables the whole customer-support UI.
        self.send_packet(&GmTicketSystemStatus::from_support_enabled_like_cpp(
            self.represented_support_enabled_like_cpp(),
        ));
    }

    pub async fn handle_gm_ticket_acknowledge_survey(&mut self, mut pkt: wow_packet::WorldPacket) {
        // C++ logs the CaseID and otherwise has only a TODO for future survey persistence.
        if let Err(error) = GmTicketAcknowledgeSurvey::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "GmTicketAcknowledgeSurvey parse failed: {error}"
            );
        }
    }

    pub async fn handle_complaint(&mut self, mut pkt: wow_packet::WorldPacket) {
        let complaint = match Complaint::read(&mut pkt) {
            Ok(complaint) => complaint,
            Err(error) => {
                warn!(account = self.account_id, "Complaint parse failed: {error}");
                return;
            }
        };

        self.send_packet(&ComplaintResult {
            complaint_type: u32::from(complaint.complaint_type),
            result: ComplaintResult::OK_LIKE_CPP,
        });
    }

    pub async fn handle_submit_user_feedback(&mut self, mut pkt: wow_packet::WorldPacket) {
        let feedback = match SubmitUserFeedback::read(&mut pkt) {
            Ok(feedback) => feedback,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SubmitUserFeedback parse failed: {error}"
                );
                return;
            }
        };

        if feedback.is_suggestion {
            if !self.represented_suggestion_system_status_like_cpp() {
                return;
            }
        } else if !self.represented_bug_system_status_like_cpp() {
            return;
        }

        // C++ creates a SuggestionTicket/BugTicket and adds it to SupportMgr.
        // Rust has no live SupportMgr ticket runtime yet; the packet has no
        // direct response, so the represented enabled branch remains silent.
    }

    pub async fn handle_support_ticket_submit_bug(&mut self, mut pkt: wow_packet::WorldPacket) {
        let bug = match SupportTicketSubmitBug::read(&mut pkt) {
            Ok(bug) => bug,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SupportTicketSubmitBug parse failed: {error}"
                );
                return;
            }
        };

        if !self.represented_bug_system_status_like_cpp() {
            return;
        }

        let _header = bug.header;
        let _message = bug.message;
        // C++ creates a BugTicket from the packet header/message, then adds it
        // to SupportMgr. Rust has no live SupportMgr ticket runtime yet; the
        // packet has no direct response.
    }

    pub async fn handle_support_ticket_submit_complaint(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let complaint = match SupportTicketSubmitComplaint::read(&mut pkt) {
            Ok(complaint) => complaint,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SupportTicketSubmitComplaint parse failed: {error}"
                );
                return;
            }
        };

        if !self.represented_complaint_system_status_like_cpp() {
            return;
        }

        let _complaint = complaint;
        // C++ creates a ComplaintTicket, copies header/chat/category/note
        // fields, then adds it to SupportMgr. Rust has no live SupportMgr
        // ticket runtime yet; the packet has no direct response.
    }

    pub async fn handle_support_ticket_submit_suggestion(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let suggestion = match SupportTicketSubmitSuggestion::read(&mut pkt) {
            Ok(suggestion) => suggestion,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SupportTicketSubmitSuggestion parse failed: {error}"
                );
                return;
            }
        };

        if !self.represented_suggestion_system_status_like_cpp() {
            return;
        }

        let _message = suggestion.message;
        // C++ creates a SuggestionTicket with the player's current map and
        // position, then adds it to SupportMgr. Rust has no live SupportMgr
        // ticket runtime yet; the packet has no direct response.
    }

    pub async fn handle_bug_report(&mut self, mut pkt: wow_packet::WorldPacket) {
        let report = match BugReport::read(&mut pkt) {
            Ok(report) => report,
            Err(error) => {
                warn!(account = self.account_id, "BugReport parse failed: {error}");
                return;
            }
        };

        if !self.represented_bug_system_status_like_cpp() {
            return;
        }

        let Some(char_db) = self.char_db().map(std::sync::Arc::clone) else {
            return;
        };
        let stmt = bug_report_insert_statement_like_cpp(&report);
        if let Err(error) = char_db.execute(&stmt).await {
            warn!(
                account = self.account_id,
                error = ?error,
                "failed to persist represented CMSG_BUG_REPORT"
            );
        }
    }

    pub async fn handle_object_update_failed(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ObjectUpdateFailed::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ObjectUpdateFailed parse failed: {error}"
                );
                return;
            }
        };

        if self.player_guid() == Some(packet.object_guid) {
            self.set_player_logout_like_cpp(true);
            return;
        }

        self.client_visible_guids_like_cpp
            .remove(&packet.object_guid);
    }

    pub async fn handle_object_update_rescued(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ObjectUpdateRescued::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ObjectUpdateRescued parse failed: {error}"
                );
                return;
            }
        };

        self.client_visible_guids_like_cpp
            .insert(packet.object_guid);
    }
}

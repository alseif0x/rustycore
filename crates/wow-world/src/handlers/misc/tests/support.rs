// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! support capability handler tests.

use super::*;
use wow_packet::packets::misc::{ComplaintResult, GmTicketSystemStatus};

struct RecordingSupportBugReportPortLikeCpp {
    requests: Mutex<Vec<wow_persistence::SupportBugReportWriteRequestLikeCpp>>,
    outcome: wow_persistence::PersistenceOutcomeLikeCpp,
}

impl RecordingSupportBugReportPortLikeCpp {
    fn new(outcome: wow_persistence::PersistenceOutcomeLikeCpp) -> Arc<Self> {
        Arc::new(Self {
            requests: Mutex::new(Vec::new()),
            outcome,
        })
    }

    fn requests(&self) -> Vec<wow_persistence::SupportBugReportWriteRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl wow_persistence::SupportBugReportPersistencePortLikeCpp
    for RecordingSupportBugReportPortLikeCpp
{
    fn persist_bug_report_like_cpp<'a>(
        &'a self,
        request: wow_persistence::SupportBugReportWriteRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'a, wow_persistence::PersistenceOutcomeLikeCpp>
    {
        self.requests.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }
}

#[tokio::test]
async fn gm_ticket_get_case_status_sends_empty_case_status_like_cpp_todo_handler() {
    let (mut session, send_rx) = make_session();

    session
        .handle_gm_ticket_get_case_status(WorldPacket::new_empty())
        .await;

    let bytes = send_rx.try_recv().expect("GM ticket case status packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::GmTicketCaseStatus as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[tokio::test]
async fn gm_ticket_get_system_status_uses_support_enabled_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_gm_ticket_get_system_status(WorldPacket::new_empty())
        .await;

    let bytes = send_rx.try_recv().expect("GM ticket system status packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::GmTicketSystemStatus as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_int32().unwrap(), GmTicketSystemStatus::ENABLED);

    session.set_represented_support_enabled_like_cpp(false);
    session
        .handle_gm_ticket_get_system_status(WorldPacket::new_empty())
        .await;

    let bytes = send_rx
        .try_recv()
        .expect("disabled GM ticket system status packet");
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_int32().unwrap(), GmTicketSystemStatus::DISABLED);
}

#[tokio::test]
async fn support_status_dispatch_borrows_process_policy_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_state(crate::session::SessionState::LoggedIn);
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp {
        support_feature_policy: Arc::new(crate::session::SupportFeaturePolicyLikeCpp {
            support_enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    let bytes = (ClientOpcodes::GmTicketGetSystemStatus as u16).to_le_bytes();

    session
        .dispatch_packet(&catalogs, WorldPacket::from_bytes(&bytes))
        .await;

    let bytes = send_rx.try_recv().expect("GM ticket system status packet");
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_int32().unwrap(), GmTicketSystemStatus::DISABLED);
}

#[tokio::test]
async fn gm_ticket_acknowledge_survey_consumes_case_id_and_is_silent_like_cpp_todo_handler() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(123);

    session.handle_gm_ticket_acknowledge_survey(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn complaint_sends_result_zero_like_cpp() {
    let (mut session, send_rx) = make_session();
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

    session.handle_complaint(pkt).await;

    let bytes = send_rx.try_recv().expect("complaint result packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::ComplaintResult as u16
    );

    let mut response = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        response.read_uint32().unwrap(),
        SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP as u32
    );
    assert_eq!(response.read_uint8().unwrap(), ComplaintResult::OK_LIKE_CPP);
}

#[tokio::test]
async fn submit_user_feedback_obeys_support_system_gates_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_submit_user_feedback(submit_user_feedback_packet(true, "suggestion"))
        .await;
    session
        .handle_submit_user_feedback(submit_user_feedback_packet(false, "bug"))
        .await;
    assert!(send_rx.try_recv().is_err());

    session.set_represented_support_enabled_like_cpp(true);
    session.set_represented_support_suggestions_enabled_like_cpp(true);
    session
        .handle_submit_user_feedback(submit_user_feedback_packet(true, "suggestion"))
        .await;
    assert!(send_rx.try_recv().is_err());

    session.set_represented_support_suggestions_enabled_like_cpp(false);
    session.set_represented_support_bugs_enabled_like_cpp(true);
    session
        .handle_submit_user_feedback(submit_user_feedback_packet(false, "bug"))
        .await;
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn support_ticket_submit_suggestion_obeys_support_system_gate_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_support_ticket_submit_suggestion(support_ticket_submit_suggestion_packet(
            "suggest me",
        ))
        .await;
    assert!(send_rx.try_recv().is_err());

    session.set_represented_support_enabled_like_cpp(true);
    session.set_represented_support_suggestions_enabled_like_cpp(true);
    session
        .handle_support_ticket_submit_suggestion(support_ticket_submit_suggestion_packet(
            "suggest me too",
        ))
        .await;
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn support_ticket_submit_bug_obeys_support_system_gate_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_support_ticket_submit_bug(support_ticket_submit_bug_packet("broken"))
        .await;
    assert!(send_rx.try_recv().is_err());

    session.set_represented_support_enabled_like_cpp(true);
    session.set_represented_support_bugs_enabled_like_cpp(true);
    session
        .handle_support_ticket_submit_bug(support_ticket_submit_bug_packet("still broken"))
        .await;
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn support_ticket_submit_complaint_obeys_support_system_gate_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_support_ticket_submit_complaint(support_ticket_submit_complaint_packet("report"))
        .await;
    assert!(send_rx.try_recv().is_err());

    session.set_represented_support_enabled_like_cpp(true);
    session.set_represented_support_complaints_enabled_like_cpp(true);
    session
        .handle_support_ticket_submit_complaint(support_ticket_submit_complaint_packet(
            "report enabled",
        ))
        .await;
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn bug_report_is_silent_when_bug_support_disabled_like_cpp_default() {
    let (mut session, send_rx) = make_session();
    session
        .handle_bug_report(bug_report_packet(true, "diag", "bug"))
        .await;

    assert!(!session.represented_support_bugs_enabled_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn bug_report_support_config_flag_is_session_wired_like_cpp() {
    let (mut session, _send_rx) = make_session();
    assert!(session.represented_support_enabled_like_cpp());
    assert!(!session.represented_support_bugs_enabled_like_cpp());
    assert!(!session.represented_bug_system_status_like_cpp());

    session.set_represented_support_enabled_like_cpp(false);
    session.set_represented_support_bugs_enabled_like_cpp(true);
    assert!(!session.represented_bug_system_status_like_cpp());

    session.set_represented_support_enabled_like_cpp(true);
    session.set_represented_support_bugs_enabled_like_cpp(true);
    assert!(session.represented_support_bugs_enabled_like_cpp());
    assert!(session.represented_bug_system_status_like_cpp());

    session.set_represented_support_complaints_enabled_like_cpp(true);
    assert!(session.represented_support_complaints_enabled_like_cpp());
    assert!(session.represented_complaint_system_status_like_cpp());

    session.set_represented_support_suggestions_enabled_like_cpp(true);
    assert!(session.represented_support_suggestions_enabled_like_cpp());
    assert!(session.represented_suggestion_system_status_like_cpp());

    session.set_represented_support_enabled_like_cpp(false);
    assert!(!session.represented_suggestion_system_status_like_cpp());

    session.set_represented_support_enabled_like_cpp(true);
    session.set_represented_support_bugs_enabled_like_cpp(false);
    assert!(!session.represented_support_bugs_enabled_like_cpp());
    assert!(!session.represented_bug_system_status_like_cpp());
}

#[tokio::test]
async fn bug_report_reaches_the_typed_support_port_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_support_bugs_enabled_like_cpp(true);
    let port = RecordingSupportBugReportPortLikeCpp::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 0 },
    );
    session.set_support_bug_report_persistence_port_like_cpp(port.clone());

    session
        .handle_bug_report(bug_report_packet(true, "diag blob", "client bug"))
        .await;

    assert_eq!(
        port.requests(),
        vec![wow_persistence::SupportBugReportWriteRequestLikeCpp {
            text: "client bug".to_owned(),
            diagnostic_info: "diag blob".to_owned(),
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn bug_report_missing_or_failed_port_remains_wire_silent_like_cpp() {
    let (mut missing, missing_rx) = make_session();
    missing.set_represented_support_bugs_enabled_like_cpp(true);
    missing
        .handle_bug_report(bug_report_packet(true, "diag", "missing"))
        .await;
    assert!(missing_rx.try_recv().is_err());

    let (mut failed, failed_rx) = make_session();
    failed.set_represented_support_bugs_enabled_like_cpp(true);
    let port = RecordingSupportBugReportPortLikeCpp::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Failed {
            reason: "forced bug-report failure".to_owned(),
        },
    );
    failed.set_support_bug_report_persistence_port_like_cpp(port.clone());
    failed
        .handle_bug_report(bug_report_packet(true, "diag", "failed"))
        .await;
    assert_eq!(port.requests().len(), 1);
    assert!(failed_rx.try_recv().is_err());
}

#[test]
fn bug_report_handler_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::BugReport)
        .expect("BugReport handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_bug_report");
}

#[test]
fn gm_ticket_get_system_status_handler_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GmTicketGetSystemStatus)
        .expect("GmTicketGetSystemStatus handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_gm_ticket_get_system_status");
}

#[test]
fn gm_ticket_acknowledge_survey_handler_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GmTicketAcknowledgeSurvey)
        .expect("GmTicketAcknowledgeSurvey handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_gm_ticket_acknowledge_survey");
}

#[test]
fn complaint_handler_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::Complaint)
        .expect("Complaint handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_complaint");
}

#[test]
fn submit_user_feedback_handler_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::SubmitUserFeedback)
        .expect("SubmitUserFeedback handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_submit_user_feedback");
}

#[test]
fn support_ticket_submit_suggestion_handler_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::SupportTicketSubmitSuggestion)
        .expect("SupportTicketSubmitSuggestion handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(
        entry.handler_name,
        "handle_support_ticket_submit_suggestion"
    );
}

#[test]
fn support_ticket_submit_bug_handler_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::SupportTicketSubmitBug)
        .expect("SupportTicketSubmitBug handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_support_ticket_submit_bug");
}

#[test]
fn support_ticket_submit_complaint_handler_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::SupportTicketSubmitComplaint)
        .expect("SupportTicketSubmitComplaint handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_support_ticket_submit_complaint");
}

#[tokio::test]
async fn object_update_failed_removes_seen_object_like_cpp() {
    let (mut session, send_rx) = make_session();
    let object_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 7, 9);
    session.client_visible_guids_like_cpp.insert(object_guid);

    session
        .handle_object_update_failed(object_update_recovery_packet(object_guid))
        .await;

    assert!(!session.client_visible_guids_like_cpp.contains(&object_guid));
    assert!(!session.player_logout_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn object_update_failed_for_player_marks_logout_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 9001);
    session.set_player_guid(Some(player_guid));
    session.client_visible_guids_like_cpp.insert(player_guid);

    session
        .handle_object_update_failed(object_update_recovery_packet(player_guid))
        .await;

    assert!(session.player_logout_like_cpp());
    assert!(session.client_visible_guids_like_cpp.contains(&player_guid));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn object_update_rescued_reinserts_seen_object_like_cpp() {
    let (mut session, send_rx) = make_session();
    let object_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 8, 3);
    assert!(!session.client_visible_guids_like_cpp.contains(&object_guid));

    session
        .handle_object_update_rescued(object_update_recovery_packet(object_guid))
        .await;

    assert!(session.client_visible_guids_like_cpp.contains(&object_guid));
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn object_update_recovery_handler_metadata_matches_cpp() {
    let failed = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::ObjectUpdateFailed)
        .expect("ObjectUpdateFailed handler entry");
    assert_eq!(failed.status, SessionStatus::LoggedIn);
    assert_eq!(failed.processing, PacketProcessing::Inplace);
    assert_eq!(failed.handler_name, "handle_object_update_failed");

    let rescued = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::ObjectUpdateRescued)
        .expect("ObjectUpdateRescued handler entry");
    assert_eq!(rescued.status, SessionStatus::LoggedIn);
    assert_eq!(rescued.processing, PacketProcessing::Inplace);
    assert_eq!(rescued.handler_name, "handle_object_update_rescued");
}

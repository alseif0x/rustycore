// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[derive(Debug)]
struct RecordingPacketSpoofBanPortLikeCpp {
    load_outcome: wow_persistence::PacketSpoofAffectedAccountsLoadOutcomeLikeCpp,
    write_outcome: wow_persistence::PersistenceOutcomeLikeCpp,
    load_addresses: std::sync::Mutex<Vec<String>>,
    writes: std::sync::Mutex<Vec<wow_persistence::PacketSpoofBanWriteRequestLikeCpp>>,
}

impl RecordingPacketSpoofBanPortLikeCpp {
    fn new(
        load_outcome: wow_persistence::PacketSpoofAffectedAccountsLoadOutcomeLikeCpp,
        write_outcome: wow_persistence::PersistenceOutcomeLikeCpp,
    ) -> Self {
        Self {
            load_outcome,
            write_outcome,
            load_addresses: Default::default(),
            writes: Default::default(),
        }
    }
}

impl wow_persistence::PacketSpoofBanPersistencePortLikeCpp for RecordingPacketSpoofBanPortLikeCpp {
    fn load_accounts_by_ip_like_cpp<'a>(
        &'a self,
        address: &'a str,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::PacketSpoofAffectedAccountsLoadOutcomeLikeCpp,
    > {
        self.load_addresses
            .lock()
            .expect("load address recorder")
            .push(address.to_string());
        let outcome = self.load_outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_packet_spoof_ban_like_cpp<'a>(
        &'a self,
        request: wow_persistence::PacketSpoofBanWriteRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<'a, wow_persistence::PersistenceOutcomeLikeCpp>
    {
        self.writes.lock().expect("write recorder").push(request);
        let outcome = self.write_outcome.clone();
        Box::pin(async move { outcome })
    }
}

#[test]
fn update_empty_queue() {
    let (mut session, _, _) = make_session();
    let processed = session.update(100);
    assert_eq!(processed, 0);
}

#[test]
fn update_processes_packets() {
    let (mut session, pkt_tx, _) = make_session();

    // Send some packets (they'll be logged as "no handler" but won't crash)
    for _ in 0..5 {
        let pkt = WorldPacket::from_bytes(&[0x00, 0x00]); // opcode 0
        pkt_tx.send(pkt).unwrap();
    }

    let processed = session.update(100);
    assert_eq!(processed, 5);
    assert_eq!(session.pending_packets.len(), 5);
}

#[test]
fn update_disconnects_when_socket_timeout_deadline_expired_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_socket_timeouts_like_cpp(SocketTimeoutsLikeCpp {
        unauthenticated_secs: 60,
        active_secs: 30,
    });
    session.socket_timeout_deadline_like_cpp = Instant::now() - Duration::from_secs(1);

    assert_eq!(session.update(100), 0);
    assert!(session.is_disconnecting());
}

#[test]
fn timed_logout_preserves_player_until_disconnect_save_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 77);
    session.set_player_guid(Some(guid));
    session.set_state(SessionState::LoggedIn);
    session.logout_time = Some(Instant::now() - Duration::from_secs(1));

    session.update(100);

    let packet = send_rx.try_recv().expect("LogoutComplete packet");
    let mut packet = WorldPacket::from_bytes(&packet);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LogoutComplete as u16
    );
    assert_eq!(session.player_guid(), Some(guid));
    assert!(session.is_disconnecting());
}

#[test]
fn update_resets_socket_timeout_on_regular_packet_like_cpp() {
    let (mut session, pkt_tx, _) = make_session();
    session.set_socket_timeouts_like_cpp(SocketTimeoutsLikeCpp {
        unauthenticated_secs: 60,
        active_secs: 30,
    });
    session.socket_timeout_deadline_like_cpp = Instant::now() - Duration::from_secs(1);
    pkt_tx
        .send(WorldPacket::from_bytes(&[0x00, 0x00]))
        .expect("packet queued");

    assert_eq!(session.update(100), 1);
    assert!(!session.is_disconnecting());
}

#[test]
fn keep_alive_only_resets_socket_timeout_for_logged_in_session_like_cpp() {
    let keep_alive_opcode = (ClientOpcodes::KeepAlive as u16).to_le_bytes();

    let (mut authed_session, authed_tx, _) = make_session();
    authed_session.set_socket_timeouts_like_cpp(SocketTimeoutsLikeCpp {
        unauthenticated_secs: 60,
        active_secs: 30,
    });
    authed_session.socket_timeout_deadline_like_cpp = Instant::now() - Duration::from_secs(1);
    authed_tx
        .send(WorldPacket::from_bytes(&keep_alive_opcode))
        .expect("keepalive queued");

    assert_eq!(authed_session.update(100), 1);
    assert!(authed_session.is_disconnecting());

    let (mut logged_in_session, logged_in_tx, _) = make_session();
    logged_in_session.set_state(SessionState::LoggedIn);
    logged_in_session.set_socket_timeouts_like_cpp(SocketTimeoutsLikeCpp {
        unauthenticated_secs: 60,
        active_secs: 30,
    });
    logged_in_session.socket_timeout_deadline_like_cpp = Instant::now() - Duration::from_secs(1);
    logged_in_tx
        .send(WorldPacket::from_bytes(&keep_alive_opcode))
        .expect("keepalive queued");

    assert_eq!(logged_in_session.update(100), 1);
    assert!(!logged_in_session.is_disconnecting());
}

#[test]
fn packet_spoof_policy_kick_blocks_over_limit_opcode_like_cpp() {
    let hotfix_opcode = (ClientOpcodes::HotfixRequest as u16).to_le_bytes();
    let (mut session, pkt_tx, _) = make_session();
    session.set_packet_spoof_config_like_cpp(PacketSpoofConfigLikeCpp {
        policy: PacketSpoofConfigLikeCpp::POLICY_KICK,
        ban_mode: PacketSpoofConfigLikeCpp::BAN_ACCOUNT,
        ban_duration_secs: 86_400,
    });
    pkt_tx
        .send(WorldPacket::from_bytes(&hotfix_opcode))
        .expect("first packet queued");
    pkt_tx
        .send(WorldPacket::from_bytes(&hotfix_opcode))
        .expect("second packet queued");

    assert_eq!(session.update(100), 1);
    assert_eq!(session.pending_packets.len(), 1);
    assert!(session.is_disconnecting());
}

#[test]
fn packet_spoof_policy_log_keeps_over_limit_opcode_like_cpp() {
    let hotfix_opcode = (ClientOpcodes::HotfixRequest as u16).to_le_bytes();
    let (mut session, pkt_tx, _) = make_session();
    session.set_packet_spoof_config_like_cpp(PacketSpoofConfigLikeCpp {
        policy: PacketSpoofConfigLikeCpp::POLICY_LOG,
        ban_mode: PacketSpoofConfigLikeCpp::BAN_ACCOUNT,
        ban_duration_secs: 86_400,
    });
    pkt_tx
        .send(WorldPacket::from_bytes(&hotfix_opcode))
        .expect("first packet queued");
    pkt_tx
        .send(WorldPacket::from_bytes(&hotfix_opcode))
        .expect("second packet queued");

    assert_eq!(session.update(100), 2);
    assert_eq!(session.pending_packets.len(), 2);
    assert!(!session.is_disconnecting());
}

#[test]
fn packet_spoof_policy_ban_stages_account_ban_like_cpp() {
    let hotfix_opcode = (ClientOpcodes::HotfixRequest as u16).to_le_bytes();
    let (mut session, pkt_tx, _) = make_session();
    session.set_packet_spoof_config_like_cpp(PacketSpoofConfigLikeCpp {
        policy: PacketSpoofConfigLikeCpp::POLICY_BAN,
        ban_mode: PacketSpoofConfigLikeCpp::BAN_ACCOUNT,
        ban_duration_secs: 3_600,
    });
    pkt_tx
        .send(WorldPacket::from_bytes(&hotfix_opcode))
        .expect("first packet queued");
    pkt_tx
        .send(WorldPacket::from_bytes(&hotfix_opcode))
        .expect("second packet queued");

    assert_eq!(session.update(100), 1);
    assert!(session.is_disconnecting());
    assert_eq!(
        session.pending_packet_spoof_ban_like_cpp,
        Some(PacketSpoofPendingBanLikeCpp {
            target: PacketSpoofPendingBanTargetLikeCpp::Account { account_id: 1 },
            duration_secs: 3_600,
        })
    );
}

#[test]
fn packet_spoof_policy_ban_stages_ip_ban_from_remote_address_like_cpp() {
    let hotfix_opcode = (ClientOpcodes::HotfixRequest as u16).to_le_bytes();
    let (mut session, pkt_tx, _) = make_session();
    session.set_remote_address_like_cpp(Some("203.0.113.77".to_string()));
    session.set_packet_spoof_config_like_cpp(PacketSpoofConfigLikeCpp {
        policy: PacketSpoofConfigLikeCpp::POLICY_BAN,
        ban_mode: PacketSpoofConfigLikeCpp::BAN_IP,
        ban_duration_secs: 7_200,
    });
    pkt_tx
        .send(WorldPacket::from_bytes(&hotfix_opcode))
        .expect("first packet queued");
    pkt_tx
        .send(WorldPacket::from_bytes(&hotfix_opcode))
        .expect("second packet queued");

    assert_eq!(session.update(100), 1);
    assert!(session.is_disconnecting());
    assert_eq!(
        session.pending_packet_spoof_ban_like_cpp,
        Some(PacketSpoofPendingBanLikeCpp {
            target: PacketSpoofPendingBanTargetLikeCpp::Ip {
                address: "203.0.113.77".to_string(),
            },
            duration_secs: 7_200,
        })
    );
}

#[tokio::test]
async fn packet_spoof_account_ban_uses_one_semantic_write_without_ip_lookup_like_cpp() {
    let (mut session, _, _) = make_session();
    let port = Arc::new(RecordingPacketSpoofBanPortLikeCpp::new(
        wow_persistence::PacketSpoofAffectedAccountsLoadOutcomeLikeCpp::Loaded(vec![99]),
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    ));
    session.set_packet_spoof_ban_persistence_port_like_cpp(port.clone());
    session.pending_packet_spoof_ban_like_cpp = Some(PacketSpoofPendingBanLikeCpp {
        target: PacketSpoofPendingBanTargetLikeCpp::Account { account_id: 7 },
        duration_secs: 60,
    });

    session.flush_packet_spoof_ban_like_cpp().await;

    assert!(session.pending_packet_spoof_ban_like_cpp.is_none());
    assert!(port.load_addresses.lock().unwrap().is_empty());
    assert_eq!(
        *port.writes.lock().unwrap(),
        vec![wow_persistence::PacketSpoofBanWriteRequestLikeCpp {
            target: wow_persistence::PacketSpoofBanTargetLikeCpp::Account { account_id: 7 },
            duration_secs: 60,
            author: PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP.to_string(),
            reason: PACKET_SPOOF_BAN_REASON_LIKE_CPP.to_string(),
        }]
    );
}

#[tokio::test]
async fn packet_spoof_ip_lookup_failure_does_not_suppress_ban_write_like_cpp() {
    let (mut session, _, _) = make_session();
    let port = Arc::new(RecordingPacketSpoofBanPortLikeCpp::new(
        wow_persistence::PacketSpoofAffectedAccountsLoadOutcomeLikeCpp::Failed {
            reason: "lookup failed".to_string(),
        },
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    ));
    session.set_packet_spoof_ban_persistence_port_like_cpp(port.clone());
    session.pending_packet_spoof_ban_like_cpp = Some(PacketSpoofPendingBanLikeCpp {
        target: PacketSpoofPendingBanTargetLikeCpp::Ip {
            address: "203.0.113.77".to_string(),
        },
        duration_secs: 120,
    });

    session.flush_packet_spoof_ban_like_cpp().await;

    assert!(session.pending_packet_spoof_ban_like_cpp.is_none());
    assert_eq!(
        *port.load_addresses.lock().unwrap(),
        vec!["203.0.113.77".to_string()]
    );
    assert_eq!(port.writes.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn packet_spoof_persistence_failure_restages_the_exact_plan_like_cpp() {
    let (mut session, _, _) = make_session();
    let port = Arc::new(RecordingPacketSpoofBanPortLikeCpp::new(
        wow_persistence::PacketSpoofAffectedAccountsLoadOutcomeLikeCpp::Loaded(vec![7, 7, 9]),
        wow_persistence::PersistenceOutcomeLikeCpp::Failed {
            reason: "write failed".to_string(),
        },
    ));
    session.set_packet_spoof_ban_persistence_port_like_cpp(port);
    let plan = PacketSpoofPendingBanLikeCpp {
        target: PacketSpoofPendingBanTargetLikeCpp::Ip {
            address: "203.0.113.77".to_string(),
        },
        duration_secs: 120,
    };
    session.pending_packet_spoof_ban_like_cpp = Some(plan.clone());

    session.flush_packet_spoof_ban_like_cpp().await;

    assert_eq!(session.pending_packet_spoof_ban_like_cpp, Some(plan));
}

#[test]
fn packet_spoof_admission_has_no_concrete_persistence_after_port_cut() {
    let source = include_str!("../admission.rs");
    for forbidden in [
        "LoginDatabase",
        "LoginStatements",
        "SqlTransaction",
        ".prepare(",
        ".query(",
        ".execute(",
        ".commit_transaction(",
    ] {
        assert!(
            !source.contains(forbidden),
            "admission regained concrete persistence syntax: {forbidden}"
        );
    }
}

#[tokio::test]
async fn kick_like_cpp_session_command_disconnects_session() {
    let (mut session, _, _) = make_session();
    session
        .session_command_tx()
        .try_send(SessionCommand::KickLikeCpp(KickLikeCppCommand {
            reason: "World::BanAccount Banning account".to_string(),
        }))
        .expect("kick command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(session.is_disconnecting());
}

#[tokio::test]
async fn shutdown_flush_command_observes_prior_kick_like_cpp() {
    let (mut session, _, _) = make_session();
    let (response_tx, response_rx) = flume::bounded(1);
    let command_tx = session.session_command_tx();

    command_tx
        .try_send(SessionCommand::KickLikeCpp(KickLikeCppCommand {
            reason: "World::KickAll".to_string(),
        }))
        .expect("kick command queued");
    command_tx
        .try_send(SessionCommand::WorldSessionShutdownFlushLikeCpp(
            WorldSessionShutdownFlushLikeCppCommand {
                diff_ms: 1,
                response_tx,
            },
        ))
        .expect("flush command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let result = response_rx.try_recv().expect("flush ack returned");
    assert_eq!(result.diff_ms, 1);
    assert!(result.disconnecting);
    assert!(session.is_disconnecting());
}

#[test]
fn packet_spoof_ban_eviction_queues_kick_for_affected_accounts_like_cpp() {
    let (mut session, _, _) = make_session();
    let registry = Arc::new(PlayerRegistry::default());
    let (send_tx_a, _send_rx_a) = flume::bounded(1);
    let (send_tx_b, _send_rx_b) = flume::bounded(1);
    let (send_tx_c, _send_rx_c) = flume::bounded(1);
    let (command_tx_a, command_rx_a) = flume::bounded(1);
    let (command_tx_b, command_rx_b) = flume::bounded(1);
    let (command_tx_c, command_rx_c) = flume::bounded(1);

    let guid_a = ObjectGuid::create_player(1, 11);
    let guid_b = ObjectGuid::create_player(1, 12);
    let guid_c = ObjectGuid::create_player(1, 13);
    let mut info_a = broadcast_info(guid_a, send_tx_a);
    info_a.identity.account_id = 7;
    info_a.command_tx = command_tx_a;
    let mut info_b = broadcast_info(guid_b, send_tx_b);
    info_b.identity.account_id = 9;
    info_b.command_tx = command_tx_b;
    let mut info_c = broadcast_info(guid_c, send_tx_c);
    info_c.identity.account_id = 7;
    info_c.command_tx = command_tx_c;

    registry.register_or_replace(guid_a, info_a, Default::default());
    registry.register_or_replace(guid_b, info_b, Default::default());
    registry.register_or_replace(guid_c, info_c, Default::default());
    session.set_player_registry(registry);

    assert_eq!(
        session.kick_packet_spoof_affected_sessions_like_cpp(&[7]),
        2
    );
    assert!(matches!(
        command_rx_a.try_recv(),
        Ok(SessionCommand::KickLikeCpp(_))
    ));
    assert!(matches!(
        command_rx_b.try_recv(),
        Err(flume::TryRecvError::Empty)
    ));
    assert!(matches!(
        command_rx_c.try_recv(),
        Ok(SessionCommand::KickLikeCpp(_))
    ));
}

#[test]
fn packet_spoof_cpp_opcode_limit_table_is_exhaustive_like_cpp() {
    for opcode in [
        ClientOpcodes::PlayerLogin,
        ClientOpcodes::QueryPlayerNames,
        ClientOpcodes::QueryPetName,
        ClientOpcodes::QueryNpcText,
        ClientOpcodes::AttackStop,
        ClientOpcodes::QueryTime,
        ClientOpcodes::QueryCorpseTransport,
        ClientOpcodes::MoveTimeSkipped,
        ClientOpcodes::QueryNextMailTime,
        ClientOpcodes::SetSheathed,
        ClientOpcodes::UpdateRaidTarget,
        ClientOpcodes::LogoutRequest,
        ClientOpcodes::PetRename,
        ClientOpcodes::QuestGiverRequestReward,
        ClientOpcodes::CompleteCinematic,
        ClientOpcodes::NextCinematicCamera,
        ClientOpcodes::OpeningCinematic,
        ClientOpcodes::BankerActivate,
        ClientOpcodes::BuyBankSlot,
        ClientOpcodes::OptOutOfLoot,
        ClientOpcodes::CalendarComplain,
        ClientOpcodes::QueryQuestInfo,
        ClientOpcodes::QueryGameObject,
        ClientOpcodes::QueryCreature,
        ClientOpcodes::QuestGiverStatusQuery,
        ClientOpcodes::QueryGuildInfo,
        ClientOpcodes::TaxiNodeStatusQuery,
        ClientOpcodes::TaxiQueryAvailableNodes,
        ClientOpcodes::QuestGiverQueryQuest,
        ClientOpcodes::QueryPageText,
        ClientOpcodes::GuildBankTextQuery,
        ClientOpcodes::QueryCorpseLocationFromClient,
        ClientOpcodes::MoveSetFacing,
        ClientOpcodes::MoveSetFacingHeartbeat,
        ClientOpcodes::MoveSetPitch,
        ClientOpcodes::RequestPartyMemberStats,
        ClientOpcodes::QuestGiverCompleteQuest,
        ClientOpcodes::SetActionButton,
        ClientOpcodes::SetActionBarToggles,
        ClientOpcodes::ResetInstances,
        ClientOpcodes::HearthAndResurrect,
        ClientOpcodes::TogglePvp,
        ClientOpcodes::SetPvp,
        ClientOpcodes::PetAbandon,
        ClientOpcodes::ActivateTaxi,
        ClientOpcodes::SelfRes,
        ClientOpcodes::UnlearnSkill,
        ClientOpcodes::SaveEquipmentSet,
        ClientOpcodes::AssignEquipmentSetSpec,
        ClientOpcodes::DeleteEquipmentSet,
        ClientOpcodes::RepopRequest,
        ClientOpcodes::PartyInvite,
        ClientOpcodes::PartyInviteResponse,
        ClientOpcodes::PartyUninvite,
        ClientOpcodes::LeaveGroup,
        ClientOpcodes::AcceptWargameInvite,
        ClientOpcodes::BattlemasterJoinArena,
        ClientOpcodes::BattlemasterHello,
        ClientOpcodes::BattlefieldList,
        ClientOpcodes::BattlefieldPort,
        ClientOpcodes::BattlefieldLeave,
        ClientOpcodes::BattlemasterJoin,
        ClientOpcodes::GuildBankLogQuery,
        ClientOpcodes::LogoutCancel,
        ClientOpcodes::AlterAppearance,
        ClientOpcodes::SetPlayerDeclinedNames,
        ClientOpcodes::QuestConfirmAccept,
        ClientOpcodes::GuildEventLogQuery,
        ClientOpcodes::QuestGiverStatusMultipleQuery,
        ClientOpcodes::InitiateTrade,
        ClientOpcodes::ChatAddonMessage,
        ClientOpcodes::ChatAddonMessageWhisper,
        ClientOpcodes::ChatMessageAfk,
        ClientOpcodes::ChatMessageChannel,
        ClientOpcodes::ChatMessageDnd,
        ClientOpcodes::ChatMessageEmote,
        ClientOpcodes::ChatMessageGuild,
        ClientOpcodes::ChatMessageOfficer,
        ClientOpcodes::ChatMessageParty,
        ClientOpcodes::ChatMessageRaid,
        ClientOpcodes::ChatMessageRaidWarning,
        ClientOpcodes::ChatMessageSay,
        ClientOpcodes::ChatMessageWhisper,
        ClientOpcodes::ChatMessageYell,
        ClientOpcodes::UpdateAadcStatus,
        ClientOpcodes::Inspect,
        ClientOpcodes::AreaSpiritHealerQuery,
        ClientOpcodes::StandStateChange,
        ClientOpcodes::RandomRoll,
        ClientOpcodes::TimeSyncResponse,
        ClientOpcodes::TimeSyncResponseDropped,
        ClientOpcodes::TimeSyncResponseFailed,
        ClientOpcodes::MoveForceRunSpeedChangeAck,
        ClientOpcodes::MoveForceSwimSpeedChangeAck,
        ClientOpcodes::MoveForceSwimBackSpeedChangeAck,
        ClientOpcodes::MoveForceRunBackSpeedChangeAck,
        ClientOpcodes::MoveForceFlightSpeedChangeAck,
        ClientOpcodes::MoveForceFlightBackSpeedChangeAck,
        ClientOpcodes::MoveForceWalkSpeedChangeAck,
        ClientOpcodes::MoveForceTurnRateChangeAck,
        ClientOpcodes::MoveForcePitchRateChangeAck,
    ] {
        assert_eq!(
            WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
            0,
            "{opcode:?}"
        );
    }

    for opcode in [
        ClientOpcodes::QuestGiverAcceptQuest,
        ClientOpcodes::QuestLogRemoveQuest,
        ClientOpcodes::QuestGiverChooseReward,
        ClientOpcodes::SendContactList,
        ClientOpcodes::AutobankItem,
        ClientOpcodes::AutostoreBankItem,
        ClientOpcodes::Who,
        ClientOpcodes::RideVehicleInteract,
        ClientOpcodes::MoveHeartbeat,
    ] {
        assert_eq!(
            WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
            200,
            "{opcode:?}"
        );
    }

    for opcode in [
        ClientOpcodes::GuildSetMemberNote,
        ClientOpcodes::SetContactNotes,
        ClientOpcodes::CalendarGet,
        ClientOpcodes::GuildBankQueryTab,
        ClientOpcodes::QueryInspectAchievements,
        ClientOpcodes::GameObjReportUse,
        ClientOpcodes::GameObjUse,
        ClientOpcodes::DeclinePetition,
    ] {
        assert_eq!(
            WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
            50,
            "{opcode:?}"
        );
    }

    assert_eq!(
        WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(
            ClientOpcodes::QuestPoiQuery
        ),
        crate::handlers::quest::MAX_QUEST_LOG_SIZE_LIKE_CPP as u32
    );

    for opcode in [ClientOpcodes::SpellClick, ClientOpcodes::MoveDismissVehicle] {
        assert_eq!(
            WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
            20,
            "{opcode:?}"
        );
    }

    for opcode in [
        ClientOpcodes::SignPetition,
        ClientOpcodes::TurnInPetition,
        ClientOpcodes::ChangeSubGroup,
        ClientOpcodes::QueryPetition,
        ClientOpcodes::CharCustomize,
        ClientOpcodes::CharRaceOrFactionChange,
        ClientOpcodes::CharDelete,
        ClientOpcodes::DelFriend,
        ClientOpcodes::AddFriend,
        ClientOpcodes::CharacterRenameRequest,
        ClientOpcodes::BugReport,
        ClientOpcodes::SetPartyLeader,
        ClientOpcodes::ConvertRaid,
        ClientOpcodes::SetAssistantLeader,
        ClientOpcodes::MoveChangeVehicleSeats,
        ClientOpcodes::PetitionBuy,
        ClientOpcodes::RequestVehiclePrevSeat,
        ClientOpcodes::RequestVehicleNextSeat,
        ClientOpcodes::RequestVehicleSwitchSeat,
        ClientOpcodes::RequestVehicleExit,
        ClientOpcodes::EjectPassenger,
        ClientOpcodes::ItemPurchaseRefund,
        ClientOpcodes::SocketGems,
        ClientOpcodes::WrapItem,
        ClientOpcodes::ReportPvpPlayerAfk,
    ] {
        assert_eq!(
            WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
            10,
            "{opcode:?}"
        );
    }

    for opcode in [
        ClientOpcodes::CreateCharacter,
        ClientOpcodes::EnumCharacters,
        ClientOpcodes::EnumCharactersDeletedByClient,
        ClientOpcodes::SubmitUserFeedback,
        ClientOpcodes::SupportTicketSubmitBug,
        ClientOpcodes::SupportTicketSubmitComplaint,
        ClientOpcodes::SupportTicketSubmitSuggestion,
        ClientOpcodes::CalendarAddEvent,
        ClientOpcodes::CalendarUpdateEvent,
        ClientOpcodes::CalendarRemoveEvent,
        ClientOpcodes::CalendarCopyEvent,
        ClientOpcodes::CalendarInvite,
        ClientOpcodes::CalendarEventSignUp,
        ClientOpcodes::CalendarRsvp,
        ClientOpcodes::CalendarStatus,
        ClientOpcodes::CalendarModeratorStatus,
        ClientOpcodes::CalendarRemoveInvite,
        ClientOpcodes::SetLootMethod,
        ClientOpcodes::GuildInviteByName,
        ClientOpcodes::AcceptGuildInvite,
        ClientOpcodes::GuildDeclineInvitation,
        ClientOpcodes::GuildLeave,
        ClientOpcodes::GuildDelete,
        ClientOpcodes::GuildSetGuildMaster,
        ClientOpcodes::GuildUpdateMotdText,
        ClientOpcodes::GuildSetRankPermissions,
        ClientOpcodes::GuildAddRank,
        ClientOpcodes::GuildDeleteRank,
        ClientOpcodes::GuildUpdateInfoText,
        ClientOpcodes::GuildBankDepositMoney,
        ClientOpcodes::GuildBankWithdrawMoney,
        ClientOpcodes::GuildBankBuyTab,
        ClientOpcodes::GuildBankUpdateTab,
        ClientOpcodes::GuildBankSetTabText,
        ClientOpcodes::SaveGuildEmblem,
        ClientOpcodes::PetitionRenameGuild,
        ClientOpcodes::ConfirmRespecWipe,
        ClientOpcodes::SetDungeonDifficulty,
        ClientOpcodes::SetRaidDifficulty,
        ClientOpcodes::SetPartyAssignment,
        ClientOpcodes::DoReadyCheck,
    ] {
        assert_eq!(
            WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
            3,
            "{opcode:?}"
        );
    }

    assert_eq!(
        WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(
            ClientOpcodes::GetItemPurchaseData
        ),
        PLAYER_SLOT_END as u32
    );
    assert_eq!(
        WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(
            ClientOpcodes::HotfixRequest
        ),
        1
    );
    assert_eq!(
        WorldSession::packet_spoof_max_packet_counter_allowed_like_cpp(ClientOpcodes::AuthSession),
        100
    );
}

#[test]
fn packet_spoof_zero_limit_opcode_is_unlimited_like_cpp() {
    let player_login_opcode = (ClientOpcodes::PlayerLogin as u16).to_le_bytes();
    let (mut session, pkt_tx, _) = make_session();
    session.set_packet_spoof_config_like_cpp(PacketSpoofConfigLikeCpp {
        policy: PacketSpoofConfigLikeCpp::POLICY_KICK,
        ban_mode: PacketSpoofConfigLikeCpp::BAN_ACCOUNT,
        ban_duration_secs: 86_400,
    });
    for _ in 0..5 {
        pkt_tx
            .send(WorldPacket::from_bytes(&player_login_opcode))
            .expect("packet queued");
    }

    assert_eq!(session.update(100), 5);
    assert_eq!(session.pending_packets.len(), 5);
    assert!(!session.is_disconnecting());
}

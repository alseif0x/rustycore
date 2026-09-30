use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};

use super::*;
use crate::session::connection_identity::PacketCounterLikeCpp;
use wow_network::SocketTimeoutsLikeCpp;

fn admission(now: Instant) -> SessionAdmissionState {
    SessionAdmissionState {
        dispatch_table: HashMap::new(),
        last_packet_time: now,
        last_phase_authority_like_cpp: [None, None],
        map_phase_coordinated_like_cpp: false,
        packet_spoof_config_like_cpp: PacketSpoofConfigLikeCpp::default(),
        packet_throttling_like_cpp: HashMap::new(),
        pending_packet_spoof_ban_like_cpp: None,
        pending_packets: VecDeque::new(),
        socket_timeout_deadline_like_cpp: now + Duration::from_secs(60),
        socket_timeouts_like_cpp: SocketTimeoutsLikeCpp {
            active_secs: 30,
            unauthenticated_secs: 60,
        },
    }
}

fn packet(opcode: ClientOpcodes) -> WorldPacket {
    WorldPacket::from_bytes(&(opcode as u16).to_le_bytes())
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
            SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
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
            SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
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
            SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
            50,
            "{opcode:?}"
        );
    }

    assert_eq!(
        SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(
            ClientOpcodes::QuestPoiQuery
        ),
        crate::handlers::quest::MAX_QUEST_LOG_SIZE_LIKE_CPP as u32
    );

    for opcode in [ClientOpcodes::SpellClick, ClientOpcodes::MoveDismissVehicle] {
        assert_eq!(
            SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
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
            SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
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
            SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(opcode),
            3,
            "{opcode:?}"
        );
    }

    assert_eq!(
        SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(
            ClientOpcodes::GetItemPurchaseData
        ),
        PLAYER_SLOT_END as u32
    );
    assert_eq!(
        SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(
            ClientOpcodes::HotfixRequest
        ),
        1
    );
    assert_eq!(
        SessionAdmissionState::packet_spoof_max_packet_counter_allowed_like_cpp(ClientOpcodes::AuthSession),
        100
    );
}


#[test]
fn received_packet_preserves_distinct_activity_timeout_and_counter_clocks() {
    let first = Instant::now();
    let second = first + Duration::from_secs(1);
    let mut owner = admission(first);
    let calls = RefCell::new(Vec::new());
    let mut instants = [first, second].into_iter();

    assert_eq!(
        owner.observe_received_packet_with_clocks(
            &SessionState::Authed,
            &packet(ClientOpcodes::HotfixRequest),
            || {
                calls.borrow_mut().push("instant");
                instants.next().expect("only activity and timeout clocks")
            },
            || {
                calls.borrow_mut().push("seconds");
                100
            },
        ),
        IngressObservation::Accepted,
    );
    assert_eq!(*calls.borrow(), ["instant", "instant", "seconds"]);
    assert_eq!(owner.last_packet_time, first);
    assert_eq!(owner.socket_timeout_deadline_like_cpp, second + Duration::from_secs(60));
    assert_eq!(owner.packet_throttling_like_cpp[&(ClientOpcodes::HotfixRequest as u16)].amount_counter, 1);
    assert!(owner.pending_packets.is_empty());
}

#[test]
fn inactive_keep_alive_records_activity_without_refreshing_the_deadline() {
    let base = Instant::now();
    for state in [SessionState::Authed, SessionState::Transfer, SessionState::Disconnecting] {
        let mut owner = admission(base);
        let deadline = owner.socket_timeout_deadline_like_cpp;
        let instant_calls = Cell::new(0);
        let seconds_calls = Cell::new(0);
        let received_at = base + Duration::from_secs(2);

        assert_eq!(
            owner.observe_received_packet_with_clocks(
                &state,
                &packet(ClientOpcodes::KeepAlive),
                || {
                    instant_calls.set(instant_calls.get() + 1);
                    received_at
                },
                || {
                    seconds_calls.set(seconds_calls.get() + 1);
                    100
                },
            ),
            IngressObservation::Accepted,
        );
        assert_eq!(owner.last_packet_time, received_at);
        assert_eq!(owner.socket_timeout_deadline_like_cpp, deadline);
        assert_eq!(instant_calls.get(), 1);
        assert_eq!(seconds_calls.get(), 1);
    }
}

#[test]
fn active_keep_alive_uses_the_second_instant_and_active_timeout() {
    let base = Instant::now();
    let mut owner = admission(base);
    let mut instants = [base, base + Duration::from_secs(2)].into_iter();

    assert_eq!(
        owner.observe_received_packet_with_clocks(
            &SessionState::LoggedIn,
            &packet(ClientOpcodes::KeepAlive),
            || instants.next().expect("two distinct instant reads"),
            || 100,
        ),
        IngressObservation::Accepted,
    );
    assert_eq!(owner.last_packet_time, base);
    assert_eq!(owner.socket_timeout_deadline_like_cpp, base + Duration::from_secs(32));
    assert_eq!(instants.next(), None);
}

#[test]
fn unknown_and_unlimited_packets_refresh_timeout_without_a_wall_clock_or_counter() {
    let base = Instant::now();
    for packet in [
        WorldPacket::from_bytes(&u16::MAX.to_le_bytes()),
        packet(ClientOpcodes::PlayerLogin),
    ] {
        let mut owner = admission(base);
        let instant_calls = Cell::new(0);
        assert_eq!(
            owner.observe_received_packet_with_clocks(
                &SessionState::Authed,
                &packet,
                || {
                    instant_calls.set(instant_calls.get() + 1);
                    base + Duration::from_secs(1)
                },
                || panic!("unknown and unlimited opcodes must not read SystemTime"),
            ),
            IngressObservation::Accepted,
        );
        assert_eq!(instant_calls.get(), 2);
        assert_eq!(owner.last_packet_time, base + Duration::from_secs(1));
        assert_eq!(owner.socket_timeout_deadline_like_cpp, base + Duration::from_secs(61));
        assert!(owner.packet_throttling_like_cpp.is_empty());
    }
}

#[test]
fn counter_rolls_over_on_any_second_change_and_keeps_opcodes_independent() {
    let mut owner = admission(Instant::now());
    let hotfix = packet(ClientOpcodes::HotfixRequest);
    let spell_click = packet(ClientOpcodes::SpellClick);

    assert_eq!(owner.observe_packet_counter(&hotfix, || 100), IngressObservation::Accepted);
    assert_eq!(
        owner.observe_packet_counter(&hotfix, || 100),
        IngressObservation::Flood { opcode: ClientOpcodes::HotfixRequest, count: 2 },
    );
    assert_eq!(owner.observe_packet_counter(&spell_click, || 100), IngressObservation::Accepted);
    assert_eq!(owner.packet_throttling_like_cpp[&(ClientOpcodes::HotfixRequest as u16)].amount_counter, 2);
    assert_eq!(owner.packet_throttling_like_cpp[&(ClientOpcodes::SpellClick as u16)].amount_counter, 1);
    assert_eq!(owner.observe_packet_counter(&hotfix, || 101), IngressObservation::Accepted);
    assert_eq!(owner.observe_packet_counter(&hotfix, || 99), IngressObservation::Accepted);
    assert_eq!(
        owner.packet_throttling_like_cpp[&(ClientOpcodes::HotfixRequest as u16)],
        PacketCounterLikeCpp { last_receive_time_secs: 99, amount_counter: 1 },
    );
}

#[test]
fn saturated_counter_keeps_reporting_flood_without_wrapping() {
    let mut owner = admission(Instant::now());
    owner.packet_throttling_like_cpp.insert(
        ClientOpcodes::HotfixRequest as u16,
        PacketCounterLikeCpp { last_receive_time_secs: 100, amount_counter: u32::MAX },
    );

    for _ in 0..2 {
        assert_eq!(
            owner.observe_packet_counter(&packet(ClientOpcodes::HotfixRequest), || 100),
            IngressObservation::Flood { opcode: ClientOpcodes::HotfixRequest, count: u32::MAX },
        );
    }
    assert_eq!(owner.packet_throttling_like_cpp[&(ClientOpcodes::HotfixRequest as u16)].amount_counter, u32::MAX);
}

#[test]
fn flood_action_reads_current_policy_after_the_observation() {
    let mut owner = admission(Instant::now());
    let hotfix = packet(ClientOpcodes::HotfixRequest);
    assert_eq!(owner.observe_packet_counter(&hotfix, || 100), IngressObservation::Accepted);
    assert_eq!(
        owner.observe_packet_counter(&hotfix, || 100),
        IngressObservation::Flood { opcode: ClientOpcodes::HotfixRequest, count: 2 },
    );

    for (policy, expected) in [
        (PacketSpoofConfigLikeCpp::POLICY_LOG, FloodAction::Allow),
        (PacketSpoofConfigLikeCpp::POLICY_KICK, FloodAction::Kick),
        (PacketSpoofConfigLikeCpp::POLICY_BAN, FloodAction::Ban),
        (u32::MAX, FloodAction::Allow),
    ] {
        owner.packet_spoof_config_like_cpp.policy = policy;
        assert_eq!(owner.flood_action(), expected);
    }
    assert!(owner.pending_packet_spoof_ban_like_cpp.is_none());
}

#[test]
fn idle_expiry_is_strictly_after_the_deadline() {
    let base = Instant::now();
    let owner = admission(base);
    let deadline = owner.socket_timeout_deadline_like_cpp;

    assert!(!owner.is_idle_at(deadline - Duration::from_nanos(1)));
    assert!(!owner.is_idle_at(deadline));
    assert!(owner.is_idle_at(deadline + Duration::from_nanos(1)));
}

#[test]
fn explicit_timeout_reset_preserves_inactive_gate_without_calling_the_clock() {
    let base = Instant::now();
    let mut owner = admission(base);
    let deadline = owner.socket_timeout_deadline_like_cpp;
    owner.reset_timeout_with_clock(
        &SessionState::Transfer,
        true,
        || panic!("inactive only-active reset must not read Instant"),
    );
    assert_eq!(owner.socket_timeout_deadline_like_cpp, deadline);
    assert_eq!(owner.last_packet_time, base);

    owner.reset_timeout_with_clock(&SessionState::Transfer, false, || base + Duration::from_secs(2));
    assert_eq!(owner.socket_timeout_deadline_like_cpp, base + Duration::from_secs(62));
    assert_eq!(owner.last_packet_time, base);
}

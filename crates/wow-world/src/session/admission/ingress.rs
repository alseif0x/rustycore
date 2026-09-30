// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ingress observation on the existing admission owner.
//!
//! Retains Rust's ingress-time AntiDOS evaluation and monotonic deadline:
//! C++ evaluates AntiDOS after dispatch status gates and exempts queued sessions
//! from idle expiry (WorldSession.cpp:782-793, 1251-1303). This extraction does
//! not change those gaps, phase permits, queue selection or socket fences.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::super::state::SessionAdmissionState;
use super::super::{
    ClientOpcodes, PLAYER_SLOT_END, PacketSpoofConfigLikeCpp, SessionState, WorldPacket,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum IngressObservation {
    Accepted,
    Flood { opcode: ClientOpcodes, count: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum FloodAction {
    Allow,
    Kick,
    Ban,
}

impl SessionAdmissionState {
    pub(in crate::session) fn observe_received_packet(
        &mut self,
        state: &SessionState,
        packet: &WorldPacket,
    ) -> IngressObservation {
        self.observe_received_packet_with_clocks(state, packet, Instant::now, || {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        })
    }

    fn observe_received_packet_with_clocks(
        &mut self,
        state: &SessionState,
        packet: &WorldPacket,
        mut instant_now: impl FnMut() -> Instant,
        seconds_now: impl FnOnce() -> u64,
    ) -> IngressObservation {
        self.last_packet_time = instant_now();
        self.reset_timeout_with_clock(
            state,
            packet.opcode_raw() == ClientOpcodes::KeepAlive as u16,
            &mut instant_now,
        );
        self.observe_packet_counter(packet, seconds_now)
    }

    pub(in crate::session) fn reset_timeout(
        &mut self,
        state: &SessionState,
        only_active: bool,
    ) {
        self.reset_timeout_with_clock(state, only_active, Instant::now);
    }

    fn reset_timeout_with_clock(
        &mut self,
        state: &SessionState,
        only_active: bool,
        instant_now: impl FnOnce() -> Instant,
    ) {
        let timeout_secs = if *state == SessionState::LoggedIn {
            Some(self.socket_timeouts_like_cpp.active_secs)
        } else if !only_active {
            Some(self.socket_timeouts_like_cpp.unauthenticated_secs)
        } else {
            None
        };

        if let Some(timeout_secs) = timeout_secs {
            self.socket_timeout_deadline_like_cpp =
                instant_now() + Duration::from_secs(timeout_secs);
        }
    }

    pub(in crate::session) fn is_idle(&self) -> bool {
        self.is_idle_at(Instant::now())
    }

    fn is_idle_at(&self, now: Instant) -> bool {
        now > self.socket_timeout_deadline_like_cpp
    }

    // Called by APP only after its flood warning and Player-name read.
    pub(in crate::session) fn flood_action(&self) -> FloodAction {
        match self.packet_spoof_config_like_cpp.policy {
            PacketSpoofConfigLikeCpp::POLICY_LOG => FloodAction::Allow,
            PacketSpoofConfigLikeCpp::POLICY_KICK => FloodAction::Kick,
            PacketSpoofConfigLikeCpp::POLICY_BAN => FloodAction::Ban,
            _ => FloodAction::Allow,
        }
    }

    fn observe_packet_counter(
        &mut self,
        packet: &WorldPacket,
        seconds_now: impl FnOnce() -> u64,
    ) -> IngressObservation {
        let Some(opcode) = packet.client_opcode() else {
            return IngressObservation::Accepted;
        };
        let max_packet_counter_allowed =
            Self::packet_spoof_max_packet_counter_allowed_like_cpp(opcode);
        if max_packet_counter_allowed == 0 {
            return IngressObservation::Accepted;
        }

        let now = seconds_now();
        let counter = self
            .packet_throttling_like_cpp
            .entry(packet.opcode_raw())
            .or_default();
        if counter.last_receive_time_secs != now {
            counter.last_receive_time_secs = now;
            counter.amount_counter = 0;
        }
        counter.amount_counter = counter.amount_counter.saturating_add(1);
        if counter.amount_counter <= max_packet_counter_allowed {
            return IngressObservation::Accepted;
        }
        IngressObservation::Flood {
            opcode,
            count: counter.amount_counter,
        }
    }

    fn packet_spoof_max_packet_counter_allowed_like_cpp(opcode: ClientOpcodes) -> u32 {
        match opcode {
            // C++ returns 0 for cheap/no-query opcodes: no AntiDOS limit.
            ClientOpcodes::PlayerLogin
            | ClientOpcodes::QueryPlayerNames
            | ClientOpcodes::QueryPetName
            | ClientOpcodes::QueryNpcText
            | ClientOpcodes::AttackStop
            | ClientOpcodes::QueryTime
            | ClientOpcodes::QueryCorpseTransport
            | ClientOpcodes::MoveTimeSkipped
            | ClientOpcodes::QueryNextMailTime
            | ClientOpcodes::SetSheathed
            | ClientOpcodes::UpdateRaidTarget
            | ClientOpcodes::LogoutRequest
            | ClientOpcodes::PetRename
            | ClientOpcodes::QuestGiverRequestReward
            | ClientOpcodes::CompleteCinematic
            | ClientOpcodes::NextCinematicCamera
            | ClientOpcodes::OpeningCinematic
            | ClientOpcodes::BankerActivate
            | ClientOpcodes::BuyBankSlot
            | ClientOpcodes::OptOutOfLoot
            | ClientOpcodes::CalendarComplain
            | ClientOpcodes::QueryQuestInfo
            | ClientOpcodes::QueryGameObject
            | ClientOpcodes::QueryCreature
            | ClientOpcodes::QuestGiverStatusQuery
            | ClientOpcodes::QueryGuildInfo
            | ClientOpcodes::TaxiNodeStatusQuery
            | ClientOpcodes::TaxiQueryAvailableNodes
            | ClientOpcodes::QuestGiverQueryQuest
            | ClientOpcodes::QueryPageText
            | ClientOpcodes::GuildBankTextQuery
            | ClientOpcodes::QueryCorpseLocationFromClient
            | ClientOpcodes::MoveSetFacing
            | ClientOpcodes::MoveSetFacingHeartbeat
            | ClientOpcodes::MoveSetPitch
            | ClientOpcodes::RequestPartyMemberStats
            | ClientOpcodes::QuestGiverCompleteQuest
            | ClientOpcodes::SetActionButton
            | ClientOpcodes::SetActionBarToggles
            | ClientOpcodes::ResetInstances
            | ClientOpcodes::HearthAndResurrect
            | ClientOpcodes::TogglePvp
            | ClientOpcodes::SetPvp
            | ClientOpcodes::PetAbandon
            | ClientOpcodes::ActivateTaxi
            | ClientOpcodes::SelfRes
            | ClientOpcodes::UnlearnSkill
            | ClientOpcodes::SaveEquipmentSet
            | ClientOpcodes::AssignEquipmentSetSpec
            | ClientOpcodes::DeleteEquipmentSet
            | ClientOpcodes::UseEquipmentSet
            | ClientOpcodes::RepopRequest
            | ClientOpcodes::PartyInvite
            | ClientOpcodes::PartyInviteResponse
            | ClientOpcodes::PartyUninvite
            | ClientOpcodes::LeaveGroup
            | ClientOpcodes::AcceptWargameInvite
            | ClientOpcodes::BattlemasterJoinArena
            | ClientOpcodes::BattlemasterJoinSkirmish
            | ClientOpcodes::BattlemasterHello
            | ClientOpcodes::BattlefieldList
            | ClientOpcodes::BattlefieldPort
            | ClientOpcodes::BattlefieldLeave
            | ClientOpcodes::BattlemasterJoin
            | ClientOpcodes::GuildBankLogQuery
            | ClientOpcodes::LogoutCancel
            | ClientOpcodes::AlterAppearance
            | ClientOpcodes::SetPlayerDeclinedNames
            | ClientOpcodes::AdventureMapStartQuest
            | ClientOpcodes::ArenaTeamAccept
            | ClientOpcodes::ArenaTeamLeave
            | ClientOpcodes::ArenaTeamRemove
            | ClientOpcodes::ArenaTeamDisband
            | ClientOpcodes::ArenaTeamLeader
            | ClientOpcodes::QueryArenaTeam
            | ClientOpcodes::QuestConfirmAccept
            | ClientOpcodes::GuildEventLogQuery
            | ClientOpcodes::QuestGiverStatusMultipleQuery
            | ClientOpcodes::InitiateTrade
            | ClientOpcodes::ChatAddonMessage
            | ClientOpcodes::ChatAddonMessageWhisper
            | ClientOpcodes::ChatMessageAfk
            | ClientOpcodes::ChatMessageChannel
            | ClientOpcodes::ChatMessageDnd
            | ClientOpcodes::ChatMessageEmote
            | ClientOpcodes::ChatMessageGuild
            | ClientOpcodes::ChatMessageOfficer
            | ClientOpcodes::ChatMessageParty
            | ClientOpcodes::ChatMessageRaid
            | ClientOpcodes::ChatMessageRaidWarning
            | ClientOpcodes::ChatMessageSay
            | ClientOpcodes::ChatMessageWhisper
            | ClientOpcodes::ChatMessageYell
            | ClientOpcodes::UpdateAadcStatus
            | ClientOpcodes::Inspect
            | ClientOpcodes::AreaSpiritHealerQuery
            | ClientOpcodes::StandStateChange
            | ClientOpcodes::RandomRoll
            | ClientOpcodes::TimeSyncResponse
            | ClientOpcodes::TimeSyncResponseDropped
            | ClientOpcodes::TimeSyncResponseFailed
            | ClientOpcodes::MoveForceRunSpeedChangeAck
            | ClientOpcodes::MoveForceSwimSpeedChangeAck
            | ClientOpcodes::MoveForceSwimBackSpeedChangeAck
            | ClientOpcodes::MoveForceRunBackSpeedChangeAck
            | ClientOpcodes::MoveForceFlightSpeedChangeAck
            | ClientOpcodes::MoveForceFlightBackSpeedChangeAck
            | ClientOpcodes::MoveForceWalkSpeedChangeAck
            | ClientOpcodes::MoveForceTurnRateChangeAck
            | ClientOpcodes::MoveForcePitchRateChangeAck => 0,

            ClientOpcodes::QuestGiverAcceptQuest
            | ClientOpcodes::QuestLogRemoveQuest
            | ClientOpcodes::QuestGiverChooseReward
            | ClientOpcodes::SendContactList
            | ClientOpcodes::AutobankItem
            | ClientOpcodes::AutostoreBankItem
            | ClientOpcodes::Who
            | ClientOpcodes::RideVehicleInteract
            | ClientOpcodes::MoveHeartbeat => 200,

            ClientOpcodes::GuildSetMemberNote
            | ClientOpcodes::SetContactNotes
            | ClientOpcodes::CalendarGet
            | ClientOpcodes::GuildBankQueryTab
            | ClientOpcodes::QueryInspectAchievements
            | ClientOpcodes::GameObjReportUse
            | ClientOpcodes::GameObjUse
            | ClientOpcodes::DeclinePetition => 50,

            ClientOpcodes::QuestPoiQuery => {
                crate::handlers::quest::MAX_QUEST_LOG_SIZE_LIKE_CPP as u32
            }

            ClientOpcodes::SpellClick | ClientOpcodes::MoveDismissVehicle => 20,

            ClientOpcodes::SignPetition
            | ClientOpcodes::TurnInPetition
            | ClientOpcodes::ChangeSubGroup
            | ClientOpcodes::QueryPetition
            | ClientOpcodes::CharCustomize
            | ClientOpcodes::CharRaceOrFactionChange
            | ClientOpcodes::CharDelete
            | ClientOpcodes::DelFriend
            | ClientOpcodes::AddFriend
            | ClientOpcodes::CharacterRenameRequest
            | ClientOpcodes::BugReport
            | ClientOpcodes::SetPartyLeader
            | ClientOpcodes::ConvertRaid
            | ClientOpcodes::SetAssistantLeader
            | ClientOpcodes::MoveChangeVehicleSeats
            | ClientOpcodes::PetitionBuy
            | ClientOpcodes::RequestVehiclePrevSeat
            | ClientOpcodes::RequestVehicleNextSeat
            | ClientOpcodes::RequestVehicleSwitchSeat
            | ClientOpcodes::RequestVehicleExit
            | ClientOpcodes::EjectPassenger
            | ClientOpcodes::ItemPurchaseRefund
            | ClientOpcodes::SocketGems
            | ClientOpcodes::WrapItem
            | ClientOpcodes::ReportPvpPlayerAfk => 10,

            ClientOpcodes::CreateCharacter
            | ClientOpcodes::EnumCharacters
            | ClientOpcodes::EnumCharactersDeletedByClient
            | ClientOpcodes::SubmitUserFeedback
            | ClientOpcodes::SupportTicketSubmitBug
            | ClientOpcodes::SupportTicketSubmitComplaint
            | ClientOpcodes::SupportTicketSubmitSuggestion
            | ClientOpcodes::CalendarAddEvent
            | ClientOpcodes::CalendarUpdateEvent
            | ClientOpcodes::CalendarRemoveEvent
            | ClientOpcodes::CalendarCopyEvent
            | ClientOpcodes::CalendarInvite
            | ClientOpcodes::CalendarEventSignUp
            | ClientOpcodes::CalendarRsvp
            | ClientOpcodes::CalendarStatus
            | ClientOpcodes::CalendarModeratorStatus
            | ClientOpcodes::CalendarRemoveInvite
            | ClientOpcodes::SetLootMethod
            | ClientOpcodes::GuildInviteByName
            | ClientOpcodes::AcceptGuildInvite
            | ClientOpcodes::GuildDeclineInvitation
            | ClientOpcodes::GuildLeave
            | ClientOpcodes::GuildDelete
            | ClientOpcodes::GuildSetGuildMaster
            | ClientOpcodes::GuildUpdateMotdText
            | ClientOpcodes::GuildSetRankPermissions
            | ClientOpcodes::GuildAddRank
            | ClientOpcodes::GuildDeleteRank
            | ClientOpcodes::GuildUpdateInfoText
            | ClientOpcodes::GuildBankDepositMoney
            | ClientOpcodes::GuildBankWithdrawMoney
            | ClientOpcodes::GuildBankBuyTab
            | ClientOpcodes::GuildBankUpdateTab
            | ClientOpcodes::GuildBankSetTabText
            | ClientOpcodes::SaveGuildEmblem
            | ClientOpcodes::PetitionRenameGuild
            | ClientOpcodes::ConfirmRespecWipe
            | ClientOpcodes::LearnTalent
            | ClientOpcodes::SetDungeonDifficulty
            | ClientOpcodes::SetRaidDifficulty
            | ClientOpcodes::SetPartyAssignment
            | ClientOpcodes::DoReadyCheck => 3,

            ClientOpcodes::GetItemPurchaseData => PLAYER_SLOT_END as u32,
            ClientOpcodes::HotfixRequest => 1,
            _ => 100,
        }
    }

}

#[cfg(test)]
mod tests;

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{
    Arc, ClientOpcodes, Duration, Instant, KickLikeCppCommand, LoginDatabase, LoginStatements,
    PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP, PACKET_SPOOF_BAN_REASON_LIKE_CPP, PLAYER_SLOT_END,
    PacketSpoofConfigLikeCpp, PacketSpoofPendingBanLikeCpp, PacketSpoofPendingBanTargetLikeCpp,
    SessionCommand, SessionState, SqlTransaction, SystemTime, UNIX_EPOCH, WorldPacket, warn,
};
use std::vec::Vec;

impl super::WorldSession {
    pub(crate) fn reset_timeout_time_like_cpp(&mut self, only_active: bool) {
        let timeout_secs = if self.state == SessionState::LoggedIn {
            Some(self.socket_timeouts_like_cpp.active_secs)
        } else if !only_active {
            Some(self.socket_timeouts_like_cpp.unauthenticated_secs)
        } else {
            None
        };

        if let Some(timeout_secs) = timeout_secs {
            self.socket_timeout_deadline_like_cpp =
                Instant::now() + Duration::from_secs(timeout_secs);
        }
    }

    pub(super) fn reset_timeout_time_for_packet_like_cpp(&mut self, opcode_raw: u16) {
        self.reset_timeout_time_like_cpp(opcode_raw == ClientOpcodes::KeepAlive as u16);
    }

    pub(crate) fn is_connection_idle_like_cpp(&self) -> bool {
        Instant::now() > self.socket_timeout_deadline_like_cpp
    }

    fn packet_spoof_now_secs_like_cpp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub(super) fn packet_spoof_max_packet_counter_allowed_like_cpp(opcode: ClientOpcodes) -> u32 {
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

    pub(super) fn evaluate_packet_spoof_like_cpp(&mut self, pkt: &WorldPacket) -> bool {
        let Some(opcode) = pkt.client_opcode() else {
            return true;
        };
        let max_packet_counter_allowed =
            Self::packet_spoof_max_packet_counter_allowed_like_cpp(opcode);
        if max_packet_counter_allowed == 0 {
            return true;
        }

        let now = Self::packet_spoof_now_secs_like_cpp();
        let counter = self
            .packet_throttling_like_cpp
            .entry(pkt.opcode_raw())
            .or_default();
        if counter.last_receive_time_secs != now {
            counter.last_receive_time_secs = now;
            counter.amount_counter = 0;
        }
        counter.amount_counter = counter.amount_counter.saturating_add(1);
        if counter.amount_counter <= max_packet_counter_allowed {
            return true;
        }
        let amount_counter = counter.amount_counter;

        warn!(
            "AntiDOS: Account {}, Character: {:?}, flooding packet (opc: {:?} (0x{:X}), count: {})",
            self.account_id,
            self.player_name_like_cpp(),
            opcode,
            pkt.opcode_raw(),
            amount_counter
        );

        match self.packet_spoof_config_like_cpp.policy {
            PacketSpoofConfigLikeCpp::POLICY_LOG => true,
            PacketSpoofConfigLikeCpp::POLICY_KICK => {
                self.kick("WorldSession::DosProtection::EvaluateOpcode AntiDOS");
                false
            }
            PacketSpoofConfigLikeCpp::POLICY_BAN => {
                self.stage_packet_spoof_ban_like_cpp();
                self.kick("WorldSession::DosProtection::EvaluateOpcode AntiDOS");
                false
            }
            _ => true,
        }
    }

    fn stage_packet_spoof_ban_like_cpp(&mut self) {
        let target = match self.packet_spoof_config_like_cpp.ban_mode {
            PacketSpoofConfigLikeCpp::BAN_IP => {
                let Some(address) = self.remote_address_like_cpp.clone() else {
                    warn!(
                        account = self.account_id,
                        "AntiDOS: PacketSpoof BAN_IP requested but remote address is unavailable; kicking without persistent IP ban"
                    );
                    return;
                };
                PacketSpoofPendingBanTargetLikeCpp::Ip { address }
            }
            _ => {
                // TrinityCore's AntiDOS path maps BAN_CHARACTER to account bans because
                // character-level packet spoof bans are not implemented there either.
                PacketSpoofPendingBanTargetLikeCpp::Account {
                    account_id: self.account_id,
                }
            }
        };

        self.pending_packet_spoof_ban_like_cpp = Some(PacketSpoofPendingBanLikeCpp {
            target,
            duration_secs: self.packet_spoof_config_like_cpp.ban_duration_secs,
        });
    }

    pub(super) async fn flush_packet_spoof_ban_like_cpp(&mut self) {
        let Some(plan) = self.pending_packet_spoof_ban_like_cpp.take() else {
            return;
        };
        let Some(login_db) = self.login_db.as_ref().map(Arc::clone) else {
            warn!(
                account = self.account_id,
                "AntiDOS: PacketSpoof ban requested but login DB is unavailable"
            );
            self.pending_packet_spoof_ban_like_cpp = Some(plan);
            return;
        };

        let affected_account_ids = self
            .packet_spoof_ban_affected_account_ids_like_cpp(&login_db, &plan)
            .await;

        let result = match &plan.target {
            PacketSpoofPendingBanTargetLikeCpp::Account { account_id } => {
                let mut tx = SqlTransaction::new();
                let mut clear_active = login_db.prepare(LoginStatements::UPD_ACCOUNT_NOT_BANNED);
                clear_active.set_u32(0, *account_id);
                tx.append(clear_active);

                let mut insert = login_db.prepare(LoginStatements::INS_ACCOUNT_BANNED);
                insert.set_u32(0, *account_id);
                insert.set_u32(1, plan.duration_secs);
                insert.set_string(2, PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP);
                insert.set_string(3, PACKET_SPOOF_BAN_REASON_LIKE_CPP);
                tx.append(insert);

                login_db.commit_transaction(tx).await
            }
            PacketSpoofPendingBanTargetLikeCpp::Ip { address } => {
                let mut insert = login_db.prepare(LoginStatements::INS_IP_BANNED);
                insert.set_string(0, address);
                insert.set_u32(1, plan.duration_secs);
                insert.set_string(2, PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP);
                insert.set_string(3, PACKET_SPOOF_BAN_REASON_LIKE_CPP);

                login_db.execute(&insert).await.map(|_| ())
            }
        };

        if let Err(error) = result {
            warn!(
                account = self.account_id,
                error = %error,
                "AntiDOS: failed to persist PacketSpoof ban"
            );
            self.pending_packet_spoof_ban_like_cpp = Some(plan);
        } else {
            self.kick_packet_spoof_affected_sessions_like_cpp(&affected_account_ids);
        }
    }

    async fn packet_spoof_ban_affected_account_ids_like_cpp(
        &self,
        login_db: &LoginDatabase,
        plan: &PacketSpoofPendingBanLikeCpp,
    ) -> Vec<u32> {
        match &plan.target {
            PacketSpoofPendingBanTargetLikeCpp::Account { account_id } => vec![*account_id],
            PacketSpoofPendingBanTargetLikeCpp::Ip { address } => {
                let mut stmt = login_db.prepare(LoginStatements::SEL_ACCOUNT_BY_IP);
                stmt.set_string(0, address);
                let mut result = match login_db.query(&stmt).await {
                    Ok(result) => result,
                    Err(error) => {
                        warn!(
                            account = self.account_id,
                            error = %error,
                            ip = address,
                            "AntiDOS: failed to query accounts affected by PacketSpoof IP ban"
                        );
                        return Vec::new();
                    }
                };

                if result.is_empty() {
                    return Vec::new();
                }

                let mut account_ids = Vec::with_capacity(result.count());
                loop {
                    account_ids.push(result.read(0));
                    if !result.next_row() {
                        break;
                    }
                }
                account_ids.sort_unstable();
                account_ids.dedup();
                account_ids
            }
        }
    }

    pub(super) fn kick_packet_spoof_affected_sessions_like_cpp(
        &self,
        affected_account_ids: &[u32],
    ) -> usize {
        if affected_account_ids.is_empty() {
            return 0;
        }
        let Some(registry) = self.player_registry() else {
            return 0;
        };

        let mut sent = 0usize;
        for (account_id, registration) in registry.registrations_for_accounts(affected_account_ids)
        {
            let command = SessionCommand::KickLikeCpp(KickLikeCppCommand {
                reason: "World::BanAccount Banning account".to_string(),
            });
            if let Err(error) = registry.try_send_current_command(registration, command) {
                warn!(
                    account = account_id,
                    ?error,
                    "AntiDOS: failed to queue PacketSpoof ban kick for affected session"
                );
                continue;
            }
            sent = sent.saturating_add(1);
        }
        sent
    }
}

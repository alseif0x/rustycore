// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for miscellaneous client opcodes:
//! SetSelection, AreaTrigger, RequestCemeteryList,
//! TaxiNodeStatusQuery, ChatJoinChannel, MoveTimeSkipped.

use tracing::{debug, info, warn};
use wow_constants::ClientOpcodes;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::misc::{RequestCemeteryListResponse, TaxiNodeStatusPkt};
use wow_packet::ServerPacket;

// ── inventory registrations ───────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetSelection,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_selection",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AreaTrigger,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_area_trigger",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::WorldPortResponse,
        status: SessionStatus::Transfer,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_world_port_response",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestCemeteryList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_cemetery_list",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TaxiNodeStatusQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_taxi_node_status_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatJoinChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_chat_join_channel",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveTimeSkipped,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_move_time_skipped",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryTime,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_time",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryNextMailTime,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_next_mail_time",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LoadingScreenNotify,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_loading_screen_notify",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ViolenceLevel,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_violence_level",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::OverrideScreenFlash,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_override_screen_flash",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueuedMessagesEnd,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_queued_messages_end",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatUnregisterAllAddonPrefixes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_chat_unregister_all_addon_prefixes",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetActionBarToggles,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_action_bar_toggles",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SaveCufProfiles,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_save_cuf_profiles",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildSetAchievementTracking,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_guild_set_achievement_tracking",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GetItemPurchaseData,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_get_item_purchase_data",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestForcedReactions,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_forced_reactions",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestBattlefieldStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_battlefield_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestRatedPvpInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_rated_pvp_info",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPvpRewards,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_pvp_rewards",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DfGetSystemInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_df_get_system_info",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DfGetJoinStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_df_get_join_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarGetNumPending,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_calendar_get_num_pending",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GmTicketGetCaseStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gm_ticket_get_case_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankRemainingWithdrawMoneyQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_guild_bank_remaining_withdraw_money_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetRequestJournal,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_battle_pet_request_journal",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamRoster,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_arena_team_roster",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestRaidInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_raid_info",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestConquestFormulaConstants,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_conquest_formula_constants",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestLfgListBlacklist,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_lfg_list_blacklist",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LfgListGetStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_lfg_list_get_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GetAccountCharacterList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_get_account_character_list",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCountdownTimer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_countdown_timer",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarGet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_calendar_get",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CloseInteraction,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_close_interaction",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListBidderItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auction_list_bidder_items",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListOwnerItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auction_list_owner_items",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListPendingSales,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auction_list_pending_sales",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CommerceTokenGetLog,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_commerce_token_get_log",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GameObjUse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_game_obj_use",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GameObjReportUse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_game_obj_report_use",
    }
}

// ── Handler implementations ───────────────────────────────────────────────────

impl crate::session::WorldSession {

    /// CMSG_SET_SELECTION — player selected a new target.
    /// C# ref: MiscHandler.HandleSetSelection → player.SetSelection(guid)
    pub async fn handle_set_selection(&mut self, mut pkt: wow_packet::WorldPacket) {
        let target_guid = pkt.read_packed_guid().unwrap_or(wow_core::ObjectGuid::EMPTY);
        self.selection_guid = Some(target_guid);
        info!(
            "SetSelection: account {} → {:?}",
            self.account_id, target_guid
        );
    }

    /// CMSG_WORLD_PORT_RESPONSE — client confirms it has loaded the new map.
    /// C# ref: MovementHandler.HandleMoveWorldportAck
    /// Sent after SMSG_TRANSFER_PENDING + SMSG_SUSPEND_TOKEN.
    /// We respond with SMSG_NEW_WORLD + SMSG_RESUME_TOKEN and resend world objects.
    pub async fn handle_world_port_response(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::{NewWorld, ResumeToken};

        let Some((new_map, new_pos)) = self.pending_teleport.take() else {
            warn!("WorldPortResponse from account {} but no pending teleport", self.account_id);
            self.set_state(crate::session::SessionState::LoggedIn);
            return;
        };

        info!(
            account = self.account_id,
            "WorldPortResponse: completing teleport to map {} ({:.2}, {:.2}, {:.2})",
            new_map, new_pos.x, new_pos.y, new_pos.z
        );

        // Update internal state
        self.current_map_id = new_map as u16;
        self.player_position = Some(new_pos);
        self.update_registry_position();

        // SMSG_NEW_WORLD — place player in new world
        self.send_packet(&NewWorld {
            map_id: new_map,
            pos: new_pos,
            reason: 0,
        });

        // SMSG_RESUME_TOKEN — resume movement processing
        self.send_packet(&ResumeToken { sequence_index: 1, reason: 1 });

        // Back to LoggedIn — handler dispatch resumes
        self.set_state(crate::session::SessionState::LoggedIn);

        // Resend nearby world objects at new position
        self.send_nearby_creatures(new_map as u16, &new_pos, 0).await;
        self.send_nearby_gameobjects(new_map as u16, &new_pos, 0).await;
    }

    /// CMSG_AREA_TRIGGER — player entered an area trigger.
    /// C# ref: MiscHandler.HandleAreaTrigger
    pub async fn handle_area_trigger(&mut self, mut pkt: wow_packet::WorldPacket) {
        let trigger_id: u32 = pkt.read_uint32().unwrap_or(0);
        
        info!(
            "AreaTrigger: account {} trigger_id={}",
            self.account_id, trigger_id
        );
        
        // Lookup in area trigger store
        if let Some(store) = self.area_trigger_store() {
            if let Some(trigger) = store.get_trigger(trigger_id) {
                info!(
                    "AreaTrigger {} detected at map {} pos ({}, {}, {})",
                    trigger_id, trigger.map_id, trigger.pos.x, trigger.pos.y, trigger.pos.z
                );
                
                if let Some(ref teleport) = trigger.teleport {
                    let target_map = teleport.target_map;
                    let target_pos = teleport.target_position;
                    info!(
                        "AreaTrigger {} → teleport to map {} ({:.2}, {:.2}, {:.2})",
                        trigger_id, target_map, target_pos.x, target_pos.y, target_pos.z
                    );
                    self.teleport_to(target_map, target_pos).await;
                }
            } else {
                debug!("Unknown area trigger ID {}", trigger_id);
            }
        } else {
            debug!("Area trigger store not available");
        }
    }

    /// CMSG_REQUEST_CEMETERY_LIST — client asks for graveyards in zone.
    /// C# ref: CharacterHandler.HandleRequestCemeteryList
    /// Returns empty list until graveyard data is implemented.
    pub async fn handle_request_cemetery_list(&mut self, mut pkt: wow_packet::WorldPacket) {
        let is_gossip: bool = pkt.read_uint8().unwrap_or(0) != 0;
        self.send_packet(&RequestCemeteryListResponse::empty(is_gossip));
    }

    /// CMSG_TAXI_NODE_STATUS_QUERY — client asks status of a taxi NPC.
    ///
    /// C# ref: `TaxiHandler.SendTaxiStatus`:
    ///   0 = None (no node found), 1 = Learned, 2 = Unlearned, 3 = NotEligible.
    ///
    /// Without a full taxi mask we default to:
    ///   - NPCFlags includes FlightMaster (0x2000) → `Unlearned` (2)
    ///     so the taxi icon shows as available.
    ///   - Otherwise → `None` (0).
    pub async fn handle_taxi_node_status_query(&mut self, mut pkt: wow_packet::WorldPacket) {
        let unit_guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("TaxiNodeStatusQuery: failed to read unit GUID");
                return;
            }
        };

        const NPC_FLAG_FLIGHT_MASTER: u32 = 0x2000;
        let is_flight_master = self.creatures
            .get(&unit_guid)
            .map(|c| c.npc_flags & NPC_FLAG_FLIGHT_MASTER != 0)
            .unwrap_or(false);

        // TaxiNodeStatus: 0=None, 1=Learned, 2=Unlearned, 3=NotEligible
        let status: u8 = if is_flight_master { 2 } else { 0 };

        debug!(
            account = self.account_id,
            ?unit_guid,
            status,
            "TaxiNodeStatusQuery"
        );
        self.send_packet(&TaxiNodeStatusPkt { unit_guid, status });
    }

    /// CMSG_CHAT_JOIN_CHANNEL — player joins a chat channel.
    /// C# ref: ChannelHandler.HandleJoinChannel
    /// Stubbed until ChannelManager is implemented.
    pub async fn handle_chat_join_channel(&mut self, _pkt: wow_packet::WorldPacket) {
        // TODO: parse channel packet and join via ChannelManager.
        // Packet structure (bit-packed): channel_id u32, has_voice bit,
        // name_len bits(7), pass_len bits(7), channel_name, password.
    }

    /// CMSG_MOVE_TIME_SKIPPED — client reports skipped movement time.
    /// C# ref: MovementHandler.HandleMoveTimeSkipped
    /// Stubbed until movement broadcast is implemented.
    pub async fn handle_move_time_skipped(&mut self, _pkt: wow_packet::WorldPacket) {
        // TODO: update mover.m_movementInfo.Time and broadcast MoveSkipTime.
    }

    // ── QueryTime ─────────────────────────────────────────────────────────────

    /// CMSG_QUERY_TIME — client requests current server time.
    /// C# ref: QueryHandler.HandleQueryTime → SendQueryTimeResponse
    pub async fn handle_query_time(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        use wow_packet::packets::misc::QueryTimeResponse;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        self.send_packet(&QueryTimeResponse { current_time: ts });
    }

    // ── QueryNextMailTime ──────────────────────────────────────────────────────

    /// CMSG_QUERY_NEXT_MAIL_TIME — client asks when next mail arrives.
    /// C# ref: MailHandler.HandleQueryNextMailTime
    /// Returns "no mail" (-1.0) until mail system is implemented.
    pub async fn handle_query_next_mail_time(&mut self) {
        use wow_packet::packets::misc::MailQueryNextTimeResult;
        self.send_packet(&MailQueryNextTimeResult::no_mail());
    }

    // ── Silent-ignore stubs ────────────────────────────────────────────────────
    // These opcodes are sent by the client at login but require no server
    // response at this stage (UI state, client-side settings, system queries
    // that return empty data until the respective subsystems are implemented).

    pub async fn handle_loading_screen_notify(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_violence_level(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_override_screen_flash(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_queued_messages_end(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_chat_unregister_all_addon_prefixes(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_set_action_bar_toggles(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_save_cuf_profiles(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_guild_set_achievement_tracking(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_get_item_purchase_data(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_forced_reactions(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_battlefield_status(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_rated_pvp_info(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_pvp_rewards(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_df_get_system_info(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_df_get_join_status(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_calendar_get_num_pending(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_gm_ticket_get_case_status(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_guild_bank_remaining_withdraw_money_query(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_battle_pet_request_journal(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_arena_team_roster(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_raid_info(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_conquest_formula_constants(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_lfg_list_blacklist(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_lfg_list_get_status(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_get_account_character_list(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_countdown_timer(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_calendar_get(&mut self, _pkt: wow_packet::WorldPacket) {}

    // ── Auction house list stubs ──────────────────────────────────────────────

    /// CMSG_AUCTION_LIST_BIDDER_ITEMS — list items bid on.
    /// Returns empty list until AH system is implemented.
    pub async fn handle_auction_list_bidder_items(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListBidderItemsResult;
        self.send_packet(&AuctionListBidderItemsResult);
    }

    /// CMSG_AUCTION_LIST_OWNER_ITEMS — list items the player put up for auction.
    /// Returns empty list until AH system is implemented.
    pub async fn handle_auction_list_owner_items(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListOwnerItemsResult;
        self.send_packet(&AuctionListOwnerItemsResult);
    }

    /// CMSG_AUCTION_LIST_PENDING_SALES — list pending sales / completed auctions.
    /// Returns empty list until AH system is implemented.
    pub async fn handle_auction_list_pending_sales(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListPendingSalesResult;
        self.send_packet(&AuctionListPendingSalesResult);
    }

    /// CMSG_COMMERCE_TOKEN_GET_LOG — WoW Token transaction log.
    /// Not implemented; silently ignored.
    pub async fn handle_commerce_token_get_log(&mut self, _pkt: wow_packet::WorldPacket) {}

    // ── Game object interaction ───────────────────────────────────────────────

    /// CMSG_GAME_OBJ_USE — player interacts with a world game object.
    /// C# ref: SpellHandler.HandleGameObjectUse → obj.Use(player)
    /// TODO: implement per-type object behavior (mailbox, door, chest, etc.)
    pub async fn handle_game_obj_use(&mut self, _pkt: wow_packet::WorldPacket) {
        info!("GameObjUse account {} (stub)", self.account_id);
    }

    /// CMSG_GAME_OBJ_REPORT_USE — client reports a game object use event.
    /// C# ref: SpellHandler.HandleGameobjectReportUse → UpdateCriteria
    pub async fn handle_game_obj_report_use(&mut self, _pkt: wow_packet::WorldPacket) {}

    /// CMSG_CLOSE_INTERACTION — player closed an NPC interaction window.
    /// C# ref: MiscHandler.HandleCloseInteraction → resets interaction data.
    pub async fn handle_close_interaction(&mut self, _pkt: wow_packet::WorldPacket) {
        // TODO: reset PlayerTalkClass interaction data and stable master.
    }
}

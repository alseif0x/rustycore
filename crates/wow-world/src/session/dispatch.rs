// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{
    ClientOpcodes, PacketProcessing, SessionState, SessionStatus, WorldPacket, debug, info, trace,
    warn,
};
use wow_packet::ClientPacket;

impl super::WorldSession {
    /// Dispatch a single packet to its registered handler.
    pub(crate) async fn dispatch_packet(&mut self, mut pkt: WorldPacket) {
        let opcode_raw = pkt.opcode_raw();
        let opcode: ClientOpcodes = match num_traits::FromPrimitive::from_u32(u32::from(opcode_raw))
        {
            Some(op) => op,
            None => {
                info!(
                    "Unknown client opcode 0x{opcode_raw:04X} from account {}",
                    self.account_id
                );
                return;
            }
        };

        let entry = match self.dispatch_table.get(&opcode) {
            Some(e) => *e,
            None => {
                info!(
                    "No handler for {:?} (0x{opcode_raw:04X}) from account {}",
                    opcode, self.account_id
                );
                return;
            }
        };

        // Check session status
        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
            && opcode == ClientOpcodes::RequestCemeteryList
        {
            info!(
                account = self.account_id,
                state = ?self.state,
                required = ?entry.status,
                handler = entry.handler_name,
                "RUST_CEMETERY_TRACE dispatch reached status gate"
            );
        }
        if !self.is_status_allowed(entry.status) {
            warn!(
                "Handler {} rejected: session state {:?} doesn't match required {:?}",
                entry.handler_name, self.state, entry.status
            );
            return;
        }

        debug!(
            "Dispatching {:?} via {} for account {}",
            opcode, entry.handler_name, self.account_id
        );

        // Skip opcode before reading payload
        pkt.skip_opcode();
        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
            && opcode == ClientOpcodes::RequestCemeteryList
        {
            info!(
                account = self.account_id,
                state = ?self.state,
                packet_size = pkt.size(),
                remaining = pkt.remaining(),
                read_position = pkt.read_position(),
                "RUST_CEMETERY_TRACE skipped opcode"
            );
        }

        match opcode {
            ClientOpcodes::EnumCharacters => {
                self.handle_enum_characters().await;
            }
            ClientOpcodes::CreateCharacter => {
                match wow_packet::packets::character::CreateCharacter::read(&mut pkt) {
                    Ok(create) => self.handle_create_character(create).await,
                    Err(e) => warn!("Failed to read CreateCharacter: {e}"),
                }
            }
            ClientOpcodes::CharDelete => {
                match wow_packet::packets::character::CharDelete::read(&mut pkt) {
                    Ok(del) => self.handle_char_delete(del).await,
                    Err(e) => warn!("Failed to read CharDelete: {e}"),
                }
            }
            ClientOpcodes::CharacterRenameRequest => {
                match wow_packet::packets::character::CharacterRenameRequest::read(&mut pkt) {
                    Ok(rename) => self.handle_character_rename_request(rename).await,
                    Err(e) => warn!("Failed to read CharacterRenameRequest: {e}"),
                }
            }
            ClientOpcodes::CharCustomize => {
                match wow_packet::packets::character::CharCustomize::read(&mut pkt) {
                    Ok(customize) => self.handle_char_customize(customize).await,
                    Err(e) => warn!("Failed to read CharCustomize: {e}"),
                }
            }
            ClientOpcodes::PlayerLogin => {
                match wow_packet::packets::character::PlayerLogin::read(&mut pkt) {
                    Ok(login) => self.handle_player_login(login).await,
                    Err(e) => warn!("Failed to read PlayerLogin: {e}"),
                }
            }
            ClientOpcodes::ConnectToFailed => {
                match wow_packet::packets::auth::ConnectToFailed::read(&mut pkt) {
                    Ok(failed) => self.handle_connect_to_failed(failed).await,
                    Err(e) => warn!("Failed to read ConnectToFailed: {e}"),
                }
            }
            ClientOpcodes::GetUndeleteCharacterCooldownStatus => {
                self.handle_get_undelete_cooldown_status().await;
            }
            ClientOpcodes::AlterAppearance => {
                self.handle_alter_appearance(pkt).await;
            }
            ClientOpcodes::ConfirmBarbersChoice => {
                self.handle_confirm_barbers_choice(pkt).await;
            }
            ClientOpcodes::ConfirmRespecWipe => {
                self.handle_confirm_respec_wipe(pkt).await;
            }
            ClientOpcodes::LearnTalent => {
                self.handle_learn_talent(pkt).await;
            }
            ClientOpcodes::SetPlayerDeclinedNames => {
                self.handle_set_player_declined_names(pkt).await;
            }
            ClientOpcodes::SaveEquipmentSet => {
                self.handle_save_equipment_set(pkt).await;
            }
            ClientOpcodes::AssignEquipmentSetSpec => {
                self.handle_assign_equipment_set_spec(pkt).await;
            }
            ClientOpcodes::DeleteEquipmentSet => {
                self.handle_delete_equipment_set(pkt).await;
            }
            ClientOpcodes::UseEquipmentSet => {
                self.handle_use_equipment_set(pkt).await;
            }
            ClientOpcodes::AdventureMapStartQuest => {
                self.handle_adventure_map_start_quest(pkt).await;
            }
            ClientOpcodes::BattlenetRequest => {
                match wow_packet::packets::battlenet::BattlenetRequest::read(&mut pkt) {
                    Ok(req) => self.handle_battlenet_request(req).await,
                    Err(e) => warn!("Failed to read BattlenetRequest: {e}"),
                }
            }
            ClientOpcodes::ChangeRealmTicket => {
                match wow_packet::packets::battlenet::ChangeRealmTicket::read(&mut pkt) {
                    Ok(ticket) => self.handle_change_realm_ticket(ticket).await,
                    Err(e) => warn!("Failed to read ChangeRealmTicket: {e}"),
                }
            }
            ClientOpcodes::ServerTimeOffsetRequest => {
                self.handle_server_time_offset_request().await;
            }
            ClientOpcodes::RequestPlayedTime => {
                // TriggerScriptEvent: 1 byte bool — mirrors it back in the response.
                let trigger = pkt.read_uint8().unwrap_or(0) != 0;
                self.handle_request_played_time(trigger).await;
            }
            ClientOpcodes::SetSelection => {
                self.handle_set_selection(pkt).await;
            }
            ClientOpcodes::FarSight => {
                self.handle_far_sight(pkt).await;
            }
            ClientOpcodes::AreaTrigger => {
                self.handle_area_trigger(pkt).await;
            }
            ClientOpcodes::RequestCemeteryList => {
                if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
                    info!(
                        account = self.account_id,
                        state = ?self.state,
                        "RUST_CEMETERY_TRACE before handler call"
                    );
                }
                self.handle_request_cemetery_list(pkt).await;
                if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
                    info!(
                        account = self.account_id,
                        state = ?self.state,
                        "RUST_CEMETERY_TRACE after handler call"
                    );
                }
            }
            ClientOpcodes::ResurrectResponse => {
                self.handle_resurrect_response(pkt).await;
            }
            ClientOpcodes::TaxiNodeStatusQuery => {
                self.handle_taxi_node_status_query(pkt).await;
            }
            ClientOpcodes::ActivateTaxi => {
                self.handle_activate_taxi(pkt).await;
            }
            ClientOpcodes::ChatJoinChannel => {
                self.handle_chat_join_channel(pkt).await;
            }
            ClientOpcodes::ChatLeaveChannel => {
                self.handle_chat_leave_channel(pkt).await;
            }
            ClientOpcodes::ChatChannelAnnouncements
            | ClientOpcodes::ChatChannelDeclineInvite
            | ClientOpcodes::ChatChannelDisplayList
            | ClientOpcodes::ChatChannelList
            | ClientOpcodes::ChatChannelOwner => {
                self.handle_chat_channel_command(pkt).await;
            }
            ClientOpcodes::ChatChannelBan
            | ClientOpcodes::ChatChannelInvite
            | ClientOpcodes::ChatChannelKick
            | ClientOpcodes::ChatChannelModerator
            | ClientOpcodes::ChatChannelSetOwner
            | ClientOpcodes::ChatChannelSilenceAll
            | ClientOpcodes::ChatChannelUnban
            | ClientOpcodes::ChatChannelUnmoderator
            | ClientOpcodes::ChatChannelUnsilenceAll => {
                self.handle_chat_channel_player_command(pkt).await;
            }
            ClientOpcodes::ChatChannelPassword => {
                self.handle_chat_channel_password(pkt).await;
            }
            ClientOpcodes::DbQueryBulk => {
                match wow_packet::packets::misc::DbQueryBulk::read(&mut pkt) {
                    Ok(query) => self.handle_db_query_bulk(query).await,
                    Err(e) => warn!("Failed to read DbQueryBulk: {e}"),
                }
            }
            ClientOpcodes::HotfixRequest => {
                match wow_packet::packets::misc::HotfixRequest::read(&mut pkt) {
                    Ok(req) => self.handle_hotfix_request(req).await,
                    Err(e) => warn!("Failed to read HotfixRequest: {e}"),
                }
            }
            ClientOpcodes::TimeSyncResponse
            | ClientOpcodes::TimeSyncResponseDropped
            | ClientOpcodes::TimeSyncResponseFailed => {
                match wow_packet::packets::misc::TimeSyncResponse::read(&mut pkt) {
                    Ok(resp) => self.handle_time_sync_response(resp).await,
                    Err(e) => warn!("Failed to read TimeSyncResponse: {e}"),
                }
            }
            ClientOpcodes::LogoutRequest => {
                match wow_packet::packets::misc::LogoutRequest::read(&mut pkt) {
                    Ok(req) => self.handle_logout_request(req).await,
                    Err(e) => warn!("Failed to read LogoutRequest: {e}"),
                }
            }
            ClientOpcodes::LogoutCancel => {
                self.handle_logout_cancel().await;
            }
            ClientOpcodes::RepopRequest => {
                self.handle_repop_request(pkt).await;
            }
            ClientOpcodes::ReclaimCorpse => {
                self.handle_reclaim_corpse(pkt).await;
            }
            ClientOpcodes::QueryCreature => {
                match wow_packet::packets::query::QueryCreature::read(&mut pkt) {
                    Ok(query) => self.handle_query_creature(query).await,
                    Err(e) => warn!("Failed to read QueryCreature: {e}"),
                }
            }

            ClientOpcodes::QueryGameObject => {
                match wow_packet::packets::query::QueryGameObject::read(&mut pkt) {
                    Ok(query) => self.handle_query_game_object(query).await,
                    Err(e) => warn!("Failed to read QueryGameObject: {e}"),
                }
            }
            ClientOpcodes::QueryCorpseLocationFromClient => {
                match wow_packet::packets::query::QueryCorpseLocationFromClient::read(&mut pkt) {
                    Ok(query) => self.handle_query_corpse_location(query).await,
                    Err(e) => warn!("Failed to read QueryCorpseLocationFromClient: {e}"),
                }
            }
            ClientOpcodes::QueryCorpseTransport => {
                match wow_packet::packets::query::QueryCorpseTransport::read(&mut pkt) {
                    Ok(query) => self.handle_query_corpse_transport(query).await,
                    Err(e) => warn!("Failed to read QueryCorpseTransport: {e}"),
                }
            }
            ClientOpcodes::QueryPageText => {
                match wow_packet::packets::query::QueryPageText::read(&mut pkt) {
                    Ok(query) => self.handle_query_page_text(query).await,
                    Err(e) => warn!("Failed to read QueryPageText: {e}"),
                }
            }
            ClientOpcodes::ItemTextQuery => {
                match wow_packet::packets::query::ItemTextQuery::read(&mut pkt) {
                    Ok(query) => self.handle_item_text_query(query).await,
                    Err(e) => warn!("Failed to read ItemTextQuery: {e}"),
                }
            }
            ClientOpcodes::QueryPetName => {
                match wow_packet::packets::query::QueryPetName::read(&mut pkt) {
                    Ok(query) => self.handle_query_pet_name(query).await,
                    Err(e) => warn!("Failed to read QueryPetName: {e}"),
                }
            }
            ClientOpcodes::QueryPlayerNames => {
                match wow_packet::packets::query::QueryPlayerNames::read(&mut pkt) {
                    Ok(query) => self.handle_query_player_names(query).await,
                    Err(e) => warn!("Failed to read QueryPlayerNames: {e}"),
                }
            }
            ClientOpcodes::QueryRealmName => {
                match wow_packet::packets::query::QueryRealmName::read(&mut pkt) {
                    Ok(query) => self.handle_query_realm_name(query),
                    Err(e) => warn!("Failed to read QueryRealmName: {e}"),
                }
            }
            ClientOpcodes::QueryQuestCompletionNpcs => {
                match wow_packet::packets::query::QueryQuestCompletionNpcs::read(&mut pkt) {
                    Ok(query) => self.handle_query_quest_completion_npcs(query).await,
                    Err(e) => warn!("Failed to read QueryQuestCompletionNpcs: {e}"),
                }
            }
            ClientOpcodes::QuestPoiQuery => {
                match wow_packet::packets::query::QuestPoiQuery::read(&mut pkt) {
                    Ok(query) => self.handle_quest_poi_query(query).await,
                    Err(e) => warn!("Failed to read QuestPoiQuery: {e}"),
                }
            }
            ClientOpcodes::Ping => match wow_packet::packets::auth::Ping::read(&mut pkt) {
                Ok(ping) => self.handle_ping(ping).await,
                Err(e) => warn!("Failed to read Ping: {e}"),
            },
            ClientOpcodes::TalkToGossip => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_gossip_hello(hello).await,
                    Err(e) => warn!("Failed to read TalkToGossip: {e}"),
                }
            }
            ClientOpcodes::AuctionHelloRequest => {
                self.handle_auction_hello_request(pkt).await;
            }
            ClientOpcodes::BankerActivate => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_banker_activate(hello).await,
                    Err(e) => warn!("Failed to read BankerActivate: {e}"),
                }
            }
            ClientOpcodes::AutobankItem => {
                match wow_packet::packets::misc::AutoBankItem::read(&mut pkt) {
                    Ok(packet) => self.handle_autobank_item(packet).await,
                    Err(e) => warn!("Failed to read AutobankItem: {e}"),
                }
            }
            ClientOpcodes::AutostoreBankItem => {
                match wow_packet::packets::misc::AutoStoreBankItem::read(&mut pkt) {
                    Ok(packet) => self.handle_autostore_bank_item(packet).await,
                    Err(e) => warn!("Failed to read AutostoreBankItem: {e}"),
                }
            }
            ClientOpcodes::BuyBankSlot => {
                match wow_packet::packets::misc::BuyBankSlot::read(&mut pkt) {
                    Ok(buy) => self.handle_buy_bank_slot(buy).await,
                    Err(e) => warn!("Failed to read BuyBankSlot: {e}"),
                }
            }
            ClientOpcodes::ChangeBankBagSlotFlag => {
                match wow_packet::packets::misc::ChangeBankBagSlotFlag::read(&mut pkt) {
                    Ok(change) => self.handle_change_bank_bag_slot_flag(change).await,
                    Err(e) => warn!("Failed to read ChangeBankBagSlotFlag: {e}"),
                }
            }
            ClientOpcodes::BinderActivate => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_binder_activate(hello).await,
                    Err(e) => warn!("Failed to read BinderActivate: {e}"),
                }
            }
            ClientOpcodes::TabardVendorActivate => {
                self.handle_tabard_vendor_activate(pkt).await;
            }
            ClientOpcodes::SpiritHealerActivate => {
                self.handle_spirit_healer_activate(pkt).await;
            }
            ClientOpcodes::AreaSpiritHealerQuery => {
                self.handle_area_spirit_healer_query(pkt).await;
            }
            ClientOpcodes::AreaSpiritHealerQueue => {
                self.handle_area_spirit_healer_queue(pkt).await;
            }
            ClientOpcodes::HearthAndResurrect => {
                self.handle_hearth_and_resurrect(pkt).await;
            }
            ClientOpcodes::RepairItem => {
                match wow_packet::packets::misc::RepairItem::read(&mut pkt) {
                    Ok(repair) => self.handle_repair_item(repair).await,
                    Err(e) => warn!("Failed to read RepairItem: {e}"),
                }
            }
            ClientOpcodes::RequestStabledPets => {
                self.handle_request_stabled_pets(pkt).await;
            }
            ClientOpcodes::GossipSelectOption => {
                match wow_packet::packets::gossip::GossipSelectOption::read(&mut pkt) {
                    Ok(select) => self.handle_gossip_select_option(select).await,
                    Err(e) => warn!("Failed to read GossipSelectOption: {e}"),
                }
            }
            ClientOpcodes::QueryNpcText => {
                match wow_packet::packets::gossip::QueryNpcText::read(&mut pkt) {
                    Ok(query) => self.handle_query_npc_text(query).await,
                    Err(e) => warn!("Failed to read QueryNpcText: {e}"),
                }
            }
            ClientOpcodes::ListInventory => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_list_inventory(hello).await,
                    Err(e) => warn!("Failed to read ListInventory: {e}"),
                }
            }
            ClientOpcodes::BuyItem => match wow_packet::packets::misc::BuyItem::read(&mut pkt) {
                Ok(buy) => self.handle_buy_item(buy).await,
                Err(e) => warn!("Failed to read BuyItem: {e}"),
            },
            ClientOpcodes::BuyBackItem => {
                match wow_packet::packets::misc::BuyBackItem::read(&mut pkt) {
                    Ok(buyback) => self.handle_buy_back_item(buyback).await,
                    Err(e) => warn!("Failed to read BuyBackItem: {e}"),
                }
            }
            ClientOpcodes::SellItem => match wow_packet::packets::misc::SellItem::read(&mut pkt) {
                Ok(sell) => self.handle_sell_item(sell).await,
                Err(e) => warn!("Failed to read SellItem: {e}"),
            },
            ClientOpcodes::ItemPurchaseRefund => {
                match wow_packet::packets::item::ItemPurchaseRefund::read(&mut pkt) {
                    Ok(refund) => self.handle_item_purchase_refund(refund).await,
                    Err(e) => warn!("Failed to read ItemPurchaseRefund: {e}"),
                }
            }
            ClientOpcodes::TrainerList => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_trainer_list(hello).await,
                    Err(e) => warn!("Failed to read TrainerList: {e}"),
                }
            }
            ClientOpcodes::TrainerBuySpell => {
                self.handle_trainer_buy_spell(pkt).await;
            }
            ClientOpcodes::QuestGiverHello => {
                self.handle_quest_giver_hello(pkt).await;
            }
            ClientOpcodes::QuestGiverStatusQuery => {
                self.handle_quest_giver_status_query(pkt).await;
            }
            ClientOpcodes::QuestGiverStatusMultipleQuery => {
                self.handle_quest_giver_status_multiple_query().await;
            }
            ClientOpcodes::QuestGiverStatusTrackedQuery => {
                self.handle_quest_giver_status_tracked_query(pkt).await;
            }
            ClientOpcodes::QuestGiverQueryQuest => {
                self.handle_quest_giver_query_quest(pkt).await;
            }
            ClientOpcodes::QuestGiverAcceptQuest => {
                self.handle_quest_giver_accept_quest(pkt).await;
            }
            ClientOpcodes::QuestGiverRequestReward => {
                self.handle_quest_giver_request_reward(pkt).await;
            }
            ClientOpcodes::QuestGiverCompleteQuest => {
                self.handle_quest_giver_complete_quest(pkt).await;
            }
            ClientOpcodes::QuestGiverChooseReward => {
                self.handle_quest_giver_choose_reward(pkt).await;
            }
            ClientOpcodes::QuestGiverCloseQuest => {
                self.handle_quest_giver_close_quest(pkt).await;
            }
            ClientOpcodes::QueryQuestInfo => {
                self.handle_query_quest_info(pkt).await;
            }
            ClientOpcodes::RequestWorldQuestUpdate => {
                self.handle_request_world_quest_update(pkt).await;
            }
            ClientOpcodes::QuestConfirmAccept => {
                self.handle_quest_confirm_accept(pkt).await;
            }
            ClientOpcodes::QuestPushResult => {
                self.handle_quest_push_result(pkt).await;
            }
            ClientOpcodes::PushQuestToParty => {
                self.handle_push_quest_to_party(pkt).await;
            }
            ClientOpcodes::QuestLogRemoveQuest => {
                self.handle_quest_log_remove_quest(pkt).await;
            }
            ClientOpcodes::SwapInvItem => {
                match wow_packet::packets::item::SwapInvItem::read(&mut pkt) {
                    Ok(swap) => self.handle_swap_inv_item(swap).await,
                    Err(e) => warn!("Failed to read SwapInvItem: {e}"),
                }
            }
            ClientOpcodes::AutoEquipItem => {
                match wow_packet::packets::item::AutoEquipItem::read(&mut pkt) {
                    Ok(equip) => self.handle_auto_equip_item(equip).await,
                    Err(e) => warn!("Failed to read AutoEquipItem: {e}"),
                }
            }
            ClientOpcodes::AutoEquipItemSlot => {
                match wow_packet::packets::item::AutoEquipItemSlot::read(&mut pkt) {
                    Ok(equip) => self.handle_auto_equip_item_slot(equip).await,
                    Err(e) => warn!("Failed to read AutoEquipItemSlot: {e}"),
                }
            }
            ClientOpcodes::SwapItem => match wow_packet::packets::item::SwapItem::read(&mut pkt) {
                Ok(swap) => self.handle_swap_item(swap).await,
                Err(e) => warn!("Failed to read SwapItem: {e}"),
            },
            ClientOpcodes::AutoStoreBagItem => {
                match wow_packet::packets::item::AutoStoreBagItem::read(&mut pkt) {
                    Ok(store) => self.handle_auto_store_bag_item(store).await,
                    Err(e) => warn!("Failed to read AutoStoreBagItem: {e}"),
                }
            }
            ClientOpcodes::DestroyItem => {
                match wow_packet::packets::item::DestroyItemPkt::read(&mut pkt) {
                    Ok(destroy) => self.handle_destroy_item(destroy).await,
                    Err(e) => warn!("Failed to read DestroyItem: {e}"),
                }
            }
            ClientOpcodes::CancelTempEnchantment => {
                match wow_packet::packets::item::CancelTempEnchantment::read(&mut pkt) {
                    Ok(cancel) => self.handle_cancel_temp_enchantment(cancel).await,
                    Err(e) => warn!("Failed to read CancelTempEnchantment: {e}"),
                }
            }
            ClientOpcodes::ShowTradeSkill => {
                match wow_packet::packets::misc::ShowTradeSkill::read(&mut pkt) {
                    Ok(_) => self.handle_show_trade_skill().await,
                    Err(e) => warn!("Failed to read ShowTradeSkill: {e}"),
                }
            }
            // ── Movement opcodes (all share the same handler) ───────
            ClientOpcodes::MoveStartForward
            | ClientOpcodes::MoveStartBackward
            | ClientOpcodes::MoveStop
            | ClientOpcodes::MoveStartStrafeLeft
            | ClientOpcodes::MoveStartStrafeRight
            | ClientOpcodes::MoveStopStrafe
            | ClientOpcodes::MoveStartTurnLeft
            | ClientOpcodes::MoveStartTurnRight
            | ClientOpcodes::MoveStopTurn
            | ClientOpcodes::MoveStartPitchUp
            | ClientOpcodes::MoveStartPitchDown
            | ClientOpcodes::MoveStopPitch
            | ClientOpcodes::MoveSetRunMode
            | ClientOpcodes::MoveSetWalkMode
            | ClientOpcodes::MoveHeartbeat
            | ClientOpcodes::MoveFallLand
            | ClientOpcodes::MoveFallReset
            | ClientOpcodes::MoveJump
            | ClientOpcodes::MoveSetFacing
            | ClientOpcodes::MoveSetFacingHeartbeat
            | ClientOpcodes::MoveSetPitch
            | ClientOpcodes::MoveSetFly
            | ClientOpcodes::MoveStartAscend
            | ClientOpcodes::MoveStopAscend
            | ClientOpcodes::MoveStartDescend
            | ClientOpcodes::MoveStartSwim
            | ClientOpcodes::MoveStopSwim
            | ClientOpcodes::MoveUpdateFallSpeed => {
                self.handle_movement(pkt).await;
            }

            ClientOpcodes::MoveAddImpulseAck
            | ClientOpcodes::MoveApplyInertiaAck
            | ClientOpcodes::MoveRemoveInertiaAck
            | ClientOpcodes::MoveRemoveMovementForces
            | ClientOpcodes::MoveSeamlessTransferComplete
            | ClientOpcodes::MoveSetAdvFly
            | ClientOpcodes::MoveSetAdvFlyingAddImpulseMaxSpeedAck
            | ClientOpcodes::MoveSetAdvFlyingAirFrictionAck
            | ClientOpcodes::MoveSetAdvFlyingBankingRateAck
            | ClientOpcodes::MoveSetAdvFlyingDoubleJumpVelModAck
            | ClientOpcodes::MoveSetAdvFlyingGlideStartMinHeightAck
            | ClientOpcodes::MoveSetAdvFlyingLaunchSpeedCoefficientAck
            | ClientOpcodes::MoveSetAdvFlyingLiftCoefficientAck
            | ClientOpcodes::MoveSetAdvFlyingMaxVelAck
            | ClientOpcodes::MoveSetAdvFlyingOverMaxDecelerationAck
            | ClientOpcodes::MoveSetAdvFlyingPitchingRateDownAck
            | ClientOpcodes::MoveSetAdvFlyingPitchingRateUpAck
            | ClientOpcodes::MoveSetAdvFlyingSurfaceFrictionAck
            | ClientOpcodes::MoveSetAdvFlyingTurnVelocityThresholdAck => {
                self.handle_unhandled_client_null_like_cpp(pkt).await;
            }

            // ── Movement control opcodes ────────────────────────────
            ClientOpcodes::SetActiveMover => {
                match wow_packet::packets::movement::SetActiveMover::read(&mut pkt) {
                    Ok(mover) => self.handle_set_active_mover(mover).await,
                    Err(e) => warn!("Failed to read SetActiveMover: {e}"),
                }
            }
            ClientOpcodes::MoveInitActiveMoverComplete => {
                match wow_packet::packets::movement::MoveInitActiveMoverComplete::read(&mut pkt) {
                    Ok(init) => self.handle_move_init_active_mover_complete(init).await,
                    Err(e) => warn!("Failed to read MoveInitActiveMoverComplete: {e}"),
                }
            }
            // C++ HandleSuspendTokenResponse (MovementHandler.cpp:239): on the client's
            // suspend ack during a far teleport, send SMSG_NEW_WORLD so it loads the
            // destination map. Then the client sends CMSG_WORLD_PORT_RESPONSE.
            ClientOpcodes::SuspendTokenResponse => {
                self.handle_suspend_token_response(pkt).await;
            }
            // C++ HandleMoveWorldportAck (MovementHandler.cpp:49): client finished loading the
            // new map; resume + replay init. (Inventory entry alone never reached the method —
            // the match arm was missing, so far teleports never completed. #NEXT.R8.ENTITIES.1229.)
            ClientOpcodes::WorldPortResponse => {
                self.handle_world_port_response(pkt).await;
            }
            ClientOpcodes::MoveSetVehicleRecIdAck => {
                let opcode = pkt.client_opcode().unwrap_or(opcode);
                match wow_packet::packets::vehicle::MoveSetVehicleRecIdAck::read(&mut pkt) {
                    Ok(ack) => self.handle_move_set_vehicle_rec_id_ack(opcode, ack).await,
                    Err(e) => warn!("Failed to read MoveSetVehicleRecIdAck: {e}"),
                }
            }
            ClientOpcodes::MoveDismissVehicle => {
                match wow_packet::packets::vehicle::MoveDismissVehicle::read(&mut pkt) {
                    Ok(packet) => self.handle_move_dismiss_vehicle(packet).await,
                    Err(e) => warn!("Failed to read MoveDismissVehicle: {e}"),
                }
            }
            ClientOpcodes::RequestVehiclePrevSeat => {
                match wow_packet::packets::vehicle::RequestVehiclePrevSeat::read(&mut pkt) {
                    Ok(packet) => self.handle_request_vehicle_prev_seat(packet).await,
                    Err(e) => warn!("Failed to read RequestVehiclePrevSeat: {e}"),
                }
            }
            ClientOpcodes::RequestVehicleNextSeat => {
                match wow_packet::packets::vehicle::RequestVehicleNextSeat::read(&mut pkt) {
                    Ok(packet) => self.handle_request_vehicle_next_seat(packet).await,
                    Err(e) => warn!("Failed to read RequestVehicleNextSeat: {e}"),
                }
            }
            ClientOpcodes::MoveChangeVehicleSeats => {
                match wow_packet::packets::vehicle::MoveChangeVehicleSeats::read(&mut pkt) {
                    Ok(packet) => self.handle_move_change_vehicle_seats(packet).await,
                    Err(e) => warn!("Failed to read MoveChangeVehicleSeats: {e}"),
                }
            }
            ClientOpcodes::RequestVehicleSwitchSeat => {
                match wow_packet::packets::vehicle::RequestVehicleSwitchSeat::read(&mut pkt) {
                    Ok(packet) => self.handle_request_vehicle_switch_seat(packet).await,
                    Err(e) => warn!("Failed to read RequestVehicleSwitchSeat: {e}"),
                }
            }
            ClientOpcodes::RideVehicleInteract => {
                match wow_packet::packets::vehicle::RideVehicleInteract::read(&mut pkt) {
                    Ok(packet) => self.handle_ride_vehicle_interact(packet).await,
                    Err(e) => warn!("Failed to read RideVehicleInteract: {e}"),
                }
            }
            ClientOpcodes::EjectPassenger => {
                match wow_packet::packets::vehicle::EjectPassenger::read(&mut pkt) {
                    Ok(packet) => self.handle_eject_passenger(packet).await,
                    Err(e) => warn!("Failed to read EjectPassenger: {e}"),
                }
            }
            ClientOpcodes::RequestVehicleExit => {
                match wow_packet::packets::vehicle::RequestVehicleExit::read(&mut pkt) {
                    Ok(packet) => self.handle_request_vehicle_exit(packet).await,
                    Err(e) => warn!("Failed to read RequestVehicleExit: {e}"),
                }
            }
            ClientOpcodes::MoveCollisionDisableAck
            | ClientOpcodes::MoveCollisionEnableAck
            | ClientOpcodes::MoveEnableDoubleJumpAck
            | ClientOpcodes::MoveEnableSwimToFlyTransAck
            | ClientOpcodes::MoveFeatherFallAck
            | ClientOpcodes::MoveForceRootAck
            | ClientOpcodes::MoveForceUnrootAck
            | ClientOpcodes::MoveGravityDisableAck
            | ClientOpcodes::MoveGravityEnableAck
            | ClientOpcodes::MoveHoverAck
            | ClientOpcodes::MoveInertiaDisableAck
            | ClientOpcodes::MoveInertiaEnableAck
            | ClientOpcodes::MoveSetCanFlyAck
            | ClientOpcodes::MoveSetCanTurnWhileFallingAck
            | ClientOpcodes::MoveSetIgnoreMovementForcesAck
            | ClientOpcodes::MoveWaterWalkAck => {
                let opcode = pkt.client_opcode().unwrap_or(opcode);
                match wow_packet::packets::movement::MovementAckMessage::read(&mut pkt) {
                    Ok(ack) => self.handle_movement_ack_message(opcode, ack).await,
                    Err(e) => warn!("Failed to read MovementAckMessage: {e}"),
                }
            }
            ClientOpcodes::MoveForceWalkSpeedChangeAck
            | ClientOpcodes::MoveForceRunSpeedChangeAck
            | ClientOpcodes::MoveForceRunBackSpeedChangeAck
            | ClientOpcodes::MoveForceSwimSpeedChangeAck
            | ClientOpcodes::MoveForceSwimBackSpeedChangeAck
            | ClientOpcodes::MoveForceTurnRateChangeAck
            | ClientOpcodes::MoveForceFlightSpeedChangeAck
            | ClientOpcodes::MoveForceFlightBackSpeedChangeAck
            | ClientOpcodes::MoveForcePitchRateChangeAck
            | ClientOpcodes::MoveSetModMovementForceMagnitudeAck => {
                let opcode = pkt.client_opcode().unwrap_or(opcode);
                match wow_packet::packets::movement::MovementSpeedAck::read(&mut pkt) {
                    Ok(ack) => self.handle_movement_speed_ack(opcode, ack).await,
                    Err(e) => warn!("Failed to read MovementSpeedAck: {e}"),
                }
            }
            ClientOpcodes::MoveKnockBackAck => {
                match wow_packet::packets::movement::MoveKnockBackAck::read(&mut pkt) {
                    Ok(ack) => self.handle_move_knock_back_ack(ack).await,
                    Err(e) => warn!("Failed to read MoveKnockBackAck: {e}"),
                }
            }
            ClientOpcodes::MoveSetCollisionHeightAck => {
                match wow_packet::packets::movement::MoveSetCollisionHeightAck::read(&mut pkt) {
                    Ok(ack) => self.handle_move_set_collision_height_ack(ack).await,
                    Err(e) => warn!("Failed to read MoveSetCollisionHeightAck: {e}"),
                }
            }
            ClientOpcodes::MoveApplyMovementForceAck => {
                match wow_packet::packets::movement::MoveApplyMovementForceAck::read(&mut pkt) {
                    Ok(ack) => self.handle_move_apply_movement_force_ack(ack).await,
                    Err(e) => warn!("Failed to read MoveApplyMovementForceAck: {e}"),
                }
            }
            ClientOpcodes::MoveRemoveMovementForceAck => {
                match wow_packet::packets::movement::MoveRemoveMovementForceAck::read(&mut pkt) {
                    Ok(ack) => self.handle_move_remove_movement_force_ack(ack).await,
                    Err(e) => warn!("Failed to read MoveRemoveMovementForceAck: {e}"),
                }
            }
            ClientOpcodes::MoveTimeSkipped => {
                match wow_packet::packets::movement::MoveTimeSkipped::read(&mut pkt) {
                    Ok(skipped) => self.handle_move_time_skipped(skipped).await,
                    Err(e) => warn!("Failed to read MoveTimeSkipped: {e}"),
                }
            }
            ClientOpcodes::MoveSplineDone => {
                match wow_packet::packets::movement::MoveSplineDone::read(&mut pkt) {
                    Ok(done) => self.handle_move_spline_done(done).await,
                    Err(e) => warn!("Failed to read MoveSplineDone: {e}"),
                }
            }
            ClientOpcodes::MoveTeleportAck => {
                match wow_packet::packets::movement::MoveTeleportAck::read(&mut pkt) {
                    Ok(ack) => self.handle_move_teleport_ack(ack).await,
                    Err(e) => warn!("Failed to read MoveTeleportAck: {e}"),
                }
            }

            // ── Combat opcodes ──────────────────────────────────────
            ClientOpcodes::AttackSwing => {
                self.handle_attack_swing(pkt).await;
            }
            ClientOpcodes::AttackStop => {
                self.handle_attack_stop(pkt).await;
            }
            ClientOpcodes::SetSheathed => {
                self.handle_set_sheathed(pkt);
            }

            // ── Loot opcodes ────────────────────────────────────────
            ClientOpcodes::LootUnit => {
                self.handle_loot_unit(pkt).await;
            }
            ClientOpcodes::LootItem => {
                self.handle_loot_item(pkt).await;
            }
            ClientOpcodes::LootMoney => {
                self.handle_loot_money(pkt).await;
            }
            ClientOpcodes::LootRelease => {
                self.handle_loot_release(pkt).await;
            }
            ClientOpcodes::LootRoll => match wow_packet::packets::loot::LootRoll::read(&mut pkt) {
                Ok(roll) => self.handle_loot_roll(roll).await,
                Err(e) => warn!("Failed to read LootRoll: {e}"),
            },
            ClientOpcodes::MasterLootItem => {
                match wow_packet::packets::loot::MasterLootItem::read(&mut pkt) {
                    Ok(master_loot_item) => self.handle_master_loot_item(master_loot_item).await,
                    Err(e) => warn!("Failed to read MasterLootItem: {e}"),
                }
            }
            ClientOpcodes::SetLootSpecialization => {
                // The inspected TrinityCore opcode table assigns the shared
                // unresolved 0xBADD placeholder to both
                // CMSG_CLEAR_RAID_MARKER (uint8 payload) and
                // CMSG_SET_LOOT_SPECIALIZATION (uint32 payload), and this fork
                // also assigns it to CMSG_SET_SAVED_INSTANCE_EXTEND
                // (int32+uint32+bit payload) and
                // CMSG_CANCEL_MOD_SPEED_NO_CONTROL_AURAS (packed GUID payload)
                // plus CMSG_CLIENT_PORT_GRAVEYARD (empty payload).
                // Rust keeps one enum variant and
                // splits by payload length until the real opcode table is
                // resolved.
                if self
                    .try_handle_cancel_mod_speed_no_control_auras_like_cpp(pkt.clone())
                    .await
                {
                    return;
                }
                if self
                    .try_handle_client_port_graveyard_like_cpp(pkt.clone())
                    .await
                {
                    return;
                }
                if pkt.remaining() == 1 {
                    self.handle_clear_raid_marker(pkt).await;
                } else if pkt.remaining() == 4 {
                    match wow_packet::packets::loot::SetLootSpecialization::read(&mut pkt) {
                        Ok(set_loot_specialization) => {
                            self.handle_set_loot_specialization(set_loot_specialization)
                                .await;
                        }
                        Err(e) => warn!("Failed to read SetLootSpecialization: {e}"),
                    }
                } else if pkt.remaining() == 9 {
                    match wow_packet::packets::misc::SetSavedInstanceExtend::read(&mut pkt) {
                        Ok(query) => self.handle_set_saved_instance_extend(query).await,
                        Err(e) => warn!("Failed to read SetSavedInstanceExtend: {e}"),
                    }
                } else {
                    warn!(
                        opcode = ?ClientOpcodes::SetLootSpecialization,
                        remaining = pkt.remaining(),
                        "unresolved 0xBADD payload shape"
                    );
                }
            }

            // ── Chat opcodes ────────────────────────────────────────
            ClientOpcodes::ChatMessageSay => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Say)
                    .await;
            }
            ClientOpcodes::ChatMessageYell => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Yell)
                    .await;
            }
            ClientOpcodes::ChatMessageParty => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Party)
                    .await;
            }
            ClientOpcodes::ChatMessageGuild => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Guild)
                    .await;
            }
            ClientOpcodes::ChatMessageOfficer => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Officer)
                    .await;
            }
            ClientOpcodes::ChatMessageRaid => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Raid)
                    .await;
            }
            ClientOpcodes::ChatMessageRaidWarning => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::RaidWarning)
                    .await;
            }
            ClientOpcodes::ChatMessageInstanceChat => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::InstanceChat)
                    .await;
            }
            ClientOpcodes::ChatMessageWhisper => {
                self.handle_chat_whisper(pkt).await;
            }
            ClientOpcodes::ChatMessageChannel => {
                self.handle_chat_channel_message(pkt).await;
            }
            ClientOpcodes::ChatMessageAfk => {
                self.handle_chat_afk(pkt).await;
            }
            ClientOpcodes::ChatMessageDnd => {
                self.handle_chat_dnd(pkt).await;
            }
            ClientOpcodes::ChatReportIgnored => {
                self.handle_chat_report_ignored(pkt).await;
            }
            ClientOpcodes::ChatReportFiltered => {
                self.handle_chat_report_filtered(pkt).await;
            }
            ClientOpcodes::UpdateAadcStatus => {
                self.handle_update_aadc_status(pkt).await;
            }
            ClientOpcodes::ChatMessageEmote => {
                self.handle_chat_emote(pkt).await;
            }
            ClientOpcodes::Emote => {
                self.handle_emote(pkt).await;
            }
            ClientOpcodes::SendTextEmote => {
                self.handle_text_emote(pkt).await;
            }
            ClientOpcodes::ChatRegisterAddonPrefixes => {
                self.handle_chat_register_addon_prefixes(pkt).await;
            }
            ClientOpcodes::ChatAddonMessage => {
                self.handle_chat_addon_message(pkt).await;
            }
            ClientOpcodes::ChatAddonMessageWhisper => {
                self.handle_chat_addon_message_whisper(pkt).await;
            }

            // ── Spell cast ────────────────────────────────────────────────────
            ClientOpcodes::CastSpell => {
                self.handle_cast_spell(pkt).await;
            }
            ClientOpcodes::CancelCast => {
                self.handle_cancel_cast(pkt).await;
            }
            ClientOpcodes::CancelAura => {
                self.handle_cancel_aura(pkt).await;
            }
            ClientOpcodes::CancelAutoRepeatSpell => {
                self.handle_cancel_auto_repeat_spell(pkt).await;
            }
            ClientOpcodes::CancelChannelling => {
                self.handle_cancel_channelling(pkt).await;
            }
            ClientOpcodes::CancelGrowthAura => {
                self.handle_cancel_growth_aura(pkt).await;
            }
            ClientOpcodes::CancelMountAura => {
                self.handle_cancel_mount_aura(pkt).await;
            }
            ClientOpcodes::CancelQueuedSpell => {
                self.handle_cancel_queued_spell(pkt).await;
            }
            ClientOpcodes::SelfRes => {
                self.handle_self_res(pkt).await;
            }
            ClientOpcodes::PetCancelAura => {
                self.handle_pet_cancel_aura(pkt).await;
            }
            ClientOpcodes::TotemDestroyed => {
                self.handle_totem_destroyed(pkt).await;
            }
            ClientOpcodes::OpenItem => {
                self.handle_open_item(pkt).await;
            }
            ClientOpcodes::UnlockVoidStorage => {
                self.handle_void_storage_unlock(pkt).await;
            }
            ClientOpcodes::QueryVoidStorage => {
                self.handle_void_storage_query(pkt).await;
            }
            ClientOpcodes::VoidStorageTransfer => {
                self.handle_void_storage_transfer(pkt).await;
            }
            ClientOpcodes::SwapVoidItem => {
                self.handle_void_storage_swap_item(pkt).await;
            }
            ClientOpcodes::SpellClick => {
                self.handle_spell_click(pkt).await;
            }

            // ── QueryTime / QueryNextMailTime ─────────────────────────────────
            ClientOpcodes::QueryTime => {
                self.handle_query_time().await;
            }
            ClientOpcodes::QueryNextMailTime => {
                self.handle_query_next_mail_time().await;
            }

            // ── Silent-ignore stubs (login-time client packets, no response) ──
            ClientOpcodes::AddonList => {
                self.handle_addon_list(pkt).await;
            }
            ClientOpcodes::AddBattlenetFriend => {
                self.handle_add_battlenet_friend(pkt).await;
            }
            ClientOpcodes::BattlenetChallengeResponse => {
                self.handle_unhandled_client_null_like_cpp(pkt).await;
            }
            ClientOpcodes::SetInsertItemsLeftToRight => {
                self.handle_set_insert_items_left_to_right(pkt).await;
            }
            ClientOpcodes::RequestAccountData => {
                self.handle_request_account_data(pkt).await;
            }
            ClientOpcodes::UpdateAccountData => {
                self.handle_update_account_data(pkt).await;
            }
            ClientOpcodes::ChangeBagSlotFlag
            | ClientOpcodes::CloseQuestChoice
            | ClientOpcodes::QueryQuestItemUsability
            | ClientOpcodes::SaveAccountDataExport
            | ClientOpcodes::SetPreferredCemetery
            | ClientOpcodes::UpdateClientSettings => {
                self.handle_unhandled_client_null_like_cpp(pkt).await;
            }
            ClientOpcodes::DiscardedTimeSyncAcks
            | ClientOpcodes::EngineSurvey
            | ClientOpcodes::LatencyReport
            | ClientOpcodes::ReportServerLag
            | ClientOpcodes::SuspendCommsAck => {
                self.handle_client_telemetry_null_like_cpp(pkt).await;
            }
            ClientOpcodes::LoadingScreenNotify => {
                self.handle_loading_screen_notify(pkt).await;
            }
            ClientOpcodes::ViolenceLevel => {
                self.handle_violence_level(pkt).await;
            }
            ClientOpcodes::OverrideScreenFlash => {
                self.handle_override_screen_flash(pkt).await;
            }
            ClientOpcodes::QueuedMessagesEnd => {
                self.handle_queued_messages_end(pkt).await;
            }
            ClientOpcodes::ChatUnregisterAllAddonPrefixes => {
                self.handle_chat_unregister_all_addon_prefixes(pkt).await;
            }
            ClientOpcodes::SetActionBarToggles => {
                self.handle_set_action_bar_toggles(pkt).await;
            }
            ClientOpcodes::SetActionButton => {
                self.handle_set_action_button(pkt).await;
            }
            ClientOpcodes::SetTaxiBenchmarkMode => {
                self.handle_set_taxi_benchmark_mode(pkt).await;
            }
            ClientOpcodes::SetAdvancedCombatLogging => {
                self.handle_set_advanced_combat_logging(pkt).await;
            }
            ClientOpcodes::SetCurrencyFlags => {
                self.handle_set_currency_flags(pkt).await;
            }
            ClientOpcodes::SetDifficultyId => {
                self.handle_set_difficulty_id(pkt).await;
            }
            ClientOpcodes::ToggleDifficulty => {
                self.handle_toggle_difficulty(pkt).await;
            }
            ClientOpcodes::SetDungeonDifficulty => {
                self.handle_set_dungeon_difficulty(pkt).await;
            }
            ClientOpcodes::SetRaidDifficulty => {
                self.handle_set_raid_difficulty(pkt).await;
            }
            ClientOpcodes::SetAmmo => {
                self.handle_set_ammo(pkt).await;
            }
            ClientOpcodes::SetGameEventDebugViewState => {
                self.handle_set_game_event_debug_view_state(pkt).await;
            }
            ClientOpcodes::ShowingHelm => {
                self.handle_showing_helm(pkt).await;
            }
            ClientOpcodes::ShowingCloak => {
                self.handle_showing_cloak(pkt).await;
            }
            ClientOpcodes::SetTitle => {
                self.handle_set_title(pkt).await;
            }
            ClientOpcodes::SaveCufProfiles => {
                self.handle_save_cuf_profiles(pkt).await;
            }
            ClientOpcodes::Tutorial => {
                self.handle_tutorial(pkt).await;
            }
            ClientOpcodes::GuildSetAchievementTracking => {
                self.handle_guild_set_achievement_tracking(pkt).await;
            }
            ClientOpcodes::DeclineGuildInvites => {
                self.handle_decline_guild_invites(pkt).await;
            }
            ClientOpcodes::GuildDeclineInvitation => {
                self.handle_guild_decline_invitation(pkt).await;
            }
            ClientOpcodes::AcceptGuildInvite => {
                self.handle_accept_guild_invite(pkt).await;
            }
            ClientOpcodes::GetItemPurchaseData => {
                self.handle_get_item_purchase_data(pkt).await;
            }
            ClientOpcodes::RequestForcedReactions => {
                self.handle_request_forced_reactions(pkt).await;
            }
            ClientOpcodes::SetFactionAtWar => {
                self.handle_set_faction_at_war(pkt).await;
            }
            ClientOpcodes::SetFactionNotAtWar => {
                self.handle_set_faction_not_at_war(pkt).await;
            }
            ClientOpcodes::SetFactionInactive => {
                self.handle_set_faction_inactive(pkt).await;
            }
            ClientOpcodes::SetWatchedFaction => {
                self.handle_set_watched_faction(pkt).await;
            }
            ClientOpcodes::CollectionItemSetFavorite => {
                self.handle_collection_item_set_favorite(pkt).await;
            }
            ClientOpcodes::MountSetFavorite => {
                self.handle_mount_set_favorite(pkt).await;
            }
            ClientOpcodes::MountSpecialAnim => {
                self.handle_mount_special_anim(pkt).await;
            }
            ClientOpcodes::MountClearFanfare => {
                self.handle_mount_clear_fanfare(pkt).await;
            }
            ClientOpcodes::AddToy => {
                self.handle_add_toy(pkt).await;
            }
            ClientOpcodes::ToyClearFanfare => {
                self.handle_toy_clear_fanfare(pkt).await;
            }
            ClientOpcodes::UseToy => {
                self.handle_use_toy(pkt).await;
            }
            ClientOpcodes::RequestBattlefieldStatus => {
                self.handle_request_battlefield_status(pkt).await;
            }
            ClientOpcodes::BattlemasterHello => {
                self.handle_battlemaster_hello(pkt).await;
            }
            ClientOpcodes::BattlefieldList => {
                self.handle_battlefield_list(pkt).await;
            }
            ClientOpcodes::BattlefieldPort => {
                self.handle_battlefield_port(pkt).await;
            }
            ClientOpcodes::BattlefieldLeave => {
                self.handle_battlefield_leave(pkt).await;
            }
            ClientOpcodes::BattlemasterJoin => {
                self.handle_battlemaster_join(pkt).await;
            }
            ClientOpcodes::BattlemasterJoinArena => {
                self.handle_battlemaster_join_arena(pkt).await;
            }
            ClientOpcodes::BattlemasterJoinSkirmish => {
                self.handle_battlemaster_join_skirmish(pkt).await;
            }
            ClientOpcodes::AcceptWargameInvite => {
                self.handle_accept_wargame_invite(pkt).await;
            }
            ClientOpcodes::RequestRatedPvpInfo => {
                self.handle_request_rated_pvp_info(pkt).await;
            }
            ClientOpcodes::RequestPvpRewards => {
                self.handle_request_pvp_rewards(pkt).await;
            }
            ClientOpcodes::TogglePvp => {
                self.handle_toggle_pvp(pkt).await;
            }
            ClientOpcodes::SetPvp => {
                self.handle_set_pvp(pkt).await;
            }
            ClientOpcodes::DfGetSystemInfo => {
                self.handle_df_get_system_info(pkt).await;
            }
            ClientOpcodes::DfGetJoinStatus => {
                self.handle_df_get_join_status(pkt).await;
            }
            ClientOpcodes::CalendarGetNumPending => {
                self.handle_calendar_get_num_pending(pkt).await;
            }
            ClientOpcodes::CalendarComplain => {
                match wow_packet::packets::misc::CalendarComplain::read(&mut pkt) {
                    Ok(complain) => self.handle_calendar_complain(complain).await,
                    Err(e) => warn!("Failed to read CalendarComplain: {e}"),
                }
            }
            ClientOpcodes::GmTicketGetCaseStatus => {
                self.handle_gm_ticket_get_case_status(pkt).await;
            }
            ClientOpcodes::GmTicketGetSystemStatus => {
                self.handle_gm_ticket_get_system_status(pkt).await;
            }
            ClientOpcodes::GmTicketAcknowledgeSurvey => {
                self.handle_gm_ticket_acknowledge_survey(pkt).await;
            }
            ClientOpcodes::Complaint => {
                self.handle_complaint(pkt).await;
            }
            ClientOpcodes::SubmitUserFeedback => {
                self.handle_submit_user_feedback(pkt).await;
            }
            ClientOpcodes::SupportTicketSubmitBug => {
                self.handle_support_ticket_submit_bug(pkt).await;
            }
            ClientOpcodes::SupportTicketSubmitComplaint => {
                self.handle_support_ticket_submit_complaint(pkt).await;
            }
            ClientOpcodes::SupportTicketSubmitSuggestion => {
                self.handle_support_ticket_submit_suggestion(pkt).await;
            }
            ClientOpcodes::BugReport => {
                self.handle_bug_report(pkt).await;
            }
            ClientOpcodes::ObjectUpdateFailed => {
                self.handle_object_update_failed(pkt).await;
            }
            ClientOpcodes::ObjectUpdateRescued => {
                self.handle_object_update_rescued(pkt).await;
            }
            ClientOpcodes::GuildBankRemainingWithdrawMoneyQuery => {
                self.handle_guild_bank_remaining_withdraw_money_query(pkt)
                    .await;
            }
            ClientOpcodes::GuildBankActivate => {
                self.handle_guild_bank_activate(pkt).await;
            }
            ClientOpcodes::GuildBankQueryTab => {
                self.handle_guild_bank_query_tab(pkt).await;
            }
            ClientOpcodes::GuildBankBuyTab => {
                self.handle_guild_bank_buy_tab(pkt).await;
            }
            ClientOpcodes::GuildBankUpdateTab => {
                self.handle_guild_bank_update_tab(pkt).await;
            }
            ClientOpcodes::GuildBankDepositMoney => {
                self.handle_guild_bank_deposit_money(pkt).await;
            }
            ClientOpcodes::GuildBankWithdrawMoney => {
                self.handle_guild_bank_withdraw_money(pkt).await;
            }
            ClientOpcodes::GuildBankLogQuery => {
                self.handle_guild_bank_log_query(pkt).await;
            }
            ClientOpcodes::GuildBankTextQuery => {
                self.handle_guild_bank_text_query(pkt).await;
            }
            ClientOpcodes::GuildBankSetTabText => {
                self.handle_guild_bank_set_tab_text(pkt).await;
            }
            ClientOpcodes::AutoGuildBankItem => {
                self.handle_auto_guild_bank_item(pkt).await;
            }
            ClientOpcodes::AutoStoreGuildBankItem => {
                self.handle_auto_store_guild_bank_item(pkt).await;
            }
            ClientOpcodes::BattlePetRequestJournal => {
                self.handle_battle_pet_request_journal(pkt).await;
            }
            ClientOpcodes::BattlePetRequestJournalLock => {
                self.handle_battle_pet_request_journal_lock(pkt).await;
            }
            ClientOpcodes::BattlePetClearFanfare => {
                self.handle_battle_pet_clear_fanfare(pkt).await;
            }
            ClientOpcodes::BattlePetSetFlags => {
                self.handle_battle_pet_set_flags(pkt).await;
            }
            ClientOpcodes::BattlePetSetBattleSlot => {
                self.handle_battle_pet_set_battle_slot(pkt).await;
            }
            ClientOpcodes::BattlePetSummon => {
                self.handle_battle_pet_summon(pkt).await;
            }
            ClientOpcodes::BattlePetUpdateNotify => {
                self.handle_battle_pet_update_notify(pkt).await;
            }
            ClientOpcodes::BattlePetUpdateDisplayNotify => {
                self.handle_battle_pet_update_display_notify(pkt).await;
            }
            ClientOpcodes::DismissCritter => {
                self.handle_dismiss_critter(pkt).await;
            }
            ClientOpcodes::QueryBattlePetName => {
                self.handle_query_battle_pet_name(pkt).await;
            }
            ClientOpcodes::ArenaTeamRoster => {
                self.handle_arena_team_roster(pkt).await;
            }
            ClientOpcodes::ArenaTeamAccept => {
                self.handle_arena_team_accept(pkt).await;
            }
            ClientOpcodes::ArenaTeamDecline => {
                self.handle_arena_team_decline(pkt).await;
            }
            ClientOpcodes::ArenaTeamLeave => {
                self.handle_arena_team_leave(pkt).await;
            }
            ClientOpcodes::ArenaTeamRemove => {
                self.handle_arena_team_remove(pkt).await;
            }
            ClientOpcodes::ArenaTeamDisband => {
                self.handle_arena_team_disband(pkt).await;
            }
            ClientOpcodes::ArenaTeamLeader => {
                self.handle_arena_team_leader(pkt).await;
            }
            ClientOpcodes::QueryArenaTeam => {
                self.handle_query_arena_team(pkt).await;
            }
            ClientOpcodes::RequestRaidInfo => {
                self.handle_request_raid_info(pkt).await;
            }
            ClientOpcodes::ResetInstances => {
                self.handle_reset_instances(pkt).await;
            }
            ClientOpcodes::InstanceLockResponse => {
                self.handle_instance_lock_response(pkt).await;
            }
            ClientOpcodes::RequestConquestFormulaConstants => {
                self.handle_request_conquest_formula_constants(pkt).await;
            }
            ClientOpcodes::RequestLfgListBlacklist => {
                self.handle_request_lfg_list_blacklist(pkt).await;
            }
            ClientOpcodes::LfgListGetStatus => {
                self.handle_lfg_list_get_status(pkt).await;
            }
            ClientOpcodes::LogStreamingError => {
                self.handle_log_streaming_error(pkt).await;
            }
            ClientOpcodes::CompleteCinematic => {
                self.handle_complete_cinematic(pkt).await;
            }
            ClientOpcodes::NextCinematicCamera => {
                self.handle_next_cinematic_camera(pkt).await;
            }
            ClientOpcodes::OpeningCinematic => {
                self.handle_opening_cinematic(pkt).await;
            }
            ClientOpcodes::CompleteMovie => {
                self.handle_complete_movie(pkt).await;
            }
            ClientOpcodes::LogoutInstant => {
                self.handle_logout_instant(pkt).await;
            }
            ClientOpcodes::SpawnTrackingUpdate => {
                self.handle_spawn_tracking_update(pkt).await;
            }
            ClientOpcodes::TimeAdjustmentResponse => {
                self.handle_time_adjustment_response(pkt).await;
            }
            ClientOpcodes::UpdateAreaTriggerVisual => {
                self.handle_update_area_trigger_visual(pkt).await;
            }
            ClientOpcodes::UpdateSpellVisual => {
                self.handle_update_spell_visual(pkt).await;
            }
            ClientOpcodes::UsedFollow => {
                self.handle_used_follow(pkt).await;
            }
            ClientOpcodes::GetAccountCharacterList => {
                self.handle_get_account_character_list(pkt).await;
            }
            ClientOpcodes::GetAccountNotifications => {
                self.handle_get_account_notifications(pkt).await;
            }
            ClientOpcodes::CancelTrade => {
                self.handle_cancel_trade(pkt).await;
            }
            ClientOpcodes::AcceptTrade => {
                self.handle_accept_trade(pkt).await;
            }
            ClientOpcodes::ClearTradeItem => {
                self.handle_clear_trade_item(pkt).await;
            }
            ClientOpcodes::SetTradeItem => {
                self.handle_set_trade_item(pkt).await;
            }
            ClientOpcodes::SetTradeGold => {
                self.handle_set_trade_gold(pkt).await;
            }
            ClientOpcodes::SetTradeSpell => {
                self.handle_set_trade_spell(pkt).await;
            }
            ClientOpcodes::SignPetition => {
                self.handle_sign_petition(pkt).await;
            }
            ClientOpcodes::DeclinePetition => {
                self.handle_decline_petition(pkt).await;
            }
            ClientOpcodes::QueryPetition => {
                self.handle_query_petition(pkt).await;
            }
            ClientOpcodes::UnacceptTrade => {
                self.handle_unaccept_trade(pkt).await;
            }
            ClientOpcodes::BusyTrade => {
                self.handle_busy_trade(pkt).await;
            }
            ClientOpcodes::BeginTrade => {
                self.handle_begin_trade(pkt).await;
            }
            ClientOpcodes::CanDuel => {
                self.handle_can_duel(pkt).await;
            }
            ClientOpcodes::DuelResponse => {
                self.handle_duel_response(pkt).await;
            }
            ClientOpcodes::IgnoreTrade => {
                self.handle_ignore_trade(pkt).await;
            }
            ClientOpcodes::ReportClientVariables => {
                self.handle_report_client_variables(pkt).await;
            }
            ClientOpcodes::ReportEnabledAddons => {
                self.handle_report_enabled_addons(pkt).await;
            }
            ClientOpcodes::ReportFrozenWhileLoadingMap => {
                self.handle_report_frozen_while_loading_map(pkt).await;
            }
            ClientOpcodes::ReportKeybindingExecutionCounts => {
                self.handle_report_keybinding_execution_counts(pkt).await;
            }
            ClientOpcodes::QueryCountdownTimer => {
                self.handle_request_countdown_timer(pkt).await;
            }
            ClientOpcodes::CalendarGet => {
                self.handle_calendar_get(pkt).await;
            }
            ClientOpcodes::CalendarCommunityInvite => {
                match wow_packet::packets::misc::CalendarCommunityInvite::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_community_invite(query).await,
                    Err(e) => warn!("Failed to read CalendarCommunityInvite: {e}"),
                }
            }
            ClientOpcodes::CalendarAddEvent => {
                match wow_packet::packets::misc::CalendarAddEvent::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_add_event(query).await,
                    Err(e) => warn!("Failed to read CalendarAddEvent: {e}"),
                }
            }
            ClientOpcodes::CalendarGetEvent => {
                match wow_packet::packets::misc::CalendarGetEvent::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_get_event(query).await,
                    Err(e) => warn!("Failed to read CalendarGetEvent: {e}"),
                }
            }
            ClientOpcodes::CalendarCopyEvent => {
                match wow_packet::packets::misc::CalendarCopyEvent::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_copy_event(query).await,
                    Err(e) => warn!("Failed to read CalendarCopyEvent: {e}"),
                }
            }
            ClientOpcodes::CalendarEventSignUp => {
                match wow_packet::packets::misc::CalendarEventSignUp::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_event_sign_up(query).await,
                    Err(e) => warn!("Failed to read CalendarEventSignUp: {e}"),
                }
            }
            ClientOpcodes::CalendarRemoveEvent => {
                match wow_packet::packets::misc::CalendarRemoveEvent::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_remove_event(query).await,
                    Err(e) => warn!("Failed to read CalendarRemoveEvent: {e}"),
                }
            }
            ClientOpcodes::CalendarRemoveInvite => {
                match wow_packet::packets::misc::CalendarRemoveInvite::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_remove_invite(query).await,
                    Err(e) => warn!("Failed to read CalendarRemoveInvite: {e}"),
                }
            }
            ClientOpcodes::CalendarInvite => {
                match wow_packet::packets::misc::CalendarInvite::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_invite(query).await,
                    Err(e) => warn!("Failed to read CalendarInvite: {e}"),
                }
            }
            ClientOpcodes::CalendarUpdateEvent => {
                match wow_packet::packets::misc::CalendarUpdateEvent::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_update_event(query).await,
                    Err(e) => warn!("Failed to read CalendarUpdateEvent: {e}"),
                }
            }
            ClientOpcodes::CalendarRsvp => {
                match wow_packet::packets::misc::CalendarRsvp::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_rsvp(query).await,
                    Err(e) => warn!("Failed to read CalendarRsvp: {e}"),
                }
            }
            ClientOpcodes::CalendarModeratorStatus => {
                match wow_packet::packets::misc::CalendarModeratorStatusQuery::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_moderator_status(query).await,
                    Err(e) => warn!("Failed to read CalendarModeratorStatusQuery: {e}"),
                }
            }
            ClientOpcodes::CalendarStatus => {
                match wow_packet::packets::misc::CalendarStatus::read(&mut pkt) {
                    Ok(query) => self.handle_calendar_status(query).await,
                    Err(e) => warn!("Failed to read CalendarStatus: {e}"),
                }
            }
            ClientOpcodes::CloseInteraction => {
                self.handle_close_interaction(pkt).await;
            }
            ClientOpcodes::AuctionListBidderItems => {
                self.handle_auction_list_bidder_items(pkt).await;
            }
            ClientOpcodes::AuctionListItems => {
                match wow_packet::packets::misc::AuctionListItems::read(&mut pkt) {
                    Ok(packet) => self.handle_auction_list_items(packet).await,
                    Err(e) => warn!("Failed to read AuctionListItems: {e}"),
                }
            }
            ClientOpcodes::AuctionPlaceBid => {
                match wow_packet::packets::misc::AuctionPlaceBid::read(&mut pkt) {
                    Ok(packet) => self.handle_auction_place_bid(packet).await,
                    Err(e) => warn!("Failed to read AuctionPlaceBid: {e}"),
                }
            }
            ClientOpcodes::AuctionRemoveItem => {
                match wow_packet::packets::misc::AuctionRemoveItem::read(&mut pkt) {
                    Ok(packet) => self.handle_auction_remove_item(packet).await,
                    Err(e) => warn!("Failed to read AuctionRemoveItem: {e}"),
                }
            }
            ClientOpcodes::AuctionSellItem => {
                match wow_packet::packets::misc::AuctionSellItem::read(&mut pkt) {
                    Ok(packet) => self.handle_auction_sell_item(packet).await,
                    Err(e) => warn!("Failed to read AuctionSellItem: {e}"),
                }
            }
            ClientOpcodes::AuctionReplicateItems => {
                match wow_packet::packets::misc::AuctionReplicateItems::read(&mut pkt) {
                    Ok(packet) => self.handle_auction_replicate_items(packet).await,
                    Err(e) => warn!("Failed to read AuctionReplicateItems: {e}"),
                }
            }
            ClientOpcodes::AuctionListOwnerItems => {
                self.handle_auction_list_owner_items(pkt).await;
            }
            ClientOpcodes::AuctionListPendingSales => {
                self.handle_auction_list_pending_sales(pkt).await;
            }
            ClientOpcodes::AuctionableTokenSell => {
                self.handle_auctionable_token_sell(pkt).await;
            }
            ClientOpcodes::AuctionableTokenSellAtMarketPrice => {
                self.handle_auctionable_token_sell_at_market_price(pkt)
                    .await;
            }
            ClientOpcodes::CommerceTokenGetLog => {
                self.handle_commerce_token_get_log(pkt).await;
            }
            ClientOpcodes::GameObjUse => {
                self.handle_game_obj_use(pkt).await;
            }
            ClientOpcodes::GameObjReportUse => {
                self.handle_game_obj_report_use(pkt).await;
            }
            ClientOpcodes::AddFriend => {
                self.handle_add_friend(pkt).await;
            }
            ClientOpcodes::AddIgnore => {
                match wow_packet::packets::social::AddIgnore::read(&mut pkt) {
                    Ok(ignore) => self.handle_add_ignore(ignore).await,
                    Err(e) => warn!("Failed to read AddIgnore: {e}"),
                }
            }
            ClientOpcodes::DelFriend => {
                self.handle_del_friend(pkt).await;
            }
            ClientOpcodes::DelIgnore => {
                match wow_packet::packets::social::DelIgnore::read(&mut pkt) {
                    Ok(ignore) => self.handle_del_ignore(ignore).await,
                    Err(e) => warn!("Failed to read DelIgnore: {e}"),
                }
            }
            ClientOpcodes::SendContactList => {
                self.handle_send_contact_list(pkt).await;
            }
            ClientOpcodes::SetContactNotes => {
                match wow_packet::packets::social::SetContactNotes::read(&mut pkt) {
                    Ok(contact) => self.handle_set_contact_notes(contact).await,
                    Err(e) => warn!("Failed to read SetContactNotes: {e}"),
                }
            }
            ClientOpcodes::SocialContractRequest => {
                match wow_packet::packets::social::SocialContractRequest::read(&mut pkt) {
                    Ok(_) => self.handle_social_contract_request().await,
                    Err(e) => warn!("Failed to read SocialContractRequest: {e}"),
                }
            }
            ClientOpcodes::AcceptSocialContract => {
                match wow_packet::packets::social::AcceptSocialContract::read(&mut pkt) {
                    Ok(accept) => self.handle_accept_social_contract(accept).await,
                    Err(e) => warn!("Failed to read AcceptSocialContract: {e}"),
                }
            }
            ClientOpcodes::AccountNotificationAcknowledged => {
                match wow_packet::packets::social::AccountNotificationAcknowledged::read(&mut pkt) {
                    Ok(packet) => self.handle_account_notification_acknowledged(packet).await,
                    Err(e) => warn!("Failed to read AccountNotificationAcknowledged: {e}"),
                }
            }

            // ── Group / Party opcodes ─────────────────────────────────────────
            ClientOpcodes::PartyInvite => {
                self.handle_party_invite(pkt).await;
            }
            ClientOpcodes::PartyInviteResponse => {
                self.handle_party_invite_response(pkt).await;
            }
            ClientOpcodes::PartyUninvite => {
                self.handle_party_uninvite(pkt).await;
            }
            ClientOpcodes::LeaveGroup => {
                self.handle_leave_group(pkt).await;
            }
            ClientOpcodes::ConvertRaid => {
                self.handle_convert_raid(pkt).await;
            }
            ClientOpcodes::ChangeSubGroup => {
                self.handle_change_sub_group(pkt).await;
            }
            ClientOpcodes::SwapSubGroups => {
                self.handle_swap_sub_groups(pkt).await;
            }
            ClientOpcodes::SetLootMethod => {
                self.handle_set_loot_method(pkt).await;
            }
            ClientOpcodes::SetPartyLeader => {
                self.handle_set_party_leader(pkt).await;
            }
            ClientOpcodes::SetAssistantLeader => {
                self.handle_set_assistant_leader(pkt).await;
            }
            ClientOpcodes::SetEveryoneIsAssistant => {
                self.handle_set_everyone_is_assistant(pkt).await;
            }
            ClientOpcodes::SilencePartyTalker => {
                self.handle_silence_party_talker(pkt).await;
            }
            ClientOpcodes::SetPartyAssignment => {
                self.handle_set_party_assignment(pkt).await;
            }
            ClientOpcodes::SetRole => {
                self.handle_set_role(pkt).await;
            }
            ClientOpcodes::InitiateRolePoll => {
                self.handle_initiate_role_poll(pkt).await;
            }
            ClientOpcodes::UpdateRaidTarget => {
                self.handle_update_raid_target(pkt).await;
            }
            ClientOpcodes::RequestPartyJoinUpdates => {
                self.handle_request_party_join_updates(pkt).await;
            }
            ClientOpcodes::RequestPartyMemberStats => {
                self.handle_request_party_member_stats(pkt).await;
            }
            ClientOpcodes::DoReadyCheck => {
                self.handle_do_ready_check(pkt).await;
            }
            ClientOpcodes::ReadyCheckResponse => {
                self.handle_ready_check_response(pkt).await;
            }
            ClientOpcodes::OptOutOfLoot => {
                self.handle_opt_out_of_loot(pkt).await;
            }
            ClientOpcodes::LowLevelRaid1 => {
                self.handle_low_level_raid1(pkt).await;
            }
            ClientOpcodes::LowLevelRaid2 => {
                self.handle_low_level_raid2(pkt).await;
            }
            ClientOpcodes::MinimapPing => {
                self.handle_minimap_ping(pkt).await;
            }
            ClientOpcodes::RandomRoll => {
                self.handle_random_roll(pkt).await;
            }

            ClientOpcodes::Inspect => {
                self.handle_inspect(pkt).await;
            }
            ClientOpcodes::StandStateChange => {
                self.handle_stand_state_change(pkt).await;
            }
            ClientOpcodes::RequestHonorStats => {
                self.handle_request_honor_stats(pkt).await;
            }
            ClientOpcodes::QueryInspectAchievements => {
                self.handle_query_inspect_achievements(pkt).await;
            }

            // Empty stubs matching TrinityCore's no-response service-opcode handling;
            // these client opcodes are sent during
            // character select but require no response (Blizzard services).
            ClientOpcodes::BattlePayGetProductList
            | ClientOpcodes::BattlePayGetPurchaseList
            | ClientOpcodes::UpdateVasPurchaseStates => {
                trace!(
                    "Stub handler for {:?} (0x{:04X}) — no response needed",
                    opcode, opcode_raw
                );
            }
            _ => match entry.processing {
                PacketProcessing::Inplace => {
                    trace!("Processing {:?} inplace via {}", opcode, entry.handler_name);
                }
                PacketProcessing::ThreadUnsafe => {
                    trace!(
                        "Queuing {:?} for thread-unsafe processing via {}",
                        opcode, entry.handler_name
                    );
                }
                PacketProcessing::ThreadSafe => {
                    trace!(
                        "Processing {:?} via thread-safe handler {}",
                        opcode, entry.handler_name
                    );
                }
            },
        }
    }

    /// Check if the handler's required status matches the current session state.
    ///
    /// Matches C++ `WorldSession::Update` status gates:
    /// - `Authed` → allowed in ANY state (authenticated, in-world, or transferring)
    /// - `LoggedIn` → only when player is in-world
    /// - `Transfer` → only during map transfers
    /// - `LoggedInOrRecentlyLogout` → in-world or recently disconnected
    fn is_status_allowed(&self, required: SessionStatus) -> bool {
        match required {
            SessionStatus::Authed => true, // C++ STATUS_AUTHED
            SessionStatus::LoggedIn => self.state == SessionState::LoggedIn,
            SessionStatus::Transfer => self.state == SessionState::Transfer,
            SessionStatus::LoggedInOrRecentlyLogout => {
                self.state == SessionState::LoggedIn || self.state == SessionState::Disconnecting
            }
        }
    }
}

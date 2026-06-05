// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for miscellaneous client opcodes:
//! SetSelection, AreaTrigger, RequestCemeteryList,
//! TaxiNodeStatusQuery, ChatJoinChannel.

use tracing::{debug, info, warn};
use wow_constants::{ClientOpcodes, InventoryResult, ItemExtendedCostFlags, SpellCastResult};
use wow_database::{SqlTransaction, WorldStatements};
use wow_entities::{
    GAMEOBJECT_TYPE_BARBER_CHAIR, GAMEOBJECT_TYPE_BUTTON, GAMEOBJECT_TYPE_CAMERA,
    GAMEOBJECT_TYPE_CAPTURE_POINT, GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_DOOR,
    GAMEOBJECT_TYPE_FISHING_HOLE, GAMEOBJECT_TYPE_FISHING_NODE, GAMEOBJECT_TYPE_FLAGDROP,
    GAMEOBJECT_TYPE_FLAGSTAND, GAMEOBJECT_TYPE_GATHERING_NODE, GAMEOBJECT_TYPE_GOOBER,
    GAMEOBJECT_TYPE_ITEM_FORGE, GAMEOBJECT_TYPE_MEETINGSTONE, GAMEOBJECT_TYPE_NEW_FLAG,
    GAMEOBJECT_TYPE_NEW_FLAG_DROP, GAMEOBJECT_TYPE_QUESTGIVER, GAMEOBJECT_TYPE_RITUAL,
    GAMEOBJECT_TYPE_SPELL_FOCUS, GAMEOBJECT_TYPE_SPELLCASTER, GAMEOBJECT_TYPE_TRAP,
    GAMEOBJECT_TYPE_UI_LINK, GameObjectTemplateData, MAX_GAMEOBJECT_DATA,
};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::ClientPacket;
use wow_packet::packets::collection::{
    COLLECTION_TYPE_APPEARANCE_LIKE_CPP, COLLECTION_TYPE_TOYBOX_LIKE_CPP,
    CollectionItemSetFavorite, TransmogrifyItems,
};
use wow_packet::packets::instance::{
    InstanceInfo, InstanceLockInfo, InstanceLockResponse, InstanceReset, InstanceResetFailed,
    PendingRaidLock,
};
use wow_packet::packets::item::{
    GetItemPurchaseData, InventoryChangeFailure, ItemPurchaseContents, ItemPurchaseRefundCurrency,
    ItemPurchaseRefundItem, SetItemPurchaseData,
};
use wow_packet::packets::loot::{LOOT_TYPE_FISHING_JUNK_LIKE_CPP, LOOT_TYPE_FISHING_LIKE_CPP};
use wow_packet::packets::misc::{
    AddToy, BattlePetClearFanfare, BattlePetRequestJournal, BattlePetSetBattleSlot,
    BattlePetSetFlags, BattlePetSummon, BattlePetUpdateNotify, FarSight, MountSetFavorite,
    QueryBattlePetName, QueryBattlePetNameResponse, RatedPvpInfo, RequestCemeteryListResponse,
    TaxiNodeStatusPkt, ToyClearFanfare, UseToy,
};
use wow_packet::packets::reputation::{
    RequestForcedReactions, SetFactionAtWarRequest, SetFactionInactive, SetFactionNotAtWarRequest,
    SetWatchedFaction,
};
use wow_packet::packets::spell::{CastFailed, SpellCastVisual, SpellPreparePkt, SpellStartPkt};

use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::handlers::loot::represented_gameobject_interaction_distance_like_cpp;
use crate::session::{
    CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP, RepresentedGameObjectAccessLikeCpp,
    RepresentedGameObjectUseEffect, SpellCastMetadata,
};

// ── inventory registrations ───────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::FarSight,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_far_sight",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetSelection,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_selection",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AreaTrigger,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
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
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_taxi_node_status_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatJoinChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_join_channel",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MountSetFavorite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_mount_set_favorite",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CollectionItemSetFavorite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_collection_item_set_favorite",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MountClearFanfare,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_mount_clear_fanfare",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AddToy,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_add_toy",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ToyClearFanfare,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_toy_clear_fanfare",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UseToy,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_use_toy",
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
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_next_mail_time",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LoadingScreenNotify,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
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
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_unregister_all_addon_prefixes",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetActionBarToggles,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
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
        processing: PacketProcessing::ThreadUnsafe,
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
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_forced_reactions",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetFactionAtWar,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_faction_at_war",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetFactionNotAtWar,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_faction_not_at_war",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetFactionInactive,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_faction_inactive",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetWatchedFaction,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_watched_faction",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestBattlefieldStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_battlefield_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestRatedPvpInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
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
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_df_get_system_info",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DfGetJoinStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_df_get_join_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarGetNumPending,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
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
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_remaining_withdraw_money_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetRequestJournal,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_request_journal",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetRequestJournalLock,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_request_journal_lock",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetClearFanfare,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_clear_fanfare",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetSetFlags,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_set_flags",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetSetBattleSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_set_battle_slot",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetSummon,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_battle_pet_summon",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetUpdateNotify,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_update_notify",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetUpdateDisplayNotify,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_update_display_notify",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryBattlePetName,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_battle_pet_name",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamRoster,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_roster",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestRaidInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_raid_info",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ResetInstances,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_reset_instances",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::InstanceLockResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_instance_lock_response",
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
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_lfg_list_blacklist",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LfgListGetStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_lfg_list_get_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GetAccountCharacterList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_account_character_list",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelTrade,
        status: SessionStatus::LoggedInOrRecentlyLogout,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_cancel_trade",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReportClientVariables,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_client_variables",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReportEnabledAddons,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_enabled_addons",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReportKeybindingExecutionCounts,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_keybinding_execution_counts",
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
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_get",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CloseInteraction,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_close_interaction",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListBidderItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_list_bidder_items",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListOwnerItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_list_owner_items",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionListPendingSales,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_list_pending_sales",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CommerceTokenGetLog,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
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

pub(crate) fn item_purchase_contents_from_extended_cost(
    extended_cost: &wow_data::item_extended_cost::ItemExtendedCostEntry,
    money: u64,
) -> ItemPurchaseContents {
    let mut contents = ItemPurchaseContents {
        money,
        ..Default::default()
    };

    for i in 0..5 {
        contents.items[i] = ItemPurchaseRefundItem {
            item_id: extended_cost.item_id[i] as i32,
            item_count: extended_cost.item_count[i] as i32,
        };

        let season_earned = match i {
            0 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1),
            1 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2),
            2 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3),
            3 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4),
            4 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5),
            _ => false,
        };
        if !season_earned {
            contents.currencies[i] = ItemPurchaseRefundCurrency {
                currency_id: extended_cost.currency_id[i] as i32,
                currency_count: extended_cost.currency_count[i] as i32,
            };
        }
    }

    contents
}

impl crate::session::WorldSession {
    /// C++ `WorldSession::HandleFarSightOpcode`: does not create/remove the
    /// viewpoint; it only switches the represented seer and forces visibility.
    pub async fn handle_far_sight(&mut self, mut pkt: wow_packet::WorldPacket) {
        let far_sight = match FarSight::read(&mut pkt) {
            Ok(far_sight) => far_sight,
            Err(err) => {
                warn!("Failed to read FarSight: {err}");
                return;
            }
        };

        self.apply_far_sight_like_cpp(far_sight.enable);
        self.force_update_visibility_like_cpp().await;
    }

    /// CMSG_SET_SELECTION — client clicked/targeted an object.
    /// Payload: packed GUID of selected object (0 clears selection).
    pub async fn handle_set_selection(&mut self, mut pkt: wow_packet::WorldPacket) {
        let target_guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        self.set_selection_guid_like_cpp(Some(target_guid));
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
            warn!(
                "WorldPortResponse from account {} but no pending teleport",
                self.account_id
            );
            self.set_state(crate::session::SessionState::LoggedIn);
            return;
        };

        info!(
            account = self.account_id,
            "WorldPortResponse: completing teleport to map {} ({:.2}, {:.2}, {:.2})",
            new_map,
            new_pos.x,
            new_pos.y,
            new_pos.z
        );

        // Update internal state
        self.set_player_map_position_like_cpp(new_map as u16, new_pos);
        self.update_registry_position();

        // SMSG_NEW_WORLD — place player in new world
        self.send_packet(&NewWorld {
            map_id: new_map,
            pos: new_pos,
            reason: 0,
        });

        // SMSG_RESUME_TOKEN — resume movement processing
        self.send_packet(&ResumeToken {
            sequence_index: 1,
            reason: 1,
        });

        // Back to LoggedIn — handler dispatch resumes
        self.set_state(crate::session::SessionState::LoggedIn);

        // Resend nearby world objects at new position
        self.send_nearby_creatures(new_map as u16, &new_pos, 0)
            .await;
        self.send_nearby_gameobjects(new_map as u16, &new_pos, 0)
            .await;
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
        let is_flight_master = self
            .mutate_world_creature(unit_guid, |creature| {
                creature.npc_flags() & NPC_FLAG_FLIGHT_MASTER != 0
            })
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

    /// CMSG_MOUNT_SET_FAVORITE — toggle the favorite bit on a known account mount.
    ///
    /// C++ ref: `WorldSession::HandleMountSetFavorite` delegates to
    /// `CollectionMgr::MountSetFavorite`, which silently ignores unknown mounts
    /// and sends a partial `SMSG_ACCOUNT_MOUNT_UPDATE` for the changed mount.
    pub async fn handle_mount_set_favorite(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match MountSetFavorite::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "MountSetFavorite parse failed: {error}"
                );
                return;
            }
        };

        self.mount_set_favorite_like_cpp(request.mount_spell_id, request.is_favorite);
    }

    /// CMSG_COLLECTION_ITEM_SET_FAVORITE — toggle favorite state for supported collections.
    ///
    /// C++ ref: `WorldSession::HandleCollectionItemSetFavorite` forwards TOYBOX
    /// ids to `CollectionMgr::ToySetFavorite`, and only forwards APPEARANCE ids
    /// when `CollectionMgr::HasItemAppearance(id)` returns a permanent
    /// appearance. Temporary appearances, unknown ids, and unsupported collection
    /// types are ignored.
    pub async fn handle_collection_item_set_favorite(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CollectionItemSetFavorite::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CollectionItemSetFavorite parse failed: {error}"
                );
                return;
            }
        };

        match request.collection_type {
            COLLECTION_TYPE_TOYBOX_LIKE_CPP => {
                self.toy_set_favorite_like_cpp(request.id, request.is_favorite);
            }
            COLLECTION_TYPE_APPEARANCE_LIKE_CPP => {
                let (has_appearance, is_temporary) = self.has_item_appearance_like_cpp(request.id);
                if !has_appearance || is_temporary {
                    return;
                }

                self.set_appearance_is_favorite_like_cpp(request.id, request.is_favorite);
            }
            _ => {}
        }
    }

    /// CMSG_TRANSMOGRIFY_ITEMS — parsed only; full C++ handler is not ported yet.
    ///
    /// C++ `WorldSession::HandleTransmogrifyItems` also validates the NPC
    /// interaction, inventory items, appearances, costs, modifiers, and reset
    /// paths before applying changes. This Rust slice only represents the
    /// client packet and keeps gameplay state unchanged.
    pub async fn handle_transmogrify_items(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match TransmogrifyItems::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "TransmogrifyItems parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            npc = ?request.npc,
            item_count = request.items.len(),
            current_spec_only = request.current_spec_only,
            "TransmogrifyItems parsed; full C++ transmogrification application is pending"
        );
    }

    /// CMSG_MOUNT_CLEAR_FANFARE — C++ currently logs only.
    pub async fn handle_mount_clear_fanfare(&mut self, _pkt: wow_packet::WorldPacket) {
        debug!(account = self.account_id, "Mount fanfare cleared");
    }

    /// CMSG_TOY_CLEAR_FANFARE — clear the account toy fanfare bit.
    ///
    /// C++ ref: `WorldSession::HandleToyClearFanfare` forwards only the item id
    /// to `CollectionMgr::ToyClearFanfare`, which silently ignores unknown toys.
    pub async fn handle_toy_clear_fanfare(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ToyClearFanfare::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ToyClearFanfare parse failed: {error}"
                );
                return;
            }
        };

        self.toy_clear_fanfare_like_cpp(request.item_id);
    }

    /// CMSG_USE_TOY — bounded C++ guard path before spell execution.
    ///
    /// C++ `HandleUseToy` validates item template, `CollectionMgr::HasToy`,
    /// item effect spell membership, `SpellMgr::GetSpellInfo`, possession, and
    /// then creates/prepares a `Spell` with toy-specific flags. Rust still uses
    /// the represented spell executor, but preserves the C++ toy metadata that
    /// must reach `SpellCastData`.
    pub async fn handle_use_toy(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match UseToy::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(account = self.account_id, "UseToy parse failed: {error}");
                return;
            }
        };

        let item_id = match u32::try_from(request.cast.misc[0]) {
            Ok(item_id) if item_id != 0 => item_id,
            _ => return,
        };

        if self.item_storage_template(item_id).is_none() {
            return;
        }

        if !self.has_account_toy_like_cpp(item_id) {
            return;
        }

        if !self.toy_item_has_spell_effect_like_cpp(item_id, request.cast.spell_id) {
            return;
        }

        let Some(spell_store) = self.spell_store() else {
            return;
        };
        let Some(spell_info) = spell_store.get(request.cast.spell_id).cloned() else {
            warn!(
                account = self.account_id,
                spell_id = request.cast.spell_id,
                item_id,
                "HandleUseToy: unknown spell id used by toy item"
            );
            return;
        };

        if self.player_is_possessing_like_cpp() {
            return;
        }

        let toy_cooldown_ms =
            self.toy_item_spell_cooldown_ms_like_cpp(item_id, request.cast.spell_id, &spell_info);
        if let Some(remaining_ms) = self.represented_spell_cooldown_remaining_ms_like_cpp(
            request.cast.spell_id,
            toy_cooldown_ms,
        ) {
            debug!(
                account = self.account_id,
                item_id,
                spell_id = request.cast.spell_id,
                remaining_ms,
                "UseToy rejected by represented item-backed cooldown"
            );
            self.send_packet(&CastFailed {
                cast_id: request.cast.cast_id,
                spell_id: request.cast.spell_id,
                reason: SpellCastResult::NotReady as i32,
                fail_arg1: 0,
                fail_arg2: 0,
            });
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let server_cast_id = self.next_represented_spell_cast_guid_like_cpp(request.cast.spell_id);
        self.send_packet(&SpellPreparePkt {
            client_cast_id: request.cast.cast_id,
            server_cast_id,
        });

        let metadata = SpellCastMetadata {
            from_client: true,
            misc: request.cast.misc,
            cast_item_entry: Some(item_id),
            cast_flags_ex: CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP,
            original_cast_id: request.cast.cast_id,
        };

        let mut spell_target = request.cast.target.clone();
        let target_guid = if !spell_target.unit.is_empty() {
            spell_target.unit
        } else {
            spell_target.flags |= 0x2; // SpellCastTargetFlags::Unit
            spell_target.unit = player_guid;
            player_guid
        };

        let spell_visual = SpellCastVisual {
            spell_visual_id: request.cast.visual.spell_visual_id,
            script_visual_id: 0,
        };

        if spell_info.has_cast_time() {
            let start_pkt = SpellStartPkt {
                caster: player_guid,
                cast_id: server_cast_id,
                original_cast_id: request.cast.cast_id,
                spell_id: request.cast.spell_id,
                visual: spell_visual.clone(),
                cast_flags_ex: CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP,
                cast_time_ms: spell_info.cast_time_ms,
                target: spell_target.clone(),
            };
            self.send_packet(&start_pkt);

            self.active_spell_cast = Some(crate::session::SpellCastState {
                spell_id: request.cast.spell_id,
                target_guid,
                target_data: spell_target,
                cast_id: server_cast_id,
                cast_start_time: std::time::Instant::now(),
                cast_time_ms: spell_info.cast_time_ms,
                spell_visual,
                metadata,
            });
        } else if let Err(error) = self
            .execute_spell_with_visual_and_target_data_with_metadata(
                request.cast.spell_id,
                target_guid,
                server_cast_id,
                spell_visual,
                spell_target,
                metadata,
            )
            .await
        {
            warn!(
                account = self.account_id,
                spell_id = request.cast.spell_id,
                item_id,
                "UseToy represented spell execution failed: {error}"
            );
        }

        debug!(
            account = self.account_id,
            item_id,
            spell_id = request.cast.spell_id,
            "UseToy executed through represented spell path"
        );
    }

    /// CMSG_ADD_TOY — learn a Toy.db2 item and consume the inventory item.
    ///
    /// C++ ref: `WorldSession::HandleAddToy` validates the item guid, checks
    /// `sDB2Manager.IsToyItem(item->GetEntry())`, calls
    /// `CollectionMgr::AddToy(item->GetEntry(), false, false)`, which inserts
    /// the account row and calls `Player::AddToy`, then destroys the item only
    /// when the account toy was newly inserted.
    pub async fn handle_add_toy(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match AddToy::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(account = self.account_id, "AddToy parse failed: {error}");
                return;
            }
        };

        if request.item_guid == wow_core::ObjectGuid::EMPTY {
            return;
        }

        let Some((bag, slot, item)) = self.get_inventory_item_by_guid_like_cpp(request.item_guid)
        else {
            self.send_packet(&InventoryChangeFailure::error(
                InventoryResult::ItemNotFound,
            ));
            return;
        };

        if !self.is_toy_item_like_cpp(item.entry_id) {
            return;
        }

        let runtime_item = self
            .inventory_item_objects_like_cpp()
            .get(&item.guid)
            .cloned();
        let can_use_result =
            self.can_use_inventory_item_represented_like_cpp(&item, runtime_item.as_ref());
        if can_use_result != InventoryResult::Ok {
            self.send_equip_error(can_use_result, Some(item.guid), None, 0, 0);
            return;
        }

        if !self.add_account_toy_like_cpp(item.entry_id, false, false) {
            return;
        }

        let destroyed_entry_id = item.entry_id;
        if self
            .destroy_inventory_full_stack_by_pos_like_cpp(bag, slot, item, runtime_item, "AddToy")
            .await
        {
            if let Some(update) = self.add_player_toy_dynamic_field_like_cpp(destroyed_entry_id) {
                if let Some(guid) = self.player_guid() {
                    if let Some(packet) = player_values_update_to_update_object(
                        guid,
                        self.player_map_id_like_cpp(),
                        &update,
                    ) {
                        self.send_packet(&packet);
                    }
                }
            }
            info!(
                "Added toy item={} from bag {} slot {} for account {}",
                destroyed_entry_id, bag, slot, self.account_id
            );
        } else {
            self.represented_account_toys_like_cpp
                .remove(&destroyed_entry_id);
        }
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
    pub async fn handle_chat_unregister_all_addon_prefixes(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        self.registered_addon_prefixes.clear();
        self.filter_addon_messages = false;
    }
    pub async fn handle_set_action_bar_toggles(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_save_cuf_profiles(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_guild_set_achievement_tracking(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_get_item_purchase_data(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match GetItemPurchaseData::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("GetItemPurchaseData parse failed: {e}");
                return;
            }
        };
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let current_total_played_time = self.total_played_time.saturating_add(
            self.login_time
                .map(|login_time| login_time.elapsed().as_secs() as u32)
                .unwrap_or(0),
        );

        let Some(packet) = (|| {
            let item = self
                .inventory_item_objects_like_cpp()
                .get(&request.item_guid)?;
            if !item.is_refundable() || item.refund_recipient() != player_guid {
                return None;
            }

            let played_time = item.played_time(i64::from(current_total_played_time));
            if played_time > 2 * 60 * 60 {
                return None;
            }

            let extended_cost = self
                .item_extended_cost_store()
                .and_then(|store| store.get(item.paid_extended_cost()))?;
            let contents =
                item_purchase_contents_from_extended_cost(extended_cost, item.paid_money());
            Some(SetItemPurchaseData {
                item_guid: request.item_guid,
                contents,
                flags: 0,
                purchase_time: current_total_played_time.saturating_sub(played_time),
            })
        })() else {
            debug!(
                "GetItemPurchaseData ignored for non-refundable or unknown item {:?}",
                request.item_guid
            );
            return;
        };

        self.send_packet(&packet);
    }
    pub async fn handle_request_forced_reactions(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = RequestForcedReactions::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "RequestForcedReactions parse failed: {error}"
            );
            return;
        }

        let packet = self
            .reputation_mgr_like_cpp()
            .set_forced_reactions_packet_like_cpp();
        self.send_packet(&packet);
    }

    pub async fn handle_set_faction_at_war(&mut self, pkt: wow_packet::WorldPacket) {
        self.handle_set_faction_at_war_like_cpp(pkt, true).await;
    }

    pub async fn handle_set_faction_not_at_war(&mut self, pkt: wow_packet::WorldPacket) {
        self.handle_set_faction_at_war_like_cpp(pkt, false).await;
    }

    async fn handle_set_faction_at_war_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        at_war: bool,
    ) {
        let faction_index = if at_war {
            match SetFactionAtWarRequest::read(&mut pkt) {
                Ok(request) => request.faction_index,
                Err(error) => {
                    warn!(
                        account = self.account_id,
                        "SetFactionAtWar parse failed: {error}"
                    );
                    return;
                }
            }
        } else {
            match SetFactionNotAtWarRequest::read(&mut pkt) {
                Ok(request) => request.faction_index,
                Err(error) => {
                    warn!(
                        account = self.account_id,
                        "SetFactionNotAtWar parse failed: {error}"
                    );
                    return;
                }
            }
        };

        let Some(faction_store) = self.faction_store().cloned() else {
            warn!(
                account = self.account_id,
                faction_index, "SetFactionAtWar ignored without Faction.db2 store"
            );
            return;
        };
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().cloned();
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();

        self.reputation_mgr_like_cpp_mut()
            .set_at_war_by_replist_like_cpp(
                u32::from(faction_index),
                at_war,
                faction_store.as_ref(),
                friendship_rep_reaction_store.as_deref(),
                race,
                class,
            );
    }

    pub async fn handle_set_faction_inactive(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match SetFactionInactive::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetFactionInactive parse failed: {error}"
                );
                return;
            }
        };

        self.reputation_mgr_like_cpp_mut()
            .set_inactive_by_replist_like_cpp(request.index, request.state);
    }

    pub async fn handle_set_watched_faction(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match SetWatchedFaction::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetWatchedFaction parse failed: {error}"
                );
                return;
            }
        };

        self.set_watched_faction_index_like_cpp(request.faction_index as i32);
    }

    pub async fn handle_request_battlefield_status(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_rated_pvp_info(&mut self, _pkt: wow_packet::WorldPacket) {
        self.send_packet(&RatedPvpInfo::default());
    }
    pub async fn handle_request_pvp_rewards(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_df_get_system_info(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_df_get_join_status(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_calendar_get_num_pending(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_gm_ticket_get_case_status(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_guild_bank_remaining_withdraw_money_query(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
    }
    /// CMSG_BATTLE_PET_REQUEST_JOURNAL — send represented journal.
    ///
    /// C++ `BattlePetMgr::SendJournal` first acquires/sends journal-lock status
    /// when needed, then sends `SMSG_BATTLE_PET_JOURNAL`.
    pub async fn handle_battle_pet_request_journal(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = BattlePetRequestJournal::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "BattlePetRequestJournal parse failed: {error}"
            );
            return;
        }

        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            self.send_battle_pet_journal_lock_status_like_cpp();
        }

        self.send_packet(&self.represented_battle_pet_journal_like_cpp());
    }

    /// CMSG_BATTLE_PET_REQUEST_JOURNAL_LOCK — acquire represented journal lock.
    ///
    /// C++ `HandleBattlePetRequestJournalLock` sends lock status and, when the
    /// lock is held, sends the journal.
    pub async fn handle_battle_pet_request_journal_lock(&mut self, _pkt: wow_packet::WorldPacket) {
        self.send_battle_pet_journal_lock_status_like_cpp();
        if self.has_represented_battle_pet_journal_lock_like_cpp() {
            self.send_packet(&self.represented_battle_pet_journal_like_cpp());
        }
    }

    /// CMSG_BATTLE_PET_CLEAR_FANFARE — clear the account battle-pet fanfare bit.
    ///
    /// C++ ref: `WorldSession::HandleBattlePetClearFanfare` forwards only the
    /// pet guid to `BattlePetMgr::ClearFanfare`, which silently ignores unknown
    /// pets.
    pub async fn handle_battle_pet_clear_fanfare(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetClearFanfare::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetClearFanfare parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_clear_fanfare_like_cpp(request.pet_guid);
    }

    /// CMSG_BATTLE_PET_SET_FLAGS — apply/remove represented battle-pet flags.
    ///
    /// C++ first requires the journal lock and then silently ignores unknown
    /// pets.
    pub async fn handle_battle_pet_set_flags(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetSetFlags::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetSetFlags parse failed: {error}"
                );
                return;
            }
        };

        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            return;
        }

        self.battle_pet_set_flags_like_cpp(request.pet_guid, request.flags, request.control_type);
    }

    /// CMSG_BATTLE_PET_SET_BATTLE_SLOT — assign an owned pet to a battle slot.
    ///
    /// C++ silently ignores unknown pets and invalid slots.
    pub async fn handle_battle_pet_set_battle_slot(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetSetBattleSlot::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetSetBattleSlot parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_set_battle_slot_like_cpp(request.pet_guid, request.slot);
    }

    /// CMSG_BATTLE_PET_SUMMON — toggle represented summoned battle-pet guid.
    ///
    /// C++ compares `ActivePlayerData::SummonedBattlePetGUID`; unknown pets are
    /// ignored by `BattlePetMgr::SummonPet`, and matching active pets dismiss.
    /// Full spell cast, creature summon/despawn and `SetBattlePetData` update
    /// fields remain part of the later live battle-pet runtime.
    pub async fn handle_battle_pet_summon(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetSummon::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetSummon parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_summon_toggle_like_cpp(request.pet_guid);
    }

    /// CMSG_BATTLE_PET_UPDATE_NOTIFY — represented update of active companion data.
    ///
    /// C++ `BattlePetMgr::UpdateBattlePetData` ignores unknown pets and only
    /// updates player/summoned-creature battle-pet fields when the currently
    /// summoned companion GUID matches the requested pet GUID.
    pub async fn handle_battle_pet_update_notify(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetUpdateNotify::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetUpdateNotify parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_update_notify_like_cpp(request.pet_guid);
    }

    /// CMSG_BATTLE_PET_UPDATE_DISPLAY_NOTIFY — explicit no-op.
    ///
    /// C++ registers this opcode as `STATUS_UNHANDLED` and dispatches it to
    /// `Handle_NULL`, so Rust intentionally performs no read or mutation.
    pub async fn handle_battle_pet_update_display_notify(&mut self, _pkt: wow_packet::WorldPacket) {
    }

    /// CMSG_QUERY_BATTLE_PET_NAME — represented negative lookup response.
    ///
    /// C++ always sends `QueryBattlePetNameResponse`; until the live summoned
    /// battle-pet creature/name runtime exists, Rust can faithfully close the
    /// unresolved branch (`Allow=false`, zero creature/timestamp).
    pub async fn handle_query_battle_pet_name(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match QueryBattlePetName::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "QueryBattlePetName parse failed: {error}"
                );
                return;
            }
        };

        self.send_packet(&QueryBattlePetNameResponse::not_allowed(
            request.battle_pet_id,
        ));
    }
    pub async fn handle_arena_team_roster(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_request_raid_info(&mut self, _pkt: wow_packet::WorldPacket) {
        let locks = match (self.player_guid(), self.instance_lock_mgr.as_ref()) {
            (Some(player_guid), Some(instance_lock_mgr)) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                instance_lock_mgr
                    .read()
                    .map(|mgr| {
                        let map_store = self.map_store().map(|store| store.as_ref());
                        let map_difficulty_store =
                            self.map_difficulty_store().map(|store| store.as_ref());
                        mgr.get_raid_info_locks_for_player_at(
                            player_guid,
                            now,
                            wow_instances::ResetSchedule::default(),
                            |map_id, difficulty_id| {
                                let map = map_store?.get(map_id)?;
                                let map_difficulty =
                                    map_difficulty_store?.get(map_id, difficulty_id)?;
                                Some(wow_instances::MapDb2Entries {
                                    map_id,
                                    difficulty_id,
                                    lock_id: u32::from(map_difficulty.lock_id),
                                    reset_interval: match map_difficulty.reset_interval {
                                        1 => wow_instances::MapDifficultyResetInterval::Daily,
                                        2 => wow_instances::MapDifficultyResetInterval::Weekly,
                                        _ => wow_instances::MapDifficultyResetInterval::Anytime,
                                    },
                                    is_flex_locking: map.is_flex_locking(),
                                    is_using_encounter_locks: map_difficulty
                                        .is_using_encounter_locks(),
                                })
                            },
                        )
                    })
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        };

        self.send_packet(&InstanceInfo {
            locks: locks
                .into_iter()
                .map(|lock| InstanceLockInfo {
                    instance_id: lock.instance_id,
                    map_id: lock.map_id,
                    difficulty_id: lock.difficulty_id,
                    time_remaining: lock.time_remaining,
                    completed_mask: lock.completed_mask,
                    locked: lock.locked,
                    extended: lock.extended,
                })
                .collect(),
        });
    }

    /// C++ `WorldSession::HandleResetInstancesOpcode`.
    pub async fn handle_reset_instances(&mut self, _pkt: wow_packet::WorldPacket) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        if self
            .map_store()
            .and_then(|store| store.get(u32::from(self.player_map_id_like_cpp())))
            .is_some_and(|map| map.instance_type != 0)
        {
            return;
        }

        let reset_owner_guid = if let Some(group_guid) = self.group_guid {
            let Some(group_registry) = self.group_registry() else {
                return;
            };
            let Some(group) = group_registry.get(&group_guid) else {
                return;
            };
            if group.leader_guid != player_guid {
                return;
            }
            group.leader_guid
        } else {
            player_guid
        };

        let Some(instance_lock_mgr) = self.instance_lock_mgr.as_ref().cloned() else {
            return;
        };

        let mut tx = SqlTransaction::new();
        let reset_result = {
            let mut mgr = match instance_lock_mgr.write() {
                Ok(mgr) => mgr,
                Err(_) => return,
            };
            let entries_by_key = mgr
                .player_lock_map_difficulties(reset_owner_guid)
                .into_iter()
                .filter_map(|(map_id, difficulty_id)| {
                    let map = self.map_store()?.get(map_id)?;
                    let map_difficulty = self.map_difficulty_store()?.get(map_id, difficulty_id)?;
                    let entries = wow_instances::MapDb2Entries {
                        map_id,
                        difficulty_id,
                        lock_id: u32::from(map_difficulty.lock_id),
                        reset_interval: match map_difficulty.reset_interval {
                            1 => wow_instances::MapDifficultyResetInterval::Daily,
                            2 => wow_instances::MapDifficultyResetInterval::Weekly,
                            _ => wow_instances::MapDifficultyResetInterval::Anytime,
                        },
                        is_flex_locking: map.is_flex_locking(),
                        is_using_encounter_locks: map_difficulty.is_using_encounter_locks(),
                    };
                    Some((entries.key(), entries))
                })
                .collect::<std::collections::HashMap<_, _>>();

            mgr.reset_instance_locks_for_player_tx_at(
                &mut tx,
                reset_owner_guid,
                None,
                None,
                &entries_by_key,
                wow_instances::ResetSchedule::default(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0),
            )
        };

        if !tx.is_empty() {
            if let Some(char_db) = self.char_db()
                && let Err(err) = char_db.commit_transaction(tx).await
            {
                warn!(
                    account = self.account_id,
                    player_guid = ?reset_owner_guid,
                    error = ?err,
                    "failed to commit CMSG_RESET_INSTANCES lock reset transaction"
                );
                return;
            }
        }

        for lock in reset_result.reset {
            self.send_packet(&InstanceReset {
                map_id: lock.map_id,
            });
        }

        for lock in reset_result.failed_to_reset {
            self.send_packet(&InstanceResetFailed {
                map_id: lock.map_id,
                reset_failed_reason: 0,
            });
        }
    }

    /// C++ `WorldSession::HandleInstanceLockResponse`.
    pub async fn handle_instance_lock_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let Ok(response) = InstanceLockResponse::read(&mut pkt) else {
            return;
        };

        let Some(pending_bind) = self.pending_bind.take() else {
            info!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "InstanceLockResponse without pending bind"
            );
            return;
        };

        if response.accept_lock {
            self.represented_confirmed_pending_binds
                .push(pending_bind.instance_id);
        } else {
            self.represented_repop_at_graveyard_count =
                self.represented_repop_at_graveyard_count.saturating_add(1);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn send_pending_raid_lock_like_cpp(
        &mut self,
        instance_id: u32,
        completed_mask: u32,
        extending: bool,
        warning_only: bool,
    ) {
        self.send_packet(&PendingRaidLock {
            time_until_lock: 60_000,
            completed_mask,
            extending,
            warning_only,
        });

        if !warning_only {
            self.pending_bind = Some(crate::session::RepresentedPendingBind {
                instance_id,
                time_until_lock_ms: 60_000,
            });
        }
    }

    pub async fn handle_request_conquest_formula_constants(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
    }
    pub async fn handle_request_lfg_list_blacklist(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_lfg_list_get_status(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_get_account_character_list(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_cancel_trade(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ calls Player::TradeCancel(true) only when a player is present.
        // Full trade state is not ported yet; no active trade means no response.
    }
    pub async fn handle_report_client_variables(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_report_enabled_addons(&mut self, _pkt: wow_packet::WorldPacket) {}
    pub async fn handle_report_keybinding_execution_counts(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
    }
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
    /// C++ ref: `GameObject::Use` dispatches by `GameObjectTemplate::type`.
    pub async fn handle_game_obj_use(&mut self, mut pkt: wow_packet::WorldPacket) {
        let gameobject_guid = match pkt.read_packed_guid() {
            Ok(guid) => guid,
            Err(e) => {
                warn!("GameObjUse: failed to read gameobject guid: {e}");
                return;
            }
        };

        if !gameobject_guid.is_game_object() {
            return;
        }

        let gameobject_access = if self.canonical_map_manager.is_some() {
            match self.canonical_gameobject_access_like_cpp(gameobject_guid) {
                Some(access) => access,
                None => return,
            }
        } else {
            if !self
                .client_visible_guids_like_cpp
                .contains(&gameobject_guid)
            {
                return;
            }
            RepresentedGameObjectAccessLikeCpp {
                entry: gameobject_guid.entry(),
                position: self
                    .represented_gameobject_use_states
                    .get(&gameobject_guid)
                    .and_then(|state| state.position)
                    .unwrap_or_default(),
            }
        };

        let Some(world_db) = self.world_db().cloned() else {
            return;
        };
        let mut stmt = world_db.prepare(WorldStatements::SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY);
        stmt.set_u32(0, gameobject_access.entry);
        let result = match world_db.query(&stmt).await {
            Ok(result) => result,
            Err(e) => {
                warn!(
                    entry = gameobject_access.entry,
                    "GameObjUse: failed to query gameobject template: {e}"
                );
                return;
            }
        };
        if result.is_empty() {
            return;
        }

        let go_type = result.try_read::<u32>(1).unwrap_or(0);
        let mut data = [0_u32; MAX_GAMEOBJECT_DATA];
        for (index, value) in data.iter_mut().enumerate() {
            *value = result
                .try_read::<i32>(8 + index)
                .and_then(|raw| u32::try_from(raw).ok())
                .unwrap_or(0);
        }

        let template = GameObjectTemplateData::new(go_type, data);
        self.record_represented_gameobject_template_quest_source_like_cpp(
            gameobject_guid,
            &template,
        );
        let icon_name: String = result.read_string(4);
        if icon_name == "Point" {
            return;
        }
        let interact_distance = represented_gameobject_interaction_distance_like_cpp(
            Some(go_type as u8),
            Some(template.get_interact_radius_override_like_cpp()),
        );
        let Some(player_position) = self.player_position_like_cpp() else {
            return;
        };
        if self.canonical_map_manager.is_some() {
            let Some(verified_access) = self.represented_gameobject_can_interact_with_like_cpp(
                gameobject_guid,
                interact_distance,
            ) else {
                return;
            };
            if verified_access.entry != gameobject_access.entry {
                return;
            }
        } else if !gameobject_access
            .position
            .is_within_dist(&player_position, interact_distance)
        {
            return;
        }
        if !self
            .represented_meets_player_condition_id_like_cpp(template.get_condition_id1_like_cpp())
        {
            debug!(
                account = self.account_id,
                guid = ?gameobject_guid,
                go_type,
                condition_id = template.get_condition_id1_like_cpp(),
                "GameObjUse: represented gameobject interact condition not met"
            );
            return;
        }
        if !self.represented_gameobject_use_allowed_by_mover_like_cpp(
            template.is_usable_mounted_like_cpp(),
        ) {
            return;
        }
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.apply_represented_gameobject_player_use_preamble_like_cpp(
            gameobject_guid,
            player_guid,
            template.is_usable_mounted_like_cpp(),
            template.get_no_damage_immune_like_cpp() != 0,
        ) {
            return;
        }
        if go_type != GAMEOBJECT_TYPE_TRAP
            && !self.apply_represented_gameobject_cooldown_like_cpp(
                gameobject_guid,
                template.get_cooldown_like_cpp(),
            )
        {
            return;
        }

        match go_type {
            GAMEOBJECT_TYPE_DOOR | GAMEOBJECT_TYPE_BUTTON => {
                self.use_represented_gameobject_door_or_button_like_cpp(
                    gameobject_guid,
                    player_guid,
                    template.get_auto_close_time_like_cpp(),
                );
                return;
            }
            GAMEOBJECT_TYPE_QUESTGIVER => {
                if let Some(source) = template.questgiver_use_source_like_cpp() {
                    self.use_represented_gameobject_questgiver_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.entry,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_TRAP => {
                if let Some(source) = template.trap_use_source_like_cpp() {
                    self.use_represented_gameobject_trap_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_FISHING_NODE => {
                let effect_start = self.represented_gameobject_use_effects.len();
                self.use_represented_gameobject_fishing_node_like_cpp(gameobject_guid, player_guid);
                let area_id = self.represented_gameobject_area_id_like_cpp(gameobject_guid);
                let loot_request = self
                    .represented_gameobject_use_effects
                    .get(effect_start..)
                    .unwrap_or(&[])
                    .iter()
                    .rev()
                    .find_map(|effect| match effect {
                        RepresentedGameObjectUseEffect::FishingLootRequested {
                            gameobject_guid: effect_guid,
                            loot_type,
                            ..
                        } if *effect_guid == gameobject_guid => Some(*loot_type),
                        _ => None,
                    });
                match loot_request {
                    Some(LOOT_TYPE_FISHING_LIKE_CPP) => {
                        self.open_represented_fishing_node_loot_like_cpp(
                            gameobject_guid,
                            area_id,
                            false,
                        )
                        .await;
                    }
                    Some(LOOT_TYPE_FISHING_JUNK_LIKE_CPP) => {
                        self.open_represented_fishing_node_loot_like_cpp(
                            gameobject_guid,
                            area_id,
                            true,
                        )
                        .await;
                    }
                    _ => {}
                }
                return;
            }
            GAMEOBJECT_TYPE_RITUAL => {
                if let Some(source) = template.ritual_use_source_like_cpp() {
                    self.use_represented_gameobject_ritual_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_CHAIR => {
                if let Some(source) = template.chair_use_source_like_cpp() {
                    let gameobject_size = result.try_read::<f32>(7).unwrap_or(1.0).max(0.0);
                    self.use_represented_gameobject_chair_like_cpp(
                        gameobject_guid,
                        player_guid,
                        player_position,
                        gameobject_access.position,
                        gameobject_size,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_BARBER_CHAIR => {
                if let Some(source) = template.barber_chair_use_source_like_cpp() {
                    self.use_represented_gameobject_barber_chair_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.position,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_UI_LINK => {
                if let Some(source) = template.ui_link_use_source_like_cpp() {
                    self.use_represented_gameobject_ui_link_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_ITEM_FORGE => {
                if let Some(source) = template.item_forge_use_source_like_cpp() {
                    self.use_represented_gameobject_item_forge_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_CAPTURE_POINT => {
                if let Some(source) = template.capture_point_use_source_like_cpp() {
                    self.use_represented_gameobject_capture_point_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_FLAGSTAND => {
                if let Some(source) = template.flag_stand_use_source_like_cpp() {
                    self.use_represented_gameobject_flagstand_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_FLAGDROP => {
                if let Some(source) = template.flag_drop_use_source_like_cpp() {
                    self.use_represented_gameobject_flagdrop_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_guid.entry(),
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_NEW_FLAG => {
                if let Some(source) = template.new_flag_use_source_like_cpp() {
                    self.use_represented_gameobject_new_flag_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.entry,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_NEW_FLAG_DROP => {
                if let Some(source) = template.new_flag_drop_use_source_like_cpp() {
                    self.use_represented_gameobject_new_flag_drop_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_MEETINGSTONE => {
                if let Some(mut source) = template.meeting_stone_use_source_like_cpp() {
                    source.content_tuning_id = result.try_read::<u32>(43).unwrap_or(0);
                    self.use_represented_gameobject_meeting_stone_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.entry,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_SPELL_FOCUS => {
                self.use_represented_gameobject_spell_focus_like_cpp(
                    gameobject_guid,
                    player_guid,
                    template.spell_focus_linked_trap_like_cpp(),
                );
                return;
            }
            GAMEOBJECT_TYPE_SPELLCASTER => {
                if let Some(source) = template.spellcaster_use_source_like_cpp() {
                    self.use_represented_gameobject_spellcaster_like_cpp(
                        gameobject_guid,
                        player_guid,
                        gameobject_access.entry,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_CAMERA => {
                if let Some(source) = template.camera_use_source_like_cpp() {
                    self.use_represented_gameobject_camera_like_cpp(
                        gameobject_guid,
                        player_guid,
                        source,
                    );
                }
                return;
            }
            GAMEOBJECT_TYPE_GOOBER => {
                if let Some(source) = template.goober_use_source_like_cpp() {
                    if self
                        .use_represented_gameobject_goober_preamble_like_cpp(
                            gameobject_guid,
                            gameobject_access.entry,
                            gameobject_access.position,
                            player_guid,
                            source,
                        )
                        .await
                    {
                        self.use_represented_gameobject_goober_state_like_cpp(
                            gameobject_guid,
                            player_guid,
                            gameobject_access.entry,
                            source,
                        );
                    }
                }
                return;
            }
            _ => {}
        }

        if let Some(source) = template.chest_loot_source_like_cpp() {
            if source.is_empty() {
                return;
            }

            self.open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
                .await;
            return;
        }

        let loot_id = template.get_loot_id_like_cpp();
        match go_type {
            GAMEOBJECT_TYPE_FISHING_HOLE if loot_id != 0 => {
                self.open_represented_fishing_hole_like_cpp(
                    gameobject_guid,
                    gameobject_access.entry,
                    loot_id,
                )
                .await;
            }
            GAMEOBJECT_TYPE_GATHERING_NODE => {
                if let Some(source) = template.gathering_node_use_source_like_cpp() {
                    self.open_represented_gathering_node_like_cpp(
                        gameobject_guid,
                        gameobject_access.entry,
                        source,
                    )
                    .await;
                }
            }
            _ => {
                debug!(
                    account = self.account_id,
                    guid = ?gameobject_guid,
                    go_type,
                    "GameObjUse: represented gameobject use type is not ported yet"
                );
            }
        }
    }

    /// CMSG_GAME_OBJ_REPORT_USE — client reports a game object use event.
    /// C++ ref: `WorldSession::HandleGameobjectReportUse`.
    pub async fn handle_game_obj_report_use(&mut self, mut pkt: wow_packet::WorldPacket) {
        let gameobject_guid = match pkt.read_packed_guid() {
            Ok(guid) => guid,
            Err(e) => {
                warn!("GameObjReportUse: failed to read gameobject guid: {e}");
                return;
            }
        };

        if !gameobject_guid.is_game_object() {
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if self.player_moved_unit_guid_like_cpp() != player_guid {
            return;
        }

        let state = self.represented_gameobject_use_states.get(&gameobject_guid);
        let interaction_distance = represented_gameobject_interaction_distance_like_cpp(
            state.and_then(|state| state.go_type),
            state.and_then(|state| state.interact_radius_override),
        );

        let gameobject_access = if self.canonical_map_manager.is_some() {
            match self.represented_gameobject_can_interact_with_like_cpp(
                gameobject_guid,
                interaction_distance,
            ) {
                Some(access) => access,
                None => return,
            }
        } else {
            if !self
                .client_visible_guids_like_cpp
                .contains(&gameobject_guid)
            {
                return;
            }
            let Some(position) = state.and_then(|state| state.position) else {
                return;
            };
            let Some(player_position) = self.player_position_like_cpp() else {
                return;
            };
            if !position.is_within_dist(&player_position, interaction_distance) {
                return;
            }
            RepresentedGameObjectAccessLikeCpp {
                entry: gameobject_guid.entry(),
                position,
            }
        };
        #[cfg(not(test))]
        let _ = gameobject_access;

        if self.record_represented_gameobject_report_use_ai_like_cpp(gameobject_guid, player_guid) {
            return;
        }

        #[cfg(test)]
        {
            self.represented_gameobject_criteria_events.push(
                crate::session::RepresentedGameObjectCriteriaEvent::UseGameobject {
                    player_guid,
                    gameobject_entry: gameobject_access.entry,
                },
            );
        }
    }

    /// CMSG_CLOSE_INTERACTION — player closed an NPC interaction window.
    /// C# ref: MiscHandler.HandleCloseInteraction → resets interaction data.
    pub async fn handle_close_interaction(&mut self, _pkt: wow_packet::WorldPacket) {
        // TODO: reset PlayerTalkClass interaction data and stable master.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};
    use wow_constants::{ClientOpcodes, ServerOpcodes};
    use wow_core::{ObjectGuid, Position, guid::HighGuid};
    use wow_data::progression_rewards::{FactionEntry, FactionStore};
    use wow_data::reputation::{ReputationFlagsLikeCpp, ReputationRankLikeCpp};
    use wow_data::{
        ItemRecord, ItemSearchNameEntry, ItemSearchNameStore, ItemSparseTemplateEntry,
        ItemStatsStore, ItemStore, MapDifficultyEntry, MapDifficultyStore, MapEntry, MapStore,
    };
    use wow_packet::ServerPacket;
    use wow_packet::WorldPacket;
    use wow_packet::packets::misc::empty_battle_pet_guid_like_cpp;

    #[test]
    fn item_purchase_contents_skip_season_earned_currency_like_cpp() {
        let extended_cost = wow_data::item_extended_cost::ItemExtendedCostEntry {
            id: 1,
            required_arena_rating: 0,
            arena_bracket: 0,
            flags: ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2,
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [100, 0, 0, 0, 0],
            item_count: [2, 0, 0, 0, 0],
            currency_id: [390, 391, 0, 0, 0],
            currency_count: [5, 7, 0, 0, 0],
        };

        let contents = item_purchase_contents_from_extended_cost(&extended_cost, 123);
        assert_eq!(contents.money, 123);
        assert_eq!(contents.items[0].item_id, 100);
        assert_eq!(contents.items[0].item_count, 2);
        assert_eq!(contents.currencies[0].currency_id, 390);
        assert_eq!(contents.currencies[0].currency_count, 5);
        assert_eq!(contents.currencies[1].currency_id, 0);
        assert_eq!(contents.currencies[1].currency_count, 0);
    }

    fn make_session() -> (crate::session::WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded(8);
        let (send_tx, send_rx) = flume::bounded(8);
        (
            crate::session::WorldSession::new(
                1,
                "TestAccount".into(),
                0,
                2,
                9,
                54261,
                vec![0; 40],
                "enUS".into(),
                pkt_rx,
                send_tx,
            ),
            send_rx,
        )
    }

    fn install_add_toy_item_templates(
        session: &mut crate::session::WorldSession,
        toy_item_id: u32,
        toy_flags2: u32,
    ) {
        session.set_item_store(Arc::new(ItemStore::from_records([
            ItemRecord {
                id: 101,
                class_id: wow_constants::ItemClass::Container as u8,
                subclass_id: 0,
                material: 0,
                inventory_type: wow_constants::InventoryType::Bag as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
            },
            ItemRecord {
                id: toy_item_id,
                class_id: wow_constants::ItemClass::Miscellaneous as u8,
                subclass_id: 0,
                material: 0,
                inventory_type: wow_constants::InventoryType::NonEquip as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
            },
        ])));
        session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
            ItemSearchNameEntry {
                id: 101,
                allowable_race: 0,
                display: String::new(),
                overall_quality_id: 1,
                expansion_id: 0,
                min_faction_id: 0,
                min_reputation: 0,
                allowable_class: 0,
                required_level: 0,
                required_skill: 0,
                required_skill_rank: 0,
                required_ability: 0,
                item_level: 1,
                flags: [0; 4],
            },
            ItemSearchNameEntry {
                id: toy_item_id,
                allowable_race: 0,
                display: String::new(),
                overall_quality_id: 1,
                expansion_id: 0,
                min_faction_id: 0,
                min_reputation: 0,
                allowable_class: 0,
                required_level: 0,
                required_skill: 0,
                required_skill_rank: 0,
                required_ability: 0,
                item_level: 1,
                flags: [0; 4],
            },
        ])));
        session.set_item_stats_store(Arc::new(
            ItemStatsStore::from_sparse_and_random_property_templates(
                [
                    (
                        101,
                        ItemSparseTemplateEntry {
                            flags: [0; 4],
                            bag_family: 0,
                            start_quest_id: 0,
                            stackable: 1,
                            max_count: 0,
                            lock_id: 0,
                            required_reputation_rank: 0,
                            sell_price: 0,
                            buy_price: 0,
                            vendor_stack_count: 1,
                            price_variance: 0.0,
                            price_random_value: 0.0,
                            max_durability: 0,
                            limit_category: 0,
                            instance_bound: 0,
                            zone_bound: [0, 0],
                            required_reputation_faction: 0,
                            allowable_class: 0,
                            required_expansion: 0,
                            bonding: wow_constants::ItemBondingType::None as u8,
                            container_slots: 12,
                            inventory_type: wow_constants::InventoryType::Bag as i8,
                        },
                    ),
                    (
                        toy_item_id,
                        ItemSparseTemplateEntry {
                            flags: [0, toy_flags2, 0, 0],
                            bag_family: 0,
                            start_quest_id: 0,
                            stackable: 1,
                            max_count: 0,
                            lock_id: 0,
                            required_reputation_rank: 0,
                            sell_price: 0,
                            buy_price: 0,
                            vendor_stack_count: 1,
                            price_variance: 0.0,
                            price_random_value: 0.0,
                            max_durability: 0,
                            limit_category: 0,
                            instance_bound: 0,
                            zone_bound: [0, 0],
                            required_reputation_faction: 0,
                            allowable_class: 0,
                            required_expansion: 0,
                            bonding: wow_constants::ItemBondingType::None as u8,
                            container_slots: 0,
                            inventory_type: wow_constants::InventoryType::NonEquip as i8,
                        },
                    ),
                ],
                [],
            ),
        ));
    }

    fn shared_canonical_map_manager_for_misc_test() -> crate::session::SharedCanonicalMapManager {
        Arc::new(Mutex::new(wow_map::MapManager::default()))
    }

    fn add_canonical_test_player_on_map_for_misc_test(
        canonical: &crate::session::SharedCanonicalMapManager,
        guid: ObjectGuid,
        position: Position,
        map_id: u32,
        instance_id: u32,
    ) {
        let mut player = wow_entities::Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_name("ToyDynamicTester");
        player
            .unit_mut()
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        player.unit_mut().world_mut().relocate(position);
        player.unit_mut().world_mut().object_mut().add_to_world();

        canonical
            .lock()
            .unwrap()
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }

    #[tokio::test]
    async fn game_obj_report_use_records_use_criteria_from_canonical_go_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        let player_guid = ObjectGuid::create_player(1, 99);
        let gameobject_guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            5,
        );

        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
        session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.record_represented_gameobject_runtime_state_like_cpp(
            571,
            gameobject_guid,
            777,
            Position::new(14.0, 0.0, 0.0, 0.0),
            3,
        );

        let mut gameobject = wow_entities::GameObject::new();
        gameobject.world_mut().object_mut().create(gameobject_guid);
        gameobject.world_mut().object_mut().set_entry(777);
        gameobject.world_mut().set_map(571, 0).unwrap();
        gameobject
            .world_mut()
            .relocate(Position::new(14.0, 0.0, 0.0, 0.0));
        gameobject.world_mut().object_mut().add_to_world();
        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
            )
            .unwrap();

        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&gameobject_guid);
        session.handle_game_obj_report_use(pkt).await;

        assert_eq!(
            session.represented_gameobject_criteria_events,
            vec![
                crate::session::RepresentedGameObjectCriteriaEvent::UseGameobject {
                    player_guid,
                    gameobject_entry: 777,
                }
            ]
        );
    }

    #[tokio::test]
    async fn game_obj_report_use_ignores_remote_control_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        let player_guid = ObjectGuid::create_player(1, 99);
        let controlled_guid = ObjectGuid::create_player(1, 100);
        let gameobject_guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            6,
        );

        session.set_player_guid(Some(player_guid));
        session.set_player_moved_unit_guid_like_cpp(controlled_guid);
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
        session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.record_represented_gameobject_runtime_state_like_cpp(
            571,
            gameobject_guid,
            777,
            Position::new(14.0, 0.0, 0.0, 0.0),
            3,
        );

        let mut gameobject = wow_entities::GameObject::new();
        gameobject.world_mut().object_mut().create(gameobject_guid);
        gameobject.world_mut().object_mut().set_entry(777);
        gameobject.world_mut().set_map(571, 0).unwrap();
        gameobject
            .world_mut()
            .relocate(Position::new(14.0, 0.0, 0.0, 0.0));
        gameobject.world_mut().object_mut().add_to_world();
        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
            )
            .unwrap();

        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&gameobject_guid);
        session.handle_game_obj_report_use(pkt).await;

        assert!(session.represented_gameobject_criteria_events.is_empty());
    }

    #[tokio::test]
    async fn game_obj_report_use_ai_can_consume_criteria_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        let player_guid = ObjectGuid::create_player(1, 99);
        let gameobject_guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            571,
            0,
            777,
            7,
        );

        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
        session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.record_represented_gameobject_runtime_state_like_cpp(
            571,
            gameobject_guid,
            777,
            Position::new(14.0, 0.0, 0.0, 0.0),
            3,
        );
        session
            .represented_gameobject_use_states
            .get_mut(&gameobject_guid)
            .unwrap()
            .report_use_ai_returns_true = true;

        let mut gameobject = wow_entities::GameObject::new();
        gameobject.world_mut().object_mut().create(gameobject_guid);
        gameobject.world_mut().object_mut().set_entry(777);
        gameobject.world_mut().set_map(571, 0).unwrap();
        gameobject
            .world_mut()
            .relocate(Position::new(14.0, 0.0, 0.0, 0.0));
        gameobject.world_mut().object_mut().add_to_world();
        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
            )
            .unwrap();

        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&gameobject_guid);
        session.handle_game_obj_report_use(pkt).await;

        assert_eq!(
            session.represented_gameobject_use_effects,
            vec![
                crate::session::RepresentedGameObjectUseEffect::ReportUseAi {
                    gameobject_guid,
                    player_guid,
                    handled: true,
                }
            ]
        );
        assert!(session.represented_gameobject_criteria_events.is_empty());
    }

    #[tokio::test]
    async fn mount_set_favorite_updates_known_mount_and_sends_partial_update_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_account_mounts_like_cpp(vec![wow_packet::packets::misc::AccountMount {
            spell_id: 1234,
            flags: 0,
        }]);

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::MountSetFavorite as u16);
        pkt.write_uint32(1234);
        pkt.write_bit(true);
        pkt.flush_bits();

        session.handle_mount_set_favorite(pkt).await;

        assert_eq!(session.account_mounts_like_cpp().get(&1234), Some(&0x01));
        let bytes = send_rx.try_recv().expect("partial mount update");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::AccountMountUpdate as u16
        );
        assert_eq!(bytes[2], 0x00);
        assert_eq!(
            i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            1
        );
        assert_eq!(
            i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
            1234
        );
        assert_eq!(bytes[11], 0x10);
    }

    #[tokio::test]
    async fn mount_set_favorite_ignores_unknown_mount_like_cpp() {
        let (mut session, send_rx) = make_session();

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::MountSetFavorite as u16);
        pkt.write_uint32(1234);
        pkt.write_bit(true);
        pkt.flush_bits();

        session.handle_mount_set_favorite(pkt).await;

        assert!(session.account_mounts_like_cpp().is_empty());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn toy_clear_fanfare_clears_known_toy_without_packet_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.load_represented_account_toys_like_cpp([(30_000, true, true)]);

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::ToyClearFanfare as u16);
        pkt.write_uint32(30_000);

        session.handle_toy_clear_fanfare(pkt).await;

        assert_eq!(
            session.account_toy_rows_like_cpp(),
            vec![(30_000, true, false)]
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn toy_clear_fanfare_ignores_unknown_toy_like_cpp() {
        let (mut session, send_rx) = make_session();

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::ToyClearFanfare as u16);
        pkt.write_uint32(40_000);

        session.handle_toy_clear_fanfare(pkt).await;

        assert!(session.account_toy_rows_like_cpp().is_empty());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn add_toy_finds_nested_bag_item_by_guid_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 55);
        let bag_guid = ObjectGuid::create_item(1, 1_001);
        let toy_guid = ObjectGuid::create_item(1, 1_002);
        let bag_slot = wow_entities::INVENTORY_SLOT_BAG_START;
        let toy_slot = 5;
        let toy_item_id = 30_000_u32;
        let toy_item_id_i32 = i32::try_from(toy_item_id).unwrap();

        session.set_player_guid(Some(player_guid));
        install_add_toy_item_templates(&mut session, toy_item_id, 0);
        session.set_toy_store(Arc::new(wow_data::ToyStore::from_entries([
            wow_data::ToyEntry {
                id: 1,
                source_text: "known".to_string(),
                item_id: toy_item_id_i32,
                flags: 0,
                source_type_enum: 0,
            },
        ])));
        session.load_represented_account_toys_like_cpp([(toy_item_id, false, false)]);
        session.insert_inventory_item_like_cpp(
            bag_slot,
            crate::session::InventoryItem {
                guid: bag_guid,
                entry_id: 101,
                db_guid: bag_guid.counter() as u64,
                inventory_type: Some(wow_constants::InventoryType::Bag as u8),
            },
        );
        let bag_item = session.make_inventory_item_object(
            bag_guid,
            101,
            player_guid,
            1,
            0,
            wow_constants::ItemContext::None,
            bag_slot,
        );
        session.insert_inventory_item_object(bag_item);
        let mut toy_item = session.make_inventory_item_object(
            toy_guid,
            toy_item_id,
            player_guid,
            1,
            0,
            wow_constants::ItemContext::None,
            toy_slot,
        );
        toy_item.set_container_guid_and_slot(bag_guid, bag_slot);
        session.insert_inventory_item_object(toy_item);

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::AddToy as u16);
        pkt.write_packed_guid(&toy_guid);

        session.handle_add_toy(pkt).await;

        assert_eq!(
            session.account_toy_rows_like_cpp(),
            vec![(toy_item_id, false, false)]
        );
        assert!(
            session
                .inventory_item_objects_like_cpp()
                .contains_key(&toy_guid)
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn add_toy_uses_can_use_item_faction_gate_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 55);
        let toy_guid = ObjectGuid::create_item(1, 1_002);
        let toy_slot = 23;
        let toy_item_id = 30_000_u32;
        let toy_item_id_i32 = i32::try_from(toy_item_id).unwrap();

        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        install_add_toy_item_templates(
            &mut session,
            toy_item_id,
            wow_constants::ItemFlags2::FactionHorde as u32,
        );
        session.set_toy_store(Arc::new(wow_data::ToyStore::from_entries([
            wow_data::ToyEntry {
                id: 1,
                source_text: "known".to_string(),
                item_id: toy_item_id_i32,
                flags: 0,
                source_type_enum: 0,
            },
        ])));
        session.insert_inventory_item_like_cpp(
            toy_slot,
            crate::session::InventoryItem {
                guid: toy_guid,
                entry_id: toy_item_id,
                db_guid: toy_guid.counter() as u64,
                inventory_type: Some(wow_constants::InventoryType::NonEquip as u8),
            },
        );
        let toy_item = session.make_inventory_item_object(
            toy_guid,
            toy_item_id,
            player_guid,
            1,
            0,
            wow_constants::ItemContext::None,
            toy_slot,
        );
        session.insert_inventory_item_object(toy_item);

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::AddToy as u16);
        pkt.write_packed_guid(&toy_guid);

        session.handle_add_toy(pkt).await;

        assert!(session.account_toy_rows_like_cpp().is_empty());
        assert!(
            session
                .inventory_item_objects_like_cpp()
                .contains_key(&toy_guid)
        );
        assert_eq!(
            send_rx.try_recv().unwrap(),
            InventoryChangeFailure::new(
                InventoryResult::CantEquipEver,
                toy_guid,
                ObjectGuid::EMPTY
            )
            .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn add_toy_rolls_back_without_player_toys_update_when_destroy_fails_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        let player_guid = ObjectGuid::create_player(1, 55);
        let toy_guid = ObjectGuid::create_item(1, 1_003);
        let toy_slot = 5;
        let toy_item_id = 30_000_u32;
        let toy_item_id_i32 = i32::try_from(toy_item_id).unwrap();
        let player_position = Position::new(10.0, 0.0, 0.0, 0.0);

        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
            player_guid,
            "ToyDynamicTester".to_string(),
            player_position,
            571,
            1,
            1,
            80,
            0,
        ));
        add_canonical_test_player_on_map_for_misc_test(
            &canonical,
            player_guid,
            player_position,
            571,
            0,
        );
        session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());
        install_add_toy_item_templates(&mut session, toy_item_id, 0);
        session.set_toy_store(Arc::new(wow_data::ToyStore::from_entries([
            wow_data::ToyEntry {
                id: 1,
                source_text: "known".to_string(),
                item_id: toy_item_id_i32,
                flags: 0,
                source_type_enum: 0,
            },
        ])));
        session.insert_inventory_item_like_cpp(
            toy_slot,
            crate::session::InventoryItem {
                guid: toy_guid,
                entry_id: toy_item_id,
                db_guid: toy_guid.counter() as u64,
                inventory_type: Some(wow_constants::InventoryType::NonEquip as u8),
            },
        );
        let toy_item = session.make_inventory_item_object(
            toy_guid,
            toy_item_id,
            player_guid,
            1,
            0,
            wow_constants::ItemContext::None,
            toy_slot,
        );
        session.insert_inventory_item_object(toy_item);

        let (_, _, preflight_item) = session
            .get_inventory_item_by_guid_like_cpp(toy_guid)
            .expect("toy item guid should resolve before AddToy");
        assert!(session.is_toy_item_like_cpp(preflight_item.entry_id));
        let runtime_item = session
            .inventory_item_objects_like_cpp()
            .get(&preflight_item.guid)
            .cloned();
        assert_eq!(
            session.can_use_inventory_item_represented_like_cpp(
                &preflight_item,
                runtime_item.as_ref()
            ),
            InventoryResult::Ok
        );

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::AddToy as u16);
        pkt.write_packed_guid(&toy_guid);

        session.handle_add_toy(pkt).await;

        let first_packet = send_rx.try_recv().ok();
        assert!(session.account_toy_rows_like_cpp().is_empty());
        assert_eq!(
            session
                .mutate_canonical_player_like_cpp(|player| player.toys_like_cpp().to_vec())
                .unwrap(),
            Vec::<i32>::new()
        );
        assert!(
            first_packet.is_none(),
            "first sent packet: {:?}",
            first_packet
        );
    }

    #[tokio::test]
    async fn add_player_toy_dynamic_field_sends_update_object_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        let player_guid = ObjectGuid::create_player(1, 56);
        let toy_item_id = 30_000_u32;
        let toy_item_id_i32 = i32::try_from(toy_item_id).unwrap();
        let player_position = Position::new(10.0, 0.0, 0.0, 0.0);

        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
            player_guid,
            "ToyDynamicTester".to_string(),
            player_position,
            571,
            1,
            1,
            80,
            0,
        ));
        add_canonical_test_player_on_map_for_misc_test(
            &canonical,
            player_guid,
            player_position,
            571,
            0,
        );
        session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

        let update = session
            .add_player_toy_dynamic_field_like_cpp(toy_item_id)
            .expect("canonical current player should receive Player::AddToy dynamic field");
        if let Some(packet) = player_values_update_to_update_object(
            player_guid,
            session.player_map_id_like_cpp(),
            &update,
        ) {
            session.send_packet(&packet);
        }

        assert_eq!(
            session
                .mutate_canonical_player_like_cpp(|player| player.toys_like_cpp().to_vec())
                .unwrap(),
            vec![toy_item_id_i32]
        );
        let update_packet = send_rx.try_recv().expect("Player::AddToy values update");
        assert_eq!(
            u16::from_le_bytes([update_packet[0], update_packet[1]]),
            ServerOpcodes::UpdateObject as u16
        );
    }

    fn collection_item_set_favorite_packet(
        collection_type: u32,
        id: u32,
        favorite: bool,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::CollectionItemSetFavorite as u16);
        pkt.write_uint32(collection_type);
        pkt.write_uint32(id);
        pkt.write_bit(favorite);
        pkt.flush_bits();
        pkt
    }

    #[tokio::test]
    async fn collection_item_set_favorite_marks_permanent_appearance_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.represented_item_appearances_like_cpp.insert(65);

        session
            .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
                COLLECTION_TYPE_APPEARANCE_LIKE_CPP,
                65,
                true,
            ))
            .await;

        assert_eq!(
            session.represented_favorite_item_appearance_state_like_cpp(65),
            Some(crate::session::FavoriteAppearanceStateLikeCpp::New)
        );
        let bytes = send_rx
            .try_recv()
            .expect("partial transmog favorite update");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::UpdateCapturePoint as u16
        );
        assert_eq!(bytes[2], 0b0100_0000);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(bytes[7..11].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(bytes[11..15].try_into().unwrap()), 65);
    }

    #[tokio::test]
    async fn collection_item_set_favorite_toggles_known_toy_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.load_represented_account_toys_like_cpp([(30_000, false, true)]);

        session
            .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
                COLLECTION_TYPE_TOYBOX_LIKE_CPP,
                30_000,
                true,
            ))
            .await;

        assert_eq!(
            session.account_toy_rows_like_cpp(),
            vec![(30_000, true, true)]
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn collection_item_set_favorite_ignores_unknown_toy_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
                COLLECTION_TYPE_TOYBOX_LIKE_CPP,
                40_000,
                true,
            ))
            .await;

        assert!(session.account_toy_rows_like_cpp().is_empty());
        assert!(send_rx.try_recv().is_err());
    }

    fn battle_pet_clear_fanfare_packet(pet_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetClearFanfare as u16);
        pkt.write_packed_guid(&pet_guid);
        pkt
    }

    fn battle_pet_set_flags_packet(
        pet_guid: ObjectGuid,
        flags: u16,
        control_type: u8,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetSetFlags as u16);
        pkt.write_packed_guid(&pet_guid);
        pkt.write_uint16(flags);
        pkt.write_bits(u32::from(control_type), 2);
        pkt.flush_bits();
        pkt
    }

    fn battle_pet_set_battle_slot_packet(pet_guid: ObjectGuid, slot: u8) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetSetBattleSlot as u16);
        pkt.write_packed_guid(&pet_guid);
        pkt.write_uint8(slot);
        pkt
    }

    fn battle_pet_summon_packet(pet_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetSummon as u16);
        pkt.write_packed_guid(&pet_guid);
        pkt
    }

    fn battle_pet_update_notify_packet(pet_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetUpdateNotify as u16);
        pkt.write_packed_guid(&pet_guid);
        pkt
    }

    fn battle_pet_update_display_notify_packet() -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetUpdateDisplayNotify as u16);
        pkt
    }

    fn query_battle_pet_name_packet(
        battle_pet_id: ObjectGuid,
        unit_guid: ObjectGuid,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::QueryBattlePetName as u16);
        pkt.write_packed_guid(&battle_pet_id);
        pkt.write_packed_guid(&unit_guid);
        pkt
    }

    fn battle_pet_request_journal_lock_packet() -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetRequestJournalLock as u16);
        pkt
    }

    fn battle_pet_request_journal_packet() -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::BattlePetRequestJournal as u16);
        pkt
    }

    #[tokio::test]
    async fn battle_pet_request_journal_lock_sends_acquired_then_journal_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
            .await;

        assert!(session.has_represented_battle_pet_journal_lock_like_cpp());
        let bytes = send_rx
            .try_recv()
            .expect("battle pet journal lock acquired packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::BattlePetJournalLockAcquired as u16
        );
        assert_eq!(bytes.len(), 2);

        let journal_bytes = send_rx.try_recv().expect("battle pet journal packet");
        assert_eq!(
            u16::from_le_bytes([journal_bytes[0], journal_bytes[1]]),
            ServerOpcodes::BattlePetJournal as u16
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_request_journal_acquires_lock_then_sends_empty_journal_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_battle_pet_request_journal(battle_pet_request_journal_packet())
            .await;

        let lock_bytes = send_rx.try_recv().expect("journal lock acquired packet");
        assert_eq!(
            u16::from_le_bytes([lock_bytes[0], lock_bytes[1]]),
            ServerOpcodes::BattlePetJournalLockAcquired as u16
        );

        let journal_bytes = send_rx.try_recv().expect("battle pet journal packet");
        assert_eq!(
            u16::from_le_bytes([journal_bytes[0], journal_bytes[1]]),
            ServerOpcodes::BattlePetJournal as u16
        );
        let mut body = WorldPacket::from_bytes(&journal_bytes[2..]);
        assert_eq!(body.read_uint16().unwrap(), 0);
        assert_eq!(body.read_uint32().unwrap(), 3);
        assert_eq!(body.read_uint32().unwrap(), 0);
        assert!(body.read_bit().unwrap());
        for index in 0..3 {
            assert_eq!(
                body.read_packed_guid().unwrap(),
                empty_battle_pet_guid_like_cpp()
            );
            assert_eq!(body.read_uint32().unwrap(), 0);
            assert_eq!(body.read_uint8().unwrap(), index);
            assert!(body.read_bit().unwrap());
        }
        assert_eq!(body.remaining(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_request_journal_with_lock_sends_only_journal_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.send_battle_pet_journal_lock_status_like_cpp();
        let _ = send_rx.try_recv().expect("initial lock packet");

        session
            .handle_battle_pet_request_journal(battle_pet_request_journal_packet())
            .await;

        let journal_bytes = send_rx.try_recv().expect("battle pet journal packet");
        assert_eq!(
            u16::from_le_bytes([journal_bytes[0], journal_bytes[1]]),
            ServerOpcodes::BattlePetJournal as u16
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_request_journal_sends_represented_pet_rows_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x4338);
        session.set_player_guid(Some(player_guid));
        session.add_represented_battle_pet_packet_info_like_cpp(
            pet_guid,
            crate::session::RepresentedBattlePetDataLikeCpp {
                species: 11,
                creature_id: 22,
                display_id: 33,
                breed: 44,
                level: 55,
                exp: 66,
                flags: 77,
                power: 88,
                health: 99,
                max_health: 111,
                speed: 222,
                quality: 3,
                owner_info: Some(wow_packet::packets::misc::BattlePetJournalPetOwnerInfo {
                    guid: player_guid,
                    player_virtual_realm: 123,
                    player_native_realm: 456,
                }),
                name: "Misha".to_string(),
                save_info: crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            },
        );
        assert!(session.battle_pet_set_battle_slot_like_cpp(pet_guid, 0));

        session
            .handle_battle_pet_request_journal(battle_pet_request_journal_packet())
            .await;

        let _ = send_rx.try_recv().expect("journal lock acquired packet");
        let journal_bytes = send_rx.try_recv().expect("battle pet journal packet");
        let mut body = WorldPacket::from_bytes(&journal_bytes[2..]);
        assert_eq!(body.read_uint16().unwrap(), 0);
        assert_eq!(body.read_uint32().unwrap(), 3);
        assert_eq!(body.read_uint32().unwrap(), 1);
        assert!(body.read_bit().unwrap());
        assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
        assert_eq!(body.read_uint32().unwrap(), 0);
        assert_eq!(body.read_uint8().unwrap(), 0);
        assert!(body.read_bit().unwrap());
        for index in 1..3 {
            assert_eq!(
                body.read_packed_guid().unwrap(),
                empty_battle_pet_guid_like_cpp()
            );
            assert_eq!(body.read_uint32().unwrap(), 0);
            assert_eq!(body.read_uint8().unwrap(), index);
            assert!(body.read_bit().unwrap());
        }
        assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
        assert_eq!(body.read_uint32().unwrap(), 11);
        assert_eq!(body.read_uint32().unwrap(), 22);
        assert_eq!(body.read_uint32().unwrap(), 33);
        assert_eq!(body.read_uint16().unwrap(), 44);
        assert_eq!(body.read_uint16().unwrap(), 55);
        assert_eq!(body.read_uint16().unwrap(), 66);
        assert_eq!(body.read_uint16().unwrap(), 77);
        assert_eq!(body.read_uint32().unwrap(), 88);
        assert_eq!(body.read_uint32().unwrap(), 99);
        assert_eq!(body.read_uint32().unwrap(), 111);
        assert_eq!(body.read_uint32().unwrap(), 222);
        assert_eq!(body.read_uint8().unwrap(), 3);
        assert_eq!(body.read_bits(7).unwrap(), 5);
        assert!(body.read_bit().unwrap());
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.read_string(5).unwrap(), "Misha");
        assert_eq!(body.read_packed_guid().unwrap(), player_guid);
        assert_eq!(body.read_uint32().unwrap(), 123);
        assert_eq!(body.read_uint32().unwrap(), 456);
        assert_eq!(body.remaining(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_clear_fanfare_clears_known_pet_silently_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x223);
        session.add_represented_battle_pet_like_cpp(
            pet_guid,
            crate::session::BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP | 0x20,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        );

        session
            .handle_battle_pet_clear_fanfare(battle_pet_clear_fanfare_packet(pet_guid))
            .await;

        assert_eq!(
            session.represented_battle_pet_like_cpp(pet_guid),
            Some(
                crate::session::RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                    0x20,
                    crate::session::RepresentedBattlePetSaveInfoLikeCpp::Changed,
                )
            )
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_clear_fanfare_ignores_unknown_pet_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x224);

        session
            .handle_battle_pet_clear_fanfare(battle_pet_clear_fanfare_packet(pet_guid))
            .await;

        assert!(session.represented_battle_pet_like_cpp(pet_guid).is_none());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_set_flags_applies_known_pet_silently_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x225);
        session.add_represented_battle_pet_like_cpp(
            pet_guid,
            0x01,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        );

        session
            .handle_battle_pet_set_flags(battle_pet_set_flags_packet(
                pet_guid,
                0x04,
                crate::session::BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP,
            ))
            .await;
        assert_eq!(
            session.represented_battle_pet_like_cpp(pet_guid),
            Some(
                crate::session::RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                    0x01,
                    crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
                )
            )
        );
        assert!(send_rx.try_recv().is_err());

        session
            .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
            .await;
        let _ = send_rx.try_recv().expect("lock acquired packet");
        let _ = send_rx.try_recv().expect("battle pet journal packet");

        session
            .handle_battle_pet_set_flags(battle_pet_set_flags_packet(
                pet_guid,
                0x04,
                crate::session::BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP,
            ))
            .await;

        assert_eq!(
            session.represented_battle_pet_like_cpp(pet_guid),
            Some(
                crate::session::RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                    0x05,
                    crate::session::RepresentedBattlePetSaveInfoLikeCpp::Changed,
                )
            )
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_set_battle_slot_assigns_known_pet_silently_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x226);
        let unknown_guid = ObjectGuid::new(0, 0x227);
        session.add_represented_battle_pet_like_cpp(
            pet_guid,
            0,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        );

        session
            .handle_battle_pet_set_battle_slot(battle_pet_set_battle_slot_packet(pet_guid, 1))
            .await;
        assert_eq!(
            session.represented_battle_pet_slot_like_cpp(1),
            Some(pet_guid)
        );

        session
            .handle_battle_pet_set_battle_slot(battle_pet_set_battle_slot_packet(unknown_guid, 2))
            .await;
        assert_eq!(session.represented_battle_pet_slot_like_cpp(2), None);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_summon_toggles_known_pet_silently_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x228);
        session.add_represented_battle_pet_like_cpp(
            pet_guid,
            0,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        );

        session
            .handle_battle_pet_summon(battle_pet_summon_packet(pet_guid))
            .await;
        assert_eq!(
            session.represented_summoned_battle_pet_guid_like_cpp(),
            Some(pet_guid)
        );
        assert!(send_rx.try_recv().is_err());

        session
            .handle_battle_pet_summon(battle_pet_summon_packet(pet_guid))
            .await;
        assert_eq!(
            session.represented_summoned_battle_pet_guid_like_cpp(),
            None
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_summon_ignores_unknown_pet_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x229);

        session
            .handle_battle_pet_summon(battle_pet_summon_packet(pet_guid))
            .await;

        assert_eq!(
            session.represented_summoned_battle_pet_guid_like_cpp(),
            None
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_update_notify_updates_known_active_pet_silently_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x22a);
        session.add_represented_battle_pet_like_cpp(
            pet_guid,
            0,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        );
        assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

        session
            .handle_battle_pet_update_notify(battle_pet_update_notify_packet(pet_guid))
            .await;

        assert_eq!(
            session.represented_battle_pet_data_updates_like_cpp(),
            &[pet_guid]
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_update_notify_ignores_inactive_or_unknown_pet_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x22b);
        let unknown_guid = ObjectGuid::new(0, 0x22c);
        session.add_represented_battle_pet_like_cpp(
            pet_guid,
            0,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        );

        session
            .handle_battle_pet_update_notify(battle_pet_update_notify_packet(pet_guid))
            .await;
        session
            .handle_battle_pet_update_notify(battle_pet_update_notify_packet(unknown_guid))
            .await;

        assert_eq!(session.represented_battle_pet_data_updates_like_cpp(), &[]);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_update_display_notify_is_explicit_noop_like_cpp() {
        let (mut session, send_rx) = make_session();
        session
            .handle_battle_pet_update_display_notify(battle_pet_update_display_notify_packet())
            .await;

        assert_eq!(session.represented_battle_pet_data_updates_like_cpp(), &[]);
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn battle_pet_update_display_notify_handler_metadata_like_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::BattlePetUpdateDisplayNotify)
            .expect("BattlePetUpdateDisplayNotify handler entry");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
        assert_eq!(
            entry.handler_name,
            "handle_battle_pet_update_display_notify"
        );
    }

    #[tokio::test]
    async fn query_battle_pet_name_sends_negative_response_like_cpp_until_runtime_exists() {
        let (mut session, send_rx) = make_session();
        let battle_pet_id = ObjectGuid::new(0, 0x22d);
        let unit_guid = ObjectGuid::new(0, 0x22e);

        session
            .handle_query_battle_pet_name(query_battle_pet_name_packet(battle_pet_id, unit_guid))
            .await;

        let bytes = send_rx.try_recv().expect("query battle pet name response");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::QueryBattlePetNameResponse as u16
        );
        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
        assert_eq!(body.read_int32().unwrap(), 0);
        assert_eq!(body.read_int64().unwrap(), 0);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.remaining(), 0);
    }

    #[tokio::test]
    async fn collection_item_set_favorite_ignores_temporary_or_unknown_appearance_like_cpp() {
        let (mut session, send_rx) = make_session();
        session
            .represented_temporary_item_appearances_like_cpp
            .insert(65, HashSet::from([ObjectGuid::create_item(1, 900)]));

        session
            .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
                COLLECTION_TYPE_APPEARANCE_LIKE_CPP,
                65,
                true,
            ))
            .await;
        session
            .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
                COLLECTION_TYPE_APPEARANCE_LIKE_CPP,
                96,
                true,
            ))
            .await;

        assert!(
            session
                .represented_favorite_item_appearance_state_like_cpp(65)
                .is_none()
        );
        assert!(
            session
                .represented_favorite_item_appearance_state_like_cpp(96)
                .is_none()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn request_forced_reactions_sends_cpp_packet_like_cpp() {
        let (mut session, send_rx) = make_session();
        session
            .reputation_mgr_like_cpp_mut()
            .apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Hostile, true);

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::RequestForcedReactions as u16);
        session.handle_request_forced_reactions(pkt).await;

        let bytes = send_rx.try_recv().expect("forced reactions packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::SetForcedReactions as u16
        );
        assert_eq!(&bytes[2..6], &1u32.to_le_bytes());
        assert_eq!(&bytes[6..10], &72i32.to_le_bytes());
        assert_eq!(
            &bytes[10..14],
            &(ReputationRankLikeCpp::Hostile.as_u8() as i32).to_le_bytes()
        );
    }

    #[tokio::test]
    async fn set_faction_at_war_handlers_mark_reputation_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
        let mut faction = FactionEntry::for_test_like_cpp(72, 4);
        faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
        session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));

        let mut at_war = WorldPacket::new_empty();
        at_war.write_uint16(ClientOpcodes::SetFactionAtWar as u16);
        at_war.write_uint16(4);
        session.handle_set_faction_at_war(at_war).await;

        let state = session
            .reputation_mgr_like_cpp()
            .get_state(4)
            .expect("reputation state");
        assert!(state.flags.contains(ReputationFlagsLikeCpp::AT_WAR));
        assert!(state.need_send);
        assert!(state.need_save);
        assert!(send_rx.try_recv().is_err());

        let mut not_at_war = WorldPacket::new_empty();
        not_at_war.write_uint16(ClientOpcodes::SetFactionNotAtWar as u16);
        not_at_war.write_uint16(4);
        session.handle_set_faction_not_at_war(not_at_war).await;

        let state = session
            .reputation_mgr_like_cpp()
            .get_state(4)
            .expect("reputation state");
        assert!(!state.flags.contains(ReputationFlagsLikeCpp::AT_WAR));
        assert!(state.need_send);
        assert!(state.need_save);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_faction_inactive_marks_visible_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        session
            .reputation_mgr_like_cpp_mut()
            .insert_state_for_test_like_cpp(
                crate::reputation::mgr::FactionStateLikeCpp::new_like_cpp(
                    72,
                    4,
                    ReputationFlagsLikeCpp::VISIBLE,
                ),
            );

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::SetFactionInactive as u16);
        pkt.write_uint32(4);
        pkt.write_bit(true);
        pkt.flush_bits();
        session.handle_set_faction_inactive(pkt).await;

        let state = session
            .reputation_mgr_like_cpp()
            .get_state(4)
            .expect("reputation state");
        assert!(state.flags.contains(ReputationFlagsLikeCpp::INACTIVE));
        assert!(state.need_send);
        assert!(state.need_save);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_watched_faction_records_active_player_index_like_cpp() {
        let (mut session, send_rx) = make_session();

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::SetWatchedFaction as u16);
        pkt.write_uint32(42);
        session.handle_set_watched_faction(pkt).await;

        assert_eq!(session.watched_faction_index_like_cpp(), 42);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn request_rated_pvp_info_sends_empty_cpp_default_packet() {
        let (mut session, send_rx) = make_session();

        session
            .handle_request_rated_pvp_info(WorldPacket::new_empty())
            .await;

        let bytes = send_rx.try_recv().expect("rated pvp info packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::RatedPvpInfo as u16
        );
        assert_eq!(
            bytes.len(),
            2 + wow_packet::packets::misc::RATED_PVP_BRACKET_COUNT_LIKE_CPP * (19 * 4 + 1)
        );
    }

    #[tokio::test]
    async fn reset_instances_handler_resets_player_lock_and_sends_cpp_success_packet() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let entries = wow_instances::MapDb2Entries {
            map_id: 631,
            difficulty_id: 4,
            lock_id: 10,
            reset_interval: wow_instances::MapDifficultyResetInterval::Weekly,
            is_flex_locking: true,
            is_using_encounter_locks: false,
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut mgr = wow_instances::InstanceLockMgr::default();
        mgr.update_instance_lock_for_player_at(
            player_guid,
            &entries,
            wow_instances::InstanceLockUpdateEvent {
                instance_id: 100,
                new_data: String::new(),
                instance_completed_encounters_mask: 0,
                completed_encounter_bit: None,
                entrance_world_safe_loc_id: None,
            },
            wow_instances::ResetSchedule::default(),
            now,
        );

        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(0, Position::ZERO);
        session.set_map_store(Arc::new(MapStore::from_entries([
            MapEntry {
                id: 0,
                instance_type: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
            },
            MapEntry {
                id: 631,
                instance_type: 2,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
            },
        ])));
        session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
            MapDifficultyEntry {
                id: 1,
                map_id: 631,
                difficulty_id: 4,
                lock_id: 10,
                reset_interval: 2,
                flags: 0,
            },
        ])));
        let mgr = Arc::new(std::sync::RwLock::new(mgr));
        session.set_instance_lock_mgr(Arc::clone(&mgr));

        session
            .handle_reset_instances(WorldPacket::from_bytes(&[]))
            .await;

        let sent = send_rx.try_recv().unwrap();
        assert_eq!(
            u16::from_le_bytes([sent[0], sent[1]]),
            ServerOpcodes::InstanceReset as u16
        );
        assert_eq!(&sent[2..], &[0x77, 0x02, 0x00, 0x00]);
        assert!(
            mgr.read()
                .unwrap()
                .find_active_instance_lock_at(player_guid, &entries, now)
                .is_none()
        );
    }

    #[test]
    fn send_pending_raid_lock_sets_pending_bind_like_cpp_for_stop_prompt() {
        let (mut session, send_rx) = make_session();

        session.send_pending_raid_lock_like_cpp(77, 0xA5, true, false);

        let sent = send_rx.try_recv().unwrap();
        assert_eq!(
            u16::from_le_bytes([sent[0], sent[1]]),
            ServerOpcodes::PendingRaidLock as u16
        );
        assert_eq!(
            session.pending_bind,
            Some(crate::session::RepresentedPendingBind {
                instance_id: 77,
                time_until_lock_ms: 60_000,
            })
        );
    }

    #[test]
    fn send_pending_raid_lock_warning_only_does_not_set_pending_bind_like_cpp() {
        let (mut session, _send_rx) = make_session();

        session.send_pending_raid_lock_like_cpp(77, 0xA5, false, true);

        assert!(session.pending_bind.is_none());
    }

    #[tokio::test]
    async fn instance_lock_response_accept_confirms_and_clears_pending_bind_like_cpp() {
        let (mut session, _send_rx) = make_session();
        session.pending_bind = Some(crate::session::RepresentedPendingBind {
            instance_id: 77,
            time_until_lock_ms: 60_000,
        });

        session
            .handle_instance_lock_response(WorldPacket::from_bytes(&[0x80]))
            .await;

        assert!(session.pending_bind.is_none());
        assert_eq!(session.represented_confirmed_pending_binds, vec![77]);
        assert_eq!(session.represented_repop_at_graveyard_count, 0);
    }

    #[tokio::test]
    async fn instance_lock_response_decline_repops_and_clears_pending_bind_like_cpp() {
        let (mut session, _send_rx) = make_session();
        session.pending_bind = Some(crate::session::RepresentedPendingBind {
            instance_id: 77,
            time_until_lock_ms: 60_000,
        });

        session
            .handle_instance_lock_response(WorldPacket::from_bytes(&[0x00]))
            .await;

        assert!(session.pending_bind.is_none());
        assert!(session.represented_confirmed_pending_binds.is_empty());
        assert_eq!(session.represented_repop_at_graveyard_count, 1);
    }
}

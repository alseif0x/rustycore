// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for miscellaneous client opcodes:
//! SetSelection, AreaTrigger, RequestCemeteryList,
//! TaxiNodeStatusQuery, ChatJoinChannel.

use tracing::{debug, info, warn};
use wow_constants::{
    ClientOpcodes, InventoryResult, ItemExtendedCostFlags, SpellCastResult, UnitStandStateType,
};
use wow_core::{GameTime, ObjectGuid};
use wow_database::{
    CharStatements, PreparedStatement, SqlTransaction, StatementDef, WorldStatements,
};
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
use wow_packet::packets::character::SetTitle;
use wow_packet::packets::chat::{
    ChannelCommand, ChannelNotify, ChannelPassword, ChannelPlayerCommand, JoinChannel,
    LeaveChannel, MAX_CHANNEL_NAME_STR_LIKE_CPP, MAX_CHANNEL_PASS_STR_LIKE_CPP,
};
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
    AcceptGuildInvite, AcceptTrade, AcceptWargameInvite, ActivateTaxi, ActivateTaxiReply, AddToy,
    AddonList, ArenaTeamDecline, ArenaTeamRoster, BattlePetClearFanfare, BattlePetDeletePet,
    BattlePetModifyName, BattlePetRequestJournal, BattlePetSetBattleSlot, BattlePetSetFlags,
    BattlePetSummon, BattlePetUpdateNotify, BattlefieldLeave, BeginTrade, BugReport, BusyTrade,
    CageBattlePet, CalendarSendCalendar, CalendarSendNumPending, CanDuel, ClearTradeItem,
    CloseInteraction, CommerceTokenGetLog, CommerceTokenGetLogResponse, Complaint, ComplaintResult,
    DeclineGuildInvites, DeclinePetition, DfGetJoinStatus, DfGetSystemInfo,
    ERR_TAXITOOFARAWAY_LIKE_CPP, FarSight, GmTicketAcknowledgeSurvey, GmTicketCaseStatus,
    GmTicketSystemStatus, GuildSetAchievementTracking, IgnoreTrade, LfgListBlacklist,
    LfgPlayerInfo, LfgUpdateStatus, LoadingScreenNotify, MAX_ACCOUNT_DATA_SIZE_LIKE_CPP,
    MountSetFavorite, MountSpecial, NUM_ACCOUNT_DATA_TYPES, ObjectUpdateFailed,
    ObjectUpdateRescued, QueryBattlePetName, QueryBattlePetNameResponse, QueryPetition,
    QueryPetitionResponse, RatedPvpInfo, RequestAccountData, RequestBattlefieldStatus,
    RequestCemeteryListResponse, ResurrectResponse, SaveCufProfiles, SetAdvancedCombatLogging,
    SetCurrencyFlags, SetTaxiBenchmarkMode, SetTradeGold, SetTradeItem, SetTradeSpell,
    SignPetition, SpecialMountAnim, StandStateChange, SubmitUserFeedback, SupportTicketSubmitBug,
    SupportTicketSubmitComplaint, SupportTicketSubmitSuggestion, TRADE_STATUS_CANCELLED_LIKE_CPP,
    TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP, TaxiNodeStatusPkt, TogglePvp, ToyClearFanfare,
    UnacceptTrade, UpdateAccountData, UseToy, UserClientUpdateAccountData, ViolenceLevel,
    compress_account_data_like_cpp, decompress_account_data_like_cpp,
};
use wow_packet::packets::reputation::{
    RequestForcedReactions, SetFactionAtWarRequest, SetFactionInactive, SetFactionNotAtWarRequest,
    SetWatchedFaction,
};
use wow_packet::packets::spell::{
    CastFailed, SetActionButton, SpellCastVisual, SpellPreparePkt, SpellStartPkt,
};
use wow_packet::{ClientPacket, ServerPacket};

use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::handlers::loot::represented_gameobject_interaction_distance_like_cpp;
use crate::session::{
    CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP, RepresentedActivateTaxiLikeCpp,
    RepresentedGameObjectAccessLikeCpp, RepresentedGameObjectUseEffect, SpellCastMetadata,
    TRADE_STATUS_PLAYER_BUSY_LIKE_CPP,
};

// ── inventory registrations ───────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ActivateTaxi,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_activate_taxi",
    }
}

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
        opcode: ClientOpcodes::ResurrectResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_resurrect_response",
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
        opcode: ClientOpcodes::StandStateChange,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_stand_state_change",
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
        opcode: ClientOpcodes::ChatLeaveChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_leave_channel",
    }
}

macro_rules! register_chat_channel_command_handler {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadUnsafe,
                handler_name: "handle_chat_channel_command",
            }
        }
    };
}

register_chat_channel_command_handler!(ChatChannelAnnouncements);
register_chat_channel_command_handler!(ChatChannelDeclineInvite);
register_chat_channel_command_handler!(ChatChannelDisplayList);
register_chat_channel_command_handler!(ChatChannelList);
register_chat_channel_command_handler!(ChatChannelOwner);

macro_rules! register_chat_channel_player_command_handler {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadUnsafe,
                handler_name: "handle_chat_channel_player_command",
            }
        }
    };
}

register_chat_channel_player_command_handler!(ChatChannelBan);
register_chat_channel_player_command_handler!(ChatChannelInvite);
register_chat_channel_player_command_handler!(ChatChannelKick);
register_chat_channel_player_command_handler!(ChatChannelModerator);
register_chat_channel_player_command_handler!(ChatChannelSetOwner);
register_chat_channel_player_command_handler!(ChatChannelSilenceAll);
register_chat_channel_player_command_handler!(ChatChannelUnban);
register_chat_channel_player_command_handler!(ChatChannelUnmoderator);
register_chat_channel_player_command_handler!(ChatChannelUnsilenceAll);

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatChannelPassword,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_password",
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
        opcode: ClientOpcodes::MountSpecialAnim,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_mount_special_anim",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JoinChannelPrecheckLikeCpp {
    Continue,
    InvalidName,
    PasswordTooLong,
}

fn join_channel_custom_precheck_like_cpp(request: &JoinChannel) -> JoinChannelPrecheckLikeCpp {
    if request.chat_channel_id != 0 {
        return JoinChannelPrecheckLikeCpp::Continue;
    }

    if request
        .channel_name
        .chars()
        .next()
        .is_none_or(|first| first.is_ascii_digit())
    {
        return JoinChannelPrecheckLikeCpp::InvalidName;
    }

    if request.channel_name.chars().count() > MAX_CHANNEL_NAME_STR_LIKE_CPP {
        return JoinChannelPrecheckLikeCpp::InvalidName;
    }

    if request.password.len() > MAX_CHANNEL_PASS_STR_LIKE_CPP {
        return JoinChannelPrecheckLikeCpp::PasswordTooLong;
    }

    JoinChannelPrecheckLikeCpp::Continue
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
        processing: PacketProcessing::Inplace,
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
        opcode: ClientOpcodes::AddonList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_addon_list",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AddBattlenetFriend,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_add_battlenet_friend",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlenetChallengeResponse,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_unhandled_client_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetInsertItemsLeftToRight,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_insert_items_left_to_right",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SaveAccountDataExport,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestAccountData,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_account_data",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAccountData,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_update_account_data",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeBagSlotFlag,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CloseQuestChoice,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestItemUsability,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPreferredCemetery,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateClientSettings,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_unhandled_client_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DiscardedTimeSyncAcks,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_client_telemetry_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::EngineSurvey,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_client_telemetry_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LatencyReport,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_client_telemetry_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReportServerLag,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_client_telemetry_null_like_cpp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SuspendCommsAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_client_telemetry_null_like_cpp",
    }
}

macro_rules! register_unhandled_threadsafe_null_handler {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::Authed,
                processing: PacketProcessing::ThreadSafe,
                handler_name: "handle_unhandled_client_null_like_cpp",
            }
        }
    };
}

register_unhandled_threadsafe_null_handler!(MoveAddImpulseAck);
register_unhandled_threadsafe_null_handler!(MoveApplyInertiaAck);
register_unhandled_threadsafe_null_handler!(MoveRemoveInertiaAck);
register_unhandled_threadsafe_null_handler!(MoveRemoveMovementForces);
register_unhandled_threadsafe_null_handler!(MoveSeamlessTransferComplete);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFly);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingAddImpulseMaxSpeedAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingAirFrictionAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingBankingRateAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingDoubleJumpVelModAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingGlideStartMinHeightAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingLaunchSpeedCoefficientAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingLiftCoefficientAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingMaxVelAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingOverMaxDecelerationAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingPitchingRateDownAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingPitchingRateUpAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingSurfaceFrictionAck);
register_unhandled_threadsafe_null_handler!(MoveSetAdvFlyingTurnVelocityThresholdAck);

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
        opcode: ClientOpcodes::SetActionButton,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_action_button",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTaxiBenchmarkMode,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_taxi_benchmark_mode",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetAdvancedCombatLogging,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_advanced_combat_logging",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetCurrencyFlags,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_currency_flags",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetAmmo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_ammo",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetGameEventDebugViewState,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_game_event_debug_view_state",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ShowingHelm,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_showing_helm",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ShowingCloak,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_showing_cloak",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTitle,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_title",
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
        opcode: ClientOpcodes::DeclineGuildInvites,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_decline_guild_invites",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildDeclineInvitation,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_decline_invitation",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptGuildInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_guild_invite",
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
        opcode: ClientOpcodes::BattlefieldLeave,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlefield_leave",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptWargameInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_wargame_invite",
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
        opcode: ClientOpcodes::TogglePvp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_toggle_pvp",
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
        opcode: ClientOpcodes::GmTicketGetSystemStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gm_ticket_get_system_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GmTicketAcknowledgeSurvey,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gm_ticket_acknowledge_survey",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Complaint,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_complaint",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SubmitUserFeedback,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_submit_user_feedback",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SupportTicketSubmitBug,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_support_ticket_submit_bug",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SupportTicketSubmitComplaint,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_support_ticket_submit_complaint",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SupportTicketSubmitSuggestion,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_support_ticket_submit_suggestion",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BugReport,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_bug_report",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ObjectUpdateFailed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_object_update_failed",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ObjectUpdateRescued,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_object_update_rescued",
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
        opcode: ClientOpcodes::ArenaTeamDecline,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_decline",
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
        opcode: ClientOpcodes::GetAccountNotifications,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_account_notifications",
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
        opcode: ClientOpcodes::AcceptTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_trade",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ClearTradeItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_clear_trade_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTradeItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_trade_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTradeGold,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_trade_gold",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTradeSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_trade_spell",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SignPetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_sign_petition",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DeclinePetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_decline_petition",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_petition",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UnacceptTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_unaccept_trade",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BusyTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_busy_trade",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BeginTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_begin_trade",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CanDuel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_can_duel",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::IgnoreTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_ignore_trade",
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
        opcode: ClientOpcodes::ReportFrozenWhileLoadingMap,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_report_frozen_while_loading_map",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogStreamingError,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_log_streaming_error",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CompleteCinematic,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_complete_cinematic",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::NextCinematicCamera,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_next_cinematic_camera",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CompleteMovie,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_complete_movie",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutInstant,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_logout_instant",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SpawnTrackingUpdate,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_spawn_tracking_update",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TimeAdjustmentResponse,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_time_adjustment_response",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAreaTriggerVisual,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_area_trigger_visual",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateSpellVisual,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_spell_visual",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UsedFollow,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_used_follow",
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

pub fn bug_report_insert_statement_like_cpp(report: &BugReport) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(CharStatements::INS_BUG_REPORT.sql());
    // C++ parses `Type` but binds Text and DiagInfo to the `(type, content)`
    // SQL columns in that order.
    stmt.set_string(0, report.text.clone());
    stmt.set_string(1, report.diag_info.clone());
    stmt
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

    pub async fn handle_stand_state_change(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match StandStateChange::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "StandStateChange parse failed: {error}"
                );
                return;
            }
        };

        let stand_state = match packet.stand_state {
            state if state == UnitStandStateType::Stand as u32 => UnitStandStateType::Stand,
            state if state == UnitStandStateType::Sit as u32 => UnitStandStateType::Sit,
            state if state == UnitStandStateType::Sleep as u32 => UnitStandStateType::Sleep,
            state if state == UnitStandStateType::Kneel as u32 => UnitStandStateType::Kneel,
            _ => return,
        };

        self.set_player_stand_state_like_cpp(stand_state);
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
        self.process_represented_delayed_resurrection_after_teleport_like_cpp();

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

    /// CMSG_RESURRECT_RESPONSE — answer to a pending resurrection request.
    /// C++ ref: `WorldSession::HandleResurrectResponse`.
    pub async fn handle_resurrect_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let response = match ResurrectResponse::read(&mut pkt) {
            Ok(response) => response,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ResurrectResponse parse failed: {error}"
                );
                return;
            }
        };

        if self.player_is_alive_like_cpp() {
            return;
        }

        if response.response != 0 {
            self.clear_represented_resurrection_request_like_cpp();
            return;
        }

        let Some(request) = self
            .take_represented_resurrection_request_if_requested_by_like_cpp(response.resurrecter)
        else {
            return;
        };

        // C++ teleports to resurrection request location before applying the
        // resurrected state. InstanceScript combat-res charges, aura original
        // caster, and SpawnCorpseBones remain represented gaps.
        self.teleport_to(request.map_id, request.position).await;
        if self.pending_teleport.is_some() {
            self.schedule_represented_resurrection_after_teleport_like_cpp(request);
        } else {
            self.apply_represented_resurrection_health_like_cpp(request.health);
        }
    }

    /// CMSG_ACTIVATE_TAXI.
    ///
    /// C++ resolves `GetNPCIfCanInteractWith(Vendor, UNIT_NPC_FLAG_FLIGHTMASTER)`,
    /// sends `ERR_TAXITOOFARAWAY` when that fails, then checks nearest taxi
    /// node, known taximask nodes, preferred mount display, `TaxiPathGraph`,
    /// and `Player::ActivateTaxiPathTo`.
    ///
    /// Rust currently has represented NPC interaction and mount display filters,
    /// but not `TaxiNodes.db2`, `TaxiPathGraph`, or live MotionMaster taxi
    /// flight. This handler preserves packet/dispatch and the first C++ failure
    /// reply, then records the accepted request for the future taxi runtime.
    pub async fn handle_activate_taxi(&mut self, mut pkt: wow_packet::WorldPacket) {
        let activate = match ActivateTaxi::read(&mut pkt) {
            Ok(activate) => activate,
            Err(error) => {
                warn!("Bad ActivateTaxi: {error}");
                return;
            }
        };

        const NPC_FLAG_FLIGHT_MASTER: u32 = 0x2000;
        let can_interact = self
            .represented_npc_can_interact_with_like_cpp(activate.vendor, NPC_FLAG_FLIGHT_MASTER, 0)
            .is_some()
            || self
                .mutate_world_creature(activate.vendor, |creature| {
                    creature.npc_flags() & NPC_FLAG_FLIGHT_MASTER != 0
                })
                .unwrap_or(false);

        if !can_interact {
            self.send_packet(&ActivateTaxiReply {
                reply: ERR_TAXITOOFARAWAY_LIKE_CPP,
            });
            return;
        }

        let preferred_mount_display = self
            .represented_taxi_usable_mount_displays_like_cpp(activate.flying_mount_id)
            .into_iter()
            .find_map(|display| u32::try_from(display).ok())
            .unwrap_or_default();

        self.record_represented_activate_taxi_like_cpp(RepresentedActivateTaxiLikeCpp {
            vendor: activate.vendor,
            node: activate.node,
            ground_mount_id: activate.ground_mount_id,
            flying_mount_id: activate.flying_mount_id,
            preferred_mount_display,
        });
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
    /// C++ ref: `WorldSession::HandleJoinChannel`.
    pub async fn handle_chat_join_channel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match JoinChannel::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "JoinChannel parse failed: {error}"
                );
                return;
            }
        };

        match join_channel_custom_precheck_like_cpp(&request) {
            JoinChannelPrecheckLikeCpp::Continue => {}
            JoinChannelPrecheckLikeCpp::InvalidName => {
                self.send_packet(&ChannelNotify::invalid_name(request.channel_name));
                return;
            }
            JoinChannelPrecheckLikeCpp::PasswordTooLong => {
                warn!(
                    account = self.account_id,
                    password_len = request.password.len(),
                    max_password_len = MAX_CHANNEL_PASS_STR_LIKE_CPP,
                    "JoinChannel password too long"
                );
                return;
            }
        }

        // ChannelMgr, system-zone channel validation, custom channel creation,
        // password handling, hyperlink kick checks, and system channel validation
        // are not represented yet.
    }

    /// CMSG_CHAT_LEAVE_CHANNEL.
    /// C++ ref: `WorldSession::HandleLeaveChannel`.
    pub async fn handle_chat_leave_channel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match LeaveChannel::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "LeaveChannel parse failed: {error}"
                );
                return;
            }
        };

        if request.channel_name.is_empty() && request.zone_channel_id == 0 {
            return;
        }

        // ChannelMgr/system-channel zone validation and LeaveChannel fanout are not
        // represented yet. With no resolved channel this is silent like C++.
    }

    /// CMSG_CHAT_CHANNEL_{ANNOUNCEMENTS,DECLINE_INVITE,DISPLAY_LIST,LIST,OWNER}.
    /// C++ ref: `WorldSession::HandleChannelCommand`.
    pub async fn handle_chat_channel_command(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ChannelCommand::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ChannelCommand parse failed: {error}"
            );
        }

        // Channel lookup and command execution require ChannelMgr and are not represented
        // yet. Missing channel is silent like C++.
    }

    /// CMSG_CHAT_CHANNEL_* player-targeted commands.
    /// C++ ref: `WorldSession::HandleChannelPlayerCommand`.
    pub async fn handle_chat_channel_player_command(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ChannelPlayerCommand::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ChannelPlayerCommand parse failed: {error}"
                );
                return;
            }
        };

        if request.name.len() >= MAX_CHANNEL_NAME_STR_LIKE_CPP {
            return;
        }

        // normalizePlayerName, ChannelMgr lookup, and the concrete channel action are not
        // represented yet. Missing/invalid channel remains silent like C++.
    }

    /// CMSG_CHAT_CHANNEL_PASSWORD.
    /// C++ ref: `WorldSession::HandleChannelPassword`.
    pub async fn handle_chat_channel_password(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ChannelPassword::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ChannelPassword parse failed: {error}"
                );
                return;
            }
        };

        if request.password.len() > MAX_CHANNEL_PASS_STR_LIKE_CPP {
            return;
        }

        // ChannelMgr lookup and Password() mutation are not represented yet. Missing
        // channel is silent like C++.
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

    /// CMSG_MOUNT_SPECIAL_ANIM — forward the requested mount animation packet.
    ///
    /// C++ ref: `WorldSession::HandleMountSpecialAnimOpcode` copies the
    /// client-provided visual kit ids and sequence variation into
    /// `SMSG_SPECIAL_MOUNT_ANIM`, sets `UnitGUID` to the player, and calls
    /// `SendMessageToSet(..., false)`. C++ `MessageDistDeliverer` still skips
    /// the source player (`player == i_source`) and then applies `HaveAtClient`
    /// for nearby receivers, so Rust queues the packet to other sessions via
    /// the existing `SendIfVisibleLikeCpp` per-session gate.
    pub async fn handle_mount_special_anim(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match MountSpecial::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "MountSpecial parse failed: {error}"
                );
                return;
            }
        };
        let Some(unit_guid) = self.player_guid() else {
            return;
        };

        let packet_bytes = SpecialMountAnim {
            unit_guid,
            spell_visual_kit_ids: request.spell_visual_kit_ids,
            sequence_variation: request.sequence_variation,
        }
        .to_bytes();

        self.send_mount_special_anim_to_visible_set_like_cpp(unit_guid, packet_bytes);
    }

    fn send_mount_special_anim_to_visible_set_like_cpp(
        &self,
        source_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) {
        let Some(registry) = self.player_registry() else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        let candidates: Vec<_> = registry
            .iter()
            .filter_map(|entry| {
                let (target_guid, info) = entry.pair();
                if *target_guid == source_guid {
                    return None;
                }
                if !info.is_in_world || info.map_id != map_id || info.instance_id != instance_id {
                    return None;
                }
                Some(info.command_tx.clone())
            })
            .collect();

        for command_tx in candidates {
            let _ = command_tx.try_send(wow_network::SessionCommand::SendIfVisibleLikeCpp(
                wow_network::player_registry::SendIfVisibleLikeCppCommand {
                    source_guid,
                    map_id,
                    instance_id,
                    packet_bytes: packet_bytes.clone(),
                },
            ));
        }
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
            cast_item_battle_pet_modifiers: None,
            cast_flags_ex: CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP,
            original_cast_id: request.cast.cast_id,
            unit_target_battle_pet_companion_guid: None,
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

    pub async fn handle_loading_screen_notify(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = LoadingScreenNotify::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "LoadingScreenNotify parse failed: {error}"
            );
            return;
        }

        // C++ `HandleLoadScreenOpcode` is a TODO after reading MapID + Showing.
    }
    pub async fn handle_violence_level(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ViolenceLevel::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ViolenceLevel parse failed: {error}"
            );
            return;
        }

        // C++ `HandleViolenceLevel` reads ViolenceLvl and has no observable action.
    }
    pub async fn handle_override_screen_flash(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_OVERRIDE_SCREEN_FLASH as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_queued_messages_end(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_QUEUED_MESSAGES_END as STATUS_LOGGEDIN/Handle_NULL.
    }
    pub async fn handle_chat_unregister_all_addon_prefixes(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        self.registered_addon_prefixes.clear();
    }
    pub async fn handle_set_action_bar_toggles(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mask = match pkt.read_uint8() {
            Ok(mask) => mask,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetActionBarToggles parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_action_bar_toggles_like_cpp(mask);
    }

    pub async fn handle_set_action_button(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetActionButton::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetActionButton parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_action_button_like_cpp(packet.index, packet.action);
    }

    pub async fn handle_set_taxi_benchmark_mode(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetTaxiBenchmarkMode::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetTaxiBenchmarkMode parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_taxi_benchmark_mode_like_cpp(packet.enable);
    }

    pub async fn handle_set_advanced_combat_logging(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetAdvancedCombatLogging::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetAdvancedCombatLogging parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_advanced_combat_logging_like_cpp(packet.enable);
    }

    pub async fn handle_set_currency_flags(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetCurrencyFlags::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetCurrencyFlags parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_currency_flags_like_cpp(packet.currency_id, packet.flags);
    }

    pub async fn handle_request_account_data(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match RequestAccountData::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "RequestAccountData parse failed: {error}"
                );
                return;
            }
        };

        if usize::from(packet.data_type) >= NUM_ACCOUNT_DATA_TYPES {
            return;
        }

        let Some(account_data) = self.account_data_like_cpp(packet.data_type) else {
            return;
        };
        let data = account_data.data.clone();
        let time = account_data.time;
        let compressed_data = match compress_account_data_like_cpp(&data) {
            Ok(compressed_data) => compressed_data,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "RequestAccountData compression failed: {error}"
                );
                return;
            }
        };

        self.send_packet(&UpdateAccountData {
            player_guid: self.player_guid().unwrap_or(ObjectGuid::EMPTY),
            time,
            size: data.len() as u32,
            data_type: packet.data_type,
            compressed_data,
        });
    }

    pub async fn handle_update_account_data(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match UserClientUpdateAccountData::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "UpdateAccountData parse failed: {error}"
                );
                return;
            }
        };

        if usize::from(packet.data_type) >= NUM_ACCOUNT_DATA_TYPES {
            return;
        }

        if packet.size == 0 {
            self.set_account_data_persisted_like_cpp(packet.data_type, 0, String::new())
                .await;
            return;
        }

        if packet.size > MAX_ACCOUNT_DATA_SIZE_LIKE_CPP {
            warn!(
                account = self.account_id,
                data_type = packet.data_type,
                size = packet.size,
                "UpdateAccountData rejected oversized payload like C++"
            );
            return;
        }

        let data = match decompress_account_data_like_cpp(&packet.compressed_data, packet.size) {
            Ok(data) => data,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    data_type = packet.data_type,
                    "UpdateAccountData decompression failed: {error}"
                );
                return;
            }
        };

        self.set_account_data_persisted_like_cpp(packet.data_type, packet.time, data)
            .await;
    }

    pub async fn handle_addon_list(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AddonList::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "AddonList parse failed: {error}");
                return;
            }
        };

        debug!(
            account = self.account_id,
            addon_count = packet.addons.len(),
            "HandleAddonList consumed addon list like C++"
        );
    }

    pub async fn handle_add_battlenet_friend(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_ADD_BATTLENET_FRIEND as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_set_insert_items_left_to_right(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_SET_INSERT_ITEMS_LEFT_TO_RIGHT as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_unhandled_client_null_like_cpp(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers this bounded client packet family as STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_client_telemetry_null_like_cpp(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers this client telemetry/ack family to WorldSession::Handle_NULL.
    }

    pub async fn handle_set_ammo(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleSetAmmoOpcode(WorldPackets::Null&)` only logs the request.
    }

    pub async fn handle_set_game_event_debug_view_state(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleSetGameEventDebugViewState(WorldPackets::Null&)` only logs the request.
    }

    pub async fn handle_showing_helm(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleShowingHelmOpcode(WorldPackets::Null&)` only logs the request.
    }

    pub async fn handle_showing_cloak(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleShowingCloakOpcode(WorldPackets::Null&)` only logs the request.
    }

    pub async fn handle_set_title(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut packet = match SetTitle::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "SetTitle parse failed: {error}");
                return;
            }
        };

        if packet.title_id > 0 {
            if !self.represented_has_title_like_cpp(packet.title_id as u32) {
                return;
            }
        } else {
            packet.title_id = 0;
        }

        self.represented_set_chosen_title_like_cpp(packet.title_id);
        if let Some(update) = self.set_canonical_chosen_title_like_cpp(packet.title_id) {
            if let Some(player_guid) = self.player_guid() {
                if let Some(packet) = player_values_update_to_update_object(
                    player_guid,
                    self.player_map_id_like_cpp(),
                    &update,
                ) {
                    self.send_packet(&packet);
                }
            }
        }
    }

    pub async fn handle_save_cuf_profiles(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SaveCufProfiles::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SaveCufProfiles parse failed: {error}"
                );
                return;
            }
        };

        if !self.represented_save_cuf_profiles_like_cpp(packet.profiles) {
            warn!(
                account = self.account_id,
                max_profiles = wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP,
                "SaveCufProfiles ignored profile count above C++ MAX_CUF_PROFILES"
            );
        }
    }
    pub async fn handle_guild_set_achievement_tracking(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        if let Err(error) = GuildSetAchievementTracking::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "GuildSetAchievementTracking parse failed: {error}"
            );
            return;
        }

        // C++ only delegates when GetPlayer()->GetGuild() resolves a live guild.
        // Rust has no represented guild-achievement manager here yet, so the
        // no-guild branch remains silent.
    }

    pub async fn handle_decline_guild_invites(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match DeclineGuildInvites::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DeclineGuildInvites parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_auto_decline_guild_invites_like_cpp(request.allow);
    }

    pub async fn handle_guild_decline_invitation(&mut self, _pkt: wow_packet::WorldPacket) {
        self.decline_guild_invitation_like_cpp();
    }

    pub async fn handle_accept_guild_invite(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = AcceptGuildInvite::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "AcceptGuildInvite parse failed: {error}"
            );
            return;
        }

        self.accept_guild_invitation_like_cpp();
    }

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

    pub async fn handle_request_battlefield_status(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = RequestBattlefieldStatus::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "RequestBattlefieldStatus parse failed: {error}"
            );
            return;
        }

        // C++ iterates PLAYER_MAX_BATTLEGROUND_QUEUES and sends active,
        // confirmation, or queued status only for non-empty queue slots.
        // Rust has no represented battleground queue state in this handler yet,
        // so the no-queue branch is silent.
    }

    /// CMSG_BATTLEFIELD_LEAVE — player asks to leave the current battleground.
    /// C++ ref: `WorldSession::HandleBattlefieldLeaveOpcode`.
    pub async fn handle_battlefield_leave(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = BattlefieldLeave::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "BattlefieldLeave parse failed: {error}"
            );
            return;
        }

        if self.in_combat
            && self.player_in_represented_battleground_like_cpp()
            && !self.represented_battleground_status_is_wait_leave_like_cpp()
        {
            return;
        }

        self.request_represented_battleground_leave_like_cpp();
    }

    pub async fn handle_accept_wargame_invite(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AcceptWargameInvite::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "AcceptWargameInvite parse failed: {error}"
                );
                return;
            }
        };

        self.accept_represented_wargame_invite_like_cpp(&packet.inviter_name);
    }

    pub async fn handle_request_rated_pvp_info(&mut self, _pkt: wow_packet::WorldPacket) {
        self.send_packet(&RatedPvpInfo::default());
    }
    pub async fn handle_request_pvp_rewards(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ dispatches to Player::SendPvpRewards(), but that method's
        // SMSG_REQUEST_PVP_REWARDS_RESPONSE send is commented out in the
        // canonical source, so the observable behavior is silence.
    }
    pub async fn handle_toggle_pvp(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = TogglePvp::read(&mut pkt) {
            warn!(account = self.account_id, "TogglePvP parse failed: {error}");
            return;
        }

        self.apply_toggle_pvp_like_cpp();
    }
    pub async fn handle_df_get_system_info(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match DfGetSystemInfo::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DFGetSystemInfo parse failed: {error}"
                );
                return;
            }
        };

        if request.player {
            // C++ `SendLfgPlayerLockInfo`: blacklist + random/seasonal dungeon
            // rows from `sLFGMgr`. Until that manager is ported, represent the
            // empty lock/dungeon response.
            self.send_packet(&LfgPlayerInfo::empty());
        } else {
            // C++ `SendLfgPartyLockInfo` returns before sending when the player
            // is not in a group. Rust does not expose a live LFG group manager
            // here yet, so the no-group branch remains silent.
        }
    }
    pub async fn handle_df_get_join_status(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = DfGetJoinStatus::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "DFGetJoinStatus parse failed: {error}"
            );
            return;
        }

        // C++ `HandleDFGetJoinStatus` returns before sending anything when
        // `Player::isUsingLfg()` is false. Rust has no represented active LFG
        // join state in this handler yet, so preserve that observable branch.
    }
    pub async fn handle_calendar_get_num_pending(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ reads `sCalendarMgr->GetPlayerNumPending(playerGuid)` and sends
        // CalendarSendNumPending. Calendar manager state is not ported yet, so
        // represent the empty pending-invite count.
        self.send_packet(&CalendarSendNumPending { num_pending: 0 });
    }
    pub async fn handle_gm_ticket_get_case_status(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleGMTicketGetCaseStatusOpcode` is still a TODO and sends a
        // default `GMTicketCaseStatus`, i.e. an empty case list.
        self.send_packet(&GmTicketCaseStatus::empty());
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

    pub async fn handle_guild_bank_remaining_withdraw_money_query(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        // C++ only sends GuildBankRemainingWithdrawMoney when GetPlayer()->GetGuild()
        // resolves a live guild. Rust has no represented guild-bank manager here
        // yet, so the no-guild branch is correctly silent.
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

    /// CMSG_BATTLE_PET_DELETE_PET — represented battle-pet removal body.
    ///
    /// C++ registers this handler and forwards only the pet guid to
    /// `BattlePetMgr::RemovePet`, which requires the journal lock and silently
    /// ignores unknown pets. The archived opcode id is the unresolved `0xBADD`
    /// placeholder, so this method is intentionally not registered for
    /// production dispatch until the real client opcode is known.
    pub async fn handle_battle_pet_delete_pet_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match BattlePetDeletePet::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetDeletePet parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_remove_pet_like_cpp(request.pet_guid);
    }

    /// CMSG_CAGE_BATTLE_PET — represented cage body.
    ///
    /// C++ registers this handler and forwards only the pet guid to
    /// `BattlePetMgr::CageBattlePet`. The manager then performs the journal,
    /// species, slot, health, inventory, item-store, remove, deleted-packet,
    /// and summoned-companion gates. The archived opcode id is still the
    /// unresolved `0xBADD` placeholder, so this method remains intentionally
    /// unregistered for production dispatch. Until the real inventory path is
    /// wired, this represented body exercises the successful inventory seam.
    pub async fn handle_cage_battle_pet_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match CageBattlePet::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CageBattlePet parse failed: {error}"
                );
                return;
            }
        };

        let _ = self.battle_pet_cage_battle_pet_represented_like_cpp(request.pet_guid, true, true);
    }

    /// CMSG_BATTLE_PET_MODIFY_NAME — represented rename body.
    ///
    /// C++ registers this handler and forwards the parsed guid/name/declined
    /// names to `BattlePetMgr::ModifyName`, which stamps `GameTime::GetGameTime`
    /// inside the manager. The archived opcode id remains the unresolved
    /// `0xBADD` placeholder, so this method is intentionally not registered for
    /// production dispatch until the real client opcode is known.
    pub async fn handle_battle_pet_modify_name_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match BattlePetModifyName::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetModifyName parse failed: {error}"
                );
                return;
            }
        };

        let timestamp = i64::try_from(GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let _ = self.battle_pet_modify_name_like_cpp(
            request.pet_guid,
            request.name,
            request.declined_names,
            timestamp,
        );
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

    /// CMSG_QUERY_BATTLE_PET_NAME — represented summoned-companion name lookup.
    ///
    /// C++ first resolves the requested unit through ObjectAccessor and requires
    /// a summon. Only after that does it copy `CreatureID` and companion-name
    /// timestamp, then it gates on player owner, known battle-pet row, and a
    /// non-empty name before setting `Allow=true`.
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

        let Some(companion) =
            self.represented_battle_pet_query_companion_like_cpp(request.unit_guid)
        else {
            self.send_packet(&QueryBattlePetNameResponse::not_allowed(
                request.battle_pet_id,
            ));
            return;
        };

        if !companion.is_summon {
            self.send_packet(&QueryBattlePetNameResponse::not_allowed(
                request.battle_pet_id,
            ));
            return;
        }

        let mut response = QueryBattlePetNameResponse {
            battle_pet_id: request.battle_pet_id,
            creature_id: companion.creature_id,
            timestamp: companion.name_timestamp,
            allow: false,
            name: String::new(),
            declined_names: None,
        };

        if companion.owner_is_player {
            if let Some(pet) = self.represented_battle_pet_like_cpp(request.battle_pet_id) {
                response.name = pet.name;
                response.declined_names = pet.declined_names;
                response.allow = !response.name.is_empty();
            }
        }

        self.send_packet(&response);
    }
    pub async fn handle_arena_team_roster(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamRoster::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ArenaTeamRoster parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        // The live arena-team manager is not ported here yet, so Rust preserves
        // that unknown-team branch instead of inventing an empty roster packet.
        debug!(
            account = self.account_id,
            team_id = request.team_id,
            "ArenaTeamRoster ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_decline(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ArenaTeamDecline::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ArenaTeamDecline parse failed: {error}"
            );
            return;
        }

        self.set_represented_arena_team_id_invited_like_cpp(0);
    }

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
        // C++ registers CMSG_REQUEST_CONQUEST_FORMULA_CONSTANTS as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_request_lfg_list_blacklist(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ builds this from `sLFGMgr->GetLockedDungeons(playerGuid)`.
        // Rust does not have that manager state yet, so represent the
        // well-defined no-locks response instead of leaving the client waiting.
        self.send_packet(&LfgListBlacklist::empty());
    }
    pub async fn handle_lfg_list_get_status(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleLfgListGetStatus` always sends LFGUpdateStatus for a live
        // player. Until `sLFGMgr` state is ported, Rust represents the
        // well-defined no-ticket/no-queue branch.
        self.send_packet(&LfgUpdateStatus::removed_from_queue());
    }
    pub async fn handle_get_account_character_list(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_GET_ACCOUNT_CHARACTER_LIST as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_get_account_notifications(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_GET_ACCOUNT_NOTIFICATIONS as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_cancel_trade(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ calls Player::TradeCancel(true) for a present player; TradeCancel
        // itself is a no-op when no active TradeData exists.
        self.cancel_represented_trade_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP, true);
    }

    pub async fn handle_accept_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AcceptTrade::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "AcceptTrade parse failed: {error}"
                );
                return;
            }
        };

        self.accept_represented_trade_like_cpp(packet.state_index);
    }

    pub async fn handle_clear_trade_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match ClearTradeItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ClearTradeItem parse failed: {error}"
                );
                return;
            }
        };

        self.clear_represented_trade_item_like_cpp(packet.trade_slot);
    }

    pub async fn handle_set_trade_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetTradeItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetTradeItem parse failed: {error}"
                );
                return;
            }
        };

        self.set_represented_trade_item_like_cpp(
            packet.trade_slot,
            packet.pack_slot,
            packet.item_slot_in_pack,
        );
    }

    pub async fn handle_set_trade_gold(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetTradeGold::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetTradeGold parse failed: {error}"
                );
                return;
            }
        };

        self.set_represented_trade_gold_like_cpp(packet.coinage);
    }

    pub async fn handle_set_trade_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetTradeSpell::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetTradeSpell parse failed: {error}"
                );
                return;
            }
        };

        self.set_represented_trade_spell_like_cpp(
            packet.spell_id,
            packet.pack_slot,
            packet.item_slot_in_pack,
        );
    }

    pub async fn handle_sign_petition(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SignPetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SignPetition parse failed: {error}"
                );
                return;
            }
        };

        self.record_represented_sign_petition_like_cpp(packet.petition_guid, packet.choice);
    }

    pub async fn handle_decline_petition(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match DeclinePetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DeclinePetition parse failed: {error}"
                );
                return;
            }
        };

        self.record_represented_decline_petition_like_cpp(packet.petition_guid);
    }

    pub async fn handle_query_petition(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match QueryPetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "QueryPetition parse failed: {error}"
                );
                return;
            }
        };

        self.record_represented_query_petition_like_cpp(packet.petition_id, packet.item_guid);
        self.send_packet(&QueryPetitionResponse::not_found_like_cpp(packet.item_guid));
    }

    pub async fn handle_unaccept_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = UnacceptTrade::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "UnacceptTrade parse failed: {error}"
            );
            return;
        }

        self.unaccept_represented_trade_like_cpp();
    }

    pub async fn handle_busy_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = BusyTrade::read(&mut pkt) {
            warn!(account = self.account_id, "BusyTrade parse failed: {error}");
            return;
        }

        self.cancel_represented_trade_like_cpp(TRADE_STATUS_PLAYER_BUSY_LIKE_CPP, true);
    }

    pub async fn handle_begin_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = BeginTrade::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "BeginTrade parse failed: {error}"
            );
            return;
        }

        self.begin_represented_trade_like_cpp();
    }

    pub async fn handle_can_duel(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match CanDuel::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "CanDuel parse failed: {error}");
                return;
            }
        };

        self.handle_can_duel_like_cpp(packet.target_guid, packet.to_the_death);
    }

    pub async fn handle_ignore_trade(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = IgnoreTrade::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "IgnoreTrade parse failed: {error}"
            );
            return;
        }

        self.cancel_represented_trade_like_cpp(TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP, true);
    }

    pub async fn handle_report_client_variables(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_REPORT_CLIENT_VARIABLES as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_report_enabled_addons(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_REPORT_ENABLED_ADDONS as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_report_frozen_while_loading_map(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_REPORT_FROZEN_WHILE_LOADING_MAP as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_log_streaming_error(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_LOG_STREAMING_ERROR as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_complete_cinematic(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ CinematicMgr::EndCinematic also clears sight binding when the
        // player is bound to a visual waypoint NPC. Rust records the represented
        // end event until the live CinematicMgr/vision runtime is ported.
        self.complete_represented_cinematic_like_cpp();
    }
    pub async fn handle_next_cinematic_camera(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ CinematicMgr::NextCinematicCamera advances the active camera
        // index and may spawn a visual waypoint for remote sight. Rust records
        // the represented camera advance until fly-by camera/TempSummon/viewpoint
        // runtime is ported.
        self.next_represented_cinematic_camera_like_cpp();
    }
    pub async fn handle_complete_movie(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ Player::GetMovie() == 0 returns early; otherwise SetMovie(0)
        // and ScriptMgr::OnMovieComplete(player, movie). Rust records the
        // script hook until the live ScriptMgr runtime is ported.
        self.complete_represented_movie_like_cpp();
    }
    pub async fn handle_logout_instant(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_LOGOUT_INSTANT as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_spawn_tracking_update(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_SPAWN_TRACKING_UPDATE as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_time_adjustment_response(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_TIME_ADJUSTMENT_RESPONSE as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_update_area_trigger_visual(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_UPDATE_AREA_TRIGGER_VISUAL as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_update_spell_visual(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_UPDATE_SPELL_VISUAL as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_used_follow(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_USED_FOLLOW as STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_report_keybinding_execution_counts(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        // C++ registers CMSG_REPORT_KEYBINDING_EXECUTION_COUNTS as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_request_countdown_timer(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ registers CMSG_QUERY_COUNTDOWN_TIMER as
        // STATUS_UNHANDLED/Handle_NULL.
    }
    pub async fn handle_calendar_get(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ fills CalendarSendCalendar from sCalendarMgr and instance locks.
        // Those live managers are not ported here yet, so represent the
        // well-defined empty calendar/lockout lists with current server time.
        self.send_packet(&CalendarSendCalendar::empty_now());
    }

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
    pub async fn handle_commerce_token_get_log(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CommerceTokenGetLog::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CommerceTokenGetLog parse failed: {error}"
                );
                return;
            }
        };

        // C++ has a TODO here and returns TOKEN_RESULT_SUCCESS with an empty
        // auctionable-token list while echoing the request integer.
        self.send_packet(&CommerceTokenGetLogResponse::success_empty(request.unk_int));
    }

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
    /// C++ ref: `WorldSession::HandleCloseInteraction`.
    pub async fn handle_close_interaction(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CloseInteraction::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CloseInteraction parse failed: {error}"
                );
                return;
            }
        };

        if self.gossip_source_guid == Some(request.source_guid) {
            self.gossip_source_guid = None;
            self.gossip_options.clear();
        }

        // C++ also clears Player::StableMaster when it matches SourceGuid. Rust
        // does not expose represented stable-master state yet.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};
    use wow_constants::{ClientOpcodes, ItemContext, ServerOpcodes};
    use wow_core::{ObjectGuid, Position, guid::HighGuid};
    use wow_data::progression_rewards::{FactionEntry, FactionStore};
    use wow_data::reputation::{ReputationFlagsLikeCpp, ReputationRankLikeCpp};
    use wow_data::{
        ItemRecord, ItemSearchNameEntry, ItemSearchNameStore, ItemSparseTemplateEntry,
        ItemStatsStore, ItemStore, MapDifficultyEntry, MapDifficultyStore, MapEntry, MapStore,
        SpellInfo, SpellStore,
    };
    use wow_database::SqlParam;
    use wow_network::{
        GroupInfo, GroupRegistry, PendingInvites, PlayerBroadcastInfo, PlayerRegistry,
        SessionCommand,
    };
    use wow_packet::ServerPacket;
    use wow_packet::WorldPacket;
    use wow_packet::packets::misc::TRADE_STATUS_INITIATED_LIKE_CPP;
    use wow_packet::packets::misc::{
        EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, TRADE_STATUS_FAILED_LIKE_CPP,
    };
    use wow_packet::packets::misc::{
        SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP, empty_battle_pet_guid_like_cpp,
    };
    use wow_packet::packets::misc::{
        TRADE_STATUS_ACCEPTED_LIKE_CPP, TRADE_STATUS_STATE_CHANGED_LIKE_CPP,
        TRADE_STATUS_UNACCEPTED_LIKE_CPP,
    };

    fn currency_entry(id: u32) -> wow_data::CurrencyTypesEntry {
        wow_data::CurrencyTypesEntry {
            id,
            category_id: 0,
            inventory_icon_file_id: 0,
            spell_weight: 0,
            spell_category: 0,
            max_qty: 0,
            max_earnable_per_week: 0,
            quality: 0,
            faction_id: 0,
            award_condition_id: 0,
            flags: wow_constants::CurrencyTypesFlags::empty(),
            flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
        }
    }

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

    fn broadcast_info_with_command_tx(
        command_tx: flume::Sender<SessionCommand>,
    ) -> PlayerBroadcastInfo {
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(4);
        PlayerBroadcastInfo {
            map_id: 571,
            instance_id: 0,
            position: Position::ZERO,
            combat_reach: 0.0,
            liquid_status: 0,
            is_in_world: true,
            send_tx,
            command_tx,
            active_loot_rolls: Vec::new(),
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            current_health: 100,
            max_health: 100,
            power_type: 0,
            current_power: 0,
            max_power: 0,
            is_pvp: false,
            is_ffa_pvp: false,
            is_ghost: false,
            is_afk: false,
            is_dnd: false,
            auto_reply_msg_like_cpp: String::new(),
            in_vehicle: false,
            has_vehicle_kit_like_cpp: false,
            party_member_vehicle_seat: 0,
            zone_id: 0,
            spec_id: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_state: 0,
            is_game_master: false,
            is_contested_pvp: false,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            faction_template_id: 0,
            reputation_standings: Vec::new(),
            reputation_state_flags: Vec::new(),
            forced_reputation_ranks: Vec::new(),
            forced_reputation_faction_ids: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_party_type: [0; 2],
            party_member_phase_states: Default::default(),
            party_member_auras: Vec::new(),
            party_member_pet_stats: None,
            player_name: "TestPlayer".to_string(),
            account_id: 1,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            level: 1,
            gray_level: 0,
            display_id: 49,
            visible_items: [(0, 0, 0); 19],
            lifetime_honorable_kills: 0,
            this_week_contribution: 0,
            yesterday_contribution: 0,
            today_honorable_kills: 0,
            yesterday_honorable_kills: 0,
            lifetime_max_rank: 0,
            honor_level: 0,
        }
    }

    fn resurrect_response_packet(resurrecter: ObjectGuid, response: u32) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&resurrecter);
        pkt.write_uint32(response);
        pkt
    }

    fn update_account_data_packet(
        player_guid: ObjectGuid,
        data_type: u8,
        time: i64,
        data: &str,
    ) -> WorldPacket {
        let compressed_data = compress_account_data_like_cpp(data).unwrap();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&player_guid);
        pkt.write_int64(time);
        pkt.write_uint32(data.len() as u32);
        pkt.write_bits(u32::from(data_type), 4);
        pkt.write_uint32(compressed_data.len() as u32);
        pkt.write_bytes(&compressed_data);
        pkt
    }

    fn request_account_data_packet(player_guid: ObjectGuid, data_type: u8) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&player_guid);
        pkt.write_bits(u32::from(data_type), 4);
        pkt.flush_bits();
        pkt
    }

    fn activate_taxi_packet(
        vendor: ObjectGuid,
        node: u32,
        ground_mount_id: u32,
        flying_mount_id: u32,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&vendor);
        pkt.write_uint32(node);
        pkt.write_uint32(ground_mount_id);
        pkt.write_uint32(flying_mount_id);
        pkt
    }

    fn add_canonical_flight_master_for_misc_test(
        canonical: &crate::session::SharedCanonicalMapManager,
        guid: ObjectGuid,
        position: Position,
    ) {
        let mut creature = wow_entities::Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(90_001);
        creature.unit_mut().world_mut().set_map(571, 0).unwrap();
        creature.unit_mut().world_mut().relocate(position);
        creature.unit_mut().world_mut().set_combat_reach(1.0);
        creature.unit_mut().set_level(80);
        creature.unit_mut().set_max_health(100);
        creature.unit_mut().set_health(100);
        creature.unit_mut().world_mut().object_mut().add_to_world();
        creature.set_ai_identity_runtime(1, 35, 0x2000, 0);

        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
            )
            .unwrap();
    }

    #[tokio::test]
    async fn update_account_data_stores_decompressed_cstring_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(player_guid));

        session
            .handle_update_account_data(update_account_data_packet(
                player_guid,
                4,
                1234,
                "macros-cache",
            ))
            .await;

        let account_data = session.account_data_like_cpp(4).unwrap();
        assert_eq!(account_data.time, 1234);
        assert_eq!(account_data.data, "macros-cache");
    }

    #[tokio::test]
    async fn update_account_data_size_zero_erases_like_cpp() {
        let (mut session, _send_rx) = make_session();
        assert!(session.set_account_data_like_cpp(4, 999, "old-cache".to_string()));
        let player_guid = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&player_guid);
        pkt.write_int64(1234);
        pkt.write_uint32(0);
        pkt.write_bits(4, 4);
        pkt.write_uint32(0);

        session.handle_update_account_data(pkt).await;

        let account_data = session.account_data_like_cpp(4).unwrap();
        assert_eq!(account_data.time, 0);
        assert!(account_data.data.is_empty());
    }

    #[tokio::test]
    async fn update_account_data_ignores_per_character_data_without_player_guid_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);

        session
            .handle_update_account_data(update_account_data_packet(
                player_guid,
                1,
                1234,
                "layout-cache",
            ))
            .await;

        let account_data = session.account_data_like_cpp(1).unwrap();
        assert_eq!(account_data.time, 0);
        assert!(account_data.data.is_empty());
    }

    #[tokio::test]
    async fn update_account_data_rejects_invalid_type_and_oversize_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);

        session
            .handle_update_account_data(update_account_data_packet(
                player_guid,
                NUM_ACCOUNT_DATA_TYPES as u8,
                1234,
                "ignored",
            ))
            .await;
        assert!(session.account_data_like_cpp(0).unwrap().data.is_empty());

        let compressed_data = compress_account_data_like_cpp("ignored").unwrap();
        let mut oversized = WorldPacket::new_empty();
        oversized.write_packed_guid(&player_guid);
        oversized.write_int64(1234);
        oversized.write_uint32(MAX_ACCOUNT_DATA_SIZE_LIKE_CPP + 1);
        oversized.write_bits(4, 4);
        oversized.write_uint32(compressed_data.len() as u32);
        oversized.write_bytes(&compressed_data);

        session.handle_update_account_data(oversized).await;
        assert!(session.account_data_like_cpp(4).unwrap().data.is_empty());
    }

    #[tokio::test]
    async fn request_account_data_sends_update_account_data_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(player_guid));
        assert!(session.set_account_data_like_cpp(4, 5678, "macro-cache".to_string()));

        session
            .handle_request_account_data(request_account_data_packet(player_guid, 4))
            .await;

        let encoded = send_rx.try_recv().unwrap();
        let mut packet = WorldPacket::new_client(encoded.as_slice().into());
        assert_eq!(
            packet.server_opcode(),
            Some(wow_constants::ServerOpcodes::UpdateAccountData)
        );
        packet.skip_opcode();
        assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
        assert_eq!(packet.read_int64().unwrap(), 5678);
        let decompressed_size = packet.read_uint32().unwrap();
        assert_eq!(decompressed_size, "macro-cache".len() as u32);
        assert_eq!(packet.read_bits(4).unwrap(), 4);
        let compressed_size = packet.read_uint32().unwrap() as usize;
        let compressed_data = packet.read_bytes(compressed_size).unwrap();
        assert_eq!(
            decompress_account_data_like_cpp(&compressed_data, decompressed_size).unwrap(),
            "macro-cache"
        );
        assert_eq!(packet.remaining(), 0);
    }

    #[tokio::test]
    async fn activate_taxi_without_interactable_flight_master_replies_too_far_like_cpp() {
        let (mut session, send_rx) = make_session();
        let vendor = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9, 77);

        session
            .handle_activate_taxi(activate_taxi_packet(vendor, 12, 101, 202))
            .await;

        let encoded = send_rx.try_recv().unwrap();
        let mut packet = WorldPacket::new_client(encoded.as_slice().into());
        assert_eq!(
            packet.server_opcode(),
            Some(ServerOpcodes::ActivateTaxiReply)
        );
        packet.skip_opcode();
        assert_eq!(
            packet.read_bits(4).unwrap(),
            u32::from(ERR_TAXITOOFARAWAY_LIKE_CPP)
        );
        assert_eq!(packet.remaining(), 0);
        assert!(
            session
                .represented_activate_taxi_requests_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn activate_taxi_records_represented_request_for_flight_master_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        let player_guid = ObjectGuid::create_player(1, 42);
        let vendor = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9, 77);
        let position = Position::new(10.0, 0.0, 0.0, 0.0);

        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_player_alive_like_cpp(true);
        session.set_player_faction_template_like_cpp(35);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
            player_guid,
            "TaxiTester".to_string(),
            position,
            571,
            1,
            1,
            80,
            0,
        ));
        add_canonical_test_player_on_map_for_misc_test(&canonical, player_guid, position, 571, 0);
        add_canonical_flight_master_for_misc_test(&canonical, vendor, position);

        session
            .handle_activate_taxi(activate_taxi_packet(vendor, 12, 101, 202))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            session.represented_activate_taxi_requests_like_cpp(),
            &[RepresentedActivateTaxiLikeCpp {
                vendor,
                node: 12,
                ground_mount_id: 101,
                flying_mount_id: 202,
                preferred_mount_display: 0,
            }]
        );
    }

    #[tokio::test]
    async fn resurrect_response_accepts_matching_request_like_cpp() {
        let (mut session, send_rx) = make_session();
        let resurrecter = ObjectGuid::create_player(1, 77);
        let target_position = Position::new(11.0, 22.0, 33.0, 1.5);
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
        session.set_player_health_like_cpp(0, 1_000);
        session.set_represented_resurrection_request_like_cpp(
            crate::session::RepresentedResurrectionRequestLikeCpp {
                resurrecter,
                map_id: 571,
                position: target_position,
                health: 450,
                mana: 120,
                aura: 0,
            },
        );

        session
            .handle_resurrect_response(resurrect_response_packet(resurrecter, 0))
            .await;

        assert!(!session.player_is_alive_like_cpp());
        assert!(
            session
                .represented_resurrection_request_like_cpp()
                .is_none()
        );
        assert!(
            session
                .represented_delayed_resurrection_after_teleport_like_cpp()
                .is_some()
        );
        let first = send_rx.try_recv().expect("transfer pending packet");
        assert_eq!(
            u16::from_le_bytes([first[0], first[1]]),
            ServerOpcodes::TransferPending as u16
        );

        session
            .handle_world_port_response(WorldPacket::new_empty())
            .await;

        assert!(session.player_is_alive_like_cpp());
        assert_eq!(session.player_health_like_cpp(), 450);
        assert_eq!(session.player_position_like_cpp(), Some(target_position));
        assert!(
            session
                .represented_delayed_resurrection_after_teleport_like_cpp()
                .is_none()
        );
    }

    #[tokio::test]
    async fn resurrect_response_decline_clears_request_like_cpp() {
        let (mut session, send_rx) = make_session();
        let resurrecter = ObjectGuid::create_player(1, 78);
        session.set_player_health_like_cpp(0, 1_000);
        session.set_represented_resurrection_request_like_cpp(
            crate::session::RepresentedResurrectionRequestLikeCpp {
                resurrecter,
                map_id: 571,
                position: Position::new(11.0, 22.0, 33.0, 1.5),
                health: 450,
                mana: 120,
                aura: 0,
            },
        );

        session
            .handle_resurrect_response(resurrect_response_packet(resurrecter, 1))
            .await;

        assert!(!session.player_is_alive_like_cpp());
        assert!(
            session
                .represented_resurrection_request_like_cpp()
                .is_none()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn resurrect_response_ignores_mismatched_resurrecter_like_cpp() {
        let (mut session, send_rx) = make_session();
        let resurrecter = ObjectGuid::create_player(1, 79);
        session.set_player_health_like_cpp(0, 1_000);
        session.set_represented_resurrection_request_like_cpp(
            crate::session::RepresentedResurrectionRequestLikeCpp {
                resurrecter,
                map_id: 571,
                position: Position::new(11.0, 22.0, 33.0, 1.5),
                health: 450,
                mana: 120,
                aura: 0,
            },
        );

        session
            .handle_resurrect_response(resurrect_response_packet(
                ObjectGuid::create_player(1, 80),
                0,
            ))
            .await;

        assert!(!session.player_is_alive_like_cpp());
        assert!(
            session
                .represented_resurrection_request_like_cpp()
                .is_some()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn resurrect_response_ignores_alive_player_like_cpp() {
        let (mut session, send_rx) = make_session();
        let resurrecter = ObjectGuid::create_player(1, 81);
        session.set_player_health_like_cpp(777, 1_000);
        session.set_represented_resurrection_request_like_cpp(
            crate::session::RepresentedResurrectionRequestLikeCpp {
                resurrecter,
                map_id: 571,
                position: Position::new(11.0, 22.0, 33.0, 1.5),
                health: 450,
                mana: 120,
                aura: 0,
            },
        );

        session
            .handle_resurrect_response(resurrect_response_packet(resurrecter, 0))
            .await;

        assert!(session.player_is_alive_like_cpp());
        assert_eq!(session.player_health_like_cpp(), 777);
        assert!(
            session
                .represented_resurrection_request_like_cpp()
                .is_some()
        );
        assert!(send_rx.try_recv().is_err());
    }

    fn bug_report_packet(report_type: bool, diag_info: &str, text: &str) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(report_type);
        pkt.write_bits(diag_info.len() as u32, 12);
        pkt.write_bits(text.len() as u32, 10);
        pkt.flush_bits();
        pkt.write_string(diag_info);
        pkt.write_string(text);
        pkt.reset_read();
        pkt
    }

    fn submit_user_feedback_packet(is_suggestion: bool, note: &str) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(571);
        pkt.write_float(1.25);
        pkt.write_float(2.5);
        pkt.write_float(3.75);
        pkt.write_float(4.0);
        pkt.write_int32(9);
        pkt.write_bits((note.len() + 1) as u32, 24);
        pkt.write_bit(is_suggestion);
        pkt.write_string(note);
        pkt.write_uint8(0);
        pkt.reset_read();
        pkt
    }

    fn support_ticket_submit_bug_packet(message: &str) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(571);
        pkt.write_float(1.25);
        pkt.write_float(2.5);
        pkt.write_float(3.75);
        pkt.write_float(4.0);
        pkt.write_int32(9);
        pkt.write_bits(message.len() as u32, 10);
        pkt.write_string(message);
        pkt
    }

    fn support_ticket_submit_complaint_packet(note: &str) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        let target = ObjectGuid::create_player(1, 42);
        pkt.write_int32(571);
        pkt.write_float(1.25);
        pkt.write_float(2.5);
        pkt.write_float(3.75);
        pkt.write_float(4.0);
        pkt.write_int32(9);
        pkt.write_packed_guid(&target);
        pkt.write_int32(1);
        pkt.write_int32(2);
        pkt.write_int32(4);
        pkt.write_uint32(0); // ChatLog.Lines.Count
        pkt.write_bit(false); // ReportLineIndex.HasValue
        pkt.flush_bits();
        pkt.write_bits(note.len() as u32, 10);
        pkt.write_bit(false); // MailInfo
        pkt.write_bit(false); // CalendarInfo
        pkt.write_bit(false); // PetInfo
        pkt.write_bit(false); // GuildInfo
        pkt.write_bit(false); // LFGListSearchResult
        pkt.write_bit(false); // LFGListApplicant
        pkt.write_bit(false); // ClubMessage
        pkt.write_bit(false); // ClubFinderResult
        pkt.write_bit(false); // Unused910
        pkt.flush_bits();
        pkt.write_uint32(0); // HorusChatLog.Lines.Count
        pkt.write_string(note);
        pkt
    }

    fn support_ticket_submit_suggestion_packet(message: &str) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(message.len() as u32, 10);
        pkt.write_string(message);
        pkt
    }

    fn object_update_recovery_packet(guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&guid);
        pkt.reset_read();
        pkt
    }

    fn stand_state_change_packet(state: u32) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(state);
        pkt.reset_read();
        pkt
    }

    #[tokio::test]
    async fn set_action_bar_toggles_updates_active_player_field_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 9001);
        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::new(1.0, 2.0, 3.0, 0.0));

        session
            .handle_set_action_bar_toggles(WorldPacket::from_bytes(&[0x2d]))
            .await;

        assert_eq!(session.active_player_multi_action_bars_like_cpp(), 0x2d);
        let sent = send_rx.try_recv().expect("VALUES update packet");
        assert_eq!(
            u16::from_le_bytes([sent[0], sent[1]]),
            ServerOpcodes::UpdateObject as u16
        );
    }

    #[tokio::test]
    async fn set_action_bar_toggles_short_packet_does_not_mutate_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 9002)));
        session
            .handle_set_action_bar_toggles(WorldPacket::from_bytes(&[]))
            .await;

        assert_eq!(session.active_player_multi_action_bars_like_cpp(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_action_button_adds_packed_action_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(12_345 | (0x80 << 24));
        pkt.write_uint8(7);
        pkt.reset_read();

        session.handle_set_action_button(pkt).await;

        assert_eq!(
            session.represented_action_button_like_cpp(7),
            Some(12_345 | (0x80 << 24))
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_action_button_zero_removes_action_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let mut add = WorldPacket::new_empty();
        add.write_uint32(1_337 | (0x40 << 24));
        add.write_uint8(9);
        add.reset_read();
        session.handle_set_action_button(add).await;
        assert_eq!(
            session.represented_action_button_like_cpp(9),
            Some(1_337 | (0x40 << 24))
        );

        let mut remove = WorldPacket::new_empty();
        remove.write_uint32(0);
        remove.write_uint8(9);
        remove.reset_read();
        session.handle_set_action_button(remove).await;

        assert_eq!(session.represented_action_button_like_cpp(9), Some(0));
    }

    #[tokio::test]
    async fn set_action_button_short_packet_does_not_mutate_like_cpp() {
        let (mut session, _send_rx) = make_session();
        session
            .handle_set_action_button(WorldPacket::from_bytes(&[0x01, 0x02]))
            .await;

        assert_eq!(session.represented_action_button_like_cpp(0), Some(0));
    }

    #[tokio::test]
    async fn set_taxi_benchmark_mode_sets_and_clears_player_flag_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        let player_guid = ObjectGuid::create_player(1, 9010);
        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        add_canonical_test_player_on_map_for_misc_test(
            &canonical,
            player_guid,
            Position::new(1.0, 2.0, 3.0, 0.0),
            571,
            0,
        );

        let mut enable = WorldPacket::new_empty();
        enable.write_bit(true);
        enable.flush_bits();
        enable.reset_read();
        session.handle_set_taxi_benchmark_mode(enable).await;
        assert!(session.represented_taxi_benchmark_mode_like_cpp());

        let mut disable = WorldPacket::new_empty();
        disable.write_bit(false);
        disable.flush_bits();
        disable.reset_read();
        session.handle_set_taxi_benchmark_mode(disable).await;
        assert!(!session.represented_taxi_benchmark_mode_like_cpp());
    }

    #[tokio::test]
    async fn set_taxi_benchmark_mode_short_packet_does_not_change_flag_like_cpp() {
        let (mut session, _send_rx) = make_session();
        session
            .handle_set_taxi_benchmark_mode(WorldPacket::from_bytes(&[]))
            .await;

        assert!(!session.represented_taxi_benchmark_mode_like_cpp());
    }

    #[tokio::test]
    async fn decline_guild_invites_sets_and_clears_auto_decline_flag_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        let player_guid = ObjectGuid::create_player(1, 9011);
        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        add_canonical_test_player_on_map_for_misc_test(
            &canonical,
            player_guid,
            Position::new(1.0, 2.0, 3.0, 0.0),
            571,
            0,
        );

        let mut enable = WorldPacket::new_empty();
        enable.write_bit(true);
        enable.flush_bits();
        enable.reset_read();
        session.handle_decline_guild_invites(enable).await;
        assert!(session.represented_auto_decline_guild_invites_like_cpp());

        let mut disable = WorldPacket::new_empty();
        disable.write_bit(false);
        disable.flush_bits();
        disable.reset_read();
        session.handle_decline_guild_invites(disable).await;
        assert!(!session.represented_auto_decline_guild_invites_like_cpp());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn decline_guild_invites_short_packet_does_not_change_flag_like_cpp() {
        let (mut session, _send_rx) = make_session();
        session
            .handle_decline_guild_invites(WorldPacket::from_bytes(&[]))
            .await;

        assert!(!session.represented_auto_decline_guild_invites_like_cpp());
    }

    #[tokio::test]
    async fn guild_decline_invitation_clears_pending_invite_when_unguilded_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_represented_guild_id_like_cpp(0);
        session.set_represented_guild_id_invited_like_cpp(7_001);

        session
            .handle_guild_decline_invitation(WorldPacket::new_empty())
            .await;

        assert_eq!(session.represented_guild_id_invited_like_cpp(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn guild_decline_invitation_preserves_pending_invite_when_already_guilded_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_represented_guild_id_like_cpp(42);
        session.set_represented_guild_id_invited_like_cpp(7_001);

        session
            .handle_guild_decline_invitation(WorldPacket::new_empty())
            .await;

        assert_eq!(session.represented_guild_id_invited_like_cpp(), 7_001);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn accept_guild_invite_records_invited_guild_when_unguilded_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_represented_guild_id_like_cpp(0);
        session.set_represented_guild_id_invited_like_cpp(7_001);

        session
            .handle_accept_guild_invite(WorldPacket::new_empty())
            .await;

        assert_eq!(
            session.represented_guild_accept_invites_like_cpp(),
            &[7_001]
        );
        assert_eq!(session.represented_guild_id_invited_like_cpp(), 7_001);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn accept_guild_invite_ignores_guilded_player_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_represented_guild_id_like_cpp(42);
        session.set_represented_guild_id_invited_like_cpp(7_001);

        session
            .handle_accept_guild_invite(WorldPacket::new_empty())
            .await;

        assert!(
            session
                .represented_guild_accept_invites_like_cpp()
                .is_empty()
        );
        assert_eq!(session.represented_guild_id_invited_like_cpp(), 7_001);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn accept_guild_invite_ignores_missing_invited_guild_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_represented_guild_id_like_cpp(0);
        session.set_represented_guild_id_invited_like_cpp(0);

        session
            .handle_accept_guild_invite(WorldPacket::new_empty())
            .await;

        assert!(
            session
                .represented_guild_accept_invites_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn arena_team_decline_clears_invited_arena_team_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_represented_arena_team_id_invited_like_cpp(12_345);

        session
            .handle_arena_team_decline(WorldPacket::new_empty())
            .await;

        assert_eq!(session.represented_arena_team_id_invited_like_cpp(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn cancel_trade_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();

        session.handle_cancel_trade(WorldPacket::new_empty()).await;

        assert!(
            session
                .represented_trade_cancel_statuses_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    fn can_duel_packet(target_guid: ObjectGuid, to_the_death: bool) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_bytes(&target_guid.to_raw_bytes());
        packet.write_bit(to_the_death);
        packet.flush_bits();
        packet.reset_read();
        packet
    }

    #[tokio::test]
    async fn can_duel_missing_target_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        session.set_canonical_map_manager(Arc::clone(&canonical));
        let target_guid = ObjectGuid::create_player(1, 88);

        session
            .handle_can_duel(can_duel_packet(target_guid, false))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert!(
            session
                .represented_can_duel_spell_casts_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn can_duel_allows_target_without_duel_and_records_spell_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        let target_guid = ObjectGuid::create_player(1, 88);
        add_canonical_test_player_on_map_for_misc_test(
            &canonical,
            target_guid,
            Position::new(1.0, 2.0, 3.0, 0.0),
            571,
            0,
        );
        session.set_canonical_map_manager(Arc::clone(&canonical));

        session
            .handle_can_duel(can_duel_packet(target_guid, true))
            .await;

        let bytes = send_rx.try_recv().expect("can duel result");
        let mut packet = WorldPacket::from_bytes(&bytes);
        assert_eq!(packet.server_opcode(), Some(ServerOpcodes::CanDuelResult));
        assert_eq!(
            packet.read_uint16().unwrap(),
            ServerOpcodes::CanDuelResult as u16
        );
        let guid_bytes = packet.read_bytes(16).unwrap();
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&guid_bytes);
        assert_eq!(ObjectGuid::from_raw_bytes(&raw), target_guid);
        assert!(packet.read_bit().unwrap());
        assert_eq!(
            session.represented_can_duel_spell_casts_like_cpp(),
            &[crate::session::RepresentedCanDuelSpellCastLikeCpp {
                target_guid,
                spell_id: crate::session::SPELL_DUEL_LIKE_CPP,
                to_the_death: true,
            }]
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn can_duel_rejects_target_with_any_duel_info_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        let target_guid = ObjectGuid::create_player(1, 88);
        let opponent_guid = ObjectGuid::create_player(1, 89);
        add_canonical_test_player_on_map_for_misc_test(
            &canonical,
            target_guid,
            Position::new(1.0, 2.0, 3.0, 0.0),
            571,
            0,
        );
        {
            let mut manager = canonical.lock().unwrap();
            let player = manager
                .find_map_mut(571, 0)
                .unwrap()
                .map_mut()
                .get_typed_player_mut(target_guid)
                .unwrap();
            player.set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: opponent_guid,
                state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
            }));
        }
        session.set_canonical_map_manager(Arc::clone(&canonical));

        session
            .handle_can_duel(can_duel_packet(target_guid, false))
            .await;

        let bytes = send_rx.try_recv().expect("can duel result");
        let mut packet = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            packet.read_uint16().unwrap(),
            ServerOpcodes::CanDuelResult as u16
        );
        let _ = packet.read_bytes(16).unwrap();
        assert!(!packet.read_bit().unwrap());
        assert!(
            session
                .represented_can_duel_spell_casts_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn can_duel_uses_mounted_spell_when_source_is_mounted_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        let target_guid = ObjectGuid::create_player(1, 88);
        add_canonical_test_player_on_map_for_misc_test(
            &canonical,
            target_guid,
            Position::new(1.0, 2.0, 3.0, 0.0),
            571,
            0,
        );
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_player_mounted_like_cpp(true);

        session
            .handle_can_duel(can_duel_packet(target_guid, false))
            .await;

        assert_eq!(
            session.represented_can_duel_spell_casts_like_cpp(),
            &[crate::session::RepresentedCanDuelSpellCastLikeCpp {
                target_guid,
                spell_id: crate::session::SPELL_MOUNTED_DUEL_LIKE_CPP,
                to_the_death: false,
            }]
        );
    }

    #[tokio::test]
    async fn cancel_trade_cancels_represented_trade_and_sends_status_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

        session.handle_cancel_trade(WorldPacket::new_empty()).await;

        assert_eq!(
            session.represented_trade_cancel_statuses_like_cpp(),
            &[TRADE_STATUS_CANCELLED_LIKE_CPP]
        );
        assert!(
            session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        let bytes = send_rx.try_recv().expect("trade status");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn cancel_trade_cancels_partner_represented_trade_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_cancel_trade(WorldPacket::new_empty())
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert!(
            source_session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        assert!(
            partner_session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        assert_eq!(
            source_session.represented_trade_cancel_statuses_like_cpp(),
            &[TRADE_STATUS_CANCELLED_LIKE_CPP]
        );
        assert_eq!(
            partner_session.represented_trade_cancel_statuses_like_cpp(),
            &[TRADE_STATUS_CANCELLED_LIKE_CPP]
        );

        let source_bytes = source_send_rx.try_recv().expect("source trade status");
        let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
        assert_eq!(source_bytes, partner_bytes);
        assert_eq!(
            u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(source_bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
    }

    fn accept_trade_packet(state_index: u32) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(state_index);
        pkt.reset_read();
        pkt
    }

    fn clear_trade_item_packet(trade_slot: u8) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(trade_slot);
        pkt.reset_read();
        pkt
    }

    fn set_trade_item_packet(trade_slot: u8, pack_slot: u8, item_slot_in_pack: u8) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(trade_slot);
        pkt.write_uint8(pack_slot);
        pkt.write_uint8(item_slot_in_pack);
        pkt.reset_read();
        pkt
    }

    fn set_trade_gold_packet(coinage: u64) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint64(coinage);
        pkt.reset_read();
        pkt
    }

    fn set_trade_spell_packet(spell_id: u32, pack_slot: u8, item_slot_in_pack: u8) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(spell_id);
        pkt.write_uint8(pack_slot);
        pkt.write_uint8(item_slot_in_pack);
        pkt.reset_read();
        pkt
    }

    fn sign_petition_packet(petition_guid: ObjectGuid, choice: u8) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&petition_guid.to_raw_bytes());
        pkt.write_uint8(choice);
        pkt.reset_read();
        pkt
    }

    fn decline_petition_packet(petition_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&petition_guid.to_raw_bytes());
        pkt.reset_read();
        pkt
    }

    fn query_petition_packet(petition_id: u32, item_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(petition_id);
        pkt.write_bytes(&item_guid.to_raw_bytes());
        pkt.reset_read();
        pkt
    }

    fn trade_test_spell_info(spell_id: i32) -> SpellInfo {
        SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            effects: Vec::new(),
        }
    }

    fn install_trade_test_spell(session: &mut crate::session::WorldSession, spell_id: i32) {
        let mut spell_store = SpellStore::new();
        spell_store.insert(spell_id, trade_test_spell_info(spell_id));
        session.set_spell_store(Arc::new(spell_store));
        session.set_known_spells_like_cpp(vec![spell_id]);
    }

    fn insert_trade_test_item(
        session: &mut crate::session::WorldSession,
        owner_guid: ObjectGuid,
        slot: u8,
        item_guid: ObjectGuid,
        entry_id: u32,
    ) {
        session.insert_inventory_item_like_cpp(
            slot,
            crate::session::InventoryItem {
                guid: item_guid,
                entry_id,
                db_guid: item_guid.counter() as u64,
                inventory_type: None,
            },
        );
        let item = session.make_inventory_item_object(
            item_guid,
            entry_id,
            owner_guid,
            1,
            0,
            ItemContext::None,
            slot,
        );
        session.insert_inventory_item_object(item);
    }

    #[tokio::test]
    async fn accept_trade_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();

        session.handle_accept_trade(accept_trade_packet(0)).await;

        assert!(!session.represented_trade_accepted_like_cpp());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn accept_trade_state_changed_resets_acceptance_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        session.set_represented_partner_trade_server_state_index_like_cpp(7);

        session.handle_accept_trade(accept_trade_packet(8)).await;

        assert!(!session.represented_trade_accepted_like_cpp());
        assert_eq!(
            session.represented_active_trade_partner_like_cpp(),
            Some(partner_guid)
        );
        let bytes = send_rx.try_recv().expect("trade status");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes[2], TRADE_STATUS_STATE_CHANGED_LIKE_CPP << 2);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn accept_trade_records_acceptance_and_notifies_partner_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
        source_session.set_represented_partner_trade_server_state_index_like_cpp(42);

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_accept_trade(accept_trade_packet(42))
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert!(source_session.represented_trade_accepted_like_cpp());
        assert!(source_send_rx.try_recv().is_err());
        let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
        assert_eq!(
            u16::from_le_bytes([partner_bytes[0], partner_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(partner_bytes[2], TRADE_STATUS_ACCEPTED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn clear_trade_item_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_clear_trade_item(clear_trade_item_packet(2))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert!(session.represented_trade_item_like_cpp(2).is_none());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn clear_trade_item_invalid_slot_updates_client_state_only_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        session.set_represented_trade_accepted_like_cpp_for_test(true);

        session
            .handle_clear_trade_item(clear_trade_item_packet(7))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 2);
        assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
        assert!(session.represented_trade_accepted_like_cpp());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn clear_trade_item_empty_slot_only_updates_client_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        session.set_represented_trade_accepted_like_cpp_for_test(true);

        session
            .handle_clear_trade_item(clear_trade_item_packet(2))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 2);
        assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
        assert!(session.represented_trade_accepted_like_cpp());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn clear_trade_item_clears_slot_and_unaccepts_both_sides_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        let item_guid = ObjectGuid::create_item(1, 1234);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
        source_session.set_represented_trade_item_like_cpp_for_test(2, item_guid);
        source_session.set_represented_trade_accepted_like_cpp_for_test(true);
        partner_session.set_represented_trade_accepted_like_cpp_for_test(true);

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_clear_trade_item(clear_trade_item_packet(2))
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert_eq!(
            source_session.represented_trade_client_state_index_like_cpp(),
            2
        );
        assert_eq!(
            source_session.represented_trade_server_state_index_like_cpp(),
            2
        );
        assert!(source_session.represented_trade_item_like_cpp(2).is_none());
        assert!(!source_session.represented_trade_accepted_like_cpp());
        assert!(!partner_session.represented_trade_accepted_like_cpp());

        let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
        let partner_bytes = partner_send_rx
            .try_recv()
            .expect("partner unaccepted status");
        assert_eq!(source_bytes, partner_bytes);
        assert_eq!(
            u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_item_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 77);
        let item_guid = ObjectGuid::create_item(1, 1234);
        session.set_player_guid(Some(player_guid));
        insert_trade_test_item(&mut session, player_guid, 23, item_guid, 700);

        session
            .handle_set_trade_item(set_trade_item_packet(2, 255, 23))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert!(session.represented_trade_item_like_cpp(2).is_none());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_trade_item_invalid_slot_cancels_without_client_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        let item_guid = ObjectGuid::create_item(1, 1234);
        session.set_player_guid(Some(player_guid));
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        insert_trade_test_item(&mut session, player_guid, 23, item_guid, 700);

        session
            .handle_set_trade_item(set_trade_item_packet(7, 255, 23))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert!(session.represented_trade_item_like_cpp(2).is_none());
        let bytes = send_rx.try_recv().expect("cancelled trade status");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_item_missing_inventory_cancels_without_client_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

        session
            .handle_set_trade_item(set_trade_item_packet(2, 255, 23))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert!(session.represented_trade_item_like_cpp(2).is_none());
        let bytes = send_rx.try_recv().expect("cancelled trade status");
        assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_item_duplicate_item_cancels_without_client_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        let item_guid = ObjectGuid::create_item(1, 1234);
        session.set_player_guid(Some(player_guid));
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        session.set_represented_trade_item_like_cpp_for_test(1, item_guid);
        insert_trade_test_item(&mut session, player_guid, 23, item_guid, 700);

        session
            .handle_set_trade_item(set_trade_item_packet(2, 255, 23))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_item_like_cpp(1), Some(item_guid));
        assert!(session.represented_trade_item_like_cpp(2).is_none());
        let bytes = send_rx.try_recv().expect("cancelled trade status");
        assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_item_records_slot_and_unaccepts_both_sides_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        let item_guid = ObjectGuid::create_item(1, 1234);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
        source_session.set_represented_trade_accepted_like_cpp_for_test(true);
        partner_session.set_represented_trade_accepted_like_cpp_for_test(true);
        insert_trade_test_item(&mut source_session, source_guid, 23, item_guid, 700);

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_set_trade_item(set_trade_item_packet(2, 255, 23))
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert_eq!(
            source_session.represented_trade_client_state_index_like_cpp(),
            2
        );
        assert_eq!(
            source_session.represented_trade_server_state_index_like_cpp(),
            2
        );
        assert_eq!(
            source_session.represented_trade_item_like_cpp(2),
            Some(item_guid)
        );
        assert!(!source_session.represented_trade_accepted_like_cpp());
        assert!(!partner_session.represented_trade_accepted_like_cpp());

        let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
        let partner_bytes = partner_send_rx
            .try_recv()
            .expect("partner unaccepted status");
        assert_eq!(source_bytes, partner_bytes);
        assert_eq!(
            u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_gold_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_player_gold_like_cpp(100);

        session
            .handle_set_trade_gold(set_trade_gold_packet(50))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_money_like_cpp(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_trade_gold_same_money_only_updates_client_state_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_player_gold_like_cpp(100);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

        session
            .handle_set_trade_gold(set_trade_gold_packet(0))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 2);
        assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_money_like_cpp(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_trade_gold_not_enough_money_sends_failed_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_player_gold_like_cpp(10);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

        session
            .handle_set_trade_gold(set_trade_gold_packet(50))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 2);
        assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_money_like_cpp(), 0);
        let bytes = send_rx.try_recv().expect("failed trade status");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes[2], TRADE_STATUS_FAILED_LIKE_CPP << 2);
        assert_eq!(
            i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP
        );
        assert_eq!(
            i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
            0
        );
    }

    #[tokio::test]
    async fn set_trade_gold_records_money_and_unaccepts_both_sides_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_player_gold_like_cpp(100);
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
        source_session.set_represented_trade_accepted_like_cpp_for_test(true);
        partner_session.set_represented_trade_accepted_like_cpp_for_test(true);

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_set_trade_gold(set_trade_gold_packet(75))
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert_eq!(
            source_session.represented_trade_client_state_index_like_cpp(),
            2
        );
        assert_eq!(
            source_session.represented_trade_server_state_index_like_cpp(),
            2
        );
        assert_eq!(source_session.represented_trade_money_like_cpp(), 75);
        assert!(!source_session.represented_trade_accepted_like_cpp());
        assert!(!partner_session.represented_trade_accepted_like_cpp());

        let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
        let partner_bytes = partner_send_rx
            .try_recv()
            .expect("partner unaccepted status");
        assert_eq!(source_bytes, partner_bytes);
        assert_eq!(
            u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_spell_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();
        install_trade_test_spell(&mut session, 7418);

        session
            .handle_set_trade_spell(set_trade_spell_packet(7418, 255, 23))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_spell_like_cpp(), 0);
        assert!(
            session
                .represented_trade_spell_cast_item_like_cpp()
                .is_none()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_trade_spell_zero_clears_spell_and_unaccepts_both_sides_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        let cast_item_guid = ObjectGuid::create_item(1, 1234);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
        source_session.set_represented_trade_spell_like_cpp_for_test(7418, Some(cast_item_guid));
        source_session.set_represented_trade_accepted_like_cpp_for_test(true);
        partner_session.set_represented_trade_accepted_like_cpp_for_test(true);

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_set_trade_spell(set_trade_spell_packet(0, 0, 255))
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert_eq!(
            source_session.represented_trade_client_state_index_like_cpp(),
            1
        );
        assert_eq!(
            source_session.represented_trade_server_state_index_like_cpp(),
            2
        );
        assert_eq!(source_session.represented_trade_spell_like_cpp(), 0);
        assert!(
            source_session
                .represented_trade_spell_cast_item_like_cpp()
                .is_none()
        );
        assert!(!source_session.represented_trade_accepted_like_cpp());
        assert!(!partner_session.represented_trade_accepted_like_cpp());

        let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
        let partner_bytes = partner_send_rx
            .try_recv()
            .expect("partner unaccepted status");
        assert_eq!(source_bytes, partner_bytes);
        assert_eq!(
            u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_spell_missing_spell_info_clears_existing_spell_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        let cast_item_guid = ObjectGuid::create_item(1, 1234);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        session.set_represented_trade_spell_like_cpp_for_test(7418, Some(cast_item_guid));
        session.set_represented_trade_accepted_like_cpp_for_test(true);

        session
            .handle_set_trade_spell(set_trade_spell_packet(9999, 0, 255))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_server_state_index_like_cpp(), 2);
        assert_eq!(session.represented_trade_spell_like_cpp(), 0);
        assert!(
            session
                .represented_trade_spell_cast_item_like_cpp()
                .is_none()
        );
        assert!(!session.represented_trade_accepted_like_cpp());
        let bytes = send_rx.try_recv().expect("unaccepted status");
        assert_eq!(bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_spell_unknown_spell_clears_existing_spell_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        let cast_item_guid = ObjectGuid::create_item(1, 1234);
        let mut spell_store = SpellStore::new();
        spell_store.insert(7418, trade_test_spell_info(7418));
        session.set_spell_store(Arc::new(spell_store));
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        session.set_represented_trade_spell_like_cpp_for_test(7418, Some(cast_item_guid));
        session.set_represented_trade_accepted_like_cpp_for_test(true);

        session
            .handle_set_trade_spell(set_trade_spell_packet(7418, 0, 255))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_server_state_index_like_cpp(), 2);
        assert_eq!(session.represented_trade_spell_like_cpp(), 0);
        assert!(
            session
                .represented_trade_spell_cast_item_like_cpp()
                .is_none()
        );
        assert!(!session.represented_trade_accepted_like_cpp());
        let bytes = send_rx.try_recv().expect("unaccepted status");
        assert_eq!(bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_spell_valid_records_spell_and_cast_item_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        let cast_item_guid = ObjectGuid::create_item(1, 1234);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
        source_session.set_represented_trade_accepted_like_cpp_for_test(true);
        partner_session.set_represented_trade_accepted_like_cpp_for_test(true);
        install_trade_test_spell(&mut source_session, 7418);
        insert_trade_test_item(&mut source_session, source_guid, 23, cast_item_guid, 700);

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_set_trade_spell(set_trade_spell_packet(7418, 255, 23))
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert_eq!(
            source_session.represented_trade_client_state_index_like_cpp(),
            1
        );
        assert_eq!(
            source_session.represented_trade_server_state_index_like_cpp(),
            2
        );
        assert_eq!(source_session.represented_trade_spell_like_cpp(), 7418);
        assert_eq!(
            source_session.represented_trade_spell_cast_item_like_cpp(),
            Some(cast_item_guid)
        );
        assert!(!source_session.represented_trade_accepted_like_cpp());
        assert!(!partner_session.represented_trade_accepted_like_cpp());

        let source_bytes = source_send_rx.try_recv().expect("source unaccepted status");
        let partner_bytes = partner_send_rx
            .try_recv()
            .expect("partner unaccepted status");
        assert_eq!(source_bytes, partner_bytes);
        assert_eq!(
            u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(source_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_trade_spell_same_spell_and_cast_item_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        let cast_item_guid = ObjectGuid::create_item(1, 1234);
        session.set_player_guid(Some(player_guid));
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        session.set_represented_trade_spell_like_cpp_for_test(7418, Some(cast_item_guid));
        session.set_represented_trade_accepted_like_cpp_for_test(true);
        install_trade_test_spell(&mut session, 7418);
        insert_trade_test_item(&mut session, player_guid, 23, cast_item_guid, 700);

        session
            .handle_set_trade_spell(set_trade_spell_packet(7418, 255, 23))
            .await;

        assert_eq!(session.represented_trade_client_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_server_state_index_like_cpp(), 1);
        assert_eq!(session.represented_trade_spell_like_cpp(), 7418);
        assert_eq!(
            session.represented_trade_spell_cast_item_like_cpp(),
            Some(cast_item_guid)
        );
        assert!(session.represented_trade_accepted_like_cpp());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn sign_petition_records_guid_and_choice_like_cpp_without_runtime_mgr() {
        let (mut session, send_rx) = make_session();
        let petition_guid = ObjectGuid::create_item(1, 91_777);

        session
            .handle_sign_petition(sign_petition_packet(petition_guid, 1))
            .await;

        assert_eq!(
            session.represented_sign_petitions_like_cpp(),
            &[crate::session::RepresentedSignPetitionLikeCpp {
                petition_guid,
                choice: 1,
            }]
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn decline_petition_records_guid_like_cpp_without_client_notification() {
        let (mut session, send_rx) = make_session();
        let petition_guid = ObjectGuid::create_item(1, 91_778);

        session
            .handle_decline_petition(decline_petition_packet(petition_guid))
            .await;

        assert_eq!(
            session.represented_decline_petitions_like_cpp(),
            &[crate::session::RepresentedDeclinePetitionLikeCpp { petition_guid }]
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn query_petition_without_runtime_mgr_sends_not_found_like_cpp() {
        let (mut session, send_rx) = make_session();
        let item_guid = ObjectGuid::create_item(1, 91_779);

        session
            .handle_query_petition(query_petition_packet(123, item_guid))
            .await;

        assert_eq!(
            session.represented_query_petitions_like_cpp(),
            &[crate::session::RepresentedQueryPetitionLikeCpp {
                petition_id: 123,
                item_guid,
            }]
        );

        let bytes = send_rx.try_recv().expect("query petition response");
        let mut body = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            body.server_opcode(),
            Some(ServerOpcodes::QueryPetitionResponse)
        );
        assert_eq!(
            body.read_uint16().unwrap(),
            ServerOpcodes::QueryPetitionResponse as u16
        );
        assert_eq!(body.read_uint32().unwrap(), item_guid.counter() as u32);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.remaining(), 0);
    }

    #[tokio::test]
    async fn unaccept_trade_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_unaccept_trade(WorldPacket::new_empty())
            .await;

        assert!(!session.represented_trade_accepted_like_cpp());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn unaccept_trade_clears_acceptance_and_notifies_partner_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));
        source_session.set_represented_partner_trade_server_state_index_like_cpp(1);
        source_session.accept_represented_trade_like_cpp(1);
        assert!(source_session.represented_trade_accepted_like_cpp());

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_unaccept_trade(WorldPacket::new_empty())
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert!(!source_session.represented_trade_accepted_like_cpp());
        assert!(source_send_rx.try_recv().is_err());
        let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
        assert_eq!(
            u16::from_le_bytes([partner_bytes[0], partner_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(partner_bytes[2], TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn busy_trade_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();

        session.handle_busy_trade(WorldPacket::new_empty()).await;

        assert!(
            session
                .represented_trade_cancel_statuses_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn busy_trade_cancels_represented_trade_and_sends_status_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

        session.handle_busy_trade(WorldPacket::new_empty()).await;

        assert_eq!(
            session.represented_trade_cancel_statuses_like_cpp(),
            &[TRADE_STATUS_PLAYER_BUSY_LIKE_CPP]
        );
        assert!(
            session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        let bytes = send_rx.try_recv().expect("trade status");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes[2], TRADE_STATUS_PLAYER_BUSY_LIKE_CPP << 1);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn busy_trade_cancels_partner_represented_trade_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_busy_trade(WorldPacket::new_empty())
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert!(
            source_session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        assert!(
            partner_session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        assert_eq!(
            source_session.represented_trade_cancel_statuses_like_cpp(),
            &[TRADE_STATUS_PLAYER_BUSY_LIKE_CPP]
        );
        assert_eq!(
            partner_session.represented_trade_cancel_statuses_like_cpp(),
            &[TRADE_STATUS_PLAYER_BUSY_LIKE_CPP]
        );

        let source_bytes = source_send_rx.try_recv().expect("source trade status");
        let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
        assert_eq!(source_bytes, partner_bytes);
        assert_eq!(
            u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(source_bytes[2], TRADE_STATUS_PLAYER_BUSY_LIKE_CPP << 1);
    }

    #[tokio::test]
    async fn begin_trade_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();

        session.handle_begin_trade(WorldPacket::new_empty()).await;

        assert!(
            session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn begin_trade_sends_initiated_status_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

        session.handle_begin_trade(WorldPacket::new_empty()).await;

        assert_eq!(
            session.represented_active_trade_partner_like_cpp(),
            Some(partner_guid)
        );
        let bytes = send_rx.try_recv().expect("trade status");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes[2], TRADE_STATUS_INITIATED_LIKE_CPP << 2);
        assert_eq!(
            u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
            0
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn begin_trade_sends_initiated_status_to_partner_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_begin_trade(WorldPacket::new_empty())
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert_eq!(
            source_session.represented_active_trade_partner_like_cpp(),
            Some(partner_guid)
        );
        assert_eq!(
            partner_session.represented_active_trade_partner_like_cpp(),
            Some(source_guid)
        );

        let source_bytes = source_send_rx.try_recv().expect("source trade status");
        let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
        assert_eq!(source_bytes, partner_bytes);
        assert_eq!(
            u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(source_bytes[2], TRADE_STATUS_INITIATED_LIKE_CPP << 2);
        assert_eq!(
            u32::from_le_bytes([
                source_bytes[3],
                source_bytes[4],
                source_bytes[5],
                source_bytes[6]
            ]),
            0
        );
    }

    #[tokio::test]
    async fn ignore_trade_without_active_trade_is_noop_like_cpp() {
        let (mut session, send_rx) = make_session();

        session.handle_ignore_trade(WorldPacket::new_empty()).await;

        assert!(
            session
                .represented_trade_cancel_statuses_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn ignore_trade_cancels_represented_trade_and_sends_status_like_cpp() {
        let (mut session, send_rx) = make_session();
        let partner_guid = ObjectGuid::create_player(1, 88);
        session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));

        session.handle_ignore_trade(WorldPacket::new_empty()).await;

        assert_eq!(
            session.represented_trade_cancel_statuses_like_cpp(),
            &[TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP]
        );
        assert!(
            session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        let bytes = send_rx.try_recv().expect("trade status");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(bytes[2], TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP << 2);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn ignore_trade_cancels_partner_represented_trade_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut partner_session, partner_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let partner_guid = ObjectGuid::create_player(1, 88);
        source_session.set_player_guid(Some(source_guid));
        partner_session.set_player_guid(Some(partner_guid));
        source_session.set_represented_active_trade_partner_like_cpp(Some(partner_guid));
        partner_session.set_represented_active_trade_partner_like_cpp(Some(source_guid));

        let registry = Arc::new(PlayerRegistry::default());
        let partner_command_tx = partner_session.session_command_tx();
        registry.insert(
            partner_guid,
            broadcast_info_with_command_tx(partner_command_tx),
        );
        source_session.set_player_registry(registry);

        source_session
            .handle_ignore_trade(WorldPacket::new_empty())
            .await;
        partner_session
            .process_represented_session_commands_like_cpp()
            .await;

        assert!(
            source_session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        assert!(
            partner_session
                .represented_active_trade_partner_like_cpp()
                .is_none()
        );
        assert_eq!(
            source_session.represented_trade_cancel_statuses_like_cpp(),
            &[TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP]
        );
        assert_eq!(
            partner_session.represented_trade_cancel_statuses_like_cpp(),
            &[TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP]
        );

        let source_bytes = source_send_rx.try_recv().expect("source trade status");
        let partner_bytes = partner_send_rx.try_recv().expect("partner trade status");
        assert_eq!(source_bytes, partner_bytes);
        assert_eq!(
            u16::from_le_bytes([source_bytes[0], source_bytes[1]]),
            ServerOpcodes::TradeStatus as u16
        );
        assert_eq!(source_bytes[2], TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP << 2);
    }

    #[tokio::test]
    async fn set_advanced_combat_logging_sets_and_clears_player_state_like_cpp() {
        let (mut session, _send_rx) = make_session();

        let mut enable = WorldPacket::new_empty();
        enable.write_bit(true);
        enable.flush_bits();
        enable.reset_read();
        session.handle_set_advanced_combat_logging(enable).await;
        assert!(session.represented_advanced_combat_logging_enabled_like_cpp());

        let mut disable = WorldPacket::new_empty();
        disable.write_bit(false);
        disable.flush_bits();
        disable.reset_read();
        session.handle_set_advanced_combat_logging(disable).await;
        assert!(!session.represented_advanced_combat_logging_enabled_like_cpp());
    }

    #[tokio::test]
    async fn set_advanced_combat_logging_short_packet_does_not_change_state_like_cpp() {
        let (mut session, _send_rx) = make_session();
        session.represented_set_advanced_combat_logging_like_cpp(true);

        session
            .handle_set_advanced_combat_logging(WorldPacket::from_bytes(&[]))
            .await;

        assert!(session.represented_advanced_combat_logging_enabled_like_cpp());
    }

    #[tokio::test]
    async fn set_currency_flags_updates_existing_currency_and_sends_setup_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
            wow_data::CurrencyTypesEntry {
                max_qty: 200,
                max_earnable_per_week: 50,
                flags: wow_constants::CurrencyTypesFlags::TRACK_QUANTITY,
                flags_b: wow_constants::CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED,
                ..currency_entry(395)
            },
        ])));
        session.set_player_currencies_like_cpp(HashMap::from([(
            395,
            crate::session::PlayerCurrency {
                state: crate::session::PlayerCurrencyState::Unchanged,
                quantity: 123,
                weekly_quantity: 20,
                tracked_quantity: 7,
                increased_cap_quantity: 0,
                earned_quantity: 300,
                flags: 0,
            },
        )]));

        let mut request = WorldPacket::new_empty();
        request.write_uint32(395);
        request.write_uint8(0x1f);
        request.reset_read();
        session.handle_set_currency_flags(request).await;

        let currency = session.player_currencies_like_cpp().get(&395).unwrap();
        assert_eq!(currency.flags, 0x1f);
        assert_eq!(currency.state, crate::session::PlayerCurrencyState::Changed);

        let sent = send_rx.try_recv().expect("SMSG_SETUP_CURRENCY");
        let mut setup = WorldPacket::from_bytes(&sent);
        assert_eq!(setup.server_opcode(), Some(ServerOpcodes::SetupCurrency));
        setup.skip_opcode();
        assert_eq!(setup.read_uint32().unwrap(), 1);
        assert_eq!(setup.read_int32().unwrap(), 395);
        assert_eq!(setup.read_int32().unwrap(), 123);
        assert!(setup.read_bit().unwrap());
        assert!(setup.read_bit().unwrap());
        assert!(setup.read_bit().unwrap());
        assert!(setup.read_bit().unwrap());
        assert!(setup.read_bit().unwrap());
        assert!(!setup.read_bit().unwrap());
        assert!(!setup.read_bit().unwrap());
        assert_eq!(setup.read_bits(5).unwrap(), 0x0c);
        assert_eq!(setup.read_uint32().unwrap(), 20);
        assert_eq!(setup.read_uint32().unwrap(), 50);
        assert_eq!(setup.read_uint32().unwrap(), 7);
        assert_eq!(setup.read_int32().unwrap(), 200);
        assert_eq!(setup.read_int32().unwrap(), 300);
    }

    #[tokio::test]
    async fn set_currency_flags_missing_player_currency_still_replays_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
            currency_entry(395),
        ])));

        let mut request = WorldPacket::new_empty();
        request.write_uint32(395);
        request.write_uint8(0x1f);
        request.reset_read();
        session.handle_set_currency_flags(request).await;

        assert!(session.player_currencies_like_cpp().get(&395).is_none());
        let sent = send_rx.try_recv().expect("C++ still calls SendCurrencies");
        assert_eq!(
            WorldPacket::from_bytes(&sent).server_opcode(),
            Some(ServerOpcodes::SetupCurrency)
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_currency_flags_short_packet_does_not_send_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_set_currency_flags(WorldPacket::from_bytes(&[0x01, 0x00]))
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_ammo_is_silent_like_cpp_debug_only_handler() {
        let (mut session, send_rx) = make_session();

        session.handle_set_ammo(WorldPacket::new_empty()).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_game_event_debug_view_state_is_silent_like_cpp_debug_only_handler() {
        let (mut session, send_rx) = make_session();

        session
            .handle_set_game_event_debug_view_state(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn addon_list_is_silent_like_cpp_log_only_handler() {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(1);
        pkt.write_bits(5, 10);
        pkt.flush_bits();
        pkt.write_string("Atlas");
        pkt.reset_read();

        session.handle_addon_list(pkt).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn add_battlenet_friend_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_add_battlenet_friend(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_insert_items_left_to_right_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_set_insert_items_left_to_right(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn client_telemetry_null_family_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        for _ in 0..5 {
            session
                .handle_client_telemetry_null_like_cpp(WorldPacket::new_empty())
                .await;
        }

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn unhandled_client_null_family_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        for _ in 0..5 {
            session
                .handle_unhandled_client_null_like_cpp(WorldPacket::new_empty())
                .await;
        }

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn showing_helm_and_cloak_are_silent_like_cpp_debug_only_handlers() {
        let (mut session, send_rx) = make_session();

        session.handle_showing_helm(WorldPacket::new_empty()).await;
        session.handle_showing_cloak(WorldPacket::new_empty()).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_title_requires_known_positive_title_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.represented_learn_title_like_cpp(42);

        let mut known = WorldPacket::new_empty();
        known.write_int32(42);
        known.reset_read();
        session.handle_set_title(known).await;
        assert_eq!(session.represented_chosen_title_like_cpp(), 42);

        let mut unknown = WorldPacket::new_empty();
        unknown.write_int32(77);
        unknown.reset_read();
        session.handle_set_title(unknown).await;
        assert_eq!(
            session.represented_chosen_title_like_cpp(),
            42,
            "C++ returns before SetChosenTitle when HasTitle fails"
        );

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_title_non_positive_clears_to_zero_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.represented_learn_title_like_cpp(42);
        session.represented_set_chosen_title_like_cpp(42);

        let mut clear = WorldPacket::new_empty();
        clear.write_int32(-1);
        clear.reset_read();
        session.handle_set_title(clear).await;

        assert_eq!(session.represented_chosen_title_like_cpp(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_title_updates_canonical_player_title_field_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager_for_misc_test();
        let player_guid = ObjectGuid::create_player(1, 57);
        let player_position = Position::new(10.0, 0.0, 0.0, 0.0);

        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.represented_learn_title_like_cpp(42);
        add_canonical_test_player_on_map_for_misc_test(
            &canonical,
            player_guid,
            player_position,
            571,
            0,
        );
        session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

        let mut request = WorldPacket::new_empty();
        request.write_int32(42);
        request.reset_read();
        session.handle_set_title(request).await;

        assert_eq!(session.represented_chosen_title_like_cpp(), 42);
        assert_eq!(
            session
                .mutate_canonical_player_like_cpp(|player| player.data().player_title)
                .unwrap(),
            42
        );
        let update_packet = send_rx.try_recv().expect("PlayerTitle values update");
        assert_eq!(
            u16::from_le_bytes([update_packet[0], update_packet[1]]),
            ServerOpcodes::UpdateObject as u16
        );
    }

    #[tokio::test]
    async fn unregister_all_addon_prefixes_preserves_filter_flag_like_cpp() {
        let (mut session, _send_rx) = make_session();
        session.registered_addon_prefixes = vec!["ABC".to_string()];
        session.filter_addon_messages = true;
        assert!(session.is_addon_registered_like_cpp("ABC"));

        session
            .handle_chat_unregister_all_addon_prefixes(WorldPacket::from_bytes(&[]))
            .await;

        assert!(session.registered_addon_prefixes.is_empty());
        assert!(session.filter_addon_messages);
        assert!(!session.is_addon_registered_like_cpp("ABC"));
    }

    fn save_cuf_profiles_packet(
        profiles: impl IntoIterator<Item = wow_packet::packets::misc::CufProfile>,
    ) -> WorldPacket {
        let profiles: Vec<_> = profiles.into_iter().collect();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::SaveCufProfiles as u16);
        pkt.write_uint32(profiles.len() as u32);
        for profile in profiles {
            pkt.write_bits(profile.profile_name.len() as u32, 7);
            for option in 0..wow_packet::packets::misc::CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
                pkt.write_bit(profile.bool_options & (1 << option) != 0);
            }
            pkt.write_uint16(profile.frame_height);
            pkt.write_uint16(profile.frame_width);
            pkt.write_uint8(profile.sort_by);
            pkt.write_uint8(profile.health_text);
            pkt.write_uint8(profile.top_point);
            pkt.write_uint8(profile.bottom_point);
            pkt.write_uint8(profile.left_point);
            pkt.write_uint16(profile.top_offset);
            pkt.write_uint16(profile.bottom_offset);
            pkt.write_uint16(profile.left_offset);
            pkt.write_string(&profile.profile_name);
        }
        WorldPacket::from_bytes(pkt.data())
    }

    fn cuf_profile(name: &str, frame_height: u16) -> wow_packet::packets::misc::CufProfile {
        wow_packet::packets::misc::CufProfile {
            profile_name: name.to_string(),
            frame_height,
            frame_width: 128,
            sort_by: 2,
            health_text: 3,
            top_point: 4,
            bottom_point: 5,
            left_point: 6,
            top_offset: 7,
            bottom_offset: 8,
            left_offset: 9,
            bool_options: (1 << 0) | (1 << 26),
        }
    }

    #[tokio::test]
    async fn save_cuf_profiles_replaces_and_clears_slots_like_cpp() {
        let (mut session, _send_rx) = make_session();
        assert!(session.represented_save_cuf_profiles_like_cpp(vec![
            cuf_profile("Old0", 10),
            cuf_profile("Old1", 11),
            cuf_profile("Old2", 12),
        ]));

        session
            .handle_save_cuf_profiles(save_cuf_profiles_packet([
                cuf_profile("Raid", 72),
                cuf_profile("Party", 64),
            ]))
            .await;

        let profiles = session.represented_cuf_profiles_like_cpp();
        assert_eq!(profiles[0].as_ref().unwrap().profile_name, "Raid");
        assert_eq!(profiles[0].as_ref().unwrap().frame_height, 72);
        assert_eq!(profiles[1].as_ref().unwrap().profile_name, "Party");
        assert_eq!(profiles[1].as_ref().unwrap().frame_height, 64);
        assert!(profiles[2].is_none());
        assert!(profiles[3].is_none());
        assert!(profiles[4].is_none());
    }

    #[tokio::test]
    async fn save_cuf_profiles_rejects_above_cpp_max_without_mutation() {
        let (mut session, _send_rx) = make_session();
        assert!(session.represented_save_cuf_profiles_like_cpp(vec![cuf_profile("Keep", 10)]));

        session
            .handle_save_cuf_profiles(save_cuf_profiles_packet([
                cuf_profile("A", 1),
                cuf_profile("B", 2),
                cuf_profile("C", 3),
                cuf_profile("D", 4),
                cuf_profile("E", 5),
                cuf_profile("F", 6),
            ]))
            .await;

        let profiles = session.represented_cuf_profiles_like_cpp();
        assert_eq!(profiles[0].as_ref().unwrap().profile_name, "Keep");
        assert!(profiles[1].is_none());
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
    async fn mount_special_anim_does_not_send_to_source_player_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 77);
        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::ZERO);

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::MountSpecialAnim as u16);
        pkt.write_uint32(2);
        pkt.write_int32(-3);
        pkt.write_int32(111);
        pkt.write_int32(222);

        session.handle_mount_special_anim(pkt).await;

        assert!(
            send_rx.try_recv().is_err(),
            "C++ MessageDistDeliverer never sends SendMessageToSet packets to i_source"
        );
    }

    #[tokio::test]
    async fn mount_special_anim_fanouts_to_visible_sessions_like_cpp() {
        let (mut source_session, source_send_rx) = make_session();
        let (mut visible_session, visible_send_rx) = make_session();
        let source_guid = ObjectGuid::create_player(1, 77);
        let visible_guid = ObjectGuid::create_player(1, 88);
        source_session.set_player_guid(Some(source_guid));
        source_session.set_player_map_position_like_cpp(571, Position::ZERO);
        visible_session.set_player_guid(Some(visible_guid));
        visible_session.set_player_map_position_like_cpp(571, Position::ZERO);
        visible_session.set_state(crate::session::SessionState::LoggedIn);
        visible_session
            .client_visible_guids_like_cpp
            .insert(source_guid);

        let registry = Arc::new(PlayerRegistry::default());
        let (source_command_tx, source_command_rx) = flume::bounded::<SessionCommand>(2);
        let source_info = broadcast_info_with_command_tx(source_command_tx);
        registry.insert(source_guid, source_info);
        let visible_command_tx = visible_session.session_command_tx();
        let visible_info = broadcast_info_with_command_tx(visible_command_tx);
        registry.insert(visible_guid, visible_info);
        source_session.set_player_registry(Arc::clone(&registry));
        visible_session.set_player_registry(registry);

        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::MountSpecialAnim as u16);
        pkt.write_uint32(2);
        pkt.write_int32(-3);
        pkt.write_int32(111);
        pkt.write_int32(222);

        source_session.handle_mount_special_anim(pkt).await;

        assert!(
            source_send_rx.try_recv().is_err(),
            "source session must not receive the packet directly"
        );
        assert!(
            source_command_rx.try_recv().is_err(),
            "source registry entry must be skipped like C++ player == i_source"
        );

        visible_session
            .process_represented_session_commands_like_cpp()
            .await;

        let bytes = visible_send_rx
            .try_recv()
            .expect("visible special mount anim");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::SpecialMountAnim as u16
        );
        assert_eq!(&bytes[2..18], &source_guid.to_raw_bytes());
        assert_eq!(
            u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]),
            2
        );
        assert_eq!(
            i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]),
            -3
        );
        assert_eq!(
            i32::from_le_bytes([bytes[26], bytes[27], bytes[28], bytes[29]]),
            111
        );
        assert_eq!(
            i32::from_le_bytes([bytes[30], bytes[31], bytes[32], bytes[33]]),
            222
        );
        assert_eq!(bytes.len(), 34);
    }

    #[tokio::test]
    async fn mount_clear_fanfare_stub_sends_no_response_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_mount_clear_fanfare(WorldPacket::new_empty())
            .await;

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

    fn battle_pet_delete_pet_packet(pet_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(0xBADD);
        pkt.write_packed_guid(&pet_guid);
        pkt
    }

    fn cage_battle_pet_packet(pet_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(0xBADD);
        pkt.write_packed_guid(&pet_guid);
        pkt
    }

    fn battle_pet_modify_name_packet(
        pet_guid: ObjectGuid,
        name: &str,
        declined_names: Option<[&str; 5]>,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(0xBADD);
        pkt.write_packed_guid(&pet_guid);
        pkt.write_bits(name.len() as u32, 7);
        pkt.write_bit(declined_names.is_some());
        if let Some(declined_names) = declined_names {
            for declined_name in declined_names {
                pkt.write_bits(declined_name.len() as u32, 7);
            }
            for declined_name in declined_names {
                pkt.write_string(declined_name);
            }
        }
        pkt.write_string(name);
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
                name_timestamp: 0,
                declined_names: None,
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
    async fn battle_pet_delete_pet_requires_lock_and_marks_removed_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x2241);
        session.add_represented_battle_pet_like_cpp(
            pet_guid,
            0x01,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        );

        session
            .handle_battle_pet_delete_pet_represented_like_cpp(battle_pet_delete_pet_packet(
                pet_guid,
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
            .handle_battle_pet_delete_pet_represented_like_cpp(battle_pet_delete_pet_packet(
                pet_guid,
            ))
            .await;
        assert_eq!(
            session
                .represented_battle_pet_like_cpp(pet_guid)
                .expect("represented pet row remains until DB save")
                .save_info,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Removed
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_delete_pet_ignores_unknown_pet_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x2242);

        session
            .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
            .await;
        let _ = send_rx.try_recv().expect("lock acquired packet");
        let _ = send_rx.try_recv().expect("battle pet journal packet");

        session
            .handle_battle_pet_delete_pet_represented_like_cpp(battle_pet_delete_pet_packet(
                pet_guid,
            ))
            .await;

        assert!(session.represented_battle_pet_like_cpp(pet_guid).is_none());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn cage_battle_pet_requires_lock_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x2243);
        session.add_represented_battle_pet_packet_info_like_cpp(
            pet_guid,
            crate::session::RepresentedBattlePetDataLikeCpp {
                species: 11,
                creature_id: 22,
                display_id: 33,
                breed: 44,
                level: 17,
                exp: 0,
                flags: 0,
                power: 0,
                health: 100,
                max_health: 100,
                speed: 0,
                quality: 3,
                owner_info: None,
                name: String::new(),
                name_timestamp: 0,
                declined_names: None,
                save_info: crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            },
        );

        session
            .handle_cage_battle_pet_represented_like_cpp(cage_battle_pet_packet(pet_guid))
            .await;

        assert_eq!(
            session
                .represented_battle_pet_like_cpp(pet_guid)
                .expect("pet remains")
                .save_info,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged
        );
        assert!(
            session
                .represented_battle_pet_cage_items_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn cage_battle_pet_handler_delegates_to_represented_manager_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x2244);
        let expected_item = crate::session::RepresentedBattlePetCageItemLikeCpp {
            item_id: crate::session::BATTLE_PET_CAGE_ITEM_ID_LIKE_CPP,
            species_id: 11,
            breed_data: 44 | (3 << 24),
            level: 17,
            display_id: 33,
        };

        session.add_represented_battle_pet_packet_info_like_cpp(
            pet_guid,
            crate::session::RepresentedBattlePetDataLikeCpp {
                species: 11,
                creature_id: 22,
                display_id: 33,
                breed: 44,
                level: 17,
                exp: 0,
                flags: 0,
                power: 0,
                health: 100,
                max_health: 100,
                speed: 0,
                quality: 3,
                owner_info: None,
                name: String::new(),
                name_timestamp: 0,
                declined_names: None,
                save_info: crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            },
        );
        assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

        session
            .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
            .await;
        let _ = send_rx.try_recv().expect("lock acquired packet");
        let _ = send_rx.try_recv().expect("battle pet journal packet");

        session
            .handle_cage_battle_pet_represented_like_cpp(cage_battle_pet_packet(pet_guid))
            .await;

        assert_eq!(
            session.represented_battle_pet_cage_items_like_cpp(),
            &[expected_item]
        );
        assert_eq!(
            session
                .represented_battle_pet_like_cpp(pet_guid)
                .expect("removed pet row remains represented")
                .save_info,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Removed
        );
        assert_eq!(
            session.represented_summoned_battle_pet_guid_like_cpp(),
            None
        );

        let packet_bytes = send_rx.try_recv().expect("battle pet deleted packet");
        let mut packet = wow_packet::WorldPacket::from_bytes(&packet_bytes);
        assert_eq!(
            packet.read_uint16().expect("opcode"),
            ServerOpcodes::BattlePetDeleted as u16
        );
        assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
        assert_eq!(packet.remaining(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_modify_name_requires_lock_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x2245);
        session.add_represented_battle_pet_like_cpp(
            pet_guid,
            0x01,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        );

        session
            .handle_battle_pet_modify_name_represented_like_cpp(battle_pet_modify_name_packet(
                pet_guid, "Misha", None,
            ))
            .await;

        let pet = session
            .represented_battle_pet_like_cpp(pet_guid)
            .expect("pet remains");
        assert_eq!(pet.name, "");
        assert_eq!(pet.name_timestamp, 0);
        assert_eq!(pet.declined_names, None);
        assert_eq!(
            pet.save_info,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battle_pet_modify_name_handler_delegates_to_manager_like_cpp() {
        let (mut session, send_rx) = make_session();
        let pet_guid = ObjectGuid::new(0, 0x2246);
        let declined = ["Alpha", "Betas", "Gamma", "Delta", "Epsil"];
        session.add_represented_battle_pet_like_cpp(
            pet_guid,
            0x01,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        );

        session
            .handle_battle_pet_request_journal_lock(battle_pet_request_journal_lock_packet())
            .await;
        let _ = send_rx.try_recv().expect("lock acquired packet");
        let _ = send_rx.try_recv().expect("battle pet journal packet");
        let before = i64::try_from(GameTime::now().as_secs()).unwrap_or(i64::MAX);

        session
            .handle_battle_pet_modify_name_represented_like_cpp(battle_pet_modify_name_packet(
                pet_guid,
                "Misha",
                Some(declined),
            ))
            .await;

        let after = i64::try_from(GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let pet = session
            .represented_battle_pet_like_cpp(pet_guid)
            .expect("pet renamed");
        assert_eq!(pet.name, "Misha");
        assert!((before..=after).contains(&pet.name_timestamp));
        assert_eq!(
            pet.declined_names.as_ref().expect("declined names").names,
            declined.map(str::to_string)
        );
        assert_eq!(
            pet.save_info,
            crate::session::RepresentedBattlePetSaveInfoLikeCpp::Changed
        );
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
    async fn query_battle_pet_name_non_summon_keeps_zero_response_like_cpp() {
        let (mut session, send_rx) = make_session();
        let battle_pet_id = ObjectGuid::new(0, 0x22f);
        let unit_guid = ObjectGuid::new(0, 0x230);
        session.set_represented_battle_pet_query_companion_like_cpp(
            unit_guid,
            crate::session::RepresentedBattlePetQueryCompanionLikeCpp {
                creature_id: 777,
                name_timestamp: 1234,
                is_summon: false,
                owner_is_player: true,
            },
        );

        session
            .handle_query_battle_pet_name(query_battle_pet_name_packet(battle_pet_id, unit_guid))
            .await;

        let bytes = send_rx.try_recv().expect("query battle pet name response");
        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
        assert_eq!(body.read_int32().unwrap(), 0);
        assert_eq!(body.read_int64().unwrap(), 0);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.remaining(), 0);
    }

    #[tokio::test]
    async fn query_battle_pet_name_preserves_summon_identity_until_allow_gate_like_cpp() {
        let (mut session, send_rx) = make_session();
        let battle_pet_id = ObjectGuid::new(0, 0x231);
        let unit_guid = ObjectGuid::new(0, 0x232);
        session.set_represented_battle_pet_query_companion_like_cpp(
            unit_guid,
            crate::session::RepresentedBattlePetQueryCompanionLikeCpp {
                creature_id: 777,
                name_timestamp: 1234,
                is_summon: true,
                owner_is_player: false,
            },
        );

        session
            .handle_query_battle_pet_name(query_battle_pet_name_packet(battle_pet_id, unit_guid))
            .await;

        let bytes = send_rx.try_recv().expect("query battle pet name response");
        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
        assert_eq!(body.read_int32().unwrap(), 777);
        assert_eq!(body.read_int64().unwrap(), 1234);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.remaining(), 0);
    }

    #[tokio::test]
    async fn query_battle_pet_name_allows_known_named_player_pet_like_cpp() {
        let (mut session, send_rx) = make_session();
        let battle_pet_id = ObjectGuid::new(0, 0x233);
        let unit_guid = ObjectGuid::new(0, 0x234);
        let declined = wow_packet::packets::misc::DeclinedNamesLikeCpp {
            names: ["Alpha", "Betas", "Gamma", "Delta", "Epsil"].map(str::to_string),
        };
        session.set_represented_battle_pet_query_companion_like_cpp(
            unit_guid,
            crate::session::RepresentedBattlePetQueryCompanionLikeCpp {
                creature_id: 777,
                name_timestamp: 1234,
                is_summon: true,
                owner_is_player: true,
            },
        );
        session.add_represented_battle_pet_packet_info_like_cpp(
            battle_pet_id,
            crate::session::RepresentedBattlePetDataLikeCpp {
                species: 11,
                creature_id: 777,
                display_id: 33,
                breed: 44,
                level: 17,
                exp: 0,
                flags: 0,
                power: 0,
                health: 100,
                max_health: 100,
                speed: 0,
                quality: 3,
                owner_info: None,
                name: "Misha".to_string(),
                name_timestamp: 1234,
                declined_names: Some(declined.clone()),
                save_info: crate::session::RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            },
        );

        session
            .handle_query_battle_pet_name(query_battle_pet_name_packet(battle_pet_id, unit_guid))
            .await;

        let bytes = send_rx.try_recv().expect("query battle pet name response");
        let mut body = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
        assert_eq!(body.read_int32().unwrap(), 777);
        assert_eq!(body.read_int64().unwrap(), 1234);
        assert!(body.read_bit().unwrap());
        assert_eq!(body.read_bits(8).unwrap(), 5);
        assert!(body.read_bit().unwrap());
        let mut declined_lengths = [0usize; 5];
        for length in &mut declined_lengths {
            *length = body.read_bits(7).unwrap() as usize;
        }
        let declined_names = declined_lengths
            .iter()
            .map(|length| body.read_string(*length).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(declined_names, declined.names);
        assert_eq!(body.read_string(5).unwrap(), "Misha");
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
    async fn lfg_list_get_status_sends_removed_from_queue_like_cpp_without_lfg_state() {
        let (mut session, send_rx) = make_session();

        session
            .handle_lfg_list_get_status(WorldPacket::new_empty())
            .await;

        let bytes = send_rx.try_recv().expect("LFG update status packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::LfgUpdateStatus as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_uint32().unwrap(), 0); // Ticket.Id
        assert_eq!(pkt.read_uint32().unwrap(), 0); // Ticket.Type
        assert_eq!(pkt.read_int64().unwrap(), 0); // Ticket.Time
        assert!(!pkt.has_bit().unwrap()); // Ticket.Unknown925
        assert_eq!(
            pkt.read_uint8().unwrap(),
            wow_packet::packets::misc::LFG_QUEUE_DUNGEON_LIKE_CPP
        );
        assert_eq!(
            pkt.read_uint8().unwrap(),
            wow_packet::packets::misc::LFG_UPDATE_TYPE_REMOVED_FROM_QUEUE_LIKE_CPP
        );
        assert_eq!(pkt.read_uint32().unwrap(), 0); // Slots.Count
        assert_eq!(pkt.read_uint8().unwrap(), 0); // RequestedRoles
        assert_eq!(pkt.read_uint32().unwrap(), 0); // SuspendedPlayers.Count
        assert_eq!(pkt.read_uint32().unwrap(), 0); // QueueMapID
        assert!(!pkt.has_bit().unwrap()); // IsParty
        assert!(pkt.has_bit().unwrap()); // NotifyUI
        assert!(!pkt.has_bit().unwrap()); // Joined
        assert!(!pkt.has_bit().unwrap()); // LfgJoined
        assert!(!pkt.has_bit().unwrap()); // Queued
        assert!(!pkt.has_bit().unwrap()); // Unused
    }

    #[tokio::test]
    async fn request_lfg_list_blacklist_sends_empty_list_like_cpp_without_locks() {
        let (mut session, send_rx) = make_session();

        session
            .handle_request_lfg_list_blacklist(WorldPacket::new_empty())
            .await;

        let bytes = send_rx.try_recv().expect("LFG blacklist packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::LfgListUpdateBlacklist as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
    }

    #[tokio::test]
    async fn df_get_system_info_player_sends_empty_player_info_like_cpp_without_lfg_mgr() {
        let (mut session, send_rx) = make_session();
        let mut request = WorldPacket::new_empty();
        request.write_bit(true); // Player
        request.write_bit(false); // PartyIndex.HasValue
        request.flush_bits();

        session.handle_df_get_system_info(request).await;

        let bytes = send_rx.try_recv().expect("LFG player info packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::LfgPlayerInfo as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0); // Dungeon.Count
        assert!(!pkt.has_bit().unwrap()); // BlackList.PlayerGuid.HasValue
        assert_eq!(pkt.read_uint32().unwrap(), 0); // BlackList.Slot.Count
    }

    #[tokio::test]
    async fn df_get_system_info_party_without_group_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut request = WorldPacket::new_empty();
        request.write_bit(false); // Player
        request.write_bit(false); // PartyIndex.HasValue
        request.flush_bits();

        session.handle_df_get_system_info(request).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn df_get_join_status_without_active_lfg_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_df_get_join_status(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
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
            .handle_support_ticket_submit_complaint(support_ticket_submit_complaint_packet(
                "report",
            ))
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

    #[test]
    fn bug_report_statement_binds_text_then_diag_info_like_cpp() {
        let report = BugReport {
            report_type: 1,
            text: "client bug".to_string(),
            diag_info: "diag blob".to_string(),
        };

        let stmt = bug_report_insert_statement_like_cpp(&report);
        assert_eq!(stmt.sql(), CharStatements::INS_BUG_REPORT.sql());
        assert_eq!(
            stmt.params(),
            &[
                SqlParam::String("client bug".to_string()),
                SqlParam::String("diag blob".to_string())
            ]
        );
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

    #[tokio::test]
    async fn stand_state_change_accepts_only_cpp_states_like_cpp() {
        let (mut session, send_rx) = make_session();

        for state in [
            UnitStandStateType::Stand,
            UnitStandStateType::Sit,
            UnitStandStateType::Sleep,
            UnitStandStateType::Kneel,
        ] {
            session
                .handle_stand_state_change(stand_state_change_packet(state as u32))
                .await;
            assert_eq!(session.player_stand_state_like_cpp(), state);
        }

        session
            .handle_stand_state_change(stand_state_change_packet(
                UnitStandStateType::SitChair as u32,
            ))
            .await;
        assert_eq!(
            session.player_stand_state_like_cpp(),
            UnitStandStateType::Kneel
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn stand_state_change_handler_metadata_matches_cpp() {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == ClientOpcodes::StandStateChange)
            .expect("StandStateChange handler entry");

        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
        assert_eq!(entry.handler_name, "handle_stand_state_change");
    }

    #[tokio::test]
    async fn calendar_get_num_pending_sends_zero_pending_like_cpp_without_calendar_mgr() {
        let (mut session, send_rx) = make_session();

        session
            .handle_calendar_get_num_pending(WorldPacket::new_empty())
            .await;

        let bytes = send_rx.try_recv().expect("calendar pending packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::CalendarSendNumPending as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
    }

    #[tokio::test]
    async fn guild_bank_remaining_withdraw_money_without_guild_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_guild_bank_remaining_withdraw_money_query(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn calendar_get_sends_empty_calendar_like_cpp_without_calendar_mgr() {
        let (mut session, send_rx) = make_session();

        session.handle_calendar_get(WorldPacket::new_empty()).await;

        let bytes = send_rx.try_recv().expect("calendar send calendar packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::CalendarSendCalendar as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        let _server_time = pkt.read_uint32().unwrap();
        assert_eq!(pkt.read_uint32().unwrap(), 0); // Invites.Count
        assert_eq!(pkt.read_uint32().unwrap(), 0); // Events.Count
        assert_eq!(pkt.read_uint32().unwrap(), 0); // RaidLockouts.Count
    }

    #[tokio::test]
    async fn loading_screen_notify_is_silent_like_cpp_todo_handler() {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(571);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();

        session.handle_loading_screen_notify(pkt).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn violence_level_is_silent_like_cpp_todo_handler() {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(2);
        pkt.reset_read();

        session.handle_violence_level(pkt).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn override_screen_flash_is_handle_null_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_override_screen_flash(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn queued_messages_end_is_handle_null_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_queued_messages_end(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn guild_set_achievement_tracking_without_guild_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(2);
        pkt.write_uint32(100);
        pkt.write_uint32(200);
        pkt.reset_read();

        session.handle_guild_set_achievement_tracking(pkt).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn close_interaction_matching_source_clears_gossip_like_cpp() {
        let (mut session, send_rx) = make_session();
        let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 42);
        session.gossip_source_guid = Some(source_guid);
        session
            .gossip_options
            .push(crate::session::GossipOptionInfo {
                gossip_option_id: 1,
                option_npc: 2,
                action_menu_id: 3,
            });
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&source_guid);
        pkt.reset_read();

        session.handle_close_interaction(pkt).await;

        assert!(session.gossip_source_guid.is_none());
        assert!(session.gossip_options.is_empty());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn close_interaction_nonmatching_source_preserves_gossip_like_cpp() {
        let (mut session, send_rx) = make_session();
        let active_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 43);
        let other_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 44);
        session.gossip_source_guid = Some(active_guid);
        session
            .gossip_options
            .push(crate::session::GossipOptionInfo {
                gossip_option_id: 1,
                option_npc: 2,
                action_menu_id: 3,
            });
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&other_guid);
        pkt.reset_read();

        session.handle_close_interaction(pkt).await;

        assert_eq!(session.gossip_source_guid, Some(active_guid));
        assert_eq!(session.gossip_options.len(), 1);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_join_channel_invalid_custom_name_sends_notice_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(0);
        pkt.write_bit(false);
        pkt.write_bit(false);
        pkt.write_bits(4, 7);
        pkt.write_bits(0, 7);
        pkt.write_string("1bad");
        pkt.reset_read();

        session.handle_chat_join_channel(pkt).await;

        let bytes = send_rx.try_recv().expect("channel notify packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::ChannelNotify as u16
        );
        let mut payload = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(
            payload.read_bits(6).unwrap() as u8,
            wow_packet::packets::chat::CHAT_INVALID_NAME_NOTICE_LIKE_CPP
        );
        assert_eq!(payload.read_bits(7).unwrap(), 4);
    }

    #[tokio::test]
    async fn chat_join_channel_too_long_custom_name_sends_notice_like_cpp() {
        let (mut session, send_rx) = make_session();
        let channel_name = "A".repeat(MAX_CHANNEL_NAME_STR_LIKE_CPP + 1);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(0);
        pkt.write_bit(false);
        pkt.write_bit(false);
        pkt.write_bits(channel_name.len() as u32, 7);
        pkt.write_bits(0, 7);
        pkt.write_string(&channel_name);
        pkt.reset_read();

        session.handle_chat_join_channel(pkt).await;

        let bytes = send_rx.try_recv().expect("channel notify packet");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::ChannelNotify as u16
        );
        let mut payload = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(
            payload.read_bits(6).unwrap() as u8,
            wow_packet::packets::chat::CHAT_INVALID_NAME_NOTICE_LIKE_CPP
        );
        assert_eq!(payload.read_bits(7).unwrap(), channel_name.len() as u32);
    }

    #[test]
    fn chat_join_channel_precheck_rejects_too_long_password_like_cpp() {
        let request = JoinChannel {
            chat_channel_id: 0,
            create_voice_session: false,
            internal: false,
            channel_name: "Valid".to_string(),
            password: "p".repeat(MAX_CHANNEL_PASS_STR_LIKE_CPP + 1),
        };

        assert_eq!(
            join_channel_custom_precheck_like_cpp(&request),
            JoinChannelPrecheckLikeCpp::PasswordTooLong
        );
    }

    #[tokio::test]
    async fn chat_leave_channel_empty_request_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(0);
        pkt.write_bits(0, 7);
        pkt.reset_read();

        session.handle_chat_leave_channel(pkt).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_channel_command_without_channel_mgr_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(5, 7);
        pkt.write_string("Trade");
        pkt.reset_read();

        session.handle_chat_channel_command(pkt).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_channel_player_command_too_long_name_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_name = "P".repeat(MAX_CHANNEL_NAME_STR_LIKE_CPP);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(5, 7);
        pkt.write_bits(player_name.len() as u32, 9);
        pkt.write_string("Trade");
        pkt.write_string(&player_name);
        pkt.reset_read();

        session.handle_chat_channel_player_command(pkt).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chat_channel_password_without_channel_mgr_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bits(5, 7);
        pkt.write_bits(4, 7);
        pkt.write_string("Trade");
        pkt.write_string("pass");
        pkt.reset_read();

        session.handle_chat_channel_password(pkt).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn request_pvp_rewards_is_silent_like_cpp_commented_send() {
        let (mut session, send_rx) = make_session();

        session
            .handle_request_pvp_rewards(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn request_battlefield_status_without_queues_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_request_battlefield_status(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battlefield_leave_records_request_when_not_in_combat_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_player_battleground_type_id_like_cpp(3);
        session.set_represented_battleground_status_like_cpp(Some(2));

        session
            .handle_battlefield_leave(WorldPacket::new_empty())
            .await;

        assert_eq!(
            session.represented_battleground_leave_requests_like_cpp(),
            1
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battlefield_leave_rejects_in_combat_active_battleground_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_player_battleground_type_id_like_cpp(3);
        session.set_represented_battleground_status_like_cpp(Some(2));
        session.in_combat = true;

        session
            .handle_battlefield_leave(WorldPacket::new_empty())
            .await;

        assert_eq!(
            session.represented_battleground_leave_requests_like_cpp(),
            0
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn battlefield_leave_allows_wait_leave_even_in_combat_like_cpp() {
        let (mut session, send_rx) = make_session();
        session.set_player_battleground_type_id_like_cpp(3);
        session.set_represented_battleground_status_like_cpp(Some(4));
        session.in_combat = true;

        session
            .handle_battlefield_leave(WorldPacket::new_empty())
            .await;

        assert_eq!(
            session.represented_battleground_leave_requests_like_cpp(),
            1
        );
        assert!(send_rx.try_recv().is_err());
    }

    fn accept_wargame_invite_packet(inviter_name: &str) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_string(inviter_name);
        pkt.write_uint8(0);
        pkt.reset_read();
        pkt
    }

    #[tokio::test]
    async fn accept_wargame_invite_missing_inviter_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 100);
        let group_registry = Arc::new(GroupRegistry::default());
        let player_registry = Arc::new(PlayerRegistry::default());
        let group = GroupInfo::new(player_guid);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        session.set_player_guid(Some(player_guid));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_accept_wargame_invite(accept_wargame_invite_packet("Missing"))
            .await;

        assert!(
            session
                .represented_wargame_invite_acceptances_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn accept_wargame_invite_records_ready_to_queue_when_groups_match_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 100);
        let player_ally_guid = ObjectGuid::create_player(1, 101);
        let inviter_guid = ObjectGuid::create_player(1, 200);
        let inviter_ally_guid = ObjectGuid::create_player(1, 201);
        let group_registry = Arc::new(GroupRegistry::default());
        let player_registry = Arc::new(PlayerRegistry::default());

        let mut player_group = GroupInfo::new(player_guid);
        player_group.members.push(player_ally_guid);
        let player_group_guid = player_group.group_guid;
        group_registry.insert(player_group_guid, player_group);

        let mut inviter_group = GroupInfo::new(inviter_guid);
        inviter_group.members.push(inviter_ally_guid);
        let inviter_group_guid = inviter_group.group_guid;
        group_registry.insert(inviter_group_guid, inviter_group);

        let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(4);
        let mut inviter_info = broadcast_info_with_command_tx(command_tx);
        inviter_info.player_name = "Inviter".to_string();
        player_registry.insert(inviter_guid, inviter_info);

        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_name_like_cpp("Player".to_string());
        session.group_guid = Some(player_group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

        session
            .handle_accept_wargame_invite(accept_wargame_invite_packet("inviter"))
            .await;

        assert_eq!(
            session.represented_wargame_invite_acceptances_like_cpp(),
            &[crate::session::RepresentedWargameInviteAcceptanceLikeCpp {
                inviter_name: "inviter".to_string(),
                inviter_guid,
                player_group_guid,
                inviter_group_guid,
                group_size: 2,
            }]
        );
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn arena_team_roster_unknown_team_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let mut request = WorldPacket::new_empty();
        request.write_uint32(1234);

        session.handle_arena_team_roster(request).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn conquest_formula_constants_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_request_conquest_formula_constants(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn get_account_character_list_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_get_account_character_list(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn get_account_notifications_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_get_account_notifications(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn report_client_variables_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_report_client_variables(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn report_enabled_addons_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_report_enabled_addons(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn report_frozen_while_loading_map_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_report_frozen_while_loading_map(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn log_streaming_error_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_log_streaming_error(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn complete_movie_clears_active_movie_and_records_script_hook_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_complete_movie(WorldPacket::new_empty())
            .await;
        assert_eq!(session.represented_movie_like_cpp(), None);
        assert!(
            session
                .represented_movie_complete_events_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());

        session.set_represented_movie_like_cpp_for_test(Some(177));
        session
            .handle_complete_movie(WorldPacket::new_empty())
            .await;

        assert_eq!(session.represented_movie_like_cpp(), None);
        assert_eq!(session.represented_movie_complete_events_like_cpp(), &[177]);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn complete_cinematic_clears_active_cinematic_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_complete_cinematic(WorldPacket::new_empty())
            .await;
        assert_eq!(session.represented_cinematic_like_cpp(), None);
        assert!(
            session
                .represented_cinematic_end_events_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());

        session.set_represented_cinematic_like_cpp_for_test(Some(444));
        session
            .handle_complete_cinematic(WorldPacket::new_empty())
            .await;

        assert_eq!(session.represented_cinematic_like_cpp(), None);
        assert_eq!(session.represented_cinematic_end_events_like_cpp(), &[444]);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn next_cinematic_camera_advances_active_camera_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_next_cinematic_camera(WorldPacket::new_empty())
            .await;
        assert!(
            session
                .represented_cinematic_next_camera_events_like_cpp()
                .is_empty()
        );
        assert!(send_rx.try_recv().is_err());

        session.set_cinematic_sequences_store(Arc::new(
            wow_data::CinematicSequencesStore::from_entries([wow_data::CinematicSequencesEntry {
                id: 444,
                sound_id: 0,
                camera: [11, 22, 0, 33, 0, 0, 0, 0],
            }]),
        ));
        assert!(session.use_represented_gameobject_camera_like_cpp(
            ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 8),
            ObjectGuid::create_player(1, 99),
            wow_entities::CameraUseSource {
                cinematic_id: 444,
                event_id: 0,
            },
        ));
        let _ = send_rx.try_recv().expect("TriggerCinematic sent");
        assert_eq!(session.represented_cinematic_like_cpp(), Some(444));
        assert_eq!(session.represented_cinematic_camera_index_like_cpp(), -1);

        session
            .handle_next_cinematic_camera(WorldPacket::new_empty())
            .await;
        session
            .handle_next_cinematic_camera(WorldPacket::new_empty())
            .await;
        session
            .handle_next_cinematic_camera(WorldPacket::new_empty())
            .await;
        session
            .handle_next_cinematic_camera(WorldPacket::new_empty())
            .await;
        assert_eq!(
            session.represented_cinematic_next_camera_events_like_cpp(),
            &[11, 22, 33]
        );
        assert_eq!(session.represented_cinematic_camera_index_like_cpp(), 3);
        assert!(send_rx.try_recv().is_err());

        for _ in 0..8 {
            session
                .handle_next_cinematic_camera(WorldPacket::new_empty())
                .await;
        }
        assert_eq!(
            session.represented_cinematic_next_camera_events_like_cpp(),
            &[11, 22, 33]
        );
    }

    #[tokio::test]
    async fn additional_status_unhandled_null_family_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_logout_instant(WorldPacket::new_empty())
            .await;
        session
            .handle_spawn_tracking_update(WorldPacket::new_empty())
            .await;
        session
            .handle_time_adjustment_response(WorldPacket::new_empty())
            .await;
        session
            .handle_update_area_trigger_visual(WorldPacket::new_empty())
            .await;
        session
            .handle_update_spell_visual(WorldPacket::new_empty())
            .await;
        session.handle_used_follow(WorldPacket::new_empty()).await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn unhandled_movement_null_family_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        for _ in 0..19 {
            session
                .handle_unhandled_client_null_like_cpp(WorldPacket::new_empty())
                .await;
        }

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn report_keybinding_execution_counts_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_report_keybinding_execution_counts(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn query_countdown_timer_is_silent_like_cpp_handle_null() {
        let (mut session, send_rx) = make_session();

        session
            .handle_request_countdown_timer(WorldPacket::new_empty())
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn commerce_token_get_log_echoes_request_and_empty_success_like_cpp_todo_handler() {
        let (mut session, send_rx) = make_session();
        let mut request = WorldPacket::new_empty();
        request.write_uint32(0x1122_3344);

        session.handle_commerce_token_get_log(request).await;

        let bytes = send_rx.try_recv().expect("commerce token get log response");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::CommerceTokenGetLogResponse as u16
        );

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint32().unwrap(), 0x1122_3344);
        assert_eq!(
            pkt.read_uint32().unwrap(),
            wow_packet::packets::misc::TOKEN_RESULT_SUCCESS_LIKE_CPP
        );
        assert_eq!(pkt.read_uint32().unwrap(), 0);
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

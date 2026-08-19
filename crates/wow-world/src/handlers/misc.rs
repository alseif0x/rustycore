// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for miscellaneous client opcodes:
//! SetSelection, AreaTrigger, RequestCemeteryList,
//! TaxiNodeStatusQuery, ChatJoinChannel.

use tracing::{debug, info, warn};
use wow_constants::unit::{NPCFlags1, Team};
use wow_constants::{
    ClientOpcodes, ConditionSourceType, ConditionType, InventoryResult, ItemExtendedCostFlags,
    SpellCastResult, UnitStandStateType,
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
    GAMEOBJECT_TYPE_UI_LINK, GameObjectTemplateData, MAX_GAMEOBJECT_DATA, MAX_MONEY_AMOUNT,
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
use wow_packet::packets::gossip::Hello;
use wow_packet::packets::instance::{
    InstanceInfo, InstanceLockInfo, InstanceLockResponse, InstanceReset, InstanceResetFailed,
    InstanceSaveCreated, PendingRaidLock,
};
use wow_packet::packets::item::{
    GetItemPurchaseData, InventoryChangeFailure, ItemPurchaseContents, ItemPurchaseRefundCurrency,
    ItemPurchaseRefundItem, SetItemPurchaseData,
};
use wow_packet::packets::loot::{LOOT_TYPE_FISHING_JUNK_LIKE_CPP, LOOT_TYPE_FISHING_LIKE_CPP};
use wow_packet::packets::misc::{
    AcceptGuildInvite, AcceptTrade, AcceptWargameInvite, ActivateTaxi, ActivateTaxiReply, AddToy,
    AddonList, ArenaTeamAccept, ArenaTeamDecline, ArenaTeamDisband, ArenaTeamLeader,
    ArenaTeamLeave, ArenaTeamRemove, ArenaTeamRoster, AuctionPlaceBid, AuctionRemoveItem,
    AuctionReplicateItems, AuctionSellItem, AuctionableTokenSell,
    AuctionableTokenSellAtMarketPrice, AutoGuildBankItem, AutoStoreGuildBankItem,
    BattlePetClearFanfare, BattlePetDeletePet, BattlePetModifyName, BattlePetRequestJournal,
    BattlePetSetBattleSlot, BattlePetSetFlags, BattlePetSummon, BattlePetUpdateNotify,
    BattlefieldLeave, BattlefieldListRequest, BattlefieldPort, BattlemasterJoin,
    BattlemasterJoinArena, BattlemasterJoinSkirmish, BeginTrade, BugReport, BusyTrade,
    CageBattlePet, CalendarAddEvent, CalendarCommandResult, CalendarCommunityInvite,
    CalendarComplain, CalendarCopyEvent, CalendarEventSignUp, CalendarGetEvent, CalendarInvite,
    CalendarModeratorStatusQuery, CalendarRaidLockoutAdded, CalendarRaidLockoutUpdated,
    CalendarRemoveEvent, CalendarRemoveInvite, CalendarRsvp, CalendarSendCalendar,
    CalendarSendNumPending, CalendarStatus, CalendarUpdateEvent, CanDuel, ClearTradeItem,
    CloseInteraction, CommerceTokenGetLog, CommerceTokenGetLogResponse, Complaint, ComplaintResult,
    DeclineGuildInvites, DeclinePetition, DfGetJoinStatus, DfGetSystemInfo, DuelResponse,
    ERR_TAXITOOFARAWAY_LIKE_CPP, FarSight, GmTicketAcknowledgeSurvey, GmTicketCaseStatus,
    GmTicketSystemStatus, GuildBankActivate, GuildBankBuyTab, GuildBankDepositMoney,
    GuildBankLogQuery, GuildBankQueryTab, GuildBankSetTabText, GuildBankTextQuery,
    GuildBankUpdateTab, GuildBankWithdrawMoney, GuildCommandResult, GuildSetAchievementTracking,
    IgnoreTrade, LfgBlackList, LfgListBlacklist, LfgListBlacklistEntry, LfgPlayerDungeonInfo,
    LfgPlayerInfo, LfgPlayerQuestRewardCurrency, LfgPlayerQuestRewardItem, LfgUpdateStatus,
    LoadingScreenNotify, MAX_ACCOUNT_DATA_SIZE_LIKE_CPP, MailNextTimeEntry,
    MailQueryNextTimeResult, MountSetFavorite, MountSpecial, NUM_ACCOUNT_DATA_TYPES,
    ObjectUpdateFailed, ObjectUpdateRescued, PortGraveyard, QueryArenaTeam, QueryBattlePetName,
    QueryBattlePetNameResponse, QueryPetition, QueryPetitionResponse, RatedPvpInfo, ReclaimCorpse,
    RepopRequest, RequestAccountData, RequestBattlefieldStatus, RequestCemeteryListResponse,
    ResurrectResponse, SaveCufProfiles, SetAdvancedCombatLogging, SetCurrencyFlags,
    SetDifficultyId, SetDungeonDifficulty, SetPvp, SetRaidDifficulty, SetSavedInstanceExtend,
    SetTaxiBenchmarkMode, SetTradeGold, SetTradeItem, SetTradeSpell, SignPetition,
    SpecialMountAnim, StandStateChange, SubmitUserFeedback, SupportTicketSubmitBug,
    SupportTicketSubmitComplaint, SupportTicketSubmitSuggestion, TRADE_STATUS_CANCELLED_LIKE_CPP,
    TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP, TaxiNodeStatusPkt, ToggleDifficulty, TogglePvp,
    ToyClearFanfare, TutorialSetFlag, UnacceptTrade, UpdateAccountData, UseToy,
    UserClientUpdateAccountData, ViolenceLevel, compress_account_data_like_cpp,
    decompress_account_data_like_cpp,
};
use wow_packet::packets::pet::DismissCritter;
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
    RepresentedAuctionPlaceBidLikeCpp, RepresentedAuctionRemoveItemLikeCpp,
    RepresentedAuctionReplicateRequestLikeCpp, RepresentedAuctionSellItemLikeCpp,
    RepresentedGameObjectAccessLikeCpp, RepresentedGameObjectUseEffect, SpellCastMetadata,
    TRADE_STATUS_PLAYER_BUSY_LIKE_CPP,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepresentedInstanceResetMethodLikeCpp {
    Manual,
    OnChangeDifficulty,
}

fn represented_gameobject_icon_allows_interaction_like_cpp(icon_name: &str) -> bool {
    // C++ `Player::GetGameObjectIfCanInteractWith` rejects exactly this
    // template sentinel before applying the distance check.
    icon_name != "Point"
}

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
        opcode: ClientOpcodes::RepopRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_repop_request",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReclaimCorpse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_reclaim_corpse",
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
        opcode: ClientOpcodes::SuspendTokenResponse,
        status: SessionStatus::Transfer,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_suspend_token_response",
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
        opcode: ClientOpcodes::SetDifficultyId,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_difficulty_id",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ToggleDifficulty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_toggle_difficulty",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetDungeonDifficulty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_dungeon_difficulty",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetRaidDifficulty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_raid_difficulty",
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
        opcode: ClientOpcodes::Tutorial,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_tutorial",
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
        opcode: ClientOpcodes::BattlemasterHello,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_hello",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlefieldList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlefield_list",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoin,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoinArena,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join_arena",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlemasterJoinSkirmish,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlemaster_join_skirmish",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlefieldPort,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battlefield_port",
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
        opcode: ClientOpcodes::SetPvp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_pvp",
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
        opcode: ClientOpcodes::CalendarComplain,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_complain",
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
        opcode: ClientOpcodes::GuildBankActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_activate",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankQueryTab,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_query_tab",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankBuyTab,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_buy_tab",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankUpdateTab,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_update_tab",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankDepositMoney,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_deposit_money",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankWithdrawMoney,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_withdraw_money",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankLogQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_log_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankTextQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_text_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankSetTabText,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_set_tab_text",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoGuildBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auto_guild_bank_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoStoreGuildBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auto_store_guild_bank_item",
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
        opcode: ClientOpcodes::DismissCritter,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_dismiss_critter",
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
        opcode: ClientOpcodes::ArenaTeamAccept,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_accept",
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
        opcode: ClientOpcodes::ArenaTeamLeave,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_leave",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamRemove,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_remove",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamDisband,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_disband",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ArenaTeamLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_arena_team_leader",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryArenaTeam,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_arena_team",
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
        opcode: ClientOpcodes::DuelResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_duel_response",
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
        opcode: ClientOpcodes::CalendarCommunityInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_community_invite",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarAddEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_add_event",
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
        opcode: ClientOpcodes::CalendarGetEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_get_event",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarCopyEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_copy_event",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarEventSignUp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_event_sign_up",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_invite",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarUpdateEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_update_event",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarRemoveEvent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_remove_event",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarRemoveInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_remove_invite",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarRsvp,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_rsvp",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarModeratorStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_moderator_status",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CalendarStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_calendar_status",
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
        opcode: ClientOpcodes::AuctionListItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_list_items",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionPlaceBid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_place_bid",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionRemoveItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_remove_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionSellItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_sell_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionReplicateItems,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_replicate_items",
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
        opcode: ClientOpcodes::AuctionableTokenSell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auctionable_token_sell",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionableTokenSellAtMarketPrice,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auctionable_token_sell_at_market_price",
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

const SILVER_LIKE_CPP: u64 = 100;
const MIN_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = 12 * 60;
const SHORT_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = MIN_AUCTION_TIME_MINUTES_LIKE_CPP;
const MEDIUM_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = 2 * MIN_AUCTION_TIME_MINUTES_LIKE_CPP;
const LONG_AUCTION_TIME_MINUTES_LIKE_CPP: u32 = 4 * MIN_AUCTION_TIME_MINUTES_LIKE_CPP;
const LFG_LOCKSTATUS_INSUFFICIENT_EXPANSION_LIKE_CPP: u32 = 1;
const LFG_LOCKSTATUS_TOO_LOW_LEVEL_LIKE_CPP: u32 = 2;
const LFG_LOCKSTATUS_TOO_HIGH_LEVEL_LIKE_CPP: u32 = 3;
const LFG_LOCKSTATUS_TOO_LOW_GEAR_SCORE_LIKE_CPP: u32 = 4;
const LFG_LOCKSTATUS_RAID_LOCKED_LIKE_CPP: u32 = 6;
const LFG_LOCKSTATUS_QUEST_NOT_COMPLETED_LIKE_CPP: u32 = 1022;
const LFG_LOCKSTATUS_MISSING_ITEM_LIKE_CPP: u32 = 1025;
const LFG_LOCKSTATUS_NOT_IN_SEASON_LIKE_CPP: u32 = 1031;
const LFG_LOCKSTATUS_MISSING_ACHIEVEMENT_LIKE_CPP: u32 = 1034;

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

        let _ = self.apply_represented_live_intent_like_cpp(
            crate::session::RepresentedLiveIntentLikeCpp::StandStateChanged(
                crate::session::RepresentedStandStateChangedLikeCpp { state: stand_state },
            ),
        );
    }

    /// C++ `Map::SendInitSelf` (Map.cpp:1877), invoked by `Map::AddPlayerToMap(initPlayer=true)`
    /// on a non-seamless far teleport (HandleMoveWorldportAck -> AddPlayerToMap, Map.cpp:470).
    /// Re-sends the player's OWN object (ActivePlayer create block) so the client finishes the
    /// loading screen and enters the destination map. Sourced from session state; combat stats
    /// are placeholders here (health from the live value, the rest defaulted) and corrected by
    /// the `send_stat_update` that follows. Inventory item objects are not yet re-sent on
    /// teleport (the client retains them from login) — a #NEXT.R8.ENTITIES.1229 follow-up.
    async fn send_player_self_create_for_teleport_like_cpp(&mut self) {
        use wow_core::guid::HighGuid;
        use wow_packet::packets::update::{PlayerCombatStats, UpdateObject};

        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(pos) = self.player_position_like_cpp() else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let (zone_id, _area_id) = self.player_zone_area_like_cpp();
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let gender = self.player_gender_like_cpp();
        let level = self.player_level_like_cpp();

        // Equipped items drive the visible model; bag slots / item objects are not re-sent here.
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        for (slot, item) in self.inventory_items_like_cpp() {
            if (*slot as usize) < 19 {
                visible_items[*slot as usize] = (item.entry_id as i32, 0, 0);
            }
        }

        let health = self.player_health_like_cpp().max(1);
        let combat = PlayerCombatStats {
            health: i64::from(health),
            max_health: i64::from(health),
            ..PlayerCombatStats::default()
        };

        let quest_log = self.quest_log_create_entries_like_cpp();
        let account_toys = self.account_toy_active_player_rows_like_cpp();
        let account_heirlooms = self.account_heirloom_active_player_rows_like_cpp();
        let account_transmog = self.account_transmog_active_player_rows_like_cpp();
        let trait_configs = self.load_active_player_trait_configs_like_cpp(guid).await;
        let player_customizations = self.load_player_customizations_like_cpp(guid).await;
        let party_type = self.party_member_party_type_like_cpp();
        let display_id = crate::handlers::character::default_display_id(race, gender);

        // Rebuild the active SkillInfo rows from the canonical login skill
        // records. This preserves persisted/default values across far
        // teleports instead of re-running LearnDefaultSkills with fabricated
        // level×5 ranks.
        let skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)> =
            if let (Some(skill_store), Some(skill_line_store), Some(skill_tiers_store)) = (
                self.skill_store(),
                self.skill_line_store(),
                self.skill_tiers_store(),
            ) {
                let mut skill_records: Vec<_> =
                    self.player_skill_records_like_cpp().values().collect();
                skill_records.sort_by_key(|skill| skill.skill_id);
                skill_records
                    .into_iter()
                    .filter_map(|skill| {
                        skill_store.loaded_skill_info_like_cpp(
                            skill.skill_id,
                            race,
                            class,
                            level,
                            skill.value,
                            skill.max,
                            skill_line_store,
                            skill_tiers_store,
                        )
                    })
                    .map(|entry| {
                        (
                            entry.skill_id,
                            entry.step,
                            entry.rank,
                            entry.starting_rank,
                            entry.max_rank,
                            entry.temp_bonus,
                            entry.perm_bonus,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            };

        let mut player_pkt = UpdateObject::create_player_with_party_type(
            guid,
            race,
            class,
            gender,
            level,
            display_id,
            &pos,
            map_id,
            zone_id,
            true, // is_self -> ActivePlayer fields
            visible_items,
            [ObjectGuid::EMPTY; 141],
            combat,
            skill_info,
            self.player_gold_like_cpp(),
            quest_log,
            party_type,
        );
        let (player_flags, player_flags_ex) = self.represented_player_flags_for_create_like_cpp();
        player_pkt.set_player_flags_like_cpp(player_flags, player_flags_ex);
        player_pkt.set_player_xp_like_cpp(self.player_xp_like_cpp() as i32);
        player_pkt.set_player_next_level_xp_like_cpp(self.player_next_level_xp_like_cpp() as i32);
        player_pkt.set_player_max_level_like_cpp(self.player_active_max_level_like_cpp() as i32);
        player_pkt
            .set_player_scaling_level_delta_like_cpp(self.player_scaling_level_delta_like_cpp());
        player_pkt.set_player_rest_info_like_cpp(
            0,
            self.represented_xp_rest_threshold_like_cpp(),
            self.represented_xp_rest_state_like_cpp(),
        );
        player_pkt.set_player_account_guids_like_cpp(
            ObjectGuid::create_global(HighGuid::WowAccount, 0, self.account_id as i64),
            ObjectGuid::create_global(HighGuid::BNetAccount, 0, self.battlenet_account_id() as i64),
        );
        player_pkt.set_player_collection_dynamic_fields_like_cpp(
            account_toys,
            account_heirlooms,
            account_transmog,
            trait_configs,
        );
        player_pkt.set_player_action_buttons_like_cpp(
            self.represented_action_buttons_snapshot_like_cpp(),
        );
        player_pkt.set_player_customizations_like_cpp(player_customizations);
        self.send_packet(&player_pkt);
        info!(
            account = self.account_id,
            map = map_id,
            "[FAR_TELEPORT] sent SendInitSelf (player ActivePlayer create) for destination map"
        );
    }

    /// CMSG_SUSPEND_TOKEN_RESPONSE — client acknowledges SMSG_SUSPEND_TOKEN during a far
    /// teleport. C++ `WorldSession::HandleSuspendTokenResponse` (MovementHandler.cpp:239)
    /// replies with SMSG_NEW_WORLD so the client loads the destination map; only then does
    /// the client send CMSG_WORLD_PORT_RESPONSE. Without this step the client sits on the
    /// loading screen at 0% forever. #NEXT.R8.ENTITIES.1229.
    pub async fn handle_suspend_token_response(&mut self, _pkt: wow_packet::WorldPacket) {
        if !self.represented_far_teleport_pending_like_cpp() {
            return;
        }
        let Some((new_map, new_pos)) = self.pending_teleport else {
            return;
        };
        self.send_packet(&wow_packet::packets::misc::NewWorld {
            map_id: new_map,
            pos: new_pos,
            reason: 0,
        });
        info!(
            account = self.account_id,
            map = new_map,
            "[FAR_TELEPORT] SuspendTokenResponse -> sent SMSG_NEW_WORLD (client now loads destination map)"
        );
    }

    /// CMSG_WORLD_PORT_RESPONSE — client confirms it has loaded the new map.
    /// C# ref: MovementHandler.HandleMoveWorldportAck
    /// Sent after SMSG_NEW_WORLD (which is emitted from handle_suspend_token_response).
    /// We respond with SMSG_RESUME_TOKEN and replay the after-add init.
    pub async fn handle_world_port_response(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::ResumeToken;

        if !self.represented_far_teleport_pending_like_cpp() {
            warn!(
                "WorldPortResponse from account {} but far teleport semaphore is not set",
                self.account_id
            );
            return;
        }
        self.set_represented_far_teleport_pending_like_cpp(false);

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
        let _ = self.update_represented_item_level_area_based_scaling_like_cpp();
        let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        self.update_registry_position();
        self.resummon_pet_temporary_unsummoned_if_any_like_cpp();
        self.process_represented_delayed_resurrection_after_teleport_like_cpp();

        // SMSG_NEW_WORLD was already sent from handle_suspend_token_response (C++ sends it in
        // HandleSuspendTokenResponse, BEFORE the client's worldport ack — MovementHandler.cpp:253);
        // it must NOT be resent here or the client never finishes loading. #NEXT.R8.ENTITIES.1229.

        // SMSG_RESUME_TOKEN — C++ HandleMoveWorldportAck sets SequenceIndex =
        // player->m_movementCounter (read here, before SendInitialPacketsBeforeAddToMap resets
        // it) and Reason = 1 for a non-seamless far teleport (MovementHandler.cpp:108-111).
        let resume_seq = self.movement_counter_like_cpp();
        self.send_packet(&ResumeToken {
            sequence_index: resume_seq,
            reason: 1,
        });
        info!(
            account = self.account_id,
            map = new_map,
            resume_seq,
            "[FAR_TELEPORT] worldport ack: sent ResumeToken(reason=1); NewWorld was sent at SuspendTokenResponse #NEXT.R8.ENTITIES.1229"
        );

        let Some(guid) = self.player_guid() else {
            self.set_state(crate::session::SessionState::LoggedIn);
            return;
        };
        let updateobject_trace_enabled = std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some();

        // Before-add control packets the client needs for the new map: C++
        // SendInitialPacketsBeforeAddToMap resets m_movementCounter (Player.cpp:23483) and
        // ends with SetMovedUnit -> SMSG_MOVE_SET_ACTIVE_MOVER, plus a fresh time sync. The
        // full before-add packet SET (spells/factions/action bars/etc.) is NOT re-sent on
        // teleport: the client retains it from login and it is unchanged, and re-running the
        // DB-backed before-add helper here is a documented #NEXT.R8.ENTITIES.1229 follow-up.
        self.reset_movement_counter_like_cpp();
        self.send_packet(&wow_packet::packets::misc::MoveSetActiveMover { mover_guid: guid });
        self.send_time_sync();

        // C++ Map::AddPlayerToMap(initPlayer=true) -> SendInitSelf (Map.cpp:470): re-send the
        // player's OWN object (ActivePlayer create block) for the destination map. Without it
        // the client loads to 100% but never enters the world. #NEXT.R8.ENTITIES.1229.
        self.send_player_self_create_for_teleport_like_cpp().await;

        // AddPlayerToMap-equivalent: refresh nearby world objects at the new position.
        self.send_nearby_creatures(new_map as u16, &new_pos, 0)
            .await;
        self.send_nearby_gameobjects(new_map as u16, &new_pos, 0)
            .await;
        info!(
            account = self.account_id,
            map = new_map,
            visible = self.client_visible_guids_like_cpp.len(),
            "[FAR_TELEPORT] replayed before-add (MoveSetActiveMover + TimeSync) + refreshed \
             nearby objects; now sending after-add init"
        );

        // SendInitialPacketsAfterAddToMap: post-add phase shift, InitWorldStates resolved for
        // the destination map, the PhasingHandler::OnMapChange phase shift, CUF profiles, auras.
        self.send_initial_packets_after_add_to_map(
            guid,
            &new_pos,
            new_map as i32,
            updateobject_trace_enabled,
        )
        .await;

        let (zone_id, area_id) = self.player_zone_area_like_cpp();
        info!(
            account = self.account_id,
            map = new_map,
            zone = zone_id,
            area = area_id,
            resume_seq,
            "[FAR_TELEPORT] COMPLETE — sent after-add init (InitWorldStates for this map + \
             phase-shift x2 + CUF + auras). Client should now be live in the new map."
        );

        // Full stat VALUES update — C++ login sends this after the create; it overwrites the
        // self-create block's placeholder combat stats with the player's real values.
        self.send_stat_update();

        // Back to LoggedIn — handler dispatch resumes.
        self.set_state(crate::session::SessionState::LoggedIn);
    }

    /// CMSG_AREA_TRIGGER — player entered an area trigger.
    /// C++ ref: `WorldSession::HandleAreaTriggerOpcode`.
    pub async fn handle_area_trigger(&mut self, mut pkt: wow_packet::WorldPacket) {
        let Ok(trigger_id) = pkt.read_uint32() else {
            warn!(
                account = self.account_id,
                "AreaTrigger packet missing trigger ID"
            );
            return;
        };
        let Ok(entered) = pkt.read_bit() else {
            warn!(
                account = self.account_id,
                trigger_id, "AreaTrigger packet missing Entered bit"
            );
            return;
        };
        let Ok(_from_client) = pkt.read_bit() else {
            warn!(
                account = self.account_id,
                trigger_id, "AreaTrigger packet missing FromClient bit"
            );
            return;
        };

        info!(
            "AreaTrigger: account {} trigger_id={} entered={}",
            self.account_id, trigger_id, entered
        );

        if self.is_in_taxi_flight_like_cpp() {
            debug!(
                "Area trigger {} ignored because player is in taxi flight",
                trigger_id
            );
            return;
        }

        let Some(at_entry) = self.area_trigger_db2_entry_like_cpp(trigger_id).cloned() else {
            debug!("Unknown area trigger ID {}", trigger_id);
            return;
        };

        let player_in_area_trigger = self.player_is_in_area_trigger_radius_like_cpp(&at_entry);
        // Legacy1 validates radius only for an enter notification and is the
        // selected parity behavior. Legacy2 instead requires `entered` to
        // equal the current inside/outside result, so it rejects a leave that
        // arrives while the player is still inside. Keep the disagreement
        // explicit; a 3.4.3 client capture is still needed to adjudicate it.
        if entered && !player_in_area_trigger {
            debug!(
                "Area trigger {} ignored because player is too far",
                trigger_id
            );
            return;
        }

        if !self.area_trigger_client_conditions_meet_like_cpp(trigger_id) {
            debug!("Area trigger {} rejected by C++ conditions", trigger_id);
            return;
        }

        // C++ continues unless `ScriptMgr::OnAreaTrigger` returns true. A DB
        // binding alone therefore cannot consume the event.
        let bound_script_id = self
            .area_trigger_script_store()
            .and_then(|store| store.get_script_id_like_cpp(trigger_id))
            .filter(|script_id| *script_id != wow_data::ScriptIdLikeCpp::NONE);
        if let Some(script_id) = bound_script_id {
            match self.dispatch_area_trigger_script_like_cpp(script_id, trigger_id, entered) {
                Some(true) => return,
                Some(false) => {}
                None => warn!(
                    trigger_id,
                    entered,
                    ?script_id,
                    "Area trigger script dispatch is unrepresented; preserving prior continuation"
                ),
            }
        }

        if self.handle_represented_tavern_area_trigger_like_cpp(trigger_id, entered) {
            return;
        }

        let Some(trigger) = self
            .area_trigger_store()
            .and_then(|store| store.get_trigger(trigger_id).cloned())
        else {
            return;
        };

        // Lookup in represented teleport store
        info!(
            "AreaTrigger {} detected at map {} pos ({}, {}, {})",
            trigger_id, trigger.map_id, trigger.pos.x, trigger.pos.y, trigger.pos.z
        );

        if !entered {
            return;
        }

        if let Some(ref teleport) = trigger.teleport {
            let target_map = teleport.target_map;
            let target_pos = teleport.target_position;
            info!(
                "AreaTrigger {} → teleport to map {} ({:.2}, {:.2}, {:.2})",
                trigger_id, target_map, target_pos.x, target_pos.y, target_pos.z
            );
            self.teleport_to(target_map, target_pos).await;
        }
    }

    fn area_trigger_client_conditions_meet_like_cpp(&mut self, trigger_id: u32) -> bool {
        let Some(condition_store) = self.condition_store().cloned() else {
            return true;
        };
        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            return false;
        };

        let player_unit_snapshot = self.condition_player_unit_snapshot_like_cpp();
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let area_table_store = self.area_table_store().cloned();

        let mut source_info =
            crate::conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, player_snapshot);

        crate::conditions::is_object_meeting_not_grouped_conditions_like_cpp(
            condition_store.as_ref(),
            ConditionSourceType::AreaTriggerClientTriggered,
            trigger_id,
            &mut source_info,
            |condition, source_info| {
                // C++ combines the base condition with
                // `ScriptMgr::OnConditionCheck`. Rust does not yet have a
                // ConditionScript dispatcher, so allowing a scripted row
                // through would silently bypass its only custom predicate.
                if condition.script_id != 0 {
                    warn!(
                        trigger_id,
                        script_id = condition.script_id,
                        "Area trigger ConditionScript dispatch is unrepresented; failing closed"
                    );
                    return false;
                }

                let context_is_represented = match condition.condition_type {
                    ConditionType::None
                    | ConditionType::MapId
                    | ConditionType::ZoneId
                    | ConditionType::Class
                    | ConditionType::Team
                    | ConditionType::Race
                    | ConditionType::Gender
                    | ConditionType::Level
                    | ConditionType::Alive
                    | ConditionType::HpVal
                    | ConditionType::HpPct
                    | ConditionType::Taxi
                    | ConditionType::ObjectEntryGuid
                    | ConditionType::ObjectEntryGuidLegacy
                    | ConditionType::TypeMask
                    | ConditionType::TypeMaskLegacy => true,
                    ConditionType::AreaId => area_table_store.is_some(),
                    _ => false,
                };
                if !context_is_represented {
                    warn!(
                        trigger_id,
                        condition_type = ?condition.condition_type,
                        "Area trigger condition context is unrepresented; failing closed"
                    );
                    return false;
                }

                match crate::conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |current_area, required_area| {
                        area_table_store.as_ref().is_some_and(|store| {
                            store.is_in_area_like_cpp(current_area, required_area)
                        })
                    },
                ) {
                    crate::conditions::ConditionMeetResult::Evaluated(value) => value,
                    crate::conditions::ConditionMeetResult::Unsupported => {
                        warn!(
                            trigger_id,
                            condition_type = ?condition.condition_type,
                            "Area trigger condition evaluation is unrepresented; failing closed"
                        );
                        false
                    }
                }
            },
        )
    }

    /// CMSG_REQUEST_CEMETERY_LIST — client asks for graveyards in zone.
    /// C++ ref: `WorldSession::HandleRequestCemeteryList`.
    pub async fn handle_request_cemetery_list(&mut self, _pkt: wow_packet::WorldPacket) {
        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
            info!(
                account = self.account_id,
                state = ?self.state(),
                "RUST_CEMETERY_TRACE handler entry"
            );
        }
        let (zone_id, area_id) = self.player_zone_area_like_cpp();
        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
            info!(
                account = self.account_id,
                state = ?self.state(),
                zone = zone_id,
                area = area_id,
                map_id = self.player_map_id_like_cpp(),
                player = ?self.player_guid(),
                "RUST_CEMETERY_TRACE handler resolved zone_area"
            );
        }
        let Some(graveyard_store) = self.graveyard_store().cloned() else {
            info!(
                zone = zone_id,
                area = area_id,
                map_id = self.player_map_id_like_cpp(),
                player = ?self.player_guid(),
                "No graveyard store available for CMSG_REQUEST_CEMETERY_LIST"
            );
            return;
        };
        let Some(graveyards) = graveyard_store.graveyards_for_zone(zone_id) else {
            info!(
                zone = zone_id,
                area = area_id,
                map_id = self.player_map_id_like_cpp(),
                player = ?self.player_guid(),
                "No graveyards found in CMSG_REQUEST_CEMETERY_LIST"
            );
            return;
        };

        let mut cemetery_ids = Vec::new();
        for graveyard in graveyards {
            if cemetery_ids.len() >= 16 {
                break;
            }
            if self.graveyard_conditions_meet_like_cpp(&graveyard.conditions) {
                cemetery_ids.push(graveyard.safe_loc_id);
            }
        }

        if cemetery_ids.is_empty() {
            info!(
                zone = zone_id,
                area = area_id,
                map_id = self.player_map_id_like_cpp(),
                candidate_count = graveyards.len(),
                player = ?self.player_guid(),
                "No graveyards passed conditions in CMSG_REQUEST_CEMETERY_LIST"
            );
            return;
        }

        info!(
            zone = zone_id,
            area = area_id,
            map_id = self.player_map_id_like_cpp(),
            candidate_count = graveyards.len(),
            accepted_count = cemetery_ids.len(),
            cemetery_ids = ?cemetery_ids,
            player = ?self.player_guid(),
            "Sending C++ RequestCemeteryListResponse"
        );
        self.send_packet(&RequestCemeteryListResponse {
            is_gossip_triggered: false,
            cemetery_ids,
        });
    }

    fn graveyard_conditions_meet_like_cpp(
        &mut self,
        conditions_ref: &wow_data::ConditionsReference,
    ) -> bool {
        let Some(conditions) = conditions_ref.upgrade() else {
            return true;
        };
        if conditions.is_empty() {
            return true;
        }

        let Some(condition_store) = self.condition_store().cloned() else {
            warn!("Cemetery condition check failed closed: missing condition store");
            return false;
        };
        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            warn!("Cemetery condition check failed closed: missing player object");
            return false;
        };

        let player_unit_snapshot = self.condition_player_unit_snapshot_like_cpp();
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let needs_player_condition_context = conditions.iter().any(|condition| {
            condition.reference_id != 0
                || condition.condition_type == ConditionType::PlayerCondition
        });
        let player_condition_store = needs_player_condition_context
            .then(|| self.player_condition_store().cloned())
            .flatten();
        let player_condition_context = needs_player_condition_context
            .then(|| self.represented_player_condition_context_like_cpp());

        let mut source_info =
            crate::conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, player_snapshot);
        if let (Some(store), Some(context)) = (
            player_condition_store.as_ref(),
            player_condition_context.as_ref(),
        ) {
            source_info.set_player_condition_store(store.as_ref());
            source_info.set_player_condition_context(0, context.as_context(self));
        }

        crate::conditions::is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store.as_ref(),
            |condition, source_info| match crate::conditions::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |current_area, required_area| current_area == required_area,
            ) {
                crate::conditions::ConditionMeetResult::Evaluated(value) => value,
                crate::conditions::ConditionMeetResult::Unsupported => {
                    warn!(
                        "Cemetery condition check failed closed: unsupported {:?}",
                        condition.condition_type
                    );
                    false
                }
            },
        )
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
        if self.pending_teleport.is_some() || self.near_teleport_pending_like_cpp() {
            self.schedule_represented_resurrection_after_teleport_like_cpp(request);
        } else {
            self.apply_represented_resurrection_health_like_cpp(request.health);
        }
    }

    /// CMSG_REPOP_REQUEST — release spirit.
    /// C++ ref: `WorldSession::HandleRepopRequest`.
    pub async fn handle_repop_request(&mut self, mut pkt: wow_packet::WorldPacket) {
        let _request = match RepopRequest::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "RepopRequest parse failed: {error}"
                );
                return;
            }
        };

        if self.player_is_alive_like_cpp() || self.player_has_ghost_flag_like_cpp() {
            return;
        }

        // C++ also blocks `SPELL_AURA_PREVENT_RESURRECTION`, handles JUST_DIED
        // promotion through KillPlayer, removes the pet, builds the corpse, and
        // teleports to the graveyard. Rust has only the represented death/ghost
        // seam here; full corpse/graveyard runtime remains open.
        self.set_player_alive_like_cpp(false);
        self.set_player_ghost_flag_like_cpp(true);
        self.represented_repop_at_graveyard_count =
            self.represented_repop_at_graveyard_count.saturating_add(1);
    }

    /// CMSG_CLIENT_PORT_GRAVEYARD — manually teleport ghost to graveyard.
    /// C++ ref: `WorldSession::HandlePortGraveyard`.
    pub async fn try_handle_client_port_graveyard_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) -> bool {
        if PortGraveyard::read(&mut pkt).is_err() {
            return false;
        }

        if self.player_is_alive_like_cpp() || !self.player_has_ghost_flag_like_cpp() {
            return true;
        }

        // C++ calls `Player::RepopAtGraveyard()`. Rust still represents the
        // graveyard selection/teleport runtime as a counter seam shared with
        // release and instance-lock decline paths.
        self.represented_repop_at_graveyard_count =
            self.represented_repop_at_graveyard_count.saturating_add(1);
        true
    }

    /// CMSG_RECLAIM_CORPSE — resurrect at corpse.
    /// C++ ref: `WorldSession::HandleReclaimCorpse`.
    pub async fn handle_reclaim_corpse(&mut self, mut pkt: wow_packet::WorldPacket) {
        let _request = match ReclaimCorpse::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ReclaimCorpse parse failed: {error}"
                );
                return;
            }
        };

        if self.player_is_alive_like_cpp() {
            return;
        }

        if !self.player_has_ghost_flag_like_cpp() {
            return;
        }

        // C++ checks arena, live corpse existence, reclaim delay, and distance
        // before `ResurrectPlayer(0.5f)` + `SpawnCorpseBones`. Those require the
        // full player-corpse runtime; this represented slice only clears the
        // ghost/dead state when the already-known C++ gates pass.
        self.set_player_ghost_flag_like_cpp(false);
        let restore_percent = if self.player_in_represented_battleground_like_cpp() {
            1.0
        } else {
            0.5
        };
        self.apply_represented_resurrection_percent_like_cpp(restore_percent);
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
                    queued_at: std::time::Instant::now(),
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
                visual: request.cast.visual.clone(),
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
            ..SpellCastMetadata::default()
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
                cast_flags: 0x0000_0002,
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
            self.send_packet_realm(&InventoryChangeFailure::error(
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

    pub async fn handle_query_next_mail_time(&mut self) {
        const MAIL_CHECK_MASK_READ_LIKE_CPP: u8 = 0x01;
        const MAIL_NORMAL_LIKE_CPP: u8 = 0;

        let Some(char_db) = self.char_db().cloned() else {
            self.send_packet_realm(&MailQueryNextTimeResult::no_mail());
            return;
        };

        let Some(player_object_guid) = self.player_guid() else {
            self.send_packet_realm(&MailQueryNextTimeResult::no_mail());
            return;
        };

        let player_guid = player_object_guid.counter() as u64;
        let now = GameTime::now().as_secs() as i64;
        let mut stmt = char_db.prepare(CharStatements::SEL_MAIL);
        stmt.set_u64(0, player_guid);

        let mut result = match char_db.query(&stmt).await {
            Ok(result) => result,
            Err(error) => {
                warn!(
                    ?error,
                    player_guid, "Failed to query mail for CMSG_QUERY_NEXT_MAIL_TIME"
                );
                self.send_packet_realm(&MailQueryNextTimeResult::no_mail());
                return;
            }
        };

        let mut packet = MailQueryNextTimeResult::no_mail();
        let mut sent_senders = std::collections::BTreeSet::new();

        if !result.is_empty() {
            loop {
                let checked = result.try_read::<u8>(10).unwrap_or(0);
                let deliver_time = result.try_read::<i64>(7).unwrap_or(0);
                let sender = result.try_read::<u64>(2).unwrap_or(0);

                if (checked & MAIL_CHECK_MASK_READ_LIKE_CPP) == 0
                    && now >= deliver_time
                    && sent_senders.insert(sender)
                {
                    let message_type = result.try_read::<u8>(1).unwrap_or(0);
                    let stationery = result.try_read::<i32>(11).unwrap_or(0);
                    let sender_guid = if message_type == MAIL_NORMAL_LIKE_CPP {
                        ObjectGuid::create_player(self.realm_id(), sender as i64)
                    } else {
                        ObjectGuid::EMPTY
                    };

                    packet.next_mail_time = 0.0;
                    packet.next.push(MailNextTimeEntry {
                        sender_guid,
                        time_left: (deliver_time - now) as f32,
                        alt_sender_id: if message_type == MAIL_NORMAL_LIKE_CPP {
                            0
                        } else {
                            sender as i32
                        },
                        alt_sender_type: message_type as i8,
                        stationery_id: stationery,
                    });

                    if sent_senders.len() > 2 {
                        break;
                    }
                }

                if !result.next_row() {
                    break;
                }
            }
        }

        self.send_packet_realm(&packet);
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

    pub async fn handle_set_difficulty_id(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetDifficultyId::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetDifficultyId parse failed: {error}"
                );
                return;
            }
        };

        self.apply_represented_difficulty_change_like_cpp(packet.difficulty_id)
            .await;
    }

    pub async fn handle_toggle_difficulty(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ToggleDifficulty::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ToggleDifficulty parse failed: {error}"
            );
            return;
        }

        let Some(difficulty_id) = self.represented_toggle_difficulty_target_like_cpp() else {
            debug!(
                account = self.account_id,
                "ToggleDifficulty has no represented toggle difficulty available"
            );
            return;
        };

        self.apply_represented_difficulty_change_like_cpp(difficulty_id)
            .await;
    }

    pub async fn handle_set_dungeon_difficulty(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetDungeonDifficulty::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetDungeonDifficulty parse failed: {error}"
                );
                return;
            }
        };

        self.apply_represented_difficulty_change_like_cpp(packet.difficulty_id)
            .await;
    }

    pub async fn handle_set_raid_difficulty(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetRaidDifficulty::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetRaidDifficulty parse failed: {error}"
                );
                return;
            }
        };

        let Some(difficulty_id) = self
            .represented_raid_difficulty_request_like_cpp(packet.difficulty_id, packet.legacy != 0)
        else {
            return;
        };

        self.apply_represented_difficulty_change_like_cpp(difficulty_id)
            .await;
    }

    async fn apply_represented_difficulty_change_like_cpp(&mut self, difficulty_id: u32) {
        let reset_owner = self.represented_set_difficulty_reset_owner_like_cpp(difficulty_id);
        if let Some(reset_owner) = reset_owner {
            self.reset_represented_instances_like_cpp(
                reset_owner,
                RepresentedInstanceResetMethodLikeCpp::OnChangeDifficulty,
            )
            .await;
        }

        let statements = self.represented_set_difficulty_id_like_cpp(difficulty_id);
        if statements.is_empty() {
            return;
        }

        if let Some(char_db) = self.char_db() {
            let mut tx = SqlTransaction::new();
            for statement in statements {
                tx.append(statement);
            }
            if let Err(error) = char_db.commit_transaction(tx).await {
                warn!(
                    account = self.account_id,
                    player_guid = ?self.player_guid(),
                    %error,
                    "failed to persist represented group difficulty change"
                );
            }
        }
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

        self.send_packet_realm(&UpdateAccountData {
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

    pub async fn handle_tutorial(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match TutorialSetFlag::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "Tutorial parse failed: {error}");
                return;
            }
        };

        if !self.apply_tutorial_action_like_cpp(packet.action, packet.tutorial_bit) {
            warn!(
                account = self.account_id,
                action = packet.action,
                tutorial_bit = packet.tutorial_bit,
                "CMSG_TUTORIAL ignored invalid action or TutorialBit like C++"
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

    /// CMSG_BATTLEMASTER_HELLO — player asks a battlemaster NPC for its queue list.
    /// C++ ref: `WorldSession::HandleBattlemasterHelloOpcode`.
    pub async fn handle_battlemaster_hello(&mut self, mut pkt: wow_packet::WorldPacket) {
        let hello = match Hello::read(&mut pkt) {
            Ok(hello) => hello,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlemasterHello parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when the target cannot be interacted with as a
        // battlemaster. The accepted branch records the list intent until
        // BattlegroundMgr::SendBattlegroundList is live in Rust.
        let _accepted = self.battlemaster_hello_like_cpp(hello.unit);
    }

    /// CMSG_BATTLEFIELD_LIST — player asks for the queue list of a battleground type.
    /// C++ ref: `WorldSession::HandleBattlefieldListOpcode`.
    pub async fn handle_battlefield_list(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlefieldListRequest::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlefieldList parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sBattlemasterListStore has no ListID row.
        // The accepted branch records the SendBattlegroundList intent until
        // BattlegroundMgr owns live queue/list packets in Rust.
        let _accepted = self.battlefield_list_like_cpp(request.list_id);
    }

    /// CMSG_BATTLEMASTER_JOIN — player asks to join a battleground queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinOpcode`.
    pub async fn handle_battlemaster_join(&mut self, mut pkt: wow_packet::WorldPacket) {
        let join = match BattlemasterJoin::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlemasterJoin parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently for missing/invalid queues and early queue gates.
        // The accepted branch records the queue intent until BattlegroundQueue
        // and BattlegroundMgr queue-status packets are live in Rust.
        let _accepted =
            self.battlemaster_join_like_cpp(&join.queue_ids, join.roles, join.blacklist_map);
    }

    /// CMSG_BATTLEMASTER_JOIN_ARENA — player asks to join a rated arena queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinArena`.
    pub async fn handle_battlemaster_join_arena(&mut self, mut pkt: wow_packet::WorldPacket) {
        let join = match BattlemasterJoinArena::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlemasterJoinArena parse failed: {error}"
                );
                return;
            }
        };

        // C++ gates on already-in-BG, the all-arenas template, disabled arena,
        // group and leader before entering ArenaTeamMgr/queue code. Rust records
        // the bounded queue intent after those representable gates until the
        // live rated-arena manager is ported.
        let _accepted = self.battlemaster_join_arena_like_cpp(join.team_size_index, join.roles);
    }

    /// CMSG_BATTLEMASTER_JOIN_SKIRMISH — player asks to join an arena skirmish queue.
    /// C++ ref: `WorldSession::HandleBattlemasterJoinSkirmish`.
    pub async fn handle_battlemaster_join_skirmish(&mut self, mut pkt: wow_packet::WorldPacket) {
        let join = match BattlemasterJoinSkirmish::read(&mut pkt) {
            Ok(join) => join,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlemasterJoinSkirmish parse failed: {error}"
                );
                return;
            }
        };

        // C++ ignores IsRated here, derives 2v2/3v3/5v5 from BgTypeId/BracketId,
        // and only applies group/leader gates when AsGroup is set. Queue add and
        // status fanout remain represented until live BattlegroundQueue is ported.
        let _accepted = self.battlemaster_join_skirmish_like_cpp(
            join.bg_type_id,
            join.bracket_id,
            join.as_group,
            join.is_rated,
        );
    }

    /// CMSG_BATTLEFIELD_PORT — player accepts an invite or leaves a BG queue slot.
    /// C++ ref: `WorldSession::HandleBattleFieldPortOpcode`.
    pub async fn handle_battlefield_port(&mut self, mut pkt: wow_packet::WorldPacket) {
        let port = match BattlefieldPort::read(&mut pkt) {
            Ok(port) => port,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlefieldPort parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently for not-in-queue, invalid queue slot, and
        // AcceptedInvite without an invitation. The accepted/leave branch is
        // represented only until live BattlegroundQueue/BattlegroundMgr exists.
        let _accepted = self.battlefield_port_like_cpp(port.ticket, port.accepted_invite);
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
        self.send_packet_realm(&RatedPvpInfo::default());
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

    pub async fn handle_set_pvp(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetPvp::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "SetPvP parse failed: {error}");
                return;
            }
        };

        self.apply_set_pvp_like_cpp(packet.enable_pvp);
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
            self.send_packet(&self.lfg_player_lock_info_like_cpp());
        } else {
            // C++ `SendLfgPartyLockInfo` returns before sending when the player
            // is not in a group. Rust does not expose a live LFG group manager
            // here yet, so the no-group branch remains silent.
        }
    }

    fn lfg_player_lock_info_like_cpp(&self) -> LfgPlayerInfo {
        let Some(store) = self.lfg_dungeon_store_like_cpp() else {
            return LfgPlayerInfo::empty();
        };

        let level = self.player_level_like_cpp();
        let expansion = self.expansion;
        let current_item_level = self.represented_average_item_level_like_cpp().max(0.0) as i32;

        let mut info = LfgPlayerInfo {
            blacklist: LfgBlackList::default(),
            dungeons: Vec::new(),
        };

        for dungeon_id in store.locked_dungeon_ids_like_cpp() {
            let Some(dungeon) = store.get(dungeon_id) else {
                continue;
            };
            if self.map_store().is_some_and(|map_store| {
                !wow_data::lfg_dungeon_is_known_map_like_cpp(dungeon, map_store)
            }) {
                continue;
            }
            if dungeon.type_id == wow_data::LFG_TYPE_RANDOM_LIKE_CPP
                && (dungeon.min_level > level || dungeon.max_level < level)
            {
                continue;
            }
            if let Some(reason) = self.lfg_lock_status_like_cpp(dungeon, level, expansion) {
                info.blacklist.slots.push(LfgListBlacklistEntry {
                    slot: dungeon.entry_like_cpp(),
                    reason,
                    sub_reason1: i32::from(dungeon.required_item_level),
                    sub_reason2: current_item_level,
                    soft_lock: 0,
                });
            }
        }
        info.blacklist.slots.sort_unstable_by_key(|lock| lock.slot);

        for slot in store.random_and_active_seasonal_dungeon_entries_like_cpp(
            level,
            expansion,
            |dungeon_id| self.lfg_season_is_active_like_cpp(dungeon_id),
        ) {
            let mut dungeon_info = LfgPlayerDungeonInfo::random_dungeon_like_cpp(slot);
            if let Some(reward) = store.random_dungeon_reward_like_cpp(slot, level) {
                self.populate_lfg_player_dungeon_reward_like_cpp(&mut dungeon_info, reward);
            }
            info.dungeons.push(dungeon_info);
        }

        info
    }

    fn lfg_season_is_active_like_cpp(&self, _dungeon_id: u32) -> bool {
        // C++ delegates this to `LFGMgr::IsSeasonActive`, backed by holiday
        // state. The current Rust runtime has no live holiday manager wired
        // into LFG yet; inactive is the C++-safe default for seasonal rows.
        false
    }

    fn lfg_lock_status_like_cpp(
        &self,
        dungeon: &wow_data::LfgDungeonDataLikeCpp,
        level: u8,
        expansion: u8,
    ) -> Option<u32> {
        if dungeon.expansion > expansion {
            return Some(LFG_LOCKSTATUS_INSUFFICIENT_EXPANSION_LIKE_CPP);
        }
        if self.lfg_is_disabled_map_type_for_player_like_cpp(
            wow_data::DISABLE_TYPE_MAP,
            dungeon.map,
            dungeon.difficulty,
        ) {
            return Some(LFG_LOCKSTATUS_NOT_IN_SEASON_LIKE_CPP);
        }
        if self.lfg_is_disabled_map_type_for_player_like_cpp(
            wow_data::DISABLE_TYPE_LFG_MAP,
            dungeon.map,
            dungeon.difficulty,
        ) {
            return Some(LFG_LOCKSTATUS_RAID_LOCKED_LIKE_CPP);
        }
        if self.lfg_has_active_instance_lock_like_cpp(dungeon.map, dungeon.difficulty) {
            return Some(LFG_LOCKSTATUS_RAID_LOCKED_LIKE_CPP);
        }
        if dungeon.min_level > level {
            return Some(LFG_LOCKSTATUS_TOO_LOW_LEVEL_LIKE_CPP);
        }
        if dungeon.max_level < level {
            return Some(LFG_LOCKSTATUS_TOO_HIGH_LEVEL_LIKE_CPP);
        }
        if dungeon.seasonal && !self.lfg_season_is_active_like_cpp(dungeon.id) {
            return Some(LFG_LOCKSTATUS_NOT_IN_SEASON_LIKE_CPP);
        }
        if f32::from(dungeon.required_item_level) > self.represented_average_item_level_like_cpp() {
            return Some(LFG_LOCKSTATUS_TOO_LOW_GEAR_SCORE_LIKE_CPP);
        }
        if let Some(requirement) = self
            .access_requirement_store()
            .and_then(|store| store.get(dungeon.map, dungeon.difficulty))
        {
            if requirement.completed_achievement != 0
                && !self.access_requirement_leader_has_achievement_like_cpp(
                    requirement.completed_achievement,
                )
            {
                return Some(LFG_LOCKSTATUS_MISSING_ACHIEVEMENT_LIKE_CPP);
            }

            match crate::session::player_team_for_race_cpp(self.player_race_like_cpp()) {
                Team::Alliance
                    if requirement.quest_done_a != 0
                        && !self.rewarded_quests.contains(&requirement.quest_done_a) =>
                {
                    return Some(LFG_LOCKSTATUS_QUEST_NOT_COMPLETED_LIKE_CPP);
                }
                Team::Horde
                    if requirement.quest_done_h != 0
                        && !self.rewarded_quests.contains(&requirement.quest_done_h) =>
                {
                    return Some(LFG_LOCKSTATUS_QUEST_NOT_COMPLETED_LIKE_CPP);
                }
                _ => {}
            }

            if requirement.item != 0 {
                if !self.represented_has_item_count_like_cpp(requirement.item, 1)
                    && (requirement.item2 == 0
                        || !self.represented_has_item_count_like_cpp(requirement.item2, 1))
                {
                    return Some(LFG_LOCKSTATUS_MISSING_ITEM_LIKE_CPP);
                }
            } else if requirement.item2 != 0
                && !self.represented_has_item_count_like_cpp(requirement.item2, 1)
            {
                return Some(LFG_LOCKSTATUS_MISSING_ITEM_LIKE_CPP);
            }
        }
        None
    }

    fn lfg_is_disabled_map_type_for_player_like_cpp(
        &self,
        disable_type: u32,
        map_id: u32,
        dungeon_difficulty: u8,
    ) -> bool {
        let Some(disable_mgr) = self.disable_mgr() else {
            return false;
        };
        let Some(map_store) = self.map_store() else {
            return false;
        };

        let current_map_id = u32::from(self.player_map_id_like_cpp());
        let (_, area_id) = self.player_zone_area_like_cpp();
        let current_map_instance_type = map_store
            .get(current_map_id)
            .map(|entry| entry.instance_type);

        disable_mgr.is_disabled_for_like_cpp(
            disable_type,
            map_id,
            Some(wow_data::DisableWorldObjectRefLikeCpp {
                type_id: wow_constants::TypeId::Player,
                map_id: current_map_id,
                area_id,
                is_pet: false,
                is_battle_arena: current_map_instance_type == Some(wow_data::MAP_ARENA_LIKE_CPP),
                is_battleground: current_map_instance_type
                    == Some(wow_data::MAP_BATTLEGROUND_LIKE_CPP),
                player_map_difficulty: Some(dungeon_difficulty),
            }),
            0,
            Some(map_store.as_ref()),
        )
    }

    fn populate_lfg_player_dungeon_reward_like_cpp(
        &self,
        dungeon_info: &mut LfgPlayerDungeonInfo,
        reward: &wow_data::LfgDungeonRewardLikeCpp,
    ) {
        let Some(quest_store) = self.quest_store.as_ref() else {
            return;
        };
        let Some(mut quest) = quest_store.get(reward.first_quest_id) else {
            return;
        };

        dungeon_info.first_reward = self.can_reward_lfg_quest_like_cpp(quest, false);
        if std::env::var_os("RUSTYCORE_LFG_TRACE").is_some() {
            info!(
                slot = dungeon_info.slot,
                first_quest_id = reward.first_quest_id,
                other_quest_id = reward.other_quest_id,
                special_flags = quest.special_flags,
                is_df = quest.is_df_quest_like_cpp(),
                df_done = self.df_quests_like_cpp.contains(&quest.id),
                first_reward = dungeon_info.first_reward,
                "RUST_LFG_TRACE reward decision"
            );
        }
        if !dungeon_info.first_reward {
            if reward.other_quest_id == 0 {
                return;
            }
            let Some(other_quest) = quest_store.get(reward.other_quest_id) else {
                return;
            };
            quest = other_quest;
        }

        dungeon_info.rewards.reward_money = self.quest_money_reward_like_cpp(quest) as i32;
        dungeon_info.rewards.reward_xp = self.quest_xp_reward_like_cpp(quest) as i32;

        for (idx, &item_id) in quest.reward_items.iter().enumerate() {
            if item_id == 0 {
                continue;
            }
            dungeon_info.rewards.items.push(LfgPlayerQuestRewardItem {
                item_id: item_id as i32,
                quantity: quest.reward_amounts.get(idx).copied().unwrap_or(0) as i32,
            });
        }

        for (idx, &currency_id) in quest.reward_currencies.iter().enumerate() {
            if currency_id == 0 {
                continue;
            }
            dungeon_info
                .rewards
                .currency
                .push(LfgPlayerQuestRewardCurrency {
                    currency_id: currency_id as i32,
                    quantity: quest.reward_currency_amounts.get(idx).copied().unwrap_or(0) as i32,
                });
        }
    }

    fn can_reward_lfg_quest_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        _msg: bool,
    ) -> bool {
        if !quest.is_df_quest_like_cpp()
            && !quest.is_turn_in_like_cpp()
            && self.player_quests.get(&quest.id).is_none_or(|status| {
                status.status != crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
            })
        {
            return false;
        }
        if quest.is_df_quest_like_cpp() {
            return !self.df_quests_like_cpp.contains(&quest.id);
        }
        if quest.is_daily_like_cpp() && self.daily_quests_completed_like_cpp.contains(&quest.id) {
            return false;
        }
        if quest.is_weekly_like_cpp() && self.weekly_quests_completed_like_cpp.contains(&quest.id) {
            return false;
        }
        if quest.is_monthly_like_cpp() && self.monthly_quests_completed_like_cpp.contains(&quest.id)
        {
            return false;
        }
        if quest.is_seasonal_like_cpp()
            && self
                .seasonal_quests_like_cpp
                .get(&quest.event_id_for_quest_like_cpp())
                .is_some_and(|quests| quests.contains_key(&quest.id))
        {
            return false;
        }

        !self.rewarded_quests.contains(&quest.id)
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
        self.send_packet_realm(&CalendarSendNumPending { num_pending: 0 });
    }
    pub async fn handle_calendar_complain(&mut self, _complain: CalendarComplain) {
        // C++ only parses/logs this packet and has no gameplay side effect.
    }
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

    pub async fn handle_guild_bank_remaining_withdraw_money_query(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        // C++ only sends GuildBankRemainingWithdrawMoney when GetPlayer()->GetGuild()
        // resolves a live guild. Rust has no represented guild-bank manager here
        // yet, so the no-guild branch is correctly silent.
    }

    /// CMSG_GUILD_BANK_ACTIVATE — click a guild-bank GameObject.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankActivate`.
    pub async fn handle_guild_bank_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankActivate::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "GuildBankActivate parse failed: {error}"
                );
                return;
            }
        };

        if self
            .represented_guild_bank_gameobject_can_interact_like_cpp(packet.banker)
            .is_none()
        {
            return;
        }

        if self.represented_guild_id_like_cpp() == 0 {
            self.send_packet(&GuildCommandResult::player_not_in_guild_view_tab_like_cpp());
            return;
        }

        let _accepted =
            self.record_guild_bank_list_request_like_cpp(packet.banker, 0, packet.full_update);
    }

    /// CMSG_GUILD_BANK_QUERY_TAB — request a single guild-bank tab.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankQueryTab`.
    pub async fn handle_guild_bank_query_tab(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankQueryTab::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "GuildBankQueryTab parse failed: {error}"
                );
                return;
            }
        };

        if self
            .represented_guild_bank_gameobject_can_interact_like_cpp(packet.banker)
            .is_none()
        {
            return;
        }

        if self.represented_guild_id_like_cpp() == 0 {
            return;
        }

        let _accepted =
            self.record_guild_bank_list_request_like_cpp(packet.banker, packet.tab, true);
    }

    /// CMSG_GUILD_BANK_BUY_TAB — buy a guild-bank tab.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankBuyTab`.
    pub async fn handle_guild_bank_buy_tab(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankBuyTab::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "GuildBankBuyTab parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_buy_tab_like_cpp(packet.banker, packet.bank_tab);
    }

    /// CMSG_GUILD_BANK_UPDATE_TAB — rename/update a guild-bank tab.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankUpdateTab`.
    pub async fn handle_guild_bank_update_tab(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankUpdateTab::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "GuildBankUpdateTab parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_update_tab_like_cpp(
            packet.banker,
            packet.bank_tab,
            packet.name,
            packet.icon,
        );
    }

    /// CMSG_GUILD_BANK_DEPOSIT_MONEY — deposit player money into the guild bank.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankDepositMoney`.
    pub async fn handle_guild_bank_deposit_money(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankDepositMoney::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "GuildBankDepositMoney parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_money_move_like_cpp(packet.banker, true, packet.money);
    }

    /// CMSG_GUILD_BANK_WITHDRAW_MONEY — withdraw money from the guild bank.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankWithdrawMoney`.
    pub async fn handle_guild_bank_withdraw_money(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankWithdrawMoney::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "GuildBankWithdrawMoney parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_money_move_like_cpp(packet.banker, false, packet.money);
    }

    /// CMSG_GUILD_BANK_LOG_QUERY — request a guild-bank tab log.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankLogQuery`.
    pub async fn handle_guild_bank_log_query(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankLogQuery::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "GuildBankLogQuery parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_log_query_like_cpp(packet.tab);
    }

    /// CMSG_GUILD_BANK_TEXT_QUERY — request a guild-bank tab text.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankTextQuery`.
    pub async fn handle_guild_bank_text_query(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankTextQuery::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "GuildBankTextQuery parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_text_query_like_cpp(packet.tab);
    }

    /// CMSG_GUILD_BANK_SET_TAB_TEXT — update a guild-bank tab text.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankSetTabText`.
    pub async fn handle_guild_bank_set_tab_text(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankSetTabText::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "GuildBankSetTabText parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_set_tab_text_like_cpp(packet.tab, packet.tab_text);
    }

    /// CMSG_AUTO_GUILD_BANK_ITEM — move from player inventory into a guild-bank slot.
    ///
    /// C++ ref: `WorldSession::HandleAutoGuildBankItem`.
    pub async fn handle_auto_guild_bank_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AutoGuildBankItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "AutoGuildBankItem parse failed: {error}"
                );
                return;
            }
        };

        let player_bag = packet
            .container_slot
            .unwrap_or(wow_entities::INVENTORY_SLOT_BAG_0);
        let _accepted = self.guild_bank_inventory_move_like_cpp(
            packet.banker,
            false,
            packet.bank_tab,
            packet.bank_slot,
            player_bag,
            packet.container_item_slot,
            0,
        );
    }

    /// CMSG_AUTO_STORE_GUILD_BANK_ITEM — auto-store from a guild-bank slot into inventory.
    ///
    /// C++ ref: `WorldSession::HandleAutoStoreGuildBankItem`.
    pub async fn handle_auto_store_guild_bank_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AutoStoreGuildBankItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "AutoStoreGuildBankItem parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_inventory_move_like_cpp(
            packet.banker,
            true,
            packet.bank_tab,
            packet.bank_slot,
            wow_entities::INVENTORY_SLOT_BAG_0,
            wow_entities::NULL_SLOT,
            0,
        );
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
            self.send_battle_pet_journal_lock_status_like_cpp().await;
        }

        self.send_packet_realm(&self.represented_battle_pet_journal_like_cpp());
    }

    /// CMSG_BATTLE_PET_REQUEST_JOURNAL_LOCK — acquire represented journal lock.
    ///
    /// C++ `HandleBattlePetRequestJournalLock` sends lock status and, when the
    /// lock is held, sends the journal.
    pub async fn handle_battle_pet_request_journal_lock(&mut self, _pkt: wow_packet::WorldPacket) {
        self.send_battle_pet_journal_lock_status_like_cpp().await;
        if self.has_represented_battle_pet_journal_lock_like_cpp() {
            self.send_packet_realm(&self.represented_battle_pet_journal_like_cpp());
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

        self.battle_pet_clear_fanfare_durable_like_cpp(request.pet_guid)
            .await;
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

        self.battle_pet_remove_pet_durable_like_cpp(request.pet_guid)
            .await;
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
        let _ = self
            .battle_pet_modify_name_durable_like_cpp(
                request.pet_guid,
                request.name,
                request.declined_names,
                timestamp,
            )
            .await;
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

        self.battle_pet_set_flags_durable_like_cpp(
            request.pet_guid,
            request.flags,
            request.control_type,
        )
        .await;
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

        self.battle_pet_set_battle_slot_durable_like_cpp(request.pet_guid, request.slot)
            .await;
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

    /// CMSG_DISMISS_CRITTER — represented companion dismissal.
    ///
    /// C++ reads a full `CritterGUID`, silently ignores missing/non-active
    /// critters, and sends no direct response. Real `TempSummon::UnSummon` and
    /// object update/despawn fanout remain part of the live companion runtime.
    pub async fn handle_dismiss_critter(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match DismissCritter::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DismissCritter parse failed: {error}"
                );
                return;
            }
        };

        self.represented_dismiss_critter_like_cpp(request.critter_guid);
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

    pub async fn handle_arena_team_accept(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ArenaTeamAccept::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ArenaTeamAccept parse failed: {error}"
            );
            return;
        }

        // C++ returns before clearing Player::m_ArenaTeamIdInvited when
        // sArenaTeamMgr has no team for the invited id. Rust has no live
        // ArenaTeamMgr in this represented seam, so preserve that no-op.
        debug!(
            account = self.account_id,
            "ArenaTeamAccept ignored without represented arena-team manager"
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

    pub async fn handle_arena_team_leave(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ArenaTeamLeave::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ArenaTeamLeave parse failed: {error}"
            );
            return;
        }

        // C++ loops arena slots and only acts when sArenaTeamMgr resolves a
        // real team. No represented ArenaTeamMgr exists yet, so the bounded
        // no-team branch is intentionally silent.
        debug!(
            account = self.account_id,
            "ArenaTeamLeave ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_remove(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamRemove::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ArenaTeamRemove parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.account_id,
            team_id = request.team_id,
            target_name = %request.target_name,
            "ArenaTeamRemove ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_disband(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamDisband::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ArenaTeamDisband parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.account_id,
            team_id = request.team_id,
            "ArenaTeamDisband ignored without represented arena-team manager"
        );
    }

    pub async fn handle_arena_team_leader(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match ArenaTeamLeader::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ArenaTeamLeader parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.account_id,
            team_id = request.team_id,
            target_name = %request.target_name,
            "ArenaTeamLeader ignored without represented arena-team manager"
        );
    }

    pub async fn handle_query_arena_team(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match QueryArenaTeam::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "QueryArenaTeam parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns silently when sArenaTeamMgr has no arena team for TeamId.
        debug!(
            account = self.account_id,
            team_id = request.team_id,
            "QueryArenaTeam ignored without represented arena-team manager"
        );
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
                                    max_players: map_difficulty.max_players,
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

        self.send_packet_realm(&InstanceInfo {
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
            if group.is_lfg_group_like_cpp() {
                return;
            }
            group.leader_guid
        } else {
            player_guid
        };

        let _ = self
            .reset_represented_instances_like_cpp(
                reset_owner_guid,
                RepresentedInstanceResetMethodLikeCpp::Manual,
            )
            .await;
    }

    async fn reset_represented_instances_like_cpp(
        &mut self,
        reset_owner_guid: ObjectGuid,
        method: RepresentedInstanceResetMethodLikeCpp,
    ) -> bool {
        let Some(instance_lock_mgr) = self.instance_lock_mgr.as_ref().cloned() else {
            return false;
        };

        let mut tx = SqlTransaction::new();
        let reset_result = {
            let mut mgr = match instance_lock_mgr.write() {
                Ok(mgr) => mgr,
                Err(_) => return false,
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
                        max_players: map_difficulty.max_players,
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
                    "failed to commit represented instance lock reset transaction"
                );
                return false;
            }
        }

        for lock in reset_result.reset {
            self.send_packet(&InstanceReset {
                map_id: lock.map_id,
            });
        }

        if method == RepresentedInstanceResetMethodLikeCpp::Manual {
            for lock in reset_result.failed_to_reset {
                self.send_packet(&InstanceResetFailed {
                    map_id: lock.map_id,
                    reset_failed_reason: 0,
                });
            }
        }

        true
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
            if self.confirm_pending_bind_like_cpp(pending_bind).await {
                self.represented_confirmed_pending_binds
                    .push(pending_bind.instance_id);
            }
        } else {
            self.represented_repop_at_graveyard_count =
                self.represented_repop_at_graveyard_count.saturating_add(1);
        }
    }

    /// Represented C++ `Player::ConfirmPendingBind`.
    ///
    /// The real C++ path asks the current `InstanceMap` to create a player lock
    /// only when the player's current map instance matches `_pendingBindId`.
    /// Rust does not own full `InstanceMap::i_data` yet, so this bridge uses the
    /// pending-lock completed mask that produced `SMSG_PENDING_RAID_LOCK` as the
    /// available represented `i_instanceLock->GetData()` state.
    async fn confirm_pending_bind_like_cpp(
        &mut self,
        pending_bind: crate::session::RepresentedPendingBind,
    ) -> bool {
        if u32::from(self.player_map_id_like_cpp()) != pending_bind.map_id {
            return false;
        }

        let difficulty_id = {
            let Some(manager) = self.canonical_map_manager.as_ref() else {
                return false;
            };
            let Ok(manager) = manager.lock() else {
                return false;
            };
            let Some(map) = manager.find_map(pending_bind.map_id, pending_bind.instance_id) else {
                return false;
            };
            map.difficulty()
        };

        if self.player_is_game_master_like_cpp() {
            return true;
        }

        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(entries) =
            self.create_map_db2_entries_like_cpp(pending_bind.map_id, difficulty_id)
        else {
            return false;
        };
        let Some(instance_lock_mgr) = self.instance_lock_mgr.as_ref().cloned() else {
            return false;
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let mut tx = SqlTransaction::new();
        let (is_new_lock, new_lock) = {
            let mut mgr = match instance_lock_mgr.write() {
                Ok(mgr) => mgr,
                Err(_) => return false,
            };
            let is_new_lock = mgr
                .find_active_instance_lock_at(player_guid, &entries, now)
                .is_none_or(|lock| lock.is_new || lock.is_expired_at(now));
            let update_event = wow_instances::InstanceLockUpdateEvent {
                instance_id: pending_bind.instance_id,
                new_data: String::new(),
                instance_completed_encounters_mask: pending_bind.completed_mask,
                completed_encounter_bit: None,
                entrance_world_safe_loc_id: None,
            };
            let Some(new_lock) = mgr.update_instance_lock_for_player_tx_at(
                &mut tx,
                player_guid,
                &entries,
                update_event,
                self.reset_schedule_like_cpp(),
                now,
            ) else {
                return false;
            };
            (is_new_lock, new_lock)
        };

        if !tx.is_empty() {
            if let Some(char_db) = self.char_db()
                && let Err(err) = char_db.commit_transaction(tx).await
            {
                warn!(
                    account = self.account_id,
                    player_guid = ?player_guid,
                    instance_id = pending_bind.instance_id,
                    error = ?err,
                    "failed to commit represented pending instance bind transaction"
                );
                return false;
            }
        }

        if is_new_lock {
            self.send_packet(&InstanceSaveCreated {
                gm: self.player_is_game_master_like_cpp(),
            });
            self.send_calendar_raid_lockout_added_like_cpp(&new_lock, &entries, now);
        }

        true
    }

    /// C++ `WorldSession::SendCalendarRaidLockoutAdded`.
    fn send_calendar_raid_lockout_added_like_cpp(
        &self,
        lock: &wow_instances::InstanceLock,
        entries: &wow_instances::MapDb2Entries,
        now: u64,
    ) {
        let effective_expiry =
            lock.effective_expiry_time_at(entries, self.reset_schedule_like_cpp(), now);
        let remaining = (effective_expiry as i128 - now as i128)
            .clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
        self.send_packet(&CalendarRaidLockoutAdded::new_at_unix(
            u64::from(lock.instance_id),
            now.min(i64::MAX as u64) as i64,
            i32::try_from(lock.map_id).unwrap_or(i32::MAX),
            u32::from(lock.difficulty_id),
            remaining,
        ));
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
                map_id: u32::from(self.player_map_id_like_cpp()),
                instance_id,
                completed_mask,
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
        self.send_packet_realm(&LfgListBlacklist::empty());
    }
    pub async fn handle_lfg_list_get_status(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleLfgListGetStatus` always sends LFGUpdateStatus for a live
        // player. Until `sLFGMgr` state is ported, Rust represents the
        // well-defined no-ticket/no-queue branch.
        self.send_packet_realm(&LfgUpdateStatus::removed_from_queue());
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

    pub async fn handle_duel_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match DuelResponse::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DuelResponse parse failed: {error}"
                );
                return;
            }
        };

        self.handle_duel_response_like_cpp(packet.arbiter_guid, packet.accepted, packet.forfeited);
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

    pub async fn handle_calendar_community_invite(&mut self, query: CalendarCommunityInvite) {
        // C++ reads ClubID but does not use it in this handler. It only calls
        // Guild::MassInviteToEvent if the player's guild resolves.
        self.calendar_community_invite_like_cpp(
            query.min_level,
            query.max_level,
            query.max_rank_order,
        );
    }

    pub async fn handle_calendar_add_event(&mut self, query: CalendarAddEvent) {
        // C++ rejects guild-scoped events before allocating CalendarMgr state.
        // Rust only has represented guild membership here, so this captures that
        // observable branch and records otherwise-accepted creation intent.
        let accepted = self.calendar_add_event_like_cpp(
            query.club_id,
            query.event_type,
            query.texture_id,
            query.time_packed,
            query.flags,
            query.invites.len(),
            query.title,
            query.description,
            query.max_size,
        );
        if !accepted {
            self.send_packet(&CalendarCommandResult::with_result_like_cpp(
                CalendarCommandResult::ERROR_GUILD_PLAYER_NOT_IN_GUILD_LIKE_CPP,
            ));
        }
    }

    pub async fn handle_calendar_get_event(&mut self, _query: CalendarGetEvent) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_copy_event(&mut self, _query: CalendarCopyEvent) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no source event for the requested id. Rust does not have CalendarMgr
        // wired yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_event_sign_up(&mut self, _query: CalendarEventSignUp) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_invite(&mut self, query: CalendarInvite) {
        // C++ only consults CalendarMgr for an existing event when Creating is
        // false. Rust does not have CalendarMgr wired yet, so this captures the
        // observable no-event branch without inventing name/cache/guild logic.
        if !query.creating {
            self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
        }
    }

    pub async fn handle_calendar_update_event(&mut self, _query: CalendarUpdateEvent) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_remove_event(&mut self, query: CalendarRemoveEvent) {
        // C++ delegates only EventID and the player GUID to CalendarMgr.
        // CalendarMgr is not live here yet, so capture the represented request.
        self.calendar_remove_event_like_cpp(query.event_id);
    }

    pub async fn handle_calendar_remove_invite(&mut self, _query: CalendarRemoveInvite) {
        // C++ sends CalendarCommandResult(NO_INVITE) when sCalendarMgr has no
        // event for the requested id. Rust does not have CalendarMgr wired yet,
        // so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::no_invite_like_cpp());
    }

    pub async fn handle_calendar_rsvp(&mut self, _query: CalendarRsvp) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_moderator_status(&mut self, _query: CalendarModeratorStatusQuery) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    pub async fn handle_calendar_status(&mut self, _query: CalendarStatus) {
        // C++ sends CalendarCommandResult(EVENT_INVALID) when sCalendarMgr has
        // no event for the requested id. Rust does not have CalendarMgr wired
        // yet, so this represents the observable miss branch.
        self.send_packet(&CalendarCommandResult::event_invalid_like_cpp());
    }

    /// C++ `WorldSession::HandleSetSavedInstanceExtend`.
    pub async fn handle_set_saved_instance_extend(&mut self, query: SetSavedInstanceExtend) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let Ok(map_id) = u32::try_from(query.map_id) else {
            return;
        };
        if u32::from(self.player_map_id_like_cpp()) == map_id {
            return;
        }

        let Ok(difficulty_id) = wow_map::Difficulty::try_from(query.difficulty_id) else {
            return;
        };
        let Some(entries) = self.create_map_db2_entries_like_cpp(map_id, difficulty_id) else {
            return;
        };
        let Some(instance_lock_mgr) = self.instance_lock_mgr.as_ref().cloned() else {
            return;
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let mut tx = SqlTransaction::new();
        let Some((old_expiry, new_expiry)) = ({
            let mut mgr = match instance_lock_mgr.write() {
                Ok(mgr) => mgr,
                Err(_) => return,
            };
            mgr.update_instance_lock_extension_for_player_tx_at(
                &mut tx,
                player_guid,
                &entries,
                query.extend,
                self.reset_schedule_like_cpp(),
                now,
            )
        }) else {
            return;
        };

        if !tx.is_empty()
            && let Some(char_db) = self.char_db()
            && let Err(err) = char_db.commit_transaction(tx).await
        {
            warn!(
                account = self.account_id,
                player_guid = ?player_guid,
                map_id,
                difficulty_id,
                error = ?err,
                "failed to commit represented instance lock extension transaction"
            );
            return;
        }

        let remaining = |expiry: u64| -> i32 {
            (expiry.saturating_sub(now) as i128)
                .min(i128::from(i32::MAX))
                .max(0) as i32
        };
        self.send_packet(&CalendarRaidLockoutUpdated::new_at_unix(
            now.min(i64::MAX as u64) as i64,
            query.map_id,
            query.difficulty_id,
            remaining(old_expiry),
            remaining(new_expiry),
        ));
    }

    // ── Auction house list stubs ──────────────────────────────────────────────

    /// CMSG_AUCTION_LIST_BIDDER_ITEMS — list items bid on.
    /// Returns empty list until AH system is implemented.
    pub async fn handle_auction_list_bidder_items(&mut self, _pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionListBidderItemsResult;
        self.send_packet(&AuctionListBidderItemsResult);
    }

    /// CMSG_AUCTION_LIST_ITEMS — legacy list opcode.
    ///
    /// The current C++ legacy branch reads no fields and only logs that this
    /// opcode is superseded by CMSG_AUCTION_BROWSE_QUERY.
    pub async fn handle_auction_list_items(
        &mut self,
        _packet: wow_packet::packets::misc::AuctionListItems,
    ) {
    }

    /// CMSG_AUCTION_PLACE_BID — bid or buyout an auction.
    ///
    /// C++ gates on throttle, auctioneer interaction, and silver granularity
    /// before reaching AuctionMgr state. Rust has no live AH state yet, so this
    /// records the represented request after the interaction gate.
    pub async fn handle_auction_place_bid(&mut self, packet: AuctionPlaceBid) {
        let Some(_auctioneer) = self.represented_npc_can_interact_with_like_cpp(
            packet.auctioneer,
            NPCFlags1::AUCTIONEER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
                auctioneer = ?packet.auctioneer,
                auction_id = packet.auction_id,
                "AuctionPlaceBid rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };

        self.record_represented_auction_place_bid_like_cpp(RepresentedAuctionPlaceBidLikeCpp {
            auctioneer: packet.auctioneer,
            auction_id: packet.auction_id,
            bid_amount: packet.bid_amount,
            tainted_by_present: packet.tainted_by.is_some(),
            copper_rejected: packet.bid_amount % SILVER_LIKE_CPP != 0,
        });
    }

    /// CMSG_AUCTION_REMOVE_ITEM — cancel one of the player's auctions.
    ///
    /// C++ gates on throttle and auctioneer interaction before checking
    /// AuctionMgr ownership/bidder state and DB. Rust has no live AH map yet,
    /// so this records the represented cancel request after the interaction
    /// gate without pretending the auction mutation exists.
    pub async fn handle_auction_remove_item(&mut self, packet: AuctionRemoveItem) {
        let Some(_auctioneer) = self.represented_npc_can_interact_with_like_cpp(
            packet.auctioneer,
            NPCFlags1::AUCTIONEER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
                auctioneer = ?packet.auctioneer,
                auction_id = packet.auction_id,
                item_id = packet.item_id,
                "AuctionRemoveItem rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };

        self.record_represented_auction_remove_item_like_cpp(RepresentedAuctionRemoveItemLikeCpp {
            auctioneer: packet.auctioneer,
            auction_id: packet.auction_id,
            item_id: packet.item_id,
            tainted_by_present: packet.tainted_by.is_some(),
        });
    }

    /// CMSG_AUCTION_SELL_ITEM — post a single non-commodity item for auction.
    ///
    /// C++ validates packet-level sell-item constraints before auctioneer
    /// lookup, then validates auctioneer, runtime, live item state, deposit,
    /// and DB. Rust captures the packet/auctioneer/runtime gates currently
    /// representable and leaves live AuctionMgr/item mutation open.
    pub async fn handle_auction_sell_item(&mut self, packet: AuctionSellItem) {
        let first_item = packet.items.first().copied();
        let item_list_rejected = packet.items.len() != 1;
        let use_count_rejected = packet.items.len() == 1
            && first_item
                .map(|item| item.use_count != 1)
                .unwrap_or_default();
        let no_price_rejected = packet.min_bid == 0 && packet.buyout_price == 0;
        let max_money_rejected =
            packet.min_bid > MAX_MONEY_AMOUNT || packet.buyout_price > MAX_MONEY_AMOUNT;
        let copper_rejected =
            packet.min_bid % SILVER_LIKE_CPP != 0 || packet.buyout_price % SILVER_LIKE_CPP != 0;

        let mut represented = RepresentedAuctionSellItemLikeCpp {
            auctioneer: packet.auctioneer,
            item_guid: first_item.map(|item| item.guid),
            item_use_count: first_item.map(|item| item.use_count),
            min_bid: packet.min_bid,
            buyout_price: packet.buyout_price,
            runtime_minutes: packet.runtime,
            tainted_by_present: packet.tainted_by.is_some(),
            item_list_rejected,
            use_count_rejected,
            no_price_rejected,
            max_money_rejected,
            copper_rejected,
            auctioneer_accepted: false,
            runtime_rejected: false,
        };

        if item_list_rejected
            || use_count_rejected
            || no_price_rejected
            || max_money_rejected
            || copper_rejected
        {
            self.record_represented_auction_sell_item_like_cpp(represented);
            return;
        }

        let Some(_auctioneer) = self.represented_npc_can_interact_with_like_cpp(
            packet.auctioneer,
            NPCFlags1::AUCTIONEER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
                auctioneer = ?packet.auctioneer,
                runtime = packet.runtime,
                "AuctionSellItem rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };
        represented.auctioneer_accepted = true;

        represented.runtime_rejected = !matches!(
            packet.runtime,
            SHORT_AUCTION_TIME_MINUTES_LIKE_CPP
                | MEDIUM_AUCTION_TIME_MINUTES_LIKE_CPP
                | LONG_AUCTION_TIME_MINUTES_LIKE_CPP
        );
        self.record_represented_auction_sell_item_like_cpp(represented);
    }

    /// CMSG_AUCTION_REPLICATE_ITEMS — replicate auction-house changes.
    ///
    /// C++ gates on an alive, usable auctioneer before building the replicate
    /// response from AuctionMgr. The live AH object map/response builder are
    /// not ported yet, so this slice records the accepted represented request.
    pub async fn handle_auction_replicate_items(&mut self, packet: AuctionReplicateItems) {
        let Some(_auctioneer) = self.represented_npc_can_interact_with_like_cpp(
            packet.auctioneer,
            NPCFlags1::AUCTIONEER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
                auctioneer = ?packet.auctioneer,
                "AuctionReplicateItems rejected: auctioneer missing, invalid, hostile/dead, out of range, or lacks AUCTIONEER flag"
            );
            return;
        };

        self.record_represented_auction_replicate_request_like_cpp(
            RepresentedAuctionReplicateRequestLikeCpp {
                auctioneer: packet.auctioneer,
                change_number_global: packet.change_number_global,
                change_number_cursor: packet.change_number_cursor,
                change_number_tombstone: packet.change_number_tombstone,
                count: packet.count,
                tainted_by_present: packet.tainted_by.is_some(),
            },
        );
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

    /// CMSG_AUCTIONABLE_TOKEN_SELL — WoW Token sell request.
    ///
    /// The legacy C++ WotLK branch keeps this as an explicit empty stub because
    /// WoW Token is not available in WotLK.
    pub async fn handle_auctionable_token_sell(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = AuctionableTokenSell::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "AuctionableTokenSell parse failed: {error}"
            );
        }
    }

    /// CMSG_AUCTIONABLE_TOKEN_SELL_AT_MARKET_PRICE — WoW Token sell confirmation.
    ///
    /// The legacy C++ WotLK branch keeps this as an explicit empty stub because
    /// WoW Token is not available in WotLK.
    pub async fn handle_auctionable_token_sell_at_market_price(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        if let Err(error) = AuctionableTokenSellAtMarketPrice::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "AuctionableTokenSellAtMarketPrice parse failed: {error}"
            );
        }
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
        let icon_allows_interaction =
            represented_gameobject_icon_allows_interaction_like_cpp(&icon_name);
        self.record_represented_gameobject_icon_interaction_like_cpp(
            gameobject_guid,
            icon_allows_interaction,
        );
        if !icon_allows_interaction {
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

    pub(crate) fn represented_gameobject_gossip_can_interact_with_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        // The caller separately requires the current server-owned
        // InteractionData source and menu item. This helper revalidates the
        // represented GameObject half; constructing full scripted GO gossip
        // menus remains an explicit runtime boundary.
        if !gameobject_guid.is_game_object()
            || self.is_in_taxi_flight_like_cpp()
            || !self.player_is_strictly_in_world_like_cpp()
        {
            return None;
        }

        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        let go_type = state.go_type?;

        // C++ checks the immutable template icon on every lookup. Rust records
        // that fact while reading the same template in HandleGameObjectUse;
        // missing evidence fails closed rather than trusting historical
        // effects or canonical existence alone.
        if state.icon_name_allows_interaction_like_cpp != Some(true) {
            return None;
        }

        let map_key = self.current_canonical_player_map_key_like_cpp()?;
        {
            let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
            let map = manager.find_map(map_key.map_id, map_key.instance_id)?;
            let gameobject = map.map().get_typed_game_object(gameobject_guid)?;
            if !gameobject.world().object().is_in_world() {
                return None;
            }
            let gameobject_phase_shift = self
                .represented_gameobject_phase_shifts
                .get(&gameobject_guid)
                .unwrap_or_else(|| gameobject.world().phase_shift());
            if !self.can_see_phase_shift_like_cpp(gameobject_phase_shift) {
                return None;
            }
        }

        let interaction_distance = represented_gameobject_interaction_distance_like_cpp(
            Some(go_type),
            state.interact_radius_override,
        );
        self.represented_gameobject_can_interact_with_like_cpp(
            gameobject_guid,
            interaction_distance,
        )
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

        self.reset_player_interaction_if_source_like_cpp(request.source_guid);

        // C++ also clears Player::StableMaster when it matches SourceGuid. Rust
        // does not expose represented stable-master state yet.
    }
}

#[cfg(test)]
#[path = "misc_tests.rs"]
mod tests;

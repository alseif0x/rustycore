// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World server networking: TCP listener, per-client WorldSocket, and
//! the authentication handshake flow.

pub mod accept;
pub mod group_registry;
pub mod player_registry;
pub mod session_mgr;
pub mod world_socket;

pub use accept::{
    SocketTimeoutsLikeCpp, WorldListenerPolicyLikeCpp, start_instance_listener,
    start_world_listener,
};
pub use group_registry::{
    AcceptGroupInviteResultLikeCpp, CreateGroupInviteResultLikeCpp, EMPTY_TARGET_ICON_RAW_LIKE_CPP,
    GROUP_ASSIGN_MAINASSIST_LIKE_CPP, GROUP_ASSIGN_MAINTANK_LIKE_CPP,
    GROUP_FLAG_DESTROYED_LIKE_CPP, GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP, GROUP_FLAG_LFG_LIKE_CPP,
    GROUP_FLAG_RAID_LIKE_CPP, GroupAuthorityErrorLikeCpp, GroupDbRowLikeCpp, GroupInfo,
    GroupInstanceResetMethodLikeCpp, GroupInstanceResetResultLikeCpp, GroupLfgDbStateLikeCpp,
    GroupLoadSummaryLikeCpp, GroupMemberCharacterLikeCpp, GroupMemberDbRowLikeCpp,
    GroupMemberRemovalFactsLikeCpp, GroupMemberRemovalKindLikeCpp, GroupMemberSlotLikeCpp,
    GroupOwnedInstanceLikeCpp, GroupRecentInstanceLikeCpp, GroupRegistry,
    GroupTransitionOutcomeLikeCpp, LFG_GROUP_KICK_VOTES_NEEDED_LIKE_CPP,
    LFG_GROUP_MAX_KICKS_LIKE_CPP, LFG_STATE_DUNGEON_LIKE_CPP, LFG_STATE_FINISHED_DUNGEON_LIKE_CPP,
    LOOT_METHOD_PERSONAL_LIKE_CPP, MAX_GROUP_SIZE_LIKE_CPP, MAX_RAID_SIZE_LIKE_CPP,
    MAX_RAID_SUBGROUPS_LIKE_CPP, MEMBER_FLAG_ASSISTANT_LIKE_CPP, MEMBER_FLAG_MAINASSIST_LIKE_CPP,
    MEMBER_FLAG_MAINTANK_LIKE_CPP, MISSING_MEMBER_GROUP_LIKE_CPP, PendingInviteLikeCpp,
    PendingInvites, READYCHECK_DURATION_MS_LIKE_CPP, ReadyCheckEventLikeCpp,
    TARGET_ICONS_COUNT_LIKE_CPP, free_group_db_store_id_like_cpp,
    get_group_by_db_store_id_like_cpp, group_guid_by_db_store_id_like_cpp,
    load_groups_from_db_rows_like_cpp, register_group_db_store_id_like_cpp,
    tick_all_group_ready_checks_like_cpp,
};
pub use player_registry::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyLootMoneyLikeCppCommand,
    ApplyLootMoneyResultLikeCpp, CreatureAttackStartLikeCppCommand,
    CreatureAttackStopLikeCppCommand, DurableCreatureRuntimeCommandsLikeCpp,
    DurableLootMoneyAdmissionClosedLikeCpp, DurableLootMoneyCompletionLikeCpp,
    DurableLootMoneyPersistenceGuardLikeCpp, DurableLootMoneyPersistenceTrackerLikeCpp,
    DurableLootMoneySaveFenceLikeCpp, GameEventQuestCompleteClientOutcomeLikeCpp,
    GameEventQuestCompleteCommandLikeCpp, GameEventQuestCompleteResponseLikeCpp,
    KickLikeCppCommand, LootRollCommandIdentityLikeCpp, LootRollStoreWinnerCommand,
    LootRollVoteCommand, MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP, MasterLootGiveCommand,
    MasterLootGiveResult, NotifyLootMoneyRemovedLikeCppCommand, PlayerAggroCandidateSnapshot,
    PlayerBroadcastInfo, PlayerControlAddress, PlayerDirectoryReliableSendOutcome,
    PlayerDirectorySendError, PlayerGroupRewardSnapshot, PlayerLootContextSnapshot,
    PlayerLootPresenceSnapshot, PlayerMovementDirectoryUpdate, PlayerQuestSharingSnapshot,
    PlayerRegistration, PlayerRegistry, PlayerRuntimeRecipient, PlayerVehicleInteractionSnapshot,
    PlayerVisibilityCreateSnapshot, PrepareLootMoneyApplicationLikeCpp,
    PreparedLootMoneyApplicationLikeCpp, RefreshVisibleWorldCreaturesLikeCppCommand,
    ResetSeasonalQuestStatusCommand, SendAddonIfRegisteredLikeCppCommand,
    SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand,
    SendPartyUpdateLikeCppCommand, SendRealmPacketLikeCppCommand,
    SendVisibleObjectValuesUpdateCommand, SessionCommand, SharedClientVisibleGuidsLikeCpp,
    WorldSessionShutdownFlushLikeCppCommand, WorldSessionShutdownFlushResultLikeCpp,
};
pub use session_mgr::{InstanceLink, SessionManager};
pub use world_socket::{
    AccountInfo, SocketReader, SocketWriteFenceLikeCpp, SocketWriteFenceWaitResultLikeCpp,
    SocketWriter, WorldSocket, WorldSocketError,
};

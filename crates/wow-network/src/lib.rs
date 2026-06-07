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
    LootDropRatesLikeCpp, ReputationRatesLikeCpp, SessionResources, start_instance_listener,
    start_world_listener,
};
pub use group_registry::{
    EMPTY_TARGET_ICON_RAW_LIKE_CPP, GROUP_FLAG_LFG_LIKE_CPP, GROUP_FLAG_RAID_LIKE_CPP,
    GroupDbRowLikeCpp, GroupInfo, GroupLfgDbStateLikeCpp, GroupLoadSummaryLikeCpp,
    GroupMemberCharacterLikeCpp, GroupMemberDbRowLikeCpp, GroupMemberSlotLikeCpp, GroupRegistry,
    LFG_STATE_DUNGEON_LIKE_CPP, LFG_STATE_FINISHED_DUNGEON_LIKE_CPP, LOOT_METHOD_PERSONAL_LIKE_CPP,
    MAX_GROUP_SIZE_LIKE_CPP, MAX_RAID_SIZE_LIKE_CPP, MAX_RAID_SUBGROUPS_LIKE_CPP,
    MISSING_MEMBER_GROUP_LIKE_CPP, PendingInvites, TARGET_ICONS_COUNT_LIKE_CPP,
    free_group_db_store_id_like_cpp, get_group_by_db_store_id_like_cpp,
    group_guid_by_db_store_id_like_cpp, load_groups_from_db_rows_like_cpp,
    register_group_db_store_id_like_cpp,
};
pub use player_registry::{
    ApplyCreatureMeleeDamageLikeCppCommand, CreatureAttackStartLikeCppCommand,
    GameEventQuestCompleteClientOutcomeLikeCpp, GameEventQuestCompleteCommandLikeCpp,
    GameEventQuestCompleteResponseLikeCpp, LootRollStoreWinnerCommand, LootRollVoteCommand,
    MasterLootGiveCommand, MasterLootGiveResult, PlayerBroadcastInfo, PlayerRegistry,
    RefreshVisibleWorldCreaturesLikeCppCommand, ResetSeasonalQuestStatusCommand,
    SendIfVisibleLikeCppCommand, SendVisibleObjectValuesUpdateCommand, SessionCommand,
};
pub use session_mgr::{InstanceLink, SessionManager};
pub use world_socket::{AccountInfo, SocketReader, SocketWriter, WorldSocket, WorldSocketError};

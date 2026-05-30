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
pub use group_registry::{GROUP_FLAG_RAID_LIKE_CPP, GroupInfo, GroupRegistry, PendingInvites};
pub use player_registry::{
    GameEventQuestCompleteClientOutcomeLikeCpp, GameEventQuestCompleteCommandLikeCpp,
    GameEventQuestCompleteResponseLikeCpp, LootRollStoreWinnerCommand, LootRollVoteCommand,
    MasterLootGiveCommand, MasterLootGiveResult, PlayerBroadcastInfo, PlayerRegistry,
    ResetSeasonalQuestStatusCommand, SendIfVisibleLikeCppCommand,
    SendVisibleObjectValuesUpdateCommand, SessionCommand,
};
pub use session_mgr::{InstanceLink, SessionManager};
pub use world_socket::{AccountInfo, SocketReader, SocketWriter, WorldSocket, WorldSocketError};

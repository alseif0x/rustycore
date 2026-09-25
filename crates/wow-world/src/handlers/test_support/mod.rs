//! Shared test fixtures for handler scenarios under [`crate::handlers`].
//!
//! Extracted from `handlers/misc/tests/mod.rs` under #584 so every domain test
//! module can reach the same entry, packet and session fixtures from anywhere
//! below `handlers/`. Moving fixtures moves no invariant: the production module
//! boundary, its visibility and its owners are untouched.
//!
//! The fixtures live in `entries`, `packets` and `world`; the re-exports below
//! are the prelude that used to sit in `handlers/misc/tests/mod.rs`, including
//! the names that reached the old prelude through `use super::*;` from
//! `handlers::misc`.

pub(crate) mod entries;
pub(crate) mod packets;
pub(crate) mod world;

pub(crate) use crate::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp,
};
pub(crate) use crate::session::mailbox::SessionCommand;
pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use std::sync::{Arc, Mutex, RwLock};
pub(crate) use wow_constants::{
    ClientOpcodes, ItemContext, ServerOpcodes, shared::DifficultyFlags,
};
pub(crate) use wow_core::{ObjectGuid, Position, guid::HighGuid};
pub(crate) use wow_data::area::AREA_FLAG_IS_SUBZONE_LIKE_CPP;
pub(crate) use wow_data::progression_rewards::{FactionEntry, FactionStore};
pub(crate) use wow_data::quest::{
    QUEST_ITEM_DROP_COUNT, QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_CURRENCY_COUNT,
    QUEST_REWARD_DISPLAY_SPELL_COUNT, QUEST_REWARD_ITEM_COUNT, QUEST_REWARD_REPUTATIONS_COUNT,
    QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP, QuestStore, QuestTemplate,
};
pub(crate) use wow_data::reputation::{ReputationFlagsLikeCpp, ReputationRankLikeCpp};
pub(crate) use wow_data::{
    DifficultyStore, GraveyardStore, ItemRecord, ItemSearchNameEntry, ItemSearchNameStore,
    ItemSparseTemplateEntry, ItemStatsStore, ItemStore, MapDifficultyEntry, MapDifficultyStore,
    MapEntry, MapStore, SpellStore,
};
pub(crate) use wow_packet::ServerPacket;
pub(crate) use wow_packet::WorldPacket;
pub(crate) use wow_packet::packets::misc::TRADE_STATUS_INITIATED_LIKE_CPP;
pub(crate) use wow_packet::packets::misc::compress_account_data_like_cpp;
pub(crate) use wow_packet::packets::misc::{
    EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, TRADE_STATUS_FAILED_LIKE_CPP,
};
pub(crate) use wow_packet::packets::misc::{
    SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP, empty_battle_pet_guid_like_cpp,
};
pub(crate) use wow_packet::packets::misc::{
    TRADE_STATUS_ACCEPTED_LIKE_CPP, TRADE_STATUS_STATE_CHANGED_LIKE_CPP,
    TRADE_STATUS_UNACCEPTED_LIKE_CPP,
};
pub(crate) use wow_social::group::{
    DIFFICULTY_NORMAL_LIKE_CPP, GROUP_FLAG_LFG_LIKE_CPP, GroupInfo, GroupRegistry, PendingInvites,
};

// Names the fixture consumers used to reach through `use super::*;` from the
// former production `misc` module. Only the ones the scenarios actually use are
// kept; `item_purchase_contents_from_extended_cost` now reaches the entity
// scenarios directly through `handlers::entities::tests::*`.
pub(crate) use crate::session::registry::PacketHandlerEntry;
pub(crate) use wow_constants::ItemExtendedCostFlags;
pub(crate) use wow_handler::{PacketProcessing, SessionStatus};
pub(crate) use wow_packet::packets::chat::{
    JoinChannel, MAX_CHANNEL_NAME_STR_LIKE_CPP, MAX_CHANNEL_PASS_STR_LIKE_CPP,
};
pub(crate) use wow_packet::packets::misc::{
    CalendarAddEvent, CalendarCommunityInvite, CalendarComplain, CalendarCopyEvent,
    CalendarEventSignUp, CalendarGetEvent, CalendarInvite, CalendarModeratorStatusQuery,
    CalendarRemoveEvent, CalendarRemoveInvite, CalendarRsvp, CalendarStatus, CalendarUpdateEvent,
};

pub(crate) use entries::*;
pub(crate) use packets::*;
pub(crate) use world::*;

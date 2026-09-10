//! Behaviour tests for [`super`].
//!
//! Extracted from `misc.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

mod account_data;
mod arena;
mod auction;
mod battle_pet;
mod calendar;
mod chat;
mod client_state;
mod collections;
mod corpse;
mod entries;
mod gameobject;
mod guild;
mod instance;
mod lfg;
mod packets;
mod player;
mod pvp;
mod reputation;
mod support;
mod trade;
mod travel;
mod world;

use super::*;
use crate::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp,
};
use crate::session::mailbox::SessionCommand;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use wow_constants::{ClientOpcodes, ItemContext, ServerOpcodes, shared::DifficultyFlags};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_data::area::AREA_FLAG_IS_SUBZONE_LIKE_CPP;
use wow_data::progression_rewards::{FactionEntry, FactionStore};
use wow_data::quest::{
    QUEST_ITEM_DROP_COUNT, QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_CURRENCY_COUNT,
    QUEST_REWARD_DISPLAY_SPELL_COUNT, QUEST_REWARD_ITEM_COUNT, QUEST_REWARD_REPUTATIONS_COUNT,
    QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP, QuestStore, QuestTemplate,
};
use wow_data::reputation::{ReputationFlagsLikeCpp, ReputationRankLikeCpp};
use wow_data::{
    DifficultyStore, GraveyardStore, ItemRecord, ItemSearchNameEntry, ItemSearchNameStore,
    ItemSparseTemplateEntry, ItemStatsStore, ItemStore, MapDifficultyEntry, MapDifficultyStore,
    MapEntry, MapStore, SpellStore,
};
use wow_packet::ServerPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::misc::TRADE_STATUS_INITIATED_LIKE_CPP;
use wow_packet::packets::misc::compress_account_data_like_cpp;
use wow_packet::packets::misc::{
    EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, TRADE_STATUS_FAILED_LIKE_CPP,
};
use wow_packet::packets::misc::{SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP, empty_battle_pet_guid_like_cpp};
use wow_packet::packets::misc::{
    TRADE_STATUS_ACCEPTED_LIKE_CPP, TRADE_STATUS_STATE_CHANGED_LIKE_CPP,
    TRADE_STATUS_UNACCEPTED_LIKE_CPP,
};
use wow_social::group::{
    DIFFICULTY_NORMAL_LIKE_CPP, GROUP_FLAG_LFG_LIKE_CPP, GroupInfo, GroupRegistry, PendingInvites,
};

use entries::{area_entry, cuf_profile, difficulty_entry, map_entry, trade_test_spell_info};
use entries::{
    battleground_queue_id_like_cpp, battlemaster_entry_like_cpp, currency_entry,
    difficulty_entry_with_toggle, graveyard_store_with_links, graveyard_team_condition,
};
use packets::{
    accept_trade_packet, accept_wargame_invite_packet, activate_taxi_packet,
    auto_guild_bank_item_packet, auto_store_guild_bank_item_packet,
    battle_pet_clear_fanfare_packet, battle_pet_delete_pet_packet, battle_pet_modify_name_packet,
    battle_pet_request_journal_lock_packet, battle_pet_request_journal_packet,
    battle_pet_set_battle_slot_packet, battle_pet_set_flags_packet, battle_pet_summon_packet,
    battle_pet_update_display_notify_packet, battle_pet_update_notify_packet,
    battlefield_list_packet, battlefield_port_packet, battlemaster_hello_packet,
    battlemaster_join_arena_packet, battlemaster_join_packet, battlemaster_join_skirmish_packet,
    bug_report_packet, cage_battle_pet_packet, can_duel_packet, clear_trade_item_packet,
    collection_item_set_favorite_packet, decline_petition_packet, dismiss_critter_packet,
    duel_response_packet, guild_bank_activate_packet, guild_bank_buy_tab_packet,
    guild_bank_money_packet, guild_bank_query_tab_packet, guild_bank_set_tab_text_packet,
    guild_bank_tab_query_packet, guild_bank_update_tab_packet, object_update_recovery_packet,
    port_graveyard_packet, query_battle_pet_name_packet, query_petition_packet,
    read_cemetery_list_response, reclaim_corpse_packet, repop_request_packet,
    request_account_data_packet, request_cemetery_list_packet, resurrect_response_packet,
    save_cuf_profiles_packet, set_difficulty_request, set_dungeon_difficulty_request,
    set_raid_difficulty_request, set_trade_gold_packet, set_trade_item_packet,
    set_trade_spell_packet, sign_petition_packet, stand_state_change_packet,
    submit_user_feedback_packet, support_ticket_submit_bug_packet,
    support_ticket_submit_complaint_packet, support_ticket_submit_suggestion_packet,
    update_account_data_packet,
};
use world::make_session;
use world::{
    add_canonical_auctioneer_for_misc_test, add_canonical_flight_master_for_misc_test,
    add_canonical_test_player_on_map_for_misc_test, broadcast_info_with_command_tx,
    insert_trade_test_item, install_add_toy_item_templates,
    install_pending_bind_instance_context_like_cpp, install_represented_guild_bank_like_cpp,
    install_trade_test_spell, make_session_with_realm_send, quest_template,
    register_misc_test_creature, shared_canonical_map_manager_for_misc_test, unique_temp_data_dir,
    write_no_area_map_file_like_cpp,
};

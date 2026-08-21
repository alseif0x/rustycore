// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for miscellaneous client opcodes:
//! SetSelection, AreaTrigger, RequestCemeteryList,
//! TaxiNodeStatusQuery, ChatJoinChannel.

mod account_data;
mod arena;
mod auction;
mod battle_pet;
mod calendar;
mod chat;
mod client_state;
mod collections;
mod corpse;
mod gameobject;
mod guild;
mod instance;
mod lfg;
mod player;
mod pvp;
mod reputation;
mod support;
mod trade;
mod travel;

use wow_constants::{ClientOpcodes, ItemExtendedCostFlags};
use wow_database::{CharStatements, PreparedStatement};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::chat::{
    JoinChannel, MAX_CHANNEL_NAME_STR_LIKE_CPP, MAX_CHANNEL_PASS_STR_LIKE_CPP,
};
use wow_packet::packets::item::{
    ItemPurchaseContents, ItemPurchaseRefundCurrency, ItemPurchaseRefundItem,
};
use wow_packet::packets::misc::{
    BugReport, CalendarAddEvent, CalendarCommandResult, CalendarCommunityInvite, CalendarComplain,
    CalendarCopyEvent, CalendarEventSignUp, CalendarGetEvent, CalendarInvite,
    CalendarModeratorStatusQuery, CalendarRemoveEvent, CalendarRemoveInvite, CalendarRsvp,
    CalendarSendCalendar, CalendarSendNumPending, CalendarStatus, CalendarUpdateEvent,
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
    let mut stmt = PreparedStatement::for_statement(CharStatements::INS_BUG_REPORT);
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

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler rules that read no session state.
//!
//! Moved out of the owner under #680. Every one was already receiver-free,
//! so it cannot read or write the owner's state: these are rules, not owner
//! behaviour. Bodies and signatures are unchanged.

use crate::handlers::character::CHAR_NAME_INVALID_CHARACTER_LIKE_CPP;
use crate::handlers::character::CHAR_NAME_NO_NAME_LIKE_CPP;
use crate::handlers::character::CHAR_NAME_TOO_LONG_LIKE_CPP;
use crate::handlers::character::CHAR_NAME_TOO_SHORT_LIKE_CPP;
use crate::handlers::character::CreatureAddonCreateFieldsLikeCpp;
use crate::handlers::character::RESPONSE_SUCCESS_LIKE_CPP;
use crate::session::*;
use wow_constants::UnitStandStateType;
use wow_constants::unit::SheathState;
use wow_data::ConditionEntriesByTypeStore;
use wow_data::PlayerConditionContextLikeCpp;
use wow_data::PlayerConditionStore;
use wow_entities::CreatureAddonLifecycleRecordLikeCpp;
use wow_entities::WorldObject;

pub(crate) fn represented_character_rename_name_result_like_cpp(name: &str) -> u8 {
    if name.is_empty() {
        return CHAR_NAME_NO_NAME_LIKE_CPP;
    }
    if name.len() < 2 {
        return CHAR_NAME_TOO_SHORT_LIKE_CPP;
    }
    if name.len() > 12 {
        return CHAR_NAME_TOO_LONG_LIKE_CPP;
    }
    if !name.chars().all(|c| c.is_ascii_alphabetic()) {
        return CHAR_NAME_INVALID_CHARACTER_LIKE_CPP;
    }

    RESPONSE_SUCCESS_LIKE_CPP
}

pub(crate) fn vendor_item_conditions_meet_like_cpp(
    condition_store: &ConditionEntriesByTypeStore,
    creature_entry: u32,
    item_id: u32,
    player_object: Option<&WorldObject>,
    vendor_object: Option<&WorldObject>,
    player_unit_snapshot: crate::conditions::ConditionUnitSnapshot,
    player_snapshot: crate::conditions::ConditionPlayerSnapshot,
    vendor_unit_snapshot: Option<crate::conditions::ConditionUnitSnapshot>,
    player_condition_store: Option<&PlayerConditionStore>,
    player_condition_context: Option<PlayerConditionContextLikeCpp<'_>>,
) -> bool {
    crate::conditions::is_object_meeting_vendor_item_conditions_like_cpp(
        condition_store,
        creature_entry,
        item_id,
        player_object,
        vendor_object,
        |condition, source_info| {
            source_info.set_unit_target_snapshot(0, player_unit_snapshot);
            source_info.set_player_target_snapshot(0, player_snapshot);
            if let Some(vendor_unit_snapshot) = vendor_unit_snapshot {
                source_info.set_unit_target_snapshot(1, vendor_unit_snapshot);
            }
            if let (Some(store), Some(context)) = (player_condition_store, player_condition_context)
            {
                source_info.set_player_condition_store(store);
                source_info.set_player_condition_context(0, context);
            }
            match crate::conditions::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |current_area, required_area| current_area == required_area,
            ) {
                crate::conditions::ConditionMeetResult::Evaluated(value) => value,
                crate::conditions::ConditionMeetResult::Unsupported => false,
            }
        },
    )
}

pub(crate) fn vendor_stock_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

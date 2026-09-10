// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handler rules that read no session state.
//!
//! Moved out of the owner under #680. Every one was already receiver-free,
//! so it cannot read or write the owner's state: these are rules, not owner
//! behaviour. Bodies and signatures are unchanged.

use crate::handlers::loot::ROLL_ALL_TYPE_MASK_LIKE_CPP;
use crate::handlers::loot::ROLL_FLAG_TYPE_DISENCHANT_LIKE_CPP;
use crate::handlers::loot::ROLL_FLAG_TYPE_NEED_LIKE_CPP;
use crate::session::mailbox::LootRollCommandIdentityLikeCpp;
use crate::session::mailbox::LootRollVoteCommand;
use crate::session::*;
use wow_constants::ItemFlags2;

pub(crate) fn represented_loot_roll_valid_rolls_like_cpp(
    item_flags2: Option<u32>,
    disenchant_skill_required: Option<u16>,
    max_enchanting_skill: u16,
) -> u8 {
    let mut valid_rolls = ROLL_ALL_TYPE_MASK_LIKE_CPP;
    if item_flags2.is_some_and(|flags| (flags & ItemFlags2::CanOnlyRollGreed as u32) != 0) {
        valid_rolls &= !ROLL_FLAG_TYPE_NEED_LIKE_CPP;
    }
    if disenchant_skill_required.is_none_or(|skill_required| skill_required > max_enchanting_skill)
    {
        valid_rolls &= !ROLL_FLAG_TYPE_DISENCHANT_LIKE_CPP;
    }

    valid_rolls
}

pub(crate) fn represented_loot_roll_vote_command_targets_identity_like_cpp(
    command: &LootRollVoteCommand,
    current_identity: &LootRollCommandIdentityLikeCpp,
) -> bool {
    current_identity.matches_key_like_cpp(command.loot_obj, command.loot_list_id)
        && current_identity.is_exact_roll_like_cpp(&command.roll_identity)
}

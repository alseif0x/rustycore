// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Roll eligibility and winner selection from caller-owned vote values.
//!
//! No random draw, timer, Session, authority handle or packet is retained here.
//! Ties deliberately preserve the caller HashMap's existing iteration order.

use std::collections::HashMap;
use wow_constants::ItemFlags2;
use wow_core::ObjectGuid;

#[cfg(test)]
mod tests;

pub const ROLL_ALL_TYPE_MASK_LIKE_CPP: u8 = 0x0F;
pub const ROLL_FLAG_TYPE_NEED_LIKE_CPP: u8 = 0x02;
pub const ROLL_FLAG_TYPE_DISENCHANT_LIKE_CPP: u8 = 0x08;
pub const ROLL_VOTE_PASS_LIKE_CPP: u8 = 0;
pub const ROLL_VOTE_NEED_LIKE_CPP: u8 = 1;
pub const ROLL_VOTE_GREED_LIKE_CPP: u8 = 2;
pub const ROLL_VOTE_DISENCHANT_LIKE_CPP: u8 = 3;
pub const ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP: u8 = 4;
pub const ROLL_VOTE_NOT_VALID_LIKE_CPP: u8 = 5;

pub fn represented_loot_roll_finish_winner_like_cpp(
    voters: &HashMap<ObjectGuid, RepresentedLootRollVote>,
) -> Option<Option<(ObjectGuid, RepresentedLootRollVote)>> {
    let mut winner = None;
    let mut has_need = false;

    for (player_guid, vote) in voters {
        match vote.vote {
            ROLL_VOTE_NEED_LIKE_CPP => {
                if !has_need
                    || winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    has_need = true;
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                if !has_need
                    && winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_PASS_LIKE_CPP | ROLL_VOTE_NOT_VALID_LIKE_CPP => {}
            ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP => return None,
            _ => {}
        }
    }

    Some(winner)
}

pub fn represented_loot_roll_current_winner_like_cpp(
    voters: &HashMap<ObjectGuid, RepresentedLootRollVote>,
) -> Option<(ObjectGuid, RepresentedLootRollVote)> {
    let mut winner = None;
    let mut has_need = false;

    for (player_guid, vote) in voters {
        match vote.vote {
            ROLL_VOTE_NEED_LIKE_CPP => {
                if !has_need
                    || winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    has_need = true;
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                if !has_need
                    && winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_PASS_LIKE_CPP
            | ROLL_VOTE_NOT_VALID_LIKE_CPP
            | ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP => {}
            _ => {}
        }
    }

    winner
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedLootRollVote {
    pub vote: u8,
    pub roll_number: u8,
}

pub fn represented_loot_roll_valid_rolls_like_cpp(
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

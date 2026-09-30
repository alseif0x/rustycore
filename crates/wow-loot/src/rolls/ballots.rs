//! Ballot formation, vote mutation and winner resolution for one loot roll.
//!
//! C++ anchors: Loot.cpp::LootRoll::TryToStart (398), PlayerVote (454),
//! UpdateRoll (498), AllPlayerVoted (520), and Finish (575).
//! Presence, deadline, identity, authority, packets and storage stay in application.
//! The existing Rust draw-before-membership and unknown-vote behavior is retained.

use std::collections::HashMap;

use super::{
    ROLL_VOTE_DISENCHANT_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP, ROLL_VOTE_NEED_LIKE_CPP,
    ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP, ROLL_VOTE_PASS_LIKE_CPP,
    RepresentedLootRollVote, represented_loot_roll_current_winner_like_cpp,
    represented_loot_roll_finish_winner_like_cpp,
};
use crate::LootEntry;
use wow_core::ObjectGuid;

#[cfg(test)]
mod tests;

/// The sole vote map for a roll; cloned only with the existing application roll snapshots.
#[derive(Debug, Clone)]
pub struct RollBallots {
    voters: HashMap<ObjectGuid, RepresentedLootRollVote>,
}

impl RollBallots {
    pub fn start(
        entry: &mut LootEntry,
        eligible_count: usize,
        current_player: ObjectGuid,
        current_pass: bool,
        mut resolve_pass: impl FnMut(ObjectGuid) -> Option<bool>,
    ) -> Option<Self> {
        if eligible_count <= 1 {
            entry.flags.under_threshold = true;
            entry.flags.blocked = false;
            return None;
        }

        let mut voters = HashMap::new();
        for looter in &entry.allowed_looters {
            let vote = if *looter == current_player {
                if current_pass {
                    ROLL_VOTE_PASS_LIKE_CPP
                } else {
                    ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
                }
            } else {
                match resolve_pass(*looter) {
                    Some(pass_on_group_loot) => {
                        if pass_on_group_loot {
                            ROLL_VOTE_PASS_LIKE_CPP
                        } else {
                            ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
                        }
                    }
                    _ => ROLL_VOTE_NOT_VALID_LIKE_CPP,
                }
            };
            voters.insert(
                *looter,
                RepresentedLootRollVote {
                    vote,
                    roll_number: 0,
                },
            );
        }
        Some(Self { voters })
    }

    /// Called before the application's second roll lookup and voter membership check.
    /// The outer None rejects an unknown vote; inner None preserves a Pass's old number.
    pub fn prepare_vote(vote: u8, draw: impl FnOnce() -> u8) -> Option<Option<u8>> {
        match vote {
            ROLL_VOTE_PASS_LIKE_CPP => Some(None),
            ROLL_VOTE_NEED_LIKE_CPP | ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                Some(Some(draw()))
            }
            _ => None,
        }
    }

    pub fn record_vote(&mut self, player: ObjectGuid, vote: u8, number: Option<u8>) -> bool {
        let Some(voter) = self.voters.get_mut(&player) else {
            return false;
        };
        voter.vote = vote;
        if let Some(number) = number {
            voter.roll_number = number;
        }
        true
    }

    pub fn finished_winner(&self) -> Option<Option<(ObjectGuid, RepresentedLootRollVote)>> {
        represented_loot_roll_finish_winner_like_cpp(&self.voters)
    }

    pub fn current_winner(&self) -> Option<(ObjectGuid, RepresentedLootRollVote)> {
        represented_loot_roll_current_winner_like_cpp(&self.voters)
    }

    pub fn vote(&self, player: ObjectGuid) -> Option<&RepresentedLootRollVote> {
        self.voters.get(&player)
    }

    pub fn votes(&self) -> impl Iterator<Item = (&ObjectGuid, &RepresentedLootRollVote)> {
        self.voters.iter()
    }
}

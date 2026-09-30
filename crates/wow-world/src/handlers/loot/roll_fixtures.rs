//! Borrowed observations of the actual application roll lifetime.
use super::*;
use crate::session::mailbox::LootRollCommandIdentityLikeCpp;

pub struct LootRollObservation<'a>(&'a RepresentedLootRollState);

impl LootRollObservation<'_> {
    pub fn vote(&self, player: ObjectGuid) -> Option<&wow_loot::RepresentedLootRollVote> {
        self.0.ballots.vote(player)
    }
    pub fn command_identity(&self) -> &LootRollCommandIdentityLikeCpp { &self.0.command_identity }
}

pub fn loot_roll_observation_for_test(session: &WorldSession, loot: ObjectGuid, index: u8) -> Option<LootRollObservation<'_>> {
    session.represented_loot_rolls.get(&(loot, index)).map(LootRollObservation)
}

impl LootRollObservation<'_> {
    pub fn owner_guid(&self) -> ObjectGuid { self.0.owner_guid }
    pub fn authority_generation(&self) -> u64 { self.0.authority_generation }
}

/// Controls the actual lifetime deadline, without creating a second roll.
pub fn set_loot_roll_deadline_for_test(session: &mut WorldSession, loot: ObjectGuid, index: u8, deadline: Instant) {
    session.represented_loot_rolls.get_mut(&(loot, index)).unwrap().end_time = deadline;
}

pub async fn tick_loot_rolls_for_test(session: &mut WorldSession) {
    let generators = session.id_generators_for_test_like_cpp();
    let item_valuation = session.item_valuation_catalogs_for_test_like_cpp();
    session.tick_represented_loot_rolls_with_generator_like_cpp(
        generators.item.as_ref(),
        &item_valuation,
    ).await;
}

pub fn loot_opened_cache_generation_for_test(session: &WorldSession, owner: ObjectGuid) -> Option<u64> {
    session.represented_loot_cache_generations_like_cpp.get(&owner).copied()
}

/// Expected values wrap the existing private callback event type.
#[derive(Debug)]
pub struct LootCriterionExpectation(crate::session::RepresentedLootRollCriteriaEvent);

impl LootCriterionExpectation {
    pub fn any_need(player_guid: ObjectGuid, quantity: u32) -> Self {
        Self(crate::session::RepresentedLootRollCriteriaEvent::RollAnyNeed { player_guid, quantity })
    }
    pub fn any_greed(player_guid: ObjectGuid, quantity: u32) -> Self {
        Self(crate::session::RepresentedLootRollCriteriaEvent::RollAnyGreed { player_guid, quantity })
    }
}

/// Borrows the event recorded at the real criterion callback.
#[derive(Debug)]
pub struct LootCriterion<'a>(&'a crate::session::RepresentedLootRollCriteriaEvent);

impl PartialEq<LootCriterionExpectation> for LootCriterion<'_> {
    fn eq(&self, expected: &LootCriterionExpectation) -> bool { self.0 == &expected.0 }
}

impl LootCriterion<'_> {
    pub fn need(&self) -> Option<(ObjectGuid, u32, u8)> {
        match *self.0 {
            crate::session::RepresentedLootRollCriteriaEvent::RollNeed { player_guid, item_id, roll_number } => Some((player_guid, item_id, roll_number)),
            _ => None,
        }
    }
    pub fn greed(&self) -> Option<(ObjectGuid, u32, u8)> {
        match *self.0 {
            crate::session::RepresentedLootRollCriteriaEvent::RollGreed { player_guid, item_id, roll_number } => Some((player_guid, item_id, roll_number)),
            _ => None,
        }
    }
}

pub fn loot_criterion_for_test(session: &WorldSession, index: usize) -> LootCriterion<'_> {
    LootCriterion(&session.represented_loot_roll_criteria_events[index])
}

pub fn loot_criteria_empty_for_test(session: &WorldSession) -> bool {
    session.represented_loot_roll_criteria_events.is_empty()
}

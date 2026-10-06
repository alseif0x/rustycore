// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! One selected-owner context for the complete quest reward operation.

mod accessors;
mod commit;
mod coordinator;
mod currencies;
mod effects;
mod grants;
mod item_planning;
mod item_storage;
mod lockout;
mod publication;
mod removals;
mod reputation;
mod validation;
mod xp;
mod xp_grants;

use wow_data::{CurrencyTypesStore, quest_xp::QuestXpStore};
use wow_world_core::session::mailbox::GameEventQuestCompleteClientOutcomeLikeCpp;
use wow_world_core::session::{
    QuestRewardPlayerAccessLikeCpp, SessionCatalogs, SessionWorldConfig,
};
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_social::SessionSocialLimits;

use super::{RepresentedQuestObjectiveProgressEventLikeCpp, SessionQuestState};

#[cfg(any(test, feature = "test-fixtures"))]
pub use item_planning::QuestRewardItemPlanningFixtureRefsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use reputation::QuestRewardReputationFixtureRefsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use xp_grants::QuestXpGainFixtureRefsLikeCpp;

pub struct QuestRewardCx<'a> {
    pub(super) inventory: &'a mut InventoryState,
    pub(super) lifecycle: &'a mut SessionLifecycleState,
    pub(super) quest_state: &'a mut SessionQuestState,
    pub(super) player: QuestRewardPlayerAccessLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) player_level: &'a mut u8,
    pub(super) catalogs: &'a SessionCatalogs,
    pub(super) config: &'a SessionWorldConfig,
    pub(super) social: &'a SessionSocialLimits,
    pub(super) currency_types: Option<&'a CurrencyTypesStore>,
    pub(super) quest_xp_store: Option<&'a QuestXpStore>,
    pub(super) world_test_consumer: bool,
}

impl<'a> QuestRewardCx<'a> {
    pub fn new(
        inventory: &'a mut InventoryState,
        lifecycle: &'a mut SessionLifecycleState,
        quest_state: &'a mut SessionQuestState,
        player: QuestRewardPlayerAccessLikeCpp<'a>,
        #[cfg(any(test, feature = "test-fixtures"))] player_level: &'a mut u8,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
        social: &'a SessionSocialLimits,
        currency_types: Option<&'a CurrencyTypesStore>,
        quest_xp_store: Option<&'a QuestXpStore>,
        world_test_consumer: bool,
    ) -> Self {
        Self {
            inventory,
            lifecycle,
            quest_state,
            player,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level,
            catalogs,
            config,
            social,
            currency_types,
            quest_xp_store,
            world_test_consumer,
        }
    }

    pub fn remove_represented_timed_quest_like_cpp(
        quest_state: &mut SessionQuestState,
        player: &mut QuestRewardPlayerAccessLikeCpp<'_>,
        quest_id: u32,
        world_test_consumer: bool,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        let fixture_owner = world_test_consumer && player.owner_handle_absent_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let removed = if fixture_owner {
            let mut state = quest_state.player_quest_gameplay_fixture_like_cpp();
            let removed = match state.status_mut_like_cpp(quest_id) {
                Some(status) if status.end_time_secs > 0 => {
                    status.end_time_secs = 0;
                    true
                }
                _ => false,
            };
            if removed {
                quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
            }
            removed
        } else {
            player.clear_quest_reward_end_time_like_cpp(quest_id)
        };
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let removed = player.clear_quest_reward_end_time_like_cpp(quest_id);

        #[cfg(any(test, feature = "test-fixtures"))]
        if world_test_consumer
            && !fixture_owner
            && let Some(state) = player.quest_gameplay_snapshot_like_cpp()
        {
            quest_state.apply_player_quest_core_compatibility_like_cpp(&state);
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if removed && world_test_consumer {
            quest_state.fixture_record_timed_quest_removal_like_cpp(quest_id);
        }

        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            let _ = (removed, world_test_consumer);
        }
    }

    pub fn set_can_delay_teleport_like_cpp(
        &mut self,
        #[cfg(any(test, feature = "test-fixtures"))]
        teleport_fixture: &mut wow_world_core::session::state::TeleportState,
        can_delay: bool,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.player
            .set_can_delay_teleport_like_cpp(teleport_fixture, can_delay);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        self.player.set_can_delay_teleport_like_cpp(can_delay);
    }

    pub fn settle_rewarded_quest_like_cpp(&mut self, quest_id: u32, repeatable: bool) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer && self.player.owner_handle_absent_like_cpp() {
            let mut state = self.quest_state.player_quest_gameplay_fixture_like_cpp();
            state.remove_status_like_cpp(quest_id);
            if !repeatable {
                state.set_rewarded_like_cpp(quest_id, true);
            }
            self.quest_state
                .apply_player_quest_gameplay_fixture_like_cpp(state);
            return true;
        }
        self.player
            .settle_rewarded_quest_like_cpp(quest_id, repeatable)
    }

    pub fn quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerQuestGameplayState> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer && self.player.owner_handle_absent_like_cpp() {
            return Some(self.quest_state.player_quest_gameplay_fixture_like_cpp());
        }
        self.player.quest_gameplay_snapshot_like_cpp()
    }

    pub fn enqueue_money_changed_like_cpp(&mut self, old_money: u64, new_money: u64) {
        self.quest_state
            .enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
    }

    pub async fn notify_game_event_quest_complete_like_cpp(
        &self,
        quest_id: u32,
    ) -> GameEventQuestCompleteClientOutcomeLikeCpp {
        self.player
            .notify_game_event_quest_complete_like_cpp(quest_id)
            .await
    }

    pub fn send_packet_like_cpp<P: wow_packet::ServerPacket>(&self, packet: &P) -> bool {
        self.player
            .packet_publication_access_like_cpp()
            .send_packet(packet)
    }
}

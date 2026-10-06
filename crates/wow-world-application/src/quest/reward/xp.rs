// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Required selected participants for the XP level-up stats publication phase.

#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::{BTreeMap, HashMap};

#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::StatsFixtureRefs;
use wow_world_core::session::{
    QuestRewardPlayerAccessLikeCpp, SessionCatalogs, SessionWorldConfig,
};
use wow_world_inventory::InventoryState;

use super::super::SessionQuestState;
use super::QuestRewardCx;

/// The synchronous XP stats phase borrows the mutable level and, in fixture
/// builds, every selected stats fixture reference required to project it.
struct QuestXpStatsUpdateCx<'access, 'player> {
    player: &'access QuestRewardPlayerAccessLikeCpp<'player>,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_level: &'access mut u8,
    catalogs: &'access SessionCatalogs,
    config: &'access SessionWorldConfig,
    inventory: &'access InventoryState,
    #[cfg(any(test, feature = "test-fixtures"))]
    stats_fixture_refs: StatsFixtureRefs<'access>,
}

impl<'access, 'player> QuestXpStatsUpdateCx<'access, 'player> {
    fn new(
        player: &'access QuestRewardPlayerAccessLikeCpp<'player>,
        #[cfg(any(test, feature = "test-fixtures"))] player_level: &'access mut u8,
        catalogs: &'access SessionCatalogs,
        config: &'access SessionWorldConfig,
        inventory: &'access InventoryState,
        #[cfg(any(test, feature = "test-fixtures"))] stats_fixture_refs: StatsFixtureRefs<'access>,
    ) -> Self {
        Self {
            player,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level,
            catalogs,
            config,
            inventory,
            #[cfg(any(test, feature = "test-fixtures"))]
            stats_fixture_refs,
        }
    }

    fn send_level_up_stat_update_like_cpp(&mut self) {
        let publication = self.player.packet_publication_access_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let player_stats = self.player.stats_access_like_cpp(
            self.catalogs,
            self.config,
            &*self.player_level,
            self.stats_fixture_refs.reborrow_like_cpp(),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let player_stats = self
            .player
            .stats_access_like_cpp(self.catalogs, self.config);
        let mut application = crate::CharacterStatsApplicationCxLikeCpp::new(
            player_stats,
            self.inventory,
            publication,
        );
        application.send_level_up_stat_update_like_cpp();
    }
}

impl super::QuestRewardCx<'_> {
    /// Run the XP stats phase with required selected references. The mutable
    /// level is reborrowed only for the stats query.
    pub fn send_level_up_stat_update_like_cpp(
        player: &QuestRewardPlayerAccessLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] player_level: &mut u8,
        catalogs: &SessionCatalogs,
        config: &SessionWorldConfig,
        inventory: &InventoryState,
        #[cfg(any(test, feature = "test-fixtures"))] stats_fixture_refs: StatsFixtureRefs<'_>,
    ) {
        let mut application = QuestXpStatsUpdateCx::new(
            player,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level,
            catalogs,
            config,
            inventory,
            #[cfg(any(test, feature = "test-fixtures"))]
            stats_fixture_refs,
        );
        application.send_level_up_stat_update_like_cpp();
    }
}

impl QuestRewardCx<'_> {
    /// Apply one level change and its talent-point refresh through the selected
    /// canonical owner. Quest-point values stay borrowed until Core reaches its
    /// original handle-less fallback.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_level_and_refresh_talent_points_like_cpp(
        player: &mut QuestRewardPlayerAccessLikeCpp<'_>,
        level: u8,
        catalogs: &SessionCatalogs,
        quest_state: &SessionQuestState,
        world_test_consumer: bool,
        player_level: &mut u8,
        gray_level_overrides: &HashMap<u8, u8>,
        talent_groups: &[BTreeMap<u32, u8>; wow_world_core::session::MAX_SPECIALIZATIONS_LIKE_CPP],
        active_talent_group: &u8,
        player_character_points: &mut i32,
    ) {
        let mut transition = player.player_level_transition_access_like_cpp(
            player_level,
            gray_level_overrides,
            talent_groups,
            active_talent_group,
            player_character_points,
        );
        transition.set_player_level_and_refresh_talent_points_like_cpp(
            level,
            catalogs,
            world_test_consumer,
            quest_state
                .fixture_represented_quest_reward_talent_points_like_cpp()
                .iter()
                .map(|reward| reward.points),
        );
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    pub fn set_player_level_and_refresh_talent_points_like_cpp(
        player: &mut QuestRewardPlayerAccessLikeCpp<'_>,
        level: u8,
        catalogs: &SessionCatalogs,
        world_test_consumer: bool,
    ) {
        let mut transition = player.player_level_transition_access_like_cpp();
        transition.set_player_level_and_refresh_talent_points_like_cpp(
            level,
            catalogs,
            world_test_consumer,
            std::iter::empty(),
        );
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn refresh_represented_talent_points_like_cpp(
        player: &mut QuestRewardPlayerAccessLikeCpp<'_>,
        catalogs: &SessionCatalogs,
        quest_state: &SessionQuestState,
        world_test_consumer: bool,
        player_level: &mut u8,
        gray_level_overrides: &HashMap<u8, u8>,
        talent_groups: &[BTreeMap<u32, u8>; wow_world_core::session::MAX_SPECIALIZATIONS_LIKE_CPP],
        active_talent_group: &u8,
        player_character_points: &mut i32,
    ) {
        let mut transition = player.player_level_transition_access_like_cpp(
            player_level,
            gray_level_overrides,
            talent_groups,
            active_talent_group,
            player_character_points,
        );
        transition.refresh_represented_talent_points_like_cpp(
            catalogs,
            world_test_consumer,
            quest_state
                .fixture_represented_quest_reward_talent_points_like_cpp()
                .iter()
                .map(|reward| reward.points),
        );
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    pub fn refresh_represented_talent_points_like_cpp(
        player: &mut QuestRewardPlayerAccessLikeCpp<'_>,
        catalogs: &SessionCatalogs,
        world_test_consumer: bool,
    ) {
        let mut transition = player.player_level_transition_access_like_cpp();
        transition.refresh_represented_talent_points_like_cpp(
            catalogs,
            world_test_consumer,
            std::iter::empty(),
        );
    }

    pub fn player_level_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.player.player_level_like_cpp(&*self.player_level)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.player.player_level_like_cpp()
        }
    }

    pub fn quest_xp_reward_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> u32 {
        let Some(quests) = self.quest_gameplay_snapshot_like_cpp() else {
            return 0;
        };
        if !Self::quest_xp_reward_allowed_like_cpp(quest, &quests) {
            return 0;
        }

        let player_level_for_quest_level = if quest.quest_level > 0 {
            0
        } else {
            self.player_level_like_cpp()
        };
        let quest_level = self
            .quest_state
            .player_quest_level_like_cpp(player_level_for_quest_level, quest);

        let player_level_for_xp = if self.quest_xp_store.is_some() {
            self.player_level_like_cpp()
        } else {
            0
        };
        Self::calculate_quest_xp_reward_like_cpp(
            self.quest_state,
            self.quest_xp_store,
            player_level_for_xp,
            quest.reward_xp_difficulty,
            quest_level,
            quest.reward_xp_multiplier,
        )
    }

    pub fn quest_xp_reward_allowed_like_cpp(
        quest: &wow_data::quest::QuestTemplate,
        quests: &wow_entities::PlayerQuestGameplayState,
    ) -> bool {
        !quests.rewarded_quest_ids_like_cpp().contains(&quest.id) || quest.is_df_quest_like_cpp()
    }

    pub fn calculate_quest_xp_reward_like_cpp(
        quest_state: &SessionQuestState,
        xp_store: Option<&wow_data::quest_xp::QuestXpStore>,
        player_level: u8,
        difficulty: u32,
        quest_level: i32,
        xp_multiplier: f32,
    ) -> u32 {
        quest_state.calculate_quest_xp_like_cpp(
            xp_store,
            player_level,
            difficulty,
            quest_level,
            xp_multiplier,
        )
    }
}

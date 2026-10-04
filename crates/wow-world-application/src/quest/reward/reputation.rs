// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordered reputation calculation, manager mutation and publication for quest rewards.

use std::sync::Arc;
use super::QuestRewardCx;
use super::super::RepresentedQuestRewardReputationSourceLikeCpp;
use wow_world_core::session::ReputationGainSourceLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub struct QuestRewardReputationFixtureRefsLikeCpp<'a> {
    state: &'a mut wow_entities::PlayerReputationStateLikeCpp,
    gray_overrides: &'a std::collections::HashMap<u8, u8>,
    position: &'a Option<wow_core::Position>,
    aura_authority: &'a bool,
    aura_tombstone: &'a bool,
    visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
    threat_auras: &'a std::collections::HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> QuestRewardReputationFixtureRefsLikeCpp<'a> {
    pub(in crate::quest) fn state_like_cpp(&self) -> &wow_entities::PlayerReputationStateLikeCpp {
        &*self.state
    }

    pub(in crate::quest) fn reborrow_like_cpp(&mut self) -> QuestRewardReputationFixtureRefsLikeCpp<'_> {
        QuestRewardReputationFixtureRefsLikeCpp {
            state: &mut *self.state,
            gray_overrides: self.gray_overrides,
            position: self.position,
            aura_authority: self.aura_authority,
            aura_tombstone: self.aura_tombstone,
            visible_auras: self.visible_auras,
            threat_auras: self.threat_auras,
        }
    }

    pub fn new_like_cpp(
        state: &'a mut wow_entities::PlayerReputationStateLikeCpp,
        gray_overrides: &'a std::collections::HashMap<u8, u8>,
        position: &'a Option<wow_core::Position>,
        aura_authority: &'a bool,
        aura_tombstone: &'a bool,
        visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
        threat_auras: &'a std::collections::HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
    ) -> Self {
        Self { state, gray_overrides, position, aura_authority, aura_tombstone,
            visible_auras, threat_auras }
    }
}

fn calculate_pct_i32_f32_like_cpp(base: i32, pct: f32) -> i32 {
    (base as f32 * pct / 100.0) as i32
}

impl QuestRewardCx<'_> {
    fn quest_reputation_gain_percent_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        quest_level: u32,
        rep: i32,
        no_quest_bonus: bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &QuestRewardReputationFixtureRefsLikeCpp<'_>,
    ) -> Option<f32> {
        let mut percent = 100.0f32;
        let rep_mod = if no_quest_bonus {
            0.0
        } else {
            let player = self.player.xp_gain_access_like_cpp(self.catalogs, self.config);
            #[cfg(any(test, feature = "test-fixtures"))]
            let modifier = player.resolved_total_represented_aura_modifier_from_selected_refs_like_cpp(
                wow_entities::RepresentedAuraEffectLikeCpp::ModReputationGain,
                fixtures.aura_authority, fixtures.aura_tombstone,
                fixtures.visible_auras, fixtures.threat_auras,
            )?;
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let modifier = player.resolved_total_represented_aura_modifier_like_cpp(
                wow_entities::RepresentedAuraEffectLikeCpp::ModReputationGain,
            )?;
            modifier as f32
        };
        percent += if rep > 0 { rep_mod } else { -rep_mod };
        let rates = self.config.reputation_rates_like_cpp();
        let low_level_rate = match source {
            ReputationGainSourceLikeCpp::Kill => rates.low_level_kill,
            ReputationGainSourceLikeCpp::Quest | ReputationGainSourceLikeCpp::DailyQuest
            | ReputationGainSourceLikeCpp::WeeklyQuest | ReputationGainSourceLikeCpp::MonthlyQuest
            | ReputationGainSourceLikeCpp::RepeatableQuest => rates.low_level_quest,
            ReputationGainSourceLikeCpp::Spell => 1.0,
        };
        if low_level_rate != 1.0 {
            let player_level = self.player.player_level_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.player_level,
            );
            if quest_level < u32::from(self.player.reward_reputation_gray_level_like_cpp(
                player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures.gray_overrides,
            )) {
                percent *= low_level_rate;
            }
        }
        (percent > 0.0).then_some(percent)
    }

    fn quest_reputation_raf_percent_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        mut percent: f32,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &QuestRewardReputationFixtureRefsLikeCpp<'_>,
    ) -> f32 {
        if source != ReputationGainSourceLikeCpp::Spell
            && Self::gets_recruit_a_friend_bonus_like_cpp(
                self.player.xp_gain_access_like_cpp(self.catalogs, self.config),
                self.social, self.config, false, self.world_test_consumer,
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures.position,
            )
        {
            percent *= 1.0 + self.config.reputation_rates_like_cpp().recruit_a_friend_bonus;
        }
        percent
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_quest_reward_reputation_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &mut QuestRewardReputationFixtureRefsLikeCpp<'_>,
    ) {
        let source = if quest.is_daily_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest
        } else if quest.is_weekly_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest
        } else if quest.is_monthly_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest
        } else if quest.is_repeatable() {
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest
        } else {
            RepresentedQuestRewardReputationSourceLikeCpp::Quest
        };
        let gain_source = match source {
            RepresentedQuestRewardReputationSourceLikeCpp::Quest => ReputationGainSourceLikeCpp::Quest,
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest => ReputationGainSourceLikeCpp::DailyQuest,
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest => ReputationGainSourceLikeCpp::WeeklyQuest,
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest => ReputationGainSourceLikeCpp::MonthlyQuest,
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest => ReputationGainSourceLikeCpp::RepeatableQuest,
        };
        let faction_store = self.catalogs.faction_store().map(Arc::clone);
        let quest_faction_reward_store = self.catalogs.quests.faction_reward_store.as_ref().map(Arc::clone);
        let reputation_reward_rate_store = self.catalogs.reputation_reward_rate_store().map(Arc::clone);
        let reputation_spillover_template_store = self.catalogs.reputation_spillover_template_store().map(Arc::clone);
        let friendship_rep_reaction_store = self.catalogs.friendship_rep_reaction_store().map(Arc::clone);
        let paragon_reputation_store = self.catalogs.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.catalogs.currency_types_store().map(Arc::clone);

        for slot in 0..wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT {
            let faction_id = quest.reward_faction_ids[slot];
            if faction_id == 0 { continue; }
            let faction_entry = match faction_store.as_deref() {
                Some(store) => match store.get(faction_id).cloned() {
                    Some(entry) => Some(entry),
                    None => continue,
                },
                None => None,
            };
            let faction_lookup_missing = faction_entry.is_none();
            let reward_faction_override = quest.reward_faction_overrides[slot];
            let (base_reputation_before_gain, no_quest_bonus, quest_faction_reward_lookup) =
                if reward_faction_override != 0 {
                    (reward_faction_override / 100, true, false)
                } else if let Some(store) = quest_faction_reward_store.as_deref() {
                    let row = if quest.reward_faction_values[slot] < 0 { 2 } else { 1 };
                    let field = quest.reward_faction_values[slot].unsigned_abs() as usize;
                    let rep = store.get(row)
                        .and_then(|entry| entry.difficulty.get(field).copied())
                        .map(i32::from).unwrap_or(0);
                    (rep, false, false)
                } else {
                    (0, false, true)
                };
            if base_reputation_before_gain == 0 && !quest_faction_reward_lookup { continue; }

            let player_level = self.player.player_level_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.player_level,
            );
            let quest_level_for_gain = self.quest_state.player_quest_level_like_cpp(player_level, quest)
                .max(0) as u32;
            let reputation_rates = self.config.reputation_rates_like_cpp();
            let Some(percent_before_reward_rate) = self.quest_reputation_gain_percent_like_cpp(
                gain_source, quest_level_for_gain, base_reputation_before_gain, no_quest_bonus,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
            ) else { continue; };
            let reputation_after_low_level_rate_like_cpp = calculate_pct_i32_f32_like_cpp(
                base_reputation_before_gain, percent_before_reward_rate,
            );
            if reputation_after_low_level_rate_like_cpp == 0 && !quest_faction_reward_lookup { continue; }
            let (reputation_after_reward_rate_like_cpp, percent_after_reward_rate_like_cpp,
                reputation_reward_rate_lookup) = if reputation_reward_rate_store.is_some() {
                if let Some(rate) = self.catalogs.reputation_reward_rate_for_source_like_cpp(gain_source, faction_id) {
                    if rate <= 0.0 { continue; }
                    let percent = percent_before_reward_rate * rate;
                    (calculate_pct_i32_f32_like_cpp(base_reputation_before_gain, percent), percent, false)
                } else {
                    (reputation_after_low_level_rate_like_cpp, percent_before_reward_rate, false)
                }
            } else {
                (reputation_after_low_level_rate_like_cpp, percent_before_reward_rate, true)
            };
            let reputation_after_recruit_a_friend_bonus_like_cpp = calculate_pct_i32_f32_like_cpp(
                base_reputation_before_gain,
                self.quest_reputation_raf_percent_like_cpp(
                    gain_source, percent_after_reward_rate_like_cpp,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures,
                ),
            );
            if reputation_after_recruit_a_friend_bonus_like_cpp == 0 && !quest_faction_reward_lookup { continue; }
            let current_rank_for_cap = if quest.reward_faction_cap_in[slot] != 0
                && reputation_after_recruit_a_friend_bonus_like_cpp > 0
            {
                self.player.canonical_reward_reputation_standing_like_cpp(faction_id)
                    .map(|standing| wow_data::reputation::reputation_rank_from_standing_like_cpp(standing).as_u8())
            } else { None };
            if current_rank_for_cap.is_some_and(|rank| i32::from(rank) >= quest.reward_faction_cap_in[slot]) {
                continue;
            }
            let no_spillover = (quest.reward_faction_flags & (1u32 << slot)) != 0;
            let modify_reputation_runtime_unrepresented =
                if let (Some(faction_entry), Some(faction_store)) = (faction_entry.as_ref(), faction_store.as_deref()) {
                    let options = wow_progression::mgr::SetReputationOptionsLikeCpp {
                        incremental: true, spillover_only: false, no_spillover,
                        reputation_gain_rate: reputation_rates.gain,
                        paragon_reward_quest_status_none_like_cpp: true,
                        renown_current_level_like_cpp: 0,
                        renown_currency_increased_cap_quantity_like_cpp: 0,
                        player_race: self.player.player_race_like_cpp(),
                        player_class: self.player.player_class_like_cpp(),
                    };
                    let db_spillover_template = reputation_spillover_template_store.as_deref()
                        .and_then(|store| store.get(faction_id));
                    let mutation = self.player.mutate_reward_reputation_mgr_like_cpp(
                        #[cfg(any(test, feature = "test-fixtures"))]
                        &mut *fixtures.state,
                        |mgr| {
                            let outcome = mgr.set_reputation_like_cpp(
                                faction_entry, reputation_after_recruit_a_friend_bonus_like_cpp,
                                options, faction_store, db_spillover_template,
                                friendship_rep_reaction_store.as_deref(),
                                paragon_reputation_store.as_deref(), currency_types_store.as_deref(),
                            );
                            let packet = outcome.send_state_rep_list_id.map(|rep_list_id| {
                                mgr.set_faction_standing_packet_like_cpp(Some(rep_list_id))
                            });
                            (outcome, packet)
                        },
                    );
                    let owner_unavailable = mutation.is_none();
                    if let Some((_outcome, Some(packet))) = mutation { self.send_packet_like_cpp(&packet); }
                    owner_unavailable
                } else { true };
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.world_test_consumer {
                self.quest_state.fixture_record_quest_reward_reputation_like_cpp(
                    super::super::RepresentedQuestRewardReputationLikeCpp {
                        quest_id: quest.id, slot: slot as u8, faction_id,
                        reward_faction_value: quest.reward_faction_values[slot],
                        reward_faction_override, reward_faction_cap_in: quest.reward_faction_cap_in[slot],
                        base_reputation_before_gain, reputation_after_low_level_rate_like_cpp,
                        reputation_after_reward_rate_like_cpp, no_quest_bonus, no_spillover, source,
                        faction_store_lookup_unrepresented: faction_lookup_missing,
                        quest_faction_reward_store_lookup_unrepresented: quest_faction_reward_lookup,
                        reputation_reward_rate_lookup_unrepresented: reputation_reward_rate_lookup,
                        gray_level_script_hook_unrepresented: true,
                        reputation_rank_cap_check_unrepresented: quest.reward_faction_cap_in[slot] != 0
                            && reputation_after_recruit_a_friend_bonus_like_cpp > 0 && current_rank_for_cap.is_none(),
                        calculate_reputation_gain_unrepresented: true, modify_reputation_runtime_unrepresented,
                    },
                );
            }
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let _ = (faction_lookup_missing, modify_reputation_runtime_unrepresented);
        }
    }
}

//! Represented reputation standings and their published changes.
//!
//! Moved out of the Session root under #611. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn attack_reputation_faction_snapshot_like_cpp(
        &self,
        creature: &wow_entities::Creature,
    ) -> Option<AttackReputationFactionSnapshotLikeCpp> {
        let faction_template_id = u32::try_from(creature.unit().data().faction_template).ok()?;
        self.factions
            .template_store
            .as_ref()
            .and_then(|store| store.get(faction_template_id))
            .map(|entry| {
                let faction_id = u32::from(entry.faction);
                AttackReputationFactionSnapshotLikeCpp {
                    faction_id,
                    contested_guard: entry.is_contested_guard_faction_like_cpp(),
                    can_have_reputation: self.factions.store.as_ref().and_then(|store| {
                        store
                            .get(faction_id)
                            .map(|faction| faction.can_have_reputation_like_cpp())
                    }),
                }
            })
            .or_else(|| {
                creature
                    .attack_reputation_faction_id_like_cpp()
                    .map(|faction_id| AttackReputationFactionSnapshotLikeCpp {
                        faction_id,
                        contested_guard: creature.is_contested_guard_like_cpp(),
                        can_have_reputation: None,
                    })
            })
    }
    pub fn set_reputation_rates_like_cpp(&mut self, rates: ReputationRatesLikeCpp) {
        self.reputation_rates = rates;
    }
    #[cfg(test)]
    pub fn set_start_all_reputation_like_cpp(&mut self, enabled: bool) {
        self.start_all_reputation_like_cpp = enabled;
    }
    #[cfg(test)]
    pub(crate) fn start_all_reputation_like_cpp(&self) -> bool {
        self.start_all_reputation_like_cpp
    }
    pub(crate) fn reputation_price_discount_for_faction_template_like_cpp(
        &self,
        faction_template_id: u32,
    ) -> f32 {
        use wow_data::reputation::ReputationRankLikeCpp;

        let Some(faction_template_store) = self.factions.template_store.as_ref() else {
            return 1.0;
        };
        let Some(faction_template) = faction_template_store.get(faction_template_id) else {
            return 1.0;
        };
        if faction_template.faction == 0 {
            return 1.0;
        }
        let Some(faction_store) = self.factions.store.as_ref() else {
            return 1.0;
        };
        let Some(faction_entry) = faction_store.get(u32::from(faction_template.faction)) else {
            return 1.0;
        };

        let Some(rank) = self.with_reputation_mgr_like_cpp(|mgr| {
            mgr.rank_for_faction_entry_like_cpp(
                faction_entry,
                self.friendship_rep_reaction_store.as_deref(),
                self.player_race_like_cpp(),
                self.player_class_like_cpp(),
            )
        }) else {
            return 1.0;
        };
        if rank <= ReputationRankLikeCpp::Neutral {
            return 1.0;
        }

        1.0 - 0.05 * f32::from(rank.as_u8() - ReputationRankLikeCpp::Neutral.as_u8())
    }
    /// Reputation rank used by deterministic trainer pricing. Missing/zero
    /// faction references retain C++'s full-price fallback (`Neutral`).
    pub(crate) fn trainer_price_reputation_rank_like_cpp(
        &self,
        faction_template_id: u32,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        use wow_data::reputation::ReputationRankLikeCpp;

        let Some(faction_template) = self
            .factions
            .template_store
            .as_ref()
            .and_then(|store| store.get(faction_template_id))
        else {
            return ReputationRankLikeCpp::Neutral;
        };
        if faction_template.faction == 0 {
            return ReputationRankLikeCpp::Neutral;
        }
        let Some(faction_entry) = self
            .factions
            .store
            .as_ref()
            .and_then(|store| store.get(u32::from(faction_template.faction)))
        else {
            return ReputationRankLikeCpp::Neutral;
        };
        self.with_reputation_mgr_like_cpp(|mgr| {
            mgr.rank_for_faction_entry_like_cpp(
                faction_entry,
                self.friendship_rep_reaction_store.as_deref(),
                self.player_race_like_cpp(),
                self.player_class_like_cpp(),
            )
        })
        .unwrap_or(ReputationRankLikeCpp::Neutral)
    }
    pub(crate) fn reputation_rates_like_cpp(&self) -> ReputationRatesLikeCpp {
        self.reputation_rates
    }
    #[cfg(test)]
    pub(crate) fn reputation_mgr_like_cpp(&self) -> ReputationMgrRefLikeCpp<'_> {
        ReputationMgrLikeCpp::borrowing_like_cpp(&self.reputation_state_like_cpp)
    }
    #[cfg(test)]
    pub(crate) fn reputation_mgr_like_cpp_mut(&mut self) -> ReputationMgrMutLikeCpp<'_> {
        ReputationMgrLikeCpp::borrowing_mut_like_cpp(&mut self.reputation_state_like_cpp)
    }
    /// Run one C++ `ReputationMgr` read against the Player's own state.
    ///
    /// C++ `Player::GetReputationMgr()` hands out a reference to the manager
    /// the Player owns (`Player.h:3116`). This borrows the equivalent canonical
    /// state instead of rebuilding a manager from it (#735).
    pub(crate) fn with_reputation_mgr_like_cpp<R>(
        &self,
        operation: impl FnOnce(&ReputationMgrRefLikeCpp<'_>) -> R,
    ) -> Option<R> {
        let mut operation = Some(operation);
        let canonical = self.with_owned_player_like_cpp(|player| {
            let manager = ReputationMgrLikeCpp::borrowing_like_cpp(player.reputation_like_cpp());
            operation.take().expect("reputation operation runs once")(&manager)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let manager = ReputationMgrLikeCpp::borrowing_like_cpp(&self.reputation_state_like_cpp);
            return Some(operation.take().expect("reputation operation is available")(&manager));
        }
        None
    }
    /// Run one C++ `ReputationMgr` transition against the Player's own state.
    ///
    /// The transition writes through the Player's named reputation owner; no
    /// aggregate is reconstructed and nothing is written back through the
    /// Player's whole gameplay state (#735).
    pub(crate) fn mutate_reputation_mgr_like_cpp<R>(
        &mut self,
        operation: impl FnOnce(&mut ReputationMgrMutLikeCpp<'_>) -> R,
    ) -> Option<R> {
        let mut operation = Some(operation);
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            let mut manager =
                ReputationMgrLikeCpp::borrowing_mut_like_cpp(player.reputation_mut_like_cpp());
            operation.take().expect("reputation mutation runs once")(&mut manager)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut manager =
                ReputationMgrLikeCpp::borrowing_mut_like_cpp(&mut self.reputation_state_like_cpp);
            return Some(operation.take().expect("reputation mutation is available")(
                &mut manager,
            ));
        }
        None
    }
    /// Clone the Player's reputation state for a read that outlives the
    /// canonical borrow. This is a read snapshot, never a writable mirror.
    pub(crate) fn cloned_reputation_state_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerReputationStateLikeCpp> {
        self.with_reputation_mgr_like_cpp(|manager| manager.cloned_state_like_cpp())
    }
    #[allow(dead_code)]
    pub(crate) fn reputation_rank_like_cpp(
        &self,
        faction_entry: &FactionEntry,
        standing: i32,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        reputation_to_rank_like_cpp(
            faction_entry,
            standing,
            self.friendship_rep_reaction_store.as_deref(),
        )
    }
    pub(crate) fn canonical_player_reputation_standing_like_cpp(
        &self,
        faction_id: u32,
    ) -> Option<i32> {
        self.canonical_player_snapshot_like_cpp(|player| {
            player
                .reputation_like_cpp()
                .factions_like_cpp()
                .find_map(|state| (state.faction_id == faction_id).then_some(state.standing))
                .unwrap_or(0)
        })
    }
    pub fn set_paragon_reputation_store(&mut self, store: Arc<ParagonReputationStore>) {
        self.paragon_reputation_store = Some(store);
        self.initialize_reputation_mgr_like_cpp();
    }
    pub(crate) fn paragon_reputation_store(&self) -> Option<&Arc<ParagonReputationStore>> {
        self.paragon_reputation_store.as_ref()
    }
    pub(in crate::session) fn initialize_reputation_mgr_like_cpp(&mut self) {
        let Some(faction_store) = self.factions.store.clone() else {
            return;
        };
        let paragon_reputation_store = self.paragon_reputation_store.clone();
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let _ = self.mutate_reputation_mgr_like_cpp(|mgr| {
            mgr.initialize_like_cpp(
                faction_store.as_ref(),
                paragon_reputation_store.as_deref(),
                race,
                class,
            );
        });
    }
    pub fn set_reputation_reward_rate_store(
        &mut self,
        store: Arc<ReputationRewardRateStoreLikeCpp>,
    ) {
        self.reputation_reward_rate_store = Some(store);
    }
    pub(crate) fn reputation_reward_rate_store(
        &self,
    ) -> Option<&Arc<ReputationRewardRateStoreLikeCpp>> {
        self.reputation_reward_rate_store.as_ref()
    }
    pub fn set_reputation_spillover_template_store(
        &mut self,
        store: Arc<RepSpilloverTemplateStoreLikeCpp>,
    ) {
        self.reputation_spillover_template_store = Some(store);
    }
    pub(crate) fn reputation_spillover_template_store(
        &self,
    ) -> Option<&Arc<RepSpilloverTemplateStoreLikeCpp>> {
        self.reputation_spillover_template_store.as_ref()
    }
    pub(crate) fn apply_represented_first_login_reputation_with_catalogs_like_cpp(
        &mut self,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
    ) -> usize {
        if !player_bootstrap.start_all_reputation {
            return 0;
        }

        let Some(faction_store) = self.faction_store().map(Arc::clone) else {
            return 0;
        };
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().map(Arc::clone);
        let paragon_reputation_store = self.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.currency_types_store().map(Arc::clone);
        let player_race = self.player_race_like_cpp();
        let player_class = self.player_class_like_cpp();
        let team_factions = match player_team_for_race_cpp(player_race) {
            Team::Horde => FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP,
            _ => FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP,
        };

        let Some((applied, packet)) = self.mutate_reputation_mgr_like_cpp(|mgr| {
            let mut applied = 0usize;
            for faction_id in FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP
                .iter()
                .chain(team_factions.iter())
            {
                let Some(faction_entry) = faction_store.get(*faction_id).cloned() else {
                    continue;
                };
                let outcome = mgr.set_one_faction_reputation_like_cpp(
                    &faction_entry,
                    FIRST_LOGIN_START_REPUTATION_STANDING_LIKE_CPP,
                    false,
                    1.0,
                    friendship_rep_reaction_store.as_deref(),
                    paragon_reputation_store.as_deref(),
                    true,
                    currency_types_store.as_deref(),
                    0,
                    0,
                    player_race,
                    player_class,
                );
                if outcome.applied {
                    applied += 1;
                }
            }
            let packet = (applied > 0).then(|| mgr.set_faction_standing_packet_like_cpp(None));
            (applied, packet)
        }) else {
            return 0;
        };
        if let Some(packet) = packet {
            self.send_packet(&packet);
        }

        applied
    }
    #[cfg(test)]
    pub(crate) fn apply_represented_first_login_reputation_like_cpp(&mut self) -> usize {
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        self.apply_represented_first_login_reputation_with_catalogs_like_cpp(&player_bootstrap)
    }
    pub(in crate::session) fn calculate_kill_reputation_gain_like_cpp(
        &self,
        creature_level: u8,
        rep: i32,
        faction_id: u32,
    ) -> i32 {
        self.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Kill,
            u32::from(creature_level),
            rep,
            faction_id,
            false,
        )
    }
    pub(crate) fn reputation_gain_percent_before_reward_rate_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        creature_or_quest_level: u32,
        rep: i32,
        faction_id: u32,
        no_quest_bonus: bool,
    ) -> Option<f32> {
        let mut percent = 100.0f32;
        let mut rep_mod = if no_quest_bonus {
            0.0
        } else {
            self.resolved_total_represented_aura_modifier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModReputationGain,
            )? as f32
        };

        if source == ReputationGainSourceLikeCpp::Kill {
            rep_mod += self.resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
                RepresentedAuraEffectLikeCpp::ModFactionReputationGain,
                faction_id as i32,
            )? as f32;
        }

        percent += if rep > 0 { rep_mod } else { -rep_mod };

        let reputation_rates = self.reputation_rates_like_cpp();
        let low_level_rate = match source {
            ReputationGainSourceLikeCpp::Kill => reputation_rates.low_level_kill,
            ReputationGainSourceLikeCpp::Quest
            | ReputationGainSourceLikeCpp::DailyQuest
            | ReputationGainSourceLikeCpp::WeeklyQuest
            | ReputationGainSourceLikeCpp::MonthlyQuest
            | ReputationGainSourceLikeCpp::RepeatableQuest => reputation_rates.low_level_quest,
            ReputationGainSourceLikeCpp::Spell => 1.0,
        };
        if low_level_rate != 1.0
            && creature_or_quest_level < u32::from(self.gray_level(self.player_level_like_cpp()))
        {
            percent *= low_level_rate;
        }

        (percent > 0.0).then_some(percent)
    }
    pub(crate) fn reputation_reward_rate_for_source_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        faction_id: u32,
    ) -> Option<f32> {
        let rates = self
            .reputation_reward_rate_store()
            .and_then(|store| store.get(faction_id))?;
        let rate = match source {
            ReputationGainSourceLikeCpp::Kill => rates.creature_rate,
            ReputationGainSourceLikeCpp::Quest => rates.quest_rate,
            ReputationGainSourceLikeCpp::DailyQuest => rates.quest_daily_rate,
            ReputationGainSourceLikeCpp::WeeklyQuest => rates.quest_weekly_rate,
            ReputationGainSourceLikeCpp::MonthlyQuest => rates.quest_monthly_rate,
            ReputationGainSourceLikeCpp::RepeatableQuest => rates.quest_repeatable_rate,
            ReputationGainSourceLikeCpp::Spell => rates.spell_rate,
        };
        Some(rate)
    }
    pub(crate) fn calculate_reputation_gain_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        creature_or_quest_level: u32,
        rep: i32,
        faction_id: u32,
        no_quest_bonus: bool,
    ) -> i32 {
        let Some(mut percent) = self.reputation_gain_percent_before_reward_rate_like_cpp(
            source,
            creature_or_quest_level,
            rep,
            faction_id,
            no_quest_bonus,
        ) else {
            return 0;
        };

        if let Some(rep_rate) = self.reputation_reward_rate_for_source_like_cpp(source, faction_id)
        {
            if rep_rate <= 0.0 {
                return 0;
            }
            percent *= rep_rate;
        }

        percent = self.apply_recruit_a_friend_reputation_bonus_like_cpp(source, percent);

        (rep as f32 * percent / 100.0) as i32
    }
    pub(crate) fn apply_recruit_a_friend_reputation_bonus_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        mut percent: f32,
    ) -> f32 {
        if source != ReputationGainSourceLikeCpp::Spell
            && self.gets_recruit_a_friend_reputation_bonus_like_cpp()
        {
            percent *= 1.0 + self.reputation_rates_like_cpp().recruit_a_friend_bonus;
        }
        percent
    }
    fn gets_recruit_a_friend_reputation_bonus_like_cpp(&self) -> bool {
        self.gets_recruit_a_friend_bonus_like_cpp(false)
    }
    #[cfg(test)]
    pub(crate) async fn reputation_changed_like_cpp(&mut self, faction_id: u32, change: i32) {
        self.enqueue_represented_quest_objective_progress_like_cpp(
            RepresentedQuestObjectiveProgressEventLikeCpp::ReputationChanged { faction_id, change },
        );
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }
}

//! Reputation manager state definitions, part 2 of 3. operations, part 1 of 2.
//!
//! The inherent impl is divided by responsibility under #662; every
//! method keeps its original body.

use super::*;

impl ReputationMgrLikeCpp {
    pub fn new_like_cpp() -> Self {
        Self::default()
    }
    /// Rebuild the C++ `ReputationMgr` working view from its canonical
    /// `Player`-owned state. DB2 catalogs and packet types deliberately remain
    /// outside `wow-entities`; this value is a short-lived service view, not a
    /// second authority.
    pub fn from_player_gameplay_state_like_cpp(state: &PlayerGameplayState) -> Self {
        let factions = state
            .reputations
            .iter()
            .map(|record| {
                (
                    record.reputation_list_id,
                    FactionStateLikeCpp {
                        id: record.faction_id,
                        reputation_list_id: record.reputation_list_id,
                        standing: record.standing,
                        visual_standing_increase: record.visual_standing_increase,
                        flags: ReputationFlagsLikeCpp::from_bits_retain(record.flags as u16),
                        need_send: record.need_send,
                        need_save: record.need_save,
                    },
                )
            })
            .collect();
        let forced_reactions = state
            .forced_reputation_ranks
            .iter()
            .filter_map(|(faction_id, rank)| {
                ReputationRankLikeCpp::from_u8_like_cpp(*rank).map(|rank| (*faction_id, rank))
            })
            .collect();
        Self {
            factions,
            forced_reactions,
            rank_counters: ReputationRankCountersLikeCpp {
                visible: state.reputation_rank_counters[0],
                honored: state.reputation_rank_counters[1],
                revered: state.reputation_rank_counters[2],
                exalted: state.reputation_rank_counters[3],
            },
            send_faction_increased: state.send_faction_increased,
        }
    }
    /// Publish one completed service mutation back to the canonical `Player`.
    pub fn write_to_player_gameplay_state_like_cpp(self, state: &mut PlayerGameplayState) {
        state.reputations = self
            .factions
            .into_values()
            .map(|faction| PlayerReputationRecord {
                faction_id: faction.id,
                reputation_list_id: faction.reputation_list_id,
                standing: faction.standing,
                flags: u32::from(faction.flags.bits()),
                visual_standing_increase: faction.visual_standing_increase,
                need_send: faction.need_send,
                need_save: faction.need_save,
            })
            .collect();
        state.forced_reputation_ranks = self
            .forced_reactions
            .into_iter()
            .map(|(faction_id, rank)| (faction_id, rank.as_u8()))
            .collect();
        state.reputation_rank_counters = [
            self.rank_counters.visible,
            self.rank_counters.honored,
            self.rank_counters.revered,
            self.rank_counters.exalted,
        ];
        state.send_faction_increased = self.send_faction_increased;
    }
    pub fn factions(&self) -> &BTreeMap<RepListIdLikeCpp, FactionStateLikeCpp> {
        &self.factions
    }
    pub fn forced_reactions(&self) -> &ForcedReactionsLikeCpp {
        &self.forced_reactions
    }
    pub fn rank_counters(&self) -> ReputationRankCountersLikeCpp {
        self.rank_counters
    }
    pub fn reputation_for_faction_like_cpp(
        &self,
        faction_entry: &FactionEntry,
        player_race: u8,
        player_class: u8,
    ) -> i32 {
        if !faction_entry.can_have_reputation_like_cpp() {
            return 0;
        }
        base_reputation_like_cpp(faction_entry, player_race, player_class)
            + self
                .get_state(faction_entry.reputation_index as RepListIdLikeCpp)
                .map(|state| state.standing)
                .unwrap_or(0)
    }
    pub fn criteria_progress_like_cpp(
        &self,
        kind: ReputationCriteriaProgressKindLikeCpp,
        faction_store: Option<&FactionStore>,
        player_race: u8,
        player_class: u8,
    ) -> Option<u32> {
        match kind {
            ReputationCriteriaProgressKindLikeCpp::ReputationGained { faction_id } => {
                let faction_entry = faction_store?.get(faction_id)?;
                let reputation =
                    self.reputation_for_faction_like_cpp(faction_entry, player_race, player_class);
                (reputation > 0).then_some(reputation as u32)
            }
            ReputationCriteriaProgressKindLikeCpp::TotalExaltedFactions => {
                Some(u32::from(self.rank_counters.exalted))
            }
            ReputationCriteriaProgressKindLikeCpp::TotalReveredFactions => {
                Some(u32::from(self.rank_counters.revered))
            }
            ReputationCriteriaProgressKindLikeCpp::TotalHonoredFactions => {
                Some(u32::from(self.rank_counters.honored))
            }
            ReputationCriteriaProgressKindLikeCpp::TotalFactionsEncountered => {
                Some(u32::from(self.rank_counters.visible))
            }
        }
    }
    pub fn send_faction_increased(&self) -> bool {
        self.send_faction_increased
    }
    pub fn get_state(&self, rep_list_id: RepListIdLikeCpp) -> Option<&FactionStateLikeCpp> {
        self.factions.get(&rep_list_id)
    }
    pub fn get_state_mut(
        &mut self,
        rep_list_id: RepListIdLikeCpp,
    ) -> Option<&mut FactionStateLikeCpp> {
        self.factions.get_mut(&rep_list_id)
    }
    pub fn initialize_like_cpp(
        &mut self,
        faction_store: &FactionStore,
        paragon_reputation_store: Option<&ParagonReputationStore>,
        player_race: u8,
        player_class: u8,
    ) {
        self.factions.clear();
        self.rank_counters = ReputationRankCountersLikeCpp::default();
        self.send_faction_increased = false;

        for faction_entry in faction_store.iter() {
            if !faction_entry.can_have_reputation_like_cpp() {
                continue;
            }

            let flags = default_state_flags_like_cpp(
                faction_entry,
                paragon_reputation_store,
                player_race,
                player_class,
            );
            let state = FactionStateLikeCpp::new_like_cpp(
                faction_entry.id,
                faction_entry.reputation_index as RepListIdLikeCpp,
                flags,
            );

            if state.flags.contains(ReputationFlagsLikeCpp::VISIBLE) {
                self.rank_counters.visible = self.rank_counters.visible.saturating_add(1);
            }

            if faction_entry.friendship_rep_id == 0 {
                self.update_rank_counters_like_cpp(
                    ReputationRankLikeCpp::Hostile,
                    base_rank_like_cpp(faction_entry, player_race, player_class),
                );
            }

            self.factions.insert(state.reputation_list_id, state);
        }
    }
    pub fn insert_state_for_test_like_cpp(&mut self, state: FactionStateLikeCpp) {
        self.factions.insert(state.reputation_list_id, state);
    }
    pub fn apply_force_reaction_like_cpp(
        &mut self,
        faction_id: u32,
        rank: ReputationRankLikeCpp,
        apply: bool,
    ) {
        if apply {
            self.forced_reactions.insert(faction_id, rank);
        } else {
            self.forced_reactions.remove(&faction_id);
        }
    }
    pub fn forced_rank_by_faction_id_like_cpp(
        &self,
        faction_id: u32,
    ) -> Option<ReputationRankLikeCpp> {
        self.forced_reactions.get(&faction_id).copied()
    }
    pub fn is_at_war_with_faction_like_cpp(&self, faction_entry: &FactionEntry) -> bool {
        if !faction_entry.can_have_reputation_like_cpp() {
            return false;
        }
        self.get_state(faction_entry.reputation_index as RepListIdLikeCpp)
            .is_some_and(|state| state.flags.contains(ReputationFlagsLikeCpp::AT_WAR))
    }
    pub fn rank_for_faction_entry_like_cpp(
        &self,
        faction_entry: &FactionEntry,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        player_race: u8,
        player_class: u8,
    ) -> ReputationRankLikeCpp {
        let reputation = if faction_entry.can_have_reputation_like_cpp() {
            self.get_state(faction_entry.reputation_index as RepListIdLikeCpp)
                .map(|state| {
                    base_reputation_like_cpp(faction_entry, player_race, player_class)
                        + state.standing
                })
                .unwrap_or(0)
        } else {
            0
        };
        reputation_to_rank_like_cpp(faction_entry, reputation, friendship_rep_reaction_store)
    }
    pub fn set_reputation_like_cpp(
        &mut self,
        faction_entry: &FactionEntry,
        standing: i32,
        options: SetReputationOptionsLikeCpp,
        faction_store: &FactionStore,
        db_spillover_template: Option<&RepSpilloverTemplateLikeCpp>,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        paragon_reputation_store: Option<&ParagonReputationStore>,
        currency_types_store: Option<&CurrencyTypesStore>,
    ) -> SetReputationOutcomeLikeCpp {
        let mut outcome = SetReputationOutcomeLikeCpp {
            applied: false,
            script_reputation_change_event: Some((faction_entry.id, standing, options.incremental)),
            spillover_mutations: Vec::new(),
            primary_mutation: None,
            send_state_rep_list_id: None,
        };

        if !options.no_spillover {
            if let Some(rep_template) = db_spillover_template {
                for index in 0..MAX_SPILLOVER_FACTIONS_LIKE_CPP {
                    let spillover_faction_id = rep_template.faction[index];
                    if spillover_faction_id == 0 {
                        continue;
                    }
                    if self.get_reputation_rank_by_faction_id_like_cpp(
                        spillover_faction_id,
                        faction_store,
                        friendship_rep_reaction_store,
                        options.player_race,
                        options.player_class,
                    ) <= ReputationRankLikeCpp::from_u8_like_cpp(
                        rep_template.faction_rank[index],
                    )
                    .unwrap_or(ReputationRankLikeCpp::Exalted)
                    {
                        let spillover_rep =
                            (standing as f32 * rep_template.faction_rate[index]) as i32;
                        if let Some(spillover_faction) = faction_store.get(spillover_faction_id) {
                            let mutation = self.set_one_faction_reputation_like_cpp(
                                spillover_faction,
                                spillover_rep,
                                options.incremental,
                                options.reputation_gain_rate,
                                friendship_rep_reaction_store,
                                paragon_reputation_store,
                                options.paragon_reward_quest_status_none_like_cpp,
                                currency_types_store,
                                options.renown_current_level_like_cpp,
                                options.renown_currency_increased_cap_quantity_like_cpp,
                                options.player_race,
                                options.player_class,
                            );
                            outcome.applied |= mutation.applied;
                            outcome
                                .spillover_mutations
                                .push((spillover_faction_id, mutation));
                        }
                    }
                }
            } else {
                let mut spillover_rep_out = standing as f32;
                let mut faction_team_list =
                    faction_store.faction_team_list_like_cpp(faction_entry.id);
                if faction_team_list.is_empty()
                    && faction_entry.parent_faction_id != 0
                    && faction_entry.parent_faction_mod[1] != 0.0
                {
                    spillover_rep_out *= faction_entry.parent_faction_mod[1];
                    if let Some(parent) =
                        faction_store.get(u32::from(faction_entry.parent_faction_id))
                    {
                        let parent_rep_list_id = parent.reputation_index as RepListIdLikeCpp;
                        if self.get_state(parent_rep_list_id).is_some_and(|state| {
                            state
                                .flags
                                .contains(ReputationFlagsLikeCpp::HEADER_SHOWS_BAR)
                        }) {
                            let mutation = self.set_one_faction_reputation_like_cpp(
                                parent,
                                spillover_rep_out as i32,
                                options.incremental,
                                options.reputation_gain_rate,
                                friendship_rep_reaction_store,
                                paragon_reputation_store,
                                options.paragon_reward_quest_status_none_like_cpp,
                                currency_types_store,
                                options.renown_current_level_like_cpp,
                                options.renown_currency_increased_cap_quantity_like_cpp,
                                options.player_race,
                                options.player_class,
                            );
                            outcome.spillover_mutations.push((parent.id, mutation));
                        } else {
                            faction_team_list = faction_store.faction_team_list_like_cpp(
                                u32::from(faction_entry.parent_faction_id),
                            );
                        }
                    }
                }

                for spillover_faction_id in faction_team_list {
                    let Some(spillover_faction) = faction_store.get(spillover_faction_id) else {
                        continue;
                    };
                    if spillover_faction.id == faction_entry.id {
                        continue;
                    }
                    let cap_rank = ReputationRankLikeCpp::from_u8_like_cpp(
                        spillover_faction.parent_faction_cap[0],
                    )
                    .unwrap_or(ReputationRankLikeCpp::Exalted);
                    if self.get_reputation_rank_by_faction_id_like_cpp(
                        spillover_faction.id,
                        faction_store,
                        friendship_rep_reaction_store,
                        options.player_race,
                        options.player_class,
                    ) > cap_rank
                    {
                        continue;
                    }

                    let spillover_rep =
                        (spillover_rep_out * spillover_faction.parent_faction_mod[0]) as i32;
                    if spillover_rep != 0 || !options.incremental {
                        let mutation = self.set_one_faction_reputation_like_cpp(
                            spillover_faction,
                            spillover_rep,
                            options.incremental,
                            options.reputation_gain_rate,
                            friendship_rep_reaction_store,
                            paragon_reputation_store,
                            options.paragon_reward_quest_status_none_like_cpp,
                            currency_types_store,
                            options.renown_current_level_like_cpp,
                            options.renown_currency_increased_cap_quantity_like_cpp,
                            options.player_race,
                            options.player_class,
                        );
                        outcome.applied |= mutation.applied;
                        outcome
                            .spillover_mutations
                            .push((spillover_faction.id, mutation));
                    }
                }
            }
        }

        let mut primary_faction_to_modify = faction_entry;
        if options.incremental
            && standing > 0
            && self.can_gain_paragon_reputation_for_faction_like_cpp(
                faction_entry,
                faction_store,
                paragon_reputation_store,
                options.renown_current_level_like_cpp,
                options.renown_currency_increased_cap_quantity_like_cpp,
                currency_types_store,
                options.player_race,
                options.player_class,
            )
        {
            if let Some(paragon_faction) =
                faction_store.get(u32::from(faction_entry.paragon_faction_id))
            {
                primary_faction_to_modify = paragon_faction;
            }
        }

        let rep_list_id = primary_faction_to_modify.reputation_index as RepListIdLikeCpp;
        if self.factions.contains_key(&rep_list_id) {
            if !options.spillover_only {
                let mutation = self.set_one_faction_reputation_like_cpp(
                    primary_faction_to_modify,
                    standing,
                    options.incremental,
                    options.reputation_gain_rate,
                    friendship_rep_reaction_store,
                    paragon_reputation_store,
                    options.paragon_reward_quest_status_none_like_cpp,
                    currency_types_store,
                    options.renown_current_level_like_cpp,
                    options.renown_currency_increased_cap_quantity_like_cpp,
                    options.player_race,
                    options.player_class,
                );
                outcome.applied |= mutation.applied;
                outcome.primary_mutation = Some((primary_faction_to_modify.id, mutation));
            }
            outcome.send_state_rep_list_id = Some(rep_list_id);
        }

        outcome
    }
    pub fn load_from_db_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = CharacterReputationRowLikeCpp>,
        faction_store: &FactionStore,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        paragon_reputation_store: Option<&ParagonReputationStore>,
        player_race: u8,
        player_class: u8,
    ) {
        self.initialize_like_cpp(
            faction_store,
            paragon_reputation_store,
            player_race,
            player_class,
        );

        for row in rows {
            let Some(faction_entry) = faction_store.get(u32::from(row.faction_id)) else {
                continue;
            };
            if !faction_entry.can_have_reputation_like_cpp() {
                continue;
            }

            let rep_list_id = faction_entry.reputation_index as RepListIdLikeCpp;
            let base_reputation =
                base_reputation_like_cpp(faction_entry, player_race, player_class);
            let old_rank = reputation_to_rank_like_cpp(
                faction_entry,
                base_reputation,
                friendship_rep_reaction_store,
            );
            let new_rank = reputation_to_rank_like_cpp(
                faction_entry,
                base_reputation + row.standing,
                friendship_rep_reaction_store,
            );
            if faction_entry.friendship_rep_id == 0 {
                self.update_rank_counters_like_cpp(old_rank, new_rank);
            }

            if let Some(faction) = self.factions.get_mut(&rep_list_id) {
                faction.standing = row.standing;
            }

            let db_flags = ReputationFlagsLikeCpp::from_bits_truncate(row.flags);
            if db_flags.contains(ReputationFlagsLikeCpp::VISIBLE) {
                self.set_visible_like_cpp(rep_list_id, paragon_reputation_store);
            }
            if db_flags.contains(ReputationFlagsLikeCpp::INACTIVE) {
                self.set_inactive_like_cpp(rep_list_id, true);
            }
            if db_flags.contains(ReputationFlagsLikeCpp::AT_WAR) {
                self.set_at_war_like_cpp(
                    rep_list_id,
                    true,
                    faction_entry,
                    friendship_rep_reaction_store,
                    player_race,
                    player_class,
                );
            } else if self
                .get_state(rep_list_id)
                .is_some_and(|faction| faction.flags.contains(ReputationFlagsLikeCpp::VISIBLE))
            {
                self.set_at_war_like_cpp(
                    rep_list_id,
                    false,
                    faction_entry,
                    friendship_rep_reaction_store,
                    player_race,
                    player_class,
                );
            }

            if new_rank <= ReputationRankLikeCpp::Hostile {
                self.set_at_war_like_cpp(
                    rep_list_id,
                    true,
                    faction_entry,
                    friendship_rep_reaction_store,
                    player_race,
                    player_class,
                );
            }

            if let Some(faction) = self.factions.get_mut(&rep_list_id) {
                if faction.flags == db_flags {
                    faction.need_send = false;
                    faction.need_save = false;
                }
            }
        }
    }
    /// SQLx-free rows consumed by the Player lifecycle persistence port.
    /// Iteration order intentionally matches the existing statement builder.
    pub fn pending_save_rows_like_cpp(&self) -> Vec<(u16, i32, u16)> {
        self.factions
            .values()
            .filter(|faction| faction.need_save)
            .map(|faction| (faction.id as u16, faction.standing, faction.flags.bits()))
            .collect()
    }
    pub fn mark_pending_save_to_db_committed_like_cpp(&mut self) {
        for faction in self.factions.values_mut() {
            if faction.need_save {
                faction.need_save = false;
            }
        }
    }
    pub fn initialize_factions_packet_like_cpp(&mut self) -> InitializeFactionsPacketLikeCpp {
        let mut packet = InitializeFactionsPacketLikeCpp::default();

        for (rep_list_id, faction) in self.factions.iter_mut() {
            let index = *rep_list_id as usize;
            if index >= FACTION_COUNT_LIKE_CPP {
                continue;
            }

            packet.faction_flags[index] = faction.flags.bits();
            packet.faction_standings[index] = faction.standing;
            faction.need_send = false;
        }

        packet
    }
    pub fn set_faction_standing_packet_like_cpp(
        &mut self,
        faction_rep_list_id: Option<RepListIdLikeCpp>,
    ) -> SetFactionStandingPacketLikeCpp {
        let primary_faction = faction_rep_list_id.and_then(|rep_list_id| {
            self.factions
                .get(&rep_list_id)
                .map(|state| FactionStandingDataPacketLikeCpp {
                    index: state.reputation_list_id as i32,
                    standing: standing_for_packet_like_cpp(state),
                })
        });

        let mut packet = SetFactionStandingPacketLikeCpp {
            bonus_from_achievement_system: 0.0,
            faction: Vec::new(),
            show_visual: self.send_faction_increased,
        };
        if let Some(primary) = primary_faction {
            packet.faction.push(primary);
        }

        for (rep_list_id, state) in self.factions.iter_mut() {
            if !state.need_send {
                continue;
            }
            state.need_send = false;
            if Some(*rep_list_id) == faction_rep_list_id {
                continue;
            }
            packet.faction.push(FactionStandingDataPacketLikeCpp {
                index: state.reputation_list_id as i32,
                standing: standing_for_packet_like_cpp(state),
            });
        }

        self.send_faction_increased = false;
        packet
    }
    pub fn set_forced_reactions_packet_like_cpp(&self) -> SetForcedReactionsPacketLikeCpp {
        SetForcedReactionsPacketLikeCpp {
            reactions: self
                .forced_reactions
                .iter()
                .map(|(faction_id, rank)| ForcedReactionPacketLikeCpp {
                    faction: *faction_id as i32,
                    reaction: i32::from(rank.as_u8()),
                })
                .collect(),
        }
    }
    pub fn set_at_war_by_replist_like_cpp(
        &mut self,
        rep_list_id: RepListIdLikeCpp,
        at_war: bool,
        faction_store: &FactionStore,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        player_race: u8,
        player_class: u8,
    ) -> bool {
        let Some(faction_id) = self.factions.get(&rep_list_id).map(|state| state.id) else {
            return false;
        };
        let Some(faction_entry) = faction_store.get(faction_id) else {
            return false;
        };
        let before = self.factions.get(&rep_list_id).cloned();

        self.set_at_war_like_cpp(
            rep_list_id,
            at_war,
            faction_entry,
            friendship_rep_reaction_store,
            player_race,
            player_class,
        );

        before.as_ref() != self.factions.get(&rep_list_id)
    }
    pub fn set_inactive_by_replist_like_cpp(
        &mut self,
        rep_list_id: RepListIdLikeCpp,
        inactive: bool,
    ) -> bool {
        let before = self.factions.get(&rep_list_id).cloned();
        if before.is_none() {
            return false;
        }

        self.set_inactive_like_cpp(rep_list_id, inactive);

        before.as_ref() != self.factions.get(&rep_list_id)
    }
}

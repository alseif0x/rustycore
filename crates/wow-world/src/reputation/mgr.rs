//! Per-player C++ `ReputationMgr` state foundation.
//!
//! This module owns only the direct `ReputationMgr.h` state shape. Initialization,
//! DB load/save, spillover and packet fanout are ported in later slices.

use std::collections::BTreeMap;

use wow_data::CurrencyTypesStore;
use wow_data::progression_rewards::{
    FactionEntry, FactionStore, FriendshipRepReactionStore, ParagonReputationStore,
};
use wow_data::reputation::{
    MAX_SPILLOVER_FACTIONS_LIKE_CPP, RepSpilloverTemplateLikeCpp, ReputationFlagsLikeCpp,
    ReputationRankLikeCpp,
};
use wow_entities::{PlayerGameplayState, PlayerReputationRecord};
use wow_packet::packets::reputation::{
    FACTION_COUNT_LIKE_CPP, FactionStandingData as FactionStandingDataPacketLikeCpp,
    ForcedReaction as ForcedReactionPacketLikeCpp,
    InitializeFactions as InitializeFactionsPacketLikeCpp,
    SetFactionStanding as SetFactionStandingPacketLikeCpp,
    SetForcedReactions as SetForcedReactionsPacketLikeCpp,
};

pub type RepListIdLikeCpp = u32;
pub type ForcedReactionsLikeCpp = BTreeMap<u32, ReputationRankLikeCpp>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetReputationOptionsLikeCpp {
    pub incremental: bool,
    pub spillover_only: bool,
    pub no_spillover: bool,
    pub reputation_gain_rate: f32,
    pub paragon_reward_quest_status_none_like_cpp: bool,
    pub renown_current_level_like_cpp: i32,
    pub renown_currency_increased_cap_quantity_like_cpp: u32,
    pub player_race: u8,
    pub player_class: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetReputationOutcomeLikeCpp {
    pub applied: bool,
    pub script_reputation_change_event: Option<(u32, i32, bool)>,
    pub spillover_mutations: Vec<(u32, ReputationMutationOutcomeLikeCpp)>,
    pub primary_mutation: Option<(u32, ReputationMutationOutcomeLikeCpp)>,
    pub send_state_rep_list_id: Option<RepListIdLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReputationMutationOutcomeLikeCpp {
    pub applied: bool,
    pub reputation_change: i32,
    pub old_rank: Option<ReputationRankLikeCpp>,
    pub new_rank: Option<ReputationRankLikeCpp>,
    pub set_at_war_for_hostile: bool,
    pub became_visible: bool,
    pub paragon_reward_quest_id_to_add_if_template_exists_like_cpp: Option<i32>,
    pub renown_currency_delta_like_cpp: Option<(u32, i32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationCriteriaProgressKindLikeCpp {
    ReputationGained { faction_id: u32 },
    TotalExaltedFactions,
    TotalReveredFactions,
    TotalHonoredFactions,
    TotalFactionsEncountered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterReputationRowLikeCpp {
    pub faction_id: u16,
    pub standing: i32,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactionStateLikeCpp {
    pub id: u32,
    pub reputation_list_id: RepListIdLikeCpp,
    pub standing: i32,
    pub visual_standing_increase: i32,
    pub flags: ReputationFlagsLikeCpp,
    pub need_send: bool,
    pub need_save: bool,
}

impl FactionStateLikeCpp {
    pub fn new_like_cpp(
        faction_id: u32,
        reputation_list_id: RepListIdLikeCpp,
        flags: ReputationFlagsLikeCpp,
    ) -> Self {
        Self {
            id: faction_id,
            reputation_list_id,
            standing: 0,
            visual_standing_increase: 0,
            flags,
            need_send: true,
            need_save: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReputationRankCountersLikeCpp {
    pub visible: u8,
    pub honored: u8,
    pub revered: u8,
    pub exalted: u8,
}

#[derive(Debug, Clone, Default)]
pub struct ReputationMgrLikeCpp {
    factions: BTreeMap<RepListIdLikeCpp, FactionStateLikeCpp>,
    forced_reactions: ForcedReactionsLikeCpp,
    rank_counters: ReputationRankCountersLikeCpp,
    send_faction_increased: bool,
}

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

    pub fn set_one_faction_reputation_like_cpp(
        &mut self,
        faction_entry: &FactionEntry,
        standing: i32,
        incremental: bool,
        reputation_gain_rate: f32,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        paragon_reputation_store: Option<&ParagonReputationStore>,
        paragon_reward_quest_status_none_like_cpp: bool,
        currency_types_store: Option<&CurrencyTypesStore>,
        renown_current_level_like_cpp: i32,
        renown_currency_increased_cap_quantity_like_cpp: u32,
        player_race: u8,
        player_class: u8,
    ) -> ReputationMutationOutcomeLikeCpp {
        let rep_list_id = faction_entry.reputation_index as RepListIdLikeCpp;
        let Some(state) = self.factions.get(&rep_list_id) else {
            return ReputationMutationOutcomeLikeCpp {
                applied: false,
                reputation_change: 0,
                old_rank: None,
                new_rank: None,
                set_at_war_for_hostile: false,
                became_visible: false,
                paragon_reward_quest_id_to_add_if_template_exists_like_cpp: None,
                renown_currency_delta_like_cpp: None,
            };
        };

        let base_reputation = base_reputation_like_cpp(faction_entry, player_race, player_class);
        let old_standing = state.standing + base_reputation;
        let is_renown = faction_entry.renown_currency_id > 0;
        if is_renown
            && standing > 0
            && renown_current_level_like_cpp
                >= renown_max_level_like_cpp(
                    faction_entry,
                    currency_types_store,
                    renown_currency_increased_cap_quantity_like_cpp,
                )
        {
            if let Some(state) = self.factions.get_mut(&rep_list_id) {
                state.need_send = false;
                state.need_save = false;
            }
            return ReputationMutationOutcomeLikeCpp {
                applied: false,
                reputation_change: 0,
                old_rank: None,
                new_rank: None,
                set_at_war_for_hostile: false,
                became_visible: false,
                paragon_reward_quest_id_to_add_if_template_exists_like_cpp: None,
                renown_currency_delta_like_cpp: None,
            };
        }

        let mut target_standing = standing;
        if incremental || is_renown {
            target_standing =
                (target_standing as f32 * reputation_gain_rate + 0.5_f32).floor() as i32;
            target_standing += old_standing;
        }

        let paragon_reputation = paragon_reputation_store
            .and_then(|store| store.get_by_faction_id_like_cpp(faction_entry.id));
        let is_paragon = paragon_reputation.is_some();
        let is_renown = faction_entry.renown_currency_id > 0;
        let min_reputation = min_reputation_like_cpp(faction_entry, friendship_rep_reaction_store);
        let max_reputation = max_reputation_like_cpp(
            faction_entry,
            friendship_rep_reaction_store,
            paragon_reputation_store,
            old_standing,
            paragon_reward_quest_status_none_like_cpp,
            currency_types_store,
            renown_currency_increased_cap_quantity_like_cpp,
            player_race,
            player_class,
        );
        target_standing = target_standing.clamp(min_reputation, max_reputation);

        let mut old_rank = None;
        let mut new_rank = None;
        let mut set_at_war_for_hostile = false;
        if !is_paragon && !is_renown {
            let old = reputation_to_rank_like_cpp(
                faction_entry,
                old_standing,
                friendship_rep_reaction_store,
            );
            let new = reputation_to_rank_like_cpp(
                faction_entry,
                target_standing,
                friendship_rep_reaction_store,
            );
            old_rank = Some(old);
            new_rank = Some(new);

            if new <= ReputationRankLikeCpp::Hostile {
                self.set_at_war_like_cpp(
                    rep_list_id,
                    true,
                    faction_entry,
                    friendship_rep_reaction_store,
                    player_race,
                    player_class,
                );
                set_at_war_for_hostile = true;
            }
            if new > old {
                self.send_faction_increased = true;
            }
            if faction_entry.friendship_rep_id == 0 {
                self.update_rank_counters_like_cpp(old, new);
            }
        } else {
            self.send_faction_increased = true;
        }

        let mut new_standing = target_standing - base_reputation;
        let mut reputation_change = target_standing - old_standing;
        let mut renown_currency_delta_like_cpp = None;
        if is_renown {
            if let Some(currency) = currency_types_store
                .and_then(|store| store.get(faction_entry.renown_currency_id as u32))
            {
                let renown_level_threshold =
                    renown_level_threshold_like_cpp(faction_entry, player_race, player_class);
                let renown_max_level = renown_max_level_like_cpp(
                    faction_entry,
                    currency_types_store,
                    renown_currency_increased_cap_quantity_like_cpp,
                );
                if renown_level_threshold > 0 {
                    let total_reputation = (renown_current_level_like_cpp * renown_level_threshold)
                        + (target_standing - base_reputation);
                    let new_renown_level = total_reputation / renown_level_threshold;
                    new_standing = total_reputation % renown_level_threshold;

                    if new_renown_level >= renown_max_level {
                        new_standing = 0;
                        reputation_change +=
                            (renown_max_level * renown_level_threshold) - total_reputation;
                    }

                    if let Some(state) = self.factions.get_mut(&rep_list_id) {
                        state.visual_standing_increase = reputation_change;
                    }
                    if renown_current_level_like_cpp != new_renown_level {
                        renown_currency_delta_like_cpp = Some((
                            currency.id,
                            new_renown_level - renown_current_level_like_cpp,
                        ));
                    }
                }
            }
        }

        if let Some(state) = self.factions.get_mut(&rep_list_id) {
            state.standing = new_standing;
            state.need_send = true;
            state.need_save = true;
        }
        let was_visible = self
            .get_state(rep_list_id)
            .is_some_and(|state| state.flags.contains(ReputationFlagsLikeCpp::VISIBLE));
        self.set_visible_like_cpp(rep_list_id, paragon_reputation_store);
        let became_visible = !was_visible
            && self
                .get_state(rep_list_id)
                .is_some_and(|state| state.flags.contains(ReputationFlagsLikeCpp::VISIBLE));
        let paragon_reward_quest_id_to_add_if_template_exists_like_cpp = paragon_reputation
            .filter(|entry| entry.level_threshold > 0)
            .filter(|entry| {
                old_standing / entry.level_threshold != target_standing / entry.level_threshold
            })
            .map(|entry| entry.quest_id);

        ReputationMutationOutcomeLikeCpp {
            applied: true,
            reputation_change,
            old_rank,
            new_rank,
            set_at_war_for_hostile,
            became_visible,
            paragon_reward_quest_id_to_add_if_template_exists_like_cpp,
            renown_currency_delta_like_cpp,
        }
    }

    fn set_visible_like_cpp(
        &mut self,
        rep_list_id: RepListIdLikeCpp,
        paragon_reputation_store: Option<&ParagonReputationStore>,
    ) {
        let Some(faction) = self.factions.get_mut(&rep_list_id) else {
            return;
        };
        if faction.flags.contains(ReputationFlagsLikeCpp::HIDDEN) {
            return;
        }
        if faction.flags.contains(ReputationFlagsLikeCpp::HEADER)
            && !faction
                .flags
                .contains(ReputationFlagsLikeCpp::HEADER_SHOWS_BAR)
        {
            return;
        }
        if paragon_reputation_store
            .is_some_and(|store| store.get_by_faction_id_like_cpp(faction.id).is_some())
        {
            return;
        }
        if faction.flags.contains(ReputationFlagsLikeCpp::VISIBLE) {
            return;
        }
        faction.flags |= ReputationFlagsLikeCpp::VISIBLE;
        faction.need_send = true;
        faction.need_save = true;
        self.rank_counters.visible = self.rank_counters.visible.saturating_add(1);
    }

    fn set_inactive_like_cpp(&mut self, rep_list_id: RepListIdLikeCpp, inactive: bool) {
        let Some(faction) = self.factions.get_mut(&rep_list_id) else {
            return;
        };
        if faction
            .flags
            .intersects(ReputationFlagsLikeCpp::HIDDEN | ReputationFlagsLikeCpp::HEADER)
            || !faction.flags.contains(ReputationFlagsLikeCpp::VISIBLE)
        {
            return;
        }
        if faction.flags.contains(ReputationFlagsLikeCpp::INACTIVE) == inactive {
            return;
        }

        if inactive {
            faction.flags |= ReputationFlagsLikeCpp::INACTIVE;
        } else {
            faction.flags &= !ReputationFlagsLikeCpp::INACTIVE;
        }
        faction.need_send = true;
        faction.need_save = true;
    }

    fn set_at_war_like_cpp(
        &mut self,
        rep_list_id: RepListIdLikeCpp,
        at_war: bool,
        faction_entry: &FactionEntry,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        player_race: u8,
        player_class: u8,
    ) {
        let rank = self
            .factions
            .get(&rep_list_id)
            .map(|faction| {
                reputation_to_rank_like_cpp(
                    faction_entry,
                    base_reputation_like_cpp(faction_entry, player_race, player_class)
                        + faction.standing,
                    friendship_rep_reaction_store,
                )
            })
            .unwrap_or(ReputationRankLikeCpp::Neutral);

        let Some(faction) = self.factions.get_mut(&rep_list_id) else {
            return;
        };
        if faction
            .flags
            .intersects(ReputationFlagsLikeCpp::HIDDEN | ReputationFlagsLikeCpp::HEADER)
        {
            return;
        }
        if at_war
            && faction.flags.contains(ReputationFlagsLikeCpp::PEACEFUL)
            && rank > ReputationRankLikeCpp::Hated
        {
            return;
        }
        if faction.flags.contains(ReputationFlagsLikeCpp::AT_WAR) == at_war {
            return;
        }

        if at_war {
            faction.flags |= ReputationFlagsLikeCpp::AT_WAR;
        } else {
            faction.flags &= !ReputationFlagsLikeCpp::AT_WAR;
        }
        faction.need_send = true;
        faction.need_save = true;
    }

    fn update_rank_counters_like_cpp(
        &mut self,
        old_rank: ReputationRankLikeCpp,
        new_rank: ReputationRankLikeCpp,
    ) {
        if old_rank >= ReputationRankLikeCpp::Exalted {
            self.rank_counters.exalted = self.rank_counters.exalted.saturating_sub(1);
        }
        if old_rank >= ReputationRankLikeCpp::Revered {
            self.rank_counters.revered = self.rank_counters.revered.saturating_sub(1);
        }
        if old_rank >= ReputationRankLikeCpp::Honored {
            self.rank_counters.honored = self.rank_counters.honored.saturating_sub(1);
        }

        if new_rank >= ReputationRankLikeCpp::Exalted {
            self.rank_counters.exalted = self.rank_counters.exalted.saturating_add(1);
        }
        if new_rank >= ReputationRankLikeCpp::Revered {
            self.rank_counters.revered = self.rank_counters.revered.saturating_add(1);
        }
        if new_rank >= ReputationRankLikeCpp::Honored {
            self.rank_counters.honored = self.rank_counters.honored.saturating_add(1);
        }
    }

    fn get_reputation_rank_by_faction_id_like_cpp(
        &self,
        faction_id: u32,
        faction_store: &FactionStore,
        friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
        player_race: u8,
        player_class: u8,
    ) -> ReputationRankLikeCpp {
        faction_store
            .get(faction_id)
            .map(|faction| {
                let standing = self
                    .get_state(faction.reputation_index as RepListIdLikeCpp)
                    .map(|state| state.standing)
                    .unwrap_or(0)
                    + base_reputation_like_cpp(faction, player_race, player_class);
                reputation_to_rank_like_cpp(faction, standing, friendship_rep_reaction_store)
            })
            .unwrap_or(ReputationRankLikeCpp::Neutral)
    }

    fn can_gain_paragon_reputation_for_faction_like_cpp(
        &self,
        faction_entry: &FactionEntry,
        faction_store: &FactionStore,
        paragon_reputation_store: Option<&ParagonReputationStore>,
        renown_current_level_like_cpp: i32,
        renown_currency_increased_cap_quantity_like_cpp: u32,
        currency_types_store: Option<&CurrencyTypesStore>,
        player_race: u8,
        player_class: u8,
    ) -> bool {
        if faction_store
            .get(u32::from(faction_entry.paragon_faction_id))
            .is_none()
        {
            return false;
        }

        let rank = self
            .get_state(faction_entry.reputation_index as RepListIdLikeCpp)
            .map(|state| {
                reputation_to_rank_like_cpp(
                    faction_entry,
                    base_reputation_like_cpp(faction_entry, player_race, player_class)
                        + state.standing,
                    None,
                )
            });
        if rank != Some(ReputationRankLikeCpp::Exalted)
            && renown_current_level_like_cpp
                < renown_max_level_like_cpp(
                    faction_entry,
                    currency_types_store,
                    renown_currency_increased_cap_quantity_like_cpp,
                )
        {
            return false;
        }

        paragon_reputation_store
            .and_then(|store| {
                store.get_by_faction_id_like_cpp(u32::from(faction_entry.paragon_faction_id))
            })
            .is_some()
    }
}

fn default_state_flags_like_cpp(
    faction_entry: &FactionEntry,
    paragon_reputation_store: Option<&ParagonReputationStore>,
    player_race: u8,
    player_class: u8,
) -> ReputationFlagsLikeCpp {
    let mut flags =
        faction_data_index_for_race_and_class_like_cpp(faction_entry, player_race, player_class)
            .map(|index| {
                ReputationFlagsLikeCpp::from_bits_truncate(faction_entry.reputation_flags[index])
            })
            .unwrap_or(ReputationFlagsLikeCpp::NONE);

    if paragon_reputation_store
        .is_some_and(|store| store.get_by_faction_id_like_cpp(faction_entry.id).is_some())
    {
        flags |= ReputationFlagsLikeCpp::SHOW_PROPAGATED;
    }

    flags
}

fn base_reputation_like_cpp(
    faction_entry: &FactionEntry,
    player_race: u8,
    player_class: u8,
) -> i32 {
    faction_data_index_for_race_and_class_like_cpp(faction_entry, player_race, player_class)
        .map(|index| faction_entry.reputation_base[index])
        .unwrap_or(0)
}

fn base_rank_like_cpp(
    faction_entry: &FactionEntry,
    player_race: u8,
    player_class: u8,
) -> ReputationRankLikeCpp {
    reputation_to_rank_like_cpp(
        faction_entry,
        base_reputation_like_cpp(faction_entry, player_race, player_class),
        None,
    )
}

fn min_reputation_like_cpp(
    faction_entry: &FactionEntry,
    friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
) -> i32 {
    friendship_rep_reaction_store
        .filter(|_| faction_entry.friendship_rep_id != 0)
        .and_then(|store| {
            store
                .reactions_for_friendship_rep_like_cpp(faction_entry.friendship_rep_id)
                .first()
                .map(|entry| i32::from(entry.reaction_threshold))
        })
        .unwrap_or(wow_data::reputation::REPUTATION_BOTTOM_LIKE_CPP)
}

fn max_reputation_like_cpp(
    faction_entry: &FactionEntry,
    friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
    paragon_reputation_store: Option<&ParagonReputationStore>,
    current_reputation: i32,
    paragon_reward_quest_status_none_like_cpp: bool,
    currency_types_store: Option<&CurrencyTypesStore>,
    renown_currency_increased_cap_quantity_like_cpp: u32,
    player_race: u8,
    player_class: u8,
) -> i32 {
    if let Some(paragon_reputation) = paragon_reputation_store
        .and_then(|store| store.get_by_faction_id_like_cpp(faction_entry.id))
        .filter(|entry| entry.level_threshold > 0)
    {
        let threshold = paragon_reputation.level_threshold;
        let mut cap = current_reputation + threshold - current_reputation % threshold - 1;
        if paragon_reward_quest_status_none_like_cpp {
            cap += threshold;
        }
        return cap;
    }

    if faction_entry.renown_currency_id > 0 {
        return renown_max_level_like_cpp(
            faction_entry,
            currency_types_store,
            renown_currency_increased_cap_quantity_like_cpp,
        ) * renown_level_threshold_like_cpp(faction_entry, player_race, player_class);
    }

    if let Some(max) = friendship_rep_reaction_store
        .filter(|_| faction_entry.friendship_rep_id != 0)
        .and_then(|store| {
            store
                .reactions_for_friendship_rep_like_cpp(faction_entry.friendship_rep_id)
                .last()
                .map(|entry| i32::from(entry.reaction_threshold))
        })
    {
        return max;
    }

    faction_data_index_for_race_and_class_like_cpp(faction_entry, player_race, player_class)
        .map(|index| faction_entry.reputation_max[index])
        .unwrap_or(wow_data::reputation::REPUTATION_CAP_LIKE_CPP)
}

fn renown_level_threshold_like_cpp(
    faction_entry: &FactionEntry,
    player_race: u8,
    player_class: u8,
) -> i32 {
    if faction_entry.renown_currency_id <= 0 {
        return 0;
    }

    faction_data_index_for_race_and_class_like_cpp(faction_entry, player_race, player_class)
        .map(|index| faction_entry.reputation_max[index])
        .unwrap_or(0)
}

fn renown_max_level_like_cpp(
    faction_entry: &FactionEntry,
    currency_types_store: Option<&CurrencyTypesStore>,
    renown_currency_increased_cap_quantity_like_cpp: u32,
) -> i32 {
    if faction_entry.renown_currency_id <= 0 {
        return 0;
    }

    currency_types_store
        .and_then(|store| store.get(faction_entry.renown_currency_id as u32))
        .filter(|currency| currency.has_max_quantity(false, false))
        .map(|currency| {
            currency
                .max_qty
                .saturating_add(renown_currency_increased_cap_quantity_like_cpp) as i32
        })
        .unwrap_or(0)
}

pub fn reputation_to_rank_like_cpp(
    faction_entry: &FactionEntry,
    standing: i32,
    friendship_rep_reaction_store: Option<&FriendshipRepReactionStore>,
) -> ReputationRankLikeCpp {
    if let Some(friendship_rep_id) =
        (faction_entry.friendship_rep_id != 0).then_some(faction_entry.friendship_rep_id)
    {
        if let Some(store) = friendship_rep_reaction_store {
            let rank = rank_from_thresholds_like_cpp(
                store
                    .reactions_for_friendship_rep_like_cpp(friendship_rep_id)
                    .into_iter()
                    .map(|entry| i32::from(entry.reaction_threshold)),
                standing,
            );
            if let Some(rank) = ReputationRankLikeCpp::from_u8_like_cpp(rank) {
                return rank;
            }
        }
    }

    wow_data::reputation::reputation_rank_from_standing_like_cpp(standing)
}

fn rank_from_thresholds_like_cpp(thresholds: impl IntoIterator<Item = i32>, standing: i32) -> u8 {
    let mut rank: i32 = -1;
    for threshold in thresholds {
        if standing < threshold {
            break;
        }
        rank += 1;
    }
    rank.clamp(
        ReputationRankLikeCpp::Hated.as_u8() as i32,
        ReputationRankLikeCpp::Exalted.as_u8() as i32,
    ) as u8
}

fn faction_data_index_for_race_and_class_like_cpp(
    faction_entry: &FactionEntry,
    player_race: u8,
    player_class: u8,
) -> Option<usize> {
    let class_mask = player_class_mask_like_cpp(player_class)?;

    for index in 0..4 {
        let race_mask = faction_entry.reputation_race_mask[index] as u64;
        let class_slot_mask = if faction_entry.reputation_class_mask[index] < 0 {
            0
        } else {
            faction_entry.reputation_class_mask[index] as u32
        };
        let race_matches = race_mask_has_race_like_cpp(race_mask, player_race)
            || (race_mask == 0 && class_slot_mask != 0);
        let class_matches = (class_slot_mask & class_mask) != 0 || class_slot_mask == 0;

        if race_matches && class_matches {
            return Some(index);
        }
    }

    None
}

fn player_class_mask_like_cpp(class_id: u8) -> Option<u32> {
    (1..=13)
        .contains(&class_id)
        .then(|| 1_u32 << (class_id - 1))
}

fn race_mask_has_race_like_cpp(mask: u64, race_id: u8) -> bool {
    player_race_mask_like_cpp(race_id).is_some_and(|race_mask| (mask & race_mask) != 0)
}

fn player_race_mask_like_cpp(race_id: u8) -> Option<u64> {
    let bit = match race_id {
        1..=11 => race_id - 1,
        22 => 21,
        24..=32 => race_id - 1,
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        52 => 16,
        70 => 15,
        _ => return None,
    };
    Some(1_u64 << bit)
}

fn standing_for_packet_like_cpp(state: &FactionStateLikeCpp) -> i32 {
    if state.visual_standing_increase != 0 {
        state.visual_standing_increase
    } else {
        state.standing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_constants::{CurrencyTypesFlags, CurrencyTypesFlagsB};
    use wow_data::CurrencyTypesEntry;
    use wow_data::progression_rewards::{FriendshipRepReactionEntry, ParagonReputationEntry};

    #[test]
    fn faction_state_defaults_match_cpp_initialize_shape() {
        let flags = ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR;
        let state = FactionStateLikeCpp::new_like_cpp(72, 4, flags);

        assert_eq!(state.id, 72);
        assert_eq!(state.reputation_list_id, 4);
        assert_eq!(state.standing, 0);
        assert_eq!(state.visual_standing_increase, 0);
        assert_eq!(state.flags, flags);
        assert!(state.need_send);
        assert!(state.need_save);
    }

    #[test]
    fn reputation_mgr_initial_state_matches_cpp_constructor_shape() {
        let mgr = ReputationMgrLikeCpp::new_like_cpp();

        assert!(mgr.factions().is_empty());
        assert!(mgr.forced_reactions().is_empty());
        assert_eq!(
            mgr.rank_counters(),
            ReputationRankCountersLikeCpp::default()
        );
        assert!(!mgr.send_faction_increased());
    }

    #[test]
    fn initialize_factions_packet_like_cpp_uses_rep_list_indices_and_clears_need_send() {
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        let mut state = FactionStateLikeCpp::new_like_cpp(
            72,
            4,
            ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR,
        );
        state.standing = 1234;
        state.need_send = true;
        mgr.insert_state_for_test_like_cpp(state);

        let packet = mgr.initialize_factions_packet_like_cpp();

        assert_eq!(
            packet.faction_flags[4],
            (ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR).bits()
        );
        assert_eq!(packet.faction_standings[4], 1234);
        assert!(!packet.faction_has_bonus[4]);
        assert!(!mgr.get_state(4).expect("state remains present").need_send);
    }

    #[test]
    fn set_faction_standing_packet_like_cpp_matches_send_state_order_and_clears_flags() {
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        let mut primary = FactionStateLikeCpp::new_like_cpp(72, 4, ReputationFlagsLikeCpp::VISIBLE);
        primary.standing = 100;
        primary.need_send = true;
        mgr.insert_state_for_test_like_cpp(primary);

        let mut secondary =
            FactionStateLikeCpp::new_like_cpp(930, 7, ReputationFlagsLikeCpp::VISIBLE);
        secondary.standing = 200;
        secondary.visual_standing_increase = 25;
        secondary.need_send = true;
        mgr.insert_state_for_test_like_cpp(secondary);
        mgr.send_faction_increased = true;

        let packet = mgr.set_faction_standing_packet_like_cpp(Some(4));

        assert_eq!(packet.bonus_from_achievement_system, 0.0);
        assert!(packet.show_visual);
        assert_eq!(
            packet.faction,
            vec![
                FactionStandingDataPacketLikeCpp {
                    index: 4,
                    standing: 100,
                },
                FactionStandingDataPacketLikeCpp {
                    index: 7,
                    standing: 25,
                },
            ]
        );
        assert!(!mgr.send_faction_increased());
        assert!(!mgr.get_state(4).expect("primary").need_send);
        assert!(!mgr.get_state(7).expect("secondary").need_send);
    }

    #[test]
    fn set_forced_reactions_packet_like_cpp_matches_cpp_map_order() {
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.apply_force_reaction_like_cpp(930, ReputationRankLikeCpp::Hated, true);
        mgr.apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Exalted, true);

        let packet = mgr.set_forced_reactions_packet_like_cpp();

        assert_eq!(
            packet.reactions,
            vec![
                ForcedReactionPacketLikeCpp {
                    faction: 72,
                    reaction: 7,
                },
                ForcedReactionPacketLikeCpp {
                    faction: 930,
                    reaction: 0,
                },
            ]
        );
    }

    #[test]
    fn reputation_mgr_uses_replist_ordered_faction_state_map_like_cpp() {
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.insert_state_for_test_like_cpp(FactionStateLikeCpp::new_like_cpp(
            10,
            3,
            ReputationFlagsLikeCpp::VISIBLE,
        ));
        mgr.insert_state_for_test_like_cpp(FactionStateLikeCpp::new_like_cpp(
            11,
            1,
            ReputationFlagsLikeCpp::HIDDEN,
        ));

        let keys: Vec<_> = mgr.factions().keys().copied().collect();
        assert_eq!(keys, vec![1, 3]);
        assert_eq!(mgr.get_state(1).map(|state| state.id), Some(11));
        assert_eq!(mgr.get_state(3).map(|state| state.id), Some(10));
    }

    #[test]
    fn apply_force_reaction_insert_and_erase_matches_cpp_map_behavior() {
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

        mgr.apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Hostile, true);
        assert_eq!(
            mgr.forced_reactions().get(&72),
            Some(&ReputationRankLikeCpp::Hostile)
        );

        mgr.apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Friendly, true);
        assert_eq!(
            mgr.forced_reactions().get(&72),
            Some(&ReputationRankLikeCpp::Friendly)
        );

        mgr.apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Friendly, false);
        assert!(!mgr.forced_reactions().contains_key(&72));
    }

    #[test]
    fn initialize_like_cpp_creates_state_for_reputation_factions_only() {
        let mut visible = FactionEntry::for_test_like_cpp(72, 3);
        visible.reputation_race_mask[0] = 1;
        visible.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
        let hidden = FactionEntry::for_test_like_cpp(73, -1);
        let faction_store = FactionStore::from_entries([visible, hidden]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        assert_eq!(mgr.factions().len(), 1);
        assert!(mgr.get_state(3).is_some());
        assert_eq!(mgr.rank_counters().visible, 1);
        assert!(!mgr.send_faction_increased());
    }

    #[test]
    fn initialize_like_cpp_selects_first_matching_race_class_slot() {
        let mut faction = FactionEntry::for_test_like_cpp(76, 5);
        faction.reputation_race_mask[0] = 1 << 1;
        faction.reputation_class_mask[0] = 1;
        faction.reputation_flags[0] = ReputationFlagsLikeCpp::AT_WAR.bits();
        faction.reputation_race_mask[1] = 1;
        faction.reputation_class_mask[1] = 1;
        faction.reputation_flags[1] =
            (ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::PEACEFUL).bits();
        faction.reputation_base[1] = 9_000;
        let faction_store = FactionStore::from_entries([faction]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        let state = mgr.get_state(5).expect("reputation state");
        assert_eq!(
            state.flags,
            ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::PEACEFUL
        );
        assert_eq!(mgr.rank_counters().visible, 1);
        assert_eq!(mgr.rank_counters().honored, 1);
        assert_eq!(mgr.rank_counters().revered, 0);
        assert_eq!(mgr.rank_counters().exalted, 0);
    }

    #[test]
    fn initialize_like_cpp_uses_class_only_slot_when_race_mask_is_empty() {
        let mut faction = FactionEntry::for_test_like_cpp(77, 6);
        faction.reputation_class_mask[0] = 1 << 1;
        faction.reputation_flags[0] = ReputationFlagsLikeCpp::HIDDEN.bits();
        let faction_store = FactionStore::from_entries([faction]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

        mgr.initialize_like_cpp(&faction_store, None, 1, 2);

        assert_eq!(
            mgr.get_state(6).map(|state| state.flags),
            Some(ReputationFlagsLikeCpp::HIDDEN)
        );
    }

    #[test]
    fn initialize_like_cpp_skips_rank_counters_for_friendship_factions() {
        let mut faction = FactionEntry::for_test_like_cpp(78, 7);
        faction.friendship_rep_id = 4;
        faction.reputation_base[0] = 42_000;
        let faction_store = FactionStore::from_entries([faction]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        assert_eq!(mgr.rank_counters().honored, 0);
        assert_eq!(mgr.rank_counters().revered, 0);
        assert_eq!(mgr.rank_counters().exalted, 0);
    }

    #[test]
    fn initialize_like_cpp_adds_show_propagated_for_paragon_factions() {
        let faction = FactionEntry::for_test_like_cpp(79, 8);
        let faction_store = FactionStore::from_entries([faction]);
        let paragon_store = ParagonReputationStore::from_entries([ParagonReputationEntry {
            id: 1,
            faction_id: 79,
            level_threshold: 10_000,
            quest_id: 100,
        }]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

        mgr.initialize_like_cpp(&faction_store, Some(&paragon_store), 1, 1);

        assert_eq!(
            mgr.get_state(8).map(|state| state.flags),
            Some(ReputationFlagsLikeCpp::SHOW_PROPAGATED)
        );
    }

    #[test]
    fn reputation_to_rank_like_cpp_uses_stock_thresholds_without_friendship_reactions() {
        let faction = FactionEntry::for_test_like_cpp(80, 9);

        assert_eq!(
            reputation_to_rank_like_cpp(&faction, 8_999, None),
            ReputationRankLikeCpp::Friendly
        );
        assert_eq!(
            reputation_to_rank_like_cpp(&faction, 9_000, None),
            ReputationRankLikeCpp::Honored
        );
    }

    #[test]
    fn reputation_to_rank_like_cpp_uses_friendship_thresholds_ordered_by_reaction_threshold() {
        let mut faction = FactionEntry::for_test_like_cpp(81, 10);
        faction.friendship_rep_id = 7;
        let store = FriendshipRepReactionStore::from_entries([
            FriendshipRepReactionEntry {
                id: 3,
                reaction: "three".to_string(),
                friendship_rep_id: 7,
                reaction_threshold: 300,
            },
            FriendshipRepReactionEntry {
                id: 1,
                reaction: "one".to_string(),
                friendship_rep_id: 7,
                reaction_threshold: 100,
            },
            FriendshipRepReactionEntry {
                id: 2,
                reaction: "other".to_string(),
                friendship_rep_id: 8,
                reaction_threshold: 0,
            },
            FriendshipRepReactionEntry {
                id: 4,
                reaction: "two".to_string(),
                friendship_rep_id: 7,
                reaction_threshold: 200,
            },
        ]);

        assert_eq!(
            reputation_to_rank_like_cpp(&faction, 199, Some(&store)),
            ReputationRankLikeCpp::Hated
        );
        assert_eq!(
            reputation_to_rank_like_cpp(&faction, 200, Some(&store)),
            ReputationRankLikeCpp::Hostile
        );
        assert_eq!(
            reputation_to_rank_like_cpp(&faction, 300, Some(&store)),
            ReputationRankLikeCpp::Unfriendly
        );
    }

    #[test]
    fn reputation_to_rank_like_cpp_falls_back_when_friendship_store_is_missing() {
        let mut faction = FactionEntry::for_test_like_cpp(82, 11);
        faction.friendship_rep_id = 9;

        assert_eq!(
            reputation_to_rank_like_cpp(&faction, 21_000, None),
            ReputationRankLikeCpp::Revered
        );
    }

    #[test]
    fn load_from_db_like_cpp_merges_standing_flags_and_clears_clean_rows() {
        let mut faction = FactionEntry::for_test_like_cpp(83, 12);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
        faction.reputation_base[0] = 3_000;
        let faction_store = FactionStore::from_entries([faction]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

        mgr.load_from_db_like_cpp(
            [CharacterReputationRowLikeCpp {
                faction_id: 83,
                standing: 6_000,
                flags: ReputationFlagsLikeCpp::VISIBLE.bits(),
            }],
            &faction_store,
            None,
            None,
            1,
            1,
        );

        let state = mgr.get_state(12).expect("loaded state");
        assert_eq!(state.standing, 6_000);
        assert_eq!(state.flags, ReputationFlagsLikeCpp::VISIBLE);
        assert!(!state.need_send);
        assert!(!state.need_save);
        assert_eq!(mgr.rank_counters().honored, 1);
        assert_eq!(mgr.rank_counters().revered, 0);
    }

    #[test]
    fn load_from_db_like_cpp_applies_hostile_at_war_even_when_db_flag_is_clear() {
        let mut faction = FactionEntry::for_test_like_cpp(84, 13);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
        let faction_store = FactionStore::from_entries([faction]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();

        mgr.load_from_db_like_cpp(
            [CharacterReputationRowLikeCpp {
                faction_id: 84,
                standing: -6_000,
                flags: ReputationFlagsLikeCpp::VISIBLE.bits(),
            }],
            &faction_store,
            None,
            None,
            1,
            1,
        );

        let state = mgr.get_state(13).expect("loaded state");
        assert!(state.flags.contains(ReputationFlagsLikeCpp::AT_WAR));
        assert!(state.need_send);
        assert!(state.need_save);
    }

    #[test]
    fn pending_save_rows_like_cpp_preserve_dirty_faction_projection() {
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.insert_state_for_test_like_cpp(FactionStateLikeCpp {
            standing: 123,
            flags: ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR,
            ..FactionStateLikeCpp::new_like_cpp(85, 14, ReputationFlagsLikeCpp::VISIBLE)
        });

        assert_eq!(
            mgr.pending_save_rows_like_cpp(),
            vec![(
                85,
                123,
                (ReputationFlagsLikeCpp::VISIBLE | ReputationFlagsLikeCpp::AT_WAR).bits(),
            )]
        );
        assert!(mgr.get_state(14).expect("pending state").need_save);

        mgr.mark_pending_save_to_db_committed_like_cpp();

        assert_eq!(
            mgr.pending_save_rows_like_cpp(),
            Vec::<(u16, i32, u16)>::new()
        );
        assert!(!mgr.get_state(14).expect("saved state").need_save);
    }

    #[test]
    fn pending_save_rows_like_cpp_do_not_clear_dirty_until_commit() {
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.insert_state_for_test_like_cpp(FactionStateLikeCpp {
            standing: 456,
            flags: ReputationFlagsLikeCpp::VISIBLE,
            ..FactionStateLikeCpp::new_like_cpp(86, 15, ReputationFlagsLikeCpp::VISIBLE)
        });

        let rows = mgr.pending_save_rows_like_cpp();

        assert_eq!(
            rows,
            vec![(86, 456, ReputationFlagsLikeCpp::VISIBLE.bits())]
        );
        assert!(
            mgr.get_state(15).expect("pending state").need_save,
            "failed Player::SaveToDB transaction must leave reputation dirty for retry"
        );

        mgr.mark_pending_save_to_db_committed_like_cpp();

        assert!(!mgr.get_state(15).expect("committed state").need_save);
    }

    #[test]
    fn set_one_faction_reputation_like_cpp_applies_incremental_rate_and_marks_visible_dirty() {
        let mut faction = FactionEntry::for_test_like_cpp(86, 15);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        let outcome = mgr.set_one_faction_reputation_like_cpp(
            &faction, 4_000, true, 1.5, None, None, true, None, 0, 0, 1, 1,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.reputation_change, 6_000);
        assert_eq!(outcome.old_rank, Some(ReputationRankLikeCpp::Neutral));
        assert_eq!(outcome.new_rank, Some(ReputationRankLikeCpp::Friendly));
        assert!(outcome.became_visible);
        assert!(mgr.send_faction_increased());
        let state = mgr.get_state(15).expect("mutated state");
        assert_eq!(state.standing, 6_000);
        assert!(state.flags.contains(ReputationFlagsLikeCpp::VISIBLE));
        assert!(state.need_send);
        assert!(state.need_save);
    }

    #[test]
    fn set_reputation_like_cpp_applies_reputation_gain_rate_to_primary_incremental_gain() {
        let mut faction = FactionEntry::for_test_like_cpp(860, 150);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);
        let mut options = set_reputation_options_for_test_like_cpp(true);
        options.reputation_gain_rate = 2.0;

        let outcome = mgr.set_reputation_like_cpp(
            &faction,
            500,
            options,
            &faction_store,
            None,
            None,
            None,
            None,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.send_state_rep_list_id, Some(150));
        let (_, mutation) = outcome.primary_mutation.expect("primary mutation");
        assert_eq!(mutation.reputation_change, 1_000);
        assert_eq!(mgr.get_state(150).expect("mutated state").standing, 1_000);
    }

    #[test]
    fn set_one_faction_reputation_like_cpp_clamps_to_slot_max() {
        let mut faction = FactionEntry::for_test_like_cpp(87, 16);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 9_000;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        let outcome = mgr.set_one_faction_reputation_like_cpp(
            &faction, 42_000, false, 1.0, None, None, true, None, 0, 0, 1, 1,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.reputation_change, 9_000);
        assert_eq!(mgr.get_state(16).expect("mutated state").standing, 9_000);
    }

    #[test]
    fn set_one_faction_reputation_like_cpp_forces_at_war_for_hostile_rank() {
        let mut faction = FactionEntry::for_test_like_cpp(88, 17);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        let outcome = mgr.set_one_faction_reputation_like_cpp(
            &faction, -6_000, false, 1.0, None, None, true, None, 0, 0, 1, 1,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.new_rank, Some(ReputationRankLikeCpp::Hostile));
        assert!(outcome.set_at_war_for_hostile);
        assert!(
            mgr.get_state(17)
                .expect("mutated state")
                .flags
                .contains(ReputationFlagsLikeCpp::AT_WAR)
        );
    }

    #[test]
    fn set_one_faction_reputation_like_cpp_updates_honored_revered_exalted_counters() {
        let mut faction = FactionEntry::for_test_like_cpp(89, 18);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        mgr.set_one_faction_reputation_like_cpp(
            &faction, 21_000, false, 1.0, None, None, true, None, 0, 0, 1, 1,
        );

        assert_eq!(mgr.rank_counters().honored, 1);
        assert_eq!(mgr.rank_counters().revered, 1);
        assert_eq!(mgr.rank_counters().exalted, 0);
    }

    #[test]
    fn criteria_progress_like_cpp_exposes_reputation_counters_and_positive_reputation() {
        let mut faction = FactionEntry::for_test_like_cpp(91, 21);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_base[0] = 500;
        faction.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        mgr.set_one_faction_reputation_like_cpp(
            &faction, 21_000, false, 1.0, None, None, true, None, 0, 0, 1, 1,
        );

        assert_eq!(
            mgr.criteria_progress_like_cpp(
                ReputationCriteriaProgressKindLikeCpp::ReputationGained { faction_id: 91 },
                Some(&faction_store),
                1,
                1,
            ),
            Some(21_000)
        );
        assert_eq!(
            mgr.criteria_progress_like_cpp(
                ReputationCriteriaProgressKindLikeCpp::TotalFactionsEncountered,
                Some(&faction_store),
                1,
                1,
            ),
            Some(1)
        );
        assert_eq!(
            mgr.criteria_progress_like_cpp(
                ReputationCriteriaProgressKindLikeCpp::TotalHonoredFactions,
                Some(&faction_store),
                1,
                1,
            ),
            Some(1)
        );
        assert_eq!(
            mgr.criteria_progress_like_cpp(
                ReputationCriteriaProgressKindLikeCpp::TotalReveredFactions,
                Some(&faction_store),
                1,
                1,
            ),
            Some(1)
        );
        assert_eq!(
            mgr.criteria_progress_like_cpp(
                ReputationCriteriaProgressKindLikeCpp::TotalExaltedFactions,
                Some(&faction_store),
                1,
                1,
            ),
            Some(0)
        );
    }

    #[test]
    fn criteria_progress_like_cpp_skips_non_positive_reputation_gained_like_cpp() {
        let mut faction = FactionEntry::for_test_like_cpp(92, 22);
        faction.reputation_race_mask[0] = 1;
        let faction_store = FactionStore::from_entries([faction]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        assert_eq!(
            mgr.criteria_progress_like_cpp(
                ReputationCriteriaProgressKindLikeCpp::ReputationGained { faction_id: 92 },
                Some(&faction_store),
                1,
                1,
            ),
            None
        );
    }

    #[test]
    fn set_one_faction_reputation_like_cpp_uses_paragon_cap_with_no_unclaimed_reward() {
        let mut faction = FactionEntry::for_test_like_cpp(90, 19);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let paragon_store = ParagonReputationStore::from_entries([ParagonReputationEntry {
            id: 2,
            faction_id: 90,
            level_threshold: 10_000,
            quest_id: 700,
        }]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, Some(&paragon_store), 1, 1);

        let outcome = mgr.set_one_faction_reputation_like_cpp(
            &faction,
            99_999,
            false,
            1.0,
            None,
            Some(&paragon_store),
            true,
            None,
            0,
            0,
            1,
            1,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.reputation_change, 19_999);
        assert_eq!(
            outcome.paragon_reward_quest_id_to_add_if_template_exists_like_cpp,
            Some(700)
        );
        assert_eq!(mgr.get_state(19).expect("mutated state").standing, 19_999);
        assert!(mgr.send_faction_increased());
    }

    #[test]
    fn set_one_faction_reputation_like_cpp_uses_paragon_cap_when_reward_is_unclaimed() {
        let mut faction = FactionEntry::for_test_like_cpp(91, 20);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let paragon_store = ParagonReputationStore::from_entries([ParagonReputationEntry {
            id: 3,
            faction_id: 91,
            level_threshold: 10_000,
            quest_id: 701,
        }]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, Some(&paragon_store), 1, 1);

        let outcome = mgr.set_one_faction_reputation_like_cpp(
            &faction,
            99_999,
            false,
            1.0,
            None,
            Some(&paragon_store),
            false,
            None,
            0,
            0,
            1,
            1,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.reputation_change, 9_999);
        assert_eq!(
            outcome.paragon_reward_quest_id_to_add_if_template_exists_like_cpp,
            None
        );
        assert_eq!(mgr.get_state(20).expect("mutated state").standing, 9_999);
        assert!(mgr.send_faction_increased());
    }

    #[test]
    fn set_one_faction_reputation_like_cpp_applies_renown_level_remainder_and_currency_delta() {
        let mut faction = FactionEntry::for_test_like_cpp(92, 21);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 2_500;
        faction.renown_currency_id = 77;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let currency_store =
            CurrencyTypesStore::from_entries([currency_entry_for_test_like_cpp(77, 5)]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        let outcome = mgr.set_one_faction_reputation_like_cpp(
            &faction,
            3_000,
            false,
            1.0,
            None,
            None,
            true,
            Some(&currency_store),
            1,
            0,
            1,
            1,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.reputation_change, 3_000);
        assert_eq!(outcome.renown_currency_delta_like_cpp, Some((77, 1)));
        let state = mgr.get_state(21).expect("mutated state");
        assert_eq!(state.standing, 500);
        assert_eq!(state.visual_standing_increase, 3_000);
        assert!(mgr.send_faction_increased());
    }

    #[test]
    fn set_one_faction_reputation_like_cpp_caps_renown_at_max_level_and_clears_remainder() {
        let mut faction = FactionEntry::for_test_like_cpp(93, 22);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 2_500;
        faction.renown_currency_id = 78;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let currency_store =
            CurrencyTypesStore::from_entries([currency_entry_for_test_like_cpp(78, 3)]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        let outcome = mgr.set_one_faction_reputation_like_cpp(
            &faction,
            9_000,
            false,
            1.0,
            None,
            None,
            true,
            Some(&currency_store),
            2,
            0,
            1,
            1,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.reputation_change, 2_500);
        assert_eq!(outcome.renown_currency_delta_like_cpp, Some((78, 3)));
        let state = mgr.get_state(22).expect("mutated state");
        assert_eq!(state.standing, 0);
        assert_eq!(state.visual_standing_increase, 2_500);
    }

    #[test]
    fn set_one_faction_reputation_like_cpp_ignores_positive_renown_when_maxed() {
        let mut faction = FactionEntry::for_test_like_cpp(94, 23);
        faction.reputation_race_mask[0] = 1;
        faction.reputation_max[0] = 2_500;
        faction.renown_currency_id = 79;
        let faction_store = FactionStore::from_entries([faction.clone()]);
        let currency_store =
            CurrencyTypesStore::from_entries([currency_entry_for_test_like_cpp(79, 3)]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);
        mgr.get_state_mut(23).expect("state").need_send = true;
        mgr.get_state_mut(23).expect("state").need_save = true;

        let outcome = mgr.set_one_faction_reputation_like_cpp(
            &faction,
            1,
            false,
            1.0,
            None,
            None,
            true,
            Some(&currency_store),
            3,
            0,
            1,
            1,
        );

        assert!(!outcome.applied);
        let state = mgr.get_state(23).expect("state");
        assert!(!state.need_send);
        assert!(!state.need_save);
    }

    #[test]
    fn set_reputation_like_cpp_applies_db_template_spillover_then_primary_and_send_state() {
        let mut source = FactionEntry::for_test_like_cpp(100, 24);
        source.reputation_race_mask[0] = 1;
        source.reputation_max[0] = 42_000;
        let mut spill = FactionEntry::for_test_like_cpp(101, 25);
        spill.reputation_race_mask[0] = 1;
        spill.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([source.clone(), spill.clone()]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);
        let mut template = RepSpilloverTemplateLikeCpp::empty_like_cpp();
        template.faction[0] = 101;
        template.faction_rate[0] = 0.5;
        template.faction_rank[0] = ReputationRankLikeCpp::Exalted.as_u8();

        let outcome = mgr.set_reputation_like_cpp(
            &source,
            1_000,
            set_reputation_options_for_test_like_cpp(false),
            &faction_store,
            Some(&template),
            None,
            None,
            None,
        );

        assert!(outcome.applied);
        assert_eq!(
            outcome.script_reputation_change_event,
            Some((100, 1_000, false))
        );
        assert_eq!(outcome.spillover_mutations.len(), 1);
        assert_eq!(outcome.spillover_mutations[0].0, 101);
        assert_eq!(outcome.spillover_mutations[0].1.reputation_change, 500);
        assert_eq!(
            outcome.primary_mutation.as_ref().map(|entry| entry.0),
            Some(100)
        );
        assert_eq!(outcome.send_state_rep_list_id, Some(24));
        assert_eq!(mgr.get_state(24).expect("source state").standing, 1_000);
        assert_eq!(mgr.get_state(25).expect("spill state").standing, 500);
    }

    #[test]
    fn set_reputation_like_cpp_db_template_respects_rank_cap_and_no_spillover() {
        let mut source = FactionEntry::for_test_like_cpp(102, 26);
        source.reputation_race_mask[0] = 1;
        source.reputation_max[0] = 42_000;
        let mut spill = FactionEntry::for_test_like_cpp(103, 27);
        spill.reputation_race_mask[0] = 1;
        spill.reputation_base[0] = 21_000;
        spill.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([source.clone(), spill.clone()]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);
        let mut template = RepSpilloverTemplateLikeCpp::empty_like_cpp();
        template.faction[0] = 103;
        template.faction_rate[0] = 1.0;
        template.faction_rank[0] = ReputationRankLikeCpp::Honored.as_u8();

        let outcome = mgr.set_reputation_like_cpp(
            &source,
            1_000,
            set_reputation_options_for_test_like_cpp(false),
            &faction_store,
            Some(&template),
            None,
            None,
            None,
        );

        assert!(outcome.applied);
        assert!(outcome.spillover_mutations.is_empty());
        assert_eq!(mgr.get_state(27).expect("spill state").standing, 0);

        let mut no_spillover_options = set_reputation_options_for_test_like_cpp(false);
        no_spillover_options.no_spillover = true;
        let no_spillover = mgr.set_reputation_like_cpp(
            &source,
            2_000,
            no_spillover_options,
            &faction_store,
            Some(&template),
            None,
            None,
            None,
        );

        assert!(no_spillover.spillover_mutations.is_empty());
        assert_eq!(mgr.get_state(27).expect("spill state").standing, 0);
        assert_eq!(mgr.get_state(26).expect("source state").standing, 2_000);
    }

    #[test]
    fn set_reputation_like_cpp_redirects_positive_exalted_gain_to_paragon_faction() {
        let mut source = FactionEntry::for_test_like_cpp(104, 28);
        source.reputation_race_mask[0] = 1;
        source.reputation_base[0] = 42_000;
        source.reputation_max[0] = 42_000;
        source.paragon_faction_id = 105;
        let mut paragon = FactionEntry::for_test_like_cpp(105, 29);
        paragon.reputation_race_mask[0] = 1;
        paragon.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([source.clone(), paragon.clone()]);
        let paragon_store = ParagonReputationStore::from_entries([ParagonReputationEntry {
            id: 4,
            faction_id: 105,
            level_threshold: 10_000,
            quest_id: 800,
        }]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, Some(&paragon_store), 1, 1);

        let outcome = mgr.set_reputation_like_cpp(
            &source,
            1_000,
            set_reputation_options_for_test_like_cpp(true),
            &faction_store,
            None,
            None,
            Some(&paragon_store),
            None,
        );

        assert!(outcome.applied);
        assert_eq!(
            outcome.primary_mutation.as_ref().map(|entry| entry.0),
            Some(105)
        );
        assert_eq!(outcome.send_state_rep_list_id, Some(29));
        assert_eq!(mgr.get_state(28).expect("source state").standing, 0);
        assert_eq!(mgr.get_state(29).expect("paragon state").standing, 1_000);
    }

    #[test]
    fn set_reputation_like_cpp_applies_dbc_sub_faction_spillover_with_cap() {
        let mut source = FactionEntry::for_test_like_cpp(110, 30);
        source.reputation_race_mask[0] = 1;
        source.reputation_max[0] = 42_000;
        let mut child = FactionEntry::for_test_like_cpp(111, 31);
        child.reputation_race_mask[0] = 1;
        child.parent_faction_id = 110;
        child.parent_faction_mod[0] = 0.5;
        child.parent_faction_cap[0] = ReputationRankLikeCpp::Exalted.as_u8();
        child.reputation_max[0] = 42_000;
        let mut capped = FactionEntry::for_test_like_cpp(112, 32);
        capped.reputation_race_mask[0] = 1;
        capped.parent_faction_id = 110;
        capped.parent_faction_mod[0] = 1.0;
        capped.parent_faction_cap[0] = ReputationRankLikeCpp::Honored.as_u8();
        capped.reputation_base[0] = 21_000;
        capped.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([source.clone(), child.clone(), capped]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        let outcome = mgr.set_reputation_like_cpp(
            &source,
            1_000,
            set_reputation_options_for_test_like_cpp(false),
            &faction_store,
            None,
            None,
            None,
            None,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.spillover_mutations.len(), 1);
        assert_eq!(outcome.spillover_mutations[0].0, 111);
        assert_eq!(outcome.spillover_mutations[0].1.reputation_change, 500);
        assert_eq!(mgr.get_state(31).expect("child state").standing, 500);
        assert_eq!(mgr.get_state(32).expect("capped state").standing, 0);
    }

    #[test]
    fn set_reputation_like_cpp_applies_dbc_sister_spillover_when_parent_has_no_bar() {
        let mut parent = FactionEntry::for_test_like_cpp(120, -1);
        parent.reputation_race_mask[0] = 1;
        let mut source = FactionEntry::for_test_like_cpp(121, 33);
        source.reputation_race_mask[0] = 1;
        source.parent_faction_id = 120;
        source.parent_faction_mod[1] = 0.5;
        source.reputation_max[0] = 42_000;
        let mut sister = FactionEntry::for_test_like_cpp(122, 34);
        sister.reputation_race_mask[0] = 1;
        sister.parent_faction_id = 120;
        sister.parent_faction_mod[0] = 0.25;
        sister.parent_faction_cap[0] = ReputationRankLikeCpp::Exalted.as_u8();
        sister.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([parent, source.clone(), sister]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        let outcome = mgr.set_reputation_like_cpp(
            &source,
            1_000,
            set_reputation_options_for_test_like_cpp(false),
            &faction_store,
            None,
            None,
            None,
            None,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.spillover_mutations.len(), 1);
        assert_eq!(outcome.spillover_mutations[0].0, 122);
        assert_eq!(outcome.spillover_mutations[0].1.reputation_change, 125);
        assert_eq!(mgr.get_state(34).expect("sister state").standing, 125);
    }

    #[test]
    fn set_reputation_like_cpp_spills_to_parent_when_parent_header_shows_bar() {
        let mut parent = FactionEntry::for_test_like_cpp(130, 35);
        parent.reputation_race_mask[0] = 1;
        parent.reputation_flags[0] = ReputationFlagsLikeCpp::HEADER_SHOWS_BAR.bits();
        parent.reputation_max[0] = 42_000;
        let mut source = FactionEntry::for_test_like_cpp(131, 36);
        source.reputation_race_mask[0] = 1;
        source.parent_faction_id = 130;
        source.parent_faction_mod[1] = 0.5;
        source.reputation_max[0] = 42_000;
        let mut sister = FactionEntry::for_test_like_cpp(132, 37);
        sister.reputation_race_mask[0] = 1;
        sister.parent_faction_id = 130;
        sister.parent_faction_mod[0] = 1.0;
        sister.parent_faction_cap[0] = ReputationRankLikeCpp::Exalted.as_u8();
        sister.reputation_max[0] = 42_000;
        let faction_store = FactionStore::from_entries([parent, source.clone(), sister]);
        let mut mgr = ReputationMgrLikeCpp::new_like_cpp();
        mgr.initialize_like_cpp(&faction_store, None, 1, 1);

        let outcome = mgr.set_reputation_like_cpp(
            &source,
            1_000,
            set_reputation_options_for_test_like_cpp(false),
            &faction_store,
            None,
            None,
            None,
            None,
        );

        assert!(outcome.applied);
        assert_eq!(outcome.spillover_mutations.len(), 1);
        assert_eq!(outcome.spillover_mutations[0].0, 130);
        assert_eq!(outcome.spillover_mutations[0].1.reputation_change, 500);
        assert_eq!(mgr.get_state(35).expect("parent state").standing, 500);
        assert_eq!(mgr.get_state(37).expect("sister state").standing, 0);
    }

    fn currency_entry_for_test_like_cpp(id: u32, max_qty: u32) -> CurrencyTypesEntry {
        CurrencyTypesEntry {
            id,
            category_id: 0,
            inventory_icon_file_id: 0,
            spell_weight: 0,
            spell_category: 0,
            max_qty,
            max_earnable_per_week: 0,
            quality: 0,
            faction_id: 0,
            award_condition_id: 0,
            flags: CurrencyTypesFlags::empty(),
            flags_b: CurrencyTypesFlagsB::empty(),
        }
    }

    fn set_reputation_options_for_test_like_cpp(incremental: bool) -> SetReputationOptionsLikeCpp {
        SetReputationOptionsLikeCpp {
            incremental,
            spillover_only: false,
            no_spillover: false,
            reputation_gain_rate: 1.0,
            paragon_reward_quest_status_none_like_cpp: true,
            renown_current_level_like_cpp: 0,
            renown_currency_increased_cap_quantity_like_cpp: 0,
            player_race: 1,
            player_class: 1,
        }
    }
}

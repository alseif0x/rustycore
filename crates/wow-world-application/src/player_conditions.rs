// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Owned player-condition values and their selected live scalar projection.
mod inputs;
pub use inputs::PlayerConditionProjectionInputsLikeCpp;

use std::collections::HashMap;
use std::sync::Arc;

use wow_conditions::{
    PlayerConditionContextLikeCpp, PlayerConditionPartyStatusLikeCpp,
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};
use wow_data::{
    AreaTableStore, ChrSpecializationStore, ConditionEntriesByTypeStore, ItemStore,
    PlayerConditionAuraLikeCpp, PlayerConditionCountLikeCpp, PlayerConditionQuestKillLikeCpp,
    PlayerConditionReputationLikeCpp, PlayerConditionSkillLikeCpp, PlayerConditionStore,
};
use wow_world_core::session::PlayerConditionAccessLikeCpp;
use wow_world_core::session::InventoryValuationCatalogViewLikeCpp;
use wow_world_social::SessionSocialLimits;
use wow_entities::{
    AuraApplicationLikeCpp, PlayerCurrency, PlayerInventoryItem, PlayerQuestGameplayState,
    PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP,
};
use wow_world_inventory::InventoryState;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_instances::InstanceState;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_spell::SessionSpellState;

/// Owned values projected for one condition-evaluation pass. Each source
/// snapshot is built at its original call site; this type only owns the same
/// vectors that the World adapter previously assembled inline.
#[derive(Debug, Clone, Default)]
pub struct RepresentedPlayerConditionContextLikeCpp {
    spells: Vec<u32>,
    items: Vec<PlayerConditionCountLikeCpp>,
    currencies: Vec<PlayerConditionCountLikeCpp>,
    completed_quests: Vec<u32>,
    current_quests: Vec<u32>,
    complete_quests: Vec<u32>,
    auras: Vec<PlayerConditionAuraLikeCpp>,
    skills: Vec<PlayerConditionSkillLikeCpp>,
    reputations: Vec<PlayerConditionReputationLikeCpp>,
    explored_area_ids: Vec<u16>,
    parent_area_ids: Vec<u32>,
    achievements: Vec<u16>,
    lfg_values: Vec<PlayerConditionCountLikeCpp>,
    modifier_tree_ids: Vec<u32>,
    quest_kills: Vec<PlayerConditionQuestKillLikeCpp>,
    avg_item_level: f32,
    avg_equipped_item_level: f32,
    mainhand_weapon_subclass: Option<u8>,
}

/// Borrowed, selected owners used to build the owned PlayerCondition vectors.
/// Construction only retains references; `project_like_cpp` performs every
/// read at the caller's original projection point.
pub struct PlayerConditionProjectionCxLikeCpp<'a> {
    pub(crate) player: PlayerConditionAccessLikeCpp<'a>,
    pub(crate) inventory: &'a InventoryState,
    pub(crate) social: &'a SessionSocialLimits,
    pub(crate) chr_specializations: Option<&'a ChrSpecializationStore>,
    #[cfg(any(test, feature = "test-fixtures"))]
    spell_state: &'a SessionSpellState,
    #[cfg(any(test, feature = "test-fixtures"))]
    quest_state: &'a crate::SessionQuestState,
    #[cfg(any(test, feature = "test-fixtures"))]
    instances: &'a InstanceState,
    #[cfg(any(test, feature = "test-fixtures"))]
    battleground_fixture: &'a wow_world_core::session::BattlegroundState,
    pub(crate) valuation_catalogs: InventoryValuationCatalogViewLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) reputation_state: &'a wow_entities::PlayerReputationStateLikeCpp,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) battleground_status: &'a Option<u8>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_skill_records_complete: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) in_combat: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) consumer_test: bool,
}

impl<'a> PlayerConditionProjectionCxLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        player: PlayerConditionAccessLikeCpp<'a>,
        inventory: &'a InventoryState,
        social: &'a SessionSocialLimits,
        chr_specializations: Option<&'a ChrSpecializationStore>,
        #[cfg(any(test, feature = "test-fixtures"))]
        spell_state: &'a SessionSpellState,
        #[cfg(any(test, feature = "test-fixtures"))]
        quest_state: &'a crate::SessionQuestState,
        #[cfg(any(test, feature = "test-fixtures"))]
        instances: &'a InstanceState,
        #[cfg(any(test, feature = "test-fixtures"))]
        battleground_fixture: &'a wow_world_core::session::BattlegroundState,
        valuation_catalogs: InventoryValuationCatalogViewLikeCpp<'a>,
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation_state: &'a wow_entities::PlayerReputationStateLikeCpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        battleground_status: &'a Option<u8>,
        #[cfg(any(test, feature = "test-fixtures"))]
        player_skill_records_complete: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        in_combat: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        consumer_test: bool,
    ) -> Self {
        Self {
            player,
            inventory,
            social,
            chr_specializations,
            #[cfg(any(test, feature = "test-fixtures"))]
            spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            quest_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            instances,
            #[cfg(any(test, feature = "test-fixtures"))]
            battleground_fixture,
            valuation_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))]
            reputation_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            battleground_status,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_skill_records_complete,
            #[cfg(any(test, feature = "test-fixtures"))]
            in_combat,
            #[cfg(any(test, feature = "test-fixtures"))]
            consumer_test,
        }
    }

    pub(crate) fn player_access_like_cpp(&self) -> &PlayerConditionAccessLikeCpp<'a> {
        &self.player
    }

    pub(crate) fn complete_player_skill_records_like_cpp(
        &self,
    ) -> Option<std::collections::HashMap<u16, wow_world_core::session::RepresentedPlayerSkillLikeCpp>> {
        self.player.complete_player_skill_records_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            *self.player_skill_records_complete,
        )
    }

    /// Preserve World `known_spells_like_cpp`'s test-only absent-owner fallback
    /// at each item-use query rather than reusing the condition projection's
    /// broader fixture snapshot.
    pub(crate) fn known_spells_for_item_use_like_cpp(&self) -> Vec<i32> {
        let canonical = self.player.known_spells_snapshot_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.consumer_test
            && canonical.is_none()
            && self.player.player_handle_absent_like_cpp()
        {
            return self
                .spell_state
                .represented_spell_runtime_fixture_like_cpp()
                .known_spells;
        }
        canonical.unwrap_or_default()
    }

    pub fn condition_context_like_cpp<'b>(
        &self,
        values: &'b RepresentedPlayerConditionContextLikeCpp,
    ) -> Option<PlayerConditionContextLikeCpp<'b>>
    where
        'a: 'b,
    {
        #[cfg(any(test, feature = "test-fixtures"))]
        let consumer_test = self.consumer_test;
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let consumer_test = false;
        values.as_context(
            &self.player,
            self.social,
            self.chr_specializations,
            consumer_test,
        )
    }

    pub(crate) fn project_values_before_item_level_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerConditionContextLikeCpp> {
        let mut values = RepresentedPlayerConditionContextLikeCpp::default();

        let spells = self.player.known_spells_snapshot_like_cpp().or_else(|| {
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.consumer_test && self.player.player_handle_absent_like_cpp() {
                return Some(
                    self.spell_state
                        .represented_spell_runtime_fixture_like_cpp()
                        .known_spells,
                );
            }
            None
        });
        values.project_known_spells_like_cpp(spells.as_deref().unwrap_or_default());

        let inventory_access = self.player.owned_inventory_access_like_cpp();
        values.project_inventory_item_counts_like_cpp(
            self.inventory
                .represented_inventory_item_counts_with_access_like_cpp(&inventory_access)?,
        );

        let currency_access = self.player.owned_player_currency_access_like_cpp();
        values.project_currencies_like_cpp(
            self.inventory
                .player_currencies_with_access_like_cpp(&currency_access)?,
        );

        let hydration_access = self.player.player_registry_hydration_access_like_cpp();
        let quests = hydration_access
            .owned_player_quest_gameplay_snapshot_like_cpp()
            .or_else(|| {
                #[cfg(any(test, feature = "test-fixtures"))]
                if self.consumer_test && self.player.player_handle_absent_like_cpp() {
                    return Some(self.quest_state.player_quest_gameplay_fixture_like_cpp());
                }
                None
            })?;
        values.project_quest_gameplay_like_cpp(&quests);

        values.project_visible_auras_like_cpp(self.player.resolved_visible_auras_like_cpp()?);
        values.project_skill_values_like_cpp(&self.player.resolved_skill_values_like_cpp()?);

        let (_, area_id) = self.player.player_zone_area_like_cpp()?;
        let explored_zones = self.player.explored_zones_snapshot_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.instances.represented_explored_zones_for_test_like_cpp(),
        )?;
        values.project_explored_zones_like_cpp(
            &explored_zones,
            area_id,
            self.valuation_catalogs.area_table_store_like_cpp().map(Arc::as_ref),
        );

        let inventory_items =
            self.inventory
                .resolved_inventory_items_with_access_like_cpp(&inventory_access)?;
        values.project_mainhand_weapon_subclass_like_cpp(
            &inventory_items,
            self.valuation_catalogs.item_store_like_cpp(),
        );

        Some(values)
    }

    pub fn project_like_cpp(&self) -> Option<RepresentedPlayerConditionContextLikeCpp> {
        let mut values = self.project_values_before_item_level_like_cpp()?;
        let avg_item_level =
            crate::inventory_valuation::represented_avg_total_item_level_like_cpp(self)?;
        let inventory_access = self.player.owned_inventory_access_like_cpp();
        let valuation_access = self.player.inventory_valuation_access_like_cpp();
        let modifier_access = self.player.owned_item_modifiers_access_like_cpp();
        let avg_equipped_item_level = self
            .inventory
            .represented_avg_equipped_item_level_with_access_like_cpp(
                &inventory_access,
                &valuation_access,
                &modifier_access,
                &self.valuation_catalogs,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.player.fixture_player_level_like_cpp(),
                crate::inventory_valuation::MIN_ITEM_LEVEL_LIKE_CPP,
                crate::inventory_valuation::MAX_ITEM_LEVEL_LIKE_CPP,
            )?;
        values.set_item_level_values_like_cpp(avg_item_level, avg_equipped_item_level);
        Some(values)
    }

    pub(crate) fn trainer_spell_condition_proof_like_cpp(
        &self,
        condition_store: Option<&ConditionEntriesByTypeStore>,
        player_condition_store: Option<&PlayerConditionStore>,
        trainer_id: u32,
        spell_id: u32,
    ) -> crate::TrainerAdmissionProofLikeCpp {
        let Some(condition_store) = condition_store else {
            return crate::TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let Some(player_object) = self.player.build_condition_player_object_like_cpp() else {
            return crate::TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let Some(player_condition_context) = self.project_like_cpp() else {
            return crate::TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let Some(player_unit_snapshot) = self
            .player
            .condition_player_unit_snapshot_like_cpp()
        else {
            return crate::TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let player_snapshot = self.player.condition_player_snapshot_like_cpp();
        let mut unsupported = false;
        let meets = wow_conditions::is_object_meeting_trainer_spell_conditions_like_cpp(
            condition_store,
            trainer_id,
            spell_id,
            Some(&player_object),
            |condition, source_info| {
                source_info.set_unit_target_snapshot(0, player_unit_snapshot);
                source_info.set_player_target_snapshot(0, player_snapshot);
                if let Some(store) = player_condition_store {
                    source_info.set_player_condition_store(store);
                    if let Some(context) =
                        self.condition_context_like_cpp(&player_condition_context)
                    {
                        source_info.set_player_condition_context(0, context);
                    }
                }
                match wow_conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |current_area, required_area| current_area == required_area,
                ) {
                    wow_conditions::ConditionMeetResult::Evaluated(value) => value,
                    wow_conditions::ConditionMeetResult::Unsupported => {
                        unsupported = true;
                        false
                    }
                }
            },
        );
        crate::trainer_condition_admission_proof_like_cpp(meets, unsupported)
    }
}

impl RepresentedPlayerConditionContextLikeCpp {
    /// Project one source collection at its original PlayerCondition snapshot
    /// point. The holder owns only the vectors consumed by the evaluator.
    fn project_known_spells_like_cpp(&mut self, spells: &[i32]) {
        self.spells = spells
            .iter()
            .filter_map(|spell_id| u32::try_from(*spell_id).ok())
            .collect();
    }

    fn project_inventory_item_counts_like_cpp(&mut self, counts: HashMap<u32, u32>) {
        self.items = counts
            .into_iter()
            .map(|(id, count)| PlayerConditionCountLikeCpp { id, count })
            .collect();
    }

    fn project_currencies_like_cpp(&mut self, currencies: HashMap<u32, PlayerCurrency>) {
        self.currencies = currencies
            .iter()
            .map(|(&id, currency)| PlayerConditionCountLikeCpp {
                id,
                count: currency.quantity,
            })
            .collect();
    }

    fn project_quest_gameplay_like_cpp(&mut self, quests: &PlayerQuestGameplayState) {
        self.completed_quests = quests.rewarded_quest_ids_like_cpp().iter().copied().collect();
        self.current_quests = quests
            .statuses_like_cpp()
            .iter()
            .filter_map(|(&quest_id, status)| {
                (status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    || status.status == QUEST_STATUS_COMPLETE_LIKE_CPP)
                    .then_some(quest_id)
            })
            .collect();
        self.complete_quests = quests
            .statuses_like_cpp()
            .iter()
            .filter_map(|(&quest_id, status)| {
                (status.status == QUEST_STATUS_COMPLETE_LIKE_CPP).then_some(quest_id)
            })
            .collect();
    }

    fn project_visible_auras_like_cpp(
        &mut self,
        auras: HashMap<u8, AuraApplicationLikeCpp>,
    ) {
        self.auras = auras
            .into_values()
            .filter_map(|aura| {
                Some(PlayerConditionAuraLikeCpp {
                    spell_id: u32::try_from(aura.spell_id).ok()?,
                    stacks: aura.stack_count,
                })
            })
            .collect();
    }

    fn project_skill_values_like_cpp(&mut self, skills: &HashMap<u16, u16>) {
        self.skills = skills
            .iter()
            .map(|(&id, &value)| PlayerConditionSkillLikeCpp { id, value })
            .collect();
    }

    fn project_explored_zones_like_cpp(
        &mut self,
        explored_zones: &[u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
        area_id: u32,
        area_table_store: Option<&AreaTableStore>,
    ) {
        (self.explored_area_ids, self.parent_area_ids) = area_table_store
            .map(|store| {
                (
                    store.explored_area_ids_from_blocks_like_cpp(explored_zones),
                    store.parent_area_ids_like_cpp(area_id),
                )
            })
            .unwrap_or_default();
    }

    fn project_mainhand_weapon_subclass_like_cpp(
        &mut self,
        inventory_items: &HashMap<u8, PlayerInventoryItem>,
        item_store: Option<&Arc<ItemStore>>,
    ) {
        self.mainhand_weapon_subclass = None;
        for (&slot, inventory_item) in inventory_items {
            if slot == wow_entities::EQUIPMENT_SLOT_MAINHAND {
                self.mainhand_weapon_subclass = item_store
                    .and_then(|store| store.get(inventory_item.entry_id))
                    .map(|record| record.subclass_id);
            }
        }
    }

    pub(crate) fn set_item_level_values_like_cpp(
        &mut self,
        avg_item_level: f32,
        avg_equipped_item_level: f32,
    ) {
        self.avg_item_level = avg_item_level;
        self.avg_equipped_item_level = avg_equipped_item_level;
    }

    /// Build the borrowed evaluator context using live, selected owner reads.
    /// Call this at the original condition callback point; it intentionally
    /// does not cache the player's scalar state in the owned projection.
    pub fn as_context<'a>(
        &'a self,
        player: &PlayerConditionAccessLikeCpp<'_>,
        social: &'a SessionSocialLimits,
        chr_specializations: Option<&'a ChrSpecializationStore>,
        consumer_test: bool,
    ) -> Option<PlayerConditionContextLikeCpp<'a>> {
        let class = player.player_class_like_cpp();
        let (_, area_id) = player.player_zone_area_like_cpp()?;
        Some(PlayerConditionContextLikeCpp {
            race: player.player_race_like_cpp(),
            class_mask: if class == 0 {
                0
            } else {
                1u32 << u32::from(class.saturating_sub(1))
            },
            gender: player.player_gender_like_cpp(),
            native_gender: player.player_gender_like_cpp(),
            power_type: -1,
            power: 0,
            max_power: 0,
            primary_specialization_id: player
                .primary_specialization_id_like_cpp(consumer_test)
                .filter(|spec_id| *spec_id != 0),
            skills: &self.skills,
            language_skill: 0,
            reputations: &self.reputations,
            current_pvp_faction: 0,
            pvp_medals_mask: 0,
            lifetime_max_pvp_rank: 0,
            movement_flags: [0, 0],
            mainhand_weapon_subclass: self.mainhand_weapon_subclass,
            party_status: if social
                .resolved_group_guid_with_access_like_cpp(
                    &player.player_group_owner_access_like_cpp(),
                    consumer_test,
                )
                .is_some()
            {
                PlayerConditionPartyStatusLikeCpp::InParty
            } else {
                PlayerConditionPartyStatusLikeCpp::Solo
            },
            completed_quests: &self.completed_quests,
            current_quests: &self.current_quests,
            complete_quests: &self.complete_quests,
            spells: &self.spells,
            items: &self.items,
            currencies: &self.currencies,
            explored_area_ids: &self.explored_area_ids,
            auras: &self.auras,
            weather_id: 0,
            achievements: &self.achievements,
            lfg_values: &self.lfg_values,
            area_id,
            parent_area_ids: &self.parent_area_ids,
            expansion: player.expansion_like_cpp() as i8,
            server_expansion: player.account_expansion_like_cpp() as i8,
            is_game_master: player.is_game_master_like_cpp(),
            phase_satisfied: true,
            quest_kill_id: 0,
            quest_kills: &self.quest_kills,
            avg_item_level: self.avg_item_level,
            avg_equipped_item_level: self.avg_equipped_item_level,
            modifier_tree_ids: &self.modifier_tree_ids,
            chr_specializations,
            world_state_expressions: None,
            world_state_expression_context: None,
        })
    }
}

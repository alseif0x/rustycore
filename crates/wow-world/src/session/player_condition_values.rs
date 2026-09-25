// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player condition values: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::is_player_meeting_condition_like_cpp;
use super::{Arc, EQUIPMENT_SLOT_MAINHAND, PlayerConditionAuraLikeCpp};
use super::{PlayerConditionContextLikeCpp, PlayerConditionCountLikeCpp};
use super::{PlayerConditionPartyStatusLikeCpp, PlayerConditionQuestKillLikeCpp};
use super::{PlayerConditionReputationLikeCpp, PlayerConditionSkillLikeCpp, WorldSession};

#[derive(Debug, Clone, Default)]
pub(crate) struct RepresentedPlayerConditionContextLikeCpp {
    pub(in crate::session) spells: Vec<u32>,
    pub(in crate::session) items: Vec<PlayerConditionCountLikeCpp>,
    pub(in crate::session) currencies: Vec<PlayerConditionCountLikeCpp>,
    pub(in crate::session) completed_quests: Vec<u32>,
    pub(in crate::session) current_quests: Vec<u32>,
    pub(in crate::session) complete_quests: Vec<u32>,
    pub(in crate::session) auras: Vec<PlayerConditionAuraLikeCpp>,
    pub(in crate::session) skills: Vec<PlayerConditionSkillLikeCpp>,
    pub(in crate::session) reputations: Vec<PlayerConditionReputationLikeCpp>,
    pub(in crate::session) explored_area_ids: Vec<u16>,
    pub(in crate::session) parent_area_ids: Vec<u32>,
    pub(in crate::session) achievements: Vec<u16>,
    pub(in crate::session) lfg_values: Vec<PlayerConditionCountLikeCpp>,
    pub(in crate::session) modifier_tree_ids: Vec<u32>,
    pub(in crate::session) quest_kills: Vec<PlayerConditionQuestKillLikeCpp>,
    pub(in crate::session) avg_item_level: f32,
    pub(in crate::session) avg_equipped_item_level: f32,
    pub(in crate::session) mainhand_weapon_subclass: Option<u8>,
}

/// Represented subset of C++ `Player::m_unitData` item-level cap fields
/// consumed by `Item::GetItemLevel(Player const*)`.
pub(crate) type RepresentedItemLevelCapsLikeCpp = wow_entities::PlayerItemLevelCapsLikeCpp;

impl RepresentedPlayerConditionContextLikeCpp {
    pub(crate) fn as_context<'a>(
        &'a self,
        session: &'a WorldSession,
    ) -> Option<PlayerConditionContextLikeCpp<'a>> {
        let class = session.player_class_like_cpp();
        let (_, area_id) = session.player_zone_area_like_cpp()?;
        Some(PlayerConditionContextLikeCpp {
            race: session.player_race_like_cpp(),
            class_mask: if class == 0 {
                0
            } else {
                1u32 << u32::from(class.saturating_sub(1))
            },
            gender: session.player_gender_like_cpp(),
            native_gender: session.player_gender_like_cpp(),
            power_type: -1,
            power: 0,
            max_power: 0,
            primary_specialization_id: session
                .represented_primary_specialization_id_like_cpp()
                .filter(|spec_id| *spec_id != 0),
            skills: &self.skills,
            language_skill: 0,
            reputations: &self.reputations,
            current_pvp_faction: 0,
            pvp_medals_mask: 0,
            lifetime_max_pvp_rank: 0,
            movement_flags: [0, 0],
            mainhand_weapon_subclass: self.mainhand_weapon_subclass,
            party_status: if session.resolved_group_guid_like_cpp().is_some() {
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
            expansion: session.expansion as i8,
            server_expansion: session.account_expansion as i8,
            is_game_master: session.security > 0,
            phase_satisfied: true,
            quest_kill_id: 0,
            quest_kills: &self.quest_kills,
            avg_item_level: self.avg_item_level,
            avg_equipped_item_level: self.avg_equipped_item_level,
            modifier_tree_ids: &self.modifier_tree_ids,
            chr_specializations: session.chr_specialization_store().map(Arc::as_ref),
            world_state_expressions: None,
            world_state_expression_context: None,
        })
    }
}

impl WorldSession {
    pub(crate) fn represented_player_condition_context_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerConditionContextLikeCpp> {
        let spells = self
            .known_spells_like_cpp()
            .iter()
            .filter_map(|spell_id| u32::try_from(*spell_id).ok())
            .collect();
        let items = self
            .represented_inventory_item_counts_like_cpp()?
            .into_iter()
            .map(|(id, count)| PlayerConditionCountLikeCpp { id, count })
            .collect();
        let currencies = self
            .player_currencies_like_cpp()?
            .iter()
            .map(|(&id, currency)| PlayerConditionCountLikeCpp {
                id,
                count: currency.quantity,
            })
            .collect();
        let quests = self.player_quest_gameplay_snapshot_like_cpp()?;
        let completed_quests = quests
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect();
        let current_quests = quests
            .statuses_like_cpp()
            .iter()
            .filter_map(|(&quest_id, status)| {
                (status.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    || status.status == wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
                    .then_some(quest_id)
            })
            .collect();
        let complete_quests = quests
            .statuses_like_cpp()
            .iter()
            .filter_map(|(&quest_id, status)| {
                (status.status == wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
                    .then_some(quest_id)
            })
            .collect();
        let auras = self
            .resolved_player_visible_auras_like_cpp()?
            .into_values()
            .filter_map(|aura| {
                Some(PlayerConditionAuraLikeCpp {
                    spell_id: u32::try_from(aura.spell_id).ok()?,
                    stacks: aura.stack_count,
                })
            })
            .collect();
        let skills = self
            .resolved_player_skill_values_like_cpp()?
            .iter()
            .map(|(&id, &value)| PlayerConditionSkillLikeCpp { id, value })
            .collect();
        let (_, area_id) = self.player_zone_area_like_cpp()?;
        let explored_zones = self.player_explored_zones_snapshot_like_cpp()?;
        let (explored_area_ids, parent_area_ids) = self
            .area_table_store
            .as_ref()
            .map(|store| {
                (
                    store.explored_area_ids_from_blocks_like_cpp(&explored_zones),
                    store.parent_area_ids_like_cpp(area_id),
                )
            })
            .unwrap_or_default();

        let mut mainhand_weapon_subclass = None;
        for (&slot, inventory_item) in &self.resolved_inventory_items_like_cpp()? {
            if slot == EQUIPMENT_SLOT_MAINHAND {
                mainhand_weapon_subclass = self
                    .items
                    .store
                    .as_ref()
                    .and_then(|store| store.get(inventory_item.entry_id))
                    .map(|record| record.subclass_id);
            }
        }

        Some(RepresentedPlayerConditionContextLikeCpp {
            spells,
            items,
            currencies,
            completed_quests,
            current_quests,
            complete_quests,
            auras,
            skills,
            explored_area_ids,
            parent_area_ids,
            avg_item_level: self.represented_avg_total_item_level_like_cpp()?,
            avg_equipped_item_level: self.represented_avg_equipped_item_level_like_cpp()?,
            mainhand_weapon_subclass,
            ..Default::default()
        })
    }

    pub(crate) fn represented_meets_player_condition_id_like_cpp(
        &self,
        player_condition_id: u32,
    ) -> bool {
        if player_condition_id == 0 {
            return true;
        }

        let Some(store) = self.player_condition_store.as_ref() else {
            return false;
        };
        let Some(condition) = store.get(player_condition_id) else {
            return true;
        };

        let Some(context) = self.represented_player_condition_context_like_cpp() else {
            return false;
        };
        context
            .as_context(self)
            .is_some_and(|context| is_player_meeting_condition_like_cpp(condition, &context))
    }
}

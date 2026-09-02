// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Levels, experience, skills, reputation, quests and currency.

use super::*;

impl Player {
    pub fn spell_runtime_like_cpp(&self) -> &PlayerSpellRuntimeState {
        &self.gameplay_state().spells
    }

    pub fn replace_spell_runtime_like_cpp(&mut self, state: PlayerSpellRuntimeState) {
        self.gameplay_state_mut().spells = state;
    }

    pub fn talent_runtime_like_cpp(&self) -> &PlayerTalentRuntimeState {
        &self.gameplay_state().talents
    }

    pub fn replace_talent_runtime_like_cpp(&mut self, state: PlayerTalentRuntimeState) {
        self.gameplay_state_mut().talents = state;
    }

    pub fn create_mode_like_cpp(&self) -> u8 {
        self.gameplay_state().create_mode
    }

    pub fn set_create_mode_like_cpp(&mut self, create_mode: u8) {
        self.gameplay_state_mut().create_mode = create_mode;
    }

    pub fn shapeshift_form_id_like_cpp(&self) -> u32 {
        self.gameplay_state().shapeshift_form_id
    }

    pub fn set_shapeshift_form_id_like_cpp(&mut self, form_id: u32) {
        self.gameplay_state_mut().shapeshift_form_id = form_id;
    }

    pub fn loot_specialization_id_like_cpp(&self) -> u32 {
        self.gameplay_state().loot_specialization_id
    }

    pub fn set_loot_specialization_id_like_cpp(&mut self, spec_id: u32) {
        self.gameplay_state_mut().loot_specialization_id = spec_id;
    }

    pub fn primary_specialization_id_like_cpp(&self) -> u32 {
        self.data().current_spec_id
    }

    pub fn replace_skill_records_like_cpp(
        &mut self,
        mut records: Vec<PlayerSkillRecord>,
        loaded: bool,
        complete: bool,
        occupied_slots: Option<u16>,
        non_durable_tombstones: BTreeSet<u16>,
    ) {
        records.sort_unstable_by_key(|record| record.skill_line_id);
        self.gameplay_state.skills = records;
        self.gameplay_state.skills_loaded = loaded;
        self.gameplay_state.skills_complete = loaded && complete;
        self.gameplay_state.occupied_skill_slots = occupied_slots;
        self.gameplay_state.non_durable_skill_tombstones = non_durable_tombstones;
    }

    pub fn skill_records_like_cpp(&self) -> &[PlayerSkillRecord] {
        &self.gameplay_state.skills
    }

    pub fn skill_records_loaded_like_cpp(&self) -> bool {
        self.gameplay_state.skills_loaded
    }

    pub fn skill_records_complete_like_cpp(&self) -> bool {
        self.gameplay_state.skills_complete
    }

    pub fn occupied_skill_slots_like_cpp(&self) -> Option<u16> {
        self.gameplay_state.occupied_skill_slots
    }

    pub fn non_durable_skill_tombstones_like_cpp(&self) -> &BTreeSet<u16> {
        &self.gameplay_state.non_durable_skill_tombstones
    }

    pub fn enchanting_skill_value_like_cpp(&self, enchanting_skill_id: u16) -> u16 {
        self.gameplay_state
            .skills
            .iter()
            .find(|record| record.skill_line_id == u32::from(enchanting_skill_id))
            .map(|record| record.current_value)
            .unwrap_or(0)
    }

    pub fn set_forced_reputation_rank_like_cpp(&mut self, faction_id: u32, forced: bool) {
        if forced {
            self.forced_reaction_faction_ids.insert(faction_id);
        } else {
            self.forced_reaction_faction_ids.remove(&faction_id);
        }
    }

    pub fn has_forced_reputation_rank_like_cpp(&self, faction_id: u32) -> bool {
        self.forced_reaction_faction_ids.contains(&faction_id)
    }

    pub fn forced_reputation_faction_ids_like_cpp(&self) -> &HashSet<u32> {
        &self.forced_reaction_faction_ids
    }

    pub fn replace_forced_reputation_faction_ids_like_cpp(&mut self, faction_ids: HashSet<u32>) {
        self.forced_reaction_faction_ids = faction_ids;
    }

    pub fn is_at_war_with_faction_like_cpp(&self, faction_id: u32) -> bool {
        self.gameplay_state
            .reputations
            .iter()
            .find(|rep| rep.faction_id == faction_id)
            .is_some_and(|rep| rep.flags & REPUTATION_FLAG_AT_WAR_LIKE_CPP != 0)
    }

    pub fn has_reputation_state_like_cpp(&self, faction_id: u32) -> bool {
        self.gameplay_state
            .reputations
            .iter()
            .any(|rep| rep.faction_id == faction_id)
    }

    pub const fn shared_quest_id(&self) -> u32 {
        self.shared_quest_id
    }

    pub fn set_honor_level_like_cpp(&mut self, level: i32) {
        self.set_player_i32(PLAYER_DATA_HONOR_LEVEL_BIT, level, |data| {
            &mut data.honor_level
        });
    }

    /// C++ `Player::GetMoney` (`Player.h:1690`).
    pub const fn money(&self) -> u64 {
        self.active_data.coinage
    }

    pub fn set_money(&mut self, value: u64) {
        self.set_active_u64(ACTIVE_PLAYER_DATA_COINAGE_BIT, value, |data| {
            &mut data.coinage
        });
    }

    pub fn mark_money_changed(&mut self) {
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_COINAGE_BIT);
    }

    pub fn modify_money(&mut self, amount: i64) -> bool {
        if amount == 0 {
            return true;
        }

        if amount < 0 {
            self.set_money(
                self.active_data
                    .coinage
                    .saturating_sub(amount.unsigned_abs()),
            );
            return true;
        }

        let amount = amount as u64;
        if amount <= MAX_MONEY_AMOUNT && self.active_data.coinage <= MAX_MONEY_AMOUNT - amount {
            self.set_money(self.active_data.coinage + amount);
            true
        } else {
            false
        }
    }

    pub fn set_xp(&mut self, xp: i32) {
        self.set_active_i32(ACTIVE_PLAYER_DATA_XP_BIT, xp, |data| &mut data.xp);
    }

    /// Mirror C++ `ModifyValue(&m_activePlayerData->XP)`, which marks XP as
    /// changed before the caller mutates it. This matters when `GiveXP`
    /// crosses a level boundary and the final remainder equals the old XP.
    pub fn mark_xp_changed_like_cpp(&mut self) {
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_XP_BIT);
    }

    pub fn set_next_level_xp(&mut self, xp: i32) {
        self.set_active_i32(ACTIVE_PLAYER_DATA_NEXT_LEVEL_XP_BIT, xp, |data| {
            &mut data.next_level_xp
        });
    }

    pub fn set_scaling_player_level_delta_like_cpp(&mut self, delta: i32) {
        self.set_active_i32_in_section(
            ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT,
            ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT,
            delta,
            |data| &mut data.scaling_player_level_delta,
        );
    }

    /// Mirror the unconditional C++ `ModifyValue` performed by `Player::SetXP`.
    pub fn mark_scaling_player_level_delta_changed_like_cpp(&mut self) {
        self.mark_active_player_data_section(
            ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT,
            ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT,
        );
    }

    pub fn set_xp_rest_info_like_cpp(&mut self, threshold: u32, state_id: u8) {
        self.set_rest_info_like_cpp(0, threshold, state_id);
    }

    pub fn set_rest_info_like_cpp(&mut self, index: usize, threshold: u32, state_id: u8) {
        let Some(rest_info) = self.active_data.rest_info.get_mut(index) else {
            return;
        };
        if rest_info.threshold != threshold || rest_info.state_id != state_id {
            rest_info.threshold = threshold;
            rest_info.state_id = state_id;
            // C++ `RestMgr::SetRestBonus` calls both `SetRestThreshold` and
            // `SetRestState` whenever either visible value changes. Each
            // `ModifyValue` marks its field before the value comparison, so
            // the nested RestInfo mask is always parent + both fields.
            self.rest_info_change_masks[index] |= 0x07;
            self.mark_active_player_data_section(
                ACTIVE_PLAYER_DATA_REST_INFO_PARENT_BIT,
                ACTIVE_PLAYER_DATA_REST_INFO_FIRST_BIT + index,
            );
        }
    }

    /// Build an isolated nested RestInfo values update with an explicit mask.
    pub fn prepare_rest_info_values_update_like_cpp(
        &mut self,
        index: usize,
        threshold: u32,
        state_id: u8,
        nested_mask: u8,
    ) {
        let Some(rest_info) = self.active_data.rest_info.get_mut(index) else {
            return;
        };
        let nested_mask = nested_mask & 0x07;
        if nested_mask & 0x01 == 0 {
            return;
        }
        rest_info.threshold = threshold;
        rest_info.state_id = state_id;
        self.rest_info_change_masks[index] = nested_mask;
        self.mark_active_player_data_section(
            ACTIVE_PLAYER_DATA_REST_INFO_PARENT_BIT,
            ACTIVE_PLAYER_DATA_REST_INFO_FIRST_BIT + index,
        );
    }

    pub fn set_honor_next_level_like_cpp(&mut self, xp: i32) {
        self.set_active_i32_in_section(
            ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT,
            ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT,
            xp,
            |data| &mut data.honor_next_level,
        );
    }

    pub fn update_honor_next_level_like_cpp(&mut self) {
        self.set_honor_next_level_like_cpp(PLAYER_HONOR_NEXT_LEVEL_XP_LIKE_CPP);
    }

    pub fn is_max_honor_level_like_cpp(&self) -> bool {
        self.data.honor_level >= PLAYER_MAX_HONOR_LEVEL_LIKE_CPP
    }

    pub fn add_honor_xp_like_cpp(&mut self, xp: u32, player_level: u8) -> bool {
        if xp < 1
            || player_level < PLAYER_LEVEL_MIN_HONOR_LIKE_CPP
            || self.is_max_honor_level_like_cpp()
        {
            return false;
        }

        if self.active_data.honor_next_level <= 0 {
            self.update_honor_next_level_like_cpp();
        }

        let mut new_honor_xp = self.active_data.honor.max(0) as u32;
        new_honor_xp = new_honor_xp.saturating_add(xp);
        let mut next_honor_level_xp = self.active_data.honor_next_level.max(1) as u32;

        while new_honor_xp >= next_honor_level_xp && !self.is_max_honor_level_like_cpp() {
            new_honor_xp -= next_honor_level_xp;

            let next_level = (self.data.honor_level + 1).min(PLAYER_MAX_HONOR_LEVEL_LIKE_CPP);
            self.set_honor_level_like_cpp(next_level);
            self.update_honor_next_level_like_cpp();
            next_honor_level_xp = self.active_data.honor_next_level.max(1) as u32;
        }

        let residual = if self.is_max_honor_level_like_cpp() {
            0
        } else {
            new_honor_xp.min(i32::MAX as u32) as i32
        };
        self.set_honor_like_cpp(residual);
        true
    }

    pub fn set_watched_faction_index_like_cpp(&mut self, index: i32) {
        self.set_active_i32(
            ACTIVE_PLAYER_DATA_WATCHED_FACTION_INDEX_BIT,
            index,
            |data| &mut data.watched_faction_index,
        );
    }

    pub fn set_quest_completed_bit_like_cpp(&mut self, quest_bit: u32, completed: bool) -> bool {
        if quest_bit == 0 {
            return false;
        }

        let field_offset = (quest_bit - 1) / QUESTS_COMPLETED_BITS_PER_BLOCK;
        if field_offset as usize >= QUESTS_COMPLETED_BITS_SIZE {
            return false;
        }

        let flag = 1u64 << ((quest_bit - 1) % QUESTS_COMPLETED_BITS_PER_BLOCK);
        let field_offset = field_offset as usize;
        let target = &mut self.active_data.quest_completed[field_offset];
        let new_value = if completed {
            *target | flag
        } else {
            *target & !flag
        };

        if *target == new_value {
            return false;
        }

        *target = new_value;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT,
            ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT,
            field_offset,
        );
        true
    }

    pub fn quest_completed_block_like_cpp(&self, index: usize) -> Option<u64> {
        self.active_data.quest_completed.get(index).copied()
    }
}

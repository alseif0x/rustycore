// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Levels, experience, skills, reputation, quests and currency.

use super::*;

impl PlayerRestState {
    /// C++ RestMgr::SetRestFlag (RestMgr.cpp:95-109). Read the clock only
    /// when the first rest flag becomes active, preserving represented timing.
    pub fn set_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
        now: impl FnOnce() -> u64,
    ) -> bool {
        let old_mask = self.rest_flag_mask;
        self.location_initialized = true;
        self.rest_flag_mask |= rest_flag;
        let crossed_zero = old_mask == 0 && self.rest_flag_mask != 0;
        if crossed_zero {
            self.rest_time_secs = now();
        }
        if trigger_id != 0 {
            self.inn_area_trigger_id = trigger_id;
        }
        if crossed_zero && self.defer_flag_sync {
            self.deferred_flag_update_dirty = true;
        }
        crossed_zero
    }

    /// C++ RestMgr::RemoveRestFlag (RestMgr.cpp:112-122), retaining Rust's
    /// existing tavern-trigger cleanup and deferred publication bookkeeping.
    pub fn remove_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        let old_mask = self.rest_flag_mask;
        self.rest_flag_mask &= !rest_flag;
        if old_mask != self.rest_flag_mask {
            self.location_initialized = true;
        }
        let tavern = 0x1; // C++ RestMgr.h:53 REST_FLAG_IN_TAVERN.
        if (rest_flag & tavern) != 0 && (self.rest_flag_mask & tavern) == 0 {
            self.inn_area_trigger_id = 0;
        }
        let crossed_zero = old_mask != 0 && self.rest_flag_mask == 0;
        if crossed_zero {
            self.rest_time_secs = 0;
            if self.defer_flag_sync {
                self.deferred_flag_update_dirty = true;
            }
        }
        crossed_zero
    }
}

impl PlayerTalentRuntimeState {
    /// C++ Player::GetNextResetTalentsCost (Player.cpp:3472-3503).
    /// Keep the represented saturating arithmetic for anomalous timestamps/costs;
    /// this is a read of this Player's reset history, not a Session policy.
    pub fn next_reset_talents_cost_like_cpp(&self, now_secs: u64) -> u32 {
        let gold = 10_000;
        let reset_cost = self.reset_talents_cost;
        if reset_cost < gold {
            return gold;
        }
        if reset_cost < 5 * gold {
            return 5 * gold;
        }
        if reset_cost < 10 * gold {
            return 10 * gold;
        }

        let months = now_secs.saturating_sub(self.reset_talents_time_secs) / (30 * 24 * 60 * 60);
        if months > 0 {
            let reduced = i64::from(reset_cost)
                - i64::try_from(5 * u64::from(gold) * months).unwrap_or(i64::MAX);
            return reduced.max(i64::from(10 * gold)) as u32;
        }

        reset_cost.saturating_add(5 * gold).min(50 * gold)
    }
}

#[cfg(test)]
mod rest_flag_tests {
    use super::*;

    #[test]
    fn rest_flags_only_start_and_stop_time_at_zero_crossings() {
        for deferred in [false, true] {
            let mut state = PlayerRestState {
                defer_flag_sync: deferred,
                ..Default::default()
            };
            assert!(!state.set_flag_like_cpp(0, 0, || panic!("empty mask reads no clock")));
            assert!(state.location_initialized);
            let calls = std::cell::Cell::new(0);
            assert!(state.set_flag_like_cpp(1, 77, || {
                calls.set(calls.get() + 1);
                100
            }));
            assert_eq!(calls.get(), 1);
            assert_eq!(state.rest_time_secs, 100);
            assert_eq!(state.inn_area_trigger_id, 77);
            assert_eq!(state.deferred_flag_update_dirty, deferred);
            state.deferred_flag_update_dirty = false;
            assert!(!state.set_flag_like_cpp(1, 88, || panic!("repeat reads no clock")));
            assert!(!state.set_flag_like_cpp(2, 0, || panic!("second flag reads no clock")));
            assert_eq!(state.inn_area_trigger_id, 88);
            assert!(!state.deferred_flag_update_dirty);
            assert!(!state.remove_flag_like_cpp(1));
            assert_eq!(state.inn_area_trigger_id, 0);
            assert_eq!(state.rest_time_secs, 100);
            assert_eq!(state.rest_flag_mask, 2);
            assert!(!state.deferred_flag_update_dirty);
            assert!(state.remove_flag_like_cpp(2));
            assert_eq!(state.rest_time_secs, 0);
            assert_eq!(state.rest_flag_mask, 0);
            assert_eq!(state.deferred_flag_update_dirty, deferred);
            state.deferred_flag_update_dirty = false;
            assert!(!state.remove_flag_like_cpp(2));
            assert!(!state.deferred_flag_update_dirty);
        }
    }

    #[test]
    fn absent_tavern_removal_preserves_uninitialized_location_and_other_rest_fields() {
        let mut state = PlayerRestState {
            inn_area_trigger_id: 77,
            rest_bonus: 123.5,
            rest_honor_bonus: 55.0,
            rest_time_secs: 100,
            deferred_flag_update_dirty: true,
            ..Default::default()
        };
        let mut expected = state.clone();
        expected.inn_area_trigger_id = 0;
        assert!(!state.remove_flag_like_cpp(1));
        assert_eq!(state, expected);
    }
}

#[cfg(test)]
mod talent_point_tests {
    use super::*;

    #[test]
    fn reset_fee_preserves_steps_monthly_decay_and_represented_arithmetic_bounds() {
        let month = 30 * 24 * 60 * 60;
        let now = 10 * month;
        for (cost, stamp, expected) in [
            (0, now, 10_000),
            (9_999, now, 10_000),
            (10_000, now, 50_000),
            (49_999, now, 50_000),
            (50_000, now, 100_000),
            (99_999, now, 100_000),
            (100_000, now, 150_000),
            (500_000, now, 500_000),
            (500_000, now - month + 1, 500_000),
            (500_000, now - month, 450_000),
            (100_000, now - month, 100_000),
            (500_000, 0, 100_000),
            (100_000, now + 1, 150_000),
            (u32::MAX, now, 500_000),
        ] {
            let state = PlayerTalentRuntimeState {
                reset_talents_cost: cost,
                reset_talents_time_secs: stamp,
                ..Default::default()
            };
            let before = state.clone();
            assert_eq!(
                state.next_reset_talents_cost_like_cpp(now),
                expected,
                "cost={cost}, timestamp={stamp}"
            );
            assert_eq!(state, before, "a price query cannot mutate talent state");
        }
    }

    #[test]
    fn refresh_counts_only_valid_active_talents_and_marks_the_same_update_field() {
        let mut player = Player::new(None, false);
        player.gameplay_state_mut().talents.active_group = 1;
        player.gameplay_state_mut().talents.talent_groups[0].insert(10, 8);
        player.gameplay_state_mut().talents.talent_groups[1].insert(20, 2);
        player.gameplay_state_mut().talents.talent_groups[1].insert(30, 1);
        player.gameplay_state_mut().quest_rewarded_talent_points = 5;
        let before = player.talent_runtime_like_cpp().clone();
        player.clear_data_changes();
        let mut visited = Vec::new();
        assert_eq!(
            player.refresh_represented_talent_points_like_cpp(71, |id, rank| {
                visited.push((id, rank));
                id == 20
            }),
            73
        );
        assert_eq!(visited, vec![(20, 2), (30, 1)]);
        assert_eq!(player.talent_runtime_like_cpp(), &before);
        assert_eq!(player.gameplay_state().quest_rewarded_talent_points, 5);
        assert_eq!(player.active_data().character_points, 73);
        let mut direct = Player::new(None, false);
        direct.clear_data_changes();
        direct.set_character_points_like_cpp(73);
        assert_eq!(
            player.active_player_data_changes_mask().blocks(),
            direct.active_player_data_changes_mask().blocks()
        );
        player.clear_data_changes();
        assert_eq!(
            player.refresh_represented_talent_points_like_cpp(71, |id, _| id == 20),
            73
        );
        assert!(!player.active_player_data_changes_mask().is_any_set());
    }

    #[test]
    fn refresh_preserves_empty_group_saturation_and_signed_field_bounds() {
        let mut player = Player::new(None, false);
        player.gameplay_state_mut().talents.talent_groups[0].insert(20, 2);
        assert_eq!(
            player.refresh_represented_talent_points_like_cpp(2, |_, _| true),
            0
        );
        player.gameplay_state_mut().talents.active_group = u8::MAX;
        assert_eq!(
            player.refresh_represented_talent_points_like_cpp(7, |_, _| {
                panic!("invalid group has no talents to validate")
            }),
            7
        );
        player.gameplay_state_mut().quest_rewarded_talent_points = u32::MAX;
        assert_eq!(
            player.refresh_represented_talent_points_like_cpp(0, |_, _| true),
            i32::MAX
        );
    }
}

impl Player {
    /// Install the immutable process-owned `player_xp_for_level` view used by
    /// C++ `Player::GiveLevel`. The canonical Player retains only the shared
    /// read handle, so active and far-teleport-detached residence use the same
    /// table without a Session mirror.
    pub fn install_player_xp_table_like_cpp(&mut self, table: Arc<Vec<u32>>) {
        self.player_xp_table_like_cpp = Some(table);
    }

    pub fn player_xp_for_level_like_cpp(&self, level: u8) -> Option<u32> {
        self.player_xp_table_like_cpp
            .as_ref()?
            .get(usize::from(level))
            .copied()
    }

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

    /// Refresh the represented CharacterPoints projection on its canonical owner.
    /// C++ Player.cpp:26356,28670 reads the active talent group and quest rewards
    /// from Player. The caller supplies level/catalog policy without retaining it
    /// here; the predicate must only read immutable data, never re-enter the owner.
    /// This preserves the port's validity filter and bounds, not full InitTalentForLevel.
    pub fn refresh_represented_talent_points_like_cpp(
        &mut self,
        base_points: u32,
        mut valid_talent: impl FnMut(u32, u8) -> bool,
    ) -> i32 {
        let runtime = self.talent_runtime_like_cpp();
        let spent: u32 = runtime
            .talent_groups
            .get(usize::from(runtime.active_group))
            .into_iter()
            .flat_map(|talents| talents.iter())
            .filter(|(talent_id, rank)| valid_talent(**talent_id, **rank))
            .map(|(_, rank)| u32::from(*rank) + 1)
            .sum();
        let total = base_points + self.gameplay_state().quest_rewarded_talent_points;
        let points = total.saturating_sub(spent).min(i32::MAX as u32) as i32;
        self.set_character_points_like_cpp(points);
        points
    }

    pub fn taxi_state_like_cpp(&self) -> &PlayerTaxiState {
        &self.gameplay_state().taxi
    }

    pub fn replace_taxi_state_like_cpp(&mut self, state: PlayerTaxiState) {
        self.gameplay_state_mut().taxi = state;
    }

    pub fn rest_state_like_cpp(&self) -> &PlayerRestState {
        &self.gameplay_state().rest
    }

    pub fn replace_rest_state_like_cpp(&mut self, state: PlayerRestState) {
        self.gameplay_state_mut().rest = state;
    }

    /// C++ RestMgr constructor (RestMgr.cpp:26-30) and LoadRestBonus
    /// (Player.cpp:17693). The caller supplies its validated persisted state.
    /// Reset transient location state without replacing loaded Player flags or
    /// unrelated XP/honor/logout state; offline accumulation happens afterward.
    pub fn load_xp_rest_bonus_like_cpp(&mut self, state_id: u8, bonus: f32) {
        self.mutate_rest_state_like_cpp(|state| {
            state.rest_flag_mask = 0;
            state.location_initialized = false;
            state.defer_flag_sync = false;
            state.deferred_flag_update_dirty = false;
            state.inn_area_trigger_id = 0;
            state.rest_time_secs = 0;
            state.rest_state = state_id;
            state.rest_bonus = bonus;
        });
    }

    /// Mutate this Player's RestMgr state and refresh its represented fields.
    /// C++ RestMgr.cpp:65-80,95-122 keeps rest values and flags on one Player.
    /// Preserve the Rust load boundary: do not normalize flags until location
    /// initialization, and keep the existing threshold clamp/update-mask rules.
    pub fn mutate_rest_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut PlayerRestState) -> R,
    ) -> R {
        let state = &mut self.gameplay_state_mut().rest;
        let result = f(state);
        let threshold = state.rest_bonus.clamp(0.0, u32::MAX as f32) as u32;
        let state_id = state.rest_state;
        let resting = state
            .location_initialized
            .then_some(state.rest_flag_mask != 0);
        self.set_xp_rest_info_like_cpp(threshold, state_id);
        if let Some(resting) = resting {
            let resting_flag = 0x0000_0020; // C++ PLAYER_FLAGS_RESTING.
            if resting {
                self.set_player_flag(resting_flag);
            } else {
                self.remove_player_flag(resting_flag);
            }
        }
        result
    }

    pub fn difficulty_preferences_like_cpp(&self) -> (u32, u32, u32) {
        let state = self.gameplay_state();
        (
            state.dungeon_difficulty_id,
            state.raid_difficulty_id,
            state.legacy_raid_difficulty_id,
        )
    }

    pub fn replace_difficulty_preferences_like_cpp(
        &mut self,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) {
        let state = self.gameplay_state_mut();
        state.dungeon_difficulty_id = dungeon;
        state.raid_difficulty_id = raid;
        state.legacy_raid_difficulty_id = legacy_raid;
    }

    pub fn pass_on_group_loot_like_cpp(&self) -> bool {
        self.gameplay_state().pass_on_group_loot
    }

    pub fn set_pass_on_group_loot_like_cpp(&mut self, pass_on_group_loot: bool) {
        self.gameplay_state_mut().pass_on_group_loot = pass_on_group_loot;
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
            if !self
                .gameplay_state
                .forced_reputation_ranks
                .iter()
                .any(|(id, _)| *id == faction_id)
            {
                self.gameplay_state
                    .forced_reputation_ranks
                    .push((faction_id, 0));
            }
        } else {
            self.gameplay_state
                .forced_reputation_ranks
                .retain(|(id, _)| *id != faction_id);
        }
    }

    pub fn has_forced_reputation_rank_like_cpp(&self, faction_id: u32) -> bool {
        self.gameplay_state
            .forced_reputation_ranks
            .iter()
            .any(|(id, _)| *id == faction_id)
    }

    pub fn forced_reputation_faction_ids_like_cpp(&self) -> impl Iterator<Item = u32> + '_ {
        self.gameplay_state
            .forced_reputation_ranks
            .iter()
            .map(|(id, _)| *id)
    }

    pub fn replace_forced_reputation_faction_ids_like_cpp(&mut self, faction_ids: HashSet<u32>) {
        self.gameplay_state.forced_reputation_ranks =
            faction_ids.into_iter().map(|id| (id, 0)).collect();
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

    pub fn watched_faction_index_like_cpp(&self) -> i32 {
        self.active_data().watched_faction_index
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

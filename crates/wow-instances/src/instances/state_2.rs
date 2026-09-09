//! Instance lifecycle state state definitions, part 2 of 2.
//!
//! Separated from the lib.rs root under #658. Behaviour is preserved.

use super::*;

impl InstanceScriptBase {
    pub fn new(difficulty_id: u32, boss_count: usize) -> Self {
        Self {
            difficulty_id,
            header: String::new(),
            bosses: vec![BossInfo::default(); boss_count],
            persistent_values: Vec::new(),
            combat_resurrections: CombatResurrectionTracker::default(),
            entrance_id: 0,
            temporary_entrance_id: 0,
            activated_area_triggers: HashSet::new(),
        }
    }

    pub fn difficulty_id(&self) -> u32 {
        self.difficulty_id
    }

    pub fn set_header(&mut self, header: impl Into<String>) {
        self.header = header.into();
    }

    pub fn header(&self) -> &str {
        &self.header
    }

    pub fn boss_count(&self) -> usize {
        self.bosses.len()
    }

    pub fn boss(&self, boss_id: u32) -> Option<&BossInfo> {
        self.bosses.get(boss_id as usize)
    }

    pub fn is_encounter_in_progress_like_cpp(&self) -> bool {
        self.bosses
            .iter()
            .any(|boss| boss.state == EncounterState::InProgress)
    }

    pub fn boss_state(&self, boss_id: u32) -> EncounterState {
        self.boss(boss_id)
            .map(|boss| boss.state)
            .unwrap_or(EncounterState::ToBeDecided)
    }

    pub fn set_boss_state_like_cpp(&mut self, boss_id: u32, state: EncounterState) -> bool {
        let Some(boss) = self.bosses.get_mut(boss_id as usize) else {
            return false;
        };
        boss.state = state;
        true
    }

    pub fn set_boss_state_planned_like_cpp(
        &mut self,
        store: &DungeonEncounterStore,
        boss_id: u32,
        state: EncounterState,
        has_alive_world_boss_minion: bool,
    ) -> Option<BossStateTransitionPlan> {
        let boss = self.bosses.get_mut(boss_id as usize)?;
        let previous_state = boss.state;

        if previous_state == EncounterState::ToBeDecided {
            boss.state = state;
            return None;
        }
        if previous_state == state || previous_state == EncounterState::Done {
            return None;
        }
        if state == EncounterState::Done && has_alive_world_boss_minion {
            return None;
        }

        let dungeon_encounter_id = boss
            .dungeon_encounter_for_difficulty(store, self.difficulty_id)
            .map(|encounter| encounter.id)
            .filter(|_| state == EncounterState::Done);
        boss.state = state;

        Some(BossStateTransitionPlan {
            boss_id,
            previous_state,
            new_state: state,
            initialize_combat_resurrections: state == EncounterState::InProgress,
            reset_combat_resurrections: matches!(
                state,
                EncounterState::Fail | EncounterState::Done
            ),
            send_encounter_start: state == EncounterState::InProgress,
            send_encounter_end: matches!(state, EncounterState::Fail | EncounterState::Done),
            notify_players_start: state == EncounterState::InProgress,
            notify_players_end: matches!(state, EncounterState::Fail | EncounterState::Done),
            dungeon_encounter_id,
            update_lock: dungeon_encounter_id.is_some(),
            update_criteria: dungeon_encounter_id.is_some(),
            send_boss_kill_credit: dungeon_encounter_id.is_some(),
            update_lfg: dungeon_encounter_id.is_some(),
            update_doors_minions_and_spawn_groups: true,
        })
    }

    pub fn create_like_cpp(&mut self) {
        for boss_id in 0..self.bosses.len() {
            self.set_boss_state_like_cpp(boss_id as u32, EncounterState::NotStarted);
        }
    }

    pub fn register_persistent_value_like_cpp(
        &mut self,
        name: impl Into<String>,
        value: PersistentInstanceScriptValue,
    ) {
        self.persistent_values.push((name.into(), value));
    }

    pub fn persistent_value(&self, name: &str) -> Option<&PersistentInstanceScriptValue> {
        self.persistent_values
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value))
    }

    pub fn combat_resurrections(&self) -> CombatResurrectionTracker {
        self.combat_resurrections
    }

    pub fn initialize_combat_resurrections_like_cpp(&mut self, charges: u8, interval_ms: u32) {
        self.combat_resurrections
            .initialize_like_cpp(charges, interval_ms);
    }

    pub fn reset_combat_resurrections_like_cpp(&mut self) {
        self.combat_resurrections.reset_like_cpp();
    }

    pub fn update_combat_resurrection_like_cpp(
        &mut self,
        diff_ms: u32,
        player_count: u32,
    ) -> Option<CombatResurrectionEvent> {
        self.combat_resurrections
            .update_like_cpp(diff_ms, player_count)
    }

    pub fn use_combat_resurrection_like_cpp(&mut self) -> CombatResurrectionEvent {
        self.combat_resurrections.use_charge_like_cpp()
    }

    pub fn set_entrance_location_like_cpp(&mut self, world_safe_location_id: u32) {
        self.entrance_id = world_safe_location_id;
        self.temporary_entrance_id = 0;
    }

    pub fn set_temporary_entrance_location_like_cpp(&mut self, world_safe_location_id: u32) {
        self.temporary_entrance_id = world_safe_location_id;
    }

    pub const fn entrance_location_like_cpp(&self) -> u32 {
        if self.temporary_entrance_id != 0 {
            self.temporary_entrance_id
        } else {
            self.entrance_id
        }
    }

    pub const fn compute_entrance_location_for_completed_encounters_like_cpp(
        &self,
        _completed_encounters_mask: u32,
    ) -> Option<u32> {
        None
    }

    pub fn entrance_location_for_completed_encounters_like_cpp(
        &self,
        is_using_encounter_locks: bool,
        completed_encounters_mask: u32,
    ) -> Option<u32> {
        if !is_using_encounter_locks {
            return Some(self.entrance_id);
        }

        self.compute_entrance_location_for_completed_encounters_like_cpp(completed_encounters_mask)
    }

    pub fn mark_area_trigger_done_like_cpp(&mut self, id: u32) {
        self.activated_area_triggers.insert(id);
    }

    pub fn reset_area_trigger_done_like_cpp(&mut self, id: u32) {
        self.activated_area_triggers.remove(&id);
    }

    pub fn is_area_trigger_done_like_cpp(&self, id: u32) -> bool {
        self.activated_area_triggers.contains(&id)
    }

    pub fn get_save_data_like_cpp(&self) -> String {
        let header = serde_json::to_string(&self.header).unwrap();
        let boss_states = self
            .bosses
            .iter()
            .map(|boss| (boss.state as u8).to_string())
            .collect::<Vec<_>>()
            .join(",");
        let mut data = format!(
            "{{\"{}\":{},\"{}\":[{}]",
            INSTANCE_SCRIPT_HEADER_KEY, header, INSTANCE_SCRIPT_BOSS_STATES_KEY, boss_states
        );
        if !self.persistent_values.is_empty() {
            let additional = self
                .persistent_values
                .iter()
                .map(|(name, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(name).unwrap(),
                        value.to_json_value()
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            data.push_str(&format!(
                ",\"{}\":{{{}}}",
                INSTANCE_SCRIPT_ADDITIONAL_DATA_KEY, additional
            ));
        }
        data.push('}');
        data
    }

    pub fn load_save_data_like_cpp(
        &mut self,
        data: &str,
    ) -> Result<(), InstanceScriptDataLoadError> {
        let doc: serde_json::Value =
            serde_json::from_str(data).map_err(|_| InstanceScriptDataLoadError::MalformedJson)?;
        let root = doc
            .as_object()
            .ok_or(InstanceScriptDataLoadError::RootIsNotAnObject)?;

        let header = root
            .get(INSTANCE_SCRIPT_HEADER_KEY)
            .ok_or(InstanceScriptDataLoadError::MissingHeader)?;
        if header.as_str() != Some(self.header()) {
            return Err(InstanceScriptDataLoadError::UnexpectedHeader);
        }

        let boss_states = root
            .get(INSTANCE_SCRIPT_BOSS_STATES_KEY)
            .ok_or(InstanceScriptDataLoadError::MissingBossStates)?
            .as_array()
            .ok_or(InstanceScriptDataLoadError::BossStatesIsNotAnArray)?;

        for (boss_id, boss_state) in boss_states.iter().enumerate() {
            if boss_id >= self.bosses.len() {
                return Err(InstanceScriptDataLoadError::UnknownBoss);
            }

            let state_value = boss_state
                .as_i64()
                .ok_or(InstanceScriptDataLoadError::BossStateIsNotANumber)?;
            let Some(state) = EncounterState::from_i64_like_cpp(state_value) else {
                continue;
            };
            let state = state.save_load_normalized_like_cpp();
            if state != EncounterState::ToBeDecided {
                self.set_boss_state_like_cpp(boss_id as u32, state);
            }
        }

        let Some(additional_data) = root.get(INSTANCE_SCRIPT_ADDITIONAL_DATA_KEY) else {
            return Ok(());
        };
        let additional_data = additional_data
            .as_object()
            .ok_or(InstanceScriptDataLoadError::AdditionalDataIsNotAnObject)?;
        for (name, value) in &mut self.persistent_values {
            let Some(saved_value) = additional_data.get(name) else {
                continue;
            };
            if saved_value.is_null() {
                continue;
            }
            *value = PersistentInstanceScriptValue::from_json_number_like_cpp(saved_value)
                .ok_or(InstanceScriptDataLoadError::AdditionalDataUnexpectedValueType)?;
        }

        Ok(())
    }

    /// C++ `InstanceScript::LoadDungeonEncounterData(uint32, array<uint32, 4>)`.
    pub fn load_dungeon_encounter_data(
        &mut self,
        store: &DungeonEncounterStore,
        boss_id: u32,
        dungeon_encounter_ids: [u32; MAX_DUNGEON_ENCOUNTERS_PER_BOSS],
    ) {
        let Some(boss) = self.bosses.get_mut(boss_id as usize) else {
            return;
        };

        for (slot, encounter_id) in dungeon_encounter_ids.into_iter().enumerate() {
            boss.dungeon_encounters[slot] = store.get(encounter_id).map(|entry| entry.id);
        }
    }

    /// C++ `InstanceScript::LoadDungeonEncounterData(T const&)`.
    pub fn load_dungeon_encounter_data_rows(
        &mut self,
        store: &DungeonEncounterStore,
        rows: impl IntoIterator<Item = DungeonEncounterData>,
    ) {
        for row in rows {
            self.load_dungeon_encounter_data(store, row.boss_id, row.dungeon_encounter_ids);
        }
    }

    /// C++ `InstanceScript::GetBossDungeonEncounter(uint32)`.
    pub fn boss_dungeon_encounter<'a>(
        &self,
        store: &'a DungeonEncounterStore,
        boss_id: u32,
    ) -> Option<&'a DungeonEncounterEntry> {
        self.boss(boss_id)?
            .dungeon_encounter_for_difficulty(store, self.difficulty_id)
    }

    pub fn is_encounter_completed_like_cpp(
        &self,
        store: &DungeonEncounterStore,
        dungeon_encounter_id: u32,
    ) -> bool {
        self.bosses.iter().any(|boss| {
            boss.dungeon_encounters
                .iter()
                .flatten()
                .filter_map(|encounter_id| store.get(*encounter_id))
                .any(|encounter| encounter.id == dungeon_encounter_id)
                && boss.state == EncounterState::Done
        })
    }

    pub fn is_encounter_completed_in_mask_by_boss_id_like_cpp(
        &self,
        store: &DungeonEncounterStore,
        completed_encounters_mask: u32,
        boss_id: u32,
    ) -> bool {
        let Some(encounter) = self.boss_dungeon_encounter(store, boss_id) else {
            return false;
        };
        let Ok(bit) = u32::try_from(encounter.bit) else {
            return false;
        };

        (completed_encounters_mask & (1u32 << bit)) != 0
            && self.boss_state(boss_id) == EncounterState::Done
    }

    /// C++ `InstanceScript::GetBossDungeonEncounter(Creature const*)` after
    /// the `dynamic_cast<BossAI const*>` succeeds.
    pub fn boss_dungeon_encounter_for_boss_ai<'a, T: BossAiLikeCpp>(
        &self,
        store: &'a DungeonEncounterStore,
        boss_ai: Option<&T>,
    ) -> Option<&'a DungeonEncounterEntry> {
        self.boss_dungeon_encounter(store, boss_ai?.boss_id())
    }
}

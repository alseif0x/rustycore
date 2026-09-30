//! Fixture access projects existing canonical rows and uses the actual writers.

use super::*;

impl TestPlayer {
    pub(in crate::player::spell_runtime::reconstruction_tests) fn set_spell_chain_store(
        &mut self,
        store: Arc<wow_data::SpellChainStoreLikeCpp>,
    ) {
        self.chains = Some(store);
    }
    pub(in crate::player::spell_runtime::reconstruction_tests) fn set_spell_learn_spell_store(
        &mut self,
        store: Arc<wow_data::SpellLearnSpellStoreLikeCpp>,
    ) {
        self.learned = Some(store);
    }
    pub(in crate::player::spell_runtime::reconstruction_tests) fn set_spell_learn_skill_store(
        &mut self,
        store: Arc<wow_data::SpellLearnSkillStoreLikeCpp>,
    ) {
        self.learn_skills = Some(store);
    }
    pub(in crate::player::spell_runtime::reconstruction_tests) fn set_skill_store(
        &mut self,
        store: Arc<wow_data::SkillStore>,
    ) {
        self.skills = Some(store);
    }
    pub(in crate::player::spell_runtime::reconstruction_tests) fn set_skill_line_store(
        &mut self,
        store: Arc<wow_data::SkillLineStore>,
    ) {
        self.lines = Some(store);
    }
    pub(in crate::player::spell_runtime::reconstruction_tests) fn set_skill_tiers_store(
        &mut self,
        store: Arc<wow_data::SkillTiersStoreLikeCpp>,
    ) {
        self.tiers = Some(store);
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn set_loaded_player_identity_like_cpp(
        &mut self,
        _map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) {
        let gender = match gender {
            1 => wow_constants::Gender::Female,
            2 => wow_constants::Gender::None,
            _ => wow_constants::Gender::Male,
        };
        self.player.set_race_class_gender(race, class, gender);
        self.player.set_level_and_gray_level_like_cpp(level, 0);
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn set_known_spells_like_cpp(
        &mut self,
        spells: Vec<i32>,
    ) {
        self.player
            .gameplay_state_mut()
            .spells
            .replace_known_spell_ids_like_cpp(spells);
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn known_spells_like_cpp(
        &self,
    ) -> &[i32] {
        self.player.spell_runtime_like_cpp().known_spells_like_cpp()
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn represented_override_spells_like_cpp(
        &self,
    ) -> HashMap<i32, BTreeSet<i32>> {
        self.player
            .spell_runtime_like_cpp()
            .override_spells_like_cpp()
            .iter()
            .map(|(&id, rows)| (id, rows.clone()))
            .collect()
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn add_represented_override_spell_like_cpp(
        &mut self,
        overridden: i32,
        replacement: i32,
    ) {
        self.player
            .gameplay_state_mut()
            .spells
            .add_override_spell_like_cpp(overridden, replacement);
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn reset_represented_talents_like_cpp(
        &mut self,
    ) {
        self.player
            .gameplay_state_mut()
            .talents
            .clear_talents_like_cpp();
        self.player
            .gameplay_state_mut()
            .spells
            .clear_trait_and_override_state_like_cpp();
        self.player
            .gameplay_state_mut()
            .spells
            .set_acquisition_snapshot_completeness_like_cpp(false, false);
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn represented_dependent_known_spells_like_cpp(
        &self,
    ) -> &BTreeSet<i32> {
        self.player
            .spell_runtime_like_cpp()
            .dependent_known_spells_like_cpp()
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn represented_favorite_known_spells_like_cpp(
        &self,
    ) -> &BTreeSet<i32> {
        self.player
            .spell_runtime_like_cpp()
            .favorite_known_spells_like_cpp()
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn replace_player_skill_records_like_cpp(
        &mut self,
        records: HashMap<u16, PlayerSkillRecord>,
        loaded: bool,
        complete: bool,
    ) -> bool {
        self.player.replace_represented_skill_records_like_cpp(
            records.into_iter().collect(),
            loaded,
            complete,
        );
        true
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn set_player_skill_records_like_cpp(
        &mut self,
        records: HashMap<u16, PlayerSkillRecord>,
    ) -> bool {
        self.replace_player_skill_records_like_cpp(records, true, false)
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn player_skill_records_like_cpp(
        &self,
    ) -> HashMap<u16, PlayerSkillRecord> {
        self.player
            .skill_records_like_cpp()
            .iter()
            .filter_map(|record| {
                u16::try_from(record.skill_line_id)
                    .ok()
                    .map(|id| (id, record.clone()))
            })
            .collect()
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn player_skill_value_like_cpp(
        &self,
        id: u16,
    ) -> u16 {
        self.player_skill_records_like_cpp()
            .get(&id)
            .map(|record| record.current_value)
            .unwrap_or(0)
    }
    pub(in crate::player::spell_runtime::reconstruction_tests) fn player_skill_max_value_like_cpp(
        &self,
        id: u16,
    ) -> u16 {
        self.player_skill_records_like_cpp()
            .get(&id)
            .map(|record| record.max_value)
            .unwrap_or(0)
    }
    pub(in crate::player::spell_runtime::reconstruction_tests) fn player_skill_records_loaded_like_cpp(
        &self,
    ) -> bool {
        self.player.skill_records_loaded_like_cpp()
    }
    pub(in crate::player::spell_runtime::reconstruction_tests) fn complete_player_skill_records_like_cpp(
        &self,
    ) -> Option<HashMap<u16, PlayerSkillRecord>> {
        self.player
            .skill_records_complete_like_cpp()
            .then(|| self.player_skill_records_like_cpp())
    }

    pub(super) fn write_skill(&mut self, write: LearnedSkillWrite) {
        let mut records = self.player_skill_records_like_cpp();
        let previous = records.get(&write.skill_id);
        let occupied = self
            .player
            .skill_records_complete_like_cpp()
            .then(|| self.player.occupied_skill_slots_like_cpp())
            .flatten();
        let prepared = PlayerGameplayState::prepare_skill_write(
            write.skill_id,
            write.step,
            write.value,
            write.max_value,
            previous,
        );
        records.insert(write.skill_id, prepared.record.clone());
        self.player.replace_represented_skill_records_like_cpp(
            records.into_iter().collect(),
            true,
            occupied.is_some(),
        );
        if let Some(occupied) = occupied {
            let _ = self
                .player
                .authorize_occupied_skill_slots_like_cpp(prepared.occupied_slots_after(occupied));
        }
    }
}

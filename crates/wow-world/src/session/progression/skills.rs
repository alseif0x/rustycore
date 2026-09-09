//! Represented skills and their ranks at the Session boundary.
//!
//! Moved out of the Session root under #611. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Publish the canonical `ActivePlayerData::Skill` image after a durable
    /// acquisition commit. The current entity bridge does not yet own these
    /// 256 complex update-field slots, so serialize their complete coherent
    /// image instead of leaving the client on its pre-purchase ranks.
    pub(crate) fn send_complete_player_skill_values_update_like_cpp(&self) {
        use wow_packet::packets::update::{
            ActivePlayerDataValuesUpdate, SkillInfoValuesUpdate, UpdateObject,
        };

        let (Some(guid), Some(skill_store), Some(skill_lines), Some(skill_tiers)) = (
            self.player_guid(),
            self.skill_store(),
            self.skill_line_store(),
            self.skill_tiers_store(),
        ) else {
            return;
        };
        let Some(player_skill_records) = self.resolved_player_skill_records_like_cpp() else {
            return;
        };
        let mut records = player_skill_records.values().collect::<Vec<_>>();
        records.sort_by_key(|record| record.skill_id);
        if records.len() > 256 {
            return;
        }

        let mut skill = SkillInfoValuesUpdate::default();
        let mut set_skill_bit = |bit: usize| {
            skill.skill_info_mask[bit / 32] |= 1 << (bit % 32);
        };
        set_skill_bit(0);
        for index in 0..256 {
            for bit in [
                1 + index,
                257 + index,
                513 + index,
                769 + index,
                1025 + index,
                1281 + index,
                1537 + index,
            ] {
                set_skill_bit(bit);
            }
        }
        for (index, record) in records.into_iter().enumerate() {
            if let Some(entry) = skill_store.loaded_skill_info_like_cpp(
                record.skill_id,
                self.player_race_like_cpp(),
                self.player_class_like_cpp(),
                self.player_level_like_cpp(),
                record.value,
                record.max,
                skill_lines,
                skill_tiers,
            ) {
                skill.skill_line_id[index] = entry.skill_id;
                skill.skill_step[index] = record.step.max(entry.step);
                skill.skill_rank[index] = entry.rank;
                skill.skill_starting_rank[index] = entry.starting_rank;
                skill.skill_max_rank[index] = entry.max_rank;
                skill.skill_temp_bonus[index] = entry.temp_bonus;
                skill.skill_perm_bonus[index] = entry.perm_bonus;
            }
        }

        let mut data = ActivePlayerDataValuesUpdate {
            skill,
            ..Default::default()
        };
        data.active_player_data_mask[0] |= 1;
        data.active_player_data_mask[1] |= 1;
        self.send_packet(&UpdateObject::full_active_player_values_update(
            guid,
            self.player_map_id_like_cpp(),
            data,
        ));
    }
    /// Set the skill store for this session.
    pub fn set_skill_store(&mut self, store: Arc<SkillStore>) {
        self.skill_store = Some(store);
    }
    /// Get the skill store reference.
    pub fn skill_store(&self) -> Option<&Arc<SkillStore>> {
        self.skill_store.as_ref()
    }
    pub fn set_skill_line_store(&mut self, store: Arc<SkillLineStore>) {
        self.skill_line_store = Some(store);
    }
    pub(crate) fn skill_line_store(&self) -> Option<&Arc<SkillLineStore>> {
        self.skill_line_store.as_ref()
    }
    pub fn set_skill_tiers_store(&mut self, store: Arc<SkillTiersStoreLikeCpp>) {
        self.skill_tiers_store = Some(store);
    }
    pub(crate) fn skill_tiers_store(&self) -> Option<&Arc<SkillTiersStoreLikeCpp>> {
        self.skill_tiers_store.as_ref()
    }
    pub fn set_fishing_base_skill_store(&mut self, store: Arc<FishingBaseSkillStoreLikeCpp>) {
        self.fishing_base_skill_store = Some(store);
    }
    pub(crate) fn fishing_base_skill_store(&self) -> Option<&Arc<FishingBaseSkillStoreLikeCpp>> {
        self.fishing_base_skill_store.as_ref()
    }
    pub fn set_max_primary_trade_skills_like_cpp(&mut self, configured: u8) {
        self.max_primary_trade_skills_like_cpp =
            if configured <= crate::profession::MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP {
                configured
            } else {
                crate::profession::DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP
            };
    }
    pub(crate) fn max_primary_trade_skills_like_cpp(&self) -> u8 {
        self.max_primary_trade_skills_like_cpp
    }
    #[allow(dead_code)]
    pub(crate) fn set_player_skill_values_like_cpp(
        &mut self,
        skill_values: HashMap<u16, u16>,
    ) -> bool {
        let skill_records = represented_skill_records_from_values_like_cpp(&skill_values);
        self.replace_player_skill_records_like_cpp(skill_records, true, false)
    }
    pub(crate) fn set_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    ) -> bool {
        // This represented runtime map does not expose the exact occupied
        // ActivePlayerData::Skill slots. Never infer that authority from the
        // number of map rows.
        self.replace_player_skill_records_like_cpp(skill_records, true, false)
    }
    pub(crate) fn set_complete_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        occupied_slots: u16,
    ) -> bool {
        self.replace_player_skill_records_like_cpp(skill_records, true, true)
            && self.set_player_skill_occupied_slots_like_cpp(occupied_slots)
    }
    pub(crate) fn replace_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
    ) -> bool {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_replace_player_skill_records_like_cpp(
                skill_records,
                loaded,
                complete,
            );
        }
        let records = skill_records
            .into_iter()
            .map(|(key, skill)| (key, canonical_player_skill_record_like_cpp(skill)))
            .collect();
        self.with_owned_player_mut_like_cpp(|player| {
            player.replace_represented_skill_records_like_cpp(records, loaded, complete);
        })
        .is_some()
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_replace_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
    ) -> bool {
        let rows_are_structurally_complete = skill_records.iter().all(|(skill_id, skill)| {
            *skill_id == skill.skill_id
                && (skill.state != RepresentedPlayerSkillStateLikeCpp::Deleted
                    || (skill.step == 0
                        && skill.value == 0
                        && skill.max == 0
                        && skill.profession_slot == -1))
        });
        let Some(mut tombstones) = self.resolved_player_skill_non_durable_tombstones_like_cpp()
        else {
            return false;
        };
        tombstones.retain(|skill_id| {
            skill_records
                .get(skill_id)
                .is_some_and(Self::is_non_durable_skill_tombstone_like_cpp)
        });
        tombstones.extend(
            skill_records
                .values()
                .filter(|skill| skill.state == RepresentedPlayerSkillStateLikeCpp::Deleted)
                .map(|skill| skill.skill_id),
        );
        let complete = loaded && complete && rows_are_structurally_complete;
        let canonical_records = skill_records
            .values()
            .copied()
            .map(canonical_player_skill_record_like_cpp)
            .collect();
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_skill_records_like_cpp(
                    canonical_records,
                    loaded,
                    complete,
                    None,
                    tombstones.clone(),
                );
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.player_skill_values_like_cpp =
                represented_skill_values_from_records_like_cpp(&skill_records);
            self.represented_enchanting_skill = skill_records
                .get(&SKILL_ENCHANTING_LIKE_CPP)
                .map(|skill| skill.value)
                .unwrap_or(0);
            self.player_skill_records_like_cpp = skill_records;
            self.player_skill_non_durable_tombstones_like_cpp = tombstones;
            self.player_skill_records_loaded_like_cpp = loaded;
            self.player_skill_records_complete_like_cpp = complete;
            self.player_skill_occupied_slots_like_cpp = None;
            return true;
        }
        _canonical
    }
    #[cfg(test)]
    pub(in crate::session) fn replace_player_skill_runtime_exact_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
        occupied_slots: Option<u16>,
        tombstones: BTreeSet<u16>,
    ) -> bool {
        let canonical_records = skill_records
            .values()
            .copied()
            .map(canonical_player_skill_record_like_cpp)
            .collect();
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_skill_records_like_cpp(
                    canonical_records,
                    loaded,
                    complete,
                    occupied_slots,
                    tombstones.clone(),
                );
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.player_skill_values_like_cpp =
                represented_skill_values_from_records_like_cpp(&skill_records);
            self.represented_enchanting_skill = skill_records
                .get(&SKILL_ENCHANTING_LIKE_CPP)
                .map(|skill| skill.value)
                .unwrap_or(0);
            self.player_skill_records_like_cpp = skill_records;
            self.player_skill_non_durable_tombstones_like_cpp = tombstones;
            self.player_skill_records_loaded_like_cpp = loaded;
            self.player_skill_records_complete_like_cpp = loaded && complete;
            self.player_skill_occupied_slots_like_cpp = occupied_slots;
            return true;
        }
        canonical
    }
    pub(in crate::session) fn clear_player_skill_tombstones_like_cpp(&mut self) {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.fixture_clear_player_skill_tombstones_like_cpp();
            return;
        }
        let _ = self.with_owned_player_mut_like_cpp(
            Player::clear_skill_tombstones_for_identity_change_like_cpp,
        );
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_clear_player_skill_tombstones_like_cpp(&mut self) {
        let Some(records) = self.resolved_player_skill_records_like_cpp() else {
            return;
        };
        let Some(loaded) = self.resolved_player_skill_records_loaded_like_cpp() else {
            return;
        };
        let complete = self.complete_player_skill_records_like_cpp().is_some();
        let occupied = self.complete_player_skill_occupied_slots_like_cpp();
        let _ = self.replace_player_skill_runtime_exact_like_cpp(
            records,
            loaded,
            complete,
            occupied,
            BTreeSet::new(),
        );
    }
    pub(crate) fn set_player_skill_occupied_slots_like_cpp(&mut self, occupied_slots: u16) -> bool {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_set_player_skill_occupied_slots_like_cpp(occupied_slots);
        }
        self.with_owned_player_mut_like_cpp(|player| {
            player.authorize_occupied_skill_slots_like_cpp(occupied_slots)
        })
        .unwrap_or(false)
    }
    // Frozen previous route for differential owner tests and handleless fixtures.
    #[cfg(test)]
    pub(in crate::session) fn fixture_set_player_skill_occupied_slots_like_cpp(
        &mut self,
        occupied_slots: u16,
    ) -> bool {
        // C++ `SetSkill(..., 0)` clears step/rank/max but retains the
        // SkillLineID in its update-field slot until that slot is explicitly
        // reused. A represented SKILL_DELETED row therefore still counts.
        let Some(skill_records) = self.resolved_player_skill_records_like_cpp() else {
            return false;
        };
        let canonical_complete =
            self.with_owned_player_like_cpp(Player::skill_records_complete_like_cpp);
        #[cfg(test)]
        let canonical_complete = canonical_complete.or_else(|| {
            self.player_handle_like_cpp
                .is_none()
                .then_some(self.player_skill_records_complete_like_cpp)
        });
        let complete = canonical_complete.unwrap_or(false);
        let exact = skill_records.len();
        if !complete || usize::from(occupied_slots) != exact || usize::from(occupied_slots) > 256 {
            let _ = self.with_owned_player_mut_like_cpp(|player| {
                let records = player.skill_records_like_cpp().to_vec();
                let loaded = player.skill_records_loaded_like_cpp();
                let complete = player.skill_records_complete_like_cpp();
                let tombstones = player.non_durable_skill_tombstones_like_cpp().clone();
                player.replace_skill_records_like_cpp(records, loaded, complete, None, tombstones);
            });
            #[cfg(test)]
            if self.player_handle_like_cpp.is_none() {
                self.player_skill_occupied_slots_like_cpp = None;
            }
            return false;
        }
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                let records = player.skill_records_like_cpp().to_vec();
                let loaded = player.skill_records_loaded_like_cpp();
                let complete = player.skill_records_complete_like_cpp();
                let tombstones = player.non_durable_skill_tombstones_like_cpp().clone();
                player.replace_skill_records_like_cpp(
                    records,
                    loaded,
                    complete,
                    Some(occupied_slots),
                    tombstones,
                );
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.player_skill_occupied_slots_like_cpp = Some(occupied_slots);
            return true;
        }
        canonical
    }
    pub(crate) fn complete_player_skill_occupied_slots_like_cpp(&self) -> Option<u16> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .skill_records_complete_like_cpp()
                .then(|| player.occupied_skill_slots_like_cpp())
                .flatten()
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self
                .player_skill_records_complete_like_cpp
                .then_some(self.player_skill_occupied_slots_like_cpp)
                .flatten();
        }
        canonical.flatten()
    }
    pub(crate) fn complete_player_skill_records_like_cpp(
        &self,
    ) -> Option<HashMap<u16, RepresentedPlayerSkillLikeCpp>> {
        let records = self.resolved_player_skill_records_like_cpp()?;
        let complete = self.with_owned_player_like_cpp(Player::skill_records_complete_like_cpp);
        #[cfg(test)]
        let complete = complete.or_else(|| {
            self.player_handle_like_cpp
                .is_none()
                .then_some(self.player_skill_records_complete_like_cpp)
        });
        complete.unwrap_or(false).then_some(records)
    }
    pub(in crate::session) fn set_represented_player_skill_like_cpp(
        &mut self,
        skill_id: u16,
        step: u16,
        value: u16,
        max: u16,
    ) {
        let step = if value == 0 { 0 } else { step };
        let Some(mut skill_records) = self.resolved_player_skill_records_like_cpp() else {
            return;
        };
        let previous = skill_records.get(&skill_id).copied();
        let complete_occupied_slots = self.complete_player_skill_occupied_slots_like_cpp();
        // Preserve the existing DB-facing profession association exactly as
        // the former active-only representation did. Persistence still
        // ignores the shadow lifecycle state in this projection-only PR.
        let profession_slot = previous.map(|skill| skill.profession_slot).unwrap_or(-1);
        let state = match previous {
            None => RepresentedPlayerSkillStateLikeCpp::New,
            Some(previous) if value == 0 && previous.value != 0 => {
                if previous.state == RepresentedPlayerSkillStateLikeCpp::New {
                    RepresentedPlayerSkillStateLikeCpp::Unchanged
                } else {
                    RepresentedPlayerSkillStateLikeCpp::Deleted
                }
            }
            Some(previous) if value == 0 => previous.state,
            Some(previous)
                if matches!(
                    previous.state,
                    RepresentedPlayerSkillStateLikeCpp::Unchanged
                        | RepresentedPlayerSkillStateLikeCpp::Deleted
                ) =>
            {
                if previous.value == 0 {
                    if previous.state == RepresentedPlayerSkillStateLikeCpp::Deleted {
                        RepresentedPlayerSkillStateLikeCpp::Changed
                    } else {
                        RepresentedPlayerSkillStateLikeCpp::New
                    }
                } else {
                    RepresentedPlayerSkillStateLikeCpp::Changed
                }
            }
            Some(previous) => previous.state,
        };
        skill_records.insert(
            skill_id,
            RepresentedPlayerSkillLikeCpp {
                skill_id,
                step,
                value,
                max,
                profession_slot,
                state,
            },
        );
        // A mutation of an already-authoritative map preserves exact slot
        // ownership: existing/tombstone rows retain their slot and a genuinely
        // new row consumes one. Incomplete sources remain fail-closed.
        let preserve_complete = complete_occupied_slots.is_some();
        if !self.replace_player_skill_records_like_cpp(skill_records, true, preserve_complete) {
            return;
        }
        if let Some(occupied_slots) = complete_occupied_slots {
            let occupied_slots = occupied_slots.saturating_add(u16::from(previous.is_none()));
            let _ = self.set_player_skill_occupied_slots_like_cpp(occupied_slots);
        }
    }
    pub(in crate::session) fn resolved_player_skill_max_value_like_cpp(
        &self,
        skill_id: u16,
    ) -> Option<u16> {
        Some(
            self.resolved_player_skill_records_like_cpp()?
                .get(&skill_id)
                .map(|skill| skill.max)
                .unwrap_or(0),
        )
    }
    #[cfg(test)]
    pub(in crate::session) fn player_skill_max_value_like_cpp(&self, skill_id: u16) -> u16 {
        self.resolved_player_skill_max_value_like_cpp(skill_id)
            .expect("test Player skill owner must resolve")
    }
    pub(in crate::session) fn max_skill_value_for_level_like_cpp(&self) -> u16 {
        u16::from(self.player_level_like_cpp()).saturating_mul(5)
    }
    pub(crate) fn resolved_player_skill_values_like_cpp(&self) -> Option<HashMap<u16, u16>> {
        Some(represented_skill_values_from_records_like_cpp(
            &self.resolved_player_skill_records_like_cpp()?,
        ))
    }
    pub(crate) fn resolved_player_skill_records_like_cpp(
        &self,
    ) -> Option<HashMap<u16, RepresentedPlayerSkillLikeCpp>> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .skill_records_like_cpp()
                .iter()
                .filter_map(represented_player_skill_record_like_cpp)
                .map(|skill| (skill.skill_id, skill))
                .collect()
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_skill_records_like_cpp.clone());
        }
        canonical
    }
    pub(crate) fn resolved_player_skill_value_like_cpp(&self, skill_id: u16) -> Option<u16> {
        Some(
            self.resolved_player_skill_values_like_cpp()?
                .get(&skill_id)
                .copied()
                .unwrap_or(0),
        )
    }
    #[cfg(test)]
    pub(crate) fn player_skill_values_like_cpp(&self) -> HashMap<u16, u16> {
        self.resolved_player_skill_values_like_cpp()
            .expect("test Player skill owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn player_skill_records_like_cpp(
        &self,
    ) -> HashMap<u16, RepresentedPlayerSkillLikeCpp> {
        self.resolved_player_skill_records_like_cpp()
            .expect("test Player skill owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn player_skill_value_like_cpp(&self, skill_id: u16) -> u16 {
        self.resolved_player_skill_value_like_cpp(skill_id)
            .expect("test Player skill owner must resolve")
    }
    pub(in crate::session) fn represented_fishing_base_skill_level_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<i32> {
        let area_id = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.area_id)?;
        let area_store = self.area_table_store()?;
        let fishing_store = self.fishing_base_skill_store()?;
        Some(fishing_store.base_skill_level_like_cpp(area_store, area_id))
    }
    pub(in crate::session) fn player_profession_skill_value_for_exp_like_cpp(
        &self,
        parent_skill_id: u16,
        expansion: i32,
    ) -> i32 {
        let Some(skill_line_store) = self.skill_line_store() else {
            return 0;
        };
        let resolved_skill_id = skill_line_store
            .profession_skill_for_exp_like_cpp(u32::from(parent_skill_id), expansion);
        if resolved_skill_id == 0 {
            return 0;
        }
        u16::try_from(resolved_skill_id)
            .ok()
            .and_then(|skill_id| self.resolved_player_skill_value_like_cpp(skill_id))
            .map(i32::from)
            .unwrap_or(0)
    }
}

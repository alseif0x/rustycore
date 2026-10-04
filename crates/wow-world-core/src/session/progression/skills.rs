// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical represented skill-state adapters shared with World.

#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::Arc;

use crate::session::{
    state::SessionCore, RepresentedPlayerSkillLikeCpp, RepresentedPlayerSkillStateLikeCpp,
    SKILL_ENCHANTING_LIKE_CPP, canonical_player_skill_record_like_cpp,
    represented_player_skill_record_like_cpp, represented_skill_records_from_values_like_cpp,
    represented_skill_values_from_records_like_cpp,
};
use wow_data::{FishingBaseSkillStoreLikeCpp, SkillLineStore, SkillStore, SkillTiersStoreLikeCpp};
use wow_entities::Player;

mod publication;
#[cfg(any(test, feature = "test-fixtures"))]
mod runtime_install;

impl crate::session::HubMut<'_> {
    #[allow(dead_code)]
    pub fn set_player_skill_values_like_cpp(&mut self, skill_values: HashMap<u16, u16>) -> bool {
        let skill_records = represented_skill_records_from_values_like_cpp(&skill_values);
        self.replace_player_skill_records_like_cpp(skill_records, true, false)
    }

    pub fn set_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    ) -> bool {
        // This represented runtime map does not expose the exact occupied
        // ActivePlayerData::Skill slots. Never infer that authority from the
        // number of map rows.
        self.replace_player_skill_records_like_cpp(skill_records, true, false)
    }

    pub fn set_complete_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        occupied_slots: u16,
    ) -> bool {
        self.replace_player_skill_records_like_cpp(skill_records, true, true)
            && self.set_player_skill_occupied_slots_like_cpp(occupied_slots)
    }

    pub fn replace_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
    ) -> bool {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
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
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_represented_skill_records_like_cpp(records, loaded, complete);
            })
            .is_some()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_replace_player_skill_records_like_cpp(
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
        let Some(mut tombstones) = self
            .shared()
            .resolved_player_skill_non_durable_tombstones_like_cpp()
        else {
            return false;
        };
        tombstones.retain(|skill_id| {
            skill_records
                .get(skill_id)
                .is_some_and(crate::session::is_non_durable_skill_tombstone_like_cpp)
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
            .core
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
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_values_like_cpp =
                represented_skill_values_from_records_like_cpp(&skill_records);
            self.fixtures.progression.represented_enchanting_skill = skill_records
                .get(&SKILL_ENCHANTING_LIKE_CPP)
                .map(|skill| skill.value)
                .unwrap_or(0);
            self.fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp = skill_records;
            self.fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_non_durable_tombstones_like_cpp = tombstones;
            self.fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_loaded_like_cpp = loaded;
            self.fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_complete_like_cpp = complete;
            self.fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_occupied_slots_like_cpp = None;
            return true;
        }
        _canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn replace_player_skill_runtime_exact_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
        occupied_slots: Option<u16>,
        tombstones: BTreeSet<u16>,
    ) -> bool {
        self.core.replace_player_skill_runtime_exact_like_cpp(
            skill_records,
            loaded,
            complete,
            occupied_slots,
            tombstones,
            (
                &mut self
                    .fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp,
                &mut self.fixtures.progression.represented_enchanting_skill,
            ),
        )
    }

    pub fn set_player_skill_occupied_slots_like_cpp(&mut self, occupied_slots: u16) -> bool {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return self.fixture_set_player_skill_occupied_slots_like_cpp(occupied_slots);
        }
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.authorize_occupied_skill_slots_like_cpp(occupied_slots)
            })
            .unwrap_or(false)
    }

    // Frozen previous route for differential owner tests and handleless fixtures.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_set_player_skill_occupied_slots_like_cpp(
        &mut self,
        occupied_slots: u16,
    ) -> bool {
        // C++ `SetSkill(..., 0)` clears step/rank/max but retains the
        // SkillLineID in its update-field slot until that slot is explicitly
        // reused. A represented SKILL_DELETED row therefore still counts.
        let Some(skill_records) = self.shared().resolved_player_skill_records_like_cpp() else {
            return false;
        };
        let canonical_complete = self
            .core
            .with_owned_player_like_cpp(Player::skill_records_complete_like_cpp);
        #[cfg(any(test, feature = "test-fixtures"))]
        let canonical_complete = canonical_complete.or_else(|| {
            self.core.player_handle_like_cpp.is_none().then_some(
                self.fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_records_complete_like_cpp,
            )
        });
        let complete = canonical_complete.unwrap_or(false);
        let exact = skill_records.len();
        if !complete || usize::from(occupied_slots) != exact || usize::from(occupied_slots) > 256 {
            let _ = self.core.with_owned_player_mut_like_cpp(|player| {
                let records = player.skill_records_like_cpp().to_vec();
                let loaded = player.skill_records_loaded_like_cpp();
                let complete = player.skill_records_complete_like_cpp();
                let tombstones = player.non_durable_skill_tombstones_like_cpp().clone();
                player.replace_skill_records_like_cpp(records, loaded, complete, None, tombstones);
            });
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.core.player_handle_like_cpp.is_none() {
                self.fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_occupied_slots_like_cpp = None;
            }
            return false;
        }
        let canonical = self
            .core
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
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_occupied_slots_like_cpp = Some(occupied_slots);
            return true;
        }
        canonical
    }

    pub fn set_represented_player_skill_like_cpp(
        &mut self,
        skill_id: u16,
        step: u16,
        value: u16,
        max: u16,
    ) {
        let step = if value == 0 { 0 } else { step };
        let Some(mut skill_records) = self.shared().resolved_player_skill_records_like_cpp() else {
            return;
        };
        let previous = skill_records.get(&skill_id).copied();
        let complete_occupied_slots = self
            .shared()
            .complete_player_skill_occupied_slots_like_cpp();
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
}

impl crate::session::state::SessionWorldConfig {
    pub fn max_primary_trade_skills_like_cpp(&self) -> u8 {
        self.max_primary_trade_skills_like_cpp
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn fishing_base_skill_store(&self) -> Option<&Arc<FishingBaseSkillStoreLikeCpp>> {
        self.fishing_base_skill_store.as_ref()
    }
}

impl SessionCore {
    pub(crate) fn resolved_player_skill_records_for_publication_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_records: &HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    ) -> Option<HashMap<u16, RepresentedPlayerSkillLikeCpp>> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .skill_records_like_cpp()
                .iter()
                .filter_map(represented_player_skill_record_like_cpp)
                .map(|skill| (skill.skill_id, skill))
                .collect()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(fixture_records.clone());
        }
        canonical
    }

    /// Publish the complete canonical `ActivePlayerData::Skill` values image.
    /// Catalog and fixture inputs stay borrowed for the duration of this call.
    pub fn send_complete_player_skill_values_update_with_inputs_like_cpp(
        &self,
        skill_store: Option<&Arc<SkillStore>>,
        skill_lines: Option<&Arc<SkillLineStore>>,
        skill_tiers: Option<&Arc<SkillTiersStoreLikeCpp>>,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_inputs: (
            &HashMap<u16, RepresentedPlayerSkillLikeCpp>,
            &u8,
            &u8,
            &u8,
        ),
    ) {
        publication::SkillValuesPublicationCxLikeCpp::new(
            self,
            skill_store,
            skill_lines,
            skill_tiers,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_inputs,
        )
        .send_like_cpp();
    }
}

impl crate::session::HubRef<'_> {
    /// Publish the canonical `ActivePlayerData::Skill` image after a durable
    /// acquisition commit. The current entity bridge does not yet own these
    /// 256 complex update-field slots, so serialize their complete coherent
    /// image instead of leaving the client on its pre-purchase ranks.
    pub fn send_complete_player_skill_values_update_like_cpp(&self) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.core.send_complete_player_skill_values_update_with_inputs_like_cpp(
            self.catalogs.skill_store(),
            self.catalogs.skill_line_store(),
            self.catalogs.skill_tiers_store(),
            (
                &self
                    .fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_records_like_cpp,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
                &self.fixtures.identity.player_level,
            ),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        self.core.send_complete_player_skill_values_update_with_inputs_like_cpp(
            self.catalogs.skill_store(),
            self.catalogs.skill_line_store(),
            self.catalogs.skill_tiers_store(),
        );
    }

    pub fn resolved_player_skill_max_value_like_cpp(&self, skill_id: u16) -> Option<u16> {
        Some(
            self.resolved_player_skill_records_like_cpp()?
                .get(&skill_id)
                .map(|skill| skill.max)
                .unwrap_or(0),
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_skill_max_value_like_cpp(&self, skill_id: u16) -> u16 {
        self.resolved_player_skill_max_value_like_cpp(skill_id)
            .expect("test Player skill owner must resolve")
    }

    pub fn max_skill_value_for_level_like_cpp(&self) -> u16 {
        u16::from(self.player_level_like_cpp()).saturating_mul(5)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_skill_records_like_cpp(&self) -> HashMap<u16, RepresentedPlayerSkillLikeCpp> {
        self.resolved_player_skill_records_like_cpp()
            .expect("test Player skill owner must resolve")
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_skill_value_like_cpp(&self, skill_id: u16) -> u16 {
        self.resolved_player_skill_value_like_cpp(skill_id)
            .expect("test Player skill owner must resolve")
    }

    pub fn player_profession_skill_value_for_exp_like_cpp(
        &self,
        parent_skill_id: u16,
        expansion: i32,
    ) -> i32 {
        let Some(skill_line_store) = self.catalogs.skill_line_store() else {
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

impl crate::session::HubRef<'_> {
    pub fn complete_player_skill_occupied_slots_like_cpp(&self) -> Option<u16> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .skill_records_complete_like_cpp()
                .then(|| player.occupied_skill_slots_like_cpp())
                .flatten()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_complete_like_cpp
                .then_some(
                    self.fixtures
                        .progression
                        .player_skill_test_fixture_like_cpp
                        .player_skill_occupied_slots_like_cpp,
                )
                .flatten();
        }
        canonical.flatten()
    }

    pub fn complete_player_skill_records_like_cpp(
        &self,
    ) -> Option<HashMap<u16, RepresentedPlayerSkillLikeCpp>> {
        let records = self.resolved_player_skill_records_like_cpp()?;
        let complete = self
            .core
            .with_owned_player_like_cpp(Player::skill_records_complete_like_cpp);
        #[cfg(any(test, feature = "test-fixtures"))]
        let complete = complete.or_else(|| {
            self.core.player_handle_like_cpp.is_none().then_some(
                self.fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_records_complete_like_cpp,
            )
        });
        complete.unwrap_or(false).then_some(records)
    }

    pub fn resolved_player_skill_values_like_cpp(&self) -> Option<HashMap<u16, u16>> {
        Some(represented_skill_values_from_records_like_cpp(
            &self.resolved_player_skill_records_like_cpp()?,
        ))
    }

    pub fn resolved_player_skill_records_like_cpp(
        &self,
    ) -> Option<HashMap<u16, RepresentedPlayerSkillLikeCpp>> {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core.resolved_player_skill_records_for_publication_like_cpp(
                &self
                    .fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_records_like_cpp,
            )
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core
                .resolved_player_skill_records_for_publication_like_cpp()
        }
    }

    pub fn resolved_player_skill_value_like_cpp(&self, skill_id: u16) -> Option<u16> {
        Some(
            self.resolved_player_skill_values_like_cpp()?
                .get(&skill_id)
                .copied()
                .unwrap_or(0),
        )
    }
}

impl crate::session::state::SessionCatalogs {
    /// Get the skill store reference.
    pub fn skill_store(&self) -> Option<&Arc<SkillStore>> {
        self.skill_store.as_ref()
    }

    pub fn skill_line_store(&self) -> Option<&Arc<SkillLineStore>> {
        self.skill_line_store.as_ref()
    }

    pub fn skill_tiers_store(&self) -> Option<&Arc<SkillTiersStoreLikeCpp>> {
        self.skill_tiers_store.as_ref()
    }
}

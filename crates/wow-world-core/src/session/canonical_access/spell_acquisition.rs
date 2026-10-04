// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::{BTreeSet, HashMap};

use crate::session::{
    RepresentedPlayerSkillLikeCpp, SessionCore, canonical_player_skill_record_like_cpp,
};
use wow_core::ObjectGuid;

/// Borrowed capability for installing a complete spell-acquisition snapshot
/// into the session's current canonical Player.
pub struct OwnedSpellAcquisitionAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build a borrowed capability for complete spell-acquisition installs.
    pub fn owned_spell_acquisition_access_like_cpp(
        &self,
    ) -> OwnedSpellAcquisitionAccessLikeCpp<'_> {
        OwnedSpellAcquisitionAccessLikeCpp { core: self }
    }
}

impl OwnedSpellAcquisitionAccessLikeCpp<'_> {
    pub fn current_map_difficulty_id_like_cpp(&self) -> u8 {
        self.core.current_map_difficulty_id_like_cpp()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    pub fn player_skill_records_loaded_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_loaded: &bool,
    ) -> Option<bool> {
        let canonical = self.core.with_owned_player_like_cpp(
            wow_entities::Player::skill_records_loaded_like_cpp,
        );
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(*fixture_loaded);
        }
        canonical
    }

    /// Read the current session identity at the acquisition snapshot point.
    pub fn player_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }

    /// Project the current spell runtime from the strict owned Player path.
    /// The canonical runtime stays borrowed for the projection and is never
    /// cloned to cross this capability boundary.
    pub fn with_player_spell_runtime_like_cpp<R>(
        &self,
        project: impl FnOnce(&wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        self.core.with_owned_player_like_cpp(|player| {
            project(player.spell_runtime_like_cpp())
        })
    }

    /// Whether the session has no represented Player handle at this instant.
    pub fn player_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    /// Resolve whether this session currently has a canonical Player, using
    /// the same GUID fallback as the existing acquisition mutation path.
    pub fn has_canonical_player_like_cpp(&self) -> bool {
        self.core
            .canonical_player_snapshot_like_cpp(|_| ())
            .is_some()
    }

    /// Grant dual wield to the canonical Player through the existing
    /// GUID-aware mutation path.
    pub fn grant_dual_wield_after_acquisition_like_cpp(&self) -> bool {
        self.core
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().set_can_dual_wield_like_cpp(true);
            })
            .is_some()
    }

    /// Snapshot skill tombstones only from the strictly owned Player handle.
    pub fn skill_non_durable_tombstones_snapshot_like_cpp(&self) -> Option<BTreeSet<u16>> {
        self.core.with_owned_player_like_cpp(|player| {
            player.non_durable_skill_tombstones_like_cpp().clone()
        })
    }

    /// Preserve the handle-less test fallback used by the existing Hub query.
    /// The canonical owner remains the only production source.
    pub fn skill_non_durable_tombstones_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture: &BTreeSet<u16>,
    ) -> Option<BTreeSet<u16>> {
        let canonical = self.skill_non_durable_tombstones_snapshot_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(fixture.clone());
        }
        canonical
    }

    /// Resolve complete slot occupancy from the canonical Player or the
    /// existing handle-less skill fixture, with the same completeness fence
    /// as the session Hub query.
    pub fn complete_player_skill_occupied_slots_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_complete: &bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_occupied: &Option<u16>,
    ) -> Option<u16> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .skill_records_complete_like_cpp()
                .then(|| player.occupied_skill_slots_like_cpp())
                .flatten()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return fixture_complete.then_some(*fixture_occupied).flatten();
        }
        canonical.flatten()
    }

    /// Validate a complete acquisition snapshot before invalidating the
    /// existing spell-hit authority or mutating the canonical Player.
    pub fn install_complete_spell_acquisition_like_cpp(
        &self,
        spell_rows: impl IntoIterator<Item = wow_entities::PlayerKnownSpellRecord>,
        traits: impl IntoIterator<Item = (i32, i32)>,
        overrides: impl IntoIterator<Item = (i32, i32)>,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        occupied_skill_slots: u16,
        non_durable_skill_tombstones: BTreeSet<u16>,
    ) -> bool {
        let Some(prepared) = wow_entities::PreparedPlayerSpellAcquisitionLikeCpp::try_new(
            spell_rows,
            traits,
            overrides,
            skill_records
                .into_iter()
                .map(|(key, skill)| (key, canonical_player_skill_record_like_cpp(skill)))
                .collect(),
            occupied_skill_slots,
            non_durable_skill_tombstones,
        ) else {
            return false;
        };

        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.apply_prepared_spell_acquisition_like_cpp(prepared)
            })
            .is_some()
    }
}

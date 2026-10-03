// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::{BTreeSet, HashMap};

use crate::session::{
    RepresentedPlayerSkillLikeCpp, SessionCore, canonical_player_skill_record_like_cpp,
};

/// Borrowed capability for installing a complete spell-acquisition snapshot
/// into the session's current canonical Player.
pub struct OwnedSpellAcquisitionAccessLikeCpp<'a> {
    core: &'a mut SessionCore,
}

impl SessionCore {
    /// Build a borrowed capability for complete spell-acquisition installs.
    pub fn owned_spell_acquisition_access_like_cpp(
        &mut self,
    ) -> OwnedSpellAcquisitionAccessLikeCpp<'_> {
        OwnedSpellAcquisitionAccessLikeCpp { core: self }
    }
}

impl OwnedSpellAcquisitionAccessLikeCpp<'_> {
    /// Validate a complete acquisition snapshot before invalidating the
    /// existing spell-hit authority or mutating the canonical Player.
    pub fn install_complete_spell_acquisition_like_cpp(
        &mut self,
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

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn spell_group_spell_map_bounds_like_cpp(&self, group_id: u32) -> &[i32] {
        self.catalogs
            .spell_group_spell_map_bounds_like_cpp(group_id)
    }
    pub(crate) fn is_spell_member_of_spell_group_like_cpp(
        &self,
        spell_id: u32,
        group_id: u32,
    ) -> bool {
        self.catalogs
            .is_spell_member_of_spell_group_like_cpp(spell_id, group_id)
    }
    pub(crate) fn set_of_spells_in_spell_group_like_cpp(&self, group_id: u32) -> BTreeSet<u32> {
        self.catalogs
            .set_of_spells_in_spell_group_like_cpp(group_id)
    }
    pub(crate) fn spell_group_stack_rule_like_cpp(
        &self,
        group_id: u32,
    ) -> SpellGroupStackRuleLikeCpp {
        self.catalogs.spell_group_stack_rule_like_cpp(group_id)
    }
    pub(crate) fn check_spell_group_stack_rules_like_cpp(
        &self,
        first_rank_spell_id_1: u32,
        first_rank_spell_id_2: u32,
    ) -> SpellGroupStackRuleLikeCpp {
        self.catalogs
            .check_spell_group_stack_rules_like_cpp(first_rank_spell_id_1, first_rank_spell_id_2)
    }
    pub(crate) fn pet_aura_like_cpp(
        &self,
        spell_id: u32,
        effect_index: u8,
    ) -> Option<&PetAuraLikeCpp> {
        self.catalogs.pet_aura_like_cpp(spell_id, effect_index)
    }
    #[cfg(test)]
    pub(crate) fn pet_levelup_spell_list_like_cpp(
        &self,
        pet_family: u32,
    ) -> Option<&PetLevelupSpellSetLikeCpp> {
        self.catalogs.pet_levelup_spell_list_like_cpp(pet_family)
    }
    #[cfg(test)]
    pub(crate) fn pet_default_spells_entry_like_cpp(
        &self,
        id: i32,
    ) -> Option<&PetDefaultSpellsEntryLikeCpp> {
        self.catalogs.pet_default_spells_entry_like_cpp(id)
    }
    #[cfg(test)]
    pub(crate) fn pet_family_spells_like_cpp(&self, pet_family: u32) -> Option<Vec<u32>> {
        self.catalogs.pet_family_spells_like_cpp(pet_family)
    }
    #[cfg(test)]
    pub(crate) fn model_for_totem_like_cpp(&self, spell_id: u32, race_id: u8) -> u32 {
        self.catalogs.model_for_totem_like_cpp(spell_id, race_id)
    }
    pub(crate) fn spell_spell_group_map_bounds_like_cpp(&self, spell_id: u32) -> &[u32] {
        self.catalogs
            .spell_spell_group_map_bounds_like_cpp(spell_id)
    }
}

// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell pet catalogs: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, BTreeSet, PetAuraLikeCpp, ScriptIdLikeCpp, ScriptNameInternerLikeCpp};
#[cfg(test)]
use super::{PetDefaultSpellsEntryLikeCpp, PetLevelupSpellSetLikeCpp};
use super::{SpellGroupStackRuleLikeCpp, WorldSession};

impl WorldSession {
    pub(crate) fn spell_spell_group_map_bounds_like_cpp(&self, spell_id: u32) -> &[u32] {
        self.spell_catalogs
            .spell_group_store
            .as_ref()
            .map(|store| {
                store.spell_spell_group_map_bounds_like_cpp(spell_id, |lookup_spell_id| {
                    self.spell_catalogs
                        .spell_chain_store
                        .as_ref()
                        .map(|spell_chains| {
                            spell_chains.first_spell_in_chain_like_cpp(lookup_spell_id)
                        })
                        .unwrap_or(lookup_spell_id)
                })
            })
            .unwrap_or(&[])
    }

    pub(crate) fn spell_group_spell_map_bounds_like_cpp(&self, group_id: u32) -> &[i32] {
        self.spell_catalogs
            .spell_group_store
            .as_ref()
            .map(|store| store.spell_group_spell_map_bounds_like_cpp(group_id))
            .unwrap_or(&[])
    }

    pub(crate) fn is_spell_member_of_spell_group_like_cpp(
        &self,
        spell_id: u32,
        group_id: u32,
    ) -> bool {
        self.spell_catalogs
            .spell_group_store
            .as_ref()
            .map(|store| {
                store.is_spell_member_of_spell_group_like_cpp(
                    spell_id,
                    group_id,
                    |lookup_spell_id| {
                        self.spell_catalogs
                            .spell_chain_store
                            .as_ref()
                            .map(|spell_chains| {
                                spell_chains.first_spell_in_chain_like_cpp(lookup_spell_id)
                            })
                            .unwrap_or(lookup_spell_id)
                    },
                )
            })
            .unwrap_or(false)
    }

    pub(crate) fn set_of_spells_in_spell_group_like_cpp(&self, group_id: u32) -> BTreeSet<u32> {
        self.spell_catalogs
            .spell_group_store
            .as_ref()
            .map(|store| store.set_of_spells_in_spell_group_like_cpp(group_id))
            .unwrap_or_default()
    }

    pub(crate) fn spell_group_stack_rule_like_cpp(
        &self,
        group_id: u32,
    ) -> SpellGroupStackRuleLikeCpp {
        self.spell_catalogs
            .spell_group_stack_rule_store
            .as_ref()
            .map(|store| store.spell_group_stack_rule_like_cpp(group_id))
            .unwrap_or(SpellGroupStackRuleLikeCpp::Default)
    }

    pub(crate) fn check_spell_group_stack_rules_like_cpp(
        &self,
        first_rank_spell_id_1: u32,
        first_rank_spell_id_2: u32,
    ) -> SpellGroupStackRuleLikeCpp {
        let Some(stack_rules) = self.spell_catalogs.spell_group_stack_rule_store.as_ref() else {
            return SpellGroupStackRuleLikeCpp::Default;
        };
        let Some(spell_groups) = self.spell_catalogs.spell_group_store.as_ref() else {
            return SpellGroupStackRuleLikeCpp::Default;
        };
        stack_rules.check_spell_group_stack_rules_like_cpp(
            spell_groups,
            first_rank_spell_id_1,
            first_rank_spell_id_2,
        )
    }

    pub(crate) fn pet_aura_like_cpp(
        &self,
        spell_id: u32,
        effect_index: u8,
    ) -> Option<&PetAuraLikeCpp> {
        self.spell_catalogs
            .spell_pet_aura_store
            .as_ref()
            .and_then(|store| store.get_pet_aura_like_cpp(spell_id, effect_index))
    }

    #[cfg(test)]
    pub(crate) fn pet_levelup_spell_list_like_cpp(
        &self,
        pet_family: u32,
    ) -> Option<&PetLevelupSpellSetLikeCpp> {
        self.spell_catalogs
            .pet_levelup_spell_store
            .as_ref()
            .and_then(|store| store.get_pet_levelup_spell_list_like_cpp(pet_family))
    }

    #[cfg(test)]
    pub(crate) fn pet_default_spells_entry_like_cpp(
        &self,
        id: i32,
    ) -> Option<&PetDefaultSpellsEntryLikeCpp> {
        self.spell_catalogs
            .pet_default_spell_store
            .as_ref()
            .and_then(|store| store.get_pet_default_spells_entry_like_cpp(id))
    }

    #[cfg(test)]
    pub(crate) fn pet_family_spells_like_cpp(&self, pet_family: u32) -> Option<Vec<u32>> {
        self.spell_catalogs
            .pet_family_spell_store
            .as_ref()
            .and_then(|store| store.get_pet_family_spells_like_cpp(pet_family))
    }

    #[cfg(test)]
    pub(crate) fn model_for_totem_like_cpp(&self, spell_id: u32, race_id: u8) -> u32 {
        self.spell_catalogs
            .spell_totem_model_store
            .as_ref()
            .map(|store| store.get_model_for_totem_like_cpp(spell_id, race_id))
            .unwrap_or(0)
    }

    pub fn set_script_name_interner(&mut self, store: Arc<ScriptNameInternerLikeCpp>) {
        self.script_name_interner = Some(store);
    }

    #[allow(dead_code)]
    pub(crate) fn script_name_like_cpp(&self, id: ScriptIdLikeCpp) -> &str {
        self.script_name_interner
            .as_ref()
            .map(|store| store.get_script_name_like_cpp(id))
            .unwrap_or("")
    }

    #[allow(dead_code)]
    pub(crate) fn script_id_bound_in_database_like_cpp(&self, id: ScriptIdLikeCpp) -> bool {
        self.script_name_interner
            .as_ref()
            .is_some_and(|store| store.is_script_database_bound_like_cpp(id))
    }

    pub(crate) fn faction_template_for_race_like_cpp(&self, race: u8) -> Option<i32> {
        self.chr
            .races_store
            .as_ref()?
            .get(u32::from(race))
            .map(|entry| i32::from(entry.faction_id))
    }
}

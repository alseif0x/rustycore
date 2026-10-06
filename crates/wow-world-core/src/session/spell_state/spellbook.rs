use crate::session::state::SessionCatalogs;
use std::collections::HashSet;
use wow_data::{SpellLearnSkillLookupLikeCpp, SpellLearnSpellNodeLikeCpp};

impl SessionCatalogs {
    pub fn spell_learn_skill_lookup_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellLearnSkillLookupLikeCpp<'_> {
        self.spell_catalogs
            .spell_learn_skill_store
            .as_ref()
            .map(|store| store.spell_learn_skill_lookup_like_cpp(spell_id))
            .unwrap_or(SpellLearnSkillLookupLikeCpp::MissingCoverage)
    }

    pub fn spell_learn_spell_map_bounds_like_cpp(
        &self,
        spell_id: u32,
    ) -> &[SpellLearnSpellNodeLikeCpp] {
        self.spell_catalogs
            .spell_learn_spell_store
            .as_ref()
            .map(|store| store.get_spell_learn_spell_map_bounds_like_cpp(spell_id))
            .unwrap_or(&[])
    }

    pub fn deactivate_lower_rank_known_spells_for_send_like_cpp(
        &self,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        let Some(spell_chains) = self.spell_catalogs.spell_chain_store() else {
            return 0;
        };

        let known_set: HashSet<i32> = known_spells.iter().copied().collect();
        let before = known_spells.len();
        known_spells.retain(|spell_id| {
            let Ok(mut next_spell_id) = u32::try_from(*spell_id) else {
                return true;
            };

            loop {
                next_spell_id = spell_chains.next_spell_in_chain_like_cpp(next_spell_id);
                if next_spell_id == 0 {
                    return true;
                }

                if let Ok(next_spell_i32) = i32::try_from(next_spell_id)
                    && known_set.contains(&next_spell_i32)
                {
                    return false;
                }
            }
        });

        before - known_spells.len()
    }
}

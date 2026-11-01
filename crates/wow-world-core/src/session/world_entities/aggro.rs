use crate::session::state::SessionCore;
use wow_data::SpellThreatEntryLikeCpp;

const DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP: f32 = 1.5;

impl SessionCore {
    pub fn canonical_player_combat_reach_snapshot_like_cpp(&self) -> f32 {
        self.canonical_player_snapshot_like_cpp(|player| player.unit().data().combat_reach)
            .unwrap_or(0.0)
    }

    pub fn player_interaction_combat_reach_like_cpp(&self) -> f32 {
        let canonical_reach = self.canonical_player_combat_reach_snapshot_like_cpp();
        if canonical_reach > 0.0 {
            canonical_reach
        } else {
            DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP
        }
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn spell_threat_entry_like_cpp(&self, spell_id: u32) -> Option<&SpellThreatEntryLikeCpp> {
        let store = self.spell_catalogs.spell_threat_store.as_ref()?;
        store.get_spell_threat_entry_like_cpp(spell_id, |lookup_spell_id| {
            self.spell_catalogs
                .spell_chain_store
                .as_ref()
                .map(|spell_chains| spell_chains.first_spell_in_chain_like_cpp(lookup_spell_id))
                .unwrap_or(lookup_spell_id)
        })
    }
}

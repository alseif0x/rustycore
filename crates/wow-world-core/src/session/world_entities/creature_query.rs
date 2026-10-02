use std::sync::Arc;

use wow_data::reputation::CreatureOnKillReputationStoreLikeCpp;
use wow_data::CreatureTemplateLifecycleStoreLikeCpp;

impl crate::session::state::SessionCatalogs {
    pub fn creature_faction_template_is_neutral_to_all_like_cpp(
        &self,
        faction_template_id: u32,
    ) -> bool {
        let Some(faction_template_store) = self.factions.template_store.as_ref() else {
            // Transitional compatibility for legacy DB-only creature loading.
            // C++ uses FactionTemplate.db2; when the store is absent, keep the
            // previous Rust no-aggro behavior for the canonical neutral faction.
            return faction_template_id == 35;
        };
        let Some(faction_template) = faction_template_store.get(faction_template_id) else {
            return false;
        };

        if faction_template.faction == 0 {
            return true;
        }

        if let Some(faction_store) = self.factions.store.as_ref()
            && let Some(raw_faction) = faction_store.get(u32::from(faction_template.faction))
            && raw_faction.can_have_reputation_like_cpp()
        {
            return false;
        }

        faction_template.is_neutral_to_all_like_cpp()
    }

    pub fn creature_onkill_reputation_store(
        &self,
    ) -> Option<&Arc<CreatureOnKillReputationStoreLikeCpp>> {
        self.creatures.onkill_reputation_store.as_ref()
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn creature_template_lifecycle_store_like_cpp(
        &self,
    ) -> Option<&Arc<CreatureTemplateLifecycleStoreLikeCpp>> {
        self.creatures.template_lifecycle_store_like_cpp.as_ref()
    }
}

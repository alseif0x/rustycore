//! The creature aggro configuration and spell-threat catalog setters
//! (world-server session construction API). The spell initial threat and the
//! threat-relevant auras moved to `wow_world_spell` (#1363 slice 10b).

use super::*;

impl WorldSession {
    pub fn set_creature_aggro_config_like_cpp(&mut self, config: CreatureAggroConfigLikeCpp) {
        self.config.creature_aggro_config_like_cpp = config;
    }
    pub fn set_spell_threat_store(&mut self, store: Arc<SpellThreatStoreLikeCpp>) {
        self.catalogs.spell_catalogs.spell_threat_store = Some(store);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/aggro/f3_shims.rs"]
mod f3_shims;

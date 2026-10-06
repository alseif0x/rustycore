use std::collections::HashMap;

use wow_entities::AuraApplicationLikeCpp as AuraApplication;

impl crate::session::HubRef<'_> {
    pub fn resolved_player_visible_auras_like_cpp(&self) -> Option<HashMap<u8, AuraApplication>> {
        self.player_aura_subsystem_snapshot_like_cpp()
            .map(|auras| auras.runtime_applications_like_cpp().clone())
    }

    pub fn player_has_visible_aura_spell_like_cpp(&self, spell_id: i32) -> Option<bool> {
        self.player_aura_subsystem_snapshot_like_cpp().map(|auras| {
            auras
                .runtime_applications_like_cpp()
                .values()
                .any(|aura| aura.spell_id == spell_id)
        })
    }
}

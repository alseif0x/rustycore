use crate::session::state::SessionCore;

impl SessionCore {
    pub fn invalidate_canonical_player_spell_hit_aura_authority_like_cpp(&self) {
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .invalidate_spell_hit_aura_authority_like_cpp();
        });
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_aura_subsystem_snapshot_like_cpp(&self) -> Option<wow_entities::AuraSubsystem> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().subsystems().auras.clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let mut auras = wow_entities::AuraSubsystem::default();
            auras.set_persisted_player_aura_authority_complete_like_cpp(
                self.fixtures.auras.player_aura_authority_complete_like_cpp,
            );
            if self
                .fixtures
                .auras
                .player_spell_hit_aura_authority_tombstoned_like_cpp
            {
                auras.tombstone_spell_hit_aura_authority_like_cpp();
            }
            for aura in self.fixtures.auras.visible_auras.values().cloned() {
                auras.insert_runtime_application_like_cpp(aura);
            }
            for (&slot, snapshot) in &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp {
                auras.insert_threat_snapshot_like_cpp(slot, snapshot.clone());
            }
            return Some(auras);
        }
        canonical
    }
}

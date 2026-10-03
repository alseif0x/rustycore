use crate::SessionSpellState;
use wow_world_core::session::{HubMut, HubRef};

impl SessionSpellState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_player_aura_subsystem_like_cpp<R>(
        &mut self,
        hub: &mut HubMut<'_>,
        mutate: impl FnOnce(&mut wow_entities::AuraSubsystem) -> R,
    ) -> Option<R> {
        let mut mutate = Some(mutate);
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            let mut auras = hub.shared().player_aura_subsystem_snapshot_like_cpp()?;
            let result =
                mutate
                    .take()
                    .expect("test Player aura mutation executes once")(&mut auras);
            hub.fixtures.auras.player_aura_authority_complete_like_cpp =
                auras.persisted_player_aura_authority_complete_like_cpp();
            hub.fixtures
                .auras
                .player_spell_hit_aura_authority_tombstoned_like_cpp =
                auras.spell_hit_aura_authority_tombstoned_like_cpp();
            hub.fixtures.auras.visible_auras = auras.runtime_applications_like_cpp().clone();
            hub.fixtures
                .auras
                .canonical_threat_aura_snapshots_like_cpp
                .clear();
            for slot in 0..=u8::MAX {
                if let Some(snapshot) = auras.threat_snapshot_like_cpp(slot) {
                    hub.fixtures
                        .auras
                        .canonical_threat_aura_snapshots_like_cpp
                        .insert(slot, snapshot.clone());
                }
            }
            return Some(result);
        }
        hub.core.with_owned_player_mut_like_cpp(|player| {
            mutate.take().expect("Player aura mutation executes once")(
                &mut player.unit_mut().subsystems_mut().auras,
            )
        })
    }

    pub fn set_player_aura_authority_complete_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        complete: bool,
    ) -> bool {
        let _canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_player_aura_authority_complete_like_cpp(complete);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !_canonical && hub.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_aura_subsystem_like_cpp(hub, |auras| {
                    auras.set_persisted_player_aura_authority_complete_like_cpp(complete);
                })
                .is_some();
        }
        _canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_aura_authority_complete_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        hub.fixtures.auras.player_aura_authority_complete_like_cpp
    }

    pub fn resolved_player_aura_authority_complete_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<bool> {
        hub.player_aura_subsystem_snapshot_like_cpp()
            .map(|auras| auras.persisted_player_aura_authority_complete_like_cpp())
    }

    pub fn tombstone_player_spell_hit_aura_authority_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) {
        let _canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.tombstone_player_spell_hit_aura_authority_like_cpp();
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !_canonical && hub.core.player_handle_like_cpp.is_none() {
            let _ = self.mutate_player_aura_subsystem_like_cpp(hub, |auras| {
                auras.tombstone_spell_hit_aura_authority_like_cpp();
            });
        }
    }

    pub fn represented_active_glyph_aura_source_is_empty_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        hub.player_talent_runtime_snapshot_like_cpp()
            .filter(|runtime| runtime.glyphs_loaded_like_cpp())
            .map(|runtime| {
                let active_group = runtime.active_group_like_cpp();
                (0..wow_entities::PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP as u8)
                    .all(|slot| runtime.glyph_like_cpp(active_group, slot) == Some(0))
            })
            .unwrap_or(false)
    }

    /// C++ `Map::AddPlayerToMap` can dispatch `InstanceScript::OnPlayerEnter`,
    /// Scenario, and Battleground hooks before the login authority is
    /// published. Those hooks are not represented, so only an exact ordinary
    /// world-map DB2 row excludes them.
    pub fn represented_add_player_to_map_aura_source_is_empty_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        hub.catalogs
            .maps
            .store
            .as_ref()
            .and_then(|store| store.get(u32::from(hub.core.player_map_id_like_cpp())))
            .is_some_and(|map| map.instance_type == wow_data::map::MAP_COMMON)
    }
}

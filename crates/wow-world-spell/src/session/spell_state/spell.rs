use crate::records::RepresentedSpellFocusObjectLikeCpp;
use wow_world_core::session::spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp;
impl crate::SessionSpellState {
    pub fn search_spell_focus_like_cpp(
        &self,
        hub: wow_world_core::session::HubRef<'_>,
        focus_id: u32,
    ) -> Option<RepresentedSpellFocusObjectLikeCpp> {
        if focus_id == 0 {
            return None;
        }
        let caster_position = hub.player_position_like_cpp()?;
        let player_map_key = hub.core.current_canonical_player_map_key_like_cpp()?;
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let managed = manager.find_map(player_map_key.map_id, player_map_key.instance_id)?;
        let map = managed.map();
        let nearby = map.nearby_cell_guids_like_cpp(
            caster_position.x,
            caster_position.y,
            map.visibility_range(),
        );
        for guid in nearby.grid.gameobjects {
            let Some(gameobject) = map.get_typed_game_object(guid) else {
                continue;
            };
            let world = gameobject.world();
            if !world.object().is_in_world() {
                continue;
            }
            let Some(source) = gameobject.represented_spell_focus_use_source_like_cpp() else {
                continue;
            };
            if source.focus_type != focus_id {
                continue;
            }
            if !world
                .position()
                .is_within_dist(&caster_position, source.radius as f32)
            {
                continue;
            }
            return Some(RepresentedSpellFocusObjectLikeCpp {
                guid,
                map_key: player_map_key,
                position: world.position(),
                source,
            });
        }
        None
    }
    pub fn reset_represented_character_spell_charges_like_cpp(
        &mut self,
        hub: &mut wow_world_core::session::HubMut<'_>,
    ) {
        let _ = self.mutate_player_spell_history_like_cpp(hub, |history| {
            history.charges.clear();
            history.charges_loaded = false;
        });
    }
    pub fn mark_represented_character_spell_charges_loaded_like_cpp(
        &mut self,
        hub: &mut wow_world_core::session::HubMut<'_>,
    ) {
        let _ = self.mutate_player_spell_history_like_cpp(hub, |history| {
            history.charges_loaded = true;
        });
    }
    pub fn record_loaded_character_spell_charge_like_cpp(
        &mut self,
        hub: &mut wow_world_core::session::HubMut<'_>,
        category_id: u32,
        recharge_start_unix_secs: i64,
        recharge_end_unix_secs: i64,
    ) {
        let _ = self.mutate_player_spell_history_like_cpp(hub, |history| {
            history.charges.entry(category_id).or_default().push_back(
                wow_entities::SpellChargeState {
                    recharge_start_ms: u64::try_from(recharge_start_unix_secs)
                        .unwrap_or(0)
                        .saturating_mul(1_000),
                    recharge_end_ms: u64::try_from(recharge_end_unix_secs)
                        .unwrap_or(0)
                        .saturating_mul(1_000),
                },
            );
        });
    }
    /// Prove that applying/casting one spell cannot enter an unrepresented
    /// C++ spell script, legacy spell script, or linked-spell hook.
    ///
    /// Rank indeterminacy is not treated as absence: a negative
    /// `spell_script_names` binding can cover the whole C++ chain.
    pub fn spell_has_no_unrepresented_runtime_hooks_like_cpp(
        &self,
        hub: wow_world_core::session::HubRef<'_>,
        spell_id: u32,
    ) -> bool {
        spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp(
            spell_id,
            self.spell_script_exact_spell_ids_like_cpp.as_deref(),
            self.spell_script_all_rank_root_spell_ids_like_cpp
                .as_deref(),
            self.legacy_spell_script_spell_ids_like_cpp.as_deref(),
            self.spell_linked_rejected_trigger_spell_ids_like_cpp
                .as_deref(),
            hub.catalogs.spell_catalogs.spell_chain_store.as_deref(),
            hub.catalogs.spell_catalogs.spell_linked_store.as_deref(),
        )
    }
    /// Prove that every effective effect and every world-table hook for one
    /// source spell is inert for the bounded rear physical/melee hit profile.
    pub fn player_target_spell_is_hit_inert_like_cpp(
        &self,
        hub: wow_world_core::session::HubRef<'_>,
        spell_id: u32,
        difficulty_id: u8,
    ) -> bool {
        let Ok(spell_id_i32) = i32::try_from(spell_id) else {
            return false;
        };
        if !self.spell_has_no_unrepresented_runtime_hooks_like_cpp(hub, spell_id) {
            return false;
        }
        let Some(spell_store) = hub.catalogs.spell_catalogs.spell_store.as_ref() else {
            return false;
        };
        if spell_store.get(spell_id_i32).is_none() {
            return false;
        }
        spell_store
            .effects_for_difficulty_like_cpp(
                spell_id_i32,
                difficulty_id,
                hub.catalogs.difficulty_store.as_deref(),
            )
            .is_some_and(|effects| {
                effects
                    .iter()
                    .all(wow_data::player_target_spell_effect_is_hit_inert_like_cpp)
            })
    }
    pub fn represented_talent_spell_id_like_cpp(
        &self,
        hub: wow_world_core::session::HubRef<'_>,
        talent_id: u32,
        rank: u8,
    ) -> Option<i32> {
        hub.catalogs
            .talent_store()?
            .get(talent_id)?
            .spell_rank
            .get(usize::from(rank))
            .copied()
            .filter(|spell_id| *spell_id > 0)
    }
    pub fn represented_talent_override_spell_pair_like_cpp(
        &self,
        hub: wow_world_core::session::HubRef<'_>,
        talent_id: u32,
    ) -> Option<(i32, i32)> {
        let talent = hub.catalogs.talent_store()?.get(talent_id)?;
        (talent.overrides_spell_id > 0 && talent.spell_id > 0)
            .then_some((talent.overrides_spell_id, talent.spell_id))
    }
    pub fn cleanup_removed_spell_dual_wield_like_cpp(
        &mut self,
        hub: &mut wow_world_core::session::HubMut<'_>,
        spell_id: i32,
    ) {
        let Some(spell_store) = hub.catalogs.spell_store() else {
            return;
        };
        let Some(spell_info) = spell_store.get(spell_id) else {
            return;
        };
        if !spell_store.is_passive_like_cpp(spell_id)
            || !spell_info
                .has_effect_like_cpp(wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD)
        {
            return;
        }
        let _ = hub.core.mutate_canonical_player_like_cpp(|player| {
            if player.unit().can_dual_wield_like_cpp() {
                player.unit_mut().set_can_dual_wield_like_cpp(false);
            }
        });
    }
    pub fn remove_represented_self_res_spell_like_cpp(
        &mut self,
        hub: &mut wow_world_core::session::HubMut<'_>,
        spell_id: i32,
    ) -> bool {
        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            player
                .resurrection_state_mut_like_cpp()
                .self_res_spells
                .remove(&spell_id)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return self.represented_self_res_spells_like_cpp.remove(&spell_id);
        }
        canonical.unwrap_or(false)
    }
}

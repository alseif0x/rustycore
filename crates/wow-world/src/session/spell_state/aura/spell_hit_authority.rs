//! Canonical Player spell-hit aura-source authority and synchronization.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn mutate_player_aura_subsystem_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::AuraSubsystem) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.mutate_player_aura_subsystem_like_cpp(&mut hub, mutate)
    }

    pub(crate) fn set_player_aura_authority_complete_like_cpp(&mut self, complete: bool) -> bool {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.set_player_aura_authority_complete_like_cpp(&mut hub, complete)
    }

    pub(crate) fn tombstone_player_spell_hit_aura_authority_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.tombstone_player_spell_hit_aura_authority_like_cpp(&mut hub)
    }

    fn represented_active_glyph_aura_source_is_empty_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_active_glyph_aura_source_is_empty_like_cpp(hub)
    }

    /// C++ `_LoadTraits` creates missing configs for specialization indexes
    /// `0..MAX_SPECIALIZATIONS - 1`, and `CreateTraitConfig` can attach granted
    /// entries. This narrow proof is therefore limited to a complete set of
    /// persisted, active-for-spec combat configs whose global entry query was
    /// empty. `ValidateConfig` accepts those empty configs, so C++ neither
    /// replaces them with granted entries nor creates a missing config.
    fn represented_trait_config_aura_source_is_empty_like_cpp(&self) -> bool {
        let Some(runtime) = self.player_spell_runtime_snapshot_like_cpp() else {
            return false;
        };
        if !runtime.trait_config_rows_complete
            || !runtime.trait_entry_rows_complete
            || !runtime.trait_entry_rows_empty
        {
            return false;
        }

        let Some(specializations) = self.catalogs.chr.specialization_store.as_ref() else {
            return false;
        };
        let mut expected_specs = BTreeSet::new();
        for index in 0..(MAX_SPECIALIZATIONS_LIKE_CPP - 1) {
            let Some(spec) = u8::try_from(index).ok().and_then(|index| {
                specializations.get_by_class_and_index_like_cpp(
                    crate::session::hub_ref(self).player_class_like_cpp(),
                    index,
                )
            }) else {
                return false;
            };
            let Ok(spec_id) = i32::try_from(spec.id) else {
                return false;
            };
            if !expected_specs.insert(spec_id) {
                return false;
            }
        }

        if runtime.trait_config_rows.len() != expected_specs.len() {
            return false;
        }
        for config in runtime.trait_config_rows.values() {
            let (config_type, specialization_id, combat_flags) = config.header;
            const TRAIT_CONFIG_TYPE_COMBAT_LIKE_CPP: i32 = 1;
            const TRAIT_COMBAT_CONFIG_ACTIVE_FOR_SPEC_LIKE_CPP: i32 = 0x1;
            if config_type != TRAIT_CONFIG_TYPE_COMBAT_LIKE_CPP
                || combat_flags & TRAIT_COMBAT_CONFIG_ACTIVE_FOR_SPEC_LIKE_CPP == 0
                || !expected_specs.remove(&specialization_id)
            {
                return false;
            }
        }
        expected_specs.is_empty()
    }

    fn represented_add_player_to_map_aura_source_is_empty_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_add_player_to_map_aura_source_is_empty_like_cpp(hub)
    }

    pub(in crate::session) fn can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(
        &self,
        difficulty_id: u8,
    ) -> bool {
        let Some(aura_subsystem) =
            crate::session::hub_ref(self).player_aura_subsystem_snapshot_like_cpp()
        else {
            return false;
        };
        if !self.player_spell_hit_source_identity_complete_like_cpp()
            || aura_subsystem.spell_hit_aura_authority_tombstoned_like_cpp()
            || !aura_subsystem.persisted_player_aura_authority_complete_like_cpp()
            || !self.player_equipment_inventory_authority_complete_like_cpp()
            || self
                .resolved_represented_guild_id_like_cpp()
                .is_none_or(|guild_id| guild_id != 0)
            || crate::session::hub_ref(self)
                .complete_player_skill_records_like_cpp()
                .is_none()
            || self
                .complete_represented_player_spell_rows_like_cpp()
                .is_none()
            || self
                .complete_represented_spell_trait_definition_ids_like_cpp()
                .is_none_or(|traits| !traits.is_empty())
            || !self.represented_trait_config_aura_source_is_empty_like_cpp()
            || !self.represented_active_glyph_aura_source_is_empty_like_cpp()
            || !self.represented_battle_pet_login_spell_source_is_empty_like_cpp()
            || !self.represented_add_player_to_map_aura_source_is_empty_like_cpp()
            || !self
                .catalogs
                .represented_auto_push_quest_aura_source_is_empty_like_cpp()
            || !crate::session::hub_ref(self)
                .represented_character_pet_aura_source_is_empty_like_cpp()
            || !self.represented_quest_login_aura_sources_are_hit_inert_like_cpp(difficulty_id)
            || !self.represented_spell_area_autocast_source_is_empty_like_cpp()
            || !self.represented_war_mode_update_zone_aura_source_is_empty_like_cpp()
            || !self.represented_update_area_pvp_rule_aura_source_is_empty_like_cpp()
            || !self.represented_update_zone_script_aura_source_is_hit_inert_like_cpp()
            || !aura_subsystem
                .runtime_applications_like_cpp()
                .values()
                .all(|aura| {
                    let (s, h) = crate::session::split_spell_state_ref(self);
                    s.player_visible_aura_is_spell_hit_inert_like_cpp(h, aura)
                })
            || self
                .resolved_inventory_items_like_cpp()
                .is_none_or(|items| !items.is_empty())
            || self
                .resolved_buyback_items_like_cpp()
                .is_none_or(|items| !items.is_empty())
            || self
                .resolved_inventory_item_objects_like_cpp()
                .is_none_or(|items| !items.is_empty())
        {
            return false;
        }

        let known_spells = self.known_spells_like_cpp();
        if known_spells.is_empty() {
            return true;
        }
        let Some(spell_store) = self.catalogs.spell_catalogs.spell_store.as_ref() else {
            return false;
        };
        known_spells.iter().copied().all(|spell_id| {
            spell_id > 0
                && spell_store
                    .misc_attributes_for_difficulty_like_cpp(
                        spell_id,
                        difficulty_id,
                        self.catalogs.difficulty_store.as_deref(),
                    )
                    .is_some_and(|attributes| {
                        attributes[0] & wow_data::spell::attributes::SPELL_ATTR0_PASSIVE == 0
                            || u32::try_from(spell_id).ok().is_some_and(|spell_id| {
                                self.player_target_spell_is_hit_inert_like_cpp(
                                    spell_id,
                                    difficulty_id,
                                )
                            })
                    })
        })
    }

    pub(crate) fn can_authorize_empty_player_spell_hit_aura_source_like_cpp(&self) -> bool {
        self.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(
            self.core.current_map_difficulty_id_like_cpp(),
        )
    }

    /// Publish the combined session-source proof to the canonical Player.
    /// Positive publication is reserved for explicit login/snapshot boundaries;
    /// individual source mutations call the invalidation helper instead.
    pub(crate) fn sync_player_spell_hit_aura_authority_to_canonical_like_cpp(
        &mut self,
    ) -> Option<bool> {
        let complete = self.can_authorize_empty_player_spell_hit_aura_source_like_cpp();
        self.core.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_spell_hit_aura_authority_inert_like_cpp(complete);
            complete
        })
    }
}


#[cfg(test)]
#[path = "../../../../unit_tests/session/spell_state/aura/spell_hit_authority/f3_shims.rs"]
mod f3_shims;

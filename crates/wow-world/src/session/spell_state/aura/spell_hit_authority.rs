//! Canonical Player spell-hit aura-source authority and synchronization.
//! The authority-complete setter and the glyph and add-to-map source queries
//! moved to `wow_world_spell` (#1363 slice 12b3).

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
            || !self
                .inventory
                .player_equipment_inventory_authority_complete_like_cpp(crate::session::hub_ref(
                    self,
                ))
            || self
                .resolved_represented_guild_id_like_cpp()
                .is_none_or(|guild_id| guild_id != 0)
            || crate::session::hub_ref(self)
                .complete_player_skill_records_like_cpp()
                .is_none()
            || wow_world_spell::complete_represented_player_spell_rows_like_cpp(self).is_none()
            || wow_world_spell::complete_represented_spell_trait_definition_ids_like_cpp(self)
                .is_none_or(|traits| !traits.is_empty())
            || !wow_world_spell::represented_trait_config_aura_source_is_empty_like_cpp(self)
            || !wow_world_spell::represented_active_glyph_aura_source_is_empty_like_cpp(self)
            || !self
                .lifecycle
                .represented_battle_pet_login_spell_source_is_empty_like_cpp(
                    crate::session::hub_ref(self),
                )
            || !wow_world_spell::represented_add_player_to_map_aura_source_is_empty_like_cpp(self)
            || !self
                .catalogs
                .represented_auto_push_quest_aura_source_is_empty_like_cpp()
            || !crate::session::hub_ref(self)
                .represented_character_pet_aura_source_is_empty_like_cpp()
            || !wow_world_quest::represented_quest_login_aura_sources_are_hit_inert_like_cpp(
                self,
                difficulty_id,
            )
            || !wow_world_spell::represented_spell_area_autocast_source_is_empty_like_cpp(self)
            || !self.represented_war_mode_update_zone_aura_source_is_empty_like_cpp()
            || !wow_world_spell::represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(
                self,
            )
            || !wow_world_spell::represented_update_zone_script_aura_source_is_hit_inert_like_cpp(
                self,
            )
            || !aura_subsystem
                .runtime_applications_like_cpp()
                .values()
                .all(|aura| {
                    let (s, h) = crate::session::split_spell_state_ref(self);
                    s.player_visible_aura_is_spell_hit_inert_like_cpp(h, aura)
                })
            || self
                .inventory
                .resolved_inventory_items_like_cpp(crate::session::hub_ref(self))
                .is_none_or(|items| !items.is_empty())
            || self
                .inventory
                .resolved_buyback_items_like_cpp(crate::session::hub_ref(self))
                .is_none_or(|items| !items.is_empty())
            || self
                .inventory
                .resolved_inventory_item_objects_like_cpp(crate::session::hub_ref(self))
                .is_none_or(|items| !items.is_empty())
        {
            return false;
        }

        let known_spells = wow_world_spell::known_spells_like_cpp(self);
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
                                wow_world_spell::player_target_spell_is_hit_inert_like_cpp(
                                    self,
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

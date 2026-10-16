//! Applying, refreshing, stacking and expiring represented auras.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn player_aura_application_cx_like_cpp(
        &mut self,
    ) -> wow_world_application::PlayerAuraApplicationCxLikeCpp<'_> {
        let (spell_state, inventory, loot, quest_state, hub) =
            crate::session::state::split_aura_application_mut(self);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = quest_state;
        wow_world_application::player_aura_application_cx_like_cpp(
            hub,
            spell_state,
            inventory,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            quest_state,
            cfg!(test),
        )
    }

    pub(in crate::session) fn apply_aura_with_effect_mask_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        // The admitted application aura owner holds this transition; the World
        // shell only builds the borrowed context (#1263 F6).
        self.player_aura_application_cx_like_cpp()
            .apply_aura_with_effect_mask_like_cpp(
                spell_id,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
            )
    }
    pub(in crate::session) fn apply_aura_with_effect_mask_and_provenance_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_provenance_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            provenance,
            true,
        )
    }
    /// Test-only entry point for a represented multi-effect application, needed
    /// by acceptance cases (for example the transform/`IsPolymorphed` path)
    /// that must mark more than the first effect active.
    #[cfg(test)]
    pub(crate) fn apply_aura_with_effect_mask_for_test_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            0x0000_0001,
            effect_mask,
        )
    }
    fn apply_aura_with_effect_mask_provenance_and_update_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
        send_update: bool,
    ) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .apply_aura_with_effect_mask_provenance_and_update_like_cpp(
                spell_id,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
                provenance,
                send_update,
            )
    }
    pub(crate) fn apply_login_passive_known_spell_auras_like_cpp(&mut self) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(spell_store) = self.spell_store().cloned() else {
            return 0;
        };

        let mut applied = 0usize;
        for spell_id in wow_world_spell::known_spells_like_cpp(self).to_vec() {
            if spell_id <= 0
                || !spell_store.is_passive_like_cpp(spell_id)
                || crate::session::hub_ref(self).player_has_visible_aura_spell_like_cpp(spell_id)
                    != Some(false)
            {
                continue;
            }

            let Some(spell_info) = spell_store.get(spell_id).cloned() else {
                continue;
            };
            let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
            if effect_mask == 0 {
                continue;
            }

            if let Some(equipped) = self
                .catalogs
                .spell_catalogs
                .spell_equipped_items_store
                .as_ref()
                .and_then(|store| store.entry_for_spell_id_like_cpp(spell_id))
                .filter(|entry| entry.equipped_item_class >= 0)
                .cloned()
            {
                let (inventory, hub) = crate::session::split_inventory_ref(self);
                if !inventory
                    .represented_has_item_fit_to_spell_requirements_like_cpp(hub, &equipped)
                {
                    continue;
                }
            } else if !wow_world_spell::represented_login_passive_spell_cast_gate_like_cpp(
                self, spell_id,
            ) {
                continue;
            }

            if self
                .apply_aura_with_effect_mask_like_cpp(
                    spell_id,
                    player_guid,
                    0,
                    AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                    effect_mask,
                )
                .is_ok()
            {
                applied += 1;
            }
        }

        applied
    }
    pub(crate) fn apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(
        &mut self,
        known_spells: &[i32],
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(spell_store) = self.spell_store().cloned() else {
            return 0;
        };

        let mut applied = 0usize;
        for &spell_id in known_spells {
            let Ok(mut previous_spell_id) = u32::try_from(spell_id)
                .map(|spell_id| self.catalogs.prev_spell_in_chain_like_cpp(spell_id))
            else {
                continue;
            };

            while previous_spell_id != 0 {
                let Ok(previous_spell_i32) = i32::try_from(previous_spell_id) else {
                    break;
                };
                if spell_store.is_passive_like_cpp(previous_spell_i32)
                    && crate::session::hub_ref(self)
                        .player_has_visible_aura_spell_like_cpp(previous_spell_i32)
                        == Some(false)
                    && wow_world_spell::represented_login_passive_spell_cast_gate_like_cpp(
                        self,
                        previous_spell_i32,
                    )
                    && let Some(spell_info) = spell_store.get(previous_spell_i32).cloned()
                {
                    let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
                    if effect_mask != 0
                        && self
                            .apply_aura_with_effect_mask_like_cpp(
                                previous_spell_i32,
                                player_guid,
                                0,
                                AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                                effect_mask,
                            )
                            .is_ok()
                    {
                        applied += 1;
                    }
                }

                previous_spell_id = self
                    .catalogs
                    .prev_spell_in_chain_like_cpp(previous_spell_id);
            }
        }

        applied
    }
    /// Remove an aura by slot and send SMSG_AURA_UPDATE.
    pub(crate) fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .remove_aura_like_cpp(slot)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura_application/f3_shims.rs"]
mod f3_shims;

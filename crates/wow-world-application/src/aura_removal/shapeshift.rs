// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;

impl AuraRemovalCxLikeCpp<'_> {
    pub fn sync_shapeshift_form_like_cpp(
        &mut self, mutation: wow_world_spell::RepresentedShapeshiftMutationLikeCpp,
    ) {
        if !self.apply_shapeshift_base_attack_time_like_cpp() { return; }
        match mutation {
            wow_world_spell::RepresentedShapeshiftMutationLikeCpp::Applied { form_id } => {
                self.apply_shapeshift_boosts_like_cpp(form_id);
            }
            wow_world_spell::RepresentedShapeshiftMutationLikeCpp::Removed { removed_form, new_form } => {
                self.remove_shapeshift_boosts_like_cpp(removed_form, new_form);
            }
        }
        self.sync_display_power_like_cpp();
        self.refresh_item_effects_at_form_change_like_cpp();
        let player = self.stats.reborrow_like_cpp(
            &self.player,
            #[cfg(any(test, feature = "test-fixtures"))] &*self.shapeshift_form,
        );
        let mut stats = crate::CharacterStatsApplicationCxLikeCpp::new(
            player, &*self.inventory, self.player.packet_publication_like_cpp(),
        );
        let _ = stats.send_stat_update_like_cpp();
    }

    pub(super) fn apply_shapeshift_base_attack_time_like_cpp(&mut self) -> bool {
        let stats = self.stats.reborrow_like_cpp(
            &self.player,
            #[cfg(any(test, feature = "test-fixtures"))] &*self.shapeshift_form,
        );
        self.inventory.apply_represented_shapeshift_base_attack_time_with_access_like_cpp(&stats)
    }
    fn apply_owned_aura_spell_like_cpp(&mut self, spell_id: i32) -> bool {
        let Some(player_guid) = self.player.player_guid_like_cpp() else { return false; };
        let Some(spell_info) = self.spell_store.and_then(|store| store.get(spell_id)).cloned() else { return false; };
        let effect_mask = wow_world_spell::unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
        if effect_mask == 0 { return false; }
        self.apply_aura_with_effect_mask_like_cpp(
            spell_id, player_guid, 0,
            wow_world_core::session::AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
            effect_mask,
        ).is_ok()
    }

    pub fn apply_shapeshift_boosts_like_cpp(&mut self, form_id: u32) -> usize {
        let boost_ids = self.spell.represented_shapeshift_boost_spell_ids_with_access_like_cpp(&self.player, form_id);
        let mut applied = 0usize;
        for spell_id in &boost_ids {
            if self.apply_owned_aura_spell_like_cpp(*spell_id) { applied += 1; }
        }
        let Some(spell_store) = self.spell_store.cloned() else { return applied; };
        let Some(stance_mask) = form_id.checked_sub(1).and_then(|shift| 1_u64.checked_shl(shift)) else { return applied; };
        for spell_id in self.spell.known_spell_ids_for_aura_with_access_like_cpp(
            &self.player.spell_acquisition_access_like_cpp(), self.consumer_test,
        ).to_vec() {
            if spell_id <= 0 || boost_ids.contains(&spell_id)
                || self.player.player_has_visible_aura_spell_like_cpp(spell_id) == Some(true)
            { continue; }
            let (stances, _) = spell_store.shapeshift_masks_like_cpp(spell_id);
            if stances == 0 || stances & stance_mask == 0 { continue; }
            if !spell_store.is_passive_like_cpp(spell_id)
                && !self.player.spell_has_attribute_like_cpp(
                    self.spell_store.map(|store| store.as_ref()), self.difficulty_store, spell_id, 0,
                    wow_data::spell::attributes::SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG,
                )
            { continue; }
            if self.apply_owned_aura_spell_like_cpp(spell_id) { applied += 1; }
        }
        applied
    }
    fn remove_owned_aura_spell_like_cpp(&mut self, spell_id: i32) -> usize {
        let Some(player_guid) = self.player.player_guid_like_cpp() else { return 0; };
        let slots: Vec<u8> = self.player.visible_auras_snapshot_like_cpp().unwrap_or_default()
            .values()
            .filter(|aura| aura.spell_id == spell_id && aura.caster_guid == player_guid)
            .map(|aura| aura.slot).collect();
        let mut removed = 0usize;
        for slot in slots {
            if self.remove_aura_like_cpp(slot).is_ok() { removed += 1; }
        }
        removed
    }

    pub fn remove_shapeshift_boosts_like_cpp(&mut self, removed_form: u32, new_form: u32) -> usize {
        let mut removed = 0usize;
        for spell_id in self.spell.represented_shapeshift_boost_spell_ids_with_access_like_cpp(&self.player, removed_form) {
            removed += self.remove_owned_aura_spell_like_cpp(spell_id);
        }
        let Some(player_guid) = self.player.player_guid_like_cpp() else { return removed; };
        let Some(spell_store) = self.spell_store.cloned() else { return removed; };
        let new_stance = new_form.checked_sub(1).and_then(|shift| 1_u64.checked_shl(shift)).unwrap_or(0);
        let slots: Vec<u8> = self.player.visible_auras_snapshot_like_cpp().unwrap_or_default()
            .values()
            .filter(|aura| aura.caster_guid == player_guid)
            .filter(|aura| {
                let (stances, _) = spell_store.shapeshift_masks_like_cpp(aura.spell_id);
                stances != 0
                    && !self.player.spell_has_attribute_like_cpp(self.spell_store.map(|store| store.as_ref()), self.difficulty_store, aura.spell_id, 2,
                        wow_data::spell::attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM)
                    && !self.player.spell_has_attribute_like_cpp(self.spell_store.map(|store| store.as_ref()), self.difficulty_store, aura.spell_id, 0,
                        wow_data::spell::attributes::SPELL_ATTR0_NOT_SHAPESHIFTED)
                    && stances & new_stance == 0
            })
            .map(|aura| aura.slot).collect();
        for slot in slots {
            if self.remove_aura_like_cpp(slot).is_ok() { removed += 1; }
        }
        removed
    }
    pub(super) fn shapeshift_ownership_phase_like_cpp(
        &mut self, spell_id: i32,
    ) -> Option<wow_world_spell::RepresentedShapeshiftMutationLikeCpp> {
        self.spell.sync_shapeshift_form_ownership_with_access_like_cpp(
            &self.player, self.spell_store, spell_id, self.consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))] &mut *self.shapeshift_form,
        )
    }

    pub(super) fn display_power_phase_like_cpp(&mut self, spell_id: i32) {
        if self.spell.represented_spell_has_power_display_effect_with_store_like_cpp(
            self.spell_store.map(|store| store.as_ref()), spell_id,
        ) {
            self.sync_display_power_like_cpp();
        }
    }

    fn sync_display_power_like_cpp(&mut self) -> bool {
        self.spell.sync_display_power_with_access_like_cpp(
            &self.player, self.classes_store, self.spell_store.map(|store| store.as_ref()),
            #[cfg(any(test, feature = "test-fixtures"))] self.player_class,
        )
    }
}

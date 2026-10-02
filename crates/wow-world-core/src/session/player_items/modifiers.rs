impl crate::session::state::SessionCatalogs {
    pub fn item_set_for_item_id_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<&wow_data::ItemSetEntry> {
        self.items
            .set_store
            .as_ref()
            .and_then(|store| store.item_set_for_item_id_like_cpp(item_id))
    }

    pub fn item_set_spells_like_cpp(
        &self,
        item_set_id: u32,
    ) -> Vec<&wow_data::ItemSetSpellEntry> {
        self.spell_catalogs
            .item_set_spell_store
            .as_ref()
            .map(|store| store.item_set_spells_like_cpp(item_set_id))
            .unwrap_or_default()
    }

    pub fn represented_item_set_spell_exists_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        self.spell_catalogs
            .spell_store
            .as_ref()
            .is_none_or(|store| store.get(spell_id).is_some())
    }

    /// C++ `ItemTemplate::Effects` ordered by `ItemEffectEntry` slot. The
    /// `CanUseItem` learning-effect gate (`Player.cpp:11110-11113`) reads the
    /// first two entries.
    pub fn represented_item_effect_spell_ids_like_cpp(
        &self,
        item_id: u32,
    ) -> Vec<(u8, i32)> {
        let mut effects: Vec<(u8, i32)> = self
            .items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .values()
                    .filter(|effect| effect.parent_item_id == item_id)
                    .map(|effect| (effect.legacy_slot_index, effect.spell_id))
                    .collect()
            })
            .unwrap_or_default();
        effects.sort_by_key(|(slot, _)| *slot);
        effects
    }
}

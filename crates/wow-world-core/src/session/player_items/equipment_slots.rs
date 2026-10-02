impl crate::session::state::SessionCatalogs {
    /// C++ `SpellInfo::IsItemFitToSpellRequirements`
    /// (`SpellInfo.cpp:1757-1768`) as reached from the `UpdateExpertise`
    /// `AuraEffectFilter`: an item-neutral spell (no `SpellEquippedItems` row
    /// or `EquippedItemClass == -1`) always matches; an item-dependent spell
    /// matches only a present weapon whose template fits the class/subclass
    /// mask.
    pub fn represented_aura_spell_fits_weapon_like_cpp(
        &self,
        spell_id: i32,
        weapon_item_id: Option<u32>,
    ) -> bool {
        let Some(equipped) = self
            .spell_catalogs
            .spell_equipped_items_store
            .as_ref()
            .and_then(|store| store.entry_for_spell_id_like_cpp(spell_id))
        else {
            return true;
        };
        if equipped.equipped_item_class < 0 {
            return true;
        }
        weapon_item_id.is_some_and(|item_id| {
            self.represented_item_fits_spell_requirements_like_cpp(item_id, equipped)
        })
    }
}

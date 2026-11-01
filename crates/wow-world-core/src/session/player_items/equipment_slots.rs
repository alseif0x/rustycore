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
        crate::session::PlayerStatCatalogs::from_session_catalogs(
            self,
            wow_data::StatsLimitsLikeCpp::default(),
        )
        .represented_aura_spell_fits_weapon(spell_id, weapon_item_id)
    }
}

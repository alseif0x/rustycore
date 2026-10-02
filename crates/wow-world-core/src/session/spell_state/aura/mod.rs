mod effect_queries;
mod player_effect_projection;
mod spell_hit_authority;

pub use player_effect_projection::{
    AppliedAuraEffectLikeCpp, PlayerAuraEffectLikeCpp, player_aura_effects_all_like_cpp,
    player_aura_effects_by_spell_aura_type_like_cpp,
    player_aura_effects_full_by_spell_aura_type_like_cpp,
};

impl crate::session::state::SessionCatalogs {
    pub fn spell_area_for_aura_map_bounds_like_cpp(
        &self,
        spell_id: u32,
    ) -> Vec<&wow_data::SpellAreaLikeCpp> {
        self.spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_aura_map_bounds_like_cpp(spell_id))
            .unwrap_or_default()
    }
}

impl crate::session::HubRef<'_> {
    /// C++ `Player::GetShapeshiftForm`'s `SpellShapeshiftFormEntry`:
    /// `Player::CalculateMinMaxDamage` (`StatSystem.cpp:461-467`) rescales the
    /// base weapon damage and `Player::_ApplyWeaponDamage` (`Player.cpp:8018-8020`)
    /// suppresses the item-delay attack time while a form carries a
    /// `CombatRoundTime`. `None` when no form, no store, or a zero field.
    pub fn represented_shapeshift_combat_round_time_like_cpp(&self) -> Option<f32> {
        let form_id = self.represented_shapeshift_form_like_cpp()?;
        let store = self.catalogs.spell_catalogs.spell_shapeshift_form_store()?;
        let form = store.get(form_id)?;
        (form.combat_round_time > 0).then(|| f32::from(form.combat_round_time))
    }
}

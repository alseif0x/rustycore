//! Canonical aura-effect queries and modifier projections.

use super::*;

impl WorldSession {
    pub(in crate::session) fn resolved_has_represented_aura_effect_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<bool> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            wow_entities::AuraSubsystem::has_represented_effect(&auras, effect)
        })
    }

    #[cfg(test)]
    fn has_represented_aura_effect_like_cpp(&self, effect: RepresentedAuraEffectLikeCpp) -> bool {
        self.resolved_has_represented_aura_effect_like_cpp(effect)
            .expect("test Player aura owner must resolve")
    }

    pub(in crate::session) fn resolved_has_represented_aura_effect_with_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<bool> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            wow_entities::AuraSubsystem::has_represented_effect_with_misc(&auras, effect, misc_value)
        })
    }

    #[cfg(test)]
    pub(in crate::session) fn has_represented_aura_effect_with_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> bool {
        self.resolved_has_represented_aura_effect_with_misc_value_like_cpp(effect, misc_value)
            .expect("test Player aura owner must resolve")
    }

    pub(in crate::session) fn resolved_total_represented_aura_modifier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            wow_entities::AuraSubsystem::represented_modifier(&auras, effect)
        })
    }

    #[cfg(test)]
    pub(in crate::session) fn total_represented_aura_modifier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> i32 {
        self.resolved_total_represented_aura_modifier_like_cpp(effect)
            .expect("test Player aura owner must resolve")
    }

    pub(in crate::session) fn resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            wow_entities::AuraSubsystem::represented_modifier_by_misc(&auras, effect, misc_value)
        })
    }

    pub(in crate::session) fn resolved_total_represented_aura_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            wow_entities::AuraSubsystem::represented_multiplier(&auras, effect)
        })
    }

    /// Resolve active aura effects directly from the canonical visible aura
    /// applications and their immutable SpellInfo. This keeps StatSystem
    /// producers independent of packet-only aura mirrors and also covers
    /// loaded applications whose represented-effect enum is intentionally
    /// unset. Each tuple is `(MiscValue, amount)` from the C++ AuraEffect.
    pub(crate) fn resolved_aura_effects_by_spell_aura_type_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.spell_store()?;
        // Delegates to the receiver-free projection the map-owned swing path
        // uses, so both owners resolve the same canonical auras identically.
        Some(
            crate::session_rules::player_aura_effects_by_spell_aura_type_like_cpp(
                &visible_auras,
                spell_store,
                aura_type,
            ),
        )
    }

    /// Resolve active aura effects of `aura_type` paired with their owning
    /// spell id. C++ `GetTotalAuraModifier(aurType, predicate)` filters the
    /// `AuraEffect` list with a predicate that reads the owning `SpellInfo`
    /// (for example `Player::UpdateExpertise`'s item-fit check), so callers
    /// need the spell id next to the amount. Each tuple is `(spell_id, amount)`.
    pub(crate) fn resolved_aura_effect_amounts_by_spell_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.spell_store()?;
        Some(wow_entities::AuraSubsystem::player_effects_in_map_order(
            &visible_auras, aura_type,
            |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
            |effect| (
                effect.effect_index, effect.effect, effect.effect_aura,
                effect.effect_misc_value_1, effect.effect_misc_value_2,
            ),
            wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
            |effect| (effect.spell_id, effect.amount),
        ))
    }

    /// Resolve active aura effects of `aura_type` with the owning spell id, the
    /// C++ `GetMiscValue()` and the amount. `Unit::UpdateDamagePctDoneMods`
    /// (`Unit.cpp:9033-9072`) filters `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` by the
    /// physical school mask and by `Player::CheckAttackFitToAuraRequirement`,
    /// which needs the spell's `SpellEquippedItems` row, so callers need
    /// `(spell_id, misc_value, amount)`.
    pub(crate) fn resolved_aura_effects_with_spell_and_misc_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.spell_store()?;
        Some(wow_entities::AuraSubsystem::player_effects_in_map_order(
            &visible_auras, aura_type,
            |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
            |effect| (
                effect.effect_index, effect.effect, effect.effect_aura,
                effect.effect_misc_value_1, effect.effect_misc_value_2,
            ),
            wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
            |effect| (effect.spell_id, effect.misc_value, effect.amount),
        ))
    }

    /// Resolve active aura effects of `aura_type` with both C++ misc values and
    /// the amount. Several `UnitMods` producers (`HandleAuraModResistance`,
    /// `HandleModResistanceOfStatPercent`) select by `GetMiscValue()` and read
    /// `GetMiscValueB()`, so callers need `(misc_value, misc_value_b, amount)`.
    pub(crate) fn resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.spell_store()?;
        Some(wow_entities::AuraSubsystem::player_effects_in_map_order(
            &visible_auras, aura_type,
            |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
            |effect| (
                effect.effect_index, effect.effect, effect.effect_aura,
                effect.effect_misc_value_1, effect.effect_misc_value_2,
            ),
            wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
            |effect| (effect.misc_value, effect.misc_value_b, effect.amount),
        ))
    }

    /// Resolve a C++ `GetTotalAuraMultiplierByMiscValue` family from the
    /// canonical aura effects.
    pub(crate) fn resolved_total_aura_multiplier_by_spell_aura_type_and_misc_value_like_cpp(
        &self,
        aura_type: i32,
        misc_value: i32,
    ) -> Option<f32> {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .map(|effects| wow_entities::AuraSubsystem::effect_multiplier_by_misc(effects, misc_value))
    }

    /// Resolve a C++ `GetTotalAuraModifierByMiscValue` family from the
    /// canonical aura effects.
    pub(crate) fn resolved_total_aura_modifier_by_spell_aura_type_and_misc_value_like_cpp(
        &self,
        aura_type: i32,
        misc_value: i32,
    ) -> Option<i32> {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .map(|effects| wow_entities::AuraSubsystem::effect_modifier_by_misc(effects, misc_value))
    }

    #[cfg(test)]
    pub(in crate::session) fn total_represented_aura_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> f32 {
        self.resolved_total_represented_aura_multiplier_like_cpp(effect)
            .expect("test Player aura owner must resolve")
    }

    pub(in crate::session) fn aura_has_total_stat_percentage_effect_like_cpp(
        &self,
        aura: &AuraApplication,
    ) -> bool {
        self.spell_store().is_some_and(|store| {
            wow_entities::AuraSubsystem::has_total_stat_percentage(
                aura,
                |spell_id| store.get(spell_id).map(|spell| spell.effects()),
                |effect| (
                    effect.effect_index, effect.effect, effect.effect_aura,
                    effect.effect_misc_value_1, effect.effect_misc_value_2,
                ),
            )
        })
    }

    pub(in crate::session) fn total_stat_percentage_aura_preserves_health_pct_like_cpp(
        &self,
        aura: &AuraApplication,
    ) -> bool {
        self.spell_store().is_some_and(|store| {
            wow_entities::AuraSubsystem::total_stat_percentage_preserves_health(
                aura,
                |spell_id| store.has_attribute0_like_cpp(
                    spell_id, wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY,
                ),
                |spell_id| store.get(spell_id).map(|spell| spell.effects()),
                |effect| (
                    effect.effect_index, effect.effect, effect.effect_aura,
                    effect.effect_misc_value_1, effect.effect_misc_value_2,
                ),
            )
        })
    }

    pub(in crate::session) fn max_represented_aura_amount_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            wow_entities::AuraSubsystem::maximum_represented_amount(&auras, effect)
        })
    }

    pub(in crate::session) fn max_negative_represented_aura_amount_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            wow_entities::AuraSubsystem::minimum_represented_amount(&auras, effect)
        })
    }

    pub(in crate::session) fn total_represented_aura_amount_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            wow_entities::AuraSubsystem::represented_amount_multiplier(&auras, effect)
        })
    }

    pub(in crate::session) fn total_represented_aura_amount_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            wow_entities::AuraSubsystem::represented_modifier(&auras, effect)
        })
    }
}

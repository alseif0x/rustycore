use crate::SessionSpellState;
use wow_entities::{AuraApplicationLikeCpp as AuraApplication, RepresentedAuraEffectLikeCpp};
use wow_world_core::session::HubRef;

impl SessionSpellState {
    pub fn resolved_has_represented_aura_effect_like_cpp(
        &self,
        hub: HubRef<'_>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<bool> {
        hub.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .any(|aura| aura.represented_effect == Some(effect))
        })
    }

    pub fn resolved_has_represented_aura_effect_with_misc_value_like_cpp(
        &self,
        hub: HubRef<'_>,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<bool> {
        hub.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras.values().any(|aura| {
                aura.represented_effect == Some(effect)
                    && aura.represented_misc_value == Some(misc_value)
            })
        })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn has_represented_aura_effect_with_misc_value_like_cpp(
        &self,
        hub: HubRef<'_>,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> bool {
        self.resolved_has_represented_aura_effect_with_misc_value_like_cpp(hub, effect, misc_value)
            .expect("test Player aura owner must resolve")
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn total_represented_aura_modifier_like_cpp(
        &self,
        hub: HubRef<'_>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> i32 {
        hub.resolved_total_represented_aura_modifier_like_cpp(effect)
            .expect("test Player aura owner must resolve")
    }

    pub fn resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
        &self,
        hub: HubRef<'_>,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<i32> {
        hub.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| {
                    aura.represented_effect == Some(effect)
                        && aura.represented_misc_value == Some(misc_value)
                })
                .map(|aura| aura.represented_amount)
                .sum()
        })
    }

    pub fn resolved_total_represented_aura_multiplier_like_cpp(
        &self,
        hub: HubRef<'_>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        hub.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .fold(1.0, |acc, aura| acc * aura.represented_multiplier)
        })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn total_represented_aura_multiplier_like_cpp(
        &self,
        hub: HubRef<'_>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> f32 {
        self.resolved_total_represented_aura_multiplier_like_cpp(hub, effect)
            .expect("test Player aura owner must resolve")
    }

    pub fn aura_has_total_stat_percentage_effect_like_cpp(
        &self,
        hub: HubRef<'_>,
        aura: &AuraApplication,
    ) -> bool {
        hub.catalogs.spell_store().is_some_and(|store| {
            store.get(aura.spell_id).is_some_and(|spell| {
                spell.effects().iter().any(|effect| {
                    1u32.checked_shl(effect.effect_index)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
                        && effect.effect_aura
                            == wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                })
            })
        })
    }

    pub fn total_stat_percentage_aura_preserves_health_pct_like_cpp(
        &self,
        hub: HubRef<'_>,
        aura: &AuraApplication,
    ) -> bool {
        hub.catalogs.spell_store().is_some_and(|store| {
            store.has_attribute0_like_cpp(
                aura.spell_id,
                wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY,
            ) && store.get(aura.spell_id).is_some_and(|spell| {
                spell.effects().iter().any(|effect| {
                    1u32.checked_shl(effect.effect_index)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
                        && effect.effect_aura
                            == wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                        && (effect.effect_misc_value_2 == 0
                            || effect.effect_misc_value_2 & (1 << 2) != 0)
                })
            })
        })
    }
}

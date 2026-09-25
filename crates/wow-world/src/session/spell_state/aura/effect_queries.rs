//! Canonical aura-effect queries and modifier projections.

use super::*;

impl WorldSession {
    pub(in crate::session) fn resolved_has_represented_aura_effect_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<bool> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .any(|aura| aura.represented_effect == Some(effect))
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
            auras.values().any(|aura| {
                aura.represented_effect == Some(effect)
                    && aura.represented_misc_value == Some(misc_value)
            })
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
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .sum()
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

    pub(in crate::session) fn resolved_total_represented_aura_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .fold(1.0, |acc, aura| acc * aura.represented_multiplier)
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
        let mut effects = Vec::new();
        for aura in visible_auras.values() {
            let Some(spell) = spell_store.get(aura.spell_id) else {
                continue;
            };
            for effect in spell.effects().iter().filter(|effect| {
                effect.effect_aura == aura_type
                    && 1u32
                        .checked_shl(effect.effect_index)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
            }) {
                let amount = aura
                    .represented_effect_amounts
                    .iter()
                    .find(|represented| {
                        u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                    })
                    .map(|represented| represented.amount)
                    .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
                effects.push((aura.spell_id, amount));
            }
        }
        Some(effects)
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
        let mut effects = Vec::new();
        for aura in visible_auras.values() {
            let Some(spell) = spell_store.get(aura.spell_id) else {
                continue;
            };
            for effect in spell.effects().iter().filter(|effect| {
                effect.effect_aura == aura_type
                    && 1u32
                        .checked_shl(effect.effect_index)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
            }) {
                let amount = aura
                    .represented_effect_amounts
                    .iter()
                    .find(|represented| {
                        u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                    })
                    .map(|represented| represented.amount)
                    .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
                effects.push((aura.spell_id, effect.effect_misc_value_1, amount));
            }
        }
        Some(effects)
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
        let mut effects = Vec::new();
        for aura in visible_auras.values() {
            let Some(spell) = spell_store.get(aura.spell_id) else {
                continue;
            };
            for effect in spell.effects().iter().filter(|effect| {
                effect.effect_aura == aura_type
                    && 1u32
                        .checked_shl(effect.effect_index)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
            }) {
                let amount = aura
                    .represented_effect_amounts
                    .iter()
                    .find(|represented| {
                        u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                    })
                    .map(|represented| represented.amount)
                    .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
                effects.push((
                    effect.effect_misc_value_1,
                    effect.effect_misc_value_2,
                    amount,
                ));
            }
        }
        Some(effects)
    }

    /// Resolve a C++ `GetTotalAuraMultiplierByMiscValue` family from the
    /// canonical aura effects.
    pub(crate) fn resolved_total_aura_multiplier_by_spell_aura_type_and_misc_value_like_cpp(
        &self,
        aura_type: i32,
        misc_value: i32,
    ) -> Option<f32> {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .map(|effects| {
                effects
                    .into_iter()
                    .filter(|(effect_misc_value, _)| *effect_misc_value == misc_value)
                    .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
            })
    }

    /// Resolve a C++ `GetTotalAuraModifierByMiscValue` family from the
    /// canonical aura effects.
    pub(crate) fn resolved_total_aura_modifier_by_spell_aura_type_and_misc_value_like_cpp(
        &self,
        aura_type: i32,
        misc_value: i32,
    ) -> Option<i32> {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .map(|effects| {
                effects
                    .into_iter()
                    .filter(|(effect_misc_value, _)| *effect_misc_value == misc_value)
                    .map(|(_, amount)| amount)
                    .sum()
            })
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

    pub(in crate::session) fn total_stat_percentage_aura_preserves_health_pct_like_cpp(
        &self,
        aura: &AuraApplication,
    ) -> bool {
        self.spell_store().is_some_and(|store| {
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

    pub(in crate::session) fn max_represented_aura_amount_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .filter(|amount| *amount > 0)
                .max()
                .unwrap_or(0)
        })
    }

    pub(in crate::session) fn max_negative_represented_aura_amount_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .filter(|amount| *amount < 0)
                .min()
                .unwrap_or(0)
        })
    }

    pub(in crate::session) fn total_represented_aura_amount_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .fold(1.0, |multiplier, aura| {
                    multiplier * (1.0 + aura.represented_amount.max(0) as f32 / 100.0)
                })
        })
    }

    pub(in crate::session) fn total_represented_aura_amount_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .sum()
        })
    }
}

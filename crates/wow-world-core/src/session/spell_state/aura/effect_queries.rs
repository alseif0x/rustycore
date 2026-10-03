use wow_entities::RepresentedAuraEffectLikeCpp;
use wow_data::SpellStore;
use wow_entities::AuraApplicationLikeCpp;

pub(crate) fn aura_effect_amounts_by_spell_from_snapshot_like_cpp(
    visible_auras: &std::collections::HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<(i32, i32)> {
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
    effects
}

pub(crate) fn aura_effects_with_spell_and_misc_from_snapshot_like_cpp(
    visible_auras: &std::collections::HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<(i32, i32, i32)> {
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
    effects
}

pub(crate) fn aura_effects_with_misc_values_from_snapshot_like_cpp(
    visible_auras: &std::collections::HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<(i32, i32, i32)> {
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
            effects.push((effect.effect_misc_value_1, effect.effect_misc_value_2, amount));
        }
    }
    effects
}

impl crate::session::HubRef<'_> {
    pub fn resolved_total_represented_aura_modifier_like_cpp(
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

    /// Resolve active aura effects directly from the canonical visible aura
    /// applications and their immutable SpellInfo. This keeps StatSystem
    /// producers independent of packet-only aura mirrors and also covers
    /// loaded applications whose represented-effect enum is intentionally
    /// unset. Each tuple is `(MiscValue, amount)` from the C++ AuraEffect.
    pub fn resolved_aura_effects_by_spell_aura_type_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.catalogs.spell_store()?;
        // Delegates to the receiver-free projection the map-owned swing path
        // uses, so both owners resolve the same canonical auras identically.
        Some(
            crate::session::player_aura_effects_by_spell_aura_type_like_cpp(
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
    pub fn resolved_aura_effect_amounts_by_spell_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.catalogs.spell_store()?;
        Some(aura_effect_amounts_by_spell_from_snapshot_like_cpp(
            &visible_auras,
            spell_store,
            aura_type,
        ))
    }

    /// Resolve active aura effects of `aura_type` with the owning spell id, the
    /// C++ `GetMiscValue()` and the amount. `Unit::UpdateDamagePctDoneMods`
    /// (`Unit.cpp:9033-9072`) filters `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` by the
    /// physical school mask and by `Player::CheckAttackFitToAuraRequirement`,
    /// which needs the spell's `SpellEquippedItems` row, so callers need
    /// `(spell_id, misc_value, amount)`.
    pub fn resolved_aura_effects_with_spell_and_misc_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.catalogs.spell_store()?;
        Some(aura_effects_with_spell_and_misc_from_snapshot_like_cpp(
            &visible_auras,
            spell_store,
            aura_type,
        ))
    }

    /// Resolve active aura effects of `aura_type` with both C++ misc values and
    /// the amount. Several `UnitMods` producers (`HandleAuraModResistance`,
    /// `HandleModResistanceOfStatPercent`) select by `GetMiscValue()` and read
    /// `GetMiscValueB()`, so callers need `(misc_value, misc_value_b, amount)`.
    pub fn resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.catalogs.spell_store()?;
        Some(aura_effects_with_misc_values_from_snapshot_like_cpp(
            &visible_auras,
            spell_store,
            aura_type,
        ))
    }

    /// Resolve a C++ `GetTotalAuraMultiplierByMiscValue` family from the
    /// canonical aura effects.
    pub fn resolved_total_aura_multiplier_by_spell_aura_type_and_misc_value_like_cpp(
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
    pub fn resolved_total_aura_modifier_by_spell_aura_type_and_misc_value_like_cpp(
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

    pub fn max_represented_aura_amount_like_cpp(
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

    pub fn max_negative_represented_aura_amount_like_cpp(
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

    pub fn total_represented_aura_amount_multiplier_like_cpp(
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

    pub fn total_represented_aura_amount_like_cpp(
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

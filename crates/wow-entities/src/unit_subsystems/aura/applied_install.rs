// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Live applied Aura preparation and commit.
//!
//! C++ Aura::TryCreate/Create (SpellAuras.cpp:384-417) and
//! Aura::SetLoadedState (1239-1262). Catalog reads and calculated amounts
//! precede admission; the application's timeline and publication follow it.
//! Target reference a5f8da2e: src/server/game/Spells/Auras/SpellAuras.cpp.
//! Scripts, recalculation and periodic execution remain outside this represented install.

use super::{
    AppliedAuraRef, AuraCastProvenanceLikeCpp, AuraRef, AuraSubsystem, LoadedAuraStateLikeCpp,
    ObjectGuid, OwnedAuraRef, VisibleAuraApplicationLikeCpp, VisibleAuraEffectAmountLikeCpp,
};

impl AuraSubsystem {
    pub fn prepare_live_applied_effects<E>(
        effects: &[E],
        effect_mask: u32,
        fields: impl Fn(&E) -> (u32, i32, i32),
        mut base_amount: impl FnMut(&E) -> i32,
    ) -> Vec<(i32, i32, i32, u8)> {
        effects
            .iter()
            .filter(|effect| {
                1u32.checked_shl(fields(effect).0)
                    .is_some_and(|bit| effect_mask & bit != 0)
            })
            .map(|effect| {
                (
                    fields(effect).1,
                    base_amount(effect),
                    fields(effect).2,
                    u8::try_from(fields(effect).0).unwrap_or(0),
                )
            })
            .collect()
    }

    pub fn install_live_applied_application(
        &mut self,
        spell_key: u32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        provenance: AuraCastProvenanceLikeCpp,
        effects: &[(i32, i32, i32, u8)],
    ) -> Option<()> {
        if self
            .applied_auras
            .iter()
            .any(|aura| aura.spell_id == spell_key && aura.caster_guid == caster_guid)
        {
            return None;
        }
        let slot = (0..u8::MAX).find(|slot| !self.visible_auras.contains_key(slot))?;
        self.add_owned(OwnedAuraRef::new(spell_key, caster_guid, None));
        // C++ `Aura::Create` builds one `AuraEffect` per applied slot
        // and `GetAuraEffectsByType` reads those effects individually.
        // The per-slot `AppliedAuraRef` keeps each slot's own amount
        // and misc value, the convention the pet-load and
        // threat-snapshot paths already use.
        for (aura_type, amount, misc_value, effect_index) in effects {
            let effect_ref = AppliedAuraRef::new(
                spell_key,
                caster_guid,
                slot,
                1_u32 << u32::from(*effect_index),
            );
            self.register_applied_aura_effect_like_cpp(
                effect_ref,
                *aura_type,
                *amount,
                *misc_value,
            );
        }
        self.set_loaded_aura_state_like_cpp(
            AuraRef::new(spell_key, caster_guid),
            LoadedAuraStateLikeCpp::new(
                i32::try_from(duration_ms).unwrap_or(i32::MAX),
                i32::try_from(duration_ms).unwrap_or(i32::MAX),
                0,
                1,
                0,
            ),
        );
        self.set_visible_with_application_like_cpp(
            slot,
            AuraRef::new(spell_key, caster_guid),
            VisibleAuraApplicationLikeCpp::new(
                0,
                effects
                    .iter()
                    .map(
                        |(_, amount, _, effect_index)| VisibleAuraEffectAmountLikeCpp {
                            effect_index: *effect_index,
                            amount: *amount,
                        },
                    )
                    .collect(),
            ),
        );
        self.set_aura_cast_provenance_like_cpp(slot, provenance);
        Some(())
    }
}

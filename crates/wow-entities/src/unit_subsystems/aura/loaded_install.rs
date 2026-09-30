// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Restored Aura materialization and canonical installs.
//!
//! C++ Player::_LoadAuras (Player.cpp:18034-18144), Pet::_LoadAuras
//! (Pet.cpp:1178-1284), Aura::SetLoadedState (SpellAuras.cpp:1239-1262).
//! Port/store validation, cast identity, periodic/recalculation gaps and
//! publication remain with the application. Existing row multiplicity,
//! simplified flags and empty-caster adaptation are preserved.
//! Target reference a5f8da2e: src/server/game/Entities/Player/Player.cpp,
//! Entities/Pet/Pet.cpp and Spells/Auras/SpellAuras.cpp.

use super::{
    AppliedAuraRef, AuraApplicationLikeCpp, AuraCastProvenanceLikeCpp, AuraRef,
    AuraSubsystem, AuraThreatSnapshotLikeCpp, Instant, LoadedAuraStateLikeCpp,
    ObjectGuid, OwnedAuraRef, RepresentedAuraEffectAmountLikeCpp,
    VisibleAuraApplicationLikeCpp, VisibleAuraEffectAmountLikeCpp,
};

impl AuraSubsystem {
    /// Join already-validated, borrowed effect rows in their original order.
    /// The field callback adapts a row's caster without copying its catalog.
    pub fn collect_loaded_effect_amounts<E>(
        caster_guid: ObjectGuid,
        spell_id: u32,
        effect_mask: u32,
        effects: &[E],
        fields: impl Fn(&E) -> (ObjectGuid, u32, u32, u8, i32),
    ) -> Vec<RepresentedAuraEffectAmountLikeCpp> {
        effects.iter()
            .filter(|effect| {
                let (caster, spell, mask, _, _) = fields(effect);
                caster == caster_guid && spell == spell_id && mask == effect_mask
            })
            .map(|effect| {
                let (_, _, _, effect_index, amount) = fields(effect);
                RepresentedAuraEffectAmountLikeCpp { effect_index, amount }
            })
            .collect()
    }

    /// Construct the restored runtime record after the application's cast
    /// GUID, visual and threat reads. The clock is the final field evaluation.
    pub fn build_loaded_runtime_application(
        spell_id: i32,
        difficulty_id: u8,
        caster_guid: ObjectGuid,
        slot: u8,
        state: LoadedAuraStateLikeCpp,
        aura_flags: u32,
        effect_mask: u32,
        amounts: Vec<RepresentedAuraEffectAmountLikeCpp>,
        now: impl FnOnce() -> Instant,
    ) -> AuraApplicationLikeCpp {
        AuraApplicationLikeCpp {
            spell_id,
            difficulty_id,
            caster_guid,
            slot,
            duration_total: u32::try_from(state.max_duration_ms).unwrap_or(0),
            duration_remaining: u32::try_from(state.duration_ms).unwrap_or(0),
            stack_count: state.stack_amount.max(1),
            aura_flags,
            effect_mask,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: amounts,
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: now(),
        }
    }

    /// Keep the loaded Player's snapshot -> runtime -> provenance sequence.
    /// This is a distinct operation from the generic threat install.
    pub fn install_loaded_runtime_application(
        &mut self,
        slot: u8,
        snapshot: AuraThreatSnapshotLikeCpp,
        aura: AuraApplicationLikeCpp,
        provenance: AuraCastProvenanceLikeCpp,
    ) {
        self.insert_threat_snapshot_like_cpp(slot, snapshot);
        self.insert_runtime_application_like_cpp(aura);
        self.set_aura_cast_provenance_like_cpp(slot, provenance);
    }

    /// Install one restored pet Aura on the existing canonical subsystem,
    /// before the application inserts that Pet into Map storage.
    pub fn install_loaded_applied_application<E>(
        &mut self,
        slot: u8,
        aura_ref: AuraRef,
        effect_mask: u32,
        state: LoadedAuraStateLikeCpp,
        effects: &[E],
        fields: impl Fn(&E) -> (ObjectGuid, u32, u32, u8, i32),
    ) {
        self.add_owned(OwnedAuraRef::new(aura_ref.spell_id, aura_ref.caster_guid, None));
        self.add_applied(AppliedAuraRef::new(
            aura_ref.spell_id, aura_ref.caster_guid, slot, effect_mask,
        ));
        self.set_loaded_aura_state_like_cpp(aura_ref, state);
        self.visible_auras.insert(slot, aura_ref);

        let effect_amounts: Vec<_> = effects.iter()
            .filter(|effect| {
                let (caster, spell, mask, _, _) = fields(effect);
                caster == aura_ref.caster_guid
                    && spell == aura_ref.spell_id && mask == effect_mask
            })
            .map(|effect| {
                let (_, _, _, effect_index, amount) = fields(effect);
                let effect_ref = AppliedAuraRef::new(
                    aura_ref.spell_id, aura_ref.caster_guid, slot,
                    1u32 << u32::from(effect_index),
                );
                self.applied_aura_amounts.insert(effect_ref, amount);
                VisibleAuraEffectAmountLikeCpp { effect_index, amount }
            })
            .collect();
        self.visible_aura_applications_like_cpp.insert(
            slot, VisibleAuraApplicationLikeCpp::new(effect_mask, effect_amounts),
        );
    }

    /// Preserve the represented offline arithmetic, including saturation,
    /// the -1 permanent sentinel and the current caller-supplied positivity.
    pub fn offline_remaining_duration(
        remain_time_ms: i32,
        timediff_secs: u32,
        is_positive: bool,
        aura_expires_offline: bool,
    ) -> Option<i32> {
        if remain_time_ms != -1 && (!is_positive || aura_expires_offline) {
            let timediff_secs = i32::try_from(timediff_secs).unwrap_or(i32::MAX);
            if remain_time_ms / 1_000 <= timediff_secs {
                return None;
            }

            return Some(remain_time_ms.saturating_sub(timediff_secs.saturating_mul(1_000)));
        }

        Some(remain_time_ms)
    }
}

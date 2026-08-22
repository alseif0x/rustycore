// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit aura subsystem.

use super::*;

/// Minimal bridge for TrinityCore `Unit` aura containers.
///
/// This is metadata/state only: it does not run aura scripts, periodic ticks, proc logic,
/// packet emission, or update-field masking by itself.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AuraSubsystem {
    /// Whether every aura source omitted from this canonical Unit has been
    /// proven inert for the bounded spell-hit resolution.
    ///
    /// This is deliberately private and defaults to false: an empty set of
    /// runtime containers is not evidence that the backing sources were read.
    spell_hit_aura_authority_inert_like_cpp: bool,
    /// Whether omitted aura sources are proven inert for the AP, spell-power,
    /// armor and effective power-cost values embedded in advanced combat-log
    /// packets.
    ///
    /// This proof is intentionally separate from spell-hit authority: the two
    /// consumers depend on different C++ aura families.
    spell_cast_log_aura_authority_inert_like_cpp: bool,
    pub owned_auras: Vec<OwnedAuraRef>,
    pub applied_auras: Vec<AppliedAuraRef>,
    pub applied_aura_types: HashMap<i32, Vec<AppliedAuraRef>>,
    pub applied_aura_amounts: HashMap<AppliedAuraRef, i32>,
    /// C++ `AuraEffect::GetMiscValue()`; distinct from effect amount.
    pub applied_aura_misc_values: HashMap<AppliedAuraRef, i32>,
    pub loaded_aura_states_like_cpp: HashMap<AuraRef, LoadedAuraStateLikeCpp>,
    pub visible_auras: HashMap<u8, AuraRef>,
    pub visible_aura_applications_like_cpp: HashMap<u8, VisibleAuraApplicationLikeCpp>,
    pub visible_auras_to_update: HashSet<u8>,
    pub removed_auras: Vec<AuraRef>,
    pub removed_auras_count: u32,
    pub passive_auras_like_cpp: HashSet<AuraRef>,
    pub death_persistent_auras_like_cpp: HashSet<AuraRef>,
    pub interruptible_auras: Vec<AppliedAuraRef>,
    pub aura_interrupt_flags: HashMap<AppliedAuraRef, (u32, u32)>,
    pub aura_state_auras: HashMap<u8, Vec<AppliedAuraRef>>,
    pub aura_state_mask: u32,
    pub interrupt_flags: u32,
    pub interrupt_flags2: u32,
    pub proc_depth: u16,
    pub proc_chain_length: i32,
    pub diminishing: [DiminishingReturnState; DIMINISHING_MAX],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadedAuraStateLikeCpp {
    pub max_duration_ms: i32,
    pub duration_ms: i32,
    pub charges: u8,
    pub stack_amount: u8,
    pub recalculate_mask: u32,
}

impl LoadedAuraStateLikeCpp {
    pub const fn new(
        max_duration_ms: i32,
        duration_ms: i32,
        charges: u8,
        stack_amount: u8,
        recalculate_mask: u32,
    ) -> Self {
        Self {
            max_duration_ms,
            duration_ms,
            charges,
            stack_amount,
            recalculate_mask,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuraRef {
    pub spell_id: u32,
    pub caster_guid: ObjectGuid,
}

impl AuraRef {
    pub const fn new(spell_id: u32, caster_guid: ObjectGuid) -> Self {
        Self {
            spell_id,
            caster_guid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OwnedAuraRef {
    pub spell_id: u32,
    pub caster_guid: ObjectGuid,
    pub item_caster_guid: Option<ObjectGuid>,
}

impl OwnedAuraRef {
    pub const fn new(
        spell_id: u32,
        caster_guid: ObjectGuid,
        item_caster_guid: Option<ObjectGuid>,
    ) -> Self {
        Self {
            spell_id,
            caster_guid,
            item_caster_guid,
        }
    }

    pub const fn aura_ref(self) -> AuraRef {
        AuraRef::new(self.spell_id, self.caster_guid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppliedAuraRef {
    pub spell_id: u32,
    pub caster_guid: ObjectGuid,
    pub slot: u8,
    pub effect_mask: u32,
}

impl AppliedAuraRef {
    pub const fn new(spell_id: u32, caster_guid: ObjectGuid, slot: u8, effect_mask: u32) -> Self {
        Self {
            spell_id,
            caster_guid,
            slot,
            effect_mask,
        }
    }

    pub const fn aura_ref(self) -> AuraRef {
        AuraRef::new(self.spell_id, self.caster_guid)
    }
}

/// Bounded snapshot of the C++ `AuraApplication` fields needed by
/// `PartyMemberAuraStates`.
///
/// This is only representation data. It does not own aura lifetime, scripts,
/// effect recalculation, proc state, or packet fanout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VisibleAuraApplicationLikeCpp {
    pub flags: u32,
    pub effect_amounts: Vec<VisibleAuraEffectAmountLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleAuraEffectAmountLikeCpp {
    pub effect_index: u8,
    pub amount: i32,
}

impl VisibleAuraApplicationLikeCpp {
    pub fn new(flags: u32, effect_amounts: Vec<VisibleAuraEffectAmountLikeCpp>) -> Self {
        Self {
            flags,
            effect_amounts,
        }
    }
}

impl AuraSubsystem {
    pub fn set_spell_hit_aura_authority_inert_like_cpp(&mut self, inert: bool) {
        self.spell_hit_aura_authority_inert_like_cpp = inert;
    }

    pub fn set_spell_cast_log_aura_authority_inert_like_cpp(&mut self, inert: bool) {
        self.spell_cast_log_aura_authority_inert_like_cpp = inert;
    }

    pub fn invalidate_spell_hit_aura_authority_like_cpp(&mut self) {
        self.spell_hit_aura_authority_inert_like_cpp = false;
        self.spell_cast_log_aura_authority_inert_like_cpp = false;
    }

    /// Proves that the represented Unit has no aura state capable of
    /// influencing the bounded spell-hit resolution.
    ///
    /// The private marker accredits omitted source state as hit-inert. Local
    /// canonical containers must still be empty because this subsystem does
    /// not classify their effects. Every local aura mutation revokes the
    /// marker, so adding and later removing an aura cannot resurrect stale
    /// authority merely because the containers became empty again.
    pub fn has_complete_spell_hit_inert_aura_authority_like_cpp(&self) -> bool {
        self.spell_hit_aura_authority_inert_like_cpp && self.has_no_local_aura_state_like_cpp()
    }

    /// Proves that no represented or omitted aura can alter the advanced
    /// combat-log stat snapshot.
    pub fn has_complete_spell_cast_log_aura_authority_like_cpp(&self) -> bool {
        self.spell_cast_log_aura_authority_inert_like_cpp && self.has_no_local_aura_state_like_cpp()
    }

    fn has_no_local_aura_state_like_cpp(&self) -> bool {
        self.owned_auras.is_empty()
            && self.applied_auras.is_empty()
            && self.applied_aura_types.is_empty()
            && self.applied_aura_amounts.is_empty()
            && self.applied_aura_misc_values.is_empty()
            && self.loaded_aura_states_like_cpp.is_empty()
            && self.visible_auras.is_empty()
            && self.visible_aura_applications_like_cpp.is_empty()
            && self.visible_auras_to_update.is_empty()
            && self.removed_auras.is_empty()
            && self.removed_auras_count == 0
            && self.passive_auras_like_cpp.is_empty()
            && self.death_persistent_auras_like_cpp.is_empty()
            && self.interruptible_auras.is_empty()
            && self.aura_interrupt_flags.is_empty()
            && self.aura_state_auras.is_empty()
            && self.aura_state_mask == 0
            && self.interrupt_flags == 0
            && self.interrupt_flags2 == 0
    }

    pub fn add_owned(&mut self, aura: OwnedAuraRef) {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        if !self.owned_auras.contains(&aura) {
            self.owned_auras.push(aura);
        }
    }

    pub fn remove_owned(&mut self, aura: OwnedAuraRef) -> bool {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        let before = self.owned_auras.len();
        self.owned_auras.retain(|known| *known != aura);
        before != self.owned_auras.len()
    }

    pub fn remove_owned_by_aura_ref_like_cpp(&mut self, aura: AuraRef) -> bool {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        let before = self.owned_auras.len();
        self.owned_auras.retain(|known| known.aura_ref() != aura);
        before != self.owned_auras.len()
    }

    pub fn has_owned(&self, aura: OwnedAuraRef) -> bool {
        self.owned_auras.contains(&aura)
    }

    pub fn add_applied(&mut self, aura: AppliedAuraRef) {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        if !self.applied_auras.contains(&aura) {
            self.applied_auras.push(aura);
        }
    }

    pub fn set_loaded_aura_state_like_cpp(&mut self, aura: AuraRef, state: LoadedAuraStateLikeCpp) {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        self.loaded_aura_states_like_cpp.insert(aura, state);
    }

    pub fn register_applied_aura_type_like_cpp(&mut self, aura: AppliedAuraRef, aura_type: i32) {
        self.add_applied(aura);
        let typed_auras = self.applied_aura_types.entry(aura_type).or_default();
        if !typed_auras.contains(&aura) {
            typed_auras.push(aura);
        }
    }

    pub fn register_applied_aura_modifier_like_cpp(
        &mut self,
        aura: AppliedAuraRef,
        aura_type: i32,
        amount: i32,
    ) {
        self.register_applied_aura_type_like_cpp(aura, aura_type);
        self.applied_aura_amounts.insert(aura, amount);
    }

    pub fn register_applied_aura_effect_like_cpp(
        &mut self,
        aura: AppliedAuraRef,
        aura_type: i32,
        amount: i32,
        misc_value: i32,
    ) {
        self.register_applied_aura_modifier_like_cpp(aura, aura_type, amount);
        self.applied_aura_misc_values.insert(aura, misc_value);
    }

    pub fn remove_applied(&mut self, aura: AppliedAuraRef) -> bool {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        let before = self.applied_auras.len();
        self.applied_auras.retain(|known| *known != aura);
        for typed_auras in self.applied_aura_types.values_mut() {
            typed_auras.retain(|known| *known != aura);
        }
        self.applied_aura_types
            .retain(|_, typed_auras| !typed_auras.is_empty());
        self.interruptible_auras.retain(|known| *known != aura);
        self.aura_interrupt_flags.remove(&aura);
        for auras in self.aura_state_auras.values_mut() {
            auras.retain(|known| *known != aura);
        }
        self.aura_state_auras.retain(|_, auras| !auras.is_empty());
        self.applied_aura_amounts.remove(&aura);
        self.applied_aura_misc_values.remove(&aura);
        self.loaded_aura_states_like_cpp.remove(&aura.aura_ref());
        self.update_interrupt_masks();
        before != self.applied_auras.len()
    }

    pub fn has_applied(&self, aura: AppliedAuraRef) -> bool {
        self.applied_auras.contains(&aura)
    }

    pub fn has_aura_spell_like_cpp(&self, spell_id: u32) -> bool {
        self.applied_auras
            .iter()
            .any(|aura| aura.spell_id == spell_id)
    }

    pub fn add_self_cast_addon_aura_like_cpp(
        &mut self,
        spell_id: u32,
        caster_guid: ObjectGuid,
    ) -> bool {
        self.add_self_cast_addon_aura_application_like_cpp(spell_id, caster_guid, 0, 0)
    }

    pub fn add_self_cast_addon_aura_application_like_cpp(
        &mut self,
        spell_id: u32,
        caster_guid: ObjectGuid,
        effect_mask: u32,
        flags: u32,
    ) -> bool {
        if self
            .applied_auras
            .iter()
            .any(|aura| aura.spell_id == spell_id && aura.caster_guid == caster_guid)
        {
            return false;
        }

        let Some(slot) = (0..u8::MAX).find(|slot| !self.visible_auras.contains_key(slot)) else {
            return false;
        };
        let owned = OwnedAuraRef::new(spell_id, caster_guid, None);
        let applied = AppliedAuraRef::new(spell_id, caster_guid, slot, effect_mask);
        let aura_ref = applied.aura_ref();
        self.add_owned(owned);
        self.add_applied(applied);
        self.set_visible_with_application_like_cpp(
            slot,
            aura_ref,
            VisibleAuraApplicationLikeCpp::new(flags, Vec::new()),
        );
        true
    }

    pub fn has_aura_type_like_cpp(&self, aura_type: i32) -> bool {
        self.applied_aura_types
            .get(&aura_type)
            .is_some_and(|auras| !auras.is_empty())
    }

    pub fn has_aura_type_with_caster_like_cpp(
        &self,
        aura_type: i32,
        caster_guid: ObjectGuid,
    ) -> bool {
        self.applied_aura_types
            .get(&aura_type)
            .is_some_and(|auras| auras.iter().any(|aura| aura.caster_guid == caster_guid))
    }

    pub fn total_aura_modifier_like_cpp(&self, aura_type: i32) -> i32 {
        self.applied_aura_types
            .get(&aura_type)
            .into_iter()
            .flatten()
            .map(|aura| self.applied_aura_amounts.get(aura).copied().unwrap_or(0))
            .sum()
    }

    /// C++ `Unit::GetSchoolImmunityMask` / `GetDamageImmunityMask` reduce
    /// the misc values of the corresponding aura effects to a school mask.
    pub fn aura_school_mask_like_cpp(&self, aura_type: i32) -> u32 {
        self.applied_aura_types
            .get(&aura_type)
            .into_iter()
            .flatten()
            .fold(0_u32, |mask, aura| {
                mask | self
                    .applied_aura_misc_values
                    .get(aura)
                    .copied()
                    .unwrap_or(0) as u32
            })
    }

    /// C++ `GetTotalAuraMultiplierByMiscMask`: multiply percentage aura
    /// amounts whose `MiscValue` intersects the requested school mask.
    pub fn total_aura_multiplier_by_misc_mask_like_cpp(
        &self,
        aura_type: i32,
        misc_mask: u32,
    ) -> f32 {
        self.applied_aura_types
            .get(&aura_type)
            .into_iter()
            .flatten()
            .filter(|aura| {
                self.applied_aura_misc_values
                    .get(aura)
                    .is_some_and(|value| (*value as u32) & misc_mask != 0)
            })
            .fold(1.0_f32, |multiplier, aura| {
                let amount = self.applied_aura_amounts.get(aura).copied().unwrap_or(0);
                multiplier * (1.0 + amount as f32 / 100.0)
            })
    }

    /// C++ `Unit::HasBreakableByDamageAuraType` requires the requested aura
    /// type and a damage interrupt flag on the same aura application.
    pub fn has_breakable_by_damage_aura_type_like_cpp(&self, aura_type: i32) -> bool {
        let damage_flag = wow_constants::SpellAuraInterruptFlags::DAMAGE.bits();
        self.applied_aura_types
            .get(&aura_type)
            .into_iter()
            .flatten()
            .any(|aura| {
                self.aura_interrupt_flags
                    .get(aura)
                    .is_some_and(|(flags, _)| flags & damage_flag != 0)
            })
    }

    pub fn remove_auras_by_type_like_cpp(&mut self, aura_type: i32) -> Vec<AppliedAuraRef> {
        let removed = self
            .applied_aura_types
            .remove(&aura_type)
            .unwrap_or_default();
        for aura in &removed {
            self.unapply_aura(*aura, 1);
        }
        removed
    }

    /// Bounded representation of C++ `Unit::RemoveAurasDueToSpell`.
    ///
    /// This removes represented applied aura refs matching `spell_id`, optional
    /// caster, and required effect mask. It does not run aura scripts/procs or
    /// packet fanout; callers use `removed_auras` evidence for that later layer.
    pub fn remove_auras_due_to_spell_like_cpp(
        &mut self,
        spell_id: u32,
        caster_guid: ObjectGuid,
        req_eff_mask: u32,
    ) -> Vec<AppliedAuraRef> {
        let removed: Vec<_> = self
            .applied_auras
            .iter()
            .copied()
            .filter(|aura| {
                aura.spell_id == spell_id
                    && (aura.effect_mask & req_eff_mask) == req_eff_mask
                    && (caster_guid.is_empty() || aura.caster_guid == caster_guid)
            })
            .collect();
        for aura in &removed {
            self.unapply_aura(*aura, 1);
            self.remove_owned_by_aura_ref_like_cpp(aura.aura_ref());
        }
        removed
    }

    pub fn set_visible(&mut self, slot: u8, aura: AuraRef) {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        self.visible_auras.insert(slot, aura);
        self.visible_aura_applications_like_cpp.remove(&slot);
        self.visible_auras_to_update.insert(slot);
    }

    pub fn set_visible_with_application_like_cpp(
        &mut self,
        slot: u8,
        aura: AuraRef,
        application: VisibleAuraApplicationLikeCpp,
    ) {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        self.visible_auras.insert(slot, aura);
        self.visible_aura_applications_like_cpp
            .insert(slot, application);
        self.visible_auras_to_update.insert(slot);
    }

    pub fn clear_visible(&mut self, slot: u8) -> Option<AuraRef> {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        self.visible_auras_to_update.remove(&slot);
        self.visible_aura_applications_like_cpp.remove(&slot);
        self.visible_auras.remove(&slot)
    }

    pub fn mark_removed(&mut self, aura: AuraRef) {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        self.removed_auras.push(aura);
        self.removed_auras_count = self.removed_auras_count.saturating_add(1);
    }

    pub fn set_aura_death_policy_like_cpp(
        &mut self,
        aura: AuraRef,
        passive: bool,
        death_persistent: bool,
    ) {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        if passive {
            self.passive_auras_like_cpp.insert(aura);
        } else {
            self.passive_auras_like_cpp.remove(&aura);
        }
        if death_persistent {
            self.death_persistent_auras_like_cpp.insert(aura);
        } else {
            self.death_persistent_auras_like_cpp.remove(&aura);
        }
    }

    pub fn remove_all_auras_on_death_like_cpp(
        &mut self,
    ) -> (Vec<AppliedAuraRef>, Vec<OwnedAuraRef>) {
        let removable_applied: Vec<_> = self
            .applied_auras
            .iter()
            .copied()
            .filter(|aura| self.aura_removed_on_death_like_cpp(aura.aura_ref()))
            .collect();
        for aura in &removable_applied {
            self.unapply_aura(*aura, 1);
        }

        let removable_owned: Vec<_> = self
            .owned_auras
            .iter()
            .copied()
            .filter(|aura| self.aura_removed_on_death_like_cpp(aura.aura_ref()))
            .collect();
        for aura in &removable_owned {
            if self.remove_owned(*aura) {
                self.mark_removed(aura.aura_ref());
            }
        }

        (removable_applied, removable_owned)
    }

    fn aura_removed_on_death_like_cpp(&self, aura: AuraRef) -> bool {
        !self.passive_auras_like_cpp.contains(&aura)
            && !self.death_persistent_auras_like_cpp.contains(&aura)
    }

    pub fn clear_removed(&mut self) {
        self.invalidate_spell_hit_aura_authority_like_cpp();
        self.removed_auras.clear();
        self.removed_auras_count = 0;
    }

    pub fn removed_count(&self) -> usize {
        self.removed_auras.len()
    }

    pub fn register_applied_aura(
        &mut self,
        aura: AppliedAuraRef,
        aura_state: Option<u8>,
        interrupt_flags: u32,
        interrupt_flags2: u32,
    ) {
        self.add_applied(aura);
        if interrupt_flags != 0 || interrupt_flags2 != 0 {
            if !self.interruptible_auras.contains(&aura) {
                self.interruptible_auras.push(aura);
            }
            self.aura_interrupt_flags
                .insert(aura, (interrupt_flags, interrupt_flags2));
            self.interrupt_flags |= interrupt_flags;
            self.interrupt_flags2 |= interrupt_flags2;
        }
        if let Some(aura_state) = aura_state.filter(|state| *state != AURA_STATE_NONE) {
            self.aura_state_auras
                .entry(aura_state)
                .or_default()
                .push(aura);
            self.modify_aura_state(aura_state, true);
        }
    }

    pub fn unapply_aura(&mut self, aura: AppliedAuraRef, remove_mode_marker: u8) -> bool {
        let removed = self.remove_applied(aura);
        if removed {
            self.mark_removed(aura.aura_ref());
            if remove_mode_marker != 0 {
                self.removed_auras_count = self.removed_auras_count.saturating_add(0);
            }
            self.rebuild_aura_state_mask();
        }
        removed
    }

    pub fn has_interrupt_flag(&self, flags: u32) -> bool {
        (self.interrupt_flags & flags) != 0
    }

    pub fn has_interrupt_flag2(&self, flags: u32) -> bool {
        (self.interrupt_flags2 & flags) != 0
    }

    pub fn remove_interruptible_auras(&mut self, flags: u32, flags2: u32) -> Vec<AppliedAuraRef> {
        let removed: Vec<_> = self
            .interruptible_auras
            .iter()
            .copied()
            .filter(|aura| {
                self.aura_interrupt_flags
                    .get(aura)
                    .is_some_and(|(known_flags, known_flags2)| {
                        (flags != 0 && (known_flags & flags) != 0)
                            || (flags2 != 0 && (known_flags2 & flags2) != 0)
                    })
            })
            .collect();
        for aura in &removed {
            self.unapply_aura(*aura, AURA_REMOVE_BY_INTERRUPT_LIKE_CPP);
        }
        removed
    }

    pub fn modify_aura_state(&mut self, flag: u8, apply: bool) {
        if flag == AURA_STATE_NONE {
            return;
        }
        self.invalidate_spell_hit_aura_authority_like_cpp();
        let mask = 1 << (flag - 1);
        if apply {
            self.aura_state_mask |= mask;
        } else {
            self.aura_state_mask &= !mask;
        }
    }

    pub fn has_aura_state(&self, flag: u8) -> bool {
        if flag == AURA_STATE_NONE {
            return false;
        }
        (self.aura_state_mask & (1 << (flag - 1))) != 0
    }

    pub fn clear_all_reactives_like_cpp(&mut self) {
        self.modify_aura_state(AURA_STATE_DEFENSIVE, false);
        self.modify_aura_state(AURA_STATE_DEFENSIVE_2, false);
    }

    pub fn build_aura_state_update_for_target(&self, target: ObjectGuid) -> u32 {
        let mut aura_states = self.aura_state_mask & !PER_CASTER_AURA_STATE_MASK;
        for (state, auras) in &self.aura_state_auras {
            let mask = 1 << (*state - 1);
            if (mask & PER_CASTER_AURA_STATE_MASK) != 0
                && auras.iter().any(|aura| aura.caster_guid == target)
            {
                aura_states |= mask;
            }
        }
        aura_states
    }

    pub fn can_proc(&self) -> bool {
        self.proc_depth == 0
    }

    pub fn set_cant_proc(&mut self, apply: bool) {
        if apply {
            self.proc_depth = self.proc_depth.saturating_add(1);
        } else {
            self.proc_depth = self.proc_depth.saturating_sub(1);
        }
    }

    pub fn get_diminishing(&self, group: usize, now_ms: u64) -> DiminishingLevel {
        let Some(diminish) = self.diminishing.get(group) else {
            return DiminishingLevel::Level1;
        };
        if diminish.hit_count == DiminishingLevel::Level1 {
            return DiminishingLevel::Level1;
        }
        if diminish.stack == 0
            && now_ms.saturating_sub(diminish.hit_time_ms) > DIMINISHING_RESET_INTERVAL_MS
        {
            return DiminishingLevel::Level1;
        }
        diminish.hit_count
    }

    pub fn incr_diminishing(&mut self, group: usize, max_level: DiminishingLevel, now_ms: u64) {
        if group >= DIMINISHING_MAX {
            return;
        }
        let current = self.get_diminishing(group, now_ms);
        if current < max_level {
            self.diminishing[group].hit_count = next_diminishing_level(current, max_level);
        }
    }

    pub fn apply_diminishing_aura(&mut self, group: usize, apply: bool, now_ms: u64) {
        let Some(diminish) = self.diminishing.get_mut(group) else {
            return;
        };
        if apply {
            diminish.stack = diminish.stack.saturating_add(1);
        } else if diminish.stack > 0 {
            diminish.stack -= 1;
            if diminish.stack == 0 {
                diminish.hit_time_ms = now_ms;
            }
        }
    }

    pub fn clear_diminishings(&mut self) {
        for diminish in &mut self.diminishing {
            diminish.clear();
        }
    }

    fn update_interrupt_masks(&mut self) {
        self.interrupt_flags = 0;
        self.interrupt_flags2 = 0;
        for (flags, flags2) in self.aura_interrupt_flags.values() {
            self.interrupt_flags |= *flags;
            self.interrupt_flags2 |= *flags2;
        }
    }

    fn rebuild_aura_state_mask(&mut self) {
        self.aura_state_mask = 0;
        let states: Vec<_> = self.aura_state_auras.keys().copied().collect();
        for state in states {
            self.modify_aura_state(state, true);
        }
    }
}

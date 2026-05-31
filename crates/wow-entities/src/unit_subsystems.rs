use std::collections::{HashMap, HashSet, VecDeque};

use wow_constants::{SpellState, TypeId, UnitState};
use wow_core::{ObjectGuid, Position};

use crate::{
    CreatureAddToWorldVehicleResetContextLikeCpp, Vehicle, VehicleResetPlan, VehicleSeatAddon,
    VehicleSeatInfo,
};

/// Minimal bridge for TrinityCore `Unit` aura containers.
///
/// This is metadata/state only: it does not run aura scripts, periodic ticks, proc logic,
/// packet emission, or update-field masking by itself.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AuraSubsystem {
    pub owned_auras: Vec<OwnedAuraRef>,
    pub applied_auras: Vec<AppliedAuraRef>,
    pub applied_aura_types: HashMap<i32, Vec<AppliedAuraRef>>,
    pub visible_auras: HashMap<u8, AuraRef>,
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

pub const AURA_STATE_NONE: u8 = 0;
pub const AURA_STATE_DEFENSIVE: u8 = 1;
pub const AURA_STATE_DEFENSIVE_2: u8 = 7;
pub const AURA_STATE_RAID_ENCOUNTER_2: u8 = 14;
pub const AURA_STATE_ROGUE_POISONED: u8 = 16;
pub const AURA_STATE_ENRAGED: u8 = 17;
pub const PER_CASTER_AURA_STATE_MASK: u32 =
    (1 << (AURA_STATE_RAID_ENCOUNTER_2 - 1)) | (1 << (AURA_STATE_ROGUE_POISONED - 1));

pub const DIMINISHING_NONE: usize = 0;
pub const DIMINISHING_ROOT: usize = 1;
pub const DIMINISHING_STUN: usize = 2;
pub const DIMINISHING_INCAPACITATE: usize = 3;
pub const DIMINISHING_DISORIENT: usize = 4;
pub const DIMINISHING_SILENCE: usize = 5;
pub const DIMINISHING_AOE_KNOCKBACK: usize = 6;
pub const DIMINISHING_TAUNT: usize = 7;
pub const DIMINISHING_LIMITONLY: usize = 8;
pub const DIMINISHING_MAX: usize = 9;
pub const DIMINISHING_RESET_INTERVAL_MS: u64 = 18_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum DiminishingLevel {
    Level1 = 0,
    Level2 = 1,
    Level3 = 2,
    Immune = 3,
    TauntImmune = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiminishingReturnState {
    pub stack: u16,
    pub hit_time_ms: u64,
    pub hit_count: DiminishingLevel,
}

impl Default for DiminishingReturnState {
    fn default() -> Self {
        Self {
            stack: 0,
            hit_time_ms: 0,
            hit_count: DiminishingLevel::Level1,
        }
    }
}

impl DiminishingReturnState {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl AuraSubsystem {
    pub fn add_owned(&mut self, aura: OwnedAuraRef) {
        if !self.owned_auras.contains(&aura) {
            self.owned_auras.push(aura);
        }
    }

    pub fn remove_owned(&mut self, aura: OwnedAuraRef) -> bool {
        let before = self.owned_auras.len();
        self.owned_auras.retain(|known| *known != aura);
        before != self.owned_auras.len()
    }

    pub fn has_owned(&self, aura: OwnedAuraRef) -> bool {
        self.owned_auras.contains(&aura)
    }

    pub fn add_applied(&mut self, aura: AppliedAuraRef) {
        if !self.applied_auras.contains(&aura) {
            self.applied_auras.push(aura);
        }
    }

    pub fn register_applied_aura_type_like_cpp(&mut self, aura: AppliedAuraRef, aura_type: i32) {
        self.add_applied(aura);
        let typed_auras = self.applied_aura_types.entry(aura_type).or_default();
        if !typed_auras.contains(&aura) {
            typed_auras.push(aura);
        }
    }

    pub fn remove_applied(&mut self, aura: AppliedAuraRef) -> bool {
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
        if self.has_aura_spell_like_cpp(spell_id) {
            return false;
        }

        let slot = self
            .applied_auras
            .iter()
            .filter(|aura| aura.caster_guid == caster_guid)
            .map(|aura| aura.slot)
            .max()
            .and_then(|slot| slot.checked_add(1))
            .unwrap_or(0);
        let owned = OwnedAuraRef::new(spell_id, caster_guid, None);
        let applied = AppliedAuraRef::new(spell_id, caster_guid, slot, 0);
        self.add_owned(owned);
        self.add_applied(applied);
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

    pub fn set_visible(&mut self, slot: u8, aura: AuraRef) {
        self.visible_auras.insert(slot, aura);
        self.visible_auras_to_update.insert(slot);
    }

    pub fn clear_visible(&mut self, slot: u8) -> Option<AuraRef> {
        self.visible_auras_to_update.remove(&slot);
        self.visible_auras.remove(&slot)
    }

    pub fn mark_removed(&mut self, aura: AuraRef) {
        self.removed_auras.push(aura);
        self.removed_auras_count = self.removed_auras_count.saturating_add(1);
    }

    pub fn set_aura_death_policy_like_cpp(
        &mut self,
        aura: AuraRef,
        passive: bool,
        death_persistent: bool,
    ) {
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
            self.unapply_aura(*aura, 1);
        }
        removed
    }

    pub fn modify_aura_state(&mut self, flag: u8, apply: bool) {
        if flag == AURA_STATE_NONE {
            return;
        }
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

fn next_diminishing_level(
    current: DiminishingLevel,
    max_level: DiminishingLevel,
) -> DiminishingLevel {
    let next = match current {
        DiminishingLevel::Level1 => DiminishingLevel::Level2,
        DiminishingLevel::Level2 => DiminishingLevel::Level3,
        DiminishingLevel::Level3 => DiminishingLevel::Immune,
        DiminishingLevel::Immune => DiminishingLevel::TauntImmune,
        DiminishingLevel::TauntImmune => DiminishingLevel::TauntImmune,
    };
    next.min(max_level)
}

/// Trinity-compatible current spell slots represented in RustyCore state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CurrentSpellSlot {
    Melee = 0,
    Generic = 1,
    Channeled = 2,
    Autorepeat = 3,
}

pub const CURRENT_FIRST_NON_MELEE_SPELL: u8 = 1;
pub const CURRENT_MAX_SPELL: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentSpellRef {
    pub spell_id: u32,
    pub caster_guid: Option<ObjectGuid>,
    pub cast_id: Option<ObjectGuid>,
    pub cast_time_ms: u32,
    pub state: SpellState,
    pub interruptible: bool,
    pub allow_actions_during_channel: bool,
    pub delay_combat_timer_during_cast: bool,
}

impl CurrentSpellRef {
    pub const fn new(
        spell_id: u32,
        caster_guid: Option<ObjectGuid>,
        cast_id: Option<ObjectGuid>,
    ) -> Self {
        Self {
            spell_id,
            caster_guid,
            cast_id,
            cast_time_ms: 0,
            state: SpellState::None,
            interruptible: true,
            allow_actions_during_channel: false,
            delay_combat_timer_during_cast: false,
        }
    }

    pub const fn with_cast_time_ms(mut self, cast_time_ms: u32) -> Self {
        self.cast_time_ms = cast_time_ms;
        self
    }

    pub const fn with_state(mut self, state: SpellState) -> Self {
        self.state = state;
        self
    }

    pub const fn with_interruptible(mut self, interruptible: bool) -> Self {
        self.interruptible = interruptible;
        self
    }

    pub const fn with_allow_actions_during_channel(
        mut self,
        allow_actions_during_channel: bool,
    ) -> Self {
        self.allow_actions_during_channel = allow_actions_during_channel;
        self
    }

    pub const fn with_delay_combat_timer_during_cast(
        mut self,
        delay_combat_timer_during_cast: bool,
    ) -> Self {
        self.delay_combat_timer_during_cast = delay_combat_timer_during_cast;
        self
    }
}

pub const MAX_SPELL_SCHOOL: usize = 7;
pub const INFINITY_COOLDOWN_DELAY_MS: u64 = 30 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCooldown {
    pub spell_id: u32,
    pub item_id: u32,
    pub cooldown_end_ms: u64,
    pub category_id: u32,
    pub category_end_ms: u64,
    pub on_hold: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellChargeState {
    pub recharge_start_ms: u64,
    pub recharge_end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellHistory {
    pub cooldowns: HashMap<u32, SpellCooldown>,
    pub cooldowns_before_duel: HashMap<u32, SpellCooldown>,
    pub category_cooldowns: HashMap<u32, u32>,
    pub school_lockouts: [u64; MAX_SPELL_SCHOOL],
    pub charges: HashMap<u32, VecDeque<SpellChargeState>>,
    pub global_cooldowns: HashMap<u32, u64>,
}

impl SpellHistory {
    pub fn start_cooldown(
        &mut self,
        now_ms: u64,
        spell_id: u32,
        item_id: u32,
        cooldown_ms: u64,
        category_id: u32,
        category_cooldown_ms: u64,
        on_hold: bool,
    ) -> bool {
        let (cooldown_end_ms, category_end_ms) = if on_hold {
            (
                if cooldown_ms > 0 {
                    now_ms + INFINITY_COOLDOWN_DELAY_MS
                } else if category_cooldown_ms > 0 {
                    now_ms + INFINITY_COOLDOWN_DELAY_MS
                } else {
                    now_ms
                },
                if category_cooldown_ms > 0 {
                    now_ms + INFINITY_COOLDOWN_DELAY_MS
                } else {
                    now_ms
                },
            )
        } else {
            (
                if cooldown_ms > 0 {
                    now_ms + cooldown_ms
                } else if category_cooldown_ms > 0 {
                    now_ms + category_cooldown_ms
                } else {
                    now_ms
                },
                if category_cooldown_ms > 0 {
                    now_ms + category_cooldown_ms
                } else {
                    now_ms
                },
            )
        };

        if cooldown_end_ms == now_ms && category_end_ms == now_ms {
            return false;
        }

        self.add_cooldown(
            spell_id,
            item_id,
            cooldown_end_ms,
            category_id,
            category_end_ms,
            on_hold,
        )
    }

    pub fn set_cooldown(&mut self, spell_id: u32, started_at_ms: u64, duration_ms: u32) {
        self.start_cooldown(
            started_at_ms,
            spell_id,
            0,
            u64::from(duration_ms),
            0,
            0,
            false,
        );
    }

    pub fn add_cooldown(
        &mut self,
        spell_id: u32,
        item_id: u32,
        cooldown_end_ms: u64,
        category_id: u32,
        category_end_ms: u64,
        on_hold: bool,
    ) -> bool {
        let should_replace = self.cooldowns.get(&spell_id).is_none_or(|current| {
            cooldown_end_ms > current.cooldown_end_ms
                || category_end_ms > current.category_end_ms
                || on_hold
        });

        if !should_replace {
            return false;
        }

        self.cooldowns.insert(
            spell_id,
            SpellCooldown {
                spell_id,
                item_id,
                cooldown_end_ms,
                category_id,
                category_end_ms,
                on_hold,
            },
        );

        if category_id != 0 {
            self.category_cooldowns.insert(category_id, spell_id);
        }

        true
    }

    pub fn cooldown(&self, spell_id: u32) -> Option<SpellCooldown> {
        self.cooldowns.get(&spell_id).copied()
    }

    pub fn has_cooldown(&self, spell_id: u32, category_id: u32, now_ms: u64) -> bool {
        self.cooldowns
            .get(&spell_id)
            .is_some_and(|cooldown| cooldown.on_hold || cooldown.cooldown_end_ms > now_ms)
            || (category_id != 0
                && self
                    .category_cooldowns
                    .get(&category_id)
                    .and_then(|spell_id| self.cooldowns.get(spell_id))
                    .is_some_and(|cooldown| cooldown.on_hold || cooldown.category_end_ms > now_ms))
    }

    pub fn remaining_cooldown_ms(&self, spell_id: u32, category_id: u32, now_ms: u64) -> u64 {
        if let Some(cooldown) = self.cooldowns.get(&spell_id) {
            return cooldown.cooldown_end_ms.saturating_sub(now_ms);
        }

        self.remaining_category_cooldown_ms(category_id, now_ms)
    }

    pub fn remaining_category_cooldown_ms(&self, category_id: u32, now_ms: u64) -> u64 {
        self.category_cooldowns
            .get(&category_id)
            .and_then(|spell_id| self.cooldowns.get(spell_id))
            .map_or(0, |cooldown| {
                cooldown.category_end_ms.saturating_sub(now_ms)
            })
    }

    pub fn modify_cooldown(
        &mut self,
        spell_id: u32,
        cooldown_delta_ms: i64,
        without_category_cooldown: bool,
        now_ms: u64,
    ) -> bool {
        if cooldown_delta_ms == 0 {
            return false;
        }

        let Some(cooldown) = self.cooldowns.get_mut(&spell_id) else {
            return false;
        };

        cooldown.cooldown_end_ms = apply_ms_delta(cooldown.cooldown_end_ms, cooldown_delta_ms);
        if cooldown.category_id != 0 {
            if !without_category_cooldown {
                cooldown.category_end_ms =
                    apply_ms_delta(cooldown.category_end_ms, cooldown_delta_ms);
            }
            if cooldown.cooldown_end_ms < cooldown.category_end_ms {
                cooldown.cooldown_end_ms = cooldown.category_end_ms;
            }
        }

        if cooldown.cooldown_end_ms <= now_ms && !cooldown.on_hold {
            self.clear_cooldown(spell_id);
        }

        true
    }

    pub fn clear_cooldown(&mut self, spell_id: u32) -> bool {
        let Some(cooldown) = self.cooldowns.remove(&spell_id) else {
            return false;
        };
        if cooldown.category_id != 0 {
            self.category_cooldowns.remove(&cooldown.category_id);
        }
        true
    }

    pub fn reset_all_cooldowns(&mut self) {
        self.cooldowns.clear();
        self.category_cooldowns.clear();
    }

    pub fn set_charges(
        &mut self,
        charge_category_id: u32,
        charges: u8,
        started_at_ms: u64,
        recharge_ms: u32,
    ) {
        let queue = self.charges.entry(charge_category_id).or_default();
        queue.clear();
        let mut start = started_at_ms;
        for _ in 0..charges {
            let end = start + u64::from(recharge_ms);
            queue.push_back(SpellChargeState {
                recharge_start_ms: start,
                recharge_end_ms: end,
            });
            start = end;
        }
    }

    pub fn charges(&self, charge_category_id: u32) -> Option<&VecDeque<SpellChargeState>> {
        self.charges.get(&charge_category_id)
    }

    pub fn consumed_charges(&self, charge_category_id: u32) -> u8 {
        self.charges
            .get(&charge_category_id)
            .map_or(0, |charges| charges.len().min(u8::MAX as usize) as u8)
    }

    pub fn has_charge(&self, charge_category_id: u32, max_charges: i32) -> bool {
        charge_category_id == 0
            || max_charges <= 0
            || self
                .charges
                .get(&charge_category_id)
                .is_none_or(|charges| charges.len() < max_charges as usize)
    }

    pub fn consume_charge(
        &mut self,
        charge_category_id: u32,
        now_ms: u64,
        recovery_ms: u32,
        max_charges: i32,
    ) -> bool {
        if charge_category_id == 0 || recovery_ms == 0 || max_charges <= 0 {
            return false;
        }

        let queue = self.charges.entry(charge_category_id).or_default();
        let recharge_start_ms = queue.back().map_or(now_ms, |charge| charge.recharge_end_ms);
        queue.push_back(SpellChargeState {
            recharge_start_ms,
            recharge_end_ms: recharge_start_ms + u64::from(recovery_ms),
        });
        true
    }

    pub fn modify_charge_recovery_time(
        &mut self,
        charge_category_id: u32,
        cooldown_delta_ms: i64,
        now_ms: u64,
    ) -> bool {
        let Some(queue) = self.charges.get_mut(&charge_category_id) else {
            return false;
        };
        if queue.is_empty() {
            return false;
        }

        for charge in queue.iter_mut() {
            charge.recharge_start_ms = apply_ms_delta(charge.recharge_start_ms, cooldown_delta_ms);
            charge.recharge_end_ms = apply_ms_delta(charge.recharge_end_ms, cooldown_delta_ms);
        }

        while queue
            .front()
            .is_some_and(|charge| charge.recharge_end_ms < now_ms)
        {
            queue.pop_front();
        }

        true
    }

    pub fn restore_charge(&mut self, charge_category_id: u32) -> bool {
        self.charges
            .get_mut(&charge_category_id)
            .and_then(VecDeque::pop_back)
            .is_some()
    }

    pub fn clear_charges(&mut self, charge_category_id: u32) -> bool {
        self.charges.remove(&charge_category_id).is_some()
    }

    pub fn reset_all_charges(&mut self) {
        self.charges.clear();
    }

    pub fn lock_spell_school(&mut self, school_mask: u32, now_ms: u64, lockout_ms: u64) {
        let lockout_end = now_ms + lockout_ms;
        for school in 0..MAX_SPELL_SCHOOL {
            if (school_mask & (1 << school)) != 0 {
                self.school_lockouts[school] = lockout_end;
            }
        }
    }

    pub fn is_school_locked(&self, school_mask: u32, now_ms: u64) -> bool {
        (0..MAX_SPELL_SCHOOL).any(|school| {
            (school_mask & (1 << school)) != 0 && self.school_lockouts[school] > now_ms
        })
    }

    pub fn add_global_cooldown(
        &mut self,
        recovery_category_id: u32,
        now_ms: u64,
        duration_ms: u64,
    ) {
        self.global_cooldowns
            .insert(recovery_category_id, now_ms + duration_ms);
    }

    pub fn has_global_cooldown(&self, recovery_category_id: u32, now_ms: u64) -> bool {
        self.global_cooldowns
            .get(&recovery_category_id)
            .is_some_and(|end_ms| *end_ms > now_ms)
    }

    pub fn cancel_global_cooldown(&mut self, recovery_category_id: u32) {
        self.global_cooldowns.insert(recovery_category_id, 0);
    }

    pub fn remaining_global_cooldown_ms(&self, recovery_category_id: u32, now_ms: u64) -> u64 {
        self.global_cooldowns
            .get(&recovery_category_id)
            .map_or(0, |end_ms| end_ms.saturating_sub(now_ms))
    }

    pub fn save_cooldown_state_before_duel(&mut self) {
        self.cooldowns_before_duel = self.cooldowns.clone();
    }

    pub fn restore_cooldown_state_after_duel(&mut self) {
        self.cooldowns = self.cooldowns_before_duel.clone();
        self.category_cooldowns.clear();
        for (spell_id, cooldown) in &self.cooldowns {
            if cooldown.category_id != 0 {
                self.category_cooldowns
                    .insert(cooldown.category_id, *spell_id);
            }
        }
    }

    pub fn update(&mut self, now_ms: u64) {
        self.category_cooldowns.retain(|_, spell_id| {
            self.cooldowns
                .get(spell_id)
                .is_some_and(|cooldown| cooldown.on_hold || cooldown.category_end_ms >= now_ms)
        });

        let expired: Vec<u32> = self
            .cooldowns
            .iter()
            .filter_map(|(spell_id, cooldown)| {
                (!cooldown.on_hold && cooldown.cooldown_end_ms < now_ms).then_some(*spell_id)
            })
            .collect();
        for spell_id in expired {
            self.clear_cooldown(spell_id);
        }

        for queue in self.charges.values_mut() {
            while queue
                .front()
                .is_some_and(|charge| charge.recharge_end_ms <= now_ms)
            {
                queue.pop_front();
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

fn apply_ms_delta(value: u64, delta: i64) -> u64 {
    if delta.is_negative() {
        value.saturating_sub(delta.unsigned_abs())
    } else {
        value.saturating_add(delta as u64)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellSubsystem {
    pub current_spells: HashMap<CurrentSpellSlot, CurrentSpellRef>,
    pub history: SpellHistory,
}

impl SpellSubsystem {
    pub fn set_current_spell(&mut self, slot: CurrentSpellSlot, spell: CurrentSpellRef) {
        self.current_spells.insert(slot, spell);
    }

    pub fn current_spell(&self, slot: CurrentSpellSlot) -> Option<CurrentSpellRef> {
        self.current_spells.get(&slot).copied()
    }

    pub fn clear_current_spell(&mut self, slot: CurrentSpellSlot) -> Option<CurrentSpellRef> {
        self.current_spells.remove(&slot)
    }

    pub fn clear_current_spells(&mut self) {
        self.current_spells.clear();
    }

    pub fn find_current_spell_by_spell_id(&self, spell_id: u32) -> Option<CurrentSpellRef> {
        self.current_spells
            .values()
            .find(|spell| spell.spell_id == spell_id)
            .copied()
    }
}

pub const THREAT_UPDATE_INTERVAL_MS: u32 = 1_000;
pub const PVP_COMBAT_TIMEOUT_MS: u32 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ThreatOnlineState {
    Offline = 0,
    Suppressed = 1,
    Online = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ThreatTauntState {
    Detaunt = 0,
    None = 1,
    Taunt = 2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreatReferenceState {
    pub base_amount: f32,
    pub temp_modifier: i32,
    pub online_state: ThreatOnlineState,
    pub taunt_state: ThreatTauntState,
}

impl Default for ThreatReferenceState {
    fn default() -> Self {
        Self {
            base_amount: 0.0,
            temp_modifier: 0,
            online_state: ThreatOnlineState::Offline,
            taunt_state: ThreatTauntState::None,
        }
    }
}

impl ThreatReferenceState {
    pub fn threat(&self) -> f32 {
        (self.base_amount + self.temp_modifier as f32).max(0.0)
    }

    pub const fn is_online(&self) -> bool {
        matches!(self.online_state, ThreatOnlineState::Online)
    }

    pub const fn is_available(&self) -> bool {
        !matches!(self.online_state, ThreatOnlineState::Offline)
    }

    pub const fn is_offline(&self) -> bool {
        matches!(self.online_state, ThreatOnlineState::Offline)
    }

    pub const fn is_suppressed(&self) -> bool {
        matches!(self.online_state, ThreatOnlineState::Suppressed)
    }

    pub const fn is_taunting(&self) -> bool {
        matches!(self.taunt_state, ThreatTauntState::Taunt)
    }

    pub const fn is_detaunted(&self) -> bool {
        matches!(self.taunt_state, ThreatTauntState::Detaunt)
    }

    pub fn add_threat(&mut self, amount: f32) {
        if amount != 0.0 {
            self.base_amount = (self.base_amount + amount).max(0.0);
        }
    }

    pub fn scale_threat(&mut self, factor: f32) {
        self.base_amount *= factor.max(0.0);
    }

    pub fn modify_threat_by_percent(&mut self, percent: i32) {
        if percent != 0 {
            self.scale_threat(0.01 * (100 + percent) as f32);
        }
    }

    pub fn set_taunt_state(&mut self, taunt_state: ThreatTauntState) {
        self.taunt_state = taunt_state;
    }

    pub fn set_online_state(&mut self, online_state: ThreatOnlineState) {
        self.online_state = online_state;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatReferenceState {
    pub pvp: bool,
    pub suppressed_for_owner: bool,
    pub timeout_ms: Option<u32>,
}

impl CombatReferenceState {
    pub const fn pve() -> Self {
        Self {
            pvp: false,
            suppressed_for_owner: false,
            timeout_ms: None,
        }
    }

    pub const fn pvp() -> Self {
        Self {
            pvp: true,
            suppressed_for_owner: false,
            timeout_ms: Some(PVP_COMBAT_TIMEOUT_MS),
        }
    }

    pub fn refresh(&mut self) {
        self.suppressed_for_owner = false;
        if self.pvp {
            self.timeout_ms = Some(PVP_COMBAT_TIMEOUT_MS);
        }
    }

    pub fn suppress_for_owner(&mut self) {
        self.suppressed_for_owner = true;
    }

    pub fn update_pvp_timer(&mut self, diff_ms: u32) -> bool {
        if !self.pvp {
            return true;
        }
        let Some(timer) = self.timeout_ms.as_mut() else {
            return true;
        };
        if *timer <= diff_ms {
            return false;
        }
        *timer -= diff_ms;
        true
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatBeginContextLikeCpp {
    pub same_unit: bool,
    pub attacker_in_world: bool,
    pub victim_in_world: bool,
    pub attacker_alive: bool,
    pub victim_alive: bool,
    pub same_map: bool,
    pub same_phase: bool,
    pub attacker_unit_state: u32,
    pub victim_unit_state: u32,
    pub attacker_combat_disallowed: bool,
    pub victim_combat_disallowed: bool,
    pub relation_represented: bool,
    pub attacker_is_friendly_to_victim: bool,
    pub victim_is_friendly_to_attacker: bool,
    pub attacker_or_owner_player_is_game_master: bool,
    pub victim_or_owner_player_is_game_master: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CombatSubsystem {
    pub threat: HashMap<ObjectGuid, f32>,
    pub threat_refs: HashMap<ObjectGuid, ThreatReferenceState>,
    pub threatened_by_me: HashMap<ObjectGuid, ThreatReferenceState>,
    pub current_victim_guid: Option<ObjectGuid>,
    pub fixate_guid: Option<ObjectGuid>,
    pub owner_can_have_threat_list: bool,
    pub need_client_update: bool,
    pub threat_update_timer_ms: u32,
    pub pve_refs: HashMap<ObjectGuid, CombatReferenceState>,
    pub pvp_refs: HashMap<ObjectGuid, CombatReferenceState>,
    pub attackers: HashSet<ObjectGuid>,
    pub attacking_guid: Option<ObjectGuid>,
    pub last_damaged_target_guid: Option<ObjectGuid>,
    pub combat_disallowed: bool,
}

impl Default for CombatSubsystem {
    fn default() -> Self {
        Self {
            threat: HashMap::new(),
            threat_refs: HashMap::new(),
            threatened_by_me: HashMap::new(),
            current_victim_guid: None,
            fixate_guid: None,
            owner_can_have_threat_list: false,
            need_client_update: false,
            threat_update_timer_ms: THREAT_UPDATE_INTERVAL_MS,
            pve_refs: HashMap::new(),
            pvp_refs: HashMap::new(),
            attackers: HashSet::new(),
            attacking_guid: None,
            last_damaged_target_guid: None,
            combat_disallowed: false,
        }
    }
}

impl CombatSubsystem {
    pub fn can_begin_combat_like_cpp(context: CombatBeginContextLikeCpp) -> bool {
        if context.same_unit {
            return false;
        }
        if !context.attacker_in_world || !context.victim_in_world {
            return false;
        }
        if !context.attacker_alive || !context.victim_alive {
            return false;
        }
        if !context.same_map {
            return false;
        }
        if !context.same_phase {
            return false;
        }
        if context.attacker_unit_state & UnitState::EVADE.bits() != 0
            || context.victim_unit_state & UnitState::EVADE.bits() != 0
        {
            return false;
        }
        if context.attacker_unit_state & UnitState::IN_FLIGHT.bits() != 0
            || context.victim_unit_state & UnitState::IN_FLIGHT.bits() != 0
        {
            return false;
        }
        if context.attacker_combat_disallowed || context.victim_combat_disallowed {
            return false;
        }
        if context.relation_represented
            && (context.attacker_is_friendly_to_victim || context.victim_is_friendly_to_attacker)
        {
            return false;
        }
        if context.attacker_or_owner_player_is_game_master
            || context.victim_or_owner_player_is_game_master
        {
            return false;
        }
        true
    }

    pub fn initialize_threat_list_capability(&mut self, can_have_threat_list: bool) {
        self.owner_can_have_threat_list = can_have_threat_list;
    }

    pub fn add_threat(&mut self, target: ObjectGuid, amount: f32) -> f32 {
        let threat_ref = self.threat_refs.entry(target).or_insert_with(|| {
            let mut threat_ref = ThreatReferenceState::default();
            threat_ref.set_online_state(ThreatOnlineState::Online);
            threat_ref
        });
        threat_ref.add_threat(amount);
        let value = threat_ref.threat();
        self.threat.insert(target, value);
        self.need_client_update = true;
        if self.current_victim_guid.is_none() && threat_ref.is_available() {
            self.current_victim_guid = Some(target);
        }
        value
    }

    pub fn set_threat(&mut self, target: ObjectGuid, value: f32) {
        let threat_ref = self.threat_refs.entry(target).or_insert_with(|| {
            let mut threat_ref = ThreatReferenceState::default();
            threat_ref.set_online_state(ThreatOnlineState::Online);
            threat_ref
        });
        threat_ref.base_amount = value.max(0.0);
        self.threat.insert(target, threat_ref.threat());
        self.need_client_update = true;
    }

    pub fn threat_value(&self, target: ObjectGuid) -> Option<f32> {
        self.threat_ref(target).map(ThreatReferenceState::threat)
    }

    pub fn remove_threat(&mut self, target: ObjectGuid) -> Option<f32> {
        let removed = self.threat_refs.remove(&target).map(|state| state.threat());
        self.threat.remove(&target);
        self.threatened_by_me.remove(&target);
        if self.current_victim_guid == Some(target) {
            self.current_victim_guid = None;
        }
        if self.fixate_guid == Some(target) {
            self.fixate_guid = None;
        }
        removed
    }

    pub fn clear_threat(&mut self) {
        self.threat.clear();
        self.threat_refs.clear();
        self.current_victim_guid = None;
        self.fixate_guid = None;
        self.need_client_update = true;
    }

    pub fn is_threatened_by(&self, target: ObjectGuid) -> bool {
        self.is_threatened_by_with_offline(target, false)
    }

    pub fn is_threatened_by_with_offline(&self, target: ObjectGuid, include_offline: bool) -> bool {
        self.threat_refs
            .get(&target)
            .is_some_and(|threat_ref| include_offline || threat_ref.is_available())
    }

    pub fn threat_ref(&self, target: ObjectGuid) -> Option<&ThreatReferenceState> {
        self.threat_refs.get(&target)
    }

    pub fn threat_ref_mut(&mut self, target: ObjectGuid) -> Option<&mut ThreatReferenceState> {
        self.threat_refs.get_mut(&target)
    }

    pub fn scale_threat(&mut self, target: ObjectGuid, factor: f32) -> Option<f32> {
        let threat_ref = self.threat_refs.get_mut(&target)?;
        threat_ref.scale_threat(factor);
        let value = threat_ref.threat();
        self.threat.insert(target, value);
        self.need_client_update = true;
        Some(value)
    }

    pub fn modify_threat_by_percent(&mut self, target: ObjectGuid, percent: i32) -> Option<f32> {
        let threat_ref = self.threat_refs.get_mut(&target)?;
        threat_ref.modify_threat_by_percent(percent);
        let value = threat_ref.threat();
        self.threat.insert(target, value);
        self.need_client_update = true;
        Some(value)
    }

    pub fn reset_all_threat(&mut self) {
        for (guid, threat_ref) in &mut self.threat_refs {
            threat_ref.scale_threat(0.0);
            self.threat.insert(*guid, threat_ref.threat());
        }
        self.need_client_update = true;
    }

    pub fn threat_list_size(&self) -> usize {
        self.threat_refs.len()
    }

    pub fn is_threat_list_empty(&self, include_offline: bool) -> bool {
        if include_offline {
            return self.threat_refs.is_empty();
        }
        self.threat_refs
            .values()
            .all(|threat_ref| !threat_ref.is_available())
    }

    pub fn sorted_threat_guids(&self) -> Vec<ObjectGuid> {
        let mut refs: Vec<_> = self
            .threat_refs
            .iter()
            .map(|(guid, threat_ref)| (*guid, *threat_ref))
            .collect();
        refs.sort_by(|(left_guid, left), (right_guid, right)| {
            compare_threat_refs(*right, *left).then_with(|| {
                (left_guid.high_value(), left_guid.low_value())
                    .cmp(&(right_guid.high_value(), right_guid.low_value()))
            })
        });
        refs.into_iter().map(|(guid, _)| guid).collect()
    }

    pub fn set_threat_online_state(
        &mut self,
        target: ObjectGuid,
        online_state: ThreatOnlineState,
    ) -> bool {
        let Some(threat_ref) = self.threat_refs.get_mut(&target) else {
            return false;
        };
        threat_ref.set_online_state(online_state);
        self.need_client_update = true;
        true
    }

    pub fn set_threat_taunt_state(
        &mut self,
        target: ObjectGuid,
        taunt_state: ThreatTauntState,
    ) -> bool {
        let Some(threat_ref) = self.threat_refs.get_mut(&target) else {
            return false;
        };
        threat_ref.set_taunt_state(taunt_state);
        self.need_client_update = true;
        true
    }

    pub fn fixate_target(&mut self, target: Option<ObjectGuid>) -> bool {
        if let Some(target) = target {
            if !self.threat_refs.contains_key(&target) {
                return false;
            }
            self.fixate_guid = Some(target);
        } else {
            self.fixate_guid = None;
        }
        true
    }

    pub fn reselect_victim(
        &mut self,
        old_victim_is_melee: bool,
        highest_is_melee: bool,
    ) -> Option<ObjectGuid> {
        if let Some(fixate) = self.fixate_guid {
            if self
                .threat_refs
                .get(&fixate)
                .is_some_and(ThreatReferenceState::is_available)
            {
                self.current_victim_guid = Some(fixate);
                return Some(fixate);
            }
        }

        let sorted = self.sorted_threat_guids();
        let highest_guid = sorted
            .into_iter()
            .find(|guid| self.threat_refs[guid].is_available())?;
        let Some(old_guid) = self.current_victim_guid else {
            self.current_victim_guid = Some(highest_guid);
            return Some(highest_guid);
        };
        let Some(old_ref) = self.threat_refs.get(&old_guid).copied() else {
            self.current_victim_guid = Some(highest_guid);
            return Some(highest_guid);
        };
        if !old_ref.is_available() || old_guid == highest_guid {
            self.current_victim_guid = Some(highest_guid);
            return Some(highest_guid);
        }

        let highest_ref = self.threat_refs[&highest_guid];
        let threshold = if old_victim_is_melee || highest_is_melee {
            1.1
        } else {
            1.3
        };
        if old_ref.threat() * threshold < highest_ref.threat() {
            self.current_victim_guid = Some(highest_guid);
        }
        self.current_victim_guid
    }

    pub fn put_threatened_by_me_ref(
        &mut self,
        owner: ObjectGuid,
        threat_ref: ThreatReferenceState,
    ) {
        self.threatened_by_me.insert(owner, threat_ref);
    }

    pub fn purge_threatened_by_me_ref(
        &mut self,
        owner: ObjectGuid,
    ) -> Option<ThreatReferenceState> {
        self.threatened_by_me.remove(&owner)
    }

    pub fn is_threatening_anyone(&self, include_offline: bool) -> bool {
        if include_offline {
            return !self.threatened_by_me.is_empty();
        }
        self.threatened_by_me
            .values()
            .any(ThreatReferenceState::is_available)
    }

    pub fn is_threatening_to(&self, owner: ObjectGuid, include_offline: bool) -> bool {
        self.threatened_by_me
            .get(&owner)
            .is_some_and(|threat_ref| include_offline || threat_ref.is_available())
    }

    pub fn set_in_combat_with(
        &mut self,
        target: ObjectGuid,
        both_player_controlled: bool,
        add_target_suppressed: bool,
    ) -> bool {
        if let Some(reference) = self.pvp_refs.get_mut(&target) {
            reference.refresh();
            return !reference.suppressed_for_owner;
        }
        if let Some(reference) = self.pve_refs.get_mut(&target) {
            reference.refresh();
            return !reference.suppressed_for_owner;
        }

        let mut reference = if both_player_controlled {
            CombatReferenceState::pvp()
        } else {
            CombatReferenceState::pve()
        };
        if add_target_suppressed {
            reference.suppress_for_owner();
        }
        if reference.pvp {
            self.pvp_refs.insert(target, reference);
        } else {
            self.pve_refs.insert(target, reference);
        }
        true
    }

    pub fn is_in_combat_with(&self, target: ObjectGuid) -> bool {
        self.pve_refs.contains_key(&target) || self.pvp_refs.contains_key(&target)
    }

    pub fn purge_combat_ref_like_cpp(&mut self, target: ObjectGuid) -> bool {
        let removed =
            self.pve_refs.remove(&target).is_some() || self.pvp_refs.remove(&target).is_some();
        if removed {
            self.remove_threat(target);
            self.threatened_by_me.remove(&target);
        }
        removed
    }

    pub fn has_pve_combat(&self) -> bool {
        self.pve_refs
            .values()
            .any(|reference| !reference.suppressed_for_owner)
    }

    pub fn has_pvp_combat(&self) -> bool {
        self.pvp_refs
            .values()
            .any(|reference| !reference.suppressed_for_owner)
    }

    pub fn has_combat(&self) -> bool {
        self.has_pve_combat() || self.has_pvp_combat()
    }

    pub fn suppress_pvp_combat(&mut self) {
        for reference in self.pvp_refs.values_mut() {
            reference.suppress_for_owner();
        }
    }

    pub fn update_pvp_combat(&mut self, diff_ms: u32) -> Vec<ObjectGuid> {
        let expired: Vec<_> = self
            .pvp_refs
            .iter_mut()
            .filter_map(|(guid, reference)| (!reference.update_pvp_timer(diff_ms)).then_some(*guid))
            .collect();
        for guid in &expired {
            self.pvp_refs.remove(guid);
        }
        expired
    }

    pub fn revalidate_combat_like_cpp(
        &mut self,
        mut can_begin_combat: impl FnMut(ObjectGuid, CombatReferenceState) -> bool,
    ) -> Vec<ObjectGuid> {
        let mut removed = Vec::new();
        self.pve_refs.retain(|guid, reference| {
            if can_begin_combat(*guid, *reference) {
                true
            } else {
                removed.push(*guid);
                false
            }
        });
        self.pvp_refs.retain(|guid, reference| {
            if can_begin_combat(*guid, *reference) {
                true
            } else {
                removed.push(*guid);
                false
            }
        });
        for guid in &removed {
            self.remove_threat(*guid);
            self.threatened_by_me.remove(guid);
        }
        removed
    }

    pub fn end_all_pve_combat(&mut self) {
        self.pve_refs.clear();
        self.clear_threat();
        self.threatened_by_me.clear();
    }

    pub fn end_all_pvp_combat(&mut self) {
        self.pvp_refs.clear();
    }

    pub fn end_all_combat(&mut self) {
        self.end_all_pve_combat();
        self.end_all_pvp_combat();
    }

    pub fn add_attacker(&mut self, attacker: ObjectGuid) -> bool {
        self.attackers.insert(attacker)
    }

    pub fn remove_attacker(&mut self, attacker: ObjectGuid) -> bool {
        self.attackers.remove(&attacker)
    }

    pub fn clear_attackers(&mut self) {
        self.attackers.clear();
        self.attacking_guid = None;
    }

    pub fn set_attacking(&mut self, victim: Option<ObjectGuid>) {
        self.attacking_guid = victim;
    }

    pub fn set_last_damaged_target_like_cpp(&mut self, target: Option<ObjectGuid>) {
        self.last_damaged_target_guid = target;
    }
}

fn compare_threat_refs(
    left: ThreatReferenceState,
    right: ThreatReferenceState,
) -> std::cmp::Ordering {
    left.online_state
        .cmp(&right.online_state)
        .then_with(|| left.taunt_state.cmp(&right.taunt_state))
        .then_with(|| {
            left.threat()
                .partial_cmp(&right.threat())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MovementGeneratorKind {
    Idle,
    Random,
    Waypoint,
    Confused,
    Chase,
    Home,
    Flight,
    Point,
    Fleeing,
    Distract,
    Assistance,
    AssistanceDistract,
    TimedFleeing,
    Follow,
    Rotate,
    Effect,
    SplineChain,
    Formation,
    Custom(u32),
}

impl MovementGeneratorKind {
    pub const fn trinity_id(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Random => 1,
            Self::Waypoint => 2,
            Self::Confused => 4,
            Self::Chase => 5,
            Self::Home => 6,
            Self::Flight => 7,
            Self::Point => 8,
            Self::Fleeing => 9,
            Self::Distract => 10,
            Self::Assistance => 11,
            Self::AssistanceDistract => 12,
            Self::TimedFleeing => 13,
            Self::Follow => 14,
            Self::Rotate => 15,
            Self::Effect => 16,
            Self::SplineChain => 17,
            Self::Formation => 18,
            Self::Custom(value) => value as u8,
        }
    }

    pub const fn from_trinity_id(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Idle),
            1 => Some(Self::Random),
            2 => Some(Self::Waypoint),
            3 | 19..=u8::MAX => None,
            4 => Some(Self::Confused),
            5 => Some(Self::Chase),
            6 => Some(Self::Home),
            7 => Some(Self::Flight),
            8 => Some(Self::Point),
            9 => Some(Self::Fleeing),
            10 => Some(Self::Distract),
            11 => Some(Self::Assistance),
            12 => Some(Self::AssistanceDistract),
            13 => Some(Self::TimedFleeing),
            14 => Some(Self::Follow),
            15 => Some(Self::Rotate),
            16 => Some(Self::Effect),
            17 => Some(Self::SplineChain),
            18 => Some(Self::Formation),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum MovementGeneratorMode {
    Default = 0,
    Override = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum MovementGeneratorPriority {
    None = 0,
    Normal = 1,
    Highest = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RotateDirection {
    Left = 0,
    Right = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MovementSlot {
    Default = 0,
    Active = 1,
}

pub const MOVEMENTGENERATOR_FLAG_NONE: u16 = 0x000;
pub const MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING: u16 = 0x001;
pub const MOVEMENTGENERATOR_FLAG_INITIALIZED: u16 = 0x002;
pub const MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING: u16 = 0x004;
pub const MOVEMENTGENERATOR_FLAG_INTERRUPTED: u16 = 0x008;
pub const MOVEMENTGENERATOR_FLAG_PAUSED: u16 = 0x010;
pub const MOVEMENTGENERATOR_FLAG_TIMED_PAUSED: u16 = 0x020;
pub const MOVEMENTGENERATOR_FLAG_DEACTIVATED: u16 = 0x040;
pub const MOVEMENTGENERATOR_FLAG_INFORM_ENABLED: u16 = 0x080;
pub const MOVEMENTGENERATOR_FLAG_FINALIZED: u16 = 0x100;
pub const MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH: u16 = 0x200;
pub const MOVEMENTGENERATOR_FLAG_TRANSITORY: u16 =
    MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING | MOVEMENTGENERATOR_FLAG_INTERRUPTED;
pub const MOTIONMASTER_FLAG_NONE: u8 = 0x0;
pub const MOTIONMASTER_FLAG_UPDATE: u8 = 0x1;
pub const MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING: u8 = 0x2;
pub const MOTIONMASTER_FLAG_INITIALIZATION_PENDING: u8 = 0x4;
pub const MOTIONMASTER_FLAG_INITIALIZING: u8 = 0x8;
pub const MOTIONMASTER_FLAG_DELAYED: u8 =
    MOTIONMASTER_FLAG_UPDATE | MOTIONMASTER_FLAG_INITIALIZATION_PENDING;
pub const EVENT_CHARGE: u32 = 1003;
pub const EVENT_JUMP: u32 = 1004;
pub const EVENT_CHARGE_PREPATH: u32 = 1005;
pub const EVENT_ASSIST_MOVE: u32 = 1009;
pub const CREATURE_FAMILY_ASSISTANCE_DELAY_MS_LIKE_CPP: u32 = 1_500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MovementGeneratorRef {
    pub kind: MovementGeneratorKind,
    pub mode: MovementGeneratorMode,
    pub priority: MovementGeneratorPriority,
    pub slot: MovementSlot,
    pub flags: u16,
    pub base_unit_state: u32,
    pub target_guid: Option<ObjectGuid>,
    pub movement_id: u32,
    pub duration_ms: Option<u32>,
    pub max_duration_ms: Option<u32>,
    pub elapsed_ms: u32,
    pub arrival_spell_id: u32,
    pub arrival_spell_target_guid: ObjectGuid,
    pub rotate_direction: Option<RotateDirection>,
}

impl MovementGeneratorRef {
    pub const fn new(kind: MovementGeneratorKind, slot: MovementSlot) -> Self {
        Self {
            kind,
            mode: MovementGeneratorMode::Default,
            priority: MovementGeneratorPriority::None,
            slot,
            flags: MOVEMENTGENERATOR_FLAG_NONE,
            base_unit_state: 0,
            target_guid: None,
            movement_id: 0,
            duration_ms: None,
            max_duration_ms: None,
            elapsed_ms: 0,
            arrival_spell_id: 0,
            arrival_spell_target_guid: ObjectGuid::EMPTY,
            rotate_direction: None,
        }
    }

    pub const fn with_mode(mut self, mode: MovementGeneratorMode) -> Self {
        self.mode = mode;
        self
    }

    pub const fn with_priority(mut self, priority: MovementGeneratorPriority) -> Self {
        self.priority = priority;
        self
    }

    pub const fn with_flags(mut self, flags: u16) -> Self {
        self.flags = flags;
        self
    }

    pub const fn with_base_unit_state(mut self, base_unit_state: u32) -> Self {
        self.base_unit_state = base_unit_state;
        self
    }

    pub const fn with_target_guid(mut self, target_guid: ObjectGuid) -> Self {
        self.target_guid = Some(target_guid);
        self
    }

    pub const fn with_movement_id(mut self, movement_id: u32) -> Self {
        self.movement_id = movement_id;
        self
    }

    pub const fn with_duration_ms(mut self, duration_ms: u32) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub const fn with_max_duration_ms(mut self, max_duration_ms: u32) -> Self {
        self.max_duration_ms = Some(max_duration_ms);
        self
    }

    pub const fn with_rotate_direction(mut self, direction: RotateDirection) -> Self {
        self.rotate_direction = Some(direction);
        self
    }

    pub const fn with_arrival_spell(mut self, spell_id: u32, target_guid: ObjectGuid) -> Self {
        self.arrival_spell_id = spell_id;
        self.arrival_spell_target_guid = target_guid;
        self
    }

    pub const fn has_flag(&self, flag: u16) -> bool {
        (self.flags & flag) != 0
    }

    pub fn initialize_for_motion_master_update_like_cpp(
        &mut self,
        context: MotionMasterUpdateContext,
    ) {
        match self.kind {
            MovementGeneratorKind::Idle => {
                self.initialize_idle_like_cpp();
            }
            MovementGeneratorKind::Point => {
                self.initialize_point_like_cpp(context.can_move);
            }
            MovementGeneratorKind::Rotate => {
                self.initialize_rotate_like_cpp();
            }
            MovementGeneratorKind::Distract => {
                self.initialize_distract_like_cpp(context.owner_is_standing);
            }
            MovementGeneratorKind::Effect => self.initialize_generic_like_cpp(),
            _ => {
                self.flags &= !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING
                    | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
                self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;
            }
        }
    }

    pub fn reset_for_motion_master_update_like_cpp(&mut self, context: MotionMasterUpdateContext) {
        match self.kind {
            MovementGeneratorKind::Point => {
                self.reset_point_like_cpp(context.can_move);
            }
            MovementGeneratorKind::Rotate => {
                self.reset_rotate_like_cpp();
            }
            MovementGeneratorKind::Distract => {
                self.reset_distract_like_cpp(context.owner_is_standing);
            }
            _ => self.initialize_for_motion_master_update_like_cpp(context),
        }
    }

    pub fn update_for_motion_master_like_cpp(
        &mut self,
        context: MotionMasterUpdateContext,
    ) -> bool {
        match self.kind {
            MovementGeneratorKind::Idle => self.update_idle_like_cpp(),
            MovementGeneratorKind::Point => {
                self.update_point_like_cpp(context.can_move, context.spline_finalized)
                    != PointMovementAction::Finished
            }
            MovementGeneratorKind::Rotate => {
                self.update_rotate_like_cpp(
                    context.owner_exists,
                    context.diff_ms,
                    context.current_orientation,
                )
                .keep_running
            }
            MovementGeneratorKind::Distract => {
                self.update_distract_like_cpp(context.owner_exists, context.diff_ms)
            }
            MovementGeneratorKind::Effect => self.update_generic_like_cpp(
                context.diff_ms,
                context.spline_cyclic,
                context.spline_finalized,
            ),
            _ => true,
        }
    }

    pub fn initialize_generic_like_cpp(&mut self) {
        if self.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED)
            && !self.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
        {
            self.flags &= !MOVEMENTGENERATOR_FLAG_DEACTIVATED;
            self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
            return;
        }

        self.flags &=
            !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;
        self.elapsed_ms = 0;
    }

    pub fn update_generic_like_cpp(
        &mut self,
        diff_ms: u32,
        spline_cyclic: bool,
        spline_finalized: bool,
    ) -> bool {
        if self.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED) {
            return false;
        }

        if !spline_cyclic {
            self.elapsed_ms = self.elapsed_ms.saturating_add(diff_ms);
        }

        if self
            .duration_ms
            .is_some_and(|duration_ms| self.elapsed_ms >= duration_ms)
            || spline_finalized
        {
            self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
            return false;
        }
        true
    }

    pub fn deactivate_generic_like_cpp(&mut self) {
        self.flags |= MOVEMENTGENERATOR_FLAG_DEACTIVATED;
    }

    pub fn finalize_generic_like_cpp(
        &mut self,
        movement_inform: bool,
    ) -> Option<GenericMovementInform> {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        if movement_inform && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED) {
            return Some(GenericMovementInform {
                kind: self.kind,
                movement_id: self.movement_id,
                arrival_spell_id: (self.arrival_spell_id != 0).then_some(self.arrival_spell_id),
                arrival_spell_target_guid: (self.arrival_spell_id != 0)
                    .then_some(self.arrival_spell_target_guid),
            });
        }
        None
    }

    pub fn initialize_point_like_cpp(&mut self, can_move: bool) -> PointMovementAction {
        self.flags &= !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING
            | MOVEMENTGENERATOR_FLAG_TRANSITORY
            | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;

        if self.movement_id == EVENT_CHARGE_PREPATH {
            return PointMovementAction::MarkRoamingMove;
        }

        if !can_move {
            self.flags |= MOVEMENTGENERATOR_FLAG_INTERRUPTED;
            return PointMovementAction::StopMoving;
        }

        PointMovementAction::LaunchSpline
    }

    pub fn reset_point_like_cpp(&mut self, can_move: bool) -> PointMovementAction {
        self.flags &= !(MOVEMENTGENERATOR_FLAG_TRANSITORY | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.initialize_point_like_cpp(can_move)
    }

    pub fn update_point_like_cpp(
        &mut self,
        can_move: bool,
        spline_finalized: bool,
    ) -> PointMovementAction {
        if self.movement_id == EVENT_CHARGE_PREPATH {
            if spline_finalized {
                self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
                return PointMovementAction::Finished;
            }
            return PointMovementAction::Continue;
        }

        if !can_move {
            self.flags |= MOVEMENTGENERATOR_FLAG_INTERRUPTED;
            return PointMovementAction::StopMovingAndContinue;
        }

        if (self.has_flag(MOVEMENTGENERATOR_FLAG_INTERRUPTED) && spline_finalized)
            || (self.has_flag(MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING) && !spline_finalized)
        {
            self.flags &=
                !(MOVEMENTGENERATOR_FLAG_INTERRUPTED | MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING);
            return PointMovementAction::RelaunchSpline;
        }

        if spline_finalized {
            self.flags &= !MOVEMENTGENERATOR_FLAG_TRANSITORY;
            self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
            return PointMovementAction::Finished;
        }

        PointMovementAction::Continue
    }

    pub fn deactivate_point_like_cpp(&mut self) -> PointMovementAction {
        self.flags |= MOVEMENTGENERATOR_FLAG_DEACTIVATED;
        PointMovementAction::ClearRoamingMove
    }

    pub fn finalize_point_like_cpp(
        &mut self,
        active: bool,
        movement_inform: bool,
    ) -> PointMovementFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        PointMovementFinalize {
            clear_roaming_move: active,
            inform: (movement_inform && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED))
                .then_some(PointMovementInform {
                    kind: MovementGeneratorKind::Point,
                    movement_id: if self.movement_id == EVENT_CHARGE_PREPATH {
                        EVENT_CHARGE
                    } else {
                        self.movement_id
                    },
                }),
        }
    }

    pub fn finalize_assistance_like_cpp(
        &mut self,
        active: bool,
        movement_inform: bool,
        owner_is_creature: bool,
        owner_is_alive: bool,
    ) -> AssistanceMovementFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        let can_inform = movement_inform
            && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED)
            && owner_is_creature;
        AssistanceMovementFinalize {
            clear_roaming_move: active,
            set_no_call_assistance: can_inform.then_some(false),
            call_assistance: can_inform,
            seek_assistance_distract_ms: (can_inform && owner_is_alive)
                .then_some(CREATURE_FAMILY_ASSISTANCE_DELAY_MS_LIKE_CPP),
        }
    }

    pub fn finalize_assistance_distract_like_cpp(
        &mut self,
        movement_inform: bool,
        owner_is_creature: bool,
    ) -> AssistanceDistractFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        AssistanceDistractFinalize {
            set_react_aggressive: movement_inform
                && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED)
                && owner_is_creature,
        }
    }

    pub fn initialize_idle_like_cpp(&self) -> IdleMovementAction {
        IdleMovementAction::StopMoving
    }

    pub fn reset_idle_like_cpp(&self) -> IdleMovementAction {
        IdleMovementAction::StopMoving
    }

    pub fn update_idle_like_cpp(&self) -> bool {
        true
    }

    pub fn finalize_idle_like_cpp(&mut self) {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
    }

    pub fn initialize_rotate_like_cpp(&mut self) -> IdleMovementAction {
        self.flags &=
            !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;
        IdleMovementAction::StopMoving
    }

    pub fn reset_rotate_like_cpp(&mut self) -> IdleMovementAction {
        self.flags &= !MOVEMENTGENERATOR_FLAG_DEACTIVATED;
        self.initialize_rotate_like_cpp()
    }

    pub fn update_rotate_like_cpp(
        &mut self,
        owner_exists: bool,
        diff_ms: u32,
        current_orientation: f32,
    ) -> RotateMovementUpdate {
        if !owner_exists {
            return RotateMovementUpdate {
                keep_running: false,
                facing_angle: None,
            };
        }

        let max_duration_ms = self.max_duration_ms.unwrap_or(0);
        let direction = self.rotate_direction.unwrap_or(RotateDirection::Left);
        let facing_angle = if max_duration_ms == 0 {
            current_orientation
        } else {
            let sign = match direction {
                RotateDirection::Left => 1.0,
                RotateDirection::Right => -1.0,
            };
            (current_orientation
                + (diff_ms as f32 * std::f32::consts::TAU / max_duration_ms as f32) * sign)
                .clamp(0.0, std::f32::consts::TAU)
        };

        let remaining = self.duration_ms.unwrap_or(0);
        if remaining > diff_ms {
            self.duration_ms = Some(remaining - diff_ms);
            RotateMovementUpdate {
                keep_running: true,
                facing_angle: Some(facing_angle),
            }
        } else {
            self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
            RotateMovementUpdate {
                keep_running: false,
                facing_angle: Some(facing_angle),
            }
        }
    }

    pub fn deactivate_timed_idle_like_cpp(&mut self) {
        self.flags |= MOVEMENTGENERATOR_FLAG_DEACTIVATED;
    }

    pub fn finalize_rotate_like_cpp(
        &mut self,
        movement_inform: bool,
        owner_is_creature: bool,
    ) -> RotateMovementFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        RotateMovementFinalize {
            inform: (movement_inform && owner_is_creature).then_some(PointMovementInform {
                kind: MovementGeneratorKind::Rotate,
                movement_id: self.movement_id,
            }),
        }
    }

    pub fn initialize_distract_like_cpp(
        &mut self,
        owner_is_standing: bool,
    ) -> DistractMovementAction {
        self.flags &=
            !(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        self.flags |= MOVEMENTGENERATOR_FLAG_INITIALIZED;
        DistractMovementAction {
            stand_up: !owner_is_standing,
            launch_facing_spline: true,
        }
    }

    pub fn reset_distract_like_cpp(&mut self, owner_is_standing: bool) -> DistractMovementAction {
        self.flags &= !MOVEMENTGENERATOR_FLAG_DEACTIVATED;
        self.initialize_distract_like_cpp(owner_is_standing)
    }

    pub fn update_distract_like_cpp(&mut self, owner_exists: bool, diff_ms: u32) -> bool {
        if !owner_exists {
            return false;
        }

        let remaining = self.duration_ms.unwrap_or(0);
        if diff_ms > remaining {
            self.flags |= MOVEMENTGENERATOR_FLAG_INFORM_ENABLED;
            return false;
        }

        self.duration_ms = Some(remaining - diff_ms);
        true
    }

    pub fn finalize_distract_like_cpp(
        &mut self,
        movement_inform: bool,
        owner_is_creature: bool,
    ) -> DistractMovementFinalize {
        self.flags |= MOVEMENTGENERATOR_FLAG_FINALIZED;
        DistractMovementFinalize {
            set_home_orientation: movement_inform
                && self.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED)
                && owner_is_creature,
        }
    }
}

fn initialize_or_reset_for_motion_master_update_like_cpp(
    generator: &mut MovementGeneratorRef,
    context: MotionMasterUpdateContext,
) {
    if generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING) {
        generator.initialize_for_motion_master_update_like_cpp(context);
    }
    if generator.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED) {
        generator.reset_for_motion_master_update_like_cpp(context);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericMovementInform {
    pub kind: MovementGeneratorKind,
    pub movement_id: u32,
    pub arrival_spell_id: Option<u32>,
    pub arrival_spell_target_guid: Option<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointMovementAction {
    Continue,
    MarkRoamingMove,
    LaunchSpline,
    RelaunchSpline,
    StopMoving,
    StopMovingAndContinue,
    ClearRoamingMove,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointMovementInform {
    pub kind: MovementGeneratorKind,
    pub movement_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointMovementFinalize {
    pub clear_roaming_move: bool,
    pub inform: Option<PointMovementInform>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssistanceMovementFinalize {
    pub clear_roaming_move: bool,
    pub set_no_call_assistance: Option<bool>,
    pub call_assistance: bool,
    pub seek_assistance_distract_ms: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssistanceDistractFinalize {
    pub set_react_aggressive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekAssistancePlan {
    pub attack_stop: bool,
    pub cast_stop: bool,
    pub do_not_reacquire_spell_focus_target: bool,
    pub set_react_passive: bool,
    pub generator_added: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleMovementAction {
    StopMoving,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotateMovementUpdate {
    pub keep_running: bool,
    pub facing_angle: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotateMovementFinalize {
    pub inform: Option<PointMovementInform>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistractMovementAction {
    pub stand_up: bool,
    pub launch_facing_spline: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistractMovementFinalize {
    pub set_home_orientation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFallPlan {
    Noop,
    PlayerFallInfo,
    SplineStarted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveSplineState {
    pub enabled: bool,
    pub finalized: bool,
    pub cyclic: bool,
    pub on_transport: bool,
    pub spline_id: u32,
    pub progress_ms: u32,
    pub duration_ms: u32,
    pub velocity: Option<u32>,
    pub final_destination: Option<(i32, i32, i32)>,
    pub current_destination: Option<(i32, i32, i32)>,
}

impl Default for MoveSplineState {
    fn default() -> Self {
        Self {
            enabled: false,
            finalized: true,
            cyclic: false,
            on_transport: false,
            spline_id: 0,
            progress_ms: 0,
            duration_ms: 0,
            velocity: None,
            final_destination: None,
            current_destination: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MotionMasterDelayedActionType {
    Clear = 0,
    ClearSlot = 1,
    ClearMode = 2,
    ClearPriority = 3,
    Add = 4,
    Remove = 5,
    RemoveType = 6,
    Initialize = 7,
}

impl MotionMasterDelayedActionType {
    pub const fn trinity_id(self) -> u8 {
        self as u8
    }

    pub const fn from_trinity_id(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Clear),
            1 => Some(Self::ClearSlot),
            2 => Some(Self::ClearMode),
            3 => Some(Self::ClearPriority),
            4 => Some(Self::Add),
            5 => Some(Self::Remove),
            6 => Some(Self::RemoveType),
            7 => Some(Self::Initialize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionMasterDelayedActionPayload {
    Clear,
    ClearSlot(MovementSlot),
    ClearMode(MovementGeneratorMode),
    ClearPriority(MovementGeneratorPriority),
    Add(MovementGeneratorRef),
    Remove {
        kind: MovementGeneratorKind,
        slot: MovementSlot,
    },
    RemoveType {
        kind: MovementGeneratorKind,
        slot: MovementSlot,
    },
    Initialize,
}

impl MotionMasterDelayedActionPayload {
    pub const fn action_type(self) -> MotionMasterDelayedActionType {
        match self {
            Self::Clear => MotionMasterDelayedActionType::Clear,
            Self::ClearSlot(_) => MotionMasterDelayedActionType::ClearSlot,
            Self::ClearMode(_) => MotionMasterDelayedActionType::ClearMode,
            Self::ClearPriority(_) => MotionMasterDelayedActionType::ClearPriority,
            Self::Add(_) => MotionMasterDelayedActionType::Add,
            Self::Remove { .. } => MotionMasterDelayedActionType::Remove,
            Self::RemoveType { .. } => MotionMasterDelayedActionType::RemoveType,
            Self::Initialize => MotionMasterDelayedActionType::Initialize,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionMasterDelayedAction {
    pub payload: MotionMasterDelayedActionPayload,
    pub validator_passed: bool,
}

impl MotionMasterDelayedAction {
    pub const fn new(payload: MotionMasterDelayedActionPayload) -> Self {
        Self {
            payload,
            validator_passed: true,
        }
    }

    pub const fn with_validator(
        payload: MotionMasterDelayedActionPayload,
        validator_passed: bool,
    ) -> Self {
        Self {
            payload,
            validator_passed,
        }
    }

    pub const fn action_type(self) -> MotionMasterDelayedActionType {
        self.payload.action_type()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionMasterResolvedDelayedAction {
    pub action_type: MotionMasterDelayedActionType,
    pub executed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionMasterUpdateContext {
    pub diff_ms: u32,
    pub can_move: bool,
    pub owner_exists: bool,
    pub owner_is_standing: bool,
    pub spline_finalized: bool,
    pub spline_cyclic: bool,
    pub current_orientation: f32,
}

impl Default for MotionMasterUpdateContext {
    fn default() -> Self {
        Self {
            diff_ms: 0,
            can_move: true,
            owner_exists: true,
            owner_is_standing: true,
            spline_finalized: false,
            spline_cyclic: false,
            current_orientation: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotionMasterUpdateOutcome {
    Stalled,
    Empty,
    Updated {
        popped: Option<MovementGeneratorRef>,
        resolved_delayed_actions: Vec<MotionMasterResolvedDelayedAction>,
    },
}

/// Represented local evidence for C++ `MotionMaster::AddToWorld()`
/// (`MotionMaster.cpp:120-132`).
///
/// This preserves the C++ initialization-pending guard and flag transitions,
/// calls the existing represented `DirectInitialize`/delayed-action helpers,
/// and does not claim real movement-generator runtime, pathing, packets, or
/// owner/fanout behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionMasterAddToWorldOutcomeLikeCpp {
    pub had_initialization_pending: bool,
    pub entered_initializing: bool,
    pub direct_initialize_represented: bool,
    pub resolved_delayed_actions: Vec<MotionMasterResolvedDelayedAction>,
    pub exited_initializing: bool,
    pub flags_before: u8,
    pub flags_after: u8,
    pub current_generator_after: MovementGeneratorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionSubsystem {
    pub default_generator: MovementGeneratorRef,
    pub active_generators: Vec<MovementGeneratorRef>,
    pub current_generator: MovementGeneratorKind,
    pub base_unit_states: HashMap<u32, usize>,
    pub flags: u8,
    pub delayed_actions: Vec<MotionMasterDelayedAction>,
    pub paused: bool,
    pub stopped: bool,
    pub spline: MoveSplineState,
}

impl Default for MotionSubsystem {
    fn default() -> Self {
        let default_generator =
            MovementGeneratorRef::new(MovementGeneratorKind::Idle, MovementSlot::Default)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZED);
        Self {
            default_generator,
            active_generators: Vec::new(),
            current_generator: MovementGeneratorKind::Idle,
            base_unit_states: HashMap::new(),
            flags: MOTIONMASTER_FLAG_INITIALIZATION_PENDING,
            delayed_actions: Vec::new(),
            paused: false,
            stopped: false,
            spline: MoveSplineState::default(),
        }
    }
}

impl MotionSubsystem {
    pub const fn has_motion_master_flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    pub const fn should_delay_motion_master_action_like_cpp(&self) -> bool {
        self.has_motion_master_flag(MOTIONMASTER_FLAG_DELAYED)
    }

    pub fn push_delayed_action_like_cpp(&mut self, action_type: MotionMasterDelayedActionType) {
        let payload = match action_type {
            MotionMasterDelayedActionType::Clear => MotionMasterDelayedActionPayload::Clear,
            MotionMasterDelayedActionType::ClearSlot => {
                MotionMasterDelayedActionPayload::ClearSlot(MovementSlot::Active)
            }
            MotionMasterDelayedActionType::ClearMode => {
                MotionMasterDelayedActionPayload::ClearMode(MovementGeneratorMode::Default)
            }
            MotionMasterDelayedActionType::ClearPriority => {
                MotionMasterDelayedActionPayload::ClearPriority(MovementGeneratorPriority::Normal)
            }
            MotionMasterDelayedActionType::Add => MotionMasterDelayedActionPayload::Add(
                MovementGeneratorRef::new(MovementGeneratorKind::Idle, MovementSlot::Active),
            ),
            MotionMasterDelayedActionType::Remove => MotionMasterDelayedActionPayload::Remove {
                kind: MovementGeneratorKind::Idle,
                slot: MovementSlot::Active,
            },
            MotionMasterDelayedActionType::RemoveType => {
                MotionMasterDelayedActionPayload::RemoveType {
                    kind: MovementGeneratorKind::Idle,
                    slot: MovementSlot::Active,
                }
            }
            MotionMasterDelayedActionType::Initialize => {
                MotionMasterDelayedActionPayload::Initialize
            }
        };
        self.push_delayed_payload_like_cpp(payload);
    }

    pub fn push_delayed_action_with_validator_like_cpp(
        &mut self,
        action_type: MotionMasterDelayedActionType,
        validator_passed: bool,
    ) {
        self.delayed_actions
            .push(MotionMasterDelayedAction::with_validator(
                match action_type {
                    MotionMasterDelayedActionType::Clear => MotionMasterDelayedActionPayload::Clear,
                    MotionMasterDelayedActionType::ClearSlot => {
                        MotionMasterDelayedActionPayload::ClearSlot(MovementSlot::Active)
                    }
                    MotionMasterDelayedActionType::ClearMode => {
                        MotionMasterDelayedActionPayload::ClearMode(MovementGeneratorMode::Default)
                    }
                    MotionMasterDelayedActionType::ClearPriority => {
                        MotionMasterDelayedActionPayload::ClearPriority(
                            MovementGeneratorPriority::Normal,
                        )
                    }
                    MotionMasterDelayedActionType::Add => {
                        MotionMasterDelayedActionPayload::Add(MovementGeneratorRef::new(
                            MovementGeneratorKind::Idle,
                            MovementSlot::Active,
                        ))
                    }
                    MotionMasterDelayedActionType::Remove => {
                        MotionMasterDelayedActionPayload::Remove {
                            kind: MovementGeneratorKind::Idle,
                            slot: MovementSlot::Active,
                        }
                    }
                    MotionMasterDelayedActionType::RemoveType => {
                        MotionMasterDelayedActionPayload::RemoveType {
                            kind: MovementGeneratorKind::Idle,
                            slot: MovementSlot::Active,
                        }
                    }
                    MotionMasterDelayedActionType::Initialize => {
                        MotionMasterDelayedActionPayload::Initialize
                    }
                },
                validator_passed,
            ));
    }

    pub fn push_delayed_payload_like_cpp(&mut self, payload: MotionMasterDelayedActionPayload) {
        self.delayed_actions
            .push(MotionMasterDelayedAction::new(payload));
    }

    pub fn push_delayed_payload_with_validator_like_cpp(
        &mut self,
        payload: MotionMasterDelayedActionPayload,
        validator_passed: bool,
    ) {
        self.delayed_actions
            .push(MotionMasterDelayedAction::with_validator(
                payload,
                validator_passed,
            ));
    }

    pub fn resolve_delayed_actions_like_cpp(&mut self) -> Vec<MotionMasterResolvedDelayedAction> {
        self.delayed_actions
            .drain(..)
            .map(|action| MotionMasterResolvedDelayedAction {
                action_type: action.action_type(),
                executed: action.validator_passed,
            })
            .collect()
    }

    pub fn resolve_delayed_action_payloads_like_cpp(
        &mut self,
    ) -> Vec<MotionMasterResolvedDelayedAction> {
        let mut resolved = Vec::new();
        while !self.delayed_actions.is_empty() {
            let action = self.delayed_actions.remove(0);
            if action.validator_passed {
                self.apply_delayed_action_payload_like_cpp(action.payload);
            }
            resolved.push(MotionMasterResolvedDelayedAction {
                action_type: action.action_type(),
                executed: action.validator_passed,
            });
        }
        resolved
    }

    pub fn update_motion_master_like_cpp(
        &mut self,
        context: MotionMasterUpdateContext,
    ) -> MotionMasterUpdateOutcome {
        if self.has_motion_master_flag(
            MOTIONMASTER_FLAG_INITIALIZATION_PENDING | MOTIONMASTER_FLAG_INITIALIZING,
        ) {
            return MotionMasterUpdateOutcome::Stalled;
        }

        if self.is_empty() {
            return MotionMasterUpdateOutcome::Empty;
        }

        self.flags |= MOTIONMASTER_FLAG_UPDATE;

        if self.has_motion_master_flag(MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING)
            && self.current_slot() == MovementSlot::Default
        {
            self.flags &= !MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
            self.default_generator
                .initialize_for_motion_master_update_like_cpp(context);
        }

        let keep_running = if self.active_generators.is_empty() {
            initialize_or_reset_for_motion_master_update_like_cpp(
                &mut self.default_generator,
                context,
            );
            self.default_generator
                .update_for_motion_master_like_cpp(context)
        } else {
            let top = &mut self.active_generators[0];
            initialize_or_reset_for_motion_master_update_like_cpp(top, context);
            top.update_for_motion_master_like_cpp(context)
        };

        let popped = if !keep_running && !self.active_generators.is_empty() {
            Some(self.remove_generator_at(0))
        } else {
            None
        };
        self.current_generator = self.current_movement_generator().kind;

        self.flags &= !MOTIONMASTER_FLAG_UPDATE;
        let resolved_delayed_actions = self.resolve_delayed_action_payloads_like_cpp();

        MotionMasterUpdateOutcome::Updated {
            popped,
            resolved_delayed_actions,
        }
    }

    pub fn set_current_generator(&mut self, generator: MovementGeneratorKind) {
        self.add_generator(MovementGeneratorRef::new(generator, MovementSlot::Active));
    }

    pub fn add_to_world(&mut self) {
        let _ = self.add_to_world_like_cpp();
    }

    pub fn add_to_world_like_cpp(&mut self) -> MotionMasterAddToWorldOutcomeLikeCpp {
        let flags_before = self.flags;
        let had_initialization_pending =
            self.has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZATION_PENDING);

        if !had_initialization_pending {
            return MotionMasterAddToWorldOutcomeLikeCpp {
                had_initialization_pending,
                entered_initializing: false,
                direct_initialize_represented: false,
                resolved_delayed_actions: Vec::new(),
                exited_initializing: false,
                flags_before,
                flags_after: self.flags,
                current_generator_after: self.current_generator,
            };
        }

        self.flags |= MOTIONMASTER_FLAG_INITIALIZING;
        self.flags &= !MOTIONMASTER_FLAG_INITIALIZATION_PENDING;

        self.direct_initialize_like_cpp();
        let resolved_delayed_actions = self.resolve_delayed_action_payloads_like_cpp();

        self.flags &= !MOTIONMASTER_FLAG_INITIALIZING;
        self.current_generator = self.current_movement_generator().kind;

        MotionMasterAddToWorldOutcomeLikeCpp {
            had_initialization_pending,
            entered_initializing: (flags_before & MOTIONMASTER_FLAG_INITIALIZING) == 0,
            direct_initialize_represented: true,
            resolved_delayed_actions,
            exited_initializing: !self.has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZING),
            flags_before,
            flags_after: self.flags,
            current_generator_after: self.current_generator,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.active_generators.is_empty()
            && self.default_generator.kind == MovementGeneratorKind::Custom(u32::MAX)
    }

    pub fn size(&self) -> usize {
        1 + self.active_generators.len()
    }

    pub fn current_slot(&self) -> MovementSlot {
        if self.active_generators.is_empty() {
            MovementSlot::Default
        } else {
            MovementSlot::Active
        }
    }

    pub fn current_movement_generator(&self) -> MovementGeneratorRef {
        self.active_generators
            .first()
            .copied()
            .unwrap_or(self.default_generator)
    }

    pub fn add_generator(&mut self, mut generator: MovementGeneratorRef) {
        match generator.slot {
            MovementSlot::Default => {
                generator.slot = MovementSlot::Default;
                self.default_generator = generator;
                if generator.kind == MovementGeneratorKind::Idle {
                    self.flags |= MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
                }
            }
            MovementSlot::Active => {
                generator.slot = MovementSlot::Active;
                if let Some(top) = self.active_generators.first().copied() {
                    if generator.priority >= top.priority {
                        if generator.priority == top.priority {
                            self.remove_generator_at(0);
                        } else if let Some(top) = self.active_generators.first_mut() {
                            top.flags |= MOVEMENTGENERATOR_FLAG_DEACTIVATED;
                        }
                    } else if let Some(index) = self
                        .active_generators
                        .iter()
                        .position(|known| known.priority == generator.priority)
                    {
                        self.remove_generator_at(index);
                    }
                }

                self.add_base_unit_state(generator.base_unit_state);
                self.active_generators.push(generator);
                self.sort_active_generators();
            }
        }
        self.current_generator = self.current_movement_generator().kind;
        self.stopped = false;
    }

    pub fn remove_generator_kind(
        &mut self,
        kind: MovementGeneratorKind,
        slot: MovementSlot,
    ) -> Option<MovementGeneratorRef> {
        let removed = match slot {
            MovementSlot::Default if self.default_generator.kind == kind => {
                let previous = self.default_generator;
                self.move_idle();
                Some(previous)
            }
            MovementSlot::Default => None,
            MovementSlot::Active => self
                .active_generators
                .iter()
                .position(|generator| generator.kind == kind)
                .map(|index| self.remove_generator_at(index)),
        };
        self.current_generator = self.current_movement_generator().kind;
        removed
    }

    pub fn clear_active(&mut self) -> Vec<MovementGeneratorRef> {
        let removed = std::mem::take(&mut self.active_generators);
        self.base_unit_states.clear();
        self.current_generator = self.default_generator.kind;
        removed
    }

    pub fn clear_slot(&mut self, slot: MovementSlot) -> Vec<MovementGeneratorRef> {
        match slot {
            MovementSlot::Default => {
                let previous = self.default_generator;
                self.move_idle();
                vec![previous]
            }
            MovementSlot::Active => self.clear_active(),
        }
    }

    pub fn clear_by_priority(
        &mut self,
        priority: MovementGeneratorPriority,
    ) -> Vec<MovementGeneratorRef> {
        let mut removed = Vec::new();
        let mut index = 0;
        while index < self.active_generators.len() {
            if self.active_generators[index].priority == priority {
                removed.push(self.remove_generator_at(index));
            } else {
                index += 1;
            }
        }
        self.current_generator = self.current_movement_generator().kind;
        removed
    }

    pub fn clear_by_mode(&mut self, mode: MovementGeneratorMode) -> Vec<MovementGeneratorRef> {
        let mut removed = Vec::new();
        let mut index = 0;
        while index < self.active_generators.len() {
            if self.active_generators[index].mode == mode {
                removed.push(self.remove_generator_at(index));
            } else {
                index += 1;
            }
        }
        self.current_generator = self.current_movement_generator().kind;
        removed
    }

    pub fn direct_initialize_like_cpp(&mut self) {
        let selected_default = self.default_generator.kind;
        self.clear_active();
        self.initialize_default_generator_like_cpp(selected_default);
    }

    fn apply_delayed_action_payload_like_cpp(&mut self, payload: MotionMasterDelayedActionPayload) {
        match payload {
            MotionMasterDelayedActionPayload::Clear => {
                self.clear_active();
            }
            MotionMasterDelayedActionPayload::ClearSlot(slot) => {
                self.clear_slot(slot);
            }
            MotionMasterDelayedActionPayload::ClearMode(mode) => {
                self.clear_by_mode(mode);
            }
            MotionMasterDelayedActionPayload::ClearPriority(priority) => {
                self.clear_by_priority(priority);
            }
            MotionMasterDelayedActionPayload::Add(generator) => {
                self.add_generator(generator);
            }
            MotionMasterDelayedActionPayload::Remove { kind, slot }
            | MotionMasterDelayedActionPayload::RemoveType { kind, slot } => {
                self.remove_generator_kind(kind, slot);
            }
            MotionMasterDelayedActionPayload::Initialize => {
                self.direct_initialize_like_cpp();
            }
        }
    }

    pub fn move_idle(&mut self) {
        self.initialize_default_generator_like_cpp(MovementGeneratorKind::Idle);
        self.flags |= MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
        if self.active_generators.is_empty() {
            self.current_generator = MovementGeneratorKind::Idle;
        }
    }

    pub fn initialize_default_generator_like_cpp(&mut self, kind: MovementGeneratorKind) {
        self.default_generator = match kind {
            MovementGeneratorKind::Waypoint => {
                MovementGeneratorRef::new(MovementGeneratorKind::Waypoint, MovementSlot::Default)
                    .with_priority(MovementGeneratorPriority::Normal)
                    .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                    .with_base_unit_state(UnitState::ROAMING.bits())
            }
            MovementGeneratorKind::Idle => {
                MovementGeneratorRef::new(MovementGeneratorKind::Idle, MovementSlot::Default)
                    .with_priority(MovementGeneratorPriority::Normal)
                    .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZED)
            }
            other => MovementGeneratorRef::new(other, MovementSlot::Default)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING),
        };
        if self.active_generators.is_empty() {
            self.current_generator = self.default_generator.kind;
        }
    }

    pub fn move_point(&mut self, movement_id: u32) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::ROAMING.bits())
                .with_movement_id(movement_id),
        );
    }

    pub fn move_seek_assistance_like_cpp(&mut self) -> SeekAssistancePlan {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Assistance, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::ROAMING.bits())
                .with_movement_id(EVENT_ASSIST_MOVE),
        );
        SeekAssistancePlan {
            attack_stop: true,
            cast_stop: true,
            do_not_reacquire_spell_focus_target: true,
            set_react_passive: true,
            generator_added: true,
        }
    }

    pub fn move_seek_assistance_distract_like_cpp(&mut self, timer_ms: u32) {
        self.add_generator(
            MovementGeneratorRef::new(
                MovementGeneratorKind::AssistanceDistract,
                MovementSlot::Active,
            )
            .with_priority(MovementGeneratorPriority::Normal)
            .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
            .with_base_unit_state(UnitState::DISTRACTED.bits())
            .with_duration_ms(timer_ms),
        );
    }

    pub fn move_distract_like_cpp(&mut self, timer_ms: u32) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Distract, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Highest)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::DISTRACTED.bits())
                .with_duration_ms(timer_ms),
        );
    }

    pub fn move_rotate_like_cpp(
        &mut self,
        movement_id: u32,
        time_ms: u32,
        direction: RotateDirection,
    ) -> bool {
        if time_ms == 0 {
            return false;
        }

        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Rotate, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::ROTATING.bits())
                .with_movement_id(movement_id)
                .with_duration_ms(time_ms)
                .with_max_duration_ms(time_ms)
                .with_rotate_direction(direction),
        );
        true
    }

    pub fn move_charge(&mut self, movement_id: u32) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Highest)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::CHARGING.bits())
                .with_movement_id(movement_id),
        );
    }

    pub fn move_follow(&mut self, target_guid: ObjectGuid, duration_ms: Option<u32>) {
        let mut generator =
            MovementGeneratorRef::new(MovementGeneratorKind::Follow, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_target_guid(target_guid);
        if let Some(duration_ms) = duration_ms {
            generator = generator.with_duration_ms(duration_ms);
        }
        self.add_generator(generator);
    }

    pub fn launch_generic_movement(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        duration_ms: u32,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) {
        self.add_generic_movement(
            kind,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Normal,
            UnitState::ROAMING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            arrival_spell,
        );
    }

    pub fn launch_move_spline_like_cpp(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        priority: MovementGeneratorPriority,
        duration_ms: u32,
    ) -> bool {
        let trinity_type = kind.trinity_id();
        if trinity_type == 3 || trinity_type >= 19 {
            return false;
        }

        self.add_generic_movement(
            kind,
            movement_id,
            duration_ms,
            priority,
            UnitState::ROAMING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            None,
        );
        true
    }

    pub fn move_jump_like_cpp(
        &mut self,
        movement_id: u32,
        duration_ms: u32,
        speed_xy: f32,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) -> bool {
        if speed_xy < 0.01 {
            return false;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Highest,
            UnitState::JUMPING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            arrival_spell,
        );
        true
    }

    pub fn move_jump_with_gravity_like_cpp(
        &mut self,
        movement_id: u32,
        duration_ms: u32,
        speed_xy: f32,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) -> bool {
        if speed_xy < 0.01 {
            return false;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Highest,
            UnitState::JUMPING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH,
            arrival_spell,
        );
        true
    }

    pub fn move_knockback_from_like_cpp(
        &mut self,
        is_player: bool,
        duration_ms: u32,
        speed_xy: f32,
    ) -> bool {
        if is_player || speed_xy < 0.01 {
            return false;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            0,
            duration_ms,
            MovementGeneratorPriority::Highest,
            0,
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH,
            None,
        );
        true
    }

    pub fn move_fall_like_cpp(
        &mut self,
        movement_id: u32,
        duration_ms: u32,
        has_valid_ground_height: bool,
        vertical_delta: f32,
        has_root_or_stun_state: bool,
        is_player: bool,
    ) -> MoveFallPlan {
        if !has_valid_ground_height || vertical_delta.abs() < 0.1 || has_root_or_stun_state {
            return MoveFallPlan::Noop;
        }

        if is_player {
            return MoveFallPlan::PlayerFallInfo;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Highest,
            0,
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            None,
        );
        MoveFallPlan::SplineStarted
    }

    fn add_generic_movement(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        duration_ms: u32,
        priority: MovementGeneratorPriority,
        base_unit_state: u32,
        flags: u16,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) {
        let mut generator = MovementGeneratorRef::new(kind, MovementSlot::Active)
            .with_priority(priority)
            .with_flags(flags)
            .with_base_unit_state(base_unit_state)
            .with_movement_id(movement_id)
            .with_duration_ms(duration_ms);
        if let Some((spell_id, target_guid)) = arrival_spell {
            generator = generator.with_arrival_spell(spell_id, target_guid);
        }
        self.add_generator(generator);
    }

    pub fn stop_on_death(&mut self) -> bool {
        if self
            .active_generators
            .first()
            .is_some_and(|generator| generator.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH))
        {
            return false;
        }

        self.clear_active();
        self.move_idle();
        self.stop_moving();
        true
    }

    pub fn pause_movement(&mut self) {
        self.paused = true;
    }

    pub fn resume_movement(&mut self) {
        self.paused = false;
    }

    pub fn stop_moving(&mut self) {
        self.stopped = true;
        self.finalize_spline();
    }

    pub fn start_spline(&mut self, spline_id: u32, duration_ms: u32) {
        self.spline = MoveSplineState {
            enabled: true,
            finalized: false,
            cyclic: false,
            on_transport: false,
            spline_id,
            progress_ms: 0,
            duration_ms,
            velocity: None,
            final_destination: None,
            current_destination: None,
        };
        self.stopped = false;
    }

    pub fn launch_spline(
        &mut self,
        spline_id: u32,
        duration_ms: u32,
        destination: (i32, i32, i32),
        cyclic: bool,
        on_transport: bool,
        velocity: Option<u32>,
    ) {
        self.spline = MoveSplineState {
            enabled: true,
            finalized: false,
            cyclic,
            on_transport,
            spline_id,
            progress_ms: 0,
            duration_ms,
            velocity,
            final_destination: Some(destination),
            current_destination: Some(destination),
        };
        self.stopped = false;
    }

    pub fn set_spline_progress(&mut self, progress_ms: u32) {
        self.spline.progress_ms = progress_ms.min(self.spline.duration_ms);
        if self.spline.progress_ms >= self.spline.duration_ms && !self.spline.cyclic {
            self.finalize_spline();
        }
    }

    pub fn update_spline(&mut self, diff_ms: u32) -> bool {
        if !self.spline.enabled || self.spline.finalized {
            return false;
        }
        let next_progress = self.spline.progress_ms.saturating_add(diff_ms);
        if self.spline.cyclic && self.spline.duration_ms > 0 {
            self.spline.progress_ms = next_progress % self.spline.duration_ms;
            return false;
        }
        self.set_spline_progress(next_progress);
        self.spline.finalized
    }

    pub fn finalize_spline(&mut self) {
        self.spline.enabled = false;
        self.spline.finalized = true;
        self.spline.progress_ms = self.spline.duration_ms;
    }

    pub fn interrupt_spline(&mut self) {
        self.finalize_spline();
        self.spline.current_destination = None;
    }

    fn sort_active_generators(&mut self) {
        self.active_generators.sort_by(|left, right| {
            right
                .mode
                .cmp(&left.mode)
                .then_with(|| right.priority.cmp(&left.priority))
        });
    }

    fn remove_generator_at(&mut self, index: usize) -> MovementGeneratorRef {
        let removed = self.active_generators.remove(index);
        self.clear_base_unit_state(removed.base_unit_state);
        removed
    }

    fn add_base_unit_state(&mut self, base_unit_state: u32) {
        if base_unit_state != 0 {
            *self.base_unit_states.entry(base_unit_state).or_insert(0) += 1;
        }
    }

    fn clear_base_unit_state(&mut self, base_unit_state: u32) {
        if base_unit_state == 0 {
            return;
        }
        if let Some(count) = self.base_unit_states.get_mut(&base_unit_state) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.base_unit_states.remove(&base_unit_state);
            }
        }
    }
}

pub const SUMMON_SLOT_PET: usize = 0;
pub const SUMMON_SLOT_TOTEM: usize = 1;
pub const SUMMON_SLOT_TOTEM_2: usize = 2;
pub const SUMMON_SLOT_TOTEM_3: usize = 3;
pub const SUMMON_SLOT_TOTEM_4: usize = 4;
pub const SUMMON_SLOT_MINIPET: usize = 5;
pub const SUMMON_SLOT_QUEST: usize = 6;
pub const MAX_SUMMON_SLOT: usize = 7;
pub const MAX_TOTEM_SLOT: usize = 5;
pub const MAX_UNIT_ACTION_BAR_INDEX: usize = 10;
pub const MAX_SPELL_CHARM: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CharmType {
    Charm = 0,
    Possess = 1,
    Vehicle = 2,
    Convert = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharmInfoState {
    pub pet_number: u32,
    pub command_state: u8,
    pub action_bar: [u32; MAX_UNIT_ACTION_BAR_INDEX],
    pub charm_spells: [u32; MAX_SPELL_CHARM],
    pub is_command_attack: bool,
    pub is_command_follow: bool,
    pub is_at_stay: bool,
    pub is_following: bool,
    pub is_returning: bool,
    pub stay_position: Option<(f32, f32, f32)>,
}

impl Default for CharmInfoState {
    fn default() -> Self {
        Self {
            pet_number: 0,
            command_state: 0,
            action_bar: [0; MAX_UNIT_ACTION_BAR_INDEX],
            charm_spells: [0; MAX_SPELL_CHARM],
            is_command_attack: false,
            is_command_follow: false,
            is_at_stay: false,
            is_following: false,
            is_returning: false,
            stay_position: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlSubsystem {
    pub owner_guid: Option<ObjectGuid>,
    pub minion_guid: Option<ObjectGuid>,
    pub summon_slots: [ObjectGuid; MAX_SUMMON_SLOT],
    pub last_charmer_guid: Option<ObjectGuid>,
    pub charmer_guid: Option<ObjectGuid>,
    pub charmed_guid: Option<ObjectGuid>,
    pub controlled_guids: HashSet<ObjectGuid>,
    pub controlled_by_player: bool,
    pub charm_type: Option<CharmType>,
    pub unit_moved_by_me: Option<ObjectGuid>,
    pub player_moving_me: Option<ObjectGuid>,
    pub shared_vision_guids: HashSet<ObjectGuid>,
    pub owner_attacked_notifications: Vec<ControlledOwnerAttackedNotification>,
    pub charm_info: Option<CharmInfoState>,
    pub old_faction_id: Option<u32>,
    pub walking_before_charm: bool,
}

impl Default for ControlSubsystem {
    fn default() -> Self {
        Self {
            owner_guid: None,
            minion_guid: None,
            summon_slots: [ObjectGuid::EMPTY; MAX_SUMMON_SLOT],
            last_charmer_guid: None,
            charmer_guid: None,
            charmed_guid: None,
            controlled_guids: HashSet::new(),
            controlled_by_player: false,
            charm_type: None,
            unit_moved_by_me: None,
            player_moving_me: None,
            shared_vision_guids: HashSet::new(),
            owner_attacked_notifications: Vec::new(),
            charm_info: None,
            old_faction_id: None,
            walking_before_charm: false,
        }
    }
}

impl ControlSubsystem {
    pub fn set_owner_guid(&mut self, owner: Option<ObjectGuid>) {
        self.owner_guid = owner;
    }

    pub fn set_minion_guid(&mut self, minion: Option<ObjectGuid>) {
        self.minion_guid = minion;
    }

    pub fn pet_guid(&self) -> ObjectGuid {
        self.summon_slots[SUMMON_SLOT_PET]
    }

    pub fn set_pet_guid(&mut self, pet: ObjectGuid) {
        self.summon_slots[SUMMON_SLOT_PET] = pet;
    }

    pub fn set_summon_slot(&mut self, slot: usize, guid: ObjectGuid) -> bool {
        let Some(target) = self.summon_slots.get_mut(slot) else {
            return false;
        };
        *target = guid;
        true
    }

    pub fn clear_summon_slot(&mut self, slot: usize) -> Option<ObjectGuid> {
        let target = self.summon_slots.get_mut(slot)?;
        let previous = *target;
        *target = ObjectGuid::EMPTY;
        Some(previous)
    }

    pub fn set_charmer(&mut self, charmer: ObjectGuid, controlled_by_player: bool) {
        self.last_charmer_guid = self.charmer_guid;
        self.charmer_guid = Some(charmer);
        self.controlled_by_player = controlled_by_player;
        self.init_charm_info();
    }

    pub fn remove_charmer(&mut self) {
        self.last_charmer_guid = self.charmer_guid;
        self.charmer_guid = None;
        self.controlled_by_player = false;
        self.charm_type = None;
        self.old_faction_id = None;
        self.delete_charm_info();
    }

    pub fn set_charmed(&mut self, charmed: ObjectGuid) {
        self.charmed_guid = Some(charmed);
        self.controlled_guids.insert(charmed);
    }

    pub fn remove_charmed(&mut self) {
        if let Some(charmed) = self.charmed_guid.take() {
            self.controlled_guids.remove(&charmed);
        }
    }

    pub fn apply_charm_as_controller(&mut self, charmed: ObjectGuid, controller_is_player: bool) {
        if controller_is_player {
            self.charmed_guid = Some(charmed);
        }
        self.controlled_guids.insert(charmed);
    }

    pub fn remove_charm_as_controller(
        &mut self,
        charmed: ObjectGuid,
        controlled_has_same_owner: bool,
        controlled_is_minion: bool,
        controlled_is_player: bool,
    ) {
        if self.charmed_guid == Some(charmed) {
            self.charmed_guid = None;
        }
        if controlled_is_player || !controlled_is_minion || !controlled_has_same_owner {
            self.controlled_guids.remove(&charmed);
        }
    }

    pub fn apply_charmed_by(
        &mut self,
        charmer: ObjectGuid,
        charm_type: CharmType,
        controlled_by_player: bool,
        old_faction_id: Option<u32>,
        was_walking: bool,
    ) -> bool {
        if self.charmer_guid.is_some() {
            return false;
        }
        self.charmer_guid = Some(charmer);
        self.controlled_by_player = controlled_by_player;
        self.charm_type = Some(charm_type);
        self.old_faction_id = old_faction_id;
        self.walking_before_charm = was_walking;
        if charm_type != CharmType::Vehicle {
            self.init_charm_info();
        }
        true
    }

    pub fn remove_charmed_by(
        &mut self,
        expected_charmer: Option<ObjectGuid>,
        is_guardian: bool,
    ) -> bool {
        let Some(charmer) = self.charmer_guid else {
            return false;
        };
        if expected_charmer.is_some_and(|expected| expected != charmer) {
            return false;
        }
        if self.charm_type != Some(CharmType::Vehicle) {
            self.last_charmer_guid = Some(charmer);
        }
        self.charmer_guid = None;
        self.controlled_by_player = false;
        self.charm_type = None;
        self.old_faction_id = None;
        if !is_guardian {
            self.delete_charm_info();
        }
        true
    }

    pub fn add_controlled(&mut self, guid: ObjectGuid) -> bool {
        self.controlled_guids.insert(guid)
    }

    pub fn remove_controlled(&mut self, guid: ObjectGuid) -> bool {
        if self.charmed_guid == Some(guid) {
            self.charmed_guid = None;
        }
        self.controlled_guids.remove(&guid)
    }

    pub fn clear_controlled(&mut self) {
        self.controlled_guids.clear();
        self.charmed_guid = None;
    }

    pub fn is_charmed(&self) -> bool {
        self.charmer_guid.is_some()
    }

    pub fn is_possessed(&self) -> bool {
        self.charm_type == Some(CharmType::Possess)
    }

    pub fn is_possessed_by_player(&self) -> bool {
        self.is_possessed() && self.controlled_by_player
    }

    pub fn is_possessing(&self) -> bool {
        self.charmed_guid.is_some()
    }

    pub fn is_possessing_guid(&self, guid: ObjectGuid) -> bool {
        self.charmed_guid == Some(guid)
    }

    pub fn charmer_or_owner_guid(&self) -> Option<ObjectGuid> {
        self.charmer_guid.or(self.owner_guid)
    }

    pub fn charmer_or_owner_or_self_guid(&self, own_guid: ObjectGuid) -> ObjectGuid {
        self.charmer_or_owner_guid().unwrap_or(own_guid)
    }

    pub fn init_charm_info(&mut self) -> &mut CharmInfoState {
        self.charm_info.get_or_insert_with(CharmInfoState::default)
    }

    pub fn delete_charm_info(&mut self) {
        self.charm_info = None;
    }

    pub fn has_charm_info(&self) -> bool {
        self.charm_info.is_some()
    }

    pub fn remove_all_controlled(&mut self) -> Vec<ObjectGuid> {
        let removed = self.controlled_guids.drain().collect();
        self.charmed_guid = None;
        removed
    }

    pub fn set_moved_unit(&mut self, target: Option<ObjectGuid>) {
        self.unit_moved_by_me = target;
    }

    pub fn set_player_moving_me(&mut self, player: Option<ObjectGuid>) {
        self.player_moving_me = player;
    }

    pub fn add_shared_vision(&mut self, guid: ObjectGuid) -> bool {
        self.shared_vision_guids.insert(guid)
    }

    pub fn remove_shared_vision(&mut self, guid: ObjectGuid) -> bool {
        self.shared_vision_guids.remove(&guid)
    }

    pub fn has_shared_vision(&self) -> bool {
        !self.shared_vision_guids.is_empty()
    }

    pub fn notify_controlled_owner_attacked_like_cpp(
        &mut self,
        controlled_creatures_with_ai: &[ObjectGuid],
        victim: ObjectGuid,
    ) {
        for controlled in controlled_creatures_with_ai {
            if self.controlled_guids.contains(controlled) {
                self.owner_attacked_notifications
                    .push(ControlledOwnerAttackedNotification {
                        controlled: *controlled,
                        victim,
                    });
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlledOwnerAttackedNotification {
    pub controlled: ObjectGuid,
    pub victim: ObjectGuid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VehicleKitState {
    pub kit_id: u32,
    pub active: bool,
    pub installed: bool,
    pub vehicle: Option<Vehicle>,
}

impl VehicleKitState {
    pub const fn kit_id(&self) -> u32 {
        self.kit_id
    }

    pub const fn active(&self) -> bool {
        self.active
    }

    pub const fn installed(&self) -> bool {
        self.installed
    }

    pub const fn vehicle(&self) -> Option<&Vehicle> {
        self.vehicle.as_ref()
    }

    pub fn seat_count(&self) -> usize {
        self.vehicle
            .as_ref()
            .map_or(0, |vehicle| vehicle.seats().len())
    }

    pub fn usable_seat_num(&self) -> u32 {
        self.vehicle.as_ref().map_or(0, Vehicle::usable_seat_num)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleKitCreateOutcomeLikeCpp {
    pub kit_id: Option<u32>,
    pub created: bool,
    pub loading: bool,
    pub seat_count: usize,
    pub usable_seat_num: u32,
    pub unit_update_flag_vehicle_represented: bool,
    pub unit_type_mask_vehicle_represented: bool,
    pub send_set_vehicle_rec_id_represented: bool,
    pub set_spellclick_or_player_vehicle_npc_flag_represented: bool,
    pub remove_spellclick_or_player_vehicle_npc_flag_represented: bool,
    pub update_display_power_represented: bool,
    pub init_movement_info_for_base_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleKitInstallOutcomeLikeCpp {
    pub kit_id: Option<u32>,
    pub had_kit: bool,
    pub previous_installed: Option<bool>,
    pub installed: bool,
    pub script_on_install_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleKitRemoveOutcomeLikeCpp {
    pub kit_id: Option<u32>,
    pub had_kit: bool,
    pub previous_installed: Option<bool>,
    pub on_remove_from_world: bool,
    pub send_set_vehicle_rec_id_zero_represented: bool,
    pub uninstall_represented: bool,
    pub remove_all_passengers_represented: bool,
    pub script_on_uninstall_represented: bool,
    pub kit_cleared: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleKitAddToWorldResetOutcomeLikeCpp {
    pub kit_id: u32,
    pub aim_create_represented: bool,
    pub ai_initialize_represented: bool,
    pub reset_evading: bool,
    pub reset_plan: VehicleResetPlan,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VehicleSubsystem {
    pub vehicle_guid: Option<ObjectGuid>,
    pub base_vehicle_guid: Option<ObjectGuid>,
    pub seat_id: Option<i8>,
    pub kit: Option<VehicleKitState>,
    pub last_create_outcome: Option<VehicleKitCreateOutcomeLikeCpp>,
}

impl VehicleSubsystem {
    pub fn enter_vehicle(&mut self, vehicle_guid: ObjectGuid, seat_id: Option<i8>) {
        self.vehicle_guid = Some(vehicle_guid);
        self.seat_id = seat_id;
    }

    pub fn exit_vehicle(&mut self) {
        self.vehicle_guid = None;
        self.seat_id = None;
    }

    pub fn set_vehicle_kit(&mut self, kit_id: u32, active: bool) {
        self.kit = Some(VehicleKitState {
            kit_id,
            active,
            installed: false,
            vehicle: None,
        });
    }

    pub fn create_vehicle_kit_like_cpp(
        &mut self,
        base_guid: ObjectGuid,
        base_position: Position,
        vehicle_id: Option<u32>,
        creature_entry: u32,
        loading: bool,
        seat_defs: Option<Vec<(i8, VehicleSeatInfo, VehicleSeatAddon)>>,
    ) -> VehicleKitCreateOutcomeLikeCpp {
        let Some(kit_id) = vehicle_id else {
            let outcome = VehicleKitCreateOutcomeLikeCpp {
                kit_id: None,
                created: false,
                loading,
                seat_count: 0,
                usable_seat_num: 0,
                unit_update_flag_vehicle_represented: false,
                unit_type_mask_vehicle_represented: false,
                send_set_vehicle_rec_id_represented: false,
                set_spellclick_or_player_vehicle_npc_flag_represented: false,
                remove_spellclick_or_player_vehicle_npc_flag_represented: false,
                update_display_power_represented: false,
                init_movement_info_for_base_represented: false,
            };
            self.last_create_outcome = Some(outcome.clone());
            return outcome;
        };
        let Some(seat_defs) = seat_defs else {
            let outcome = VehicleKitCreateOutcomeLikeCpp {
                kit_id: Some(kit_id),
                created: false,
                loading,
                seat_count: 0,
                usable_seat_num: 0,
                unit_update_flag_vehicle_represented: false,
                unit_type_mask_vehicle_represented: false,
                send_set_vehicle_rec_id_represented: false,
                set_spellclick_or_player_vehicle_npc_flag_represented: false,
                remove_spellclick_or_player_vehicle_npc_flag_represented: false,
                update_display_power_represented: false,
                init_movement_info_for_base_represented: false,
            };
            self.last_create_outcome = Some(outcome.clone());
            return outcome;
        };

        let vehicle = Vehicle::new(
            base_guid,
            TypeId::Unit,
            base_position,
            kit_id,
            creature_entry,
            seat_defs,
        );
        let seat_count = vehicle.seats().len();
        let usable_seat_num = vehicle.usable_seat_num();
        self.kit = Some(VehicleKitState {
            kit_id,
            active: true,
            installed: false,
            vehicle: Some(vehicle),
        });
        let outcome = VehicleKitCreateOutcomeLikeCpp {
            kit_id: Some(kit_id),
            created: true,
            loading,
            seat_count,
            usable_seat_num,
            unit_update_flag_vehicle_represented: true,
            unit_type_mask_vehicle_represented: true,
            send_set_vehicle_rec_id_represented: !loading,
            set_spellclick_or_player_vehicle_npc_flag_represented: usable_seat_num != 0,
            remove_spellclick_or_player_vehicle_npc_flag_represented: usable_seat_num == 0,
            update_display_power_represented: true,
            init_movement_info_for_base_represented: true,
        };
        self.last_create_outcome = Some(outcome.clone());
        outcome
    }

    pub fn install_vehicle_kit_like_cpp(&mut self) -> VehicleKitInstallOutcomeLikeCpp {
        let Some(kit) = self.kit.as_mut() else {
            return VehicleKitInstallOutcomeLikeCpp {
                kit_id: None,
                had_kit: false,
                previous_installed: None,
                installed: false,
                script_on_install_represented: false,
            };
        };

        let previous_installed = kit.installed;
        if !kit.installed {
            kit.installed = true;
            if let Some(vehicle) = kit.vehicle.as_mut() {
                vehicle.install();
            }
        }

        VehicleKitInstallOutcomeLikeCpp {
            kit_id: Some(kit.kit_id),
            had_kit: true,
            previous_installed: Some(previous_installed),
            installed: kit.installed,
            script_on_install_represented: true,
        }
    }

    pub fn reset_vehicle_kit_for_creature_add_to_world_like_cpp(
        &mut self,
        context: &CreatureAddToWorldVehicleResetContextLikeCpp,
        base_is_alive: bool,
    ) -> Option<VehicleKitAddToWorldResetOutcomeLikeCpp> {
        let kit = self.kit.as_mut()?;
        let vehicle = kit.vehicle.as_mut()?;
        let reset_plan = vehicle.reset_plan_like_cpp(
            false,
            base_is_alive,
            context.is_mechanical_creature,
            context.is_world_boss,
            &context.accessories,
        )?;

        Some(VehicleKitAddToWorldResetOutcomeLikeCpp {
            kit_id: kit.kit_id,
            aim_create_represented: true,
            ai_initialize_represented: true,
            reset_evading: false,
            reset_plan,
        })
    }

    pub fn remove_vehicle_kit_like_cpp(
        &mut self,
        on_remove_from_world: bool,
    ) -> VehicleKitRemoveOutcomeLikeCpp {
        let Some(kit) = self.kit.take() else {
            return VehicleKitRemoveOutcomeLikeCpp {
                kit_id: None,
                had_kit: false,
                previous_installed: None,
                on_remove_from_world,
                send_set_vehicle_rec_id_zero_represented: false,
                uninstall_represented: false,
                remove_all_passengers_represented: false,
                script_on_uninstall_represented: false,
                kit_cleared: false,
            };
        };

        if let Some(mut vehicle) = kit.vehicle {
            vehicle.uninstall();
        }

        VehicleKitRemoveOutcomeLikeCpp {
            kit_id: Some(kit.kit_id),
            had_kit: true,
            previous_installed: Some(kit.installed),
            on_remove_from_world,
            send_set_vehicle_rec_id_zero_represented: !on_remove_from_world,
            uninstall_represented: true,
            remove_all_passengers_represented: true,
            script_on_uninstall_represented: true,
            kit_cleared: true,
        }
    }

    pub fn clear_vehicle_kit(&mut self) {
        self.kit = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AiSubsystem {
    pub active_ai: Option<String>,
    pub ai_stack: Vec<String>,
    pub locked: bool,
    pub scheduled_change_pending: bool,
    pub update_ticks: u64,
    pub last_update_diff_ms: u32,
    pub hostile_reaction_count: u32,
    pub call_assistance_count: u32,
}

impl AiSubsystem {
    pub fn set_active(&mut self, ai: Option<impl Into<String>>) {
        if !self.locked {
            self.active_ai = ai.map(Into::into);
        }
    }

    pub fn push(&mut self, ai: impl Into<String>) {
        if self.locked {
            self.scheduled_change_pending = true;
            return;
        }
        if let Some(active) = self.active_ai.take() {
            self.ai_stack.push(active);
        }
        self.active_ai = Some(ai.into());
    }

    pub fn pop(&mut self) -> Option<String> {
        if self.locked {
            self.scheduled_change_pending = true;
            return None;
        }
        let popped = self.active_ai.take();
        self.active_ai = self.ai_stack.pop();
        popped
    }

    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    pub fn is_enabled(&self) -> bool {
        self.active_ai.is_some()
    }

    pub fn update_tick(&mut self, diff_ms: u32) -> bool {
        let Some(_) = self.active_ai else {
            return false;
        };
        self.locked = true;
        self.update_ticks = self.update_ticks.saturating_add(1);
        self.last_update_diff_ms = diff_ms;
        self.locked = false;
        true
    }

    pub fn send_hostile_reaction_like_cpp(&mut self) {
        self.hostile_reaction_count = self.hostile_reaction_count.saturating_add(1);
    }

    pub fn call_assistance_like_cpp(&mut self) {
        self.call_assistance_count = self.call_assistance_count.saturating_add(1);
    }

    pub fn schedule_change(&mut self) {
        self.scheduled_change_pending = true;
    }

    pub fn apply_scheduled_change(&mut self, ai: impl Into<String>, charmed: bool) {
        if self.locked {
            self.scheduled_change_pending = true;
            return;
        }
        if !charmed {
            self.restore_disabled_ai();
        }
        self.push(ai);
        self.scheduled_change_pending = false;
    }

    pub fn restore_disabled_ai(&mut self) {
        while self
            .active_ai
            .as_deref()
            .is_some_and(|ai| ai == "ScheduledChangeAI")
        {
            if self.pop().is_none() {
                break;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnitSubsystems {
    pub auras: AuraSubsystem,
    pub spells: SpellSubsystem,
    pub combat: CombatSubsystem,
    pub motion: MotionSubsystem,
    pub control: ControlSubsystem,
    pub vehicle: VehicleSubsystem,
    pub ai: AiSubsystem,
}

impl UnitSubsystems {
    pub fn clear_runtime_state(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod unit_subsystems_tests {
    use super::*;

    fn guid(low: i64) -> ObjectGuid {
        ObjectGuid::new(0, low)
    }

    #[test]
    fn aura_spell_history_and_current_spell_helpers_roundtrip() {
        let mut subsystems = UnitSubsystems::default();
        let caster = guid(1);
        let owned = OwnedAuraRef::new(100, caster, None);
        let applied = AppliedAuraRef::new(100, caster, 2, 0x5);

        subsystems.auras.add_owned(owned);
        subsystems.auras.add_applied(applied);
        subsystems.auras.set_visible(2, AuraRef::new(100, caster));
        subsystems.auras.mark_removed(AuraRef::new(100, caster));
        subsystems.auras.interrupt_flags = 0x10;
        subsystems.auras.interrupt_flags2 = 0x20;

        assert!(subsystems.auras.has_owned(owned));
        assert!(subsystems.auras.has_applied(applied));
        assert_eq!(
            subsystems.auras.visible_auras.get(&2).copied(),
            Some(AuraRef::new(100, caster))
        );
        assert_eq!(subsystems.auras.removed_count(), 1);
        assert!(subsystems.auras.remove_owned(owned));
        assert!(subsystems.auras.remove_applied(applied));
        assert_eq!(
            subsystems.auras.clear_visible(2),
            Some(AuraRef::new(100, caster))
        );
        subsystems.auras.clear_removed();
        assert_eq!(subsystems.auras.removed_count(), 0);

        let spell = CurrentSpellRef::new(200, Some(caster), Some(guid(3)));
        subsystems
            .spells
            .set_current_spell(CurrentSpellSlot::Generic, spell);
        assert_eq!(
            subsystems.spells.current_spell(CurrentSpellSlot::Generic),
            Some(spell)
        );
        assert_eq!(
            subsystems
                .spells
                .clear_current_spell(CurrentSpellSlot::Generic),
            Some(spell)
        );

        subsystems.spells.history.set_cooldown(200, 1_000, 30_000);
        subsystems.spells.history.set_charges(200, 2, 1_000, 10_000);
        assert_eq!(
            subsystems.spells.history.cooldown(200),
            Some(SpellCooldown {
                spell_id: 200,
                item_id: 0,
                cooldown_end_ms: 31_000,
                category_id: 0,
                category_end_ms: 1_000,
                on_hold: false,
            })
        );
        assert_eq!(
            subsystems.spells.history.charges(200).map(VecDeque::len),
            Some(2)
        );
        assert!(subsystems.spells.history.clear_cooldown(200));
        subsystems.spells.history.reset();
        assert!(subsystems.spells.history.cooldowns.is_empty());
        assert!(subsystems.spells.history.charges.is_empty());
    }

    #[test]
    fn aura_application_interrupt_state_and_diminishing_match_cpp_shape() {
        let mut auras = AuraSubsystem::default();
        let caster = guid(2);
        let other = guid(3);
        let defensive = AppliedAuraRef::new(200, caster, 0, 0x1);
        let poison = AppliedAuraRef::new(201, caster, 1, 0x2);
        let other_poison = AppliedAuraRef::new(202, other, 2, 0x4);

        auras.register_applied_aura(defensive, Some(AURA_STATE_DEFENSIVE), 0x8, 0);
        assert!(auras.has_applied(defensive));
        assert!(auras.has_interrupt_flag(0x8));
        assert!(auras.has_aura_state(AURA_STATE_DEFENSIVE));
        assert_eq!(
            auras.build_aura_state_update_for_target(other),
            1 << (AURA_STATE_DEFENSIVE - 1)
        );

        auras.register_applied_aura(poison, Some(AURA_STATE_ROGUE_POISONED), 0, 0x20);
        auras.register_applied_aura(other_poison, Some(AURA_STATE_ROGUE_POISONED), 0, 0);
        assert!(auras.has_interrupt_flag2(0x20));
        assert_eq!(
            auras.build_aura_state_update_for_target(caster),
            (1 << (AURA_STATE_DEFENSIVE - 1)) | (1 << (AURA_STATE_ROGUE_POISONED - 1))
        );

        assert_eq!(auras.remove_interruptible_auras(0, 0x20), vec![poison]);
        assert!(!auras.has_applied(poison));
        assert!(auras.has_applied(other_poison));
        assert!(!auras.has_interrupt_flag2(0x20));
        assert_eq!(auras.removed_auras_count, 1);

        assert!(auras.can_proc());
        auras.set_cant_proc(true);
        assert!(!auras.can_proc());
        auras.set_cant_proc(false);
        assert!(auras.can_proc());

        assert_eq!(
            auras.get_diminishing(DIMINISHING_STUN, 1_000),
            DiminishingLevel::Level1
        );
        auras.incr_diminishing(DIMINISHING_STUN, DiminishingLevel::Immune, 1_000);
        assert_eq!(
            auras.get_diminishing(DIMINISHING_STUN, 1_000),
            DiminishingLevel::Level2
        );
        auras.apply_diminishing_aura(DIMINISHING_STUN, true, 2_000);
        auras.apply_diminishing_aura(DIMINISHING_STUN, false, 3_000);
        assert_eq!(auras.diminishing[DIMINISHING_STUN].hit_time_ms, 3_000);
        assert_eq!(
            auras.get_diminishing(DIMINISHING_STUN, 21_001),
            DiminishingLevel::Level1
        );
        auras.clear_diminishings();
        assert_eq!(
            auras.diminishing[DIMINISHING_STUN],
            DiminishingReturnState::default()
        );
    }

    #[test]
    fn aura_type_removal_matches_cpp_remove_auras_by_type_shape() {
        let mut auras = AuraSubsystem::default();
        let caster = guid(1);
        let unattackable = AppliedAuraRef::new(300, caster, 0, 0x1);
        let other_same_type = AppliedAuraRef::new(301, caster, 1, 0x2);
        let different = AppliedAuraRef::new(302, caster, 2, 0x4);

        auras.register_applied_aura_type_like_cpp(unattackable, 93);
        auras.register_applied_aura_type_like_cpp(other_same_type, 93);
        auras.register_applied_aura_type_like_cpp(different, 8);

        assert!(auras.has_aura_type_like_cpp(93));
        assert_eq!(
            auras.remove_auras_by_type_like_cpp(93),
            vec![unattackable, other_same_type]
        );

        assert!(!auras.has_applied(unattackable));
        assert!(!auras.has_applied(other_same_type));
        assert!(auras.has_applied(different));
        assert!(!auras.has_aura_type_like_cpp(93));
        assert!(auras.has_aura_type_like_cpp(8));
        assert_eq!(auras.removed_count(), 2);
    }

    #[test]
    fn spell_history_cooldowns_track_spell_category_hold_and_update_like_cpp() {
        let mut history = SpellHistory::default();

        assert!(history.start_cooldown(1_000, 100, 7, 3_000, 9, 1_500, false));
        assert!(history.has_cooldown(100, 9, 2_000));
        assert_eq!(history.remaining_cooldown_ms(100, 9, 2_000), 2_000);
        assert_eq!(history.remaining_category_cooldown_ms(9, 2_000), 500);

        assert!(!history.add_cooldown(100, 7, 2_000, 9, 1_500, false));
        assert_eq!(
            history
                .cooldown(100)
                .map(|cooldown| cooldown.cooldown_end_ms),
            Some(4_000)
        );

        assert!(history.start_cooldown(2_000, 101, 0, 1, 11, 1, true));
        let held = history.cooldown(101).expect("on-hold cooldown");
        assert!(held.on_hold);
        assert_eq!(held.cooldown_end_ms, 2_000 + INFINITY_COOLDOWN_DELAY_MS);
        assert_eq!(held.category_end_ms, 2_000 + INFINITY_COOLDOWN_DELAY_MS);

        assert!(history.modify_cooldown(100, -2_000, false, 2_500));
        assert_eq!(history.cooldown(100), None);
        assert!(!history.has_cooldown(100, 9, 2_500));

        history.update(2_501);
        assert!(!history.has_cooldown(100, 9, 2_501));
        assert!(history.has_cooldown(101, 11, 2_501));
    }

    #[test]
    fn spell_history_charges_school_locks_gcd_and_duel_snapshot_match_cpp_shape() {
        let mut history = SpellHistory::default();

        assert!(history.consume_charge(44, 1_000, 5_000, 2));
        assert!(history.consume_charge(44, 1_500, 5_000, 2));
        assert!(!history.has_charge(44, 2));
        assert_eq!(history.consumed_charges(44), 2);
        assert_eq!(
            history
                .charges(44)
                .and_then(|charges| charges.front())
                .map(|charge| charge.recharge_end_ms),
            Some(6_000)
        );

        assert!(history.modify_charge_recovery_time(44, -1_000, 1_500));
        assert_eq!(
            history
                .charges(44)
                .and_then(|charges| charges.front())
                .map(|charge| charge.recharge_end_ms),
            Some(5_000)
        );
        assert!(history.restore_charge(44));
        assert_eq!(history.consumed_charges(44), 1);
        history.update(5_000);
        assert_eq!(history.consumed_charges(44), 0);

        history.lock_spell_school(0b0010_1000, 10_000, 3_000);
        assert!(history.is_school_locked(0b0000_1000, 12_000));
        assert!(history.is_school_locked(0b0010_0000, 12_000));
        assert!(!history.is_school_locked(0b0000_1000, 13_001));

        history.add_global_cooldown(12, 20_000, 1_500);
        assert!(history.has_global_cooldown(12, 21_000));
        assert_eq!(history.remaining_global_cooldown_ms(12, 21_000), 500);
        history.cancel_global_cooldown(12);
        assert!(!history.has_global_cooldown(12, 21_000));

        history.start_cooldown(30_000, 777, 0, 10_000, 55, 5_000, false);
        history.save_cooldown_state_before_duel();
        history.start_cooldown(31_000, 888, 0, 10_000, 66, 5_000, false);
        history.restore_cooldown_state_after_duel();
        assert!(history.has_cooldown(777, 55, 31_000));
        assert!(!history.has_cooldown(888, 66, 31_000));
        assert_eq!(history.category_cooldowns.get(&55), Some(&777));
    }

    #[test]
    fn current_spell_slots_match_trinity_values_and_roundtrip() {
        assert_eq!(CurrentSpellSlot::Melee as u8, 0);
        assert_eq!(CurrentSpellSlot::Generic as u8, 1);
        assert_eq!(CurrentSpellSlot::Channeled as u8, 2);
        assert_eq!(CurrentSpellSlot::Autorepeat as u8, 3);
        assert_eq!(CURRENT_FIRST_NON_MELEE_SPELL, 1);
        assert_eq!(CURRENT_MAX_SPELL, 4);

        let caster = guid(4);
        let mut spells = SpellSubsystem::default();
        let slots = [
            CurrentSpellSlot::Melee,
            CurrentSpellSlot::Generic,
            CurrentSpellSlot::Channeled,
            CurrentSpellSlot::Autorepeat,
        ];

        for (index, slot) in slots.into_iter().enumerate() {
            let spell = CurrentSpellRef::new(300 + index as u32, Some(caster), None);
            spells.set_current_spell(slot, spell);
            assert_eq!(spells.current_spell(slot), Some(spell));
            assert_eq!(spells.clear_current_spell(slot), Some(spell));
            assert_eq!(spells.current_spell(slot), None);
        }
    }

    #[test]
    fn threat_combat_helpers_roundtrip() {
        let mut combat = CombatSubsystem::default();
        let attacker = guid(10);

        assert!(!combat.combat_disallowed);
        assert_eq!(combat.threat_update_timer_ms, THREAT_UPDATE_INTERVAL_MS);
        assert_eq!(combat.add_threat(attacker, 5.0), 5.0);
        assert_eq!(combat.add_threat(attacker, 2.5), 7.5);
        assert!(combat.is_threatened_by(attacker));
        assert_eq!(combat.threat_value(attacker), Some(7.5));
        combat.set_threat(attacker, 1.0);
        assert_eq!(combat.remove_threat(attacker), Some(1.0));

        assert!(combat.add_attacker(attacker));
        combat.set_attacking(Some(attacker));
        combat.combat_disallowed = true;
        assert!(combat.attackers.contains(&attacker));
        assert_eq!(combat.attacking_guid, Some(attacker));
        assert!(combat.combat_disallowed);
        assert!(combat.remove_attacker(attacker));
        combat.clear_attackers();
        assert!(combat.attackers.is_empty());
        assert_eq!(combat.attacking_guid, None);
    }

    #[test]
    fn threat_refs_sort_and_scale_like_cpp_threat_manager_shape() {
        let mut combat = CombatSubsystem::default();
        let low = guid(20);
        let high = guid(21);
        let taunter = guid(22);
        let offline = guid(23);

        combat.initialize_threat_list_capability(true);
        assert!(combat.owner_can_have_threat_list);
        assert_eq!(combat.add_threat(low, 100.0), 100.0);
        assert_eq!(combat.add_threat(high, 120.0), 120.0);
        assert_eq!(combat.add_threat(taunter, 1.0), 1.0);
        assert_eq!(combat.add_threat(offline, 999.0), 999.0);
        assert!(combat.set_threat_taunt_state(taunter, ThreatTauntState::Taunt));
        assert!(combat.set_threat_online_state(offline, ThreatOnlineState::Offline));

        assert_eq!(
            combat.sorted_threat_guids(),
            vec![taunter, high, low, offline]
        );
        assert_eq!(combat.threat_list_size(), 4);
        assert!(!combat.is_threat_list_empty(false));
        assert!(combat.is_threatened_by_with_offline(offline, true));
        assert!(!combat.is_threatened_by(offline));

        assert_eq!(combat.modify_threat_by_percent(high, -50), Some(60.0));
        assert_eq!(combat.scale_threat(low, 2.0), Some(200.0));
        assert_eq!(combat.threat_value(low), Some(200.0));
        assert_eq!(
            combat.threat_ref(low).map(|state| state.threat()),
            Some(200.0)
        );

        combat.reset_all_threat();
        assert_eq!(combat.threat_value(low), Some(0.0));
        assert!(combat.need_client_update);
    }

    #[test]
    fn threat_reselect_victim_matches_cpp_110_130_and_fixate_shape() {
        let mut combat = CombatSubsystem::default();
        let current = guid(30);
        let ranged = guid(31);
        let melee = guid(32);

        combat.add_threat(current, 100.0);
        combat.current_victim_guid = Some(current);
        combat.add_threat(ranged, 120.0);
        assert_eq!(combat.reselect_victim(false, false), Some(current));

        combat.set_threat(ranged, 131.0);
        assert_eq!(combat.reselect_victim(false, false), Some(ranged));

        combat.current_victim_guid = Some(current);
        combat.set_threat(ranged, 120.0);
        assert_eq!(combat.reselect_victim(false, true), Some(ranged));

        combat.add_threat(melee, 1.0);
        assert!(combat.fixate_target(Some(melee)));
        assert_eq!(combat.reselect_victim(false, false), Some(melee));
        assert!(combat.fixate_target(None));
        assert!(!combat.fixate_target(Some(guid(99))));
    }

    #[test]
    fn combat_refs_track_pve_pvp_suppression_and_timeout_like_cpp() {
        let mut combat = CombatSubsystem::default();
        let creature = guid(40);
        let player = guid(41);

        assert!(combat.set_in_combat_with(creature, false, false));
        assert!(combat.has_pve_combat());
        assert!(combat.is_in_combat_with(creature));

        assert!(combat.set_in_combat_with(player, true, false));
        assert!(combat.has_pvp_combat());
        assert_eq!(
            combat
                .pvp_refs
                .get(&player)
                .and_then(|reference| reference.timeout_ms),
            Some(PVP_COMBAT_TIMEOUT_MS)
        );

        combat.suppress_pvp_combat();
        assert!(!combat.has_pvp_combat());
        assert!(combat.set_in_combat_with(player, true, false));
        assert!(combat.has_pvp_combat());

        assert!(
            combat
                .update_pvp_combat(PVP_COMBAT_TIMEOUT_MS - 1)
                .is_empty()
        );
        assert_eq!(combat.update_pvp_combat(1), vec![player]);
        assert!(!combat.has_pvp_combat());

        combat.end_all_pve_combat();
        assert!(!combat.has_pve_combat());
        assert!(!combat.has_combat());
    }

    #[test]
    fn combat_can_begin_matches_cpp_guard_order_shape() {
        let valid = CombatBeginContextLikeCpp {
            attacker_in_world: true,
            victim_in_world: true,
            attacker_alive: true,
            victim_alive: true,
            same_map: true,
            same_phase: true,
            ..Default::default()
        };

        assert!(CombatSubsystem::can_begin_combat_like_cpp(valid));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                same_unit: true,
                ..valid
            }
        ));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                attacker_in_world: false,
                ..valid
            }
        ));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                victim_alive: false,
                ..valid
            }
        ));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                same_map: false,
                ..valid
            }
        ));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                same_phase: false,
                ..valid
            }
        ));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                attacker_unit_state: UnitState::EVADE.bits(),
                ..valid
            }
        ));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                victim_unit_state: UnitState::IN_FLIGHT.bits(),
                ..valid
            }
        ));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                attacker_combat_disallowed: true,
                ..valid
            }
        ));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                relation_represented: true,
                victim_is_friendly_to_attacker: true,
                ..valid
            }
        ));
        assert!(!CombatSubsystem::can_begin_combat_like_cpp(
            CombatBeginContextLikeCpp {
                attacker_or_owner_player_is_game_master: true,
                ..valid
            }
        ));
    }

    #[test]
    fn combat_revalidate_removes_invalid_refs_and_related_threat_like_cpp() {
        let mut combat = CombatSubsystem::default();
        let valid_pve = guid(42);
        let invalid_pve = guid(43);
        let invalid_pvp = guid(44);

        combat.set_in_combat_with(valid_pve, false, false);
        combat.set_in_combat_with(invalid_pve, false, false);
        combat.set_in_combat_with(invalid_pvp, true, false);
        combat.add_threat(valid_pve, 10.0);
        combat.add_threat(invalid_pve, 20.0);
        combat.put_threatened_by_me_ref(invalid_pve, ThreatReferenceState::default());

        let removed =
            combat.revalidate_combat_like_cpp(|guid, _| guid != invalid_pve && guid != invalid_pvp);

        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&invalid_pve));
        assert!(removed.contains(&invalid_pvp));
        assert!(combat.is_in_combat_with(valid_pve));
        assert!(!combat.is_in_combat_with(invalid_pve));
        assert!(!combat.is_in_combat_with(invalid_pvp));
        assert_eq!(combat.threat_value(valid_pve), Some(10.0));
        assert_eq!(combat.threat_value(invalid_pve), None);
        assert!(!combat.is_threatening_to(invalid_pve, true));
    }

    #[test]
    fn combat_purge_ref_removes_ref_and_related_threat_like_cpp_end_combat_side() {
        let mut combat = CombatSubsystem::default();
        let target = guid(45);
        combat.set_in_combat_with(target, false, false);
        combat.add_threat(target, 30.0);
        combat.put_threatened_by_me_ref(target, ThreatReferenceState::default());

        assert!(combat.purge_combat_ref_like_cpp(target));
        assert!(!combat.is_in_combat_with(target));
        assert_eq!(combat.threat_value(target), None);
        assert!(!combat.is_threatening_to(target, true));
        assert!(!combat.purge_combat_ref_like_cpp(target));
    }

    #[test]
    fn threatened_by_me_refs_follow_cpp_reverse_lookup_shape() {
        let mut combat = CombatSubsystem::default();
        let owner = guid(50);
        let mut reference = ThreatReferenceState::default();
        reference.set_online_state(ThreatOnlineState::Suppressed);
        reference.base_amount = 10.0;

        combat.put_threatened_by_me_ref(owner, reference);
        assert!(combat.is_threatening_anyone(false));
        assert!(combat.is_threatening_to(owner, false));
        combat
            .threatened_by_me
            .get_mut(&owner)
            .expect("reverse threat ref")
            .set_online_state(ThreatOnlineState::Offline);
        reference.set_online_state(ThreatOnlineState::Offline);
        assert!(!combat.is_threatening_anyone(false));
        assert!(combat.is_threatening_anyone(true));
        assert_eq!(combat.purge_threatened_by_me_ref(owner), Some(reference));
        assert!(!combat.is_threatening_anyone(true));
    }

    #[test]
    fn motion_generator_ids_slots_and_priorities_match_cpp_motion_master_shape() {
        assert_eq!(MovementGeneratorKind::Idle.trinity_id(), 0);
        assert_eq!(MovementGeneratorKind::Random.trinity_id(), 1);
        assert_eq!(MovementGeneratorKind::Waypoint.trinity_id(), 2);
        assert_eq!(MovementGeneratorKind::from_trinity_id(3), None);
        assert_eq!(
            MovementGeneratorKind::from_trinity_id(14),
            Some(MovementGeneratorKind::Follow)
        );
        assert_eq!(
            MovementGeneratorKind::from_trinity_id(18),
            Some(MovementGeneratorKind::Formation)
        );
        assert_eq!(MovementSlot::Default as u8, 0);
        assert_eq!(MovementSlot::Active as u8, 1);

        let mut motion = MotionSubsystem::default();
        motion.add_to_world();
        assert_eq!(motion.size(), 1);
        assert_eq!(motion.current_slot(), MovementSlot::Default);
        assert_eq!(
            motion.current_movement_generator().kind,
            MovementGeneratorKind::Idle
        );
        assert_eq!(
            motion.current_movement_generator().priority,
            MovementGeneratorPriority::Normal
        );
        assert!(
            motion
                .current_movement_generator()
                .has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED)
        );

        motion.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Follow, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_target_guid(guid(30)),
        );
        assert_eq!(motion.current_slot(), MovementSlot::Active);
        assert_eq!(
            motion.current_movement_generator().kind,
            MovementGeneratorKind::Follow
        );

        motion.move_charge(42);
        let current = motion.current_movement_generator();
        assert_eq!(current.kind, MovementGeneratorKind::Point);
        assert_eq!(current.priority, MovementGeneratorPriority::Highest);
        assert_eq!(current.base_unit_state, UnitState::CHARGING.bits());
        assert!(current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
        assert_eq!(
            motion.base_unit_states.get(&UnitState::CHARGING.bits()),
            Some(&1)
        );
        assert!(
            motion
                .active_generators
                .iter()
                .any(|generator| generator.kind == MovementGeneratorKind::Follow
                    && generator.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED))
        );

        let removed = motion.clear_by_priority(MovementGeneratorPriority::Highest);
        assert_eq!(removed.len(), 1);
        assert_eq!(
            motion.base_unit_states.get(&UnitState::CHARGING.bits()),
            None
        );
        assert_eq!(
            motion.current_movement_generator().kind,
            MovementGeneratorKind::Follow
        );
    }

    #[test]
    fn motion_direct_initialize_preserves_selected_waypoint_default_like_cpp() {
        let mut motion = MotionSubsystem::default();
        motion.initialize_default_generator_like_cpp(MovementGeneratorKind::Waypoint);
        motion.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal),
        );

        motion.direct_initialize_like_cpp();

        assert!(motion.active_generators.is_empty());
        let current = motion.current_movement_generator();
        assert_eq!(
            current.kind,
            MovementGeneratorKind::Waypoint,
            "C++ MotionMaster::DirectInitialize clears generators then InitializeDefault selects owner GetDefaultMovementType(), not unconditional idle"
        );
        assert_eq!(current.priority, MovementGeneratorPriority::Normal);
        assert_eq!(current.base_unit_state, UnitState::ROAMING.bits());
        assert!(current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
        assert!(!current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));
    }

    #[test]
    fn motion_master_flags_and_delayed_actions_match_cpp_shape() {
        assert_eq!(MOTIONMASTER_FLAG_NONE, 0x0);
        assert_eq!(MOTIONMASTER_FLAG_UPDATE, 0x1);
        assert_eq!(MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING, 0x2);
        assert_eq!(MOTIONMASTER_FLAG_INITIALIZATION_PENDING, 0x4);
        assert_eq!(MOTIONMASTER_FLAG_INITIALIZING, 0x8);
        assert_eq!(
            MOTIONMASTER_FLAG_DELAYED,
            MOTIONMASTER_FLAG_UPDATE | MOTIONMASTER_FLAG_INITIALIZATION_PENDING
        );

        assert_eq!(MotionMasterDelayedActionType::Clear.trinity_id(), 0);
        assert_eq!(MotionMasterDelayedActionType::ClearSlot.trinity_id(), 1);
        assert_eq!(MotionMasterDelayedActionType::ClearMode.trinity_id(), 2);
        assert_eq!(MotionMasterDelayedActionType::ClearPriority.trinity_id(), 3);
        assert_eq!(MotionMasterDelayedActionType::Add.trinity_id(), 4);
        assert_eq!(MotionMasterDelayedActionType::Remove.trinity_id(), 5);
        assert_eq!(MotionMasterDelayedActionType::RemoveType.trinity_id(), 6);
        assert_eq!(MotionMasterDelayedActionType::Initialize.trinity_id(), 7);
        assert_eq!(
            MotionMasterDelayedActionType::from_trinity_id(6),
            Some(MotionMasterDelayedActionType::RemoveType)
        );
        assert_eq!(MotionMasterDelayedActionType::from_trinity_id(8), None);

        let mut motion = MotionSubsystem::default();
        assert!(motion.should_delay_motion_master_action_like_cpp());
        motion.flags = MOTIONMASTER_FLAG_UPDATE;
        assert!(motion.should_delay_motion_master_action_like_cpp());
        motion.flags = MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
        assert!(!motion.should_delay_motion_master_action_like_cpp());
        motion.flags = MOTIONMASTER_FLAG_INITIALIZING;
        assert!(!motion.should_delay_motion_master_action_like_cpp());

        motion.push_delayed_action_like_cpp(MotionMasterDelayedActionType::Add);
        motion.push_delayed_action_with_validator_like_cpp(
            MotionMasterDelayedActionType::RemoveType,
            false,
        );
        motion.push_delayed_action_like_cpp(MotionMasterDelayedActionType::Initialize);

        let resolved = motion.resolve_delayed_actions_like_cpp();
        assert_eq!(
            resolved,
            vec![
                MotionMasterResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::Add,
                    executed: true,
                },
                MotionMasterResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::RemoveType,
                    executed: false,
                },
                MotionMasterResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::Initialize,
                    executed: true,
                },
            ]
        );
        assert!(motion.delayed_actions.is_empty());
    }

    #[test]
    fn motion_master_delayed_action_payloads_apply_fifo_like_cpp() {
        let mut motion = MotionSubsystem::default();
        motion.add_to_world();
        motion.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Follow, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_base_unit_state(UnitState::FOLLOW.bits()),
        );
        motion.push_delayed_payload_like_cpp(MotionMasterDelayedActionPayload::Add(
            MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Highest)
                .with_base_unit_state(UnitState::JUMPING.bits())
                .with_movement_id(7),
        ));
        motion.push_delayed_payload_with_validator_like_cpp(
            MotionMasterDelayedActionPayload::RemoveType {
                kind: MovementGeneratorKind::Effect,
                slot: MovementSlot::Active,
            },
            false,
        );
        motion.push_delayed_payload_like_cpp(MotionMasterDelayedActionPayload::ClearPriority(
            MovementGeneratorPriority::Highest,
        ));

        let resolved = motion.resolve_delayed_action_payloads_like_cpp();
        assert_eq!(
            resolved,
            vec![
                MotionMasterResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::Add,
                    executed: true,
                },
                MotionMasterResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::RemoveType,
                    executed: false,
                },
                MotionMasterResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::ClearPriority,
                    executed: true,
                },
            ]
        );
        assert!(motion.delayed_actions.is_empty());
        assert_eq!(
            motion.current_movement_generator().kind,
            MovementGeneratorKind::Follow
        );
        assert_eq!(
            motion.base_unit_states.get(&UnitState::JUMPING.bits()),
            None
        );
        assert_eq!(
            motion.base_unit_states.get(&UnitState::FOLLOW.bits()),
            Some(&1)
        );
    }

    #[test]
    fn motion_master_update_initializes_updates_pops_and_resolves_like_cpp() {
        let mut motion = MotionSubsystem::default();
        assert_eq!(
            motion.update_motion_master_like_cpp(MotionMasterUpdateContext {
                diff_ms: 10,
                spline_finalized: true,
                ..MotionMasterUpdateContext::default()
            }),
            MotionMasterUpdateOutcome::Stalled
        );
        motion.add_to_world();
        motion.launch_generic_movement(MovementGeneratorKind::Effect, 11, 10, None);
        motion.push_delayed_payload_like_cpp(MotionMasterDelayedActionPayload::Add(
            MovementGeneratorRef::new(MovementGeneratorKind::Follow, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_base_unit_state(UnitState::FOLLOW.bits()),
        ));

        let outcome = motion.update_motion_master_like_cpp(MotionMasterUpdateContext {
            diff_ms: 10,
            ..MotionMasterUpdateContext::default()
        });

        let mut expected_popped =
            MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(
                    MOVEMENTGENERATOR_FLAG_INITIALIZED | MOVEMENTGENERATOR_FLAG_INFORM_ENABLED,
                )
                .with_base_unit_state(UnitState::ROAMING.bits())
                .with_movement_id(11)
                .with_duration_ms(10);
        expected_popped.elapsed_ms = 10;
        assert_eq!(
            outcome,
            MotionMasterUpdateOutcome::Updated {
                popped: Some(expected_popped),
                resolved_delayed_actions: vec![MotionMasterResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::Add,
                    executed: true,
                }],
            }
        );
        assert!(!motion.has_motion_master_flag(MOTIONMASTER_FLAG_UPDATE));
        assert_eq!(
            motion.current_movement_generator().kind,
            MovementGeneratorKind::Follow
        );
        assert_eq!(
            motion.base_unit_states.get(&UnitState::ROAMING.bits()),
            None
        );
        assert_eq!(
            motion.base_unit_states.get(&UnitState::FOLLOW.bits()),
            Some(&1)
        );
    }

    #[test]
    fn idle_rotate_and_distract_generators_match_cpp_lifecycle_shape() {
        let mut idle = MotionSubsystem::default().default_generator;
        assert_eq!(
            idle.initialize_idle_like_cpp(),
            IdleMovementAction::StopMoving
        );
        assert_eq!(idle.reset_idle_like_cpp(), IdleMovementAction::StopMoving);
        assert!(idle.update_idle_like_cpp());
        idle.finalize_idle_like_cpp();
        assert!(idle.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));

        let mut motion = MotionSubsystem::default();
        assert!(!motion.move_rotate_like_cpp(7, 0, RotateDirection::Left));
        assert!(motion.move_rotate_like_cpp(7, 1_000, RotateDirection::Left));
        let mut rotate = motion.current_movement_generator();
        assert_eq!(rotate.kind, MovementGeneratorKind::Rotate);
        assert_eq!(rotate.priority, MovementGeneratorPriority::Normal);
        assert_eq!(rotate.base_unit_state, UnitState::ROTATING.bits());
        assert_eq!(rotate.movement_id, 7);
        assert_eq!(rotate.duration_ms, Some(1_000));
        assert_eq!(rotate.max_duration_ms, Some(1_000));
        assert_eq!(rotate.rotate_direction, Some(RotateDirection::Left));

        assert_eq!(
            rotate.initialize_rotate_like_cpp(),
            IdleMovementAction::StopMoving
        );
        assert!(rotate.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));
        let update = rotate.update_rotate_like_cpp(true, 250, 0.0);
        assert!(update.keep_running);
        assert_eq!(rotate.duration_ms, Some(750));
        assert!(
            update
                .facing_angle
                .is_some_and(|angle| (angle - std::f32::consts::FRAC_PI_2).abs() < 0.0001)
        );

        let finished = rotate.update_rotate_like_cpp(true, 750, std::f32::consts::FRAC_PI_2);
        assert!(!finished.keep_running);
        assert!(rotate.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));
        assert_eq!(
            rotate.finalize_rotate_like_cpp(true, true),
            RotateMovementFinalize {
                inform: Some(PointMovementInform {
                    kind: MovementGeneratorKind::Rotate,
                    movement_id: 7,
                }),
            }
        );
        assert!(rotate.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));

        let mut right =
            MovementGeneratorRef::new(MovementGeneratorKind::Rotate, MovementSlot::Active)
                .with_duration_ms(1_000)
                .with_max_duration_ms(1_000)
                .with_rotate_direction(RotateDirection::Right);
        let right_update = right.update_rotate_like_cpp(true, 250, std::f32::consts::PI);
        assert!(
            right_update
                .facing_angle
                .is_some_and(|angle| (angle - std::f32::consts::FRAC_PI_2).abs() < 0.0001)
        );

        let mut distract_motion = MotionSubsystem::default();
        distract_motion.move_distract_like_cpp(500);
        let mut distract = distract_motion.current_movement_generator();
        assert_eq!(distract.kind, MovementGeneratorKind::Distract);
        assert_eq!(distract.priority, MovementGeneratorPriority::Highest);
        assert_eq!(distract.base_unit_state, UnitState::DISTRACTED.bits());
        assert_eq!(distract.duration_ms, Some(500));
        assert_eq!(
            distract.initialize_distract_like_cpp(false),
            DistractMovementAction {
                stand_up: true,
                launch_facing_spline: true,
            }
        );
        assert!(distract.update_distract_like_cpp(true, 500));
        assert_eq!(distract.duration_ms, Some(0));
        assert!(!distract.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));
        assert!(!distract.update_distract_like_cpp(true, 1));
        assert!(distract.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));
        assert_eq!(
            distract.finalize_distract_like_cpp(true, true),
            DistractMovementFinalize {
                set_home_orientation: true,
            }
        );

        let mut deactivated =
            MovementGeneratorRef::new(MovementGeneratorKind::Distract, MovementSlot::Active);
        deactivated.deactivate_timed_idle_like_cpp();
        assert!(deactivated.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED));
    }

    #[test]
    fn motion_move_point_tracks_cpp_point_generator_base_state() {
        let mut motion = MotionSubsystem::default();

        motion.move_point(9);

        let current = motion.current_movement_generator();
        assert_eq!(current.kind, MovementGeneratorKind::Point);
        assert_eq!(current.priority, MovementGeneratorPriority::Normal);
        assert_eq!(current.base_unit_state, UnitState::ROAMING.bits());
        assert_eq!(current.movement_id, 9);
        assert!(current.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
        assert_eq!(
            motion.base_unit_states.get(&UnitState::ROAMING.bits()),
            Some(&1)
        );

        let removed = motion.clear_by_priority(MovementGeneratorPriority::Normal);
        assert_eq!(removed.len(), 1);
        assert_eq!(
            motion.base_unit_states.get(&UnitState::ROAMING.bits()),
            None
        );
    }

    #[test]
    fn point_movement_generator_lifecycle_matches_cpp_shape() {
        let mut generator =
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(
                    MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING
                        | MOVEMENTGENERATOR_FLAG_DEACTIVATED,
                )
                .with_base_unit_state(UnitState::ROAMING.bits())
                .with_movement_id(9);

        assert_eq!(
            generator.initialize_point_like_cpp(true),
            PointMovementAction::LaunchSpline
        );
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));
        assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
        assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED));

        assert_eq!(
            generator.update_point_like_cpp(true, false),
            PointMovementAction::Continue
        );
        assert_eq!(
            generator.update_point_like_cpp(true, true),
            PointMovementAction::Finished
        );
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));

        let finalized = generator.finalize_point_like_cpp(true, true);
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));
        assert_eq!(
            finalized,
            PointMovementFinalize {
                clear_roaming_move: true,
                inform: Some(PointMovementInform {
                    kind: MovementGeneratorKind::Point,
                    movement_id: 9,
                }),
            }
        );

        let mut blocked =
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING);
        assert_eq!(
            blocked.initialize_point_like_cpp(false),
            PointMovementAction::StopMoving
        );
        assert!(blocked.has_flag(MOVEMENTGENERATOR_FLAG_INTERRUPTED));
        assert_eq!(
            blocked.update_point_like_cpp(false, false),
            PointMovementAction::StopMovingAndContinue
        );

        let mut speed_update =
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_flags(MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING);
        assert_eq!(
            speed_update.update_point_like_cpp(true, false),
            PointMovementAction::RelaunchSpline
        );
        assert!(!speed_update.has_flag(MOVEMENTGENERATOR_FLAG_SPEED_UPDATE_PENDING));

        let mut interrupted =
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_flags(MOVEMENTGENERATOR_FLAG_INTERRUPTED);
        assert_eq!(
            interrupted.update_point_like_cpp(true, true),
            PointMovementAction::RelaunchSpline
        );
        assert!(!interrupted.has_flag(MOVEMENTGENERATOR_FLAG_INTERRUPTED));
    }

    #[test]
    fn point_movement_charge_prepath_informs_as_event_charge_like_cpp() {
        let mut generator =
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Highest)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::CHARGING.bits())
                .with_movement_id(EVENT_CHARGE_PREPATH);

        assert_eq!(
            generator.initialize_point_like_cpp(true),
            PointMovementAction::MarkRoamingMove
        );
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));

        assert_eq!(
            generator.update_point_like_cpp(true, false),
            PointMovementAction::Continue
        );
        assert_eq!(
            generator.update_point_like_cpp(true, true),
            PointMovementAction::Finished
        );
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));

        assert_eq!(
            generator.finalize_point_like_cpp(true, true),
            PointMovementFinalize {
                clear_roaming_move: true,
                inform: Some(PointMovementInform {
                    kind: MovementGeneratorKind::Point,
                    movement_id: EVENT_CHARGE,
                }),
            }
        );

        let mut deactivated =
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active);
        assert_eq!(
            deactivated.deactivate_point_like_cpp(),
            PointMovementAction::ClearRoamingMove
        );
        assert!(deactivated.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED));
    }

    #[test]
    fn assistance_movement_generators_match_cpp_constructor_and_finalize_shape() {
        let mut motion = MotionSubsystem::default();

        assert_eq!(
            motion.move_seek_assistance_like_cpp(),
            SeekAssistancePlan {
                attack_stop: true,
                cast_stop: true,
                do_not_reacquire_spell_focus_target: true,
                set_react_passive: true,
                generator_added: true,
            }
        );

        let assist = motion.current_movement_generator();
        assert_eq!(assist.kind, MovementGeneratorKind::Assistance);
        assert_eq!(assist.priority, MovementGeneratorPriority::Normal);
        assert_eq!(assist.base_unit_state, UnitState::ROAMING.bits());
        assert_eq!(assist.movement_id, EVENT_ASSIST_MOVE);
        assert!(assist.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));

        let mut finalized = assist.with_flags(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED);
        assert_eq!(
            finalized.finalize_assistance_like_cpp(true, true, true, true),
            AssistanceMovementFinalize {
                clear_roaming_move: true,
                set_no_call_assistance: Some(false),
                call_assistance: true,
                seek_assistance_distract_ms: Some(CREATURE_FAMILY_ASSISTANCE_DELAY_MS_LIKE_CPP),
            }
        );
        assert!(finalized.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));

        let mut non_creature = assist.with_flags(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED);
        assert_eq!(
            non_creature.finalize_assistance_like_cpp(true, true, false, true),
            AssistanceMovementFinalize {
                clear_roaming_move: true,
                set_no_call_assistance: None,
                call_assistance: false,
                seek_assistance_distract_ms: None,
            }
        );

        motion.move_seek_assistance_distract_like_cpp(777);
        let distract = motion.current_movement_generator();
        assert_eq!(distract.kind, MovementGeneratorKind::AssistanceDistract);
        assert_eq!(distract.priority, MovementGeneratorPriority::Normal);
        assert_eq!(distract.base_unit_state, UnitState::DISTRACTED.bits());
        assert_eq!(distract.duration_ms, Some(777));
        assert!(distract.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));

        let mut distract_finalized = distract.with_flags(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED);
        assert_eq!(
            distract_finalized.finalize_assistance_distract_like_cpp(true, true),
            AssistanceDistractFinalize {
                set_react_aggressive: true,
            }
        );
        assert!(distract_finalized.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));
    }

    #[test]
    fn generic_movement_generator_lifecycle_matches_cpp_shape() {
        let mut motion = MotionSubsystem::default();
        let target = guid(88);

        motion.launch_generic_movement(
            MovementGeneratorKind::Effect,
            42,
            1_000,
            Some((1234, target)),
        );

        let mut generator = motion.current_movement_generator();
        assert_eq!(generator.kind, MovementGeneratorKind::Effect);
        assert_eq!(generator.priority, MovementGeneratorPriority::Normal);
        assert_eq!(
            generator.flags,
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING
        );
        assert_eq!(generator.base_unit_state, UnitState::ROAMING.bits());
        assert_eq!(generator.movement_id, 42);
        assert_eq!(generator.duration_ms, Some(1_000));
        assert_eq!(generator.arrival_spell_id, 1234);
        assert_eq!(generator.arrival_spell_target_guid, target);
        assert_eq!(
            motion.base_unit_states.get(&UnitState::ROAMING.bits()),
            Some(&1)
        );

        generator.initialize_generic_like_cpp();
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZED));
        assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));

        assert!(generator.update_generic_like_cpp(999, false, false));
        assert_eq!(generator.elapsed_ms, 999);
        assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));

        assert!(!generator.update_generic_like_cpp(1, false, false));
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));
        let inform = generator
            .finalize_generic_like_cpp(true)
            .expect("inform enabled");
        assert_eq!(
            inform,
            GenericMovementInform {
                kind: MovementGeneratorKind::Effect,
                movement_id: 42,
                arrival_spell_id: Some(1234),
                arrival_spell_target_guid: Some(target),
            }
        );
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));

        let mut cyclic =
            MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZED)
                .with_duration_ms(10);
        assert!(cyclic.update_generic_like_cpp(100, true, false));
        assert_eq!(cyclic.elapsed_ms, 0);
        assert!(!cyclic.update_generic_like_cpp(0, true, true));
        assert!(cyclic.has_flag(MOVEMENTGENERATOR_FLAG_INFORM_ENABLED));

        let mut deactivated =
            MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
                .with_flags(MOVEMENTGENERATOR_FLAG_DEACTIVATED);
        deactivated.initialize_generic_like_cpp();
        assert!(deactivated.has_flag(MOVEMENTGENERATOR_FLAG_FINALIZED));
        assert!(!deactivated.has_flag(MOVEMENTGENERATOR_FLAG_DEACTIVATED));
    }

    #[test]
    fn launch_move_spline_like_cpp_rejects_invalid_generator_types() {
        let mut motion = MotionSubsystem::default();

        assert!(!motion.launch_move_spline_like_cpp(
            MovementGeneratorKind::Custom(3),
            7,
            MovementGeneratorPriority::Highest,
            250
        ));
        assert!(motion.active_generators.is_empty());

        assert!(!motion.launch_move_spline_like_cpp(
            MovementGeneratorKind::Custom(19),
            7,
            MovementGeneratorPriority::Highest,
            250
        ));
        assert!(motion.active_generators.is_empty());

        assert!(motion.launch_move_spline_like_cpp(
            MovementGeneratorKind::Point,
            7,
            MovementGeneratorPriority::Highest,
            250
        ));
        let generator = motion.current_movement_generator();
        assert_eq!(generator.kind, MovementGeneratorKind::Point);
        assert_eq!(generator.priority, MovementGeneratorPriority::Highest);
        assert_eq!(generator.base_unit_state, UnitState::ROAMING.bits());
        assert_eq!(generator.movement_id, 7);
        assert_eq!(generator.duration_ms, Some(250));
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    }

    #[test]
    fn move_jump_generators_match_cpp_priority_state_and_persist_flags() {
        let mut motion = MotionSubsystem::default();
        let target = guid(99);

        assert!(!motion.move_jump_like_cpp(1, 500, 0.009, Some((777, target))));
        assert!(motion.active_generators.is_empty());

        assert!(motion.move_jump_like_cpp(1, 500, 0.01, Some((777, target))));
        let jump = motion.current_movement_generator();
        assert_eq!(jump.kind, MovementGeneratorKind::Effect);
        assert_eq!(jump.priority, MovementGeneratorPriority::Highest);
        assert_eq!(jump.base_unit_state, UnitState::JUMPING.bits());
        assert_eq!(jump.movement_id, 1);
        assert_eq!(jump.duration_ms, Some(500));
        assert_eq!(jump.arrival_spell_id, 777);
        assert_eq!(jump.arrival_spell_target_guid, target);
        assert!(jump.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
        assert!(!jump.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH));
        assert_eq!(
            motion.base_unit_states.get(&UnitState::JUMPING.bits()),
            Some(&1)
        );

        assert!(motion.move_jump_with_gravity_like_cpp(2, 600, 1.0, None));
        let gravity_jump = motion.current_movement_generator();
        assert_eq!(gravity_jump.kind, MovementGeneratorKind::Effect);
        assert_eq!(gravity_jump.priority, MovementGeneratorPriority::Highest);
        assert_eq!(gravity_jump.base_unit_state, UnitState::JUMPING.bits());
        assert_eq!(gravity_jump.movement_id, 2);
        assert_eq!(gravity_jump.duration_ms, Some(600));
        assert_eq!(gravity_jump.arrival_spell_id, 0);
        assert_eq!(gravity_jump.arrival_spell_target_guid, ObjectGuid::EMPTY);
        assert!(gravity_jump.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
        assert!(gravity_jump.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH));
    }

    #[test]
    fn knockback_generator_matches_cpp_player_guard_and_persist_flag() {
        let mut motion = MotionSubsystem::default();

        assert!(!motion.move_knockback_from_like_cpp(true, 300, 1.0));
        assert!(motion.active_generators.is_empty());

        assert!(!motion.move_knockback_from_like_cpp(false, 300, 0.009));
        assert!(motion.active_generators.is_empty());

        assert!(motion.move_knockback_from_like_cpp(false, 300, 0.01));
        let generator = motion.current_movement_generator();
        assert_eq!(generator.kind, MovementGeneratorKind::Effect);
        assert_eq!(generator.priority, MovementGeneratorPriority::Highest);
        assert_eq!(generator.base_unit_state, 0);
        assert_eq!(generator.movement_id, 0);
        assert_eq!(generator.duration_ms, Some(300));
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH));
    }

    #[test]
    fn move_fall_like_cpp_guards_player_and_creature_spline_paths() {
        let mut motion = MotionSubsystem::default();

        assert_eq!(
            motion.move_fall_like_cpp(3, 400, false, 10.0, false, false),
            MoveFallPlan::Noop
        );
        assert_eq!(
            motion.move_fall_like_cpp(3, 400, true, 0.099, false, false),
            MoveFallPlan::Noop
        );
        assert_eq!(
            motion.move_fall_like_cpp(3, 400, true, 10.0, true, false),
            MoveFallPlan::Noop
        );
        assert!(motion.active_generators.is_empty());

        assert_eq!(
            motion.move_fall_like_cpp(3, 400, true, 10.0, false, true),
            MoveFallPlan::PlayerFallInfo
        );
        assert!(motion.active_generators.is_empty());

        assert_eq!(
            motion.move_fall_like_cpp(3, 400, true, 10.0, false, false),
            MoveFallPlan::SplineStarted
        );
        let generator = motion.current_movement_generator();
        assert_eq!(generator.kind, MovementGeneratorKind::Effect);
        assert_eq!(generator.priority, MovementGeneratorPriority::Highest);
        assert_eq!(generator.base_unit_state, 0);
        assert_eq!(generator.movement_id, 3);
        assert_eq!(generator.duration_ms, Some(400));
        assert!(generator.has_flag(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
        assert!(!generator.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH));
    }

    #[test]
    fn motion_stop_on_death_preserves_persistent_generators_like_cpp() {
        let mut motion = MotionSubsystem::default();
        motion.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Effect, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Highest)
                .with_flags(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH),
        );

        assert!(!motion.stop_on_death());
        assert_eq!(
            motion.current_movement_generator().kind,
            MovementGeneratorKind::Effect
        );

        motion.clear_active();
        motion.move_point(9);
        motion.start_spline(7, 1_000);
        assert!(motion.stop_on_death());
        assert_eq!(motion.current_slot(), MovementSlot::Default);
        assert_eq!(
            motion.current_movement_generator().kind,
            MovementGeneratorKind::Idle
        );
        assert!(motion.stopped);
        assert!(!motion.spline.enabled);
    }

    #[test]
    fn move_spline_runtime_state_tracks_cpp_finalized_cyclic_and_destination_shape() {
        let mut motion = MotionSubsystem::default();

        assert!(motion.spline.finalized);
        motion.launch_spline(77, 1_000, (10, 20, 30), false, true, Some(700));
        assert!(motion.spline.enabled);
        assert!(!motion.spline.finalized);
        assert!(motion.spline.on_transport);
        assert_eq!(motion.spline.final_destination, Some((10, 20, 30)));
        assert_eq!(motion.spline.velocity, Some(700));
        assert!(!motion.update_spline(999));
        assert_eq!(motion.spline.progress_ms, 999);
        assert!(motion.update_spline(1));
        assert!(motion.spline.finalized);
        assert!(!motion.spline.enabled);

        motion.launch_spline(78, 1_000, (1, 2, 3), true, false, None);
        assert!(!motion.update_spline(1_250));
        assert!(motion.spline.enabled);
        assert!(!motion.spline.finalized);
        assert_eq!(motion.spline.progress_ms, 250);
        motion.interrupt_spline();
        assert!(motion.spline.finalized);
        assert_eq!(motion.spline.current_destination, None);
    }

    #[test]
    fn ai_stack_lock_and_scheduled_change_follow_cpp_unit_ai_shape() {
        let mut ai = AiSubsystem::default();

        assert!(!ai.is_enabled());
        ai.set_active(Some("NullAI"));
        assert!(ai.is_enabled());
        assert!(ai.update_tick(50));
        assert_eq!(ai.update_ticks, 1);
        assert_eq!(ai.last_update_diff_ms, 50);

        ai.push("CombatAI");
        assert_eq!(ai.active_ai.as_deref(), Some("CombatAI"));
        assert_eq!(ai.ai_stack, vec![String::from("NullAI")]);
        assert_eq!(ai.pop().as_deref(), Some("CombatAI"));
        assert_eq!(ai.active_ai.as_deref(), Some("NullAI"));

        ai.set_locked(true);
        ai.push("ScheduledChangeAI");
        assert_eq!(ai.active_ai.as_deref(), Some("NullAI"));
        assert!(ai.scheduled_change_pending);
        ai.set_locked(false);
        ai.apply_scheduled_change("ScheduledChangeAI", true);
        assert_eq!(ai.active_ai.as_deref(), Some("ScheduledChangeAI"));
        ai.apply_scheduled_change("RestoredAI", false);
        assert_eq!(ai.active_ai.as_deref(), Some("RestoredAI"));
        assert!(!ai.scheduled_change_pending);
    }

    #[test]
    fn control_summon_slots_match_cpp_shared_defines() {
        assert_eq!(SUMMON_SLOT_PET, 0);
        assert_eq!(SUMMON_SLOT_TOTEM, 1);
        assert_eq!(SUMMON_SLOT_TOTEM_2, 2);
        assert_eq!(SUMMON_SLOT_TOTEM_3, 3);
        assert_eq!(SUMMON_SLOT_TOTEM_4, 4);
        assert_eq!(SUMMON_SLOT_MINIPET, 5);
        assert_eq!(SUMMON_SLOT_QUEST, 6);
        assert_eq!(MAX_SUMMON_SLOT, 7);
        assert_eq!(MAX_TOTEM_SLOT, 5);

        let mut control = ControlSubsystem::default();
        let pet = guid(40);
        let totem = guid(41);

        assert_eq!(control.pet_guid(), ObjectGuid::EMPTY);
        control.set_pet_guid(pet);
        assert_eq!(control.pet_guid(), pet);
        assert!(control.set_summon_slot(SUMMON_SLOT_TOTEM_3, totem));
        assert_eq!(control.summon_slots[SUMMON_SLOT_TOTEM_3], totem);
        assert!(!control.set_summon_slot(MAX_SUMMON_SLOT, guid(42)));
        assert_eq!(control.clear_summon_slot(SUMMON_SLOT_TOTEM_3), Some(totem));
        assert_eq!(control.summon_slots[SUMMON_SLOT_TOTEM_3], ObjectGuid::EMPTY);
    }

    #[test]
    fn control_charm_controller_and_target_state_follow_cpp_set_charm() {
        let mut controller = ControlSubsystem::default();
        let mut target = ControlSubsystem::default();
        let controller_guid = guid(50);
        let target_guid = guid(51);
        let other_guid = guid(52);

        controller.apply_charm_as_controller(target_guid, true);
        assert_eq!(controller.charmed_guid, Some(target_guid));
        assert!(controller.controlled_guids.contains(&target_guid));
        assert!(!controller.has_charm_info());

        assert!(target.apply_charmed_by(
            controller_guid,
            CharmType::Possess,
            true,
            Some(123),
            true,
        ));
        assert_eq!(target.charmer_guid, Some(controller_guid));
        assert_eq!(target.charm_type, Some(CharmType::Possess));
        assert_eq!(target.old_faction_id, Some(123));
        assert!(target.walking_before_charm);
        assert!(target.is_charmed());
        assert!(target.is_possessed_by_player());
        assert!(target.has_charm_info());
        assert!(!target.apply_charmed_by(other_guid, CharmType::Charm, false, None, false,));

        assert!(!target.remove_charmed_by(Some(other_guid), false));
        assert!(target.remove_charmed_by(Some(controller_guid), false));
        assert_eq!(target.charmer_guid, None);
        assert_eq!(target.last_charmer_guid, Some(controller_guid));
        assert_eq!(target.charm_type, None);
        assert_eq!(target.old_faction_id, None);
        assert!(!target.has_charm_info());

        controller.remove_charm_as_controller(target_guid, false, true, false);
        assert_eq!(controller.charmed_guid, None);
        assert!(!controller.controlled_guids.contains(&target_guid));
    }

    #[test]
    fn control_remove_charm_preserves_owned_minions_like_cpp() {
        let mut controller = ControlSubsystem::default();
        let minion = guid(60);

        controller.apply_charm_as_controller(minion, false);
        controller.remove_charm_as_controller(minion, true, true, false);
        assert!(controller.controlled_guids.contains(&minion));

        controller.remove_charm_as_controller(minion, true, false, false);
        assert!(!controller.controlled_guids.contains(&minion));
    }

    #[test]
    fn control_remove_vehicle_charm_does_not_mark_last_charmer_like_cpp() {
        let mut passenger = ControlSubsystem::default();
        let vehicle = guid(65);

        assert!(passenger.apply_charmed_by(vehicle, CharmType::Vehicle, true, Some(321), false,));
        assert_eq!(passenger.charmer_guid, Some(vehicle));
        assert_eq!(passenger.charm_type, Some(CharmType::Vehicle));
        assert!(!passenger.has_charm_info());

        assert!(passenger.remove_charmed_by(Some(vehicle), false));
        assert_eq!(passenger.charmer_guid, None);
        assert_eq!(passenger.last_charmer_guid, None);
        assert_eq!(passenger.charm_type, None);
        assert_eq!(passenger.old_faction_id, None);
    }

    #[test]
    fn charm_info_direct_control_and_shared_vision_helpers_roundtrip() {
        let mut control = ControlSubsystem::default();
        let controller = guid(70);
        let controlled = guid(71);
        let observer = guid(72);

        control.set_owner_guid(Some(controller));
        assert_eq!(control.charmer_or_owner_guid(), Some(controller));
        assert_eq!(
            control.charmer_or_owner_or_self_guid(controlled),
            controller
        );

        let charm_info = control.init_charm_info();
        charm_info.pet_number = 9;
        charm_info.command_state = 2;
        charm_info.action_bar[0] = 100;
        charm_info.charm_spells[0] = 200;
        charm_info.is_command_follow = true;
        charm_info.stay_position = Some((1.0, 2.0, 3.0));
        assert!(control.has_charm_info());
        assert_eq!(
            control.charm_info.as_ref().map(|info| info.pet_number),
            Some(9)
        );

        control.add_controlled(controlled);
        control.set_charmed(controlled);
        control.set_moved_unit(Some(controlled));
        control.set_player_moving_me(Some(controller));
        assert!(control.is_possessing_guid(controlled));
        assert_eq!(control.unit_moved_by_me, Some(controlled));
        assert_eq!(control.player_moving_me, Some(controller));

        assert!(control.add_shared_vision(observer));
        assert!(control.has_shared_vision());
        assert!(control.remove_shared_vision(observer));
        assert!(!control.has_shared_vision());

        let removed = control.remove_all_controlled();
        assert_eq!(removed, vec![controlled]);
        assert_eq!(control.charmed_guid, None);
        control.delete_charm_info();
        assert!(!control.has_charm_info());
    }

    #[test]
    fn vehicle_remove_kit_without_kit_returns_before_send_like_cpp() {
        let mut vehicle = VehicleSubsystem::default();

        let remove = vehicle.remove_vehicle_kit_like_cpp(false);

        assert_eq!(remove.kit_id, None);
        assert!(!remove.had_kit);
        assert_eq!(remove.previous_installed, None);
        assert!(!remove.on_remove_from_world);
        assert!(!remove.send_set_vehicle_rec_id_zero_represented);
        assert!(!remove.uninstall_represented);
        assert!(!remove.remove_all_passengers_represented);
        assert!(!remove.script_on_uninstall_represented);
        assert!(!remove.kit_cleared);
        assert_eq!(vehicle.kit, None);
    }

    #[test]
    fn vehicle_remove_existing_kit_sends_rec_id_zero_before_uninstall_like_cpp() {
        let mut vehicle = VehicleSubsystem::default();
        vehicle.set_vehicle_kit(467, true);
        let install = vehicle.install_vehicle_kit_like_cpp();
        assert_eq!(install.kit_id, Some(467));
        assert!(install.installed);

        let remove = vehicle.remove_vehicle_kit_like_cpp(false);

        assert_eq!(remove.kit_id, Some(467));
        assert!(remove.had_kit);
        assert_eq!(remove.previous_installed, Some(true));
        assert!(!remove.on_remove_from_world);
        assert!(remove.send_set_vehicle_rec_id_zero_represented);
        assert!(remove.uninstall_represented);
        assert!(remove.remove_all_passengers_represented);
        assert!(remove.script_on_uninstall_represented);
        assert!(remove.kit_cleared);
        assert_eq!(vehicle.kit, None);
    }

    #[test]
    fn motion_charm_vehicle_and_ai_helpers_roundtrip() {
        let mut subsystems = UnitSubsystems::default();
        let controller = guid(20);
        let controlled = guid(21);
        let vehicle = guid(30);

        subsystems
            .motion
            .set_current_generator(MovementGeneratorKind::Chase);
        subsystems.motion.start_spline(7, 1_000);
        subsystems.motion.set_spline_progress(1_500);
        assert_eq!(
            subsystems.motion.current_generator,
            MovementGeneratorKind::Chase
        );
        assert_eq!(subsystems.motion.spline.progress_ms, 1_000);
        subsystems.motion.pause_movement();
        assert!(subsystems.motion.paused);
        subsystems.motion.resume_movement();
        subsystems.motion.stop_moving();
        assert!(!subsystems.motion.paused);
        assert!(subsystems.motion.stopped);
        assert!(!subsystems.motion.spline.enabled);

        subsystems.control.set_charmer(controller, true);
        subsystems.control.set_charmed(controlled);
        subsystems.control.unit_moved_by_me = Some(controlled);
        subsystems.control.player_moving_me = Some(controller);
        assert!(subsystems.control.is_charmed());
        assert!(subsystems.control.controlled_by_player);
        assert!(subsystems.control.controlled_guids.contains(&controlled));
        assert!(subsystems.control.add_shared_vision(controlled));
        subsystems.control.remove_charmed();
        subsystems.control.remove_charmer();
        assert!(!subsystems.control.is_charmed());
        assert_eq!(subsystems.control.last_charmer_guid, Some(controller));

        subsystems.vehicle.enter_vehicle(vehicle, Some(1));
        subsystems.vehicle.base_vehicle_guid = Some(vehicle);
        subsystems.vehicle.set_vehicle_kit(42, true);
        assert_eq!(subsystems.vehicle.vehicle_guid, Some(vehicle));
        assert_eq!(subsystems.vehicle.seat_id, Some(1));
        assert_eq!(
            subsystems.vehicle.kit.as_ref().map(|kit| kit.kit_id),
            Some(42)
        );
        assert_eq!(
            subsystems.vehicle.kit.as_ref().map(|kit| kit.installed),
            Some(false)
        );
        let install = subsystems.vehicle.install_vehicle_kit_like_cpp();
        assert_eq!(install.kit_id, Some(42));
        assert!(install.had_kit);
        assert_eq!(install.previous_installed, Some(false));
        assert!(install.installed);
        assert!(install.script_on_install_represented);
        let reinstall = subsystems.vehicle.install_vehicle_kit_like_cpp();
        assert_eq!(reinstall.previous_installed, Some(true));
        assert!(reinstall.installed);
        subsystems.vehicle.exit_vehicle();
        subsystems.vehicle.clear_vehicle_kit();
        assert_eq!(subsystems.vehicle.vehicle_guid, None);
        assert_eq!(subsystems.vehicle.kit, None);
        let missing_install = subsystems.vehicle.install_vehicle_kit_like_cpp();
        assert_eq!(missing_install.kit_id, None);
        assert!(!missing_install.had_kit);
        assert_eq!(missing_install.previous_installed, None);
        assert!(!missing_install.installed);
        assert!(!missing_install.script_on_install_represented);

        subsystems.vehicle.set_vehicle_kit(43, true);
        let install_before_remove = subsystems.vehicle.install_vehicle_kit_like_cpp();
        assert!(install_before_remove.installed);
        subsystems.vehicle.vehicle_guid = Some(vehicle);
        subsystems.vehicle.base_vehicle_guid = Some(vehicle);
        subsystems.vehicle.seat_id = Some(2);
        let remove = subsystems.vehicle.remove_vehicle_kit_like_cpp(true);
        assert_eq!(remove.kit_id, Some(43));
        assert!(remove.had_kit);
        assert_eq!(remove.previous_installed, Some(true));
        assert!(remove.on_remove_from_world);
        assert!(!remove.send_set_vehicle_rec_id_zero_represented);
        assert!(remove.uninstall_represented);
        assert!(remove.remove_all_passengers_represented);
        assert!(remove.script_on_uninstall_represented);
        assert!(remove.kit_cleared);
        assert_eq!(subsystems.vehicle.kit, None);
        assert_eq!(subsystems.vehicle.vehicle_guid, Some(vehicle));
        assert_eq!(subsystems.vehicle.base_vehicle_guid, Some(vehicle));
        assert_eq!(subsystems.vehicle.seat_id, Some(2));
        let missing_remove = subsystems.vehicle.remove_vehicle_kit_like_cpp(true);
        assert_eq!(missing_remove.kit_id, None);
        assert!(!missing_remove.had_kit);
        assert_eq!(missing_remove.previous_installed, None);
        assert!(missing_remove.on_remove_from_world);
        assert!(!missing_remove.send_set_vehicle_rec_id_zero_represented);
        assert!(!missing_remove.uninstall_represented);
        assert!(!missing_remove.remove_all_passengers_represented);
        assert!(!missing_remove.script_on_uninstall_represented);
        assert!(!missing_remove.kit_cleared);

        subsystems.ai.set_active(Some("NullAI"));
        subsystems.ai.push("CombatAI");
        assert_eq!(subsystems.ai.active_ai.as_deref(), Some("CombatAI"));
        assert_eq!(subsystems.ai.ai_stack, vec![String::from("NullAI")]);
        assert_eq!(subsystems.ai.pop().as_deref(), Some("CombatAI"));
        assert_eq!(subsystems.ai.active_ai.as_deref(), Some("NullAI"));
        subsystems.ai.set_locked(true);
        assert!(subsystems.ai.locked);
    }
}

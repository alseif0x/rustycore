// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit spell casting and history.

use super::*;

mod charges;

/// Trinity-compatible current spell slots represented in RustyCore state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CurrentSpellSlot {
    Melee = 0,
    Generic = 1,
    Channeled = 2,
    Autorepeat = 3,
}

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
    pub cooldowns_loaded: bool,
    pub cooldowns_before_duel: HashMap<u32, SpellCooldown>,
    pub category_cooldowns: HashMap<u32, u32>,
    pub school_lockouts: [u64; MAX_SPELL_SCHOOL],
    pub charges: HashMap<u32, VecDeque<SpellChargeState>>,
    pub charges_loaded: bool,
    pub global_cooldowns: HashMap<u32, u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellHistoryPetSaveOperationLikeCpp {
    DeleteCooldowns {
        pet_number: u32,
    },
    InsertCooldown {
        pet_number: u32,
        spell_id: u32,
        cooldown_end_time_secs: i64,
        category_id: u32,
        category_end_time_secs: i64,
    },
    DeleteCharges {
        pet_number: u32,
    },
    InsertCharge {
        pet_number: u32,
        category_id: u32,
        recharge_start_time_secs: i64,
        recharge_end_time_secs: i64,
    },
}

impl SpellHistory {
    fn unix_secs_from_ms_like_cpp(ms: u64) -> i64 {
        (ms / 1_000).min(i64::MAX as u64) as i64
    }

    pub fn save_pet_spell_history_plan_like_cpp(
        &self,
        pet_number: u32,
    ) -> Vec<SpellHistoryPetSaveOperationLikeCpp> {
        let mut operations =
            vec![SpellHistoryPetSaveOperationLikeCpp::DeleteCooldowns { pet_number }];

        for (&spell_id, cooldown) in &self.cooldowns {
            if cooldown.on_hold {
                continue;
            }

            operations.push(SpellHistoryPetSaveOperationLikeCpp::InsertCooldown {
                pet_number,
                spell_id,
                cooldown_end_time_secs: Self::unix_secs_from_ms_like_cpp(cooldown.cooldown_end_ms),
                category_id: cooldown.category_id,
                category_end_time_secs: Self::unix_secs_from_ms_like_cpp(cooldown.category_end_ms),
            });
        }

        operations.push(SpellHistoryPetSaveOperationLikeCpp::DeleteCharges { pet_number });

        for (&category_id, charges) in &self.charges {
            for charge in charges {
                operations.push(SpellHistoryPetSaveOperationLikeCpp::InsertCharge {
                    pet_number,
                    category_id,
                    recharge_start_time_secs: Self::unix_secs_from_ms_like_cpp(
                        charge.recharge_start_ms,
                    ),
                    recharge_end_time_secs: Self::unix_secs_from_ms_like_cpp(
                        charge.recharge_end_ms,
                    ),
                });
            }
        }

        operations
    }

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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellSubsystem {
    pub current_spells: HashMap<CurrentSpellSlot, CurrentSpellRef>,
    pub history: SpellHistory,
    pub execution: crate::CastExecutionStateLikeCpp,
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

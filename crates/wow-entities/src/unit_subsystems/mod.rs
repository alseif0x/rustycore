// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit subsystems.
//!
//! Issue #226 split the former 7,071-line `unit_subsystems.rs` into private
//! per-subsystem modules. Every subsystem type, writer and clock is unchanged;
//! `Unit` itself keeps its own module.

mod aura;
mod combat;
mod control;
mod movement;
mod spell;
mod threat;

pub use aura::*;
pub use combat::*;
pub use control::*;
pub use movement::*;
pub use spell::*;
pub use threat::*;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

use wow_constants::{SpellState, TypeId, UnitState};
use wow_core::{ObjectGuid, Position};

use crate::{
    CreatureAddToWorldVehicleResetContextLikeCpp, Vehicle, VehicleResetPlan, VehicleSeatAddon,
    VehicleSeatInfo,
};

/// C++ `AuraRemoveMode::AURA_REMOVE_BY_INTERRUPT`.
pub const AURA_REMOVE_BY_INTERRUPT_LIKE_CPP: u8 = 2;

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

pub const CURRENT_FIRST_NON_MELEE_SPELL: u8 = 1;

pub const CURRENT_MAX_SPELL: usize = 4;

pub const MAX_SPELL_SCHOOL: usize = 7;

pub const INFINITY_COOLDOWN_DELAY_MS: u64 = 30 * 24 * 60 * 60 * 1_000;

fn apply_ms_delta(value: u64, delta: i64) -> u64 {
    if delta.is_negative() {
        value.saturating_sub(delta.unsigned_abs())
    } else {
        value.saturating_add(delta as u64)
    }
}

pub const THREAT_UPDATE_INTERVAL_MS: u32 = 1_000;

pub const PVP_COMBAT_TIMEOUT_MS: u32 = 5_000;

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
            extra_attacks_targets: HashMap::new(),
            combat_disallowed: false,
            pending_suppressed_threat_like_cpp: HashMap::new(),
            reevaluate_all_suppressed_like_cpp: false,
        }
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
#[repr(u8)]
pub enum RotateDirection {
    Left = 0,
    Right = 1,
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
pub enum MoveFallPlan {
    Noop,
    PlayerFallInfo,
    SplineStarted,
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

pub const SUMMON_SLOT_PET: usize = 0;

pub const SUMMON_SLOT_TOTEM: usize = 1;

pub const SUMMON_SLOT_TOTEM_2: usize = 2;

pub const SUMMON_SLOT_TOTEM_3: usize = 3;

pub const SUMMON_SLOT_TOTEM_4: usize = 4;

pub const SUMMON_SLOT_MINIPET: usize = 5;

pub const SUMMON_SLOT_QUEST: usize = 6;

pub const MAX_SUMMON_SLOT: usize = 7;

pub const MAX_GAMEOBJECT_SLOT: usize = 4;

pub const MAX_TOTEM_SLOT: usize = 5;

pub const ACTION_BAR_INDEX_START: usize = 0;

pub const ACTION_BAR_INDEX_PET_SPELL_START: usize = 3;

pub const ACTION_BAR_INDEX_PET_SPELL_END: usize = 7;

pub const ACTION_BAR_INDEX_END: usize = 10;

pub const MAX_UNIT_ACTION_BAR_INDEX: usize = 10;

pub const MAX_SPELL_CHARM: usize = 4;

pub const ACT_PASSIVE_LIKE_CPP: u8 = 0x01;

pub const ACT_DISABLED_LIKE_CPP: u8 = 0x81;

pub const ACT_ENABLED_LIKE_CPP: u8 = 0xC1;

pub const ACT_COMMAND_LIKE_CPP: u8 = 0x07;

pub const ACT_REACTION_LIKE_CPP: u8 = 0x06;

pub const COMMAND_STAY_LIKE_CPP: u32 = 0;

pub const COMMAND_FOLLOW_LIKE_CPP: u32 = 1;

pub const COMMAND_ATTACK_LIKE_CPP: u32 = 2;

pub const fn make_unit_action_button_like_cpp(action: u32, active_type: u8) -> u32 {
    (action & 0x007F_FFFF) | ((active_type as u32) << 23)
}

pub const fn unit_action_button_action_like_cpp(packed: u32) -> u32 {
    packed & 0x007F_FFFF
}

pub const fn unit_action_button_type_like_cpp(packed: u32) -> u8 {
    ((packed & 0xFF80_0000) >> 23) as u8
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

impl Default for ControlSubsystem {
    fn default() -> Self {
        Self {
            owner_guid: None,
            minion_guid: None,
            summon_slots: [ObjectGuid::EMPTY; MAX_SUMMON_SLOT],
            gameobject_slots: [ObjectGuid::EMPTY; MAX_GAMEOBJECT_SLOT],
            owned_gameobjects: Vec::new(),
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
    pub just_summoned_gameobject_count: u32,
    pub summoned_gameobject_despawn_count: u32,
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

    pub fn just_summoned_gameobject_like_cpp(&mut self) -> bool {
        if !self.is_enabled() {
            return false;
        }
        self.just_summoned_gameobject_count = self.just_summoned_gameobject_count.saturating_add(1);
        true
    }

    pub fn summoned_gameobject_despawn_like_cpp(&mut self) -> bool {
        if !self.is_enabled() {
            return false;
        }
        self.summoned_gameobject_despawn_count =
            self.summoned_gameobject_despawn_count.saturating_add(1);
        true
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
#[path = "tests.rs"]
mod tests;

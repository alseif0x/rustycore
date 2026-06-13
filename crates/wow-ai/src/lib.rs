// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature AI — state machine for NPC/mob behavior.
//!
//! Implements idle wandering, random movement, aggro detection, and
//! basic melee combat for server-controlled creatures.

use std::collections::VecDeque;
use std::time::Instant;

use wow_core::{ObjectGuid, Position, random_resize_vec_like_cpp};
use wow_instances::BossAiRef;

// ── CreatureAISelector ────────────────────────────────────────────

/// Represented result of TrinityCore `FactorySelector::SelectAI(Creature*)`.
///
/// This is selector evidence only: no virtual AI object is instantiated and no
/// hooks are executed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatureAiKindLikeCpp {
    PetAI,
    ScriptedAI(String),
    NullCreatureAI,
    TriggerAI,
    AggressorAI,
    ReactorAI,
    PassiveAI,
    PossessedAI,
    CritterAI,
    GuardAI,
    TotemAI,
    CombatAI,
    TurretAI,
    VehicleAI,
    SmartAI,
    ScheduledChangeAI,
    UnknownNamedAI(String),
}

/// Minimal, already-resolved C++ creature facts needed by the stock selector.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureAiSelectionInputLikeCpp {
    pub ai_name: String,
    pub script_name: String,
    pub script_can_create_creature_ai: bool,
    pub is_pet: bool,
    pub is_vehicle: bool,
    pub is_totem: bool,
    pub is_trigger: bool,
    pub first_spell_id: u32,
    pub is_critter: bool,
    pub is_guardian: bool,
    pub is_guard: bool,
    pub is_civilian: bool,
    pub is_neutral_to_all: bool,
    pub has_spellclick_npc_flag: bool,
    pub is_controllable_guardian: bool,
    pub controllable_guardian_owner_is_player: bool,
}

impl CreatureAiKindLikeCpp {
    fn from_registered_ai_name_like_cpp(ai_name: &str) -> Self {
        match ai_name {
            "NullCreatureAI" => Self::NullCreatureAI,
            "TriggerAI" => Self::TriggerAI,
            "AggressorAI" => Self::AggressorAI,
            "ReactorAI" => Self::ReactorAI,
            "PassiveAI" => Self::PassiveAI,
            "PossessedAI" => Self::PossessedAI,
            "CritterAI" => Self::CritterAI,
            "GuardAI" => Self::GuardAI,
            "PetAI" => Self::PetAI,
            "TotemAI" => Self::TotemAI,
            "CombatAI" => Self::CombatAI,
            "TurretAI" => Self::TurretAI,
            "VehicleAI" => Self::VehicleAI,
            "SmartAI" => Self::SmartAI,
            "ScheduledChangeAI" => Self::ScheduledChangeAI,
            other => Self::UnknownNamedAI(other.to_string()),
        }
    }
}

pub fn select_creature_ai_like_cpp(
    input: &CreatureAiSelectionInputLikeCpp,
) -> CreatureAiKindLikeCpp {
    // C++ `FactorySelector::SelectAI`: pet override happens before DB ScriptName
    // and AIName so tamed creatures cannot keep a template SmartAI.
    if input.is_pet {
        return CreatureAiKindLikeCpp::PetAI;
    }

    if input.script_can_create_creature_ai && !input.script_name.is_empty() {
        return CreatureAiKindLikeCpp::ScriptedAI(input.script_name.clone());
    }

    if !input.ai_name.is_empty() {
        return CreatureAiKindLikeCpp::from_registered_ai_name_like_cpp(&input.ai_name);
    }

    select_creature_ai_by_permit_like_cpp(input)
}

fn select_creature_ai_by_permit_like_cpp(
    input: &CreatureAiSelectionInputLikeCpp,
) -> CreatureAiKindLikeCpp {
    // C++ iterates ObjectRegistry's std::map and picks max permit; equal permits
    // keep the first lexicographic AIName because std::max_element is stable for
    // equivalent values.
    [
        ("AggressorAI", permit_aggressor_ai_like_cpp(input)),
        ("CombatAI", -1),
        ("CritterAI", permit_critter_ai_like_cpp(input)),
        ("GuardAI", permit_guard_ai_like_cpp(input)),
        ("NullCreatureAI", permit_null_creature_ai_like_cpp(input)),
        ("PassiveAI", -1),
        ("PetAI", permit_pet_ai_like_cpp(input)),
        ("PossessedAI", -1),
        ("ReactorAI", permit_reactor_ai_like_cpp(input)),
        ("ScheduledChangeAI", -1),
        ("SmartAI", -1),
        ("TotemAI", permit_totem_ai_like_cpp(input)),
        ("TriggerAI", permit_trigger_ai_like_cpp(input)),
        ("TurretAI", -1),
        ("VehicleAI", permit_vehicle_ai_like_cpp(input)),
    ]
    .into_iter()
    .filter(|(_, permit)| *permit >= 0)
    .fold(None::<(&str, i32)>, |selected, candidate| match selected {
        Some(current) if current.1 >= candidate.1 => Some(current),
        _ => Some(candidate),
    })
    .map(|(ai_name, _)| CreatureAiKindLikeCpp::from_registered_ai_name_like_cpp(ai_name))
    .unwrap_or(CreatureAiKindLikeCpp::NullCreatureAI)
}

fn permit_aggressor_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if !input.is_civilian && !input.is_neutral_to_all {
        100
    } else {
        -1
    }
}

fn permit_critter_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_critter && !input.is_guardian {
        200
    } else {
        -1
    }
}

fn permit_guard_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_guard { 200 } else { -1 }
}

fn permit_null_creature_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.has_spellclick_npc_flag {
        250
    } else if input.is_trigger {
        200
    } else {
        1
    }
}

fn permit_pet_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_controllable_guardian {
        if input.controllable_guardian_owner_is_player {
            200
        } else {
            100
        }
    } else {
        -1
    }
}

fn permit_reactor_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_civilian || input.is_neutral_to_all {
        100
    } else {
        -1
    }
}

fn permit_totem_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_totem { 200 } else { -1 }
}

fn permit_trigger_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_trigger && input.first_spell_id != 0 {
        800
    } else {
        -1
    }
}

fn permit_vehicle_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_vehicle { 800 } else { -1 }
}

// ── CreatureAI::CanAIAttack ───────────────────────────────────────

/// Already-resolved facts for represented `AI()->CanAIAttack(target)`.
///
/// Most stock C++ creature AIs inherit `UnitAI::CanAIAttack == true`; this
/// input carries only the extra facts needed by represented overrides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureAiCanAttackInputLikeCpp {
    pub target_within_turret_combat_range: bool,
    pub target_within_turret_min_range: bool,
    pub boss_boundary_contains_target: Option<bool>,
}

impl Default for CreatureAiCanAttackInputLikeCpp {
    fn default() -> Self {
        Self {
            target_within_turret_combat_range: true,
            target_within_turret_min_range: false,
            boss_boundary_contains_target: None,
        }
    }
}

pub fn creature_ai_can_attack_like_cpp(
    ai_kind: &CreatureAiKindLikeCpp,
    input: &CreatureAiCanAttackInputLikeCpp,
) -> bool {
    // C++ `BossAI::CanAIAttack` is a script-provided AI override, not a stock
    // registry name. The caller must prove the selected script is BossAI before
    // passing this boundary fact.
    if let Some(in_boundary) = input.boss_boundary_contains_target {
        return in_boundary;
    }

    match ai_kind {
        CreatureAiKindLikeCpp::TurretAI => {
            input.target_within_turret_combat_range && !input.target_within_turret_min_range
        }
        _ => true,
    }
}

pub fn creature_ai_uses_base_move_in_line_of_sight_like_cpp(
    ai_kind: &CreatureAiKindLikeCpp,
) -> bool {
    match ai_kind {
        CreatureAiKindLikeCpp::NullCreatureAI
        | CreatureAiKindLikeCpp::TriggerAI
        | CreatureAiKindLikeCpp::ReactorAI
        | CreatureAiKindLikeCpp::PassiveAI
        | CreatureAiKindLikeCpp::PossessedAI
        | CreatureAiKindLikeCpp::CritterAI
        | CreatureAiKindLikeCpp::PetAI
        | CreatureAiKindLikeCpp::TotemAI
        | CreatureAiKindLikeCpp::VehicleAI
        | CreatureAiKindLikeCpp::ScheduledChangeAI => false,
        CreatureAiKindLikeCpp::ScriptedAI(_)
        | CreatureAiKindLikeCpp::UnknownNamedAI(_)
        | CreatureAiKindLikeCpp::AggressorAI
        | CreatureAiKindLikeCpp::GuardAI
        | CreatureAiKindLikeCpp::CombatAI
        | CreatureAiKindLikeCpp::TurretAI
        | CreatureAiKindLikeCpp::SmartAI => true,
    }
}

/// Already-resolved facts for C++ `Creature::GetAttackDistance(Unit const*)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureAttackDistanceInputLikeCpp {
    pub aggro_rate: f32,
    pub creature_combat_reach: f32,
    pub expansion_max_level: u8,
    pub max_player_level_config: u32,
    pub player_level_for_target: u8,
    pub creature_level_for_target: u8,
    pub creature_detect_range_aura_mod: f32,
    pub player_detected_range_aura_mod: f32,
}

impl Default for CreatureAttackDistanceInputLikeCpp {
    fn default() -> Self {
        Self {
            aggro_rate: 1.0,
            creature_combat_reach: 0.0,
            expansion_max_level: 80,
            max_player_level_config: 80,
            player_level_for_target: 80,
            creature_level_for_target: 80,
            creature_detect_range_aura_mod: 0.0,
            player_detected_range_aura_mod: 0.0,
        }
    }
}

pub fn creature_attack_distance_like_cpp(input: CreatureAttackDistanceInputLikeCpp) -> f32 {
    let aggro_rate = input.aggro_rate;
    if aggro_rate == 0.0 {
        return 0.0;
    }

    let max_radius = 45.0 * aggro_rate;
    let min_radius = 5.0 * aggro_rate;

    let player_level = i32::from(input.player_level_for_target);
    let creature_level = i32::from(input.creature_level_for_target);
    let expansion_max_level = i32::from(input.expansion_max_level);
    let base_aggro_distance = 20.0 - input.creature_combat_reach;
    let mut aggro_radius = base_aggro_distance + (creature_level - player_level) as f32;

    if u32::from(input.creature_level_for_target) + 5 <= input.max_player_level_config {
        aggro_radius += input.creature_detect_range_aura_mod;
        aggro_radius += input.player_detected_range_aura_mod;
    }

    if creature_level > expansion_max_level {
        aggro_radius = base_aggro_distance + (expansion_max_level - player_level) as f32;
    }

    if aggro_radius > max_radius {
        aggro_radius = max_radius;
    } else if aggro_radius < min_radius {
        aggro_radius = min_radius;
    }

    aggro_radius * aggro_rate
}

/// C++ `GetMaxLevelForExpansion` from `SharedDefines.h`.
pub const CURRENT_EXPANSION_LIKE_CPP: u8 = 2;

pub const fn max_level_for_expansion_like_cpp(expansion: u8) -> u8 {
    match expansion {
        0 => 60,
        1 => 70,
        2..=9 => 80,
        _ => 0,
    }
}

// ── UnitAI::SelectTarget ──────────────────────────────────────────

/// C++ `SelectTargetMethod` from `CoreAI/UnitAICommon.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectTargetMethodLikeCpp {
    Random,
    MaxThreat,
    MinThreat,
    MaxDistance,
    MinDistance,
}

/// Already-resolved target facts consumed by represented `UnitAI::SelectTarget`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitAiTargetCandidateLikeCpp {
    pub guid: ObjectGuid,
    pub is_offline: bool,
    pub is_current_victim: bool,
    pub is_last_victim: bool,
    pub threat_order: u32,
    pub distance_to_me: f32,
    pub is_player: bool,
    pub has_aura: bool,
}

/// C++ `DefaultTargetSelector` arguments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefaultTargetSelectorLikeCpp {
    pub dist: f32,
    pub player_only: bool,
    pub with_tank: bool,
    pub aura: i32,
}

impl Default for DefaultTargetSelectorLikeCpp {
    fn default() -> Self {
        Self {
            dist: 0.0,
            player_only: false,
            with_tank: true,
            aura: 0,
        }
    }
}

pub fn default_target_selector_accepts_like_cpp(
    target: &UnitAiTargetCandidateLikeCpp,
    selector: DefaultTargetSelectorLikeCpp,
) -> bool {
    if !selector.with_tank && target.is_last_victim {
        return false;
    }

    if selector.player_only && !target.is_player {
        return false;
    }

    if selector.dist > 0.0 && target.distance_to_me > selector.dist {
        return false;
    }

    if selector.dist < 0.0 && target.distance_to_me <= -selector.dist {
        return false;
    }

    if selector.aura > 0 && !target.has_aura {
        return false;
    }

    if selector.aura < 0 && target.has_aura {
        return false;
    }

    true
}

pub fn select_target_list_like_cpp(
    candidates: &[UnitAiTargetCandidateLikeCpp],
    num: usize,
    method: SelectTargetMethodLikeCpp,
    offset: usize,
    selector: DefaultTargetSelectorLikeCpp,
) -> Vec<ObjectGuid> {
    if candidates.len() <= offset {
        return Vec::new();
    }

    let mut target_list = prepare_target_list_selection_like_cpp(candidates, method, offset);
    target_list.retain(|target| default_target_selector_accepts_like_cpp(target, selector));
    finalize_target_list_selection_like_cpp(&mut target_list, num, method);
    target_list.into_iter().map(|target| target.guid).collect()
}

pub fn select_target_like_cpp(
    candidates: &[UnitAiTargetCandidateLikeCpp],
    method: SelectTargetMethodLikeCpp,
    offset: usize,
    selector: DefaultTargetSelectorLikeCpp,
) -> Option<ObjectGuid> {
    let num = if method == SelectTargetMethodLikeCpp::Random {
        1
    } else {
        usize::MAX
    };
    select_target_list_like_cpp(candidates, num, method, offset, selector)
        .into_iter()
        .next()
}

fn prepare_target_list_selection_like_cpp(
    candidates: &[UnitAiTargetCandidateLikeCpp],
    method: SelectTargetMethodLikeCpp,
    offset: usize,
) -> Vec<UnitAiTargetCandidateLikeCpp> {
    let mut target_list = if matches!(
        method,
        SelectTargetMethodLikeCpp::MaxDistance | SelectTargetMethodLikeCpp::MinDistance
    ) {
        candidates
            .iter()
            .copied()
            .filter(|target| !target.is_offline)
            .collect::<Vec<_>>()
    } else {
        let mut list = Vec::new();
        if let Some(current) = candidates
            .iter()
            .copied()
            .find(|target| target.is_current_victim)
        {
            list.push(current);
        }

        let mut sorted_threat = candidates
            .iter()
            .copied()
            .filter(|target| !target.is_offline && !target.is_current_victim)
            .collect::<Vec<_>>();
        sorted_threat.sort_by_key(|target| target.threat_order);
        list.extend(sorted_threat);
        list
    };

    if target_list.len() <= offset {
        return Vec::new();
    }

    match method {
        SelectTargetMethodLikeCpp::MaxDistance => target_list.sort_by(|left, right| {
            right
                .distance_to_me
                .total_cmp(&left.distance_to_me)
                .then_with(|| left.threat_order.cmp(&right.threat_order))
        }),
        SelectTargetMethodLikeCpp::MinDistance => target_list.sort_by(|left, right| {
            left.distance_to_me
                .total_cmp(&right.distance_to_me)
                .then_with(|| left.threat_order.cmp(&right.threat_order))
        }),
        SelectTargetMethodLikeCpp::MinThreat => target_list.reverse(),
        SelectTargetMethodLikeCpp::Random | SelectTargetMethodLikeCpp::MaxThreat => {}
    }

    target_list.into_iter().skip(offset).collect()
}

fn finalize_target_list_selection_like_cpp(
    target_list: &mut Vec<UnitAiTargetCandidateLikeCpp>,
    num: usize,
    method: SelectTargetMethodLikeCpp,
) {
    if target_list.len() <= num {
        return;
    }

    if method == SelectTargetMethodLikeCpp::Random {
        random_resize_vec_like_cpp(target_list, num);
    } else {
        target_list.truncate(num);
    }
}

// ── CreatureAI::EnterEvadeMode ────────────────────────────────────

/// C++ `EvadeReason` from `CoreAI/UnitAICommon.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvadeReasonLikeCpp {
    NoHostiles,
    Boundary,
    NoPath,
    SequenceBreak,
    Other,
}

impl EvadeReasonLikeCpp {
    pub const COUNT_LIKE_CPP: usize = 5;

    pub fn to_index_like_cpp(self) -> usize {
        match self {
            Self::NoHostiles => 0,
            Self::Boundary => 1,
            Self::NoPath => 2,
            Self::SequenceBreak => 3,
            Self::Other => 4,
        }
    }

    pub fn from_index_like_cpp(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::NoHostiles),
            1 => Some(Self::Boundary),
            2 => Some(Self::NoPath),
            3 => Some(Self::SequenceBreak),
            4 => Some(Self::Other),
            _ => None,
        }
    }

    pub fn constant_like_cpp(self) -> &'static str {
        match self {
            Self::NoHostiles => "NoHostiles",
            Self::Boundary => "Boundary",
            Self::NoPath => "NoPath",
            Self::SequenceBreak => "SequenceBreak",
            Self::Other => "Other",
        }
    }

    pub fn description_like_cpp(self) -> &'static str {
        match self {
            Self::NoHostiles => "the creature's threat list is empty",
            Self::Boundary => "the creature has moved outside its evade boundary",
            Self::NoPath => "the creature was unable to reach its target for over 5 seconds",
            Self::SequenceBreak => {
                "this is a boss and the pre-requisite encounters for engaging it are not defeated yet"
            }
            Self::Other => "anything else",
        }
    }
}

/// Already-resolved C++ creature facts needed by represented `EnterEvadeMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureEnterEvadeInputLikeCpp {
    pub reason: EvadeReasonLikeCpp,
    pub is_in_evade_mode: bool,
    pub is_alive: bool,
    pub has_vehicle: bool,
    pub owner_guid: Option<ObjectGuid>,
    pub tap_list_not_cleared_on_evade: bool,
}

impl Default for CreatureEnterEvadeInputLikeCpp {
    fn default() -> Self {
        Self {
            reason: EvadeReasonLikeCpp::Other,
            is_in_evade_mode: false,
            is_alive: true,
            has_vehicle: false,
            owner_guid: None,
            tap_list_not_cleared_on_evade: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CreatureEvadeMovementLikeCpp {
    NoneVehicle,
    FollowOwner {
        owner_guid: ObjectGuid,
        pet_follow_distance: f32,
    },
    TargetedHomeAddEvadeState,
}

/// Pure side-effect plan for C++ `CreatureAI::EnterEvadeMode`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureEnterEvadePlanLikeCpp {
    pub reason: EvadeReasonLikeCpp,
    pub remove_auras_on_evade: bool,
    pub combat_stop_with_pets: bool,
    pub clear_tap: bool,
    pub reset_player_damage_req: bool,
    pub clear_last_damaged_time: bool,
    pub clear_cannot_reach_target: bool,
    pub clear_spell_focus_target: bool,
    pub clear_target: bool,
    pub reset_spell_cooldowns: bool,
    pub engagement_over: bool,
    pub movement: CreatureEvadeMovementLikeCpp,
    pub reset_ai: bool,
}

pub fn creature_enter_evade_mode_plan_like_cpp(
    input: CreatureEnterEvadeInputLikeCpp,
) -> Option<CreatureEnterEvadePlanLikeCpp> {
    if input.is_in_evade_mode {
        return None;
    }

    if !input.is_alive {
        return Some(CreatureEnterEvadePlanLikeCpp {
            reason: input.reason,
            remove_auras_on_evade: false,
            combat_stop_with_pets: false,
            clear_tap: false,
            reset_player_damage_req: false,
            clear_last_damaged_time: false,
            clear_cannot_reach_target: false,
            clear_spell_focus_target: false,
            clear_target: false,
            reset_spell_cooldowns: false,
            engagement_over: true,
            movement: CreatureEvadeMovementLikeCpp::NoneVehicle,
            reset_ai: false,
        });
    }

    let movement = if input.has_vehicle {
        CreatureEvadeMovementLikeCpp::NoneVehicle
    } else if let Some(owner_guid) = input.owner_guid {
        CreatureEvadeMovementLikeCpp::FollowOwner {
            owner_guid,
            pet_follow_distance: 1.0,
        }
    } else {
        CreatureEvadeMovementLikeCpp::TargetedHomeAddEvadeState
    };

    Some(CreatureEnterEvadePlanLikeCpp {
        reason: input.reason,
        remove_auras_on_evade: true,
        combat_stop_with_pets: true,
        clear_tap: !input.tap_list_not_cleared_on_evade,
        reset_player_damage_req: true,
        clear_last_damaged_time: true,
        clear_cannot_reach_target: true,
        clear_spell_focus_target: true,
        clear_target: true,
        reset_spell_cooldowns: true,
        engagement_over: true,
        movement,
        reset_ai: true,
    })
}

// ── CreatureAI::TriggerAlert ──────────────────────────────────────

/// Already-resolved C++ facts needed by `CreatureAI::TriggerAlert(Unit const*)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureTriggerAlertInputLikeCpp {
    pub target_exists: bool,
    pub target_is_player: bool,
    pub creature_is_unit: bool,
    pub creature_is_engaged: bool,
    pub creature_is_confused: bool,
    pub creature_is_stunned: bool,
    pub creature_is_fleeing: bool,
    pub creature_is_distracted: bool,
    pub creature_is_civilian: bool,
    pub creature_has_react_passive: bool,
    pub creature_is_hostile_to_target: bool,
    pub target_acceptable: bool,
    pub absolute_angle_to_target: f32,
}

impl Default for CreatureTriggerAlertInputLikeCpp {
    fn default() -> Self {
        Self {
            target_exists: true,
            target_is_player: true,
            creature_is_unit: true,
            creature_is_engaged: false,
            creature_is_confused: false,
            creature_is_stunned: false,
            creature_is_fleeing: false,
            creature_is_distracted: false,
            creature_is_civilian: false,
            creature_has_react_passive: false,
            creature_is_hostile_to_target: true,
            target_acceptable: true,
            absolute_angle_to_target: 0.0,
        }
    }
}

/// Pure side-effect plan for C++ `CreatureAI::TriggerAlert`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureTriggerAlertPlanLikeCpp {
    pub send_ai_reaction_alert: bool,
    pub move_distract_ms: u32,
    pub orientation: f32,
}

pub fn creature_trigger_alert_plan_like_cpp(
    input: CreatureTriggerAlertInputLikeCpp,
) -> Option<CreatureTriggerAlertPlanLikeCpp> {
    if !input.target_exists || !input.target_is_player {
        return None;
    }

    if !input.creature_is_unit
        || input.creature_is_engaged
        || input.creature_is_confused
        || input.creature_is_stunned
        || input.creature_is_fleeing
        || input.creature_is_distracted
    {
        return None;
    }

    if input.creature_is_civilian
        || input.creature_has_react_passive
        || !input.creature_is_hostile_to_target
        || !input.target_acceptable
    {
        return None;
    }

    Some(CreatureTriggerAlertPlanLikeCpp {
        send_ai_reaction_alert: true,
        move_distract_ms: 5_000,
        orientation: input.absolute_angle_to_target,
    })
}

// ── CreatureAI::DoZoneInCombat ────────────────────────────────────

/// Already-resolved player facts used by C++ `CreatureAI::DoZoneInCombat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureZoneInCombatPlayerLikeCpp {
    pub player_guid: ObjectGuid,
    pub is_alive: bool,
    pub can_begin_combat: bool,
    pub controlled_unit_guids: Vec<ObjectGuid>,
    pub vehicle_base_guid: Option<ObjectGuid>,
}

/// Already-resolved map/player facts needed by `CreatureAI::DoZoneInCombat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureZoneInCombatInputLikeCpp {
    pub creature_guid: ObjectGuid,
    pub map_is_dungeon: bool,
    pub players: Vec<CreatureZoneInCombatPlayerLikeCpp>,
}

/// Pure side-effect plan for C++ `CreatureAI::DoZoneInCombat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureZoneInCombatPlanLikeCpp {
    pub log_non_dungeon_error: bool,
    pub engage_targets: Vec<ObjectGuid>,
}

pub fn creature_do_zone_in_combat_plan_like_cpp(
    input: CreatureZoneInCombatInputLikeCpp,
) -> CreatureZoneInCombatPlanLikeCpp {
    if !input.map_is_dungeon {
        return CreatureZoneInCombatPlanLikeCpp {
            log_non_dungeon_error: true,
            engage_targets: Vec::new(),
        };
    }

    let mut engage_targets = Vec::new();
    for player in input.players {
        if !player.is_alive || !player.can_begin_combat {
            continue;
        }

        engage_targets.push(player.player_guid);
        engage_targets.extend(player.controlled_unit_guids);
        if let Some(vehicle_base_guid) = player.vehicle_base_guid {
            engage_targets.push(vehicle_base_guid);
        }
    }

    CreatureZoneInCombatPlanLikeCpp {
        log_non_dungeon_error: false,
        engage_targets,
    }
}

// ── ScriptedAI::SummonList ────────────────────────────────────────

/// Already-resolved creature facts needed by represented `SummonList` helpers.
///
/// C++ `SummonList` stores only GUIDs and asks `ObjectAccessor::GetCreature`
/// at the point where a side effect is needed. Rust callers provide that
/// resolved view so this crate can preserve list semantics without pretending
/// to own world creatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummonListCreatureViewLikeCpp {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub ai_enabled: bool,
}

/// Side effect requested by `SummonList::DoActionImpl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummonListActionLikeCpp {
    pub guid: ObjectGuid,
    pub action: i32,
}

/// C++ `SummonList` storage and pure planning helpers.
///
/// `GuidList` is list-like: order is preserved, duplicate GUIDs are allowed,
/// and `Despawn(Creature const*)` removes every matching GUID.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SummonListLikeCpp {
    storage: VecDeque<ObjectGuid>,
}

impl SummonListLikeCpp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn empty_like_cpp(&self) -> bool {
        self.storage.is_empty()
    }

    pub fn size_like_cpp(&self) -> usize {
        self.storage.len()
    }

    pub fn clear_like_cpp(&mut self) {
        self.storage.clear();
    }

    pub fn iter_like_cpp(&self) -> impl Iterator<Item = ObjectGuid> + '_ {
        self.storage.iter().copied()
    }

    pub fn summon_like_cpp(&mut self, summon_guid: ObjectGuid) {
        self.storage.push_back(summon_guid);
    }

    pub fn despawn_like_cpp(&mut self, summon_guid: ObjectGuid) {
        self.storage.retain(|guid| *guid != summon_guid);
    }

    pub fn despawn_all_like_cpp<F>(&mut self, mut resolve: F) -> Vec<ObjectGuid>
    where
        F: FnMut(ObjectGuid) -> Option<SummonListCreatureViewLikeCpp>,
    {
        let mut despawn_plan = Vec::new();

        while let Some(guid) = self.storage.pop_front() {
            if resolve(guid).is_some() {
                despawn_plan.push(guid);
            }
        }

        despawn_plan
    }

    pub fn despawn_entry_like_cpp<F>(&mut self, entry: u32, mut resolve: F) -> Vec<ObjectGuid>
    where
        F: FnMut(ObjectGuid) -> Option<SummonListCreatureViewLikeCpp>,
    {
        let mut despawn_plan = Vec::new();
        let mut kept = VecDeque::with_capacity(self.storage.len());

        while let Some(guid) = self.storage.pop_front() {
            match resolve(guid) {
                None => {}
                Some(creature) if creature.entry == entry => despawn_plan.push(guid),
                Some(_) => kept.push_back(guid),
            }
        }

        self.storage = kept;
        despawn_plan
    }

    pub fn remove_not_existing_like_cpp<F>(&mut self, mut resolve: F)
    where
        F: FnMut(ObjectGuid) -> Option<SummonListCreatureViewLikeCpp>,
    {
        self.storage.retain(|guid| resolve(*guid).is_some());
    }

    pub fn has_entry_like_cpp<F>(&self, entry: u32, mut resolve: F) -> bool
    where
        F: FnMut(ObjectGuid) -> Option<SummonListCreatureViewLikeCpp>,
    {
        self.storage
            .iter()
            .copied()
            .any(|guid| resolve(guid).is_some_and(|creature| creature.entry == entry))
    }

    pub fn do_zone_in_combat_like_cpp<F>(&self, entry: u32, mut resolve: F) -> Vec<ObjectGuid>
    where
        F: FnMut(ObjectGuid) -> Option<SummonListCreatureViewLikeCpp>,
    {
        self.storage
            .iter()
            .copied()
            .filter(|guid| {
                resolve(*guid).is_some_and(|creature| {
                    creature.ai_enabled && (entry == 0 || creature.entry == entry)
                })
            })
            .collect()
    }

    pub fn do_action_like_cpp<F, P>(
        &self,
        action: i32,
        predicate: P,
        resolve: F,
    ) -> Vec<SummonListActionLikeCpp>
    where
        F: FnMut(ObjectGuid) -> Option<SummonListCreatureViewLikeCpp>,
        P: FnMut(ObjectGuid) -> bool,
    {
        self.do_action_with_max_like_cpp(action, predicate, 0, resolve)
    }

    pub fn do_action_with_max_like_cpp<F, P>(
        &self,
        action: i32,
        mut predicate: P,
        max: u16,
        mut resolve: F,
    ) -> Vec<SummonListActionLikeCpp>
    where
        F: FnMut(ObjectGuid) -> Option<SummonListCreatureViewLikeCpp>,
        P: FnMut(ObjectGuid) -> bool,
    {
        let mut summons = self
            .storage
            .iter()
            .copied()
            .filter(|guid| predicate(*guid))
            .collect::<Vec<_>>();

        if max != 0 {
            random_resize_vec_like_cpp(&mut summons, usize::from(max));
        }

        summons
            .into_iter()
            .filter_map(|guid| {
                resolve(guid)
                    .filter(|creature| creature.ai_enabled)
                    .map(|_| SummonListActionLikeCpp { guid, action })
            })
            .collect()
    }
}

// ── CreatureState ──────────────────────────────────────────────────

/// Current AI state for a creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureState {
    /// Idle — creature stands still or wanders randomly.
    Idle,
    /// Walking to a random point within home radius.
    WalkingRandom,
    /// Walking along a waypoint path.
    WalkingWaypoint,
    /// In combat with a player.
    InCombat,
    /// Dead — waiting for respawn.
    Dead,
    /// Returning to spawn point after combat reset.
    Returning,
}

// ── CreatureAI ────────────────────────────────────────────────────

/// Server-side state for one spawned creature.
///
/// Tracks position, health, movement timers, and combat state.
/// The session update loop calls [`CreatureAI::update`] every tick.
#[derive(Debug, Clone)]
pub struct CreatureAI {
    pub guid: ObjectGuid,
    pub entry: u32,

    /// Spawn (home) position.
    pub home_pos: Position,
    /// Current position (updated as it moves).
    pub current_pos: Position,
    /// Destination for current movement (None if standing still).
    pub move_target: Option<Position>,
    /// When the current movement started.
    pub move_start: Instant,
    /// How long the current movement takes (ms).
    pub move_duration_ms: u32,
    /// Spline ID counter (incremented on each move command).
    pub spline_id: u32,

    /// Current AI state.
    pub state: CreatureState,
    /// Time until next random movement attempt.
    pub wander_timer: Instant,
    /// Wander delay before moving again (random 5–15s).
    pub wander_delay_ms: u64,

    /// Current HP.
    pub hp: u32,
    /// Max HP.
    pub max_hp: u32,

    /// Level (used for damage/aggro calculations).
    pub level: u8,
    /// Melee damage range.
    pub min_dmg: u32,
    pub max_dmg: u32,

    /// Current combat target (player GUID).
    pub combat_target: Option<ObjectGuid>,
    /// Last time this creature swung its weapon.
    pub last_swing: Instant,
    /// Swing timer in ms (base 2000ms for most mobs).
    pub swing_timer_ms: u64,

    /// Aggro radius (yards). Typical is 10–20.
    pub aggro_radius: f32,

    /// Maximum wander distance from home position (yards).
    pub wander_radius: f32,

    /// Whether this creature is alive.
    pub is_alive: bool,

    /// Time of death (for respawn logic).
    pub death_time: Option<Instant>,
    /// Respawn time in seconds.
    pub respawn_time_secs: u64,

    /// When this corpse should despawn from the world (set after fully looted).
    /// None = corpse never explicitly looted / not yet triggered.
    ///
    /// C# ref: `AllLootRemovedFromCorpse()` sets `m_corpseRemoveTime`.
    pub corpse_despawn_at: Option<Instant>,

    /// NPC flags (vendor, quest giver, etc.) — stored here for convenience.
    pub npc_flags: u32,
    /// Unit flags.
    pub unit_flags: u32,
    /// Display ID.
    pub display_id: u32,
    /// Faction template ID.
    pub faction: u32,
    /// Resolved C++ `Creature::GetLootId()` value for corpse loot.
    pub loot_id: u32,
    /// C++ `CreatureDifficulty::GoldMin`.
    pub gold_min: u32,
    /// C++ `CreatureDifficulty::GoldMax`.
    pub gold_max: u32,
    /// Represented C++ `BossAI::_bossId`, if this creature uses BossAI.
    pub boss_id: Option<u32>,
    /// Represented C++ `Loot::_dungeonEncounterId` source for corpse loot.
    pub dungeon_encounter_id: u32,
}

impl CreatureAI {
    /// Create a new creature AI with default idle state.
    pub fn new(
        guid: ObjectGuid,
        entry: u32,
        pos: Position,
        hp: u32,
        level: u8,
        min_dmg: u32,
        max_dmg: u32,
        aggro_radius: f32,
        display_id: u32,
        faction: u32,
        npc_flags: u32,
        unit_flags: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        boss_id: Option<u32>,
        dungeon_encounter_id: u32,
    ) -> Self {
        let now = Instant::now();
        // Derive rough damage if zero
        let (min_dmg, max_dmg) = if min_dmg == 0 {
            let base = (level as u32) * 3 + 5;
            (base, base + base / 2)
        } else {
            (min_dmg, max_dmg)
        };
        Self {
            guid,
            entry,
            home_pos: pos.clone(),
            current_pos: pos,
            move_target: None,
            move_start: now,
            move_duration_ms: 0,
            spline_id: 1,
            state: CreatureState::Idle,
            wander_timer: now,
            wander_delay_ms: 8_000,
            hp,
            max_hp: hp,
            level,
            min_dmg,
            max_dmg,
            combat_target: None,
            last_swing: now,
            swing_timer_ms: 2_000,
            aggro_radius,
            wander_radius: 5.0,
            is_alive: true,
            death_time: None,
            respawn_time_secs: 30,
            corpse_despawn_at: None,
            npc_flags,
            unit_flags,
            display_id,
            faction,
            loot_id,
            gold_min,
            gold_max,
            boss_id,
            dungeon_encounter_id,
        }
    }

    /// Returns true if this creature can wander randomly.
    /// Creatures with vendors, quest givers etc. typically don't wander.
    pub fn can_wander(&self) -> bool {
        // npc_flags: 1=gossip, 2=quest giver, 4=unk, 8=vendor, 16=trainer...
        // creatures with certain flags stay put
        self.npc_flags == 0 || (self.npc_flags & 0x80) == 0 // no UNIT_NPC_FLAG_INNKEEPER etc
    }

    /// Try to engage a player in combat.
    ///
    /// Returns true if the creature enters combat (was idle, player in range).
    pub fn try_aggro(&mut self, player_guid: ObjectGuid, player_pos: &Position) -> bool {
        if !self.is_alive || self.state == CreatureState::InCombat {
            return false;
        }
        let dist = self.current_pos.distance(player_pos);
        if dist <= self.aggro_radius {
            self.enter_combat(player_guid);
            return true;
        }
        false
    }

    /// Enter combat with a specific player.
    pub fn enter_combat(&mut self, player_guid: ObjectGuid) {
        self.state = CreatureState::InCombat;
        self.combat_target = Some(player_guid);
        self.move_target = None;
    }

    /// Leave combat and return to home position.
    pub fn reset_combat(&mut self) {
        self.state = CreatureState::Returning;
        self.combat_target = None;
        self.hp = self.max_hp;
        self.move_target = Some(self.home_pos.clone());
    }

    /// Apply damage to the creature.
    ///
    /// Returns true if the creature just died.
    pub fn take_damage(&mut self, dmg: u32) -> bool {
        if !self.is_alive {
            return false;
        }
        self.hp = self.hp.saturating_sub(dmg);
        if self.hp == 0 {
            self.die();
            return true;
        }
        false
    }

    /// Kill the creature.
    pub fn die(&mut self) {
        self.is_alive = false;
        self.state = CreatureState::Dead;
        self.combat_target = None;
        self.death_time = Some(Instant::now());
    }

    /// Check if the creature should respawn.
    pub fn should_respawn(&self) -> bool {
        if let Some(dt) = self.death_time {
            dt.elapsed().as_secs() >= self.respawn_time_secs
        } else {
            false
        }
    }

    /// Respawn the creature at its home position.
    pub fn respawn(&mut self) {
        self.hp = self.max_hp;
        self.is_alive = true;
        self.state = CreatureState::Idle;
        self.current_pos = self.home_pos.clone();
        self.move_target = None;
        self.death_time = None;
        self.spline_id += 1;
        self.wander_timer = Instant::now();
    }

    /// Check if the creature's current movement is complete.
    pub fn movement_finished(&self) -> bool {
        if self.move_target.is_none() {
            return true;
        }
        self.move_start.elapsed().as_millis() as u32 >= self.move_duration_ms
    }

    /// Interpolate the creature's current position along its movement path.
    pub fn interpolated_position(&self) -> Position {
        let Some(ref dst) = self.move_target else {
            return self.current_pos.clone();
        };
        let elapsed = self.move_start.elapsed().as_millis() as f32;
        let total = self.move_duration_ms as f32;
        if total <= 0.0 {
            return dst.clone();
        }
        let t = (elapsed / total).min(1.0);
        Position::new(
            self.current_pos.x + (dst.x - self.current_pos.x) * t,
            self.current_pos.y + (dst.y - self.current_pos.y) * t,
            self.current_pos.z + (dst.z - self.current_pos.z) * t,
            dst.orientation,
        )
    }

    /// Begin a move to the destination at walk speed (2.5 y/s).
    pub fn begin_move(&mut self, dst: Position) {
        let dist = self.current_pos.distance(&dst);
        let walk_speed = 2.5f32;
        let duration_ms = ((dist / walk_speed) * 1000.0) as u32;
        self.move_target = Some(dst);
        self.move_start = Instant::now();
        self.move_duration_ms = duration_ms.max(500);
        self.spline_id += 1;
    }

    /// Finalize movement — snap to destination.
    pub fn finish_move(&mut self) {
        if let Some(dst) = self.move_target.take() {
            self.current_pos = dst;
        }
        self.move_duration_ms = 0;
    }

    /// Check if it's time to swing the weapon.
    pub fn can_swing(&self) -> bool {
        self.is_alive
            && self.state == CreatureState::InCombat
            && self.last_swing.elapsed().as_millis() as u64 >= self.swing_timer_ms
    }

    /// Record that a swing happened.
    pub fn record_swing(&mut self) {
        self.last_swing = Instant::now();
    }

    /// Roll a random damage value in [min_dmg, max_dmg].
    pub fn roll_damage(&self) -> u32 {
        if self.min_dmg >= self.max_dmg {
            return self.min_dmg;
        }
        let range = self.max_dmg - self.min_dmg;
        // Simple LCG-style pseudo-random based on timer
        let seed = self.last_swing.elapsed().subsec_nanos();
        self.min_dmg + (seed % (range + 1))
    }

    /// Check if creature should check wander movement.
    pub fn should_wander(&self) -> bool {
        self.is_alive
            && self.state == CreatureState::Idle
            && self.can_wander()
            && self.wander_timer.elapsed().as_millis() as u64 >= self.wander_delay_ms
    }

    /// Pick a random wander destination near home.
    pub fn pick_wander_destination(&mut self) -> Position {
        // Simple pseudo-random using elapsed time as seed
        let seed = self.wander_timer.elapsed().subsec_nanos() as f32;
        let angle = (seed * 0.001) % (2.0 * std::f32::consts::PI);
        let dist = (seed * 0.0001) % self.wander_radius + 1.0;
        let x = self.home_pos.x + angle.cos() * dist;
        let y = self.home_pos.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI; // face movement direction
        Position::new(x, y, self.home_pos.z, o)
    }

    /// Reset the wander timer with a random delay.
    pub fn reset_wander_timer(&mut self) {
        self.wander_timer = Instant::now();
        // Random delay 5–15 seconds
        let seed = self.wander_timer.elapsed().subsec_nanos() as u64;
        self.wander_delay_ms = 5_000 + (seed % 10_000);
    }
}

impl CreatureAI {
    /// Return the represented C++ `BossAI` view only when this creature has
    /// script-provided boss identity. A plain creature must behave like a
    /// failed `dynamic_cast<BossAI const*>`.
    pub fn boss_ai_like_cpp(&self) -> Option<BossAiRef> {
        self.boss_id.map(BossAiRef::new)
    }
}

// ── Position distance helper ──────────────────────────────────────
// Position already has .distance() from wow-core; we define this
// convenience method here for internal use.

fn position_dist(a: &Position, b: &Position) -> f32 {
    a.distance(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use wow_instances::BossAiLikeCpp;

    fn selector_input() -> CreatureAiSelectionInputLikeCpp {
        CreatureAiSelectionInputLikeCpp::default()
    }

    #[test]
    fn creature_ai_selector_pet_overrides_script_and_ai_name_like_cpp() {
        let input = CreatureAiSelectionInputLikeCpp {
            is_pet: true,
            script_name: "boss_should_not_win".to_string(),
            script_can_create_creature_ai: true,
            ai_name: "SmartAI".to_string(),
            ..selector_input()
        };

        assert_eq!(
            select_creature_ai_like_cpp(&input),
            CreatureAiKindLikeCpp::PetAI
        );
    }

    #[test]
    fn creature_ai_selector_uses_script_before_ai_name_like_cpp() {
        let input = CreatureAiSelectionInputLikeCpp {
            script_name: "npc_scripted".to_string(),
            script_can_create_creature_ai: true,
            ai_name: "AggressorAI".to_string(),
            ..selector_input()
        };

        assert_eq!(
            select_creature_ai_like_cpp(&input),
            CreatureAiKindLikeCpp::ScriptedAI("npc_scripted".to_string())
        );
    }

    #[test]
    fn creature_ai_selector_uses_registered_ai_name_before_permits_like_cpp() {
        let input = CreatureAiSelectionInputLikeCpp {
            ai_name: "TurretAI".to_string(),
            is_vehicle: true,
            ..selector_input()
        };

        assert_eq!(
            select_creature_ai_like_cpp(&input),
            CreatureAiKindLikeCpp::TurretAI
        );
    }

    #[test]
    fn creature_ai_selector_falls_back_to_stock_permits_like_cpp() {
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_vehicle: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::VehicleAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_trigger: true,
                first_spell_id: 133,
                is_vehicle: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::TriggerAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_trigger: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::NullCreatureAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                has_spellclick_npc_flag: true,
                is_guard: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::NullCreatureAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_guard: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::GuardAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_controllable_guardian: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::AggressorAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_controllable_guardian: true,
                is_civilian: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::PetAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_civilian: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::ReactorAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_neutral_to_all: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::ReactorAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&selector_input()),
            CreatureAiKindLikeCpp::AggressorAI
        );
    }

    #[test]
    fn creature_ai_selector_preserves_unknown_ai_name_as_unrepresented_like_cpp() {
        let input = CreatureAiSelectionInputLikeCpp {
            ai_name: "CustomPrivateAI".to_string(),
            ..selector_input()
        };

        assert_eq!(
            select_creature_ai_like_cpp(&input),
            CreatureAiKindLikeCpp::UnknownNamedAI("CustomPrivateAI".to_string())
        );
    }

    #[test]
    fn creature_ai_can_attack_defaults_true_for_stock_ai_like_cpp() {
        for ai_kind in [
            CreatureAiKindLikeCpp::AggressorAI,
            CreatureAiKindLikeCpp::ReactorAI,
            CreatureAiKindLikeCpp::GuardAI,
            CreatureAiKindLikeCpp::SmartAI,
            CreatureAiKindLikeCpp::VehicleAI,
            CreatureAiKindLikeCpp::UnknownNamedAI("CustomPrivateAI".to_string()),
        ] {
            assert!(
                creature_ai_can_attack_like_cpp(
                    &ai_kind,
                    &CreatureAiCanAttackInputLikeCpp {
                        target_within_turret_combat_range: false,
                        target_within_turret_min_range: true,
                        boss_boundary_contains_target: None,
                    },
                ),
                "{ai_kind:?} should inherit UnitAI::CanAIAttack == true"
            );
        }
    }

    #[test]
    fn creature_ai_can_attack_applies_turret_range_override_like_cpp() {
        assert!(creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            &CreatureAiCanAttackInputLikeCpp {
                target_within_turret_combat_range: true,
                target_within_turret_min_range: false,
                boss_boundary_contains_target: None,
            },
        ));
        assert!(!creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            &CreatureAiCanAttackInputLikeCpp {
                target_within_turret_combat_range: false,
                target_within_turret_min_range: false,
                boss_boundary_contains_target: None,
            },
        ));
        assert!(!creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            &CreatureAiCanAttackInputLikeCpp {
                target_within_turret_combat_range: true,
                target_within_turret_min_range: true,
                boss_boundary_contains_target: None,
            },
        ));
    }

    #[test]
    fn creature_ai_can_attack_applies_boss_boundary_override_like_cpp() {
        assert!(!creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::ScriptedAI("boss_script".to_string()),
            &CreatureAiCanAttackInputLikeCpp {
                boss_boundary_contains_target: Some(false),
                ..CreatureAiCanAttackInputLikeCpp::default()
            },
        ));
        assert!(creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::ScriptedAI("boss_script".to_string()),
            &CreatureAiCanAttackInputLikeCpp {
                boss_boundary_contains_target: Some(true),
                target_within_turret_combat_range: false,
                target_within_turret_min_range: true,
            },
        ));
    }

    fn assert_close_like_cpp(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= f32::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn creature_attack_distance_zero_rate_returns_zero_like_cpp() {
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                aggro_rate: 0.0,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            0.0,
        );
    }

    #[test]
    fn creature_attack_distance_equal_level_subtracts_combat_reach_like_cpp() {
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                creature_combat_reach: 1.5,
                player_level_for_target: 60,
                creature_level_for_target: 60,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            18.5,
        );
    }

    #[test]
    fn creature_attack_distance_applies_level_difference_and_clamps_like_cpp() {
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                player_level_for_target: 20,
                creature_level_for_target: 25,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            25.0,
        );
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                player_level_for_target: 80,
                creature_level_for_target: 1,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            5.0,
        );
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                player_level_for_target: 1,
                creature_level_for_target: 80,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            45.0,
        );
    }

    #[test]
    fn creature_attack_distance_detect_range_auras_are_level_gated_like_cpp() {
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                player_level_for_target: 40,
                creature_level_for_target: 40,
                max_player_level_config: 80,
                creature_detect_range_aura_mod: 3.0,
                player_detected_range_aura_mod: 2.0,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            25.0,
        );
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                player_level_for_target: 80,
                creature_level_for_target: 80,
                max_player_level_config: 80,
                creature_detect_range_aura_mod: 3.0,
                player_detected_range_aura_mod: 2.0,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            20.0,
        );
    }

    #[test]
    fn creature_attack_distance_caps_creatures_above_expansion_max_like_cpp() {
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                player_level_for_target: 70,
                creature_level_for_target: 83,
                expansion_max_level: 80,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            30.0,
        );
    }

    #[test]
    fn creature_attack_distance_preserves_cpp_rate_order_like_cpp() {
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                aggro_rate: 2.0,
                player_level_for_target: 80,
                creature_level_for_target: 80,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            40.0,
        );
        assert_close_like_cpp(
            creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
                aggro_rate: 2.0,
                player_level_for_target: 80,
                creature_level_for_target: 1,
                ..CreatureAttackDistanceInputLikeCpp::default()
            }),
            20.0,
        );
    }

    #[test]
    fn max_level_for_expansion_matches_cpp_shared_defines_like_cpp() {
        assert_eq!(max_level_for_expansion_like_cpp(0), 60);
        assert_eq!(max_level_for_expansion_like_cpp(1), 70);
        assert_eq!(max_level_for_expansion_like_cpp(2), 80);
        assert_eq!(max_level_for_expansion_like_cpp(3), 80);
        assert_eq!(max_level_for_expansion_like_cpp(9), 80);
        assert_eq!(max_level_for_expansion_like_cpp(10), 0);
    }

    #[test]
    fn creature_ai_move_in_line_of_sight_empty_overrides_are_suppressed_like_cpp() {
        for ai_kind in [
            CreatureAiKindLikeCpp::NullCreatureAI,
            CreatureAiKindLikeCpp::TriggerAI,
            CreatureAiKindLikeCpp::ReactorAI,
            CreatureAiKindLikeCpp::PassiveAI,
            CreatureAiKindLikeCpp::PossessedAI,
            CreatureAiKindLikeCpp::CritterAI,
            CreatureAiKindLikeCpp::PetAI,
            CreatureAiKindLikeCpp::TotemAI,
            CreatureAiKindLikeCpp::VehicleAI,
            CreatureAiKindLikeCpp::ScheduledChangeAI,
        ] {
            assert!(
                !creature_ai_uses_base_move_in_line_of_sight_like_cpp(&ai_kind),
                "{ai_kind:?} overrides MoveInLineOfSight with no base auto-aggro"
            );
        }
    }

    #[test]
    fn creature_ai_move_in_line_of_sight_base_users_keep_auto_aggro_path_like_cpp() {
        for ai_kind in [
            CreatureAiKindLikeCpp::AggressorAI,
            CreatureAiKindLikeCpp::GuardAI,
            CreatureAiKindLikeCpp::CombatAI,
            CreatureAiKindLikeCpp::TurretAI,
            CreatureAiKindLikeCpp::SmartAI,
            CreatureAiKindLikeCpp::ScriptedAI("npc_scripted".to_string()),
            CreatureAiKindLikeCpp::UnknownNamedAI("CustomAI".to_string()),
        ] {
            assert!(
                creature_ai_uses_base_move_in_line_of_sight_like_cpp(&ai_kind),
                "{ai_kind:?} should reach the base MoveInLineOfSight aggro path"
            );
        }
    }

    #[test]
    fn evade_reason_indices_and_text_match_cpp_enumutils() {
        let expected = [
            (
                EvadeReasonLikeCpp::NoHostiles,
                "NoHostiles",
                "the creature's threat list is empty",
            ),
            (
                EvadeReasonLikeCpp::Boundary,
                "Boundary",
                "the creature has moved outside its evade boundary",
            ),
            (
                EvadeReasonLikeCpp::NoPath,
                "NoPath",
                "the creature was unable to reach its target for over 5 seconds",
            ),
            (
                EvadeReasonLikeCpp::SequenceBreak,
                "SequenceBreak",
                "this is a boss and the pre-requisite encounters for engaging it are not defeated yet",
            ),
            (EvadeReasonLikeCpp::Other, "Other", "anything else"),
        ];

        assert_eq!(EvadeReasonLikeCpp::COUNT_LIKE_CPP, expected.len());
        for (index, (reason, constant, description)) in expected.into_iter().enumerate() {
            assert_eq!(reason.to_index_like_cpp(), index);
            assert_eq!(EvadeReasonLikeCpp::from_index_like_cpp(index), Some(reason));
            assert_eq!(reason.constant_like_cpp(), constant);
            assert_eq!(reason.description_like_cpp(), description);
        }
        assert_eq!(
            EvadeReasonLikeCpp::from_index_like_cpp(expected.len()),
            None
        );
    }

    #[test]
    fn enter_evade_plan_returns_none_if_already_evading_like_cpp() {
        let plan = creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
            is_in_evade_mode: true,
            ..CreatureEnterEvadeInputLikeCpp::default()
        });

        assert!(plan.is_none());
    }

    #[test]
    fn enter_evade_plan_dead_creature_only_ends_engagement_like_cpp() {
        let plan = creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
            reason: EvadeReasonLikeCpp::NoPath,
            is_alive: false,
            ..CreatureEnterEvadeInputLikeCpp::default()
        })
        .unwrap();

        assert_eq!(plan.reason, EvadeReasonLikeCpp::NoPath);
        assert!(plan.engagement_over);
        assert!(!plan.remove_auras_on_evade);
        assert!(!plan.combat_stop_with_pets);
        assert!(!plan.reset_ai);
        assert_eq!(plan.movement, CreatureEvadeMovementLikeCpp::NoneVehicle);
    }

    #[test]
    fn enter_evade_plan_targets_home_and_resets_alive_ownerless_creature_like_cpp() {
        let plan = creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
            reason: EvadeReasonLikeCpp::Boundary,
            ..CreatureEnterEvadeInputLikeCpp::default()
        })
        .unwrap();

        assert_eq!(plan.reason, EvadeReasonLikeCpp::Boundary);
        assert!(plan.remove_auras_on_evade);
        assert!(plan.combat_stop_with_pets);
        assert!(plan.clear_tap);
        assert!(plan.reset_player_damage_req);
        assert!(plan.clear_last_damaged_time);
        assert!(plan.clear_cannot_reach_target);
        assert!(plan.clear_spell_focus_target);
        assert!(plan.clear_target);
        assert!(plan.reset_spell_cooldowns);
        assert!(plan.engagement_over);
        assert!(plan.reset_ai);
        assert_eq!(
            plan.movement,
            CreatureEvadeMovementLikeCpp::TargetedHomeAddEvadeState
        );
    }

    #[test]
    fn enter_evade_plan_follows_owner_unless_vehicle_like_cpp() {
        let owner = guid(900);
        let owner_plan = creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
            owner_guid: Some(owner),
            tap_list_not_cleared_on_evade: true,
            ..CreatureEnterEvadeInputLikeCpp::default()
        })
        .unwrap();

        assert_eq!(
            owner_plan.movement,
            CreatureEvadeMovementLikeCpp::FollowOwner {
                owner_guid: owner,
                pet_follow_distance: 1.0,
            }
        );
        assert!(!owner_plan.clear_tap);

        let vehicle_plan =
            creature_enter_evade_mode_plan_like_cpp(CreatureEnterEvadeInputLikeCpp {
                has_vehicle: true,
                owner_guid: Some(owner),
                ..CreatureEnterEvadeInputLikeCpp::default()
            })
            .unwrap();

        assert_eq!(
            vehicle_plan.movement,
            CreatureEvadeMovementLikeCpp::NoneVehicle
        );
    }

    #[test]
    fn trigger_alert_plans_ai_reaction_and_distract_like_cpp() {
        let plan = creature_trigger_alert_plan_like_cpp(CreatureTriggerAlertInputLikeCpp {
            absolute_angle_to_target: 1.25,
            ..CreatureTriggerAlertInputLikeCpp::default()
        })
        .unwrap();

        assert!(plan.send_ai_reaction_alert);
        assert_eq!(plan.move_distract_ms, 5_000);
        assert_eq!(plan.orientation, 1.25);
    }

    #[test]
    fn trigger_alert_requires_existing_player_target_like_cpp() {
        for input in [
            CreatureTriggerAlertInputLikeCpp {
                target_exists: false,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
            CreatureTriggerAlertInputLikeCpp {
                target_is_player: false,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
        ] {
            assert!(creature_trigger_alert_plan_like_cpp(input).is_none());
        }
    }

    #[test]
    fn trigger_alert_skips_invalid_creature_states_like_cpp() {
        for input in [
            CreatureTriggerAlertInputLikeCpp {
                creature_is_unit: false,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
            CreatureTriggerAlertInputLikeCpp {
                creature_is_engaged: true,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
            CreatureTriggerAlertInputLikeCpp {
                creature_is_confused: true,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
            CreatureTriggerAlertInputLikeCpp {
                creature_is_stunned: true,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
            CreatureTriggerAlertInputLikeCpp {
                creature_is_fleeing: true,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
            CreatureTriggerAlertInputLikeCpp {
                creature_is_distracted: true,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
        ] {
            assert!(creature_trigger_alert_plan_like_cpp(input).is_none());
        }
    }

    #[test]
    fn trigger_alert_requires_hostile_acceptable_non_passive_non_civilian_like_cpp() {
        for input in [
            CreatureTriggerAlertInputLikeCpp {
                creature_is_civilian: true,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
            CreatureTriggerAlertInputLikeCpp {
                creature_has_react_passive: true,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
            CreatureTriggerAlertInputLikeCpp {
                creature_is_hostile_to_target: false,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
            CreatureTriggerAlertInputLikeCpp {
                target_acceptable: false,
                ..CreatureTriggerAlertInputLikeCpp::default()
            },
        ] {
            assert!(creature_trigger_alert_plan_like_cpp(input).is_none());
        }
    }

    fn zone_player(
        low: i64,
        controlled_unit_lows: &[i64],
        vehicle_base_low: Option<i64>,
    ) -> CreatureZoneInCombatPlayerLikeCpp {
        CreatureZoneInCombatPlayerLikeCpp {
            player_guid: guid(low),
            is_alive: true,
            can_begin_combat: true,
            controlled_unit_guids: controlled_unit_lows.iter().copied().map(guid).collect(),
            vehicle_base_guid: vehicle_base_low.map(guid),
        }
    }

    #[test]
    fn do_zone_in_combat_logs_and_returns_on_non_dungeon_like_cpp() {
        let plan = creature_do_zone_in_combat_plan_like_cpp(CreatureZoneInCombatInputLikeCpp {
            creature_guid: guid(1),
            map_is_dungeon: false,
            players: vec![zone_player(10, &[11], Some(12))],
        });

        assert!(plan.log_non_dungeon_error);
        assert!(plan.engage_targets.is_empty());
    }

    #[test]
    fn do_zone_in_combat_dungeon_without_players_is_noop_like_cpp() {
        let plan = creature_do_zone_in_combat_plan_like_cpp(CreatureZoneInCombatInputLikeCpp {
            creature_guid: guid(1),
            map_is_dungeon: true,
            players: Vec::new(),
        });

        assert!(!plan.log_non_dungeon_error);
        assert!(plan.engage_targets.is_empty());
    }

    #[test]
    fn do_zone_in_combat_skips_dead_or_combat_blocked_players_like_cpp() {
        let mut dead = zone_player(10, &[11], Some(12));
        dead.is_alive = false;
        let mut blocked = zone_player(20, &[21], Some(22));
        blocked.can_begin_combat = false;

        let plan = creature_do_zone_in_combat_plan_like_cpp(CreatureZoneInCombatInputLikeCpp {
            creature_guid: guid(1),
            map_is_dungeon: true,
            players: vec![dead, blocked, zone_player(30, &[], None)],
        });

        assert_eq!(plan.engage_targets, vec![guid(30)]);
    }

    #[test]
    fn do_zone_in_combat_engages_player_controlled_units_and_vehicle_in_order_like_cpp() {
        let plan = creature_do_zone_in_combat_plan_like_cpp(CreatureZoneInCombatInputLikeCpp {
            creature_guid: guid(1),
            map_is_dungeon: true,
            players: vec![
                zone_player(10, &[11, 12], Some(13)),
                zone_player(20, &[21], None),
            ],
        });

        assert_eq!(
            plan.engage_targets,
            vec![guid(10), guid(11), guid(12), guid(13), guid(20), guid(21)]
        );
    }

    fn target_candidate(
        low: i64,
        threat_order: u32,
        distance_to_me: f32,
    ) -> UnitAiTargetCandidateLikeCpp {
        UnitAiTargetCandidateLikeCpp {
            guid: guid(low),
            is_offline: false,
            is_current_victim: false,
            is_last_victim: false,
            threat_order,
            distance_to_me,
            is_player: true,
            has_aura: false,
        }
    }

    #[test]
    fn select_target_list_uses_current_victim_then_sorted_threat_like_cpp() {
        let mut low_threat = target_candidate(70, 2, 20.0);
        let mut current = target_candidate(71, 1, 30.0);
        current.is_current_victim = true;
        let high_threat = target_candidate(72, 0, 10.0);

        let selected = select_target_list_like_cpp(
            &[low_threat, current, high_threat],
            usize::MAX,
            SelectTargetMethodLikeCpp::MaxThreat,
            0,
            DefaultTargetSelectorLikeCpp::default(),
        );

        assert_eq!(
            selected,
            vec![current.guid, high_threat.guid, low_threat.guid]
        );

        low_threat.is_offline = true;
        let selected = select_target_list_like_cpp(
            &[low_threat, current, high_threat],
            usize::MAX,
            SelectTargetMethodLikeCpp::MaxThreat,
            0,
            DefaultTargetSelectorLikeCpp::default(),
        );

        assert_eq!(
            selected,
            vec![current.guid, high_threat.guid],
            "C++ skips offline sorted threat refs but keeps current victim if present"
        );
    }

    #[test]
    fn select_target_list_min_threat_reverses_prepared_max_threat_order_like_cpp() {
        let mut current = target_candidate(80, 0, 10.0);
        current.is_current_victim = true;
        let middle = target_candidate(81, 1, 10.0);
        let low = target_candidate(82, 2, 10.0);

        let selected = select_target_list_like_cpp(
            &[current, middle, low],
            2,
            SelectTargetMethodLikeCpp::MinThreat,
            0,
            DefaultTargetSelectorLikeCpp::default(),
        );

        assert_eq!(selected, vec![low.guid, middle.guid]);
    }

    #[test]
    fn select_target_list_distance_methods_sort_unsorted_live_threat_like_cpp() {
        let near = target_candidate(90, 2, 5.0);
        let far = target_candidate(91, 1, 30.0);
        let mid = target_candidate(92, 0, 15.0);

        assert_eq!(
            select_target_list_like_cpp(
                &[near, far, mid],
                usize::MAX,
                SelectTargetMethodLikeCpp::MaxDistance,
                0,
                DefaultTargetSelectorLikeCpp::default(),
            ),
            vec![far.guid, mid.guid, near.guid]
        );
        assert_eq!(
            select_target_list_like_cpp(
                &[near, far, mid],
                usize::MAX,
                SelectTargetMethodLikeCpp::MinDistance,
                1,
                DefaultTargetSelectorLikeCpp::default(),
            ),
            vec![mid.guid, far.guid],
            "C++ applies offset after distance sorting"
        );
    }

    #[test]
    fn select_target_list_applies_offset_before_default_selector_like_cpp() {
        let skipped = target_candidate(100, 0, 5.0);
        let kept = target_candidate(101, 1, 5.0);

        let selected = select_target_list_like_cpp(
            &[skipped, kept],
            usize::MAX,
            SelectTargetMethodLikeCpp::MaxThreat,
            1,
            DefaultTargetSelectorLikeCpp {
                dist: 10.0,
                player_only: true,
                with_tank: true,
                aura: 0,
            },
        );

        assert_eq!(selected, vec![kept.guid]);
    }

    #[test]
    fn default_target_selector_matches_cpp_filters() {
        let mut target = target_candidate(110, 0, 12.0);
        assert!(!default_target_selector_accepts_like_cpp(
            &target,
            DefaultTargetSelectorLikeCpp {
                dist: 10.0,
                ..DefaultTargetSelectorLikeCpp::default()
            }
        ));
        assert!(!default_target_selector_accepts_like_cpp(
            &target,
            DefaultTargetSelectorLikeCpp {
                dist: -20.0,
                ..DefaultTargetSelectorLikeCpp::default()
            }
        ));

        target.is_player = false;
        assert!(!default_target_selector_accepts_like_cpp(
            &target,
            DefaultTargetSelectorLikeCpp {
                player_only: true,
                ..DefaultTargetSelectorLikeCpp::default()
            }
        ));

        target.is_player = true;
        target.is_last_victim = true;
        assert!(!default_target_selector_accepts_like_cpp(
            &target,
            DefaultTargetSelectorLikeCpp {
                with_tank: false,
                ..DefaultTargetSelectorLikeCpp::default()
            }
        ));

        target.is_last_victim = false;
        target.has_aura = false;
        assert!(!default_target_selector_accepts_like_cpp(
            &target,
            DefaultTargetSelectorLikeCpp {
                aura: 123,
                ..DefaultTargetSelectorLikeCpp::default()
            }
        ));

        target.has_aura = true;
        assert!(!default_target_selector_accepts_like_cpp(
            &target,
            DefaultTargetSelectorLikeCpp {
                aura: -123,
                ..DefaultTargetSelectorLikeCpp::default()
            }
        ));
    }

    fn creature_with_boss_id(boss_id: Option<u32>) -> CreatureAI {
        CreatureAI::new(
            ObjectGuid::EMPTY,
            1,
            Position::ZERO,
            100,
            1,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
            0,
            0,
            0,
            boss_id,
            0,
        )
    }

    #[test]
    fn plain_creature_has_no_boss_ai_view_like_cpp_failed_dynamic_cast() {
        let creature = creature_with_boss_id(None);

        assert!(creature.boss_ai_like_cpp().is_none());
    }

    #[test]
    fn boss_creature_exposes_script_boss_id_like_cpp_boss_ai() {
        let creature = creature_with_boss_id(Some(7));

        assert_eq!(creature.boss_ai_like_cpp().unwrap().boss_id(), 7);
    }

    fn guid(low: i64) -> ObjectGuid {
        ObjectGuid::new(0, low)
    }

    fn creature_view(
        guid: ObjectGuid,
        entry: u32,
        ai_enabled: bool,
    ) -> SummonListCreatureViewLikeCpp {
        SummonListCreatureViewLikeCpp {
            guid,
            entry,
            ai_enabled,
        }
    }

    fn resolve_from(
        creatures: &HashMap<ObjectGuid, SummonListCreatureViewLikeCpp>,
        guid: ObjectGuid,
    ) -> Option<SummonListCreatureViewLikeCpp> {
        creatures.get(&guid).copied()
    }

    #[test]
    fn summon_list_preserves_order_and_removes_all_matching_guids_like_cpp() {
        let mut summons = SummonListLikeCpp::new();
        let a = guid(1);
        let b = guid(2);

        summons.summon_like_cpp(a);
        summons.summon_like_cpp(b);
        summons.summon_like_cpp(a);

        assert_eq!(summons.iter_like_cpp().collect::<Vec<_>>(), vec![a, b, a]);
        assert_eq!(summons.size_like_cpp(), 3);

        summons.despawn_like_cpp(a);

        assert_eq!(summons.iter_like_cpp().collect::<Vec<_>>(), vec![b]);
    }

    #[test]
    fn summon_list_despawn_all_drains_fifo_and_ignores_missing_like_cpp() {
        let mut summons = SummonListLikeCpp::new();
        let a = guid(10);
        let missing = guid(11);
        let b = guid(12);
        let creatures = HashMap::from([
            (a, creature_view(a, 100, true)),
            (b, creature_view(b, 200, true)),
        ]);

        summons.summon_like_cpp(a);
        summons.summon_like_cpp(missing);
        summons.summon_like_cpp(b);

        let plan = summons.despawn_all_like_cpp(|guid| resolve_from(&creatures, guid));

        assert_eq!(plan, vec![a, b]);
        assert!(summons.empty_like_cpp());
    }

    #[test]
    fn summon_list_despawn_entry_erases_missing_and_matching_entry_like_cpp() {
        let mut summons = SummonListLikeCpp::new();
        let first_match = guid(20);
        let kept = guid(21);
        let missing = guid(22);
        let second_match = guid(23);
        let creatures = HashMap::from([
            (first_match, creature_view(first_match, 777, true)),
            (kept, creature_view(kept, 888, true)),
            (second_match, creature_view(second_match, 777, true)),
        ]);

        for guid in [first_match, kept, missing, second_match] {
            summons.summon_like_cpp(guid);
        }

        let plan = summons.despawn_entry_like_cpp(777, |guid| resolve_from(&creatures, guid));

        assert_eq!(plan, vec![first_match, second_match]);
        assert_eq!(summons.iter_like_cpp().collect::<Vec<_>>(), vec![kept]);
    }

    #[test]
    fn summon_list_remove_not_existing_and_has_entry_use_object_accessor_view_like_cpp() {
        let mut summons = SummonListLikeCpp::new();
        let missing = guid(30);
        let wrong_entry = guid(31);
        let wanted = guid(32);
        let creatures = HashMap::from([
            (wrong_entry, creature_view(wrong_entry, 1, true)),
            (wanted, creature_view(wanted, 2, true)),
        ]);

        for guid in [missing, wrong_entry, wanted] {
            summons.summon_like_cpp(guid);
        }

        assert!(summons.has_entry_like_cpp(2, |guid| resolve_from(&creatures, guid)));
        assert!(!summons.has_entry_like_cpp(3, |guid| resolve_from(&creatures, guid)));

        summons.remove_not_existing_like_cpp(|guid| resolve_from(&creatures, guid));

        assert_eq!(
            summons.iter_like_cpp().collect::<Vec<_>>(),
            vec![wrong_entry, wanted]
        );
    }

    #[test]
    fn summon_list_do_zone_in_combat_filters_ai_and_optional_entry_like_cpp() {
        let mut summons = SummonListLikeCpp::new();
        let ai_match = guid(40);
        let no_ai = guid(41);
        let ai_other_entry = guid(42);
        let missing = guid(43);
        let creatures = HashMap::from([
            (ai_match, creature_view(ai_match, 9, true)),
            (no_ai, creature_view(no_ai, 9, false)),
            (ai_other_entry, creature_view(ai_other_entry, 10, true)),
        ]);

        for guid in [ai_match, no_ai, ai_other_entry, missing] {
            summons.summon_like_cpp(guid);
        }

        assert_eq!(
            summons.do_zone_in_combat_like_cpp(9, |guid| resolve_from(&creatures, guid)),
            vec![ai_match]
        );
        assert_eq!(
            summons.do_zone_in_combat_like_cpp(0, |guid| resolve_from(&creatures, guid)),
            vec![ai_match, ai_other_entry]
        );
        assert_eq!(
            summons.iter_like_cpp().collect::<Vec<_>>(),
            vec![ai_match, no_ai, ai_other_entry, missing],
            "C++ DoZoneInCombat does not prune missing GUIDs"
        );
    }

    #[test]
    fn summon_list_do_action_uncapped_copies_then_resolves_ai_like_cpp() {
        let mut summons = SummonListLikeCpp::new();
        let selected_ai = guid(50);
        let selected_no_ai = guid(51);
        let not_selected = guid(52);
        let creatures = HashMap::from([
            (selected_ai, creature_view(selected_ai, 1, true)),
            (selected_no_ai, creature_view(selected_no_ai, 1, false)),
            (not_selected, creature_view(not_selected, 2, true)),
        ]);

        for guid in [selected_ai, selected_no_ai, not_selected] {
            summons.summon_like_cpp(guid);
        }

        let actions = summons.do_action_like_cpp(
            42,
            |guid| guid != not_selected,
            |guid| resolve_from(&creatures, guid),
        );

        assert_eq!(
            actions,
            vec![SummonListActionLikeCpp {
                guid: selected_ai,
                action: 42,
            }]
        );
    }

    #[test]
    fn summon_list_do_action_capped_uses_random_resize_before_ai_resolution_like_cpp() {
        let mut summons = SummonListLikeCpp::new();
        let selected_ai = guid(60);
        let creatures = HashMap::from([(selected_ai, creature_view(selected_ai, 1, true))]);

        for guid in [selected_ai, selected_ai, selected_ai] {
            summons.summon_like_cpp(guid);
        }

        let actions = summons.do_action_with_max_like_cpp(
            77,
            |_| true,
            2,
            |guid| resolve_from(&creatures, guid),
        );

        assert_eq!(
            actions,
            vec![
                SummonListActionLikeCpp {
                    guid: selected_ai,
                    action: 77,
                },
                SummonListActionLikeCpp {
                    guid: selected_ai,
                    action: 77,
                },
            ]
        );
        assert_eq!(
            summons.iter_like_cpp().collect::<Vec<_>>(),
            vec![selected_ai, selected_ai, selected_ai],
            "C++ DoAction works on a copy and must not mutate the original SummonList"
        );
    }
}

//! Creature AI state machines state definitions, part 1 of 2.
//!
//! Separated from the lib.rs root under #658. Behaviour is preserved.

use super::*;

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
    pub(crate) fn from_registered_ai_name_like_cpp(ai_name: &str) -> Self {
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

pub(crate) fn select_creature_ai_by_permit_like_cpp(
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

pub(crate) fn permit_aggressor_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if !input.is_civilian && !input.is_neutral_to_all {
        100
    } else {
        -1
    }
}

pub(crate) fn permit_critter_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_critter && !input.is_guardian {
        200
    } else {
        -1
    }
}

pub(crate) fn permit_guard_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_guard { 200 } else { -1 }
}

pub(crate) fn permit_null_creature_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.has_spellclick_npc_flag {
        250
    } else if input.is_trigger {
        200
    } else {
        1
    }
}

pub(crate) fn permit_pet_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
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

pub(crate) fn permit_reactor_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_civilian || input.is_neutral_to_all {
        100
    } else {
        -1
    }
}

pub(crate) fn permit_totem_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_totem { 200 } else { -1 }
}

pub(crate) fn permit_trigger_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_trigger && input.first_spell_id != 0 {
        800
    } else {
        -1
    }
}

pub(crate) fn permit_vehicle_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_vehicle { 800 } else { -1 }
}

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

pub(crate) fn prepare_target_list_selection_like_cpp(
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

pub(crate) fn finalize_target_list_selection_like_cpp(
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
    pub(crate) storage: VecDeque<ObjectGuid>,
}

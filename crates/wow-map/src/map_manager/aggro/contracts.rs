//! Owned facts and effects for the complete map Aggro operation.
use wow_constants::UnitFlags;
use wow_core::{ObjectGuid, Position};
use wow_entities::{PhaseShift, UnitVisibilityDetectionStateLikeCpp};

#[derive(Debug)]
pub struct AggroCandidate {
    pub player_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub map_difficulty_id: u8,
    pub position: Position,
    pub player_visibility_represented: bool,
    pub player_phase_shift: PhaseShift,
    pub player_visibility_detection: UnitVisibilityDetectionStateLikeCpp,
    pub player_combat_reach: f32,
    pub player_detected_range_aura_mod: f32,
    pub player_liquid_status: u32,
    pub player_level: u8,
    pub player_gray_level: u8,
    pub player_unit_flags: u32,
    pub player_unit_flags2: u32,
    pub player_unit_state: u32,
    pub player_is_game_master: bool,
    pub player_is_contested_pvp: bool,
    pub player_faction_template_id: u32,
    pub player_reputation_standings: Vec<(u32, i32)>,
    pub player_reputation_state_flags: Vec<(u32, u32)>,
    pub player_forced_reputation_ranks:
        Vec<(u32, wow_constants::reputation::ReputationRankLikeCpp)>,
    pub player_forced_reputation_faction_ids: Vec<u32>,
    pub player_school_immunity_mask: u32,
    pub player_damage_immunity_mask: u32,
    pub player_has_confuse_aura: bool,
    pub player_has_breakable_stun_aura: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggroOwnerSnapshot {
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub phase_shift: PhaseShift,
    pub combat_reach: f32,
    pub alive: bool,
    pub in_water: bool,
    pub in_evade_mode: bool,
    pub unit_flags: UnitFlags,
    pub faction_template_id: Option<u32>,
    pub school_immunity_mask: u32,
    pub damage_immunity_mask: u32,
    pub has_confuse_aura: bool,
    pub has_breakable_stun_aura: bool,
}

#[derive(Debug, Default)]
pub struct AggroOutcome {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub sightless_creatures_skipped: usize,
    pub candidates_seen: usize,
    pub targetability_rejections: usize,
    pub visibility_unrepresented: usize,
    pub visibility_rejections: usize,
    pub hostility_rejections: usize,
    pub hostility_unrepresented: usize,
    pub accessibility_rejections: usize,
    pub owner_position_unrepresented: usize,
    pub attacker_evade_rejections: usize,
    pub home_range_rejections: usize,
    pub gray_aggro_rejections: usize,
    pub ai_selection_unrepresented: usize,
    pub ai_los_suppressed: usize,
    pub ai_can_attack_unrepresented: usize,
    pub ai_can_attack_rejections: usize,
    pub alert_triggers: usize,
    pub alert_rejections: usize,
    pub movement_interrupts: usize,
    pub victim_switches: usize,
    pub evades_started: usize,
    pub assistance_scheduled: usize,
    pub assistance_starts: usize,
    pub effects: Vec<AggroEffect>,
    pub aggro_starts: usize,
    pub commands: Vec<AggroAttackStart>,
    pub stop_commands: Vec<AggroAttackStop>,
}

#[derive(Debug, Clone, Copy)]
pub struct AggroSettings {
    pub no_gray_aggro_above: u32,
    pub no_gray_aggro_below: u32,
    pub creature_aggro_rate: f32,
    pub max_player_level_config: u32,
    pub family_assistance_radius: f32,
    pub family_assistance_delay_ms: u32,
    pub map_is_dungeon: bool,
    pub map_visibility_range: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum AggroAiKind {
    Base,
    Turret,
    NoBaseLos,
}
#[derive(Debug, Clone, Copy)]
pub enum AggroAiSelection {
    Selected(AggroAiKind),
    Unrepresented,
}
#[derive(Debug, Clone, Copy)]
pub enum AggroAttackDecision {
    Allowed,
    Rejected,
    Unrepresented,
}
#[derive(Debug, Clone, Copy)]
pub enum AggroVisibility {
    Allowed,
    Rejected,
    Unrepresented,
}
#[derive(Debug, Clone, Copy)]
pub enum AggroLeash {
    Allowed,
    OwnerPositionUnrepresented,
    HomeRangeRejected,
}
#[derive(Debug)]
pub enum AggroThreatUpdate {
    Unchanged,
    Switched {
        previous_victim: ObjectGuid,
    },
    Evade {
        previous_victim: Option<ObjectGuid>,
        participant_guids: Vec<ObjectGuid>,
        removed_taunt_slots: Vec<u8>,
    },
}

pub enum AggroFactionTarget<'a> {
    Player(&'a AggroCandidate),
    Unit(Option<u32>),
}
pub struct AggroAiFacts<'a> {
    pub ai_name: &'a str,
    pub script_name: &'a str,
    pub is_pet: bool,
    pub is_vehicle: bool,
    pub is_totem: bool,
    pub flags_extra: u32,
    pub first_spell_id: u32,
    pub creature_type: u32,
    pub is_guardian: bool,
    pub is_civilian: bool,
    pub faction_template: i32,
    pub npc_flags: u32,
    pub is_controllable_guardian: bool,
    pub owner_is_player: bool,
}
pub struct AggroTurretFacts {
    pub kind: AggroAiKind,
    pub first_spell_id: u32,
    pub difficulty: u8,
    pub position: Position,
    pub combat_reach: f32,
    pub target_position: Position,
    pub target_combat_reach: f32,
}
pub struct AggroDistanceFacts {
    pub aggro_rate: f32,
    pub creature_combat_reach: f32,
    pub required_expansion: u8,
    pub max_player_level_config: u32,
    pub player_level_for_target: u8,
    pub creature_level_for_target: u8,
    pub creature_detect_range_aura_mod: f32,
    pub player_detected_range_aura_mod: f32,
}

/// Catalog callbacks accept immutable Aggro facts only. They perform no I/O
/// and expose neither map storage nor an Actor/witness to the application.
pub struct AggroPolicies<'a> {
    pub hostility: &'a mut dyn FnMut(i32, AggroFactionTarget<'_>) -> Option<bool>,
    pub select_ai: &'a mut dyn FnMut(AggroAiFacts<'_>) -> AggroAiSelection,
    pub can_attack: &'a mut dyn FnMut(AggroTurretFacts) -> AggroAttackDecision,
    pub attack_distance: &'a mut dyn FnMut(AggroDistanceFacts) -> f32,
}

#[derive(Debug)]
pub struct AggroEffect {
    pub source_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub visibility_range: f32,
    pub kind: AggroEffectKind,
}
#[derive(Debug)]
pub enum AggroEffectKind {
    RemoveAuras(Vec<u8>),
    AttackStart { victim: ObjectGuid },
    AttackStop { victim: ObjectGuid },
    MoveStop(wow_movement::MoveSplineStopResult),
    Alert,
}
#[derive(Debug)]
pub struct AggroAttackStart {
    pub attacker_guid: ObjectGuid,
    pub victim_guid: ObjectGuid,
    pub previous_victim_guid: Option<ObjectGuid>,
    pub map_id: u16,
    pub instance_id: u32,
    pub packet_already_broadcast: bool,
}
#[derive(Debug)]
pub struct AggroAttackStop {
    pub attacker_guid: ObjectGuid,
    pub victim_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
}

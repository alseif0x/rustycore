//! Spell-specific immutable catalog facts and owned operation results.
use wow_core::{ObjectGuid, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellAiKind {
    Combat,
    Turret,
    Other,
    Unrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellCondition {
    Aggro,
    Combat,
    Die,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpellTarget {
    SelfTarget,
    Victim,
    Enemy,
    Buff,
    Debuff,
}

impl SpellTarget {
    pub fn requires_random_threat_selection(self) -> bool {
        matches!(self, Self::Enemy | Self::Debuff)
    }

    pub fn resolve_single_player(self, caster: ObjectGuid, victim: ObjectGuid) -> ObjectGuid {
        match self {
            Self::SelfTarget | Self::Buff => caster,
            Self::Victim | Self::Enemy | Self::Debuff => victim,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellDisable {
    Enabled,
    Disabled,
    Unrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellTopologyError {
    NonInstant,
    ProjectileOrAmmo,
    EffectOrTarget,
}

#[derive(Debug)]
pub struct SpellEffectFacts {
    pub effect: u32,
    pub effect_index: u32,
    pub effect_aura: i32,
    pub effect_base_points: i32,
    pub implicit_target_1: u32,
    pub implicit_target_2: u32,
    pub chain_targets: i32,
    pub effect_radius_index_1: u32,
    pub effect_trigger_spell: i32,
}

#[derive(Debug)]
pub struct SpellPowerFacts {
    pub power_type: i8,
    pub mana_cost: i32,
    pub mana_cost_per_level: i32,
    pub mana_per_second: i32,
    pub power_cost_pct: f32,
    pub power_cost_max_pct: f32,
    pub power_pct_per_second: f32,
    pub required_aura_spell_id: i32,
    pub optional_cost: u32,
}

/// An effective metadata projection, never a Creature or a catalog cache.
#[derive(Debug)]
pub struct SpellInfoFacts {
    pub spell_id: i32,
    pub cast_time_ms: u32,
    pub requires_spell_focus: u32,
    pub effect_type: u32,
    pub aura_type: Option<i32>,
    pub effect_base_points: i32,
    pub effects: Vec<SpellEffectFacts>,
    pub power_costs: Vec<SpellPowerFacts>,
}

#[derive(Debug, Clone, Copy)]
pub struct SpellCooldown {
    pub spell_id: u32,
    pub category_id: u32,
    pub recovery_time_ms: u64,
    pub category_recovery_time_ms: u64,
    pub passive: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SpellRange {
    pub minimum: f32,
    pub maximum: f32,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellHitProfile {
    NoAttackMissAfterRequiredRoll,
    BaseMeleeMiss {
        miss_threshold_per_ten_thousand: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellHit {
    Hit,
    Miss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellValidation {
    Ready(SpellHit),
    OutOfRange,
    LosRejected,
    MissingTarget,
    TargetRejected,
    CooldownRejected,
    HitResultUnrepresented,
    RuntimeRngAuthorityRejected,
    CasterIncarnationRejected,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SpellLogPower {
    pub power_type: i32,
    pub amount: i32,
    pub cost: i32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SpellLog {
    pub health: i64,
    pub attack_power: i32,
    pub spell_power: i32,
    pub armor: i32,
    pub power_data: Vec<SpellLogPower>,
}

/// Already-committed state. APP samples the GO timestamp during append.
#[derive(Debug)]
pub struct SpellCompletion {
    pub caster_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub spell_id: i32,
    pub spell_x_spell_visual_id: u32,
    pub cast_time_ms: u32,
    pub spell_go_cast_flags: u32,
    pub cast_id: ObjectGuid,
    pub hit: SpellHit,
    pub position: Position,
    pub visibility_range: f32,
    pub log: SpellLog,
}

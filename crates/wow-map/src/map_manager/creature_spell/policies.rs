//! Lazy spell-specific catalog interfaces. No mutable actor/storage callback.
use super::*;
use crate::map_manager::AggroAiFacts;

#[derive(Debug, Clone, Copy)]
pub enum SpellPreparationCheck {
    RuntimeHooks, CastingRequirements, ShapeshiftRequirements, AuraRestrictions,
    CooldownSemantics, CombatForbidden, TargetRestrictions, Projectile,
}

#[derive(Debug)]
pub struct SpellHitFacts {
    pub defense_type: i8, pub school_mask: u8, pub spell_mechanic: i8,
    pub effect_mechanics: std::collections::BTreeMap<u32, i32>,
}

#[derive(Debug, Clone, Copy)]
pub struct SpellFactionFacts {
    pub creature_faction_id: u32, pub contested_guard: bool,
    pub caster_hostile: bool, pub victim_hostile: bool,
    pub caster_friendly: bool, pub victim_friendly: bool,
}

pub struct SpellPolicies<'a> {
    pub select_ai: &'a mut dyn FnMut(AggroAiFacts<'_>) -> SpellAiKind,
    pub info: &'a mut dyn FnMut(u32, u8, bool) -> Option<SpellInfoFacts>,
    pub condition: &'a mut dyn FnMut(u32, u8) -> SpellCondition,
    pub target: &'a mut dyn FnMut(u32, &SpellInfoFacts, u8) -> SpellTarget,
    pub disable: &'a mut dyn FnMut(u32, u16, u32) -> SpellDisable,
    pub check: &'a mut dyn FnMut(SpellPreparationCheck, u32, u8) -> bool,
    pub attributes: &'a mut dyn FnMut(i32, u8) -> Option<[u32; 15]>,
    pub has_attribute: &'a mut dyn FnMut(i32, u8, usize, u32) -> bool,
    pub minimum: &'a mut dyn FnMut(u32, u8) -> u64,
    pub visual: &'a mut dyn FnMut(u32, u8) -> Result<u32, ()>,
    pub go_flags: &'a mut dyn FnMut(u32, u8) -> u32,
    pub cooldown: &'a mut dyn FnMut(u32, u8) -> Option<SpellCooldown>,
    pub range: &'a mut dyn FnMut(u32, u8) -> Option<SpellRange>,
    pub hit_metadata: &'a mut dyn FnMut(i32, u8) -> Option<SpellHitFacts>,
    pub faction_authority: &'a mut dyn FnMut() -> bool,
    pub factions: &'a mut dyn FnMut(u32, u32) -> Option<SpellFactionFacts>,
    pub can_have_reputation: &'a mut dyn FnMut(u32) -> Option<bool>,
}

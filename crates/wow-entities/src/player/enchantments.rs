//! Native skill-gated enchantment application (`Player.cpp:13663-14068`).
//!
//! The catalog is read-only. Live item facts and every effect are supplied by
//! the same Player owner; effect callbacks are synchronous and may re-enter its
//! skills/book state.

mod apply;
mod effects;
mod scaling;
mod scan;
#[cfg(test)]
#[path = "../../unit_tests/player/enchantments/mod.rs"]
mod tests;

use super::PlayerSkills;
use wow_constants::{EnchantmentSlot, Stats, WeaponAttackType};
use wow_core::ObjectGuid;

pub const SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP: usize = crate::MAX_ENCHANTMENT_SLOT;
pub const SKILL_ENCHANTMENT_SLOT_PERMANENT_LIKE_CPP: u16 =
    EnchantmentSlot::EnhancementPermanent as u16;
pub const PLAYER_ITEM_SLOT_END_LIKE_CPP: u8 = crate::REAGENT_BAG_SLOT_END;
pub const PLAYER_VISIBLE_ITEM_SLOT_END_LIKE_CPP: u8 = crate::EQUIPMENT_SLOT_END;

/// Applies one enchantment through the same Player-owned skill state used by
/// its guards and synchronous effects.
pub fn apply_enchantment_native_like_cpp<S, E>(
    skills: &mut PlayerSkills,
    item_guid: Option<ObjectGuid>,
    slot: u16,
    apply: bool,
    args: SkillEnchantmentApplyArgs,
    sources: &S,
    effects: &mut E,
) -> Result<(), SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    apply::apply_enchantment(skills, item_guid, slot, apply, args, sources, effects)
}

/// Applies every enchantment slot in native order, using the source overload's
/// fixed duration and condition defaults.
pub fn apply_all_enchantments_native_like_cpp<S, E>(
    skills: &mut PlayerSkills,
    item_guid: Option<ObjectGuid>,
    apply: bool,
    sources: &S,
    effects: &mut E,
) -> Result<(), SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    apply::apply_all_enchantments(skills, item_guid, apply, sources, effects)
}

/// Re-evaluates item enchantments for a native skill value transition.
pub fn update_skill_enchantments_native_like_cpp<S, E>(
    skills: &mut PlayerSkills,
    skill_id: u16,
    curr_value: u16,
    new_value: u16,
    sources: &S,
    effects: &mut E,
) -> Result<(), SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    scan::update_skill_enchantments(skills, skill_id, curr_value, new_value, sources, effects)
}

/// Exact numeric fields consumed from one native SpellItemEnchantmentEntry.
/// DB2 widths are kept here; in particular amounts remain signed `int32`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillEnchantmentRecord {
    id: u32,
    effects: [u32; 3],
    effect_amounts: [i32; 3],
    effect_args: [u32; 3],
    flags: i32,
    scaling_points: [f32; 3],
    scaling_class: i32,
    scaling_class_restricted: i32,
    condition_id: u32,
    required_skill_id: u32,
    required_skill_rank: u32,
    min_level: u32,
    max_level: u32,
}

impl SkillEnchantmentRecord {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        id: u32,
        effects: [u32; 3],
        effect_amounts: [i32; 3],
        effect_args: [u32; 3],
        flags: i32,
        scaling_points: [f32; 3],
        scaling_class: i32,
        scaling_class_restricted: i32,
        condition_id: u32,
        required_skill_id: u32,
        required_skill_rank: u32,
        min_level: u32,
        max_level: u32,
    ) -> Self {
        Self {
            id,
            effects,
            effect_amounts,
            effect_args,
            flags,
            scaling_points,
            scaling_class,
            scaling_class_restricted,
            condition_id,
            required_skill_id,
            required_skill_rank,
            min_level,
            max_level,
        }
    }

    pub const fn id(&self) -> u32 {
        self.id
    }
    pub const fn effects(&self) -> &[u32; 3] {
        &self.effects
    }
    pub const fn effect_amounts(&self) -> &[i32; 3] {
        &self.effect_amounts
    }
    pub const fn effect_args(&self) -> &[u32; 3] {
        &self.effect_args
    }
    pub const fn flags(&self) -> i32 {
        self.flags
    }
    pub const fn scaling_points(&self) -> &[f32; 3] {
        &self.scaling_points
    }
    pub const fn scaling_class(&self) -> i32 {
        self.scaling_class
    }
    pub const fn scaling_class_restricted(&self) -> i32 {
        self.scaling_class_restricted
    }
    pub const fn condition_id(&self) -> u32 {
        self.condition_id
    }
    pub const fn required_skill_id(&self) -> u32 {
        self.required_skill_id
    }
    pub const fn required_skill_rank(&self) -> u32 {
        self.required_skill_rank
    }
    pub const fn min_level(&self) -> u32 {
        self.min_level
    }
    pub const fn max_level(&self) -> u32 {
        self.max_level
    }
}

/// Small item-template facts needed by enchantment guards and attack routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillEnchantmentItemTemplate {
    inventory_type: i32,
    required_skill_id: u32,
    required_skill_rank: u32,
}

impl SkillEnchantmentItemTemplate {
    pub const fn new(
        inventory_type: i32,
        required_skill_id: u32,
        required_skill_rank: u32,
    ) -> Self {
        Self {
            inventory_type,
            required_skill_id,
            required_skill_rank,
        }
    }
    pub const fn inventory_type(&self) -> i32 {
        self.inventory_type
    }
    pub const fn required_skill_id(&self) -> u32 {
        self.required_skill_id
    }
    pub const fn required_skill_rank(&self) -> u32 {
        self.required_skill_rank
    }
}

/// A short-lived value read from the canonical item owner. It owns no Item and
/// is reacquired after every callback that can mutate Player state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillEnchantmentItemFacts {
    item_guid: ObjectGuid,
    item_id: u32,
    inventory_slot: u8,
    equipped: bool,
    broken: bool,
    enchantment_ids: [u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
    enchantment_durations: [u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
    socket_colors: [u32; 3],
    socketed_gem_item_ids: [Option<u32>; 3],
}

impl SkillEnchantmentItemFacts {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        item_guid: ObjectGuid,
        item_id: u32,
        inventory_slot: u8,
        equipped: bool,
        broken: bool,
        enchantment_ids: [u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        enchantment_durations: [u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        socket_colors: [u32; 3],
        socketed_gem_item_ids: [Option<u32>; 3],
    ) -> Self {
        Self {
            item_guid,
            item_id,
            inventory_slot,
            equipped,
            broken,
            enchantment_ids,
            enchantment_durations,
            socket_colors,
            socketed_gem_item_ids,
        }
    }
    pub const fn item_guid(&self) -> ObjectGuid {
        self.item_guid
    }
    pub const fn item_id(&self) -> u32 {
        self.item_id
    }
    pub const fn inventory_slot(&self) -> u8 {
        self.inventory_slot
    }
    pub const fn equipped(&self) -> bool {
        self.equipped
    }
    pub const fn broken(&self) -> bool {
        self.broken
    }
    pub const fn enchantment_ids(&self) -> &[u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP] {
        &self.enchantment_ids
    }
    pub const fn enchantment_durations(&self) -> &[u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP] {
        &self.enchantment_durations
    }
    pub const fn socket_colors(&self) -> &[u32; 3] {
        &self.socket_colors
    }
    pub const fn socketed_gem_item_ids(&self) -> &[Option<u32>; 3] {
        &self.socketed_gem_item_ids
    }
}

/// Immutable target catalog reads. `Err` means unavailable source authority;
/// `Ok(None)` means the admitted source genuinely has no matching row.
pub trait SkillEnchantmentSource {
    type Error;

    fn enchantment(&self, id: u32) -> Result<Option<SkillEnchantmentRecord>, Self::Error>;
    fn item_template(&self, id: u32) -> Result<Option<SkillEnchantmentItemTemplate>, Self::Error>;
    fn spell_scaling_row_count(&self) -> Result<usize, Self::Error>;
    /// None is an absent physical scaling row. An existing row with an
    /// unsupported class selector returns Some(0.0), as C++ does.
    fn spell_scaling_column(&self, level: u8, class: i32) -> Result<Option<f32>, Self::Error>;
}

/// Synchronous canonical-owner capabilities required by the native operation.
/// Reads return facts, never borrowed Items. `apply_operation` must complete
/// the operation inline or return an error; it may re-enter the supplied skills.
pub trait SkillEnchantmentEffects {
    type Error;

    /// The fact, level, item-limit, and condition queries below are
    /// gameplay-read-only: they do not mutate or re-enter Player state. Only
    /// `apply_operation` may mutate the supplied skills or re-enter owners.
    fn item_in_slot(&self, slot: u8) -> Result<Option<SkillEnchantmentItemFacts>, Self::Error>;
    fn item_by_guid(
        &self,
        guid: ObjectGuid,
    ) -> Result<Option<SkillEnchantmentItemFacts>, Self::Error>;
    fn player_level(&self, skills: &PlayerSkills) -> Result<u8, Self::Error>;
    fn item_level_limits(&self, skills: &PlayerSkills) -> Result<(u32, u32), Self::Error>;
    fn enchantment_condition_fits(
        &self,
        skills: &PlayerSkills,
        condition_id: u32,
    ) -> Result<bool, Self::Error>;
    fn apply_operation(
        &mut self,
        skills: &mut PlayerSkills,
        operation: SkillEnchantmentEffectOperation,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillEnchantmentApplyArgs {
    apply_duration: bool,
    ignore_condition: bool,
}

impl SkillEnchantmentApplyArgs {
    pub const fn new() -> Self {
        Self {
            apply_duration: true,
            ignore_condition: false,
        }
    }
    pub const fn with_options(apply_duration: bool, ignore_condition: bool) -> Self {
        Self {
            apply_duration,
            ignore_condition,
        }
    }
    pub const fn apply_duration(self) -> bool {
        self.apply_duration
    }
    pub const fn ignore_condition(self) -> bool {
        self.ignore_condition
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillEnchantmentUnitMod {
    Mana,
    Health,
    Stat(Stats),
    Resistance(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillEnchantmentUnitModifier {
    BaseValue,
    TotalValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillEnchantmentRating {
    DefenseSkill,
    Dodge,
    Parry,
    Block,
    HitMelee,
    HitRanged,
    HitSpell,
    CritMelee,
    CritRanged,
    CritSpell,
    HasteMelee,
    HasteRanged,
    HasteSpell,
    ResiliencePlayerDamage,
    Expertise,
    ArmorPenetration,
    Mastery,
    VersatilityDamageDone,
    VersatilityHealingDone,
    VersatilityDamageTaken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillEnchantmentAttackPowerIndex {
    Melee,
    Ranged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillEnchantmentAttackPowerModifier {
    FlatPositive,
    FlatNegative,
}

/// One immediate operation delegated to the canonical Player effect receiver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkillEnchantmentEffectOperation {
    UpdateDamageDoneMods {
        attack_type: WeaponAttackType,
        modifier_slot: i16,
    },
    CastItemEquipSpell {
        spell_id: u32,
        item_guid: ObjectGuid,
    },
    RemoveAurasDueToItemSpell {
        spell_id: u32,
        item_guid: ObjectGuid,
    },
    HandleStatFlatModifier {
        unit_mod: SkillEnchantmentUnitMod,
        modifier: SkillEnchantmentUnitModifier,
        amount: f32,
        apply: bool,
    },
    UpdateStatBuffMod(Stats),
    ApplyRatingMod {
        rating: SkillEnchantmentRating,
        amount: i32,
        apply: bool,
    },
    HandleAttackPowerModifier {
        index: SkillEnchantmentAttackPowerIndex,
        kind: SkillEnchantmentAttackPowerModifier,
        amount: f32,
        apply: bool,
    },
    ApplyManaRegenBonus {
        amount: i32,
        apply: bool,
    },
    ApplySpellPowerBonus {
        amount: i32,
        apply: bool,
    },
    ApplyHealthRegenBonus {
        amount: i32,
        apply: bool,
    },
    ApplySpellPenetrationBonus {
        amount: i32,
        apply: bool,
    },
    HandleShieldBlockBaseMod {
        amount: f32,
        apply: bool,
    },
    UpdateVisibleItemVisual {
        item_guid: ObjectGuid,
        inventory_slot: u8,
    },
    AddEnchantmentDuration {
        item_guid: ObjectGuid,
        slot: u16,
        duration_ms: u32,
    },
    LogUnknownEffect {
        enchantment_id: u32,
        effect_type: u32,
    },
}

#[derive(Debug)]
pub enum SkillEnchantmentError<SourceError, EffectError> {
    Source(SourceError),
    Effect(EffectError),
    ItemInvalidated(ObjectGuid),
    MissingLiveItemTemplate(u32),
    InvalidScalingConversion {
        enchantment_id: u32,
        column: u8,
        product_bits: u32,
    },
}

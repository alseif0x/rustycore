use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaEnchantmentItemRef {
    pub guid: ObjectGuid,
    pub bag: u8,
    pub slot: u8,
    pub enchantment_id: i32,
    pub arena_allowed: bool,
}

impl ArenaEnchantmentItemRef {
    pub const fn new(
        guid: ObjectGuid,
        bag: u8,
        slot: u8,
        enchantment_id: i32,
        arena_allowed: bool,
    ) -> Self {
        Self {
            guid,
            bag,
            slot,
            enchantment_id,
            arena_allowed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveArenaEnchantmentAction {
    RemoveDurationReference {
        item_guid: ObjectGuid,
        enchantment_slot: EnchantmentSlot,
    },
    ClearEquippedEnchantment {
        item_guid: ObjectGuid,
        enchantment_slot: EnchantmentSlot,
    },
    ClearInventoryEnchantment {
        item_guid: ObjectGuid,
        bag: u8,
        slot: u8,
        enchantment_slot: EnchantmentSlot,
    },
    MissingInventoryItemRef {
        item_guid: ObjectGuid,
        bag: u8,
        slot: u8,
        enchantment_slot: EnchantmentSlot,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentTemplateRef {
    pub enchantment_id: i32,
    pub condition_id: u32,
    pub condition_fits: bool,
    pub min_level: u8,
    pub required_skill_id: u32,
    pub required_skill_rank: u16,
    pub required_skill_value: u16,
}

impl ApplyEnchantmentTemplateRef {
    pub const fn new(enchantment_id: i32) -> Self {
        Self {
            enchantment_id,
            condition_id: 0,
            condition_fits: true,
            min_level: 0,
            required_skill_id: 0,
            required_skill_rank: 0,
            required_skill_value: 0,
        }
    }

    pub const fn skill_fits(&self) -> bool {
        self.required_skill_id == 0 || self.required_skill_value >= self.required_skill_rank
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentGemRequirementRef {
    pub required_skill_id: u32,
    pub required_skill_rank: u16,
    pub required_skill_value: u16,
}

impl ApplyEnchantmentGemRequirementRef {
    pub const fn new(
        required_skill_id: u32,
        required_skill_rank: u16,
        required_skill_value: u16,
    ) -> Self {
        Self {
            required_skill_id,
            required_skill_rank,
            required_skill_value,
        }
    }

    pub const fn skill_fits(&self) -> bool {
        self.required_skill_id == 0 || self.required_skill_value >= self.required_skill_rank
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentSocketContext {
    pub socket_color: u32,
    pub prismatic_enchantment: Option<ApplyEnchantmentTemplateRef>,
    pub gem_requirement: Option<ApplyEnchantmentGemRequirementRef>,
}

impl ApplyEnchantmentSocketContext {
    pub const fn prismatic(
        prismatic_enchantment: Option<ApplyEnchantmentTemplateRef>,
        gem_requirement: Option<ApplyEnchantmentGemRequirementRef>,
    ) -> Self {
        Self {
            socket_color: 0,
            prismatic_enchantment,
            gem_requirement,
        }
    }

    pub const fn colored(
        socket_color: u32,
        gem_requirement: Option<ApplyEnchantmentGemRequirementRef>,
    ) -> Self {
        Self {
            socket_color,
            prismatic_enchantment: None,
            gem_requirement,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentArgs {
    pub apply: bool,
    pub apply_dur: bool,
    pub ignore_condition: bool,
    pub socket_context: Option<ApplyEnchantmentSocketContext>,
}

impl ApplyEnchantmentArgs {
    pub const fn apply() -> Self {
        Self {
            apply: true,
            apply_dur: true,
            ignore_condition: false,
            socket_context: None,
        }
    }

    pub const fn remove() -> Self {
        Self {
            apply: false,
            apply_dur: true,
            ignore_condition: false,
            socket_context: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentSkipReason {
    MissingItem,
    NotEquipped,
    NoEnchantment,
    MissingEnchantmentTemplate,
    ConditionFailed,
    PlayerLevelTooLow,
    RequiredSkillTooLow,
    MissingPrismaticEnchantment,
    PrismaticRequiredSkillTooLow,
    GemRequiredSkillTooLow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentDurationAction {
    Added(PlayerEnchantTimeUpdate),
    Removed {
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentResult {
    Skipped(ApplyEnchantmentSkipReason),
    Applied {
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        enchantment_id: i32,
        apply: bool,
        effects_allowed: bool,
        update_permanent_visible_item: bool,
        duration_action: Option<ApplyEnchantmentDurationAction>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentPlan {
    pub result: ApplyEnchantmentResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentEffectKind {
    Known(ItemEnchantmentType),
    Unknown(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentEffectRef {
    pub effect_kind: ApplyEnchantmentEffectKind,
    pub amount: u32,
    pub arg: u32,
}

impl ApplyEnchantmentEffectRef {
    pub const fn known(effect_type: ItemEnchantmentType, amount: u32, arg: u32) -> Self {
        Self {
            effect_kind: ApplyEnchantmentEffectKind::Known(effect_type),
            amount,
            arg,
        }
    }

    pub const fn unknown(effect_type: u32, amount: u32, arg: u32) -> Self {
        Self {
            effect_kind: ApplyEnchantmentEffectKind::Unknown(effect_type),
            amount,
            arg,
        }
    }
}

pub const APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentRandomSuffixRef {
    pub id: u32,
    pub enchantments: [u16; APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS],
    pub allocation_pct: [u16; APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS],
}

impl ApplyEnchantmentRandomSuffixRef {
    pub const fn new(
        id: u32,
        enchantments: [u16; APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS],
        allocation_pct: [u16; APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS],
    ) -> Self {
        Self {
            id,
            enchantments,
            allocation_pct,
        }
    }

    pub fn amount_for(&self, enchantment_id: i32, property_seed: i32) -> Option<u32> {
        self.enchantments
            .iter()
            .position(|enchantment| i32::from(*enchantment) == enchantment_id)
            .map(|index| {
                ((f64::from(self.allocation_pct[index]) * f64::from(property_seed)) / 10_000.0)
                    as u32
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentEffectAction {
    Noop,
    DeferredCombatSpell,
    DeferredUseSpell,
    UpdateDamageDoneMods {
        attack_type: WeaponAttackType,
        modifier_slot: i16,
    },
    CastEquipSpell {
        spell_id: u32,
        item_guid: ObjectGuid,
    },
    RemoveEquipSpellAura {
        spell_id: u32,
        item_guid: ObjectGuid,
    },
    UnitModifier {
        unit_mod: ApplyEnchantmentUnitMod,
        modifier: ApplyEnchantmentUnitModifier,
        amount: u32,
        apply: bool,
    },
    UpdateStatBuffMod(Stats),
    RatingModifier {
        rating: ApplyEnchantmentCombatRating,
        amount: u32,
        apply: bool,
    },
    ManaRegenBonus {
        amount: u32,
        apply: bool,
    },
    SpellPowerBonus {
        amount: u32,
        apply: bool,
    },
    HealthRegenBonus {
        amount: u32,
        apply: bool,
    },
    SpellPenetrationBonus {
        amount: u32,
        apply: bool,
    },
    BaseModFlatValue {
        base_mod: ApplyEnchantmentBaseMod,
        amount: u32,
        apply: bool,
    },
    SetShieldBlockValue {
        amount: u32,
    },
    SetBaseWeaponDamage {
        attack_type: WeaponAttackType,
        bound: WeaponDamageBoundLikeCpp,
        amount_bits: u32,
    },
    SetBaseAttackTime {
        attack_type: WeaponAttackType,
        time_ms: u32,
    },
    UpdateDamagePhysical {
        attack_type: WeaponAttackType,
    },
    UnhandledStatModifier {
        item_mod: ItemModType,
        amount: u32,
        apply: bool,
    },
    MissingItemTemplateForAttack {
        effect_kind: ApplyEnchantmentEffectKind,
    },
    Unknown {
        effect_type: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponDamageBoundLikeCpp {
    Min,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentUnitModifier {
    BaseValue,
    TotalValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentUnitMod {
    Mana,
    Health,
    Armor,
    StatAgility,
    StatStrength,
    StatIntellect,
    StatSpirit,
    StatStamina,
    AttackPower,
    AttackPowerRanged,
    Resistance(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentCombatRating {
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
    Expertise,
    ArmorPenetration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentBaseMod {
    ShieldBlockValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillEnchantmentTemplateRef {
    pub enchantment_id: i32,
    pub required_skill_id: u16,
    pub required_skill_rank: u16,
}

impl SkillEnchantmentTemplateRef {
    pub const fn new(
        enchantment_id: i32,
        required_skill_id: u16,
        required_skill_rank: u16,
    ) -> Self {
        Self {
            enchantment_id,
            required_skill_id,
            required_skill_rank,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillEnchantmentItemRef {
    pub item_guid: ObjectGuid,
    pub inventory_slot: u8,
    pub enchantment_ids: [i32; MAX_ENCHANTMENT_SLOT],
    pub socket_colors: [u32; 3],
}

impl SkillEnchantmentItemRef {
    pub const fn new(
        item_guid: ObjectGuid,
        inventory_slot: u8,
        enchantment_ids: [i32; MAX_ENCHANTMENT_SLOT],
        socket_colors: [u32; 3],
    ) -> Self {
        Self {
            item_guid,
            inventory_slot,
            enchantment_ids,
            socket_colors,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSkillEnchantmentReason {
    EnchantmentRequiredSkill,
    PrismaticRequiredSkill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSkillEnchantmentAction {
    Apply {
        item_guid: ObjectGuid,
        inventory_slot: u8,
        enchantment_slot: EnchantmentSlot,
        enchantment_id: i32,
        reason: UpdateSkillEnchantmentReason,
    },
    Remove {
        item_guid: ObjectGuid,
        inventory_slot: u8,
        enchantment_slot: EnchantmentSlot,
        enchantment_id: i32,
        reason: UpdateSkillEnchantmentReason,
    },
    MissingEnchantmentTemplateAbort {
        item_guid: ObjectGuid,
        inventory_slot: u8,
        enchantment_slot: EnchantmentSlot,
        enchantment_id: i32,
    },
}

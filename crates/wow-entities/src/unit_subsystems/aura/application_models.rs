use super::{Instant, ObjectGuid};

/// Represented C++ aura effect families consumed by Player/Unit rules that
/// have not yet been promoted to complete `AuraEffect` execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedAuraEffectLikeCpp {
    FeignDeath,
    FeatherFall,
    Hover,
    SafeFall,
    Fly,
    Ghost,
    Invisibility,
    Mounted,
    SwimSpeed,
    Speed,
    SpeedAlways,
    SpeedNotStack,
    UseNormalMovementSpeed,
    DecreaseSpeed,
    MinimumSpeed,
    MinimumSpeedRate,
    MountedSpeed,
    MountedSpeedAlways,
    MountedSpeedNotStack,
    FlightSpeed,
    VehicleFlightSpeed,
    MountedFlightSpeed,
    MountedFlightSpeedAlways,
    FlightSpeedNotStack,
    ModifyFallDamagePct,
    ModDetectRange,
    ModDetectedRange,
    ModFactionReputationGain,
    ModScale,
    ModSpeedNoControl,
    ModBattlePetXpPct,
    ModReputationGain,
    ModRestedXpConsumption,
    ModTotalStatPercentage,
    ProvideSpellFocus,
    Stealth,
    WaterWalk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedAuraEffectAmountLikeCpp {
    pub effect_index: u8,
    pub amount: i32,
}

/// Full represented application record owned by C++ `Unit`.
#[derive(Debug, Clone, PartialEq)]
pub struct AuraApplicationLikeCpp {
    pub spell_id: i32,
    pub difficulty_id: u8,
    pub caster_guid: ObjectGuid,
    pub slot: u8,
    pub duration_total: u32,
    pub duration_remaining: u32,
    pub stack_count: u8,
    pub aura_flags: u32,
    pub effect_mask: u32,
    pub aura_interrupt_flags: u32,
    pub aura_interrupt_flags2: u32,
    pub represented_effect: Option<RepresentedAuraEffectLikeCpp>,
    pub represented_amount: i32,
    pub represented_effect_amounts: Vec<RepresentedAuraEffectAmountLikeCpp>,
    pub represented_misc_value: Option<i32>,
    pub represented_multiplier: f32,
    pub applied_at: Instant,
}

/// Cast identity retained by a C++ `Aura` base for split-damage log entries.
///
/// This is deliberately separate from `AuraApplicationLikeCpp`: the
/// application owns the visible slot while the base owns cast provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuraCastProvenanceLikeCpp {
    pub cast_id: ObjectGuid,
    pub spell_visual_id: i32,
}

impl Default for AuraCastProvenanceLikeCpp {
    fn default() -> Self {
        Self {
            cast_id: ObjectGuid::EMPTY,
            spell_visual_id: 0,
        }
    }
}

/// Immutable, difficulty-selected C++ `AuraEffect` metadata retained beside
/// the owning Unit aura application rather than on its WorldSession adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuraThreatSnapshotLikeCpp {
    interrupt_flags: [u32; 2],
    effects: Vec<(u32, i32, i32, i32)>,
}

impl AuraThreatSnapshotLikeCpp {
    pub fn new(interrupt_flags: [u32; 2], effects: Vec<(u32, i32, i32, i32)>) -> Self {
        Self {
            interrupt_flags,
            effects,
        }
    }

    pub const fn interrupt_flags(&self) -> [u32; 2] {
        self.interrupt_flags
    }

    pub fn effects(&self) -> &[(u32, i32, i32, i32)] {
        &self.effects
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadedAuraStateLikeCpp {
    pub max_duration_ms: i32,
    pub duration_ms: i32,
    pub charges: u8,
    pub stack_amount: u8,
    pub recalculate_mask: u32,
}

impl LoadedAuraStateLikeCpp {
    pub const fn new(
        max_duration_ms: i32,
        duration_ms: i32,
        charges: u8,
        stack_amount: u8,
        recalculate_mask: u32,
    ) -> Self {
        Self {
            max_duration_ms,
            duration_ms,
            charges,
            stack_amount,
            recalculate_mask,
        }
    }
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

/// Bounded snapshot of the C++ `AuraApplication` fields needed by
/// `PartyMemberAuraStates`.
///
/// This is only representation data. It does not own aura lifetime, scripts,
/// effect recalculation, proc state, or packet fanout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VisibleAuraApplicationLikeCpp {
    pub flags: u32,
    pub effect_amounts: Vec<VisibleAuraEffectAmountLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleAuraEffectAmountLikeCpp {
    pub effect_index: u8,
    pub amount: i32,
}

impl VisibleAuraApplicationLikeCpp {
    pub fn new(flags: u32, effect_amounts: Vec<VisibleAuraEffectAmountLikeCpp>) -> Self {
        Self {
            flags,
            effect_amounts,
        }
    }
}

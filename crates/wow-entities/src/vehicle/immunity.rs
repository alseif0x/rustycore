//! Immunity packets.
//!
//! Separated from vehicle.rs under #693.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleSpellImmunityKind {
    Effect,
    State,
    Mechanic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleSpellImmunity {
    pub kind: VehicleSpellImmunityKind,
    pub spell_or_mechanic: i32,
    pub apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VehicleImmunityPlan {
    pub immunities: Vec<VehicleSpellImmunity>,
    pub root: bool,
}

pub const SPELL_EFFECT_HEAL_LIKE_CPP: i32 = 10;

pub const SPELL_EFFECT_DISPEL_LIKE_CPP: i32 = 38;

pub const SPELL_EFFECT_KNOCK_BACK_LIKE_CPP: i32 = 98;

pub const SPELL_EFFECT_HEAL_PCT_LIKE_CPP: i32 = 136;

pub const SPELL_EFFECT_KNOCK_BACK_DEST_LIKE_CPP: i32 = 144;

pub const SPELL_AURA_PERIODIC_HEAL_LIKE_CPP: i32 = 8;

pub const SPELL_AURA_DAMAGE_SHIELD_LIKE_CPP: i32 = 15;

pub const SPELL_AURA_MOD_RESISTANCE_LIKE_CPP: i32 = 22;

pub const SPELL_AURA_MOD_STAT_LIKE_CPP: i32 = 29;

pub const SPELL_AURA_MOD_DECREASE_SPEED_LIKE_CPP: i32 = 33;

pub const SPELL_AURA_SCHOOL_IMMUNITY_LIKE_CPP: i32 = 39;

pub const SPELL_AURA_SCHOOL_ABSORB_LIKE_CPP: i32 = 69;

pub const SPELL_AURA_SPLIT_DAMAGE_PCT_LIKE_CPP: i32 = 81;

pub const SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN_LIKE_CPP: i32 = 87;

pub const SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP: i32 = 93;

pub const MECHANIC_BANISH_LIKE_CPP: i32 = 18;

pub const MECHANIC_SHIELD_LIKE_CPP: i32 = 19;

pub const MECHANIC_IMMUNE_SHIELD_LIKE_CPP: i32 = 29;

pub fn vehicle_immunity_plan_like_cpp(
    vehicle_id: u32,
    is_mechanical_creature: bool,
    is_world_boss: bool,
) -> VehicleImmunityPlan {
    use VehicleSpellImmunityKind::{Effect, Mechanic, State};

    let mut plan = VehicleImmunityPlan::default();
    plan.immunities.extend([
        VehicleSpellImmunity {
            kind: Effect,
            spell_or_mechanic: SPELL_EFFECT_KNOCK_BACK_LIKE_CPP,
            apply: true,
        },
        VehicleSpellImmunity {
            kind: Effect,
            spell_or_mechanic: SPELL_EFFECT_KNOCK_BACK_DEST_LIKE_CPP,
            apply: true,
        },
    ]);

    if is_mechanical_creature && !is_world_boss {
        plan.immunities.extend([
            VehicleSpellImmunity {
                kind: Effect,
                spell_or_mechanic: SPELL_EFFECT_HEAL_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: Effect,
                spell_or_mechanic: SPELL_EFFECT_HEAL_PCT_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: Effect,
                spell_or_mechanic: SPELL_EFFECT_DISPEL_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_PERIODIC_HEAL_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_SCHOOL_IMMUNITY_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_MOD_UNATTACKABLE_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_SCHOOL_ABSORB_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: Mechanic,
                spell_or_mechanic: MECHANIC_BANISH_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: Mechanic,
                spell_or_mechanic: MECHANIC_SHIELD_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: Mechanic,
                spell_or_mechanic: MECHANIC_IMMUNE_SHIELD_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_DAMAGE_SHIELD_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_SPLIT_DAMAGE_PCT_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_MOD_RESISTANCE_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_MOD_STAT_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN_LIKE_CPP,
                apply: true,
            },
        ]);
    }

    match vehicle_id {
        160 | 244 | 510 | 452 | 543 => {
            plan.root = true;
            plan.immunities.push(VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_MOD_DECREASE_SPEED_LIKE_CPP,
                apply: true,
            });
        }
        335 | 336 | 338 => {
            plan.immunities.push(VehicleSpellImmunity {
                kind: State,
                spell_or_mechanic: SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN_LIKE_CPP,
                apply: false,
            });
        }
        _ => {}
    }

    plan
}

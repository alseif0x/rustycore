//! 02245dcd SpellInfo.cpp:2718-2800 / :2807-2989, in source phase order.
mod aura;
#[cfg(test)]
mod tests;
use super::{Definition, SpellDefinitionSeeds};

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpellSpecific {
    #[default]
    Normal = 0,
    Seal = 1,
    Aura = 3,
    Sting = 4,
    Curse = 5,
    Aspect = 6,
    Tracker = 7,
    WarlockArmor = 8,
    MageArmor = 9,
    ElementalShield = 10,
    MagePolymorph = 11,
    Food = 19,
    Drink = 20,
    FoodAndDrink = 21,
    Presence = 22,
    Charm = 23,
    Scroll = 24,
    MageArcaneBrilliance = 25,
    WarriorEnrage = 26,
    PriestDivineSpirit = 27,
    Hand = 28,
    Phase = 29,
    Bane = 30,
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpellAuraState {
    #[default]
    None = 0,
    Frozen = 4,
    Banished = 8,
    Dazed = 9,
    Victorious = 10,
    FaerieFire = 12,
    DruidPeriodicHeal = 15,
    RoguePoisoned = 16,
    Enraged = 17,
    Bleed = 18,
    RaidEncounter = 22,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpecificCounts {
    pub definitions: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellSpecificError {
    RequiresLearnSkills,
    AlreadyApplied,
    MissingPolymorphEffect,
    UndefinedMechanicShift,
}

impl SpellDefinitionSeeds {
    pub fn with_specific_and_aura_state(mut self) -> Result<Self, SpellSpecificError> {
        if self.specific.is_some() {
            return Err(SpellSpecificError::AlreadyApplied);
        }
        if self.learn_skills.is_none() {
            return Err(SpellSpecificError::RequiresLearnSkills);
        }
        let mut counts = SpecificCounts::default();
        let length = self
            .source_order
            .as_ref()
            .expect("learn-skills admission follows source order")
            .primary
            .len();
        for position in 0..length {
            let key = self.source_order.as_ref().expect("admitted order").primary[position];
            let first = self.first_spell_in_chain(key.0);
            let definition = self.definitions.get_mut(&key).expect("admitted definition");
            definition.specific = classify(definition, key.0, first)?;
            definition.aura_state = aura::classify(definition, key.0)?;
            counts.definitions += 1;
        }
        self.specific = Some(counts);
        Ok(self)
    }
    pub fn specific_counts(&self) -> Option<SpecificCounts> {
        self.specific
    }
}

fn classify(
    definition: &Definition,
    id: u32,
    first: u32,
) -> Result<SpellSpecific, SpellSpecificError> {
    use SpellSpecific::*;
    let flags = definition.fields.spell_family_flags;
    match definition.fields.spell_family_name {
        0 => {
            if definition.fields.aura_interrupt_flags[0] & 0x40000 != 0 {
                let food = definition
                    .effects
                    .iter()
                    .any(|effect| effect.is_aura() && matches!(effect.aura, 84 | 20));
                let drink = definition
                    .effects
                    .iter()
                    .any(|effect| effect.is_aura() && matches!(effect.aura, 85 | 21));
                match (food, drink) {
                    (true, true) => return Ok(FoodAndDrink),
                    (true, false) => return Ok(Food),
                    (false, true) => return Ok(Drink),
                    _ => {}
                }
            } else if matches!(first, 8118 | 8099 | 8112 | 8096 | 8115 | 8091) {
                return Ok(Scroll);
            }
        }
        3 => {
            if flags[0] & 0x12040000 != 0 {
                return Ok(MageArmor);
            }
            if flags[0] & 0x400 != 0 {
                return Ok(MageArcaneBrilliance);
            }
            if flags[0] & 0x1000000 != 0 {
                if definition
                    .effects
                    .first()
                    .ok_or(SpellSpecificError::MissingPolymorphEffect)?
                    .is_aura_kind(5)
                {
                    return Ok(MagePolymorph);
                }
            }
        }
        4 if id == 12292 => return Ok(WarriorEnrage),
        5 => {
            if matches!(id, 603 | 980 | 80240) {
                return Ok(Bane);
            }
            if definition.fields.dispel == 2 {
                return Ok(Curse);
            }
            if flags[1] & 0x20000020 != 0 || flags[2] & 0x10 != 0 {
                return Ok(WarlockArmor);
            }
        }
        6 if flags[0] & 0x20 != 0 => return Ok(PriestDivineSpirit),
        9 => {
            if definition.fields.dispel == 4 {
                return Ok(Sting);
            }
            if flags[0] & 0x200000 != 0 || flags[2] & 0x1010 != 0 {
                return Ok(Aspect);
            }
        }
        10 => {
            if flags[1] & 0xA2000800 != 0 {
                return Ok(Seal);
            }
            if flags[0] & 0x2190 != 0 {
                return Ok(Hand);
            }
            if matches!(id, 465 | 32223 | 183435 | 317920) {
                return Ok(Aura);
            }
        }
        11 if flags[1] & 0x420 != 0 || flags[0] & 0x400 != 0 || id == 23552 => {
            return Ok(ElementalShield);
        }
        15 if matches!(id, 48266 | 48263 | 48265) => return Ok(Presence),
        _ => {}
    }
    for effect in &definition.effects {
        if effect.effect != 6 {
            continue;
        }
        match effect.aura {
            6 | 378 | 2 | 177 => return Ok(Charm),
            44 if id == 30645 => return Ok(Normal),
            44 | 45 | 151 => return Ok(Tracker),
            _ => {}
        }
    }
    Ok(Normal)
}

//! 02245dcd SpellInfo.cpp:662-723 and GameTables.h:137-166,286-348.
//! Complete input rows, not CalcValue/SpellInfo/Player readiness.
use super::{BaseMpEntry, fingerprint, parse_float_rows, read_and_parse};
use anyhow::Result;
use std::path::Path;
#[cfg(test)]
mod tests;

const SCALING: &str = "SpellScaling.txt";
const RATINGS: &str = "CombatRatingsMultByILvl.txt";
const STAMINA: &str = "StaminaMultByILvl.txt";

/// Source's fifteen class columns plus all nine special scaling columns.
pub struct SpellScalingEntry {
    pub classes: BaseMpEntry,
    pub item: f32,
    pub consumable: f32,
    pub gems: [f32; 3],
    pub health: f32,
    pub damage_replace_stat: f32,
    pub damage_secondary: f32,
    pub mana_consumable: f32,
}
impl SpellScalingEntry {
    /// GetSpellScalingColumnForClass: negative selectors are not class IDs.
    /// Unknown selectors return genuine source zero only for an existing row.
    pub fn for_class(&self, class: i32) -> f32 {
        match class {
            -1 | -7 => self.item,
            -2 => self.consumable,
            -3 => self.gems[0],
            -4 => self.gems[1],
            -5 => self.gems[2],
            -6 => self.health,
            -8 => self.damage_replace_stat,
            -9 => self.damage_secondary,
            -10 => self.mana_consumable,
            _ => self.classes.for_class(class as u32),
        }
    }
    fn columns(&self) -> [f32; 24] {
        let c = self.classes;
        [
            c.rogue,
            c.druid,
            c.hunter,
            c.mage,
            c.paladin,
            c.priest,
            c.shaman,
            c.warlock,
            c.warrior,
            c.death_knight,
            c.monk,
            c.demon_hunter,
            c.evoker,
            c.adventurer,
            c.traveler,
            self.item,
            self.consumable,
            self.gems[0],
            self.gems[1],
            self.gems[2],
            self.health,
            self.damage_replace_stat,
            self.damage_secondary,
            self.mana_consumable,
        ]
    }
}

/// Both source multiplier structures have the same four columns.
pub struct ItemLevelMultipliers {
    pub armor: f32,
    pub weapon: f32,
    pub trinket: f32,
    pub jewelry: f32,
}
impl ItemLevelMultipliers {
    /// GameTables.cpp:157-188 and ItemTemplate.h:397-434. Ammo, thrown,
    /// relics and unknown enum bits use Armor, not the Weapon branch.
    pub fn for_inventory_type(&self, inventory_type: u8) -> f32 {
        match inventory_type {
            2 | 11 => self.jewelry,
            12 => self.trinket,
            13 | 14 | 15 | 17 | 21 | 22 | 23 | 26 => self.weapon,
            _ => self.armor,
        }
    }
    fn columns(&self) -> [f32; 4] {
        [self.armor, self.weapon, self.trinket, self.jewelry]
    }
}

/// One immutable three-table admission unit. Shares the established target
/// text parser, not a legacy projection or separately mutable numeric mirror.
pub struct SpellValueGameTables {
    scaling: Vec<SpellScalingEntry>,
    ratings: Vec<ItemLevelMultipliers>,
    stamina: Vec<ItemLevelMultipliers>,
}
impl SpellValueGameTables {
    pub fn load(directory: impl AsRef<Path>) -> Result<Self> {
        let root = directory.as_ref().join("gt");
        Ok(Self {
            scaling: read_and_parse(&root.join(SCALING), SCALING, scaling)?,
            ratings: read_and_parse(&root.join(RATINGS), RATINGS, multipliers)?,
            stamina: read_and_parse(&root.join(STAMINA), STAMINA, multipliers)?,
        })
    }
    /// SQL-free synthetic/source-asset parser, no filesystem snapshot claim.
    pub fn parse_strs(scaling_text: &str, ratings: &str, stamina: &str) -> Result<Self> {
        Ok(Self {
            scaling: scaling(scaling_text, SCALING)?,
            ratings: multipliers(ratings, RATINGS)?,
            stamina: multipliers(stamina, STAMINA)?,
        })
    }
    pub fn scaling(&self, level: u32) -> Option<&SpellScalingEntry> {
        self.scaling.get(level as usize)
    }
    pub fn ratings(&self, item_level: u32) -> Option<&ItemLevelMultipliers> {
        self.ratings.get(item_level as usize)
    }
    pub fn stamina(&self, item_level: u32) -> Option<&ItemLevelMultipliers> {
        self.stamina.get(item_level as usize)
    }
    /// Physical rows including unused row zero, not maximum playable level.
    pub fn counts(&self) -> [usize; 3] {
        [self.scaling.len(), self.ratings.len(), self.stamina.len()]
    }
    pub fn numeric_bit_fingerprints(&self) -> [u64; 3] {
        [
            fingerprint(self.scaling.iter().flat_map(SpellScalingEntry::columns)),
            fingerprint(self.ratings.iter().flat_map(ItemLevelMultipliers::columns)),
            fingerprint(self.stamina.iter().flat_map(ItemLevelMultipliers::columns)),
        ]
    }
}

fn scaling(content: &str, source: &str) -> Result<Vec<SpellScalingEntry>> {
    Ok(parse_float_rows(content, source, 24)?
        .into_iter()
        .map(|row| {
            let values: [f32; 24] = row.try_into().expect("checked scaling columns");
            SpellScalingEntry {
                classes: BaseMpEntry::from_columns(std::array::from_fn(|i| values[i])),
                item: values[15],
                consumable: values[16],
                gems: [values[17], values[18], values[19]],
                health: values[20],
                damage_replace_stat: values[21],
                damage_secondary: values[22],
                mana_consumable: values[23],
            }
        })
        .collect())
}
fn multipliers(content: &str, source: &str) -> Result<Vec<ItemLevelMultipliers>> {
    Ok(parse_float_rows(content, source, 4)?
        .into_iter()
        .map(|row| {
            let [armor, weapon, trinket, jewelry]: [f32; 4] =
                row.try_into().expect("checked multiplier columns");
            ItemLevelMultipliers {
                armor,
                weapon,
                trinket,
                jewelry,
            }
        })
        .collect())
}

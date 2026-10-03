//! ObjectMgr::LoadPlayerInfo / PlayerCreateInfoAddItemHelper at 02245dcd:
//! :3823-3848,3968-4096. Source list only, NOT Player inventory/equipment.
use super::{SourceError, WorldSources};
use wow_data::{
    forever_birth::{BirthCatalog, item_quantities::ItemQuantitySources, race_in_mask},
    forever_initialization::InitializationCatalog,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InitialItem {
    pub item: u32,
    pub amount: u32,
}

/// Transient ordered source result consumed by future inventory initialization.
/// No instance GUID, random bonus list, placement, use admission or save proof.
#[derive(Debug, PartialEq, Eq)]
pub struct InitialItems {
    pub context: u8,
    pub items: Vec<InitialItem>,
}

impl WorldSources {
    pub fn initial_items(
        &self,
        race: u8,
        class: u8,
        birth: &BirthCatalog,
        quantities: &ItemQuantitySources,
        initialization: &InitializationCatalog,
    ) -> Result<InitialItems, SourceError> {
        self.initial_items_with_identities(
            race,
            class,
            birth,
            quantities,
            (
                |race| initialization.race(u32::from(race)).is_some(),
                |class| initialization.class(u32::from(class)).is_some(),
            ),
        )
    }

    fn initial_items_with_identities(
        &self,
        race: u8,
        class: u8,
        birth: &BirthCatalog,
        quantities: &ItemQuantitySources,
        identities: (impl Fn(u8) -> bool, impl Fn(u8) -> bool),
    ) -> Result<InitialItems, SourceError> {
        let (race_exists, class_exists) = identities;
        if !race_exists(race) {
            return Err(SourceError::MissingRace);
        }
        if !class_exists(class) {
            return Err(SourceError::MissingClass);
        }
        if self.definition(race, class).is_none() {
            return Err(SourceError::MissingDefinition);
        }
        let mut result = InitialItems {
            context: 0,
            items: Vec::new(),
        };
        for loadout in birth.loadouts() {
            // Classes is uint8; preserve source enum conversion, not a signed
            // comparison or an unrelated legacy class restriction.
            if loadout.purpose != 9
                || loadout.class as u8 != class
                || !race_in_mask(loadout.race_mask, u32::from(race))
            {
                continue;
            }
            // Relations store uint16; a uint32 loadout ID above 65535 has no
            // matching key. Never truncate it and alias another loadout.
            let Ok(id) = u16::try_from(loadout.id) else {
                continue;
            };
            let mut present = false;
            for relation in birth.items_for_loadout(id) {
                if let Some(amount) = quantities.loadout_quantity(relation.item, class) {
                    present = true;
                    result.items.push(InitialItem {
                        item: relation.item,
                        amount,
                    });
                }
            }
            // Source changes itemContext only when a joined item vector
            // exists; a dangling loadout must not override the previous one.
            if present {
                result.context = loadout.item_context;
            }
        }
        // SQL order is retained. Positive values APPEND (no merge); every
        // negative value removes ALL equal item entries, including count<-1.
        // Source logs count<-1 but still removes; zero/invalid IDs are skipped.
        for row in &self.remaining.items {
            if (row.race != 0 && !race_exists(row.race))
                || (row.class != 0 && !class_exists(row.class))
                || !quantities.contains(row.item)
                || row.amount == 0
                || (row.race != 0 && row.race != race)
                || (row.class != 0 && row.class != class)
            {
                continue;
            }
            if row.amount > 0 {
                result.items.push(InitialItem {
                    item: row.item,
                    amount: row.amount as u32,
                });
            } else {
                result.items.retain(|item| item.item != row.item);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "items/tests.rs"]
mod tests;

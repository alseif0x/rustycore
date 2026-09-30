use std::collections::BTreeMap;

/// Static spell-to-pet aura configuration loaded from spell data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetAuraLikeCpp {
    pub auras_by_pet_entry: BTreeMap<u32, u32>,
    pub remove_on_change_pet: bool,
    pub damage: i32,
}

impl PetAuraLikeCpp {
    pub fn new(pet_entry: u32, aura_id: u32, remove_on_change_pet: bool, damage: i32) -> Self {
        let mut auras_by_pet_entry = BTreeMap::new();
        auras_by_pet_entry.insert(pet_entry, aura_id);
        Self {
            auras_by_pet_entry,
            remove_on_change_pet,
            damage,
        }
    }

    pub fn with_aura(mut self, pet_entry: u32, aura_id: u32) -> Self {
        self.auras_by_pet_entry.insert(pet_entry, aura_id);
        self
    }

    pub fn add_aura_like_cpp(&mut self, pet_entry: u32, aura_id: u32) {
        self.auras_by_pet_entry.insert(pet_entry, aura_id);
    }

    pub fn aura_for_pet_entry_like_cpp(&self, pet_entry: u32) -> u32 {
        self.auras_by_pet_entry
            .get(&pet_entry)
            .copied()
            .or_else(|| self.auras_by_pet_entry.get(&0).copied())
            .unwrap_or(0)
    }
}

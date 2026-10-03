//! Typed transient locale contributions, applied only to existing numeric rows.
pub struct SpellNameLocaleRecord {
    pub id: u32,
    pub name: Vec<u8>,
}

pub struct DifficultyLocaleRecord {
    pub id: u32,
    pub name: Vec<u8>,
}

pub struct SpellRangeLocaleRecord {
    pub id: u32,
    pub display_name: Vec<u8>,
    pub display_name_short: Vec<u8>,
}

pub struct SpellShapeshiftFormLocaleRecord {
    pub id: u32,
    pub name: Vec<u8>,
}

pub struct BattlePetSpeciesLocaleRecord {
    pub id: u32,
    pub description: Vec<u8>,
    pub source_text: Vec<u8>,
}

pub struct SpellCategoryLocaleRecord {
    pub id: u32,
    pub name: Vec<u8>,
}

pub struct TalentLocaleRecord {
    pub id: u32,
    pub description: Vec<u8>,
}

pub struct SpellItemEnchantmentLocaleRecord {
    pub id: u32,
    pub name: Vec<u8>,
    pub horde_name: Vec<u8>,
}

#[derive(Default)]
pub struct SpellLocaleRecords {
    pub talents: Vec<TalentLocaleRecord>,
    pub spell_item_enchantments: Vec<SpellItemEnchantmentLocaleRecord>,
    pub spell_names: Vec<SpellNameLocaleRecord>,
    pub difficulties: Vec<DifficultyLocaleRecord>,
    pub spell_ranges: Vec<SpellRangeLocaleRecord>,
    pub spell_shapeshift_forms: Vec<SpellShapeshiftFormLocaleRecord>,
    pub battle_pet_species: Vec<BattlePetSpeciesLocaleRecord>,
    pub spell_category_definitions: Vec<SpellCategoryLocaleRecord>,
}

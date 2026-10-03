//! Locale overlays do not create numeric rows; main-table text is enUS.
pub struct SpellNameLocaleRow {
    pub id: u32,
    pub name: Vec<u8>,
}

pub struct DifficultyLocaleRow {
    pub id: u32,
    pub name: Vec<u8>,
}

pub struct SpellRangeLocaleRow {
    pub id: u32,
    pub display_name: Vec<u8>,
    pub display_name_short: Vec<u8>,
}

pub struct SpellShapeshiftFormLocaleRow {
    pub id: u32,
    pub name: Vec<u8>,
}

pub struct BattlePetSpeciesLocaleRow {
    pub id: u32,
    pub description: Vec<u8>,
    pub source_text: Vec<u8>,
}

pub struct SpellCategoryLocaleRow {
    pub id: u32,
    pub name: Vec<u8>,
}

pub struct TalentLocaleRow {
    pub id: u32,
    pub description: Vec<u8>,
}

pub struct SpellItemEnchantmentLocaleRow {
    pub id: u32,
    pub name: Vec<u8>,
    pub horde_name: Vec<u8>,
}

#[derive(Default)]
pub struct SpellLocaleRows {
    pub talents: Vec<TalentLocaleRow>,
    pub spell_item_enchantments: Vec<SpellItemEnchantmentLocaleRow>,
    pub spell_names: Vec<SpellNameLocaleRow>,
    pub difficulties: Vec<DifficultyLocaleRow>,
    pub spell_ranges: Vec<SpellRangeLocaleRow>,
    pub spell_shapeshift_forms: Vec<SpellShapeshiftFormLocaleRow>,
    pub battle_pet_species: Vec<BattlePetSpeciesLocaleRow>,
    pub spell_category_definitions: Vec<SpellCategoryLocaleRow>,
}

pub struct SpellLocaleOverlays {
    pub official: SpellLocaleRows,
    pub custom: SpellLocaleRows,
}

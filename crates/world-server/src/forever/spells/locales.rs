//! Raw selected-locale bytes are moved, not normalized or used as fallback.
use wow_data::forever_spells::{
    BattlePetSpeciesLocaleRecord, DifficultyLocaleRecord, SpellCategoryLocaleRecord,
    SpellLocaleRecords, SpellNameLocaleRecord, SpellRangeLocaleRecord,
    SpellShapeshiftFormLocaleRecord,
};
use wow_data::forever_spells::{SpellItemEnchantmentLocaleRecord, TalentLocaleRecord};
use wow_persistence::forever::spells::SpellLocaleRows;

pub(super) fn records(rows: SpellLocaleRows) -> SpellLocaleRecords {
    SpellLocaleRecords {
        talents: rows
            .talents
            .into_iter()
            .map(|row| TalentLocaleRecord {
                id: row.id,
                description: row.description,
            })
            .collect(),
        spell_item_enchantments: rows
            .spell_item_enchantments
            .into_iter()
            .map(|row| SpellItemEnchantmentLocaleRecord {
                id: row.id,
                name: row.name,
                horde_name: row.horde_name,
            })
            .collect(),
        spell_names: rows
            .spell_names
            .into_iter()
            .map(|row| SpellNameLocaleRecord {
                id: row.id,
                name: row.name,
            })
            .collect(),
        difficulties: rows
            .difficulties
            .into_iter()
            .map(|row| DifficultyLocaleRecord {
                id: row.id,
                name: row.name,
            })
            .collect(),
        spell_ranges: rows
            .spell_ranges
            .into_iter()
            .map(|row| SpellRangeLocaleRecord {
                id: row.id,
                display_name: row.display_name,
                display_name_short: row.display_name_short,
            })
            .collect(),
        spell_shapeshift_forms: rows
            .spell_shapeshift_forms
            .into_iter()
            .map(|row| SpellShapeshiftFormLocaleRecord {
                id: row.id,
                name: row.name,
            })
            .collect(),
        battle_pet_species: rows
            .battle_pet_species
            .into_iter()
            .map(|row| BattlePetSpeciesLocaleRecord {
                id: row.id,
                description: row.description,
                source_text: row.source_text,
            })
            .collect(),
        spell_category_definitions: rows
            .spell_category_definitions
            .into_iter()
            .map(|row| SpellCategoryLocaleRecord {
                id: row.id,
                name: row.name,
            })
            .collect(),
    }
}

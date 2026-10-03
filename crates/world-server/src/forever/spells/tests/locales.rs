use wow_persistence::forever::spells::*;

#[test]
fn all_eight_locale_families_move_raw_bytes_without_numeric_creation() {
    let rows = SpellLocaleRows {
        spell_names: vec![SpellNameLocaleRow {
            id: u32::MAX,
            name: vec![0xFF, 0, b'a'],
        }],
        difficulties: vec![DifficultyLocaleRow {
            id: u32::MAX,
            name: vec![0xFF, 0, b'a'],
        }],
        spell_ranges: vec![SpellRangeLocaleRow {
            id: u32::MAX,
            display_name: vec![0xFF, 0, b'a'],
            display_name_short: vec![0xFF, 0, b'a'],
        }],
        spell_shapeshift_forms: vec![SpellShapeshiftFormLocaleRow {
            id: u32::MAX,
            name: vec![0xFF, 0, b'a'],
        }],
        battle_pet_species: vec![BattlePetSpeciesLocaleRow {
            id: u32::MAX,
            description: vec![0xFF, 0, b'a'],
            source_text: vec![0xFF, 0, b'a'],
        }],
        spell_category_definitions: vec![SpellCategoryLocaleRow {
            id: u32::MAX,
            name: vec![0xFF, 0, b'a'],
        }],
        talents: vec![TalentLocaleRow {
            id: u32::MAX,
            description: vec![0xFF, 0, b'a'],
        }],
        spell_item_enchantments: vec![SpellItemEnchantmentLocaleRow {
            id: u32::MAX,
            name: vec![0xFF, 0, b'a'],
            horde_name: vec![0xFE, 0, b'b'],
        }],
    };
    let result = super::super::locale_records(rows);
    assert_eq!(result.spell_names[0].id, u32::MAX);
    assert_eq!(result.spell_names[0].name, [0xFF, 0, b'a']);
    assert_eq!(result.difficulties[0].id, u32::MAX);
    assert_eq!(result.difficulties[0].name, [0xFF, 0, b'a']);
    assert_eq!(result.spell_ranges[0].id, u32::MAX);
    assert_eq!(result.spell_ranges[0].display_name, [0xFF, 0, b'a']);
    assert_eq!(result.spell_ranges[0].display_name_short, [0xFF, 0, b'a']);
    assert_eq!(result.spell_shapeshift_forms[0].id, u32::MAX);
    assert_eq!(result.spell_shapeshift_forms[0].name, [0xFF, 0, b'a']);
    assert_eq!(result.battle_pet_species[0].id, u32::MAX);
    assert_eq!(result.battle_pet_species[0].description, [0xFF, 0, b'a']);
    assert_eq!(result.battle_pet_species[0].source_text, [0xFF, 0, b'a']);
    assert_eq!(result.spell_category_definitions[0].id, u32::MAX);
    assert_eq!(result.spell_category_definitions[0].name, [0xFF, 0, b'a']);
    assert_eq!(result.talents[0].id, u32::MAX);
    assert_eq!(result.talents[0].description, [0xFF, 0, b'a']);
    assert_eq!(result.spell_item_enchantments[0].id, u32::MAX);
    assert_eq!(result.spell_item_enchantments[0].name, [0xFF, 0, b'a']);
    assert_eq!(
        result.spell_item_enchantments[0].horde_name,
        [0xFE, 0, b'b']
    );
}

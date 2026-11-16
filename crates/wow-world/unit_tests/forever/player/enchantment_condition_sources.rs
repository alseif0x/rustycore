use super::EnchantmentConditionSources;
use crate::forever::{
    creation::NumericItemTemplates,
    player::SkillEnchantmentSources,
    spells::{SpellDefinitionSeeds, SpellLoadPlan},
};
use std::sync::Arc;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::item_specs::ItemSpecCatalog,
    forever_spells::{SpellItemEnchantmentConditionRecord, SpellLocaleRecords, SpellRecords},
};
use wow_entities::{EnchantmentConditionFacts, EnchantmentConditionSource};
use wow_persistence::forever::spells::server::ServerSpellRows;

fn condition_row(id: u32) -> SpellItemEnchantmentConditionRecord {
    SpellItemEnchantmentConditionRecord {
        id,
        lt_operand_type: [1, 2, 3, 4, 0],
        lt_operand: [u32::MAX, 2, 3, 4, 5],
        operator: [2, 3, 5, 99, 0],
        rt_operand_type: [0, 1, 0, 255, 0],
        rt_operand: [u8::MAX, 2, 3, 4, 5],
        logic: [7, 6, 5, 4, 3],
    }
}

fn definitions(row: Option<SpellItemEnchantmentConditionRecord>) -> SpellDefinitionSeeds {
    let records = SpellRecords {
        spell_item_enchantment_conditions: row.into_iter().collect(),
        ..Default::default()
    }
    .finish(
        SpellRecords::default(),
        SpellRecords::default(),
        6,
        SpellLocaleRecords::default(),
        SpellLocaleRecords::default(),
        &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
    )
    .unwrap();
    SpellLoadPlan::build(Arc::new(records))
        .unwrap()
        .with_server_spells(ServerSpellRows::default())
        .unwrap()
}

fn source<'a>(
    definitions: &'a SpellDefinitionSeeds,
    templates: &'a NumericItemTemplates,
    specs: &'a ItemSpecCatalog,
) -> EnchantmentConditionSources<'a> {
    EnchantmentConditionSources::new(definitions, templates, specs)
}

#[test]
fn condition_projection_and_gem_lookup_borrow_the_admitted_native_owners() {
    let row = condition_row(u32::MAX);
    let definitions = definitions(Some(row));
    let (templates, specs) = NumericItemTemplates::gem_condition_source_test_catalogs(
        0x8000_1234,
        0xf123,
        Some(0x8000_000b),
    );
    let sources = source(&definitions, &templates, &specs);

    assert!(std::ptr::eq(sources.definitions, &definitions));
    assert!(std::ptr::eq(sources.item_templates, &templates));
    assert!(std::ptr::eq(sources.item_specs, &specs));
    let projected = EnchantmentConditionSource::condition(&sources, u32::MAX)
        .unwrap()
        .unwrap();
    assert_eq!(
        projected,
        EnchantmentConditionFacts::new(
            u32::MAX,
            [1, 2, 3, 4, 0],
            [2, 3, 5, 99, 0],
            [0, 1, 0, 255, 0],
            [255, 2, 3, 4, 5],
        )
    );
    assert_eq!(
        EnchantmentConditionSource::gem_color_mask(&sources, 0x8000_1234).unwrap(),
        Some(0x8000_000b)
    );
    assert_eq!(
        EnchantmentConditionSource::gem_color_mask(&sources, 0x1234).unwrap(),
        None
    );
    let raw_template = templates.template(0x8000_1234).unwrap();
    assert_eq!(raw_template.extended.gem_properties, 0xf123);
    assert_eq!(
        specs.gem(0xf123).map(|gem| gem.kind as u32),
        Some(0x8000_000b)
    );
    let canonical_catalog = definitions.raw_catalog();
    assert!(std::ptr::eq(
        definitions
            .spell_item_enchantment_condition_record(u32::MAX)
            .unwrap(),
        canonical_catalog
            .spell_item_enchantment_condition(u32::MAX)
            .unwrap()
    ));
}

#[test]
fn a_present_template_with_missing_gem_properties_is_absent_from_the_view() {
    let definitions = definitions(None);
    let (templates, specs) = NumericItemTemplates::gem_condition_source_test_catalogs(44, 0, None);
    let sources = source(&definitions, &templates, &specs);

    assert!(templates.template(44).is_some());
    assert_eq!(templates.template(44).unwrap().extended.gem_properties, 0);
    assert!(specs.gem(0).is_none());
    assert_eq!(
        EnchantmentConditionSource::gem_color_mask(&sources, 44).unwrap(),
        None
    );
    assert!(
        EnchantmentConditionSource::condition(&sources, 1)
            .unwrap()
            .is_none()
    );
}

#[test]
fn zero_item_and_gem_properties_ids_use_the_native_catalog_rows() {
    let definitions = definitions(None);
    let (templates, specs) =
        NumericItemTemplates::gem_condition_source_test_catalogs(0, 0, Some(0x8000_0009));
    let sources = source(&definitions, &templates, &specs);

    assert!(templates.template(0).is_some());
    assert!(specs.gem(0).is_some());
    assert_eq!(
        EnchantmentConditionSource::gem_color_mask(&sources, 0).unwrap(),
        Some(0x8000_0009)
    );
}

#[test]
fn gem_lookup_does_not_require_k10_value_game_tables() {
    let definitions = definitions(None);
    let (templates, specs) =
        NumericItemTemplates::gem_condition_source_test_catalogs(45, 7, Some(8));
    assert!(definitions.value_game_tables().is_none());
    let sources = source(&definitions, &templates, &specs);
    assert_eq!(
        EnchantmentConditionSource::gem_color_mask(&sources, 45).unwrap(),
        Some(8)
    );
    assert!(SkillEnchantmentSources::new(&definitions, &templates).is_err());
}

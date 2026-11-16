use super::*;
use crate::forever::spells::SpellLoadPlan;
use std::sync::Arc;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_game_tables::SpellValueGameTables,
    forever_spells::{
        SpellItemEnchantmentRecord, SpellLocaleRecords, SpellRecords,
        test_fixtures::spell_item_enchantment,
    },
};
use wow_entities::SkillEnchantmentSource;
use wow_persistence::forever::spells::server::ServerSpellRows;

fn game_tables() -> Arc<SpellValueGameTables> {
    let labels = (0..24)
        .map(|index| format!("C{index}"))
        .collect::<Vec<_>>()
        .join("\t");
    let values = (1..=24)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("\t");
    let scaling = format!("id\t{labels}\n1\t{values}\n");
    Arc::new(
        SpellValueGameTables::parse_strs(
            &scaling,
            "id\tA\tB\tC\tD\n1\t1\t2\t3\t4\n",
            "id\tA\tB\tC\tD\n1\t5\t6\t7\t8\n",
        )
        .unwrap(),
    )
}

fn definitions(
    enchantments: Vec<SpellItemEnchantmentRecord>,
    with_tables: bool,
) -> SpellDefinitionSeeds {
    let catalog = SpellRecords {
        spell_item_enchantments: enchantments,
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
    let seeds = SpellLoadPlan::build(Arc::new(catalog))
        .unwrap()
        .with_server_spells(ServerSpellRows::default())
        .unwrap();
    if with_tables {
        seeds.with_value_game_tables(game_tables()).unwrap()
    } else {
        seeds
    }
}

fn source<'a>(
    definitions: &'a SpellDefinitionSeeds,
    items: &'a NumericItemTemplates,
) -> SkillEnchantmentSources<'a> {
    SkillEnchantmentSources::new(definitions, items).unwrap()
}

#[test]
fn construction_requires_the_admitted_value_game_tables() {
    let definitions = definitions(Vec::new(), false);
    let items = NumericItemTemplates::enchantment_source_test_templates(9, 5, -19, 0x8001, 0xf00d);
    assert_eq!(
        SkillEnchantmentSources::new(&definitions, &items).err(),
        Some(SkillEnchantmentSourcesError::MissingValueGameTables)
    );
}

#[test]
fn enchantment_projection_preserves_all_consumed_columns_and_catalog_identity() {
    let mut row = spell_item_enchantment(0x8000_1234, 6, b"");
    row.effect = [i32::MIN, -1, 0x1234_5678];
    row.effect_points_min = [i32::MIN, -123_456, i32::MAX];
    row.effect_arg = [0, 0x8000_0001, u32::MAX];
    row.flags = i32::MIN;
    row.effect_scaling_points = [
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0x8000_0000),
        17.25,
    ];
    row.scaling_class = i32::MIN;
    row.scaling_class_restricted = -17;
    row.condition_id = i32::MIN;
    row.required_skill_id = -1;
    row.required_skill_rank = i32::MIN;
    row.min_level = -2;
    row.max_level = i32::MAX;

    let definitions = definitions(vec![row], true);
    let items = NumericItemTemplates::enchantment_source_test_templates(9, 5, -19, 0x8001, 0xf00d);
    let sources = source(&definitions, &items);
    assert!(std::ptr::eq(sources.definitions, &definitions));
    assert!(std::ptr::eq(
        sources.value_game_tables,
        definitions.value_game_tables().unwrap()
    ));
    let projected = SkillEnchantmentSource::enchantment(&sources, 0x8000_1234)
        .unwrap()
        .unwrap();

    assert_eq!(projected.id(), 0x8000_1234);
    assert_eq!(
        projected.effects(),
        &[i32::MIN as u32, u32::MAX, 0x1234_5678]
    );
    assert_eq!(projected.effect_amounts(), &[i32::MIN, -123_456, i32::MAX]);
    assert_eq!(projected.effect_args(), &[0, 0x8000_0001, u32::MAX]);
    assert_eq!(projected.flags(), i32::MIN);
    assert_eq!(
        projected.scaling_points().map(f32::to_bits),
        [0x7fc0_1234, 0x8000_0000, 17.25f32.to_bits()]
    );
    assert_eq!(projected.scaling_class(), i32::MIN);
    assert_eq!(projected.scaling_class_restricted(), -17);
    assert_eq!(projected.condition_id(), i32::MIN as u32);
    assert_eq!(projected.required_skill_id(), u32::MAX);
    assert_eq!(projected.required_skill_rank(), i32::MIN as u32);
    assert_eq!(projected.min_level(), (-2i32) as u32);
    assert_eq!(projected.max_level(), i32::MAX as u32);

    let through_seed_accessor = definitions
        .spell_item_enchantment_record(0x8000_1234)
        .unwrap();
    let canonical_catalog = definitions.raw_catalog();
    let through_raw_catalog = canonical_catalog
        .spell_item_enchantment(0x8000_1234)
        .unwrap();
    assert!(std::ptr::eq(through_seed_accessor, through_raw_catalog));
}

#[test]
fn template_projection_uses_extended_inventory_enum_and_widens_required_skill() {
    let definitions = definitions(Vec::new(), true);
    let items = NumericItemTemplates::enchantment_source_test_templates(9, 5, -19, 0x8001, 0xf00d);
    let raw = items.template(9).unwrap();
    assert_eq!(raw.basic.inventory_type, 5);
    assert_eq!(raw.extended.inventory_type, -19);
    assert_eq!(raw.extended.required_skill, 0x8001);
    assert_eq!(raw.extended.required_skill_rank, 0xf00d);

    let sources = source(&definitions, &items);
    let projected = SkillEnchantmentSource::item_template(&sources, 9)
        .unwrap()
        .unwrap();
    assert_eq!(projected.inventory_type(), 237);
    assert_eq!(projected.required_skill_id(), 0x8001);
    assert_eq!(projected.required_skill_rank(), 0xf00d);
}

#[test]
fn missing_rows_are_absent_from_the_currently_admitted_sources() {
    let definitions = definitions(vec![spell_item_enchantment(7, 6, b"")], true);
    let items = NumericItemTemplates::enchantment_source_test_templates(9, 5, 17, 300, 450);
    let sources = source(&definitions, &items);

    assert!(
        SkillEnchantmentSource::enchantment(&sources, 8)
            .unwrap()
            .is_none()
    );
    assert!(
        SkillEnchantmentSource::item_template(&sources, 10)
            .unwrap()
            .is_none()
    );
    assert!(
        SkillEnchantmentSource::spell_scaling_column(&sources, 2, 1)
            .unwrap()
            .is_none()
    );
}

#[test]
fn scaling_reads_physical_rows_and_preserves_target_class_selectors() {
    let definitions = definitions(Vec::new(), true);
    let items = NumericItemTemplates::enchantment_source_test_templates(9, 5, 17, 0, 0);
    let sources = source(&definitions, &items);

    assert_eq!(
        SkillEnchantmentSource::spell_scaling_row_count(&sources).unwrap(),
        2
    );
    assert_eq!(
        SkillEnchantmentSource::spell_scaling_column(&sources, 0, 1).unwrap(),
        Some(0.0)
    );
    assert_eq!(
        SkillEnchantmentSource::spell_scaling_column(&sources, 1, 1).unwrap(),
        Some(9.0)
    );
    assert_eq!(
        SkillEnchantmentSource::spell_scaling_column(&sources, 1, -1).unwrap(),
        Some(16.0)
    );
    assert_eq!(
        SkillEnchantmentSource::spell_scaling_column(&sources, 1, -3).unwrap(),
        Some(18.0)
    );
    assert_eq!(
        SkillEnchantmentSource::spell_scaling_column(&sources, 1, i32::MAX).unwrap(),
        Some(0.0)
    );
}

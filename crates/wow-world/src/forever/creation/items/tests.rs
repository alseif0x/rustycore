use super::*;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{
        BirthRecords, LoadoutItemRecord, LoadoutRecord,
        item_quantities::{ItemClassQuantityRecord, ItemSparseQuantityRecord},
    },
};
use wow_persistence::forever::creation::{
    ClassLevelStats, CreationWorldRows, ItemOverride, RaceStats, StartDefinition,
};

fn sources(race: u8, class: u8, overrides: Vec<ItemOverride>) -> WorldSources {
    let tables = wow_data::forever_game_tables::InitialGameTables::parse_strs(
        "L\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\n",
        "L\tHealth\n",
        "L\tTotal\tKill\tJunk\tStats\tDivisor\n1\t400\t0\t0\t0\t0\n2\t500\t0\t0\t0\t0\n",
    )
    .unwrap();
    let rows = CreationWorldRows {
        definitions: vec![StartDefinition {
            race,
            class,
            map: 0,
            position: [0.; 4],
            npe_map: None,
            npe_position: [None; 4],
            npe_transport: None,
            intro_movie: None,
            intro_scene: None,
            npe_intro_scene: None,
        }],
        race_stats: vec![RaceStats {
            race,
            modifiers: [0; 5],
        }],
        class_stats: vec![ClassLevelStats {
            class,
            level: 1,
            stats: [1; 5],
        }],
        item_overrides: overrides,
        ..Default::default()
    };
    WorldSources::from_rows(rows, |_| true, |_| true, &tables, 2).unwrap()
}
fn quantities() -> ItemQuantitySources {
    ItemQuantitySources::from_effective_records(
        (1..=3)
            .map(|id| ItemClassQuantityRecord {
                id,
                class: 2,
                subclass: 0,
            })
            .collect(),
        (1..=3)
            .map(|id| ItemSparseQuantityRecord {
                id,
                vendor_stack: id + 1,
                stackable: 1,
            })
            .collect(),
        vec![],
        vec![],
    )
    .unwrap()
}
fn birth(loadouts: Vec<LoadoutRecord>, loadout_items: Vec<LoadoutItemRecord>) -> BirthCatalog {
    BirthRecords {
        loadouts,
        loadout_items,
        ..Default::default()
    }
    .finish(
        BirthRecords::default(),
        BirthRecords::default(),
        &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
    )
    .unwrap()
}
fn loadout(id: u32, context: u8) -> LoadoutRecord {
    LoadoutRecord {
        id,
        class: 1,
        purpose: 9,
        item_context: context,
        field_1_60_1_69876_003: 0,
        race_mask: 1,
    }
}
fn link(id: u32, loadout: u16, item: u32) -> LoadoutItemRecord {
    LoadoutItemRecord { id, loadout, item }
}
fn override_row(race: u8, class: u8, item: u32, amount: i8) -> ItemOverride {
    ItemOverride {
        race,
        class,
        item,
        amount,
    }
}
fn plan(
    sources: &WorldSources,
    race: u8,
    class: u8,
    birth: &BirthCatalog,
) -> Result<InitialItems, SourceError> {
    sources.initial_items_with_identities(
        race,
        class,
        birth,
        &quantities(),
        (|id| matches!(id, 1 | 95 | 96), |id| matches!(id, 1 | 6)),
    )
}

#[test]
fn final_loadout_and_relation_order_preserve_duplicate_items_and_last_joined_context() {
    let catalog = birth(
        vec![loadout(2, 12), loadout(1, 11)],
        vec![link(4, 2, 1), link(2, 1, 1), link(1, 1, 2), link(3, 2, 3)],
    );
    let result = plan(&sources(1, 1, vec![]), 1, 1, &catalog).unwrap();
    assert_eq!(result.context, 12);
    assert_eq!(
        result.items,
        vec![
            InitialItem { item: 2, amount: 3 },
            InitialItem { item: 1, amount: 2 },
            InitialItem { item: 3, amount: 4 },
            InitialItem { item: 1, amount: 2 }
        ]
    );
}

#[test]
fn absent_item_joins_cannot_set_context_and_wide_loadout_ids_cannot_alias() {
    let catalog = birth(
        vec![loadout(1, 11), loadout(2, 12), loadout(65537, 13)],
        vec![link(1, 1, 1), link(2, 2, 99)],
    );
    let result = plan(&sources(1, 1, vec![]), 1, 1, &catalog).unwrap();
    assert_eq!(result.context, 11);
    assert_eq!(result.items, vec![InitialItem { item: 1, amount: 2 }]);
    let absent = birth(vec![loadout(1, 12)], vec![link(1, 1, 99)]);
    assert_eq!(
        plan(&sources(1, 1, vec![]), 1, 1, &absent).unwrap(),
        InitialItems {
            context: 0,
            items: vec![]
        }
    );
}

#[test]
fn only_purpose_nine_matching_class_and_nonempty_matching_mask_apply() {
    let mut wrong_purpose = loadout(1, 11);
    wrong_purpose.purpose = 8;
    let mut wrong_class = loadout(2, 12);
    wrong_class.class = 6;
    let mut empty_mask = loadout(3, 13);
    empty_mask.race_mask = 0;
    let mut wrong_race = loadout(4, 14);
    wrong_race.race_mask = 2;
    let catalog = birth(
        vec![wrong_purpose, wrong_class, empty_mask, wrong_race],
        (1..=4).map(|id| link(id, id as u16, 1)).collect(),
    );
    assert_eq!(
        plan(&sources(1, 1, vec![]), 1, 1, &catalog).unwrap(),
        InitialItems {
            context: 0,
            items: vec![]
        }
    );
}

#[test]
fn skyborne_masks_use_bits_32_and_33_not_race_ids_minus_one() {
    let mut alliance = loadout(1, 11);
    alliance.race_mask = 1 << 32;
    let mut horde = loadout(2, 12);
    horde.race_mask = 1 << 33;
    let catalog = birth(vec![alliance, horde], vec![link(1, 1, 1), link(2, 2, 2)]);
    let result = plan(&sources(95, 1, vec![]), 95, 1, &catalog).unwrap();
    assert_eq!(result.context, 11);
    assert_eq!(result.items, vec![InitialItem { item: 1, amount: 2 }]);
    let result = plan(&sources(96, 1, vec![]), 96, 1, &catalog).unwrap();
    assert_eq!(result.context, 12);
    assert_eq!(result.items, vec![InitialItem { item: 2, amount: 3 }]);
}

#[test]
fn signed_overrides_append_without_stack_clamp_then_remove_all_and_allow_readdition() {
    let catalog = birth(
        vec![loadout(1, 11)],
        vec![link(1, 1, 1), link(2, 1, 1), link(3, 1, 2)],
    );
    let sources = sources(
        1,
        1,
        vec![
            override_row(0, 0, 1, 127),
            override_row(1, 0, 1, -128),
            override_row(0, 1, 2, -1),
            override_row(1, 1, 1, 5),
            override_row(0, 0, 1, 6),
        ],
    );
    let result = plan(&sources, 1, 1, &catalog).unwrap();
    assert_eq!(result.context, 11);
    assert_eq!(
        result.items,
        vec![
            InitialItem { item: 1, amount: 5 },
            InitialItem { item: 1, amount: 6 }
        ]
    );
}

#[test]
fn invalid_zero_missing_and_other_pair_overrides_are_skipped_not_errors_or_wildcards() {
    let catalog = birth(vec![], vec![]);
    let sources = sources(
        1,
        1,
        vec![
            override_row(2, 0, 1, 5),
            override_row(0, 2, 1, 5),
            override_row(95, 0, 1, 5),
            override_row(0, 6, 1, 5),
            override_row(0, 0, 99, 5),
            override_row(0, 0, 1, 0),
            override_row(0, 0, 1, -2),
            override_row(0, 0, 3, 127),
        ],
    );
    assert_eq!(
        plan(&sources, 1, 1, &catalog).unwrap(),
        InitialItems {
            context: 0,
            items: vec![InitialItem {
                item: 3,
                amount: 127
            }]
        }
    );
}

#[test]
fn a_source_definition_and_both_effective_identities_are_required() {
    let catalog = birth(vec![], vec![]);
    let sources = sources(1, 1, vec![]);
    assert_eq!(
        plan(&sources, 2, 1, &catalog),
        Err(SourceError::MissingRace)
    );
    assert_eq!(
        plan(&sources, 1, 2, &catalog),
        Err(SourceError::MissingClass)
    );
    assert_eq!(
        plan(&sources, 95, 1, &catalog),
        Err(SourceError::MissingDefinition)
    );
}

#[test]
fn final_hotfix_removal_of_loadout_relation_cannot_reappear_in_plan() {
    let rows = BirthRecords {
        loadouts: vec![loadout(1, 11)],
        loadout_items: vec![link(1, 1, 1)],
        ..Default::default()
    };
    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(
        wow_data::forever_birth::LOADOUT_ITEM_HASH,
        1,
        2,
    )]);
    let catalog = rows
        .finish(BirthRecords::default(), BirthRecords::default(), &removals)
        .unwrap();
    assert_eq!(
        plan(&sources(1, 1, vec![]), 1, 1, &catalog).unwrap(),
        InitialItems {
            context: 0,
            items: vec![]
        }
    );
}

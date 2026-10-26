use super::visual_metadata_loading::{FixtureDirectory, unresolved_error};
use crate::{
    Db2HotfixRemovalStoreLikeCpp, SpellAuraOptionsEntry, SpellAuraOptionsStore,
    SpellClassOptionsEntry, SpellClassOptionsStore, SpellLabelEntry, SpellLabelStore,
    SpellLevelsEntry, SpellLevelsStore,
};

const AURA_TABLE_HASH: u32 = 0xA0A0_0101;
const CLASS_TABLE_HASH: u32 = 0xA0A0_0202;
const LABEL_TABLE_HASH: u32 = 0xA0A0_0303;
const LEVELS_TABLE_HASH: u32 = 0xA0A0_0404;

fn aura_entry(
    id: u32,
    cumulative_aura: u32,
    proc_category_recovery: i32,
    proc_charges: i32,
    proc_type_mask: [i32; 2],
    spell_id: u32,
) -> SpellAuraOptionsEntry {
    SpellAuraOptionsEntry {
        id,
        difficulty_id: 3,
        cumulative_aura,
        proc_category_recovery,
        proc_chance: 77,
        proc_charges,
        spell_procs_per_minute_id: 19,
        proc_type_mask,
        spell_id,
    }
}

fn empty_aura_file_entry(id: u32) -> SpellAuraOptionsEntry {
    SpellAuraOptionsEntry {
        id,
        difficulty_id: 0,
        cumulative_aura: 0,
        proc_category_recovery: 0,
        proc_chance: 0,
        proc_charges: 0,
        spell_procs_per_minute_id: 0,
        proc_type_mask: [0; 2],
        spell_id: 0,
    }
}

fn class_options_entry(
    id: u32,
    spell_id: i32,
    modal_next_spell: u32,
    spell_class_mask: [u32; 4],
) -> SpellClassOptionsEntry {
    SpellClassOptionsEntry {
        id,
        spell_id,
        modal_next_spell,
        spell_class_set: 9,
        spell_class_mask,
    }
}

fn empty_class_options_file_entry(id: u32) -> SpellClassOptionsEntry {
    SpellClassOptionsEntry {
        id,
        spell_id: 0,
        modal_next_spell: 0,
        spell_class_set: 0,
        spell_class_mask: [0; 4],
    }
}

fn label_entry(id: u32, label_id: u32, spell_id: u32) -> SpellLabelEntry {
    SpellLabelEntry {
        id,
        label_id,
        spell_id,
    }
}

fn levels_entry(
    id: u32,
    base_level: i16,
    max_level: i16,
    spell_level: i16,
    spell_id: u32,
) -> SpellLevelsEntry {
    SpellLevelsEntry {
        id,
        difficulty_id: 5,
        base_level,
        max_level,
        spell_level,
        max_passive_aura_level: u8::MAX,
        spell_id,
    }
}

fn empty_levels_file_entry(id: u32) -> SpellLevelsEntry {
    SpellLevelsEntry {
        id,
        difficulty_id: 0,
        base_level: 0,
        max_level: 0,
        spell_level: 0,
        max_passive_aura_level: 0,
        spell_id: 0,
    }
}

fn scoped_removals(table_hash: u32) -> Db2HotfixRemovalStoreLikeCpp {
    Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (table_hash, 300, 2),
        (table_hash.wrapping_add(1), 100, 2),
        (table_hash, 999, 2),
    ])
}

#[test]
fn effective_scalar_loaders_replace_in_order_and_apply_scoped_removals() {
    let fixtures = FixtureDirectory::new();
    fixtures.write_complete_records(
        "SpellAuraOptions.db2",
        AURA_TABLE_HASH,
        100,
        2,
        &[8, 32, 32, 8, 32, 16, 64],
    );
    fixtures.write_complete_records(
        "SpellClassOptions.db2",
        CLASS_TABLE_HASH,
        100,
        2,
        &[32, 32, 8, 128],
    );
    fixtures.write_complete_records("SpellLabel.db2", LABEL_TABLE_HASH, 100, 2, &[32]);
    fixtures.write_complete_records(
        "SpellLevels.db2",
        LEVELS_TABLE_HASH,
        100,
        2,
        &[8, 16, 16, 16, 8],
    );
    let data_dir = fixtures.data_dir();

    let aura_file = SpellAuraOptionsStore::load_complete(&data_dir, "enUS")
        .expect("load the complete aura file row");
    assert_eq!(aura_file.len(), 2);
    assert_eq!(aura_file.get(100), Some(&empty_aura_file_entry(100)));
    let aura = SpellAuraOptionsStore::load_effective_from_hotfix_rows(
        &data_dir,
        "enUS",
        [
            aura_entry(100, 11, 12, 13, [14, 15], 16),
            aura_entry(101, 21, 22, 23, [24, 25], 26),
            aura_entry(200, 41, 42, 43, [44, 45], 46),
            aura_entry(300, 31, 32, 33, [34, 35], 36),
            aura_entry(100, u32::MAX, i32::MIN, i32::MIN, [i32::MIN, -1], u32::MAX),
        ],
        &scoped_removals(AURA_TABLE_HASH),
    )
    .expect("load aura rows over the complete file");
    assert_eq!(aura.len(), 3);
    assert_eq!(
        aura.get(100),
        Some(&aura_entry(
            100,
            u32::MAX,
            i32::MIN,
            i32::MIN,
            [i32::MIN, -1],
            u32::MAX,
        ))
    );
    assert_eq!(
        aura.get(101),
        Some(&aura_entry(101, 21, 22, 23, [24, 25], 26))
    );
    assert_eq!(
        aura.get(200),
        Some(&aura_entry(200, 41, 42, 43, [44, 45], 46))
    );
    assert!(aura.get(300).is_none());

    let class_file = SpellClassOptionsStore::load_complete(&data_dir, "enUS")
        .expect("load the complete class-options file row");
    assert_eq!(class_file.len(), 2);
    assert_eq!(
        class_file.get(100),
        Some(&empty_class_options_file_entry(100))
    );
    let class_options = SpellClassOptionsStore::load_effective_from_hotfix_rows(
        &data_dir,
        "enUS",
        [
            class_options_entry(100, 11, 12, [13; 4]),
            class_options_entry(101, 21, 22, [23; 4]),
            class_options_entry(200, 41, 42, [43; 4]),
            class_options_entry(300, 31, 32, [33; 4]),
            class_options_entry(
                100,
                i32::MIN,
                u32::MAX,
                [0x8000_0000, u32::MAX, 0xAABB_CCDD, 0x0102_0304],
            ),
        ],
        &scoped_removals(CLASS_TABLE_HASH),
    )
    .expect("load class-options rows over the complete file");
    assert_eq!(class_options.len(), 3);
    assert_eq!(
        class_options.get(100),
        Some(&class_options_entry(
            100,
            i32::MIN,
            u32::MAX,
            [0x8000_0000, u32::MAX, 0xAABB_CCDD, 0x0102_0304],
        ))
    );
    assert_eq!(
        class_options.get(101),
        Some(&class_options_entry(101, 21, 22, [23; 4]))
    );
    assert_eq!(
        class_options.get(200),
        Some(&class_options_entry(200, 41, 42, [43; 4]))
    );
    assert!(class_options.get(300).is_none());

    let label_file = SpellLabelStore::load_complete(&data_dir, "enUS")
        .expect("load the complete label file row");
    assert_eq!(label_file.len(), 2);
    assert_eq!(label_file.get(100), Some(&label_entry(100, 0, 0)));
    let labels = SpellLabelStore::load_effective_from_hotfix_rows(
        &data_dir,
        "enUS",
        [
            label_entry(100, 11, 12),
            label_entry(101, 21, 22),
            label_entry(200, 41, 42),
            label_entry(300, 31, 32),
            label_entry(100, u32::MAX, u32::MAX),
        ],
        &scoped_removals(LABEL_TABLE_HASH),
    )
    .expect("load label rows over the complete file");
    assert_eq!(labels.len(), 3);
    assert_eq!(labels.get(100), Some(&label_entry(100, u32::MAX, u32::MAX)));
    assert_eq!(labels.get(101), Some(&label_entry(101, 21, 22)));
    assert_eq!(labels.get(200), Some(&label_entry(200, 41, 42)));
    assert!(labels.get(300).is_none());

    let levels_file = SpellLevelsStore::load_complete(&data_dir, "enUS")
        .expect("load the complete levels file row");
    assert_eq!(levels_file.len(), 2);
    assert_eq!(levels_file.get(100), Some(&empty_levels_file_entry(100)));
    let levels = SpellLevelsStore::load_effective_from_hotfix_rows(
        &data_dir,
        "enUS",
        [
            levels_entry(100, 11, 12, 13, 14),
            levels_entry(101, 21, 22, 23, 24),
            levels_entry(200, 41, 42, 43, 44),
            levels_entry(300, 31, 32, 33, 34),
            levels_entry(100, i16::MIN, i16::MAX, -12_345, u32::MAX),
        ],
        &scoped_removals(LEVELS_TABLE_HASH),
    )
    .expect("load levels rows over the complete file");
    assert_eq!(levels.len(), 3);
    assert_eq!(
        levels.get(100),
        Some(&levels_entry(100, i16::MIN, i16::MAX, -12_345, u32::MAX))
    );
    assert_eq!(levels.get(101), Some(&levels_entry(101, 21, 22, 23, 24)));
    assert_eq!(levels.get(200), Some(&levels_entry(200, 41, 42, 43, 44)));
    assert!(levels.get(300).is_none());
}

#[test]
fn empty_complete_scalar_files_accept_new_rows() {
    let fixtures = FixtureDirectory::new();
    fixtures.write_complete_empty("SpellAuraOptions.db2", AURA_TABLE_HASH);
    fixtures.write_complete_empty("SpellClassOptions.db2", CLASS_TABLE_HASH);
    fixtures.write_complete_empty("SpellLabel.db2", LABEL_TABLE_HASH);
    fixtures.write_complete_empty("SpellLevels.db2", LEVELS_TABLE_HASH);
    let data_dir = fixtures.data_dir();
    let removals = Db2HotfixRemovalStoreLikeCpp::default();

    assert!(
        SpellAuraOptionsStore::load_complete(&data_dir, "enUS")
            .expect("load empty complete aura file")
            .is_empty()
    );
    assert_eq!(
        SpellAuraOptionsStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [aura_entry(200, 1, 2, 3, [4, 5], 6)],
            &removals,
        )
        .expect("overlay aura row onto an empty file")
        .get(200),
        Some(&aura_entry(200, 1, 2, 3, [4, 5], 6))
    );

    assert!(
        SpellClassOptionsStore::load_complete(&data_dir, "enUS")
            .expect("load empty complete class-options file")
            .is_empty()
    );
    assert_eq!(
        SpellClassOptionsStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [class_options_entry(200, -1, 2, [3, 4, 5, 6])],
            &removals,
        )
        .expect("overlay class-options row onto an empty file")
        .get(200),
        Some(&class_options_entry(200, -1, 2, [3, 4, 5, 6]))
    );

    assert!(
        SpellLabelStore::load_complete(&data_dir, "enUS")
            .expect("load empty complete label file")
            .is_empty()
    );
    assert_eq!(
        SpellLabelStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [label_entry(200, 3, 4)],
            &removals,
        )
        .expect("overlay label row onto an empty file")
        .get(200),
        Some(&label_entry(200, 3, 4))
    );

    assert!(
        SpellLevelsStore::load_complete(&data_dir, "enUS")
            .expect("load empty complete levels file")
            .is_empty()
    );
    assert_eq!(
        SpellLevelsStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [levels_entry(200, -1, 2, 3, 4)],
            &removals,
        )
        .expect("overlay levels row onto an empty file")
        .get(200),
        Some(&levels_entry(200, -1, 2, 3, 4))
    );
}

#[test]
fn scalar_complete_and_effective_loaders_reject_missing_files() {
    let fixtures = FixtureDirectory::new();
    let data_dir = fixtures.data_dir();
    let removals = Db2HotfixRemovalStoreLikeCpp::default();

    assert!(SpellAuraOptionsStore::load_complete(&data_dir, "enUS").is_err());
    assert!(
        SpellAuraOptionsStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [aura_entry(1, 2, 3, 4, [5, 6], 7)],
            &removals,
        )
        .is_err()
    );
    assert!(SpellClassOptionsStore::load_complete(&data_dir, "enUS").is_err());
    assert!(
        SpellClassOptionsStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [class_options_entry(1, 2, 3, [4; 4])],
            &removals,
        )
        .is_err()
    );
    assert!(SpellLabelStore::load_complete(&data_dir, "enUS").is_err());
    assert!(
        SpellLabelStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [label_entry(1, 2, 3)],
            &removals,
        )
        .is_err()
    );
    assert!(SpellLevelsStore::load_complete(&data_dir, "enUS").is_err());
    assert!(
        SpellLevelsStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [levels_entry(1, 2, 3, 4, 5)],
            &removals,
        )
        .is_err()
    );
}

#[test]
fn scalar_complete_and_effective_loaders_reject_decode_failures() {
    let fixtures = FixtureDirectory::new();
    for file_name in [
        "SpellAuraOptions.db2",
        "SpellClassOptions.db2",
        "SpellLabel.db2",
        "SpellLevels.db2",
    ] {
        fixtures.write_invalid_file(file_name);
    }
    let data_dir = fixtures.data_dir();
    let removals = Db2HotfixRemovalStoreLikeCpp::default();

    assert!(SpellAuraOptionsStore::load_complete(&data_dir, "enUS").is_err());
    assert!(
        SpellAuraOptionsStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [aura_entry(1, 2, 3, 4, [5, 6], 7)],
            &removals,
        )
        .is_err()
    );
    assert!(SpellClassOptionsStore::load_complete(&data_dir, "enUS").is_err());
    assert!(
        SpellClassOptionsStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [class_options_entry(1, 2, 3, [4; 4])],
            &removals,
        )
        .is_err()
    );
    assert!(SpellLabelStore::load_complete(&data_dir, "enUS").is_err());
    assert!(
        SpellLabelStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [label_entry(1, 2, 3)],
            &removals,
        )
        .is_err()
    );
    assert!(SpellLevelsStore::load_complete(&data_dir, "enUS").is_err());
    assert!(
        SpellLevelsStore::load_effective_from_hotfix_rows(
            &data_dir,
            "enUS",
            [levels_entry(1, 2, 3, 4, 5)],
            &removals,
        )
        .is_err()
    );
}

#[test]
fn scalar_complete_loaders_reject_unresolved_copies_before_overlays() {
    let fixtures = FixtureDirectory::new();
    for file_name in [
        "SpellAuraOptions.db2",
        "SpellClassOptions.db2",
        "SpellLabel.db2",
        "SpellLevels.db2",
    ] {
        fixtures.write_incomplete_copy(file_name);
    }
    let data_dir = fixtures.data_dir();
    let removals = Db2HotfixRemovalStoreLikeCpp::default();

    assert!(
        SpellAuraOptionsStore::load(&data_dir, "enUS")
            .expect("legacy aura loader keeps incomplete-copy behavior")
            .is_empty()
    );
    unresolved_error(SpellAuraOptionsStore::load_complete(&data_dir, "enUS"));
    unresolved_error(SpellAuraOptionsStore::load_effective_from_hotfix_rows(
        &data_dir,
        "enUS",
        [aura_entry(1, 2, 3, 4, [5, 6], 7)],
        &removals,
    ));

    assert!(
        SpellClassOptionsStore::load(&data_dir, "enUS")
            .expect("legacy class-options loader keeps incomplete-copy behavior")
            .is_empty()
    );
    unresolved_error(SpellClassOptionsStore::load_complete(&data_dir, "enUS"));
    unresolved_error(SpellClassOptionsStore::load_effective_from_hotfix_rows(
        &data_dir,
        "enUS",
        [class_options_entry(1, 2, 3, [4; 4])],
        &removals,
    ));

    assert!(
        SpellLabelStore::load(&data_dir, "enUS")
            .expect("legacy label loader keeps incomplete-copy behavior")
            .is_empty()
    );
    unresolved_error(SpellLabelStore::load_complete(&data_dir, "enUS"));
    unresolved_error(SpellLabelStore::load_effective_from_hotfix_rows(
        &data_dir,
        "enUS",
        [label_entry(1, 2, 3)],
        &removals,
    ));

    assert!(
        SpellLevelsStore::load(&data_dir, "enUS")
            .expect("legacy levels loader keeps incomplete-copy behavior")
            .is_empty()
    );
    unresolved_error(SpellLevelsStore::load_complete(&data_dir, "enUS"));
    unresolved_error(SpellLevelsStore::load_effective_from_hotfix_rows(
        &data_dir,
        "enUS",
        [levels_entry(1, 2, 3, 4, 5)],
        &removals,
    ));
}

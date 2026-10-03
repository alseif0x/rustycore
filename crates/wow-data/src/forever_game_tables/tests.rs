use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

const BASE_HEADER: &str = "Level\tRogue\tDruid\tHunter\tMage\tPaladin\tPriest\tShaman\tWarlock\tWarrior\tDeath Knight\tMonk\tDemon Hunter\tEvoker\tAdventurer\tTraveler";
const HP_HEADER: &str = "Level\tHealth";
const XP_HEADER: &str = "Level\tTotal\tPerKill\tJunk\tStats\tDivisor";

fn base_file(rows: &[&str]) -> String {
    format!("{}\r\n{}\r\n", BASE_HEADER, rows.join("\r\n"))
}

fn hp_file(rows: &[&str]) -> String {
    format!("{}\r\n{}\r\n", HP_HEADER, rows.join("\r\n"))
}

fn xp_file(rows: &[&str]) -> String {
    format!("{}\r\n{}\r\n", XP_HEADER, rows.join("\r\n"))
}

#[test]
fn loads_all_tables_with_physical_rows_and_target_class_mapping() {
    let base_values = (1..=15)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("\t");
    let tables = InitialGameTables::parse_strs(
        &base_file(&[
            &format!("9001\t{base_values}"),
            "9002\t101\t102\t103\t104\t105\t106\t107\t108\t109\t110\t111\t112\t113\t114\t115",
        ]),
        &hp_file(&["7001\t9.5"]),
        &xp_file(&["42\t100\t20\t3\t4\t5"]),
    )
    .expect("synthetic tables should parse");

    assert_eq!(tables.base_mp(0).map(|row| row.warrior), Some(0.0));
    assert_eq!(tables.base_mp(1).map(|row| row.warrior), Some(9.0));
    assert_eq!(tables.base_mp(2).map(|row| row.warrior), Some(109.0));
    assert_eq!(tables.base_mana(1, 1), Some(9.0));
    let expected_by_class = [
        9.0, 5.0, 3.0, 1.0, 6.0, 10.0, 7.0, 4.0, 8.0, 11.0, 2.0, 12.0, 13.0, 14.0, 15.0,
    ];
    for (class_id, expected) in (1..=15).zip(expected_by_class) {
        assert_eq!(tables.base_mana(1, class_id), Some(expected));
    }
    assert_eq!(tables.base_mana(1, 99), Some(0.0));
    assert_eq!(tables.counts(), [3, 2, 2]);
    assert_eq!(tables.hp_per_sta(1).map(|row| row.health), Some(9.5));
    assert_eq!(tables.xp(1).map(|row| row.total), Some(100.0));
    assert_eq!(tables.xp(1).map(|row| row.divisor), Some(5.0));
}

#[test]
fn source_row_zero_is_unused_and_ids_do_not_select_rows() {
    let tables = InitialGameTables::parse_strs(
        &base_file(&["not-an-id\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15"]),
        &hp_file(&["not-an-id\t2"]),
        &xp_file(&["not-an-id\t3\t4\t5\t6\t7"]),
    )
    .expect("first column is ignored by the source loader");

    assert_eq!(tables.base_mp(0).map(|row| row.rogue), Some(0.0));
    assert_eq!(tables.base_mp(1).map(|row| row.rogue), Some(1.0));
    assert!(tables.base_mp(2).is_none());
    assert_eq!(tables.hp_per_sta(1).map(|row| row.health), Some(2.0));
    assert_eq!(tables.xp(1).map(|row| row.total), Some(3.0));
}

#[test]
fn trailing_empty_tokens_are_ignored_but_interior_empty_cells_fail_closed() {
    let base = format!(
        "{BASE_HEADER}\r\n1\t{}\t\r\n",
        (1..=15)
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\t")
    );
    let hp = format!("{HP_HEADER}\r\n1\t2\t\r\n");
    let xp = format!("{XP_HEADER}\r\n1\t3\t4\t5\t6\t7\t\r\n");
    assert!(InitialGameTables::parse_strs(&base, &hp, &xp).is_ok());

    let base_values = (1..=15).map(|value| value.to_string()).collect::<Vec<_>>();
    let mut interior = base_values;
    interior[4].clear();
    let bad_base = base_file(&[&format!("1\t{}", interior.join("\t"))]);
    assert!(
        InitialGameTables::parse_strs(
            &bad_base,
            &hp_file(&["1\t2"]),
            &xp_file(&["1\t3\t4\t5\t6\t7"])
        )
        .is_err()
    );
}

#[test]
fn malformed_header_or_row_width_fails_each_table_and_all_tables_is_atomic() {
    assert!(parse_base_mp("Level\tRogue\n1\t1\n", BASE_MP_FILE).is_err());
    assert!(parse_hp_per_sta("Level\tHealth\n1\t1\t2\n", HP_PER_STA_FILE).is_err());
    assert!(
        parse_xp(
            "Level\tTotal\tPerKill\tJunk\tStats\tDivisor\n1\t1\t2\n",
            XP_FILE
        )
        .is_err()
    );

    let valid_base = base_file(&["1\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15"]);
    let error = InitialGameTables::parse_strs(&valid_base, "", &xp_file(&["1\t1\t2\t3\t4\t5"]))
        .err()
        .expect("a missing table must prevent snapshot creation");
    assert!(error.to_string().contains("empty file"));
}

#[test]
fn header_only_and_blank_rows_keep_explicit_zero_row() {
    let header_only = InitialGameTables::parse_strs(BASE_HEADER, HP_HEADER, XP_HEADER)
        .expect("header-only tables keep source row zero");
    assert_eq!(header_only.counts(), [1, 1, 1]);

    let base = format!(
        "{BASE_HEADER}\r\n\r\n1\t{}\r\n",
        (1..=15)
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\t")
    );
    let hp = format!("{HP_HEADER}\r\n\r\n1\t2\r\n");
    let xp = format!("{XP_HEADER}\r\n\r\n1\t3\t4\t5\t6\t7\r\n");
    let tables =
        InitialGameTables::parse_strs(&base, &hp, &xp).expect("blank row stops source loading");

    assert_eq!(tables.counts(), [1, 1, 1]);
    assert_eq!(tables.base_mp(0).map(|row| row.warrior), Some(0.0));
    assert!(tables.base_mp(1).is_none());
}

#[test]
fn filesystem_loader_requires_all_three_without_leaking_root_path() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "rustycore-forever-game-tables-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("gt")).expect("create synthetic game-table directory");
    fs::write(
        root.join("gt").join(BASE_MP_FILE),
        base_file(&["1\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15"]),
    )
    .expect("write synthetic BaseMp");
    fs::write(
        root.join("gt").join(XP_FILE),
        xp_file(&["1\t1\t2\t3\t4\t5"]),
    )
    .expect("write synthetic xp");

    let error = InitialGameTables::load(&root)
        .err()
        .expect("missing HpPerSta must reject the snapshot");
    let message = error.to_string();
    let root_string = root.to_string_lossy().into_owned();
    assert!(message.contains(HP_PER_STA_FILE));
    assert!(!message.contains(&root_string));
    fs::remove_dir_all(root).expect("remove synthetic game-table directory");
}

#[test]
fn numeric_admission_is_decimal_finite_and_fully_consumed() {
    assert_eq!(parse_decimal("  +1.25"), Some(1.25));
    assert_eq!(parse_decimal("-0.5e1"), Some(-5.0));
    assert_eq!(parse_decimal("0"), Some(0.0));
    assert_eq!(parse_decimal("0e400"), Some(0.0));
    assert!(parse_decimal("1 ").is_none());
    assert!(parse_decimal("0x1p2").is_none());
    assert!(parse_decimal("NaN").is_none());
    assert!(parse_decimal("Infinity").is_none());
    assert!(parse_decimal("1e400").is_none());
    assert!(parse_decimal("1e-400").is_none());
    assert!(parse_decimal("1e-50").is_none());
    assert!(parse_decimal("1.2oops").is_none());
    assert!(parse_decimal("+-1").is_none());
    assert!(parse_decimal("++1").is_none());
}

#[test]
fn header_retains_cr_but_data_is_truncated_at_first_cr_like_source() {
    // Header Tokenize does not call RemoveCRLF: CR after a trailing tab is
    // a nonempty extra token and source structure-size admission fails.
    assert!(parse_hp_per_sta("Level\tHealth\t\r\n1\t2\r\n", HP_PER_STA_FILE).is_err());
    let rows = parse_hp_per_sta(
        "Level\tHealth\r\nignored\t2\rnot-a-cell\r\n",
        HP_PER_STA_FILE,
    )
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].health, 2.);
}

#[test]
fn numeric_fingerprints_cover_column_order_row_zero_and_signed_zero() {
    let tables = InitialGameTables::parse_strs(
        &base_file(&["ignored\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15"]),
        &hp_file(&["ignored\t-0"]),
        &xp_file(&["ignored\t100\t20\t3\t4\t5"]),
    )
    .unwrap();
    // Independent FNV-1a LE-f32 fixtures, not values produced by the method
    // under test. Reordering class columns or losing -0 changes these.
    assert_eq!(
        tables.numeric_bit_fingerprints(),
        [
            9_000_784_081_756_505_640,
            12_161_821_475_553_763_397,
            8_930_079_713_595_422_996
        ]
    );
}

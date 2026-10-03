use super::*;

fn text(columns: usize, label: &str, values: impl IntoIterator<Item = f32>) -> String {
    format!(
        "id\t{}\r\n{label}\t{}\t\r\n",
        (0..columns)
            .map(|i| format!("C{i}"))
            .collect::<Vec<_>>()
            .join("\t"),
        values
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\t")
    )
}
fn source() -> [String; 3] {
    [
        text(24, "12345", (1..=24).map(|v| v as f32)),
        text(4, "-9", [1.0, 2.0, 3.0, 4.0]),
        text(4, "ignored", [-0.0, 1.5, -2.0, 3.0]),
    ]
}
fn tables() -> SpellValueGameTables {
    let s = source();
    SpellValueGameTables::parse_strs(&s[0], &s[1], &s[2]).unwrap()
}

#[test]
fn scaling_retains_all_class_and_special_columns_with_native_selectors() {
    let t = tables();
    let row = t.scaling(1).unwrap();
    for (class, expected) in [
        9.0, 5.0, 3.0, 1.0, 6.0, 10.0, 7.0, 4.0, 8.0, 11.0, 2.0, 12.0, 13.0, 14.0, 15.0,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(row.for_class(class as i32 + 1), expected);
    }
    for (class, expected) in [
        (-1, 16.0),
        (-7, 16.0),
        (-2, 17.0),
        (-3, 18.0),
        (-4, 19.0),
        (-5, 20.0),
        (-6, 21.0),
        (-8, 22.0),
        (-9, 23.0),
        (-10, 24.0),
    ] {
        assert_eq!(row.for_class(class), expected);
    }
    for class in [0, 16, -11, i32::MIN, i32::MAX] {
        assert_eq!(row.for_class(class), 0.0);
    }
}

#[test]
fn all_inventory_enum_bits_select_source_multiplier_not_an_equipment_guess() {
    let t = tables();
    for inventory_type in 0..=u8::MAX {
        let column = match inventory_type {
            2 | 11 => 3,
            12 => 2,
            13 | 14 | 15 | 17 | 21 | 22 | 23 | 26 => 1,
            _ => 0,
        };
        assert_eq!(
            t.ratings(1).unwrap().for_inventory_type(inventory_type),
            [1.0, 2.0, 3.0, 4.0][column]
        );
        assert_eq!(
            t.stamina(1)
                .unwrap()
                .for_inventory_type(inventory_type)
                .to_bits(),
            [-0.0f32, 1.5, -2.0, 3.0][column].to_bits()
        );
    }
}

#[test]
fn physical_rows_zero_and_missing_rows_remain_distinct_from_labels_and_selector_zero() {
    let t = tables();
    assert_eq!(t.counts(), [2, 2, 2]);
    assert_eq!(t.scaling(0).unwrap().for_class(-10).to_bits(), 0);
    assert_eq!(t.ratings(0).unwrap().for_inventory_type(13).to_bits(), 0);
    assert_eq!(t.stamina(0).unwrap().for_inventory_type(12).to_bits(), 0);
    for row in [2, 12345, u32::MAX] {
        assert!(t.scaling(row).is_none());
        assert!(t.ratings(row).is_none());
        assert!(t.stamina(row).is_none());
    }
}

#[test]
fn full_numeric_fingerprints_include_all_cells_unused_zero_and_signed_zero() {
    assert_eq!(
        tables().numeric_bit_fingerprints(),
        [
            3895469710102651531,
            3051449540718509784,
            7524576399414174216
        ]
    );
}

#[test]
fn every_table_rejects_wrong_columns_and_unsupported_cells_instead_of_substituting_zero() {
    for index in 0..3 {
        for invalid in ["", "id\tvalue\n1\t0\n", "id\n", "bad\0header"] {
            let mut s = source();
            s[index] = invalid.to_owned();
            assert!(SpellValueGameTables::parse_strs(&s[0], &s[1], &s[2]).is_err());
        }
        for invalid in ["NaN", "inf", "0x1p2", "1e1000", "1e-1000", "1suffix", ""] {
            let mut s = source();
            s[index] = s[index].replacen("12345\t1", &format!("12345\t{invalid}"), 1);
            if index != 0 {
                let label = if index == 1 { "-9" } else { "ignored" };
                s[index] = format!("id\tA\tB\tC\tD\n{label}\t{invalid}\t2\t3\t4\n");
            }
            assert!(SpellValueGameTables::parse_strs(&s[0], &s[1], &s[2]).is_err());
        }
    }
}

#[test]
fn first_blank_row_stops_source_admission_without_renumbering_or_numeric_read_past_it() {
    let mut s = source();
    for text in &mut s {
        text.push_str("\r\nnot-a-number\tbad\n");
    }
    let t = SpellValueGameTables::parse_strs(&s[0], &s[1], &s[2]).unwrap();
    assert_eq!(t.counts(), [2, 2, 2]);
}

#[test]
fn filesystem_batch_requires_all_three_files_without_returning_a_partial_catalog() {
    let root = std::env::temp_dir().join(format!("forever-spell-value-gt-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let gt = root.join("gt");
    std::fs::create_dir(&gt).unwrap();
    let source = source();
    for (index, name) in [SCALING, RATINGS, STAMINA].into_iter().enumerate() {
        std::fs::write(gt.join(name), &source[index]).unwrap();
    }
    assert_eq!(
        SpellValueGameTables::load(&root).unwrap().counts(),
        [2, 2, 2]
    );
    for (index, name) in [SCALING, RATINGS, STAMINA].into_iter().enumerate() {
        std::fs::remove_file(gt.join(name)).unwrap();
        assert!(SpellValueGameTables::load(&root).is_err());
        std::fs::write(gt.join(name), &source[index]).unwrap();
    }
    for name in [SCALING, RATINGS, STAMINA] {
        std::fs::remove_file(gt.join(name)).unwrap();
    }
    std::fs::remove_dir(gt).unwrap();
    std::fs::remove_dir(root).unwrap();
}

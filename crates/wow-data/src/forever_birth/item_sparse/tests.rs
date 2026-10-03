use super::*;
use prefix::CatalogLayout;

fn value(field: usize, array: usize) -> u32 {
    match decode::width(field) {
        4 => 0x8000_0000 | ((field as u32) << 8) | array as u32,
        2 => 0x8000 | ((field as u32) << 4) | array as u32,
        _ => 0x80 | (field + array) as u32,
    }
}
fn blob(strings: &[&[u8]; 5], changed: Option<(usize, usize, u32)>) -> Vec<u8> {
    let mut bytes = Vec::new();
    for string in strings {
        bytes.extend_from_slice(string);
        bytes.push(0);
    }
    for field in 5..68 {
        for array in 0..decode::count(field) {
            let v = changed
                .filter(|&(f, a, _)| f == field && a == array)
                .map(|(_, _, v)| v)
                .unwrap_or_else(|| value(field, array));
            bytes.extend_from_slice(&v.to_le_bytes()[..decode::width(field)]);
        }
    }
    bytes
}
fn record_blob() -> Vec<u8> {
    blob(&[b"a", b"", b"synthetic", b"\xff", b"label"], None)
}
fn put(bytes: &mut [u8], at: usize, v: u32) {
    bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

#[test]
fn variable_inline_strings_and_catalog_padding_do_not_change_numeric_offsets() {
    let first = decode::record(1, &record_blob()).unwrap();
    assert_eq!(first.strings.field(6, 0), b"a");
    assert_eq!(first.strings.field(6, 3), b"\xff");
    assert!(first.strings.field(0, 4).is_empty()); // no locale fallback
    let mut alternative = blob(
        &[b"different length", b"byte string", b"", b"\xfe\xfd", b""],
        None,
    );
    alternative.extend_from_slice(&[33, 44, 55]); // source ignores catalog padding
    let second = decode::record(2, &alternative).unwrap();
    assert_eq!(
        first.quantity_projection().stackable,
        second.quantity_projection().stackable
    );
    assert_eq!(first.vendor_stack, second.vendor_stack);
    assert_eq!(first.ammunition, second.ammunition);
    assert_eq!(second.id, 2);
}

#[test]
fn main_and_locale_sql_preserve_baseline_slots_and_source_new_id_batch_semantics() {
    use crate::Db2HotfixRemovalStoreLikeCpp;
    use crate::forever_birth::item_records::{ITEM_SPARSE_HASH, ItemRecords};
    let sql = |id, fields: [&[u8]; 5]| {
        let mut row = decode::record(id, &record_blob()).unwrap();
        row.strings = SparseItemStrings::for_locale(0, fields.map(<[u8]>::to_vec));
        row
    };
    let baseline = ItemRecords {
        sparse: vec![decode::record(1, &record_blob()).unwrap()],
        ..Default::default()
    };
    let official = ItemRecords {
        sparse: vec![
            sql(1, [b"English", b"", b"", b"", b"name"]),
            sql(2, [b"first-new", b"", b"", b"", b""]),
            sql(2, [b"", b"second-new", b"", b"", b""]),
        ],
        ..Default::default()
    };
    let custom = ItemRecords {
        sparse: vec![
            sql(1, [b"", b"custom", b"", b"", b""]),
            sql(3, [b"removed", b"", b"", b"", b""]),
        ],
        ..Default::default()
    };
    let catalog = baseline
        .finish(
            official,
            custom,
            &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(ITEM_SPARSE_HASH, 3, 2)]),
        )
        .unwrap()
        .with_sparse_locale(
            6,
            [(1, [b"oficial".to_vec(), vec![], vec![], vec![], vec![]])],
            [
                (1, [vec![], vec![], vec![], b"\0suffix".to_vec(), vec![]]),
                (99, std::array::from_fn(|_| b"not-created".to_vec())),
            ],
        );
    let r = catalog.sparse(1).unwrap();
    assert_eq!(r.strings.field(0, 0), b"English");
    assert_eq!(r.strings.field(0, 1), b"custom");
    assert_eq!(r.strings.field(0, 4), b"name");
    assert_eq!(r.strings.field(6, 0), b"oficial");
    assert_eq!(r.strings.field(6, 2), b"synthetic");
    assert_eq!(r.strings.field(6, 3), b"\0suffix");
    let r = catalog.sparse(2).unwrap();
    assert!(r.strings.field(0, 0).is_empty()); // new ID does not accumulate first row
    assert_eq!(r.strings.field(0, 1), b"second-new");
    assert!(r.strings.field(6, 1).is_empty());
    assert!(catalog.sparse(3).is_none() && catalog.sparse(99).is_none());
}

#[test]
fn five_flags_both_mask_words_and_all_numeric_arrays_keep_their_target_shapes() {
    let row = decode::record(1, &record_blob()).unwrap();
    assert_eq!(row.flags, std::array::from_fn(|i| value(27, i) as i32));
    assert_eq!(row.faction_related, value(28, 0) as i32);
    assert_eq!(row.squish_era, value(34, 0) as i32);
    assert_eq!(
        row.socket_percentage.map(f32::to_bits),
        std::array::from_fn(|i| value(14, i))
    );
    assert_eq!(
        row.stat_percent,
        std::array::from_fn(|i| value(15, i) as i32)
    );
    assert_eq!(row.stat_bonus, std::array::from_fn(|i| value(16, i) as i32));
    assert_eq!(
        row.allowable_race,
        u64::from(value(21, 0)) | (u64::from(value(21, 1)) << 32)
    );
    assert_eq!(row.zone_bound, [value(42, 0) as u16, value(42, 1) as u16]);
    assert_eq!(row.socket_type, std::array::from_fn(|i| value(55, i) as u8));
}

#[test]
fn signed_unsigned_and_float_values_preserve_source_bits_without_finite_value_policy() {
    let row = decode::record(u32::MAX, &record_blob()).unwrap();
    assert_eq!(row.stackable, value(17, 0) as i32);
    assert_eq!(row.vendor_stack, value(24, 0));
    assert_eq!(row.required_ability, value(20, 0));
    assert_eq!(row.allowable_class, value(51, 0) as i16);
    assert_eq!(row.name_description, value(35, 0) as u16);
    assert_eq!(row.artifact, value(52, 0) as u8);
    assert_eq!(row.required_level, value(64, 0) as i8);
    assert_eq!(row.quality, value(66, 0) as i8);
    assert_eq!(row.damage_variance.to_bits(), value(6, 0));
    for bits in [0x8000_0000, f32::INFINITY.to_bits(), 0x7FC0_1234, 1] {
        let row = decode::record(1, &blob(&[b""; 5], Some((6, 0, bits)))).unwrap();
        assert_eq!(row.damage_variance.to_bits(), bits);
    }
}

#[test]
fn unterminated_strings_truncated_cells_and_oversize_rows_fail_without_cross_record_reads() {
    assert!(decode::record(1, &[b'a'; 400]).is_err());
    let mut bytes = record_blob();
    bytes.pop();
    assert!(decode::record(1, &bytes).is_err());
    assert!(decode::record(1, &vec![0; u16::MAX as usize + 1]).is_err());
}

fn catalog_fixture(ids: &[u32], copies: &[(u32, u32)]) -> (Vec<u8>, CatalogLayout) {
    let raw = record_blob();
    let at = 4096;
    let copy_at = at + ids.len() * 4;
    let entries_at = copy_at + copies.len() * 8;
    let mut data = vec![0; entries_at + ids.len() * 6];
    for (i, &id) in ids.iter().enumerate() {
        let start = 100 + i * raw.len();
        data[start..start + raw.len()].copy_from_slice(&raw);
        put(&mut data, at + i * 4, id);
        put(&mut data, entries_at + i * 6, start as u32);
        data[entries_at + i * 6 + 4..entries_at + i * 6 + 6]
            .copy_from_slice(&(raw.len() as u16).to_le_bytes());
    }
    for (i, &(new, source)) in copies.iter().enumerate() {
        put(&mut data, copy_at + i * 8, new);
        put(&mut data, copy_at + i * 8 + 4, source);
    }
    (
        data,
        CatalogLayout {
            at,
            count: ids.len(),
            copies: copies.len(),
            body: 100..at,
            max_id: 10,
        },
    )
}

#[test]
fn source_copy_order_allows_prior_copies_and_overwrites_but_skips_zero_missing_and_out_of_range() {
    let copies = [
        (8, 7),
        (7, 1),
        (8, 7),
        (3, 7),
        (0, 1),
        (9, 0),
        (11, 1),
        (9, 11),
    ];
    let (data, layout) = catalog_fixture(&[1, 3], &copies);
    let rows = prefix::catalog(&data, layout).unwrap();
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [0, 1, 3, 7, 8]
    );
    for row in rows {
        assert_eq!(row.vendor_stack, value(24, 0));
    }
    // Copy source 7 wasn't available for the first entry and is not retried.
    let (data, layout) = catalog_fixture(&[1], &[(8, 7), (7, 1)]);
    assert_eq!(
        prefix::catalog(&data, layout)
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [1, 7]
    );
}

#[test]
fn catalog_rejects_duplicate_baselines_invalid_ids_and_out_of_body_entries() {
    let (data, layout) = catalog_fixture(&[1, 1], &[]);
    assert!(prefix::catalog(&data, layout).is_err());
    let (data, layout) = catalog_fixture(&[11], &[]);
    assert!(prefix::catalog(&data, layout).is_err());
    for (start, size) in [(99, 300), (4095, 300), (100, 0)] {
        let (mut data, layout) = catalog_fixture(&[1], &[]);
        put(&mut data, 4100, start);
        data[4104..4106].copy_from_slice(&(size as u16).to_le_bytes());
        assert!(prefix::catalog(&data, layout).is_err());
    }
}

fn prefix_metadata() -> Vec<u8> {
    let mut data = vec![0; 6971318];
    for (at, v) in [
        (0, 0x35434457),
        (4, 5),
        (136, 19236),
        (140, 68),
        (144, 356),
        (152, 0x919BE54E),
        (156, 0x6FCC3191),
        (168, 1 << 6),
        (172, 5),
        (176, 68),
        (188, 68 * 24),
        (200, 8),
    ] {
        put(&mut data, at, v);
    }
    let counts = [19167, 1, 1, 1, 1, 63, 1, 1];
    let offsets = [
        2732, 6971318, 6971702, 6972058, 6972438, 6972806, 6998406, 6998758,
    ];
    for i in 0..8 {
        let at = 204 + i * 40;
        let copies = if i == 0 { 57 } else { 0 };
        let end = if i == 7 { 6999130 } else { offsets[i + 1] };
        for (field, v) in [
            (0, if i == 0 { 0 } else { 1 }),
            (8, offsets[i]),
            (12, counts[i]),
            (20, end - counts[i] * 14 - copies * 8),
            (24, counts[i] * 4),
            (32, counts[i]),
            (36, copies),
        ] {
            put(&mut data, at + field, v);
        }
    }
    for field in 0..68 {
        data[524 + field * 4..526 + field * 4]
            .copy_from_slice(&(((4 - decode::width(field)) * 8) as i16).to_le_bytes());
    }
    data
}

#[test]
fn exact_prefix_admission_checks_schema_visibility_widths_and_both_id_table_extents() {
    let data = prefix_metadata();
    assert!(prefix::validate(&data).is_ok());
    for at in [
        0, 136, 140, 152, 156, 168, 172, 188, 200, 224, 228, 244, 252, 256, 524, 728,
    ] {
        let mut invalid = data.clone();
        invalid[at] ^= 1;
        assert!(prefix::validate(&invalid).is_err(), "offset {at}");
    }
    assert!(prefix::validate(&data[..data.len() - 1]).is_err());
    let mut extra = data;
    extra.push(0);
    assert!(prefix::validate(&extra).is_err());
}

#[test]
fn header_success_never_returns_zero_filled_missing_records_as_a_valid_batch() {
    assert!(prefix::load(&prefix_metadata()).is_err());
}

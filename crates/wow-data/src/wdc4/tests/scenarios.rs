//! WDC4 reader regressions.
//!
//! Moved out of wdc4.rs under #685; every test is unchanged.

use super::*;

#[test]
fn test_read_bits_simple() {
    // Byte 0 = 0b1010_0101
    let data = vec![0xA5];
    // Read 4 bits from bit 0: should be 0b0101 = 5
    assert_eq!(read_bits(&data, 0, 0, 4), 5);
    // Read 4 bits from bit 4: should be 0b1010 = 10
    assert_eq!(read_bits(&data, 0, 4, 4), 10);
    // Read 8 bits from bit 0: should be 0xA5
    assert_eq!(read_bits(&data, 0, 0, 8), 0xA5);
}

#[test]
fn test_read_bits_cross_byte() {
    let data = vec![0xFF, 0x00];
    // Read 4 bits starting at bit 6: crosses byte boundary
    // Byte 0 bits 6-7 = 11, Byte 1 bits 0-1 = 00 → 0b0011 = 3
    assert_eq!(read_bits(&data, 0, 6, 4), 3);
}

#[test]
fn test_read_bits_multi_byte() {
    let data = vec![0x12, 0x34, 0x56];
    // Read 16 bits from bit 0
    assert_eq!(read_bits(&data, 0, 0, 16), 0x3412);
}

#[test]
fn test_sign_extend() {
    // 5-bit value 0b11111 = 31 → sign-extended = -1
    assert_eq!(sign_extend(0x1F, 5), -1);
    // 5-bit value 0b01111 = 15 → positive = 15
    assert_eq!(sign_extend(0x0F, 5), 15);
    // 8-bit value 0xFF → -1
    assert_eq!(sign_extend(0xFF, 8), -1);
    // 8-bit value 0x7F → 127
    assert_eq!(sign_extend(0x7F, 8), 127);
    // Zero-width fields have no sign bit.
    assert_eq!(sign_extend(0, 0), 0);
    // A full-width signed field already has the correct two's-complement bits.
    assert_eq!(sign_extend(0x8000_0000, 32), i32::MIN);
}

#[test]
fn get_field_i32_sign_extends_narrow_bitpacked_signed_like_cpp() {
    let reader = Wdc4Reader {
        header: Wdc4Header {
            record_count: 2,
            field_count: 2,
            record_size: 1,
            string_table_size: 0,
            table_hash: 0,
            _layout_hash: 0,
            min_id: 1,
            max_id: 2,
            _locale: 0,
            flags: 0,
            id_index: 0,
            total_field_count: 2,
            _packed_data_offset: 0,
            _lookup_column_count: 0,
            field_storage_info_size: (2 * FIELD_STORAGE_INFO_SIZE) as u32,
            common_data_size: 0,
            pallet_data_size: 0,
            section_count: 1,
        },
        field_info: vec![
            FieldStorageInfo {
                field_offset_bits: 0,
                field_size_bits: 5,
                additional_data_size: 0,
                compression: CompressionType::BitpackedSigned,
                val1: 0,
                val2: 0,
                val3: 0,
            },
            FieldStorageInfo {
                field_offset_bits: 0,
                field_size_bits: 5,
                additional_data_size: 0,
                compression: CompressionType::Bitpacked,
                val1: 0,
                val2: 0,
                val3: 0,
            },
        ],
        pallet_data: vec![Vec::new(), Vec::new()],
        common_data: vec![HashMap::new(), HashMap::new()],
        // 0b0_1111 is +15; 0b1_1111 is -1 in a signed five-bit field.
        record_data: vec![0x0F, 0x1F],
        record_ids: vec![1, 2],
        copy_table: Vec::new(),
        id_to_index: HashMap::from([(1, 0), (2, 1)]),
        relationship_ids: vec![None, None],
        record_offsets: Vec::new(),
        record_sizes: vec![1, 1],
        string_tables: Vec::new(),
        record_string_table_indices: vec![None, None],
    };

    assert_eq!(reader.get_field_i32(0, 0), 15);
    assert_eq!(reader.get_field_i32(1, 0), -1);
    assert_eq!(
        reader.get_field_i32(1, 1),
        31,
        "narrow unsigned Bitpacked fields must not be sign-extended"
    );
    assert_eq!(
        reader.get_field_u32(1, 0),
        0x1F,
        "unsigned access must continue exposing the raw payload"
    );
}

#[test]
fn real_spell_effect_base_points_decode_negative_int32_like_cpp() {
    let path = [
        "/home/server/woltk-server-core/Data/dbc/enUS/SpellEffect.db2",
        "/home/server/woltk-server-core/Data/dbc/esES/SpellEffect.db2",
    ]
    .into_iter()
    .map(std::path::Path::new)
    .find(|path| path.exists());
    let Some(path) = path else {
        eprintln!("Skipping test: SpellEffect.db2 not found");
        return;
    };

    let reader = Wdc4Reader::open(path).expect("failed to parse SpellEffect.db2");
    let base_points = &reader.field_info[7];
    assert_eq!(
        base_points.compression,
        CompressionType::BitpackedSigned,
        "the 3.4.3 fixture stores regular SpellEffect.EffectBasePoints as a signed integer"
    );

    let (record_id, index, value) = reader
        .iter_records()
        .find_map(|(record_id, index)| {
            let value = reader.get_field_i32(index, 7);
            (value < 0).then_some((record_id, index, value))
        })
        .expect("the 3.4.3 SpellEffect fixture must contain negative base points");
    let raw = reader.get_field_u32(index, 7);
    assert_eq!(
        value,
        sign_extend(raw, u32::from(base_points.field_size_bits)),
        "record {record_id} must retain its signed EffectBasePoints payload"
    );
    assert_ne!(
        raw as i32, value,
        "the fixture must exercise an actually narrow negative payload"
    );
}

#[test]
fn test_compression_type_from_u32() {
    assert_eq!(CompressionType::from_u32(0).unwrap(), CompressionType::None);
    assert_eq!(
        CompressionType::from_u32(3).unwrap(),
        CompressionType::Pallet
    );
    assert!(CompressionType::from_u32(99).is_err());
}

#[test]
fn inline_record_ids_key_common_fields_like_cpp() {
    let record_data = [100u32.to_le_bytes(), 300u32.to_le_bytes()].concat();
    let record_offsets = vec![0, 4];
    let field_info = vec![
        FieldStorageInfo {
            field_offset_bits: 0,
            field_size_bits: 32,
            additional_data_size: 0,
            compression: CompressionType::None,
            val1: 0,
            val2: 0,
            val3: 0,
        },
        FieldStorageInfo {
            field_offset_bits: 0,
            field_size_bits: 0,
            additional_data_size: 0,
            compression: CompressionType::Common,
            val1: 5,
            val2: 0,
            val3: 0,
        },
    ];
    let pallet_data = vec![Vec::new(), Vec::new()];
    let record_ids = (0..2)
        .map(|record_idx| {
            read_inline_record_id(
                &record_data,
                &record_offsets,
                4,
                &field_info,
                &pallet_data,
                record_idx,
                0,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let reader = Wdc4Reader {
        header: Wdc4Header {
            record_count: 2,
            field_count: 2,
            record_size: 4,
            string_table_size: 0,
            table_hash: 0,
            _layout_hash: 0,
            min_id: 100,
            max_id: 300,
            _locale: 0,
            flags: 0,
            id_index: 0,
            total_field_count: 2,
            _packed_data_offset: 0,
            _lookup_column_count: 0,
            field_storage_info_size: 0,
            common_data_size: 0,
            pallet_data_size: 0,
            section_count: 1,
        },
        field_info,
        pallet_data,
        common_data: vec![HashMap::new(), HashMap::from([(100, 7), (300, 9)])],
        record_data,
        record_ids,
        copy_table: Vec::new(),
        id_to_index: HashMap::from([(100, 0), (300, 1)]),
        relationship_ids: Vec::new(),
        record_offsets,
        record_sizes: vec![4, 4],
        string_tables: Vec::new(),
        record_string_table_indices: vec![None, None],
    };

    assert_eq!(
        reader.iter_records().map(|(id, _)| id).collect::<Vec<_>>(),
        vec![100, 300]
    );
    assert_eq!(reader.get_field_u32(0, 1), 7);
    assert_eq!(reader.get_field_u32(1, 1), 9);
}

#[test]
fn test_parse_item_db2() {
    let path = std::path::Path::new("/home/server/woltk-server-core/Data/dbc/esES/Item.db2");
    if !path.exists() {
        eprintln!("Skipping test: Item.db2 not found");
        return;
    }

    let reader = Wdc4Reader::open(path).expect("failed to parse Item.db2");
    assert!(reader.record_count() > 0, "should have records");
    assert!(reader.total_count() > 20000, "expected >20k total items");

    // Verify a known item: Thunderfury (entry 19019)
    // class_id=2 (Weapon), subclass_id=7 (Swords), inventory_type=13 (Weapon/One-Hand)
    let mut found_any = false;
    for (id, idx) in reader.iter_records() {
        if id == 19019 {
            let class_id = reader.get_field_u8(idx, 0);
            let subclass_id = reader.get_field_u8(idx, 1);
            let inv_type = reader.get_field_i8(idx, 3);
            assert_eq!(class_id, 2, "Thunderfury class should be 2 (Weapon)");
            assert_eq!(subclass_id, 7, "Thunderfury subclass should be 7 (Sword)");
            assert_eq!(inv_type, 13, "Thunderfury inv_type should be 13 (One-Hand)");
            found_any = true;
            break;
        }
    }
    // At minimum, verify we can read fields without panicking
    if !found_any {
        let (id, idx) = reader.iter_records().next().unwrap();
        let _ = reader.get_field_u8(idx, 0);
        let _ = reader.get_field_i8(idx, 3);
        eprintln!("Thunderfury not found, first record id={id}");
    }
}

#[test]
fn test_probe_item_sparse_db2() {
    let path = std::path::Path::new("/home/server/woltk-server-core/Data/dbc/esES/ItemSparse.db2");
    if !path.exists() {
        eprintln!("Skipping test: ItemSparse.db2 not found");
        return;
    }

    let reader = Wdc4Reader::open(path).expect("failed to parse ItemSparse.db2");
    eprintln!(
        "ItemSparse: {} records, {} total, {} fields",
        reader.record_count(),
        reader.total_count(),
        reader.field_count()
    );

    // Print all field info with byte offsets
    let mut prev_end_bits = 0u32;
    for i in 0..reader.field_count() {
        let info = &reader.field_info[i];
        let start_byte = info.field_offset_bits / 8;
        let end_byte = (info.field_offset_bits + info.field_size_bits + 7) / 8;
        let gap = if info.field_offset_bits as u32 > prev_end_bits {
            format!(" GAP={}bits", info.field_offset_bits as u32 - prev_end_bits)
        } else {
            String::new()
        };
        eprintln!(
            "  f[{:2}] byte {:3}..{:3} ({:4}bits) {:?}{}",
            i, start_byte, end_byte, info.field_size_bits, info.compression, gap
        );
        prev_end_bits = info.field_offset_bits as u32 + info.field_size_bits as u32;
    }
    eprintln!(
        "Total record bit-width: {prev_end_bits} ({} bytes)",
        prev_end_bits / 8
    );

    // f[53] = StatModifierBonusAmount[10] (i16[10], 160 bits)
    // f[65] = _statModifierBonusStat[10] (i8[10], 80 bits)
    // f[45] = ItemLevel (u16)
    // f[69] = _inventoryType (i8)
    // Verify with known items
    eprintln!(
        "record_ids={}, record_offsets={}",
        reader.record_ids.len(),
        reader.record_offsets.len()
    );
    for &check_id in &[6948u32, 49623, 19364, 19019] {
        if let Some(idx) = reader.get_record_index(check_id) {
            let offset = reader.record_offsets[idx];
            let item_level = reader.get_field_u16(idx, 45);
            let inv_type = reader.get_field_i8(idx, 69);
            eprintln!(
                "\n=== Item {check_id} (idx={idx}, offset={offset}, iLvl={item_level}, invType={inv_type}) ==="
            );
            for i in 0..10 {
                let stat_type = reader.get_array_i8(idx, 65, i);
                let stat_amount = reader.get_array_i16(idx, 53, i);
                if stat_type != 0 || stat_amount != 0 {
                    eprintln!("  slot[{i}]: type={stat_type:3}, amount={stat_amount:5}");
                }
            }
        }
    }
}

#[test]
fn test_tact_key_db2_uses_table_hash_not_layout_hash_like_cpp() {
    let path = std::path::Path::new("/home/server/woltk-server-core/Data/dbc/esES/TactKey.db2");
    if !path.exists() {
        eprintln!("Skipping test: TactKey.db2 not found");
        return;
    }

    let reader = Wdc4Reader::open(path).expect("failed to parse TactKey.db2");
    assert_eq!(
        reader.table_hash(),
        0xDF2F_53CF,
        "C++ DBQueryBulk uses DB2Header::TableHash, not LayoutHash"
    );
    assert_eq!(
        reader.header._layout_hash, 0xD3F6_1A9E,
        "layout hash must not be used as the DBQueryBulk table hash"
    );

    let mut ids = reader.iter_records().map(|(id, _)| id).collect::<Vec<_>>();
    ids.sort_unstable();
    eprintln!("TactKey ids: {ids:?}");
    assert_eq!(
        ids,
        vec![
            15, 52, 55, 56, 57, 81, 92, 93, 94, 97, 98, 99, 100, 101, 103, 105
        ],
        "this fixture has no copy-table records; C++ will also answer Invalid for other TactKey IDs"
    );
}

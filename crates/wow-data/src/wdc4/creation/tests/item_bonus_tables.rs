use super::super::super::{CompressionType, FieldStorageInfo, Wdc4Reader};
use crate::wdc4::creation::{CreationDb2, CreationTable};
use std::collections::HashMap;

const TABLES: [CreationTable; 5] = [
    CreationTable::ItemBonus,
    CreationTable::AzeriteEmpoweredItem,
    CreationTable::ItemScalingConfig,
    CreationTable::ItemOffsetCurve,
    CreationTable::ItemSquishEra,
];

#[test]
fn five_bonus_schemas_freeze_hash_layout_id_and_numeric_shapes() {
    let expected = [
        ("ItemBonus", 0x8318_900A, 0x79CD_4525, 4, None, Some(1)),
        (
            "AzeriteEmpoweredItem",
            0xAFB1_B60F,
            0x48AC_4E05,
            3,
            None,
            None,
        ),
        ("ItemScalingConfig", 0xF6AA_409E, 0x7BEA_BE68, 5, None, None),
        ("ItemOffsetCurve", 0x8993_63A4, 0x6F21_57CA, 2, None, None),
        ("ItemSquishEra", 0xACAA_F53B, 0xED6F_4ECF, 4, Some(0), None),
    ];
    for (table, (name, hash, layout, fields, id, parent)) in TABLES.into_iter().zip(expected) {
        let schema = table.schema();
        assert_eq!(schema.name, name);
        assert_eq!(
            (
                schema.hash,
                schema.layout,
                schema.fields,
                schema.id,
                schema.parent
            ),
            (hash, layout, fields, id, parent)
        );
        assert_eq!(table.parent_lookup_count(), u32::from(parent.is_some()));
    }
    assert_eq!(CreationTable::ItemBonus.numeric(0), Some((32, 4)));
    assert_eq!(CreationTable::ItemBonus.numeric(1), Some((16, 1)));
    assert_eq!(CreationTable::ItemBonus.numeric(2), Some((8, 1)));
    assert_eq!(CreationTable::ItemBonus.numeric(3), Some((8, 1)));
    assert_eq!(CreationTable::ItemSquishEra.numeric(0), Some((32, 1)));
}

#[test]
fn all_five_tables_decode_raw_signed_and_array_bits_and_item_bonus_parent_modes() {
    for table in TABLES {
        let id = 77;
        let mut raw = populated(table, id);
        raw.header.id_index = 0;
        let decoded = CreationDb2::checked(raw, table).unwrap();
        match table {
            CreationTable::ItemBonus => {
                for (index, value) in [0x8000_0000, u32::MAX, 0x7FC0_1234, 0x0102_0304]
                    .into_iter()
                    .enumerate()
                {
                    assert_eq!(decoded.bits(id, 0, index).unwrap(), value);
                }
                assert_eq!(decoded.bits(id, 1, 0).unwrap(), 0x1234);
                assert_eq!(decoded.bits(id, 2, 0).unwrap(), 0xFE);
                assert_eq!(decoded.bits(id, 3, 0).unwrap(), 0x80);

                let mut raw_field_parent = populated(table, id);
                raw_field_parent.header.id_index = 0;
                raw_field_parent.header._parent_lookup_count = 0;
                let decoded = CreationDb2::checked(raw_field_parent, table).unwrap();
                assert_eq!(decoded.bits(id, 1, 0).unwrap(), 0x1234);

                let mut relationship_parent = populated(table, id);
                relationship_parent.header.id_index = 0;
                relationship_parent.relationship_ids[0] = Some(u16::MAX as u32);
                let decoded = CreationDb2::checked(relationship_parent, table).unwrap();
                assert_eq!(decoded.bits(id, 1, 0).unwrap(), u16::MAX as u32);
            }
            CreationTable::AzeriteEmpoweredItem => {
                assert_eq!(decoded.bits(id, 0, 0).unwrap() as i32, i32::MIN);
                assert_eq!(decoded.bits(id, 1, 0).unwrap(), u32::MAX);
                assert_eq!(decoded.bits(id, 2, 0).unwrap(), 0x0102_0304);
            }
            CreationTable::ItemScalingConfig => {
                assert_eq!(decoded.bits(id, 0, 0).unwrap() as i32, i32::MIN);
                assert_eq!(decoded.bits(id, 1, 0).unwrap() as i32, -2);
                assert_eq!(decoded.bits(id, 2, 0).unwrap(), 0x7FC0_1234);
                assert_eq!(decoded.bits(id, 3, 0).unwrap(), 0x0102_0304);
                assert_eq!(decoded.bits(id, 4, 0).unwrap(), u32::MAX);
            }
            CreationTable::ItemOffsetCurve => {
                assert_eq!(decoded.bits(id, 0, 0).unwrap() as i32, -1);
                assert_eq!(decoded.bits(id, 1, 0).unwrap() as i32, i32::MIN);
            }
            CreationTable::ItemSquishEra => {
                assert_eq!(decoded.bits(id, 0, 0).unwrap(), id);
                assert_eq!(decoded.bits(id, 1, 0).unwrap() as i32, i32::MIN);
                assert_eq!(decoded.bits(id, 2, 0).unwrap() as i32, -1);
                assert_eq!(decoded.bits(id, 3, 0).unwrap(), 0x0102_0304);
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn squish_copy_targets_use_regular_loader_order_and_reserve_missing_indices() {
    let table = CreationTable::ItemSquishEra;
    let mut raw = populated(table, 77);
    raw.header.max_id = 95;
    raw.copy_table = vec![(80, 77), (81, 80), (95, 99), (90, 0)];
    let decoded = CreationDb2::checked(raw, table).unwrap();
    assert_eq!(decoded.ids().collect::<Vec<_>>(), [77, 80, 81]);
    assert_eq!(decoded.bits(80, 0, 0).unwrap(), 80);
    assert_eq!(decoded.bits(81, 0, 0).unwrap(), 81);
    assert!(decoded.bits(95, 0, 0).is_err());
    assert!(decoded.bits(90, 0, 0).is_err());
    assert_eq!(decoded.storage_last_index(), 95);
}

#[test]
fn complete_empty_files_use_exact_metadata_id_flags_and_parent_gates() {
    let directory = std::env::temp_dir().join(format!(
        "forever-empty-item-bonus-tables-{}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    for table in TABLES {
        let schema = table.schema();
        let parent_counts: &[u32] = if matches!(table, CreationTable::ItemBonus) {
            &[0, 1]
        } else {
            &[0]
        };
        for &parent_count in parent_counts {
            let path = directory.join(format!("{}.db2", schema.name));
            let bytes = empty_file(table, parent_count);
            assert_eq!(bytes.len(), 204 + schema.fields * 4);
            std::fs::write(&path, &bytes).unwrap();
            let decoded = CreationDb2::open(&directory, table).unwrap();
            assert_eq!(decoded.ids().count(), 0);
            if matches!(table, CreationTable::ItemSquishEra) {
                assert_eq!(decoded.storage_last_index(), 0);
            }
            std::fs::write(&path, &bytes[..204]).unwrap();
            assert!(CreationDb2::open(&directory, table).is_err());
            std::fs::remove_file(&path).unwrap();
        }
    }

    let path = directory.join("AzeriteEmpoweredItem.db2");
    let mut wrong_id = empty_file(CreationTable::AzeriteEmpoweredItem, 0);
    word(&mut wrong_id, 172, 4 | (1 << 16));
    std::fs::write(&path, wrong_id).unwrap();
    assert!(CreationDb2::open(&directory, CreationTable::AzeriteEmpoweredItem).is_err());
    std::fs::remove_file(&path).unwrap();

    let path = directory.join("ItemBonus.db2");
    let mut too_many_parents = empty_file(CreationTable::ItemBonus, 2);
    word(&mut too_many_parents, 184, 2);
    std::fs::write(&path, too_many_parents).unwrap();
    assert!(CreationDb2::open(&directory, CreationTable::ItemBonus).is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

fn populated(table: CreationTable, id: u32) -> Wdc4Reader {
    let schema = table.schema();
    let mut infos = Vec::with_capacity(schema.fields);
    let mut bit_offset = 0usize;
    for field_index in 0..schema.fields {
        let (bits, count) = table.numeric(field_index).unwrap();
        infos.push(FieldStorageInfo {
            field_offset_bits: bit_offset as u16,
            field_size_bits: (bits * count) as u16,
            additional_data_size: 0,
            compression: CompressionType::None,
            val1: 0,
            val2: 0,
            val3: 0,
        });
        bit_offset += bits * count;
    }
    let record_size = bit_offset / 8;
    let mut data = vec![0; record_size];
    let fields: Vec<Vec<u32>> = match table {
        CreationTable::ItemBonus => vec![
            vec![0x8000_0000, u32::MAX, 0x7FC0_1234, 0x0102_0304],
            vec![0x1234],
            vec![0xFE],
            vec![0x80],
        ],
        CreationTable::AzeriteEmpoweredItem => {
            vec![vec![0x8000_0000], vec![u32::MAX], vec![0x0102_0304]]
        }
        CreationTable::ItemScalingConfig => vec![
            vec![0x8000_0000],
            vec![0xFFFF_FFFE],
            vec![0x7FC0_1234],
            vec![0x0102_0304],
            vec![u32::MAX],
        ],
        CreationTable::ItemOffsetCurve => vec![vec![u32::MAX], vec![0x8000_0000]],
        CreationTable::ItemSquishEra => {
            vec![
                vec![id],
                vec![0x8000_0000],
                vec![u32::MAX],
                vec![0x0102_0304],
            ]
        }
        _ => unreachable!(),
    };
    for (field, values) in fields.iter().enumerate() {
        let (bits, _) = table.numeric(field).unwrap();
        let start = usize::from(infos[field].field_offset_bits) / 8;
        for (array_index, value) in values.iter().copied().enumerate() {
            let offset = start + array_index * bits / 8;
            data[offset..offset + bits / 8].copy_from_slice(&value.to_le_bytes()[..bits / 8]);
        }
    }
    let mut value = super::reader(
        table,
        &[id],
        record_size,
        data,
        infos,
        vec![vec![]; schema.fields],
        vec![HashMap::new(); schema.fields],
        vec![],
        vec![None],
        0,
        0,
        0,
    );
    value.header.id_index = 0;
    value.header._locale = u32::MAX;
    value
}

fn empty_file(table: CreationTable, parent_count: u32) -> Vec<u8> {
    let schema = table.schema();
    let flags = if schema.id.is_some() { 0 } else { 4 };
    let mut bytes = vec![0; 204 + schema.fields * 4];
    word(&mut bytes, 0, 0x3543_4457);
    word(&mut bytes, 4, 5);
    word(&mut bytes, 140, schema.fields as u32);
    word(&mut bytes, 152, schema.hash);
    word(&mut bytes, 156, schema.layout);
    word(&mut bytes, 168, u32::MAX);
    word(&mut bytes, 172, flags);
    word(&mut bytes, 176, schema.fields as u32);
    word(&mut bytes, 184, parent_count);
    bytes
}

fn word(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

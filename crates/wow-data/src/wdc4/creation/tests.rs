use super::super::{CompressionType, FieldStorageInfo, Wdc4Header, Wdc4Reader};
use super::{CreationDb2, CreationTable};
use std::collections::HashMap;

const FIELDS: usize = 9;

#[test]
fn birth_masks_keep_both_words_and_signed_skill_metadata_keeps_unsigned_ids() {
    let table = CreationTable::SkillRaceClass;
    let fields = 7;
    let mut info = vec![field(CompressionType::None, 0, 0, 0, 0); fields];
    info[4].field_offset_bits = 16;
    info[5].field_offset_bits = 24;
    info[6].field_offset_bits = 40;
    let data = vec![0xff, 0xff, 0x80, 0, 0x80, 1, 0, 0, 0x80, 2, 0, 0, 0x80];
    let make = |parent| {
        reader(
            table,
            &[7],
            data.len(),
            data.clone(),
            info.clone(),
            vec![vec![]; fields],
            vec![HashMap::new(); fields],
            vec![],
            vec![parent],
            0,
            0,
            0,
        )
    };
    let value = CreationDb2::checked(make(None), table).unwrap();
    assert_eq!(value.bits(7, 0, 0).unwrap() as u16, u16::MAX);
    assert_eq!(value.bits(7, 4, 0).unwrap() as i8, i8::MIN);
    assert_eq!(value.bits(7, 5, 0).unwrap() as i16, i16::MIN);
    assert_eq!(value.bits(7, 6, 0).unwrap(), 0x8000_0001);
    assert_eq!(value.bits(7, 6, 1).unwrap(), 0x8000_0002);
    assert!(value.bits(7, 6, 2).is_err());
    let value = CreationDb2::checked(make(Some(65535)), table).unwrap();
    assert_eq!(value.bits(7, 0, 0).unwrap(), 65535);
    let value = CreationDb2::checked(make(Some(65536)), table).unwrap();
    assert!(value.bits(7, 0, 0).is_err());
}

#[test]
fn birth_schema_hash_and_layout_gates_apply_to_every_numeric_table() {
    for table in [
        CreationTable::SkillLine,
        CreationTable::SkillRaceClass,
        CreationTable::SkillAbility,
        CreationTable::Loadout,
        CreationTable::LoadoutItem,
    ] {
        let fields = table.schema().fields;
        let make = || {
            reader(
                table,
                &[7],
                12,
                [7u32, 0x8000_0001, 0x8000_0002]
                    .into_iter()
                    .flat_map(u32::to_le_bytes)
                    .collect(),
                vec![field(CompressionType::None, 0, 0, 0, 0); fields],
                vec![vec![]; fields],
                vec![HashMap::new(); fields],
                vec![],
                vec![None],
                0,
                0,
                0,
            )
        };
        assert!(CreationDb2::checked(make(), table).is_ok());
        let mut invalid = make();
        invalid.header.table_hash ^= 1;
        assert!(CreationDb2::checked(invalid, table).is_err());
        let mut invalid = make();
        invalid.header._layout_hash ^= 1;
        assert!(CreationDb2::checked(invalid, table).is_err());
        let mut invalid = make();
        invalid.header._parent_lookup_count ^= 1;
        assert!(CreationDb2::checked(invalid, table).is_err());
    }
}

#[test]
fn class_inline_id_is_a_byte_not_a_four_byte_record_read() {
    let make = |ids: &[u32], copies| {
        reader(
            CreationTable::Class,
            ids,
            1,
            vec![7],
            vec![field(CompressionType::None, 0, 0, 0, 0); 43],
            vec![vec![]; 43],
            vec![HashMap::new(); 43],
            copies,
            vec![None],
            0,
            0,
            0,
        )
    };
    let table = CreationDb2::checked(make(&[7], vec![(255, 7)]), CreationTable::Class).unwrap();
    assert_eq!(table.bits(7, 29, 0).unwrap(), 7);
    assert_eq!(table.bits(255, 29, 0).unwrap(), 255);
    assert_eq!(table.bits(7, 32, 0).unwrap(), 7);
    assert!(CreationDb2::checked(make(&[8], vec![]), CreationTable::Class).is_err());
    assert!(CreationDb2::checked(make(&[7], vec![(256, 7)]), CreationTable::Class).is_err());
}

#[test]
fn class_power_class_comes_from_byte_parent_not_a_fabricated_in_record_column() {
    let make = |parent| {
        reader(
            CreationTable::ClassPower,
            &[7],
            1,
            vec![255],
            vec![field(CompressionType::None, 0, 0, 0, 0)],
            vec![vec![]],
            vec![HashMap::new()],
            vec![],
            vec![parent],
            0,
            0,
            0,
        )
    };
    let table = CreationDb2::checked(make(Some(9)), CreationTable::ClassPower).unwrap();
    assert_eq!(table.bits(7, 0, 0).unwrap() as i8, -1);
    assert_eq!(table.bits(7, 1, 0).unwrap(), 9);
    // Missing relationship leaves the source-initialized extra parent zero.
    let table = CreationDb2::checked(make(None), CreationTable::ClassPower).unwrap();
    assert_eq!(table.bits(7, 1, 0).unwrap(), 0);
    let table = CreationDb2::checked(make(Some(256)), CreationTable::ClassPower).unwrap();
    assert!(table.bits(7, 1, 0).is_err());
}

#[test]
fn specialization_in_record_parent_does_not_invent_a_relationship_lookup() {
    let make = || {
        let fields = 13;
        let mut value = reader(
            CreationTable::Specialization,
            &[7],
            4,
            7u32.to_le_bytes().to_vec(),
            vec![field(CompressionType::None, 0, 0, 0, 0); fields],
            vec![vec![]; fields],
            vec![HashMap::new(); fields],
            vec![],
            vec![None],
            0,
            0,
            0,
        );
        value.header._parent_lookup_count = 0;
        value
    };
    let store = CreationDb2::checked(make(), CreationTable::Specialization).unwrap();
    assert_eq!(store.bits(7, 4, 0).unwrap(), 7);
    let mut invalid = make();
    invalid.header._parent_lookup_count = 1;
    assert!(CreationDb2::checked(invalid, CreationTable::Specialization).is_err());
    // A no-parent schema still forbids relationship data.
    let mut invalid = make();
    let schema = CreationTable::PowerType.schema();
    invalid.header.table_hash = schema.hash;
    invalid.header._layout_hash = schema.layout;
    invalid.header.id_index = 2;
    invalid.header._parent_lookup_count = 1;
    assert!(CreationDb2::checked(invalid, CreationTable::PowerType).is_err());
}

fn field(
    compression: CompressionType,
    additional_data_size: u32,
    val1: u32,
    val2: u32,
    val3: u32,
) -> FieldStorageInfo {
    FieldStorageInfo {
        field_offset_bits: 0,
        field_size_bits: 32,
        additional_data_size,
        compression,
        val1,
        val2,
        val3,
    }
}

fn requirement_fields() -> Vec<FieldStorageInfo> {
    vec![field(CompressionType::None, 0, 0, 0, 0); FIELDS]
}

fn reader(
    table: CreationTable,
    ids: &[u32],
    record_size: usize,
    record_data: Vec<u8>,
    field_info: Vec<FieldStorageInfo>,
    pallet_data: Vec<Vec<u32>>,
    common_data: Vec<HashMap<u32, u32>>,
    copy_table: Vec<(u32, u32)>,
    relationship_ids: Vec<Option<u32>>,
    packed_data_offset: u32,
    common_data_size: u32,
    pallet_data_size: u32,
) -> Wdc4Reader {
    let schema = table.schema();
    assert_eq!(field_info.len(), schema.fields);
    assert_eq!(pallet_data.len(), schema.fields);
    assert_eq!(common_data.len(), schema.fields);
    assert_eq!(record_data.len(), ids.len() * record_size);
    let id_to_index = ids
        .iter()
        .copied()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect();
    Wdc4Reader {
        header: Wdc4Header {
            format_version: 5,
            record_count: ids.len() as u32,
            field_count: schema.fields as u32,
            record_size: record_size as u32,
            string_table_size: 0,
            table_hash: schema.hash,
            _layout_hash: schema.layout,
            min_id: ids.iter().copied().min().unwrap_or(0),
            max_id: ids
                .iter()
                .copied()
                .chain(copy_table.iter().map(|&(new, _)| new))
                .max()
                .unwrap_or(0),
            _locale: 0,
            flags: if schema.id.is_some() { 0 } else { 4 },
            id_index: schema.id.map(|id| id as u16).unwrap_or(u16::MAX),
            total_field_count: schema.fields as u32,
            _packed_data_offset: packed_data_offset,
            _lookup_column_count: 0,
            _parent_lookup_count: u32::from(schema.parent.is_some()),
            field_storage_info_size: (schema.fields * super::super::FIELD_STORAGE_INFO_SIZE) as u32,
            common_data_size,
            pallet_data_size,
            section_count: 1,
        },
        field_info,
        pallet_data,
        common_data,
        record_data,
        record_ids: ids.to_vec(),
        copy_table,
        id_to_index,
        relationship_ids,
        record_offsets: (0..ids.len()).map(|index| index * record_size).collect(),
        record_sizes: vec![record_size; ids.len()],
        string_tables: Vec::new(),
        record_string_table_indices: vec![None; ids.len()],
    }
}

fn empty_blobs() -> (Vec<Vec<u32>>, Vec<HashMap<u32, u32>>) {
    (vec![Vec::new(); FIELDS], vec![HashMap::new(); FIELDS])
}

fn set_bits(bytes: &mut [u8], offset: usize, width: usize, value: u32) {
    for bit in 0..width {
        let byte = (offset + bit) / 8;
        let mask = 1u8 << ((offset + bit) % 8);
        if value & (1 << bit) != 0 {
            bytes[byte] |= mask;
        } else {
            bytes[byte] &= !mask;
        }
    }
}

#[test]
fn packed_values_use_header_absolute_offset() {
    let mut fields = requirement_fields();
    fields[1] = field(CompressionType::Bitpacked, 0, 3, 5, 0);
    let mut bytes = vec![0; 2];
    set_bits(&mut bytes, 8 + 3, 5, 21);
    let (pallet_data, common_data) = empty_blobs();
    let reader = reader(
        CreationTable::Requirement,
        &[7],
        2,
        bytes,
        fields,
        pallet_data,
        common_data,
        Vec::new(),
        vec![None],
        1,
        0,
        0,
    );
    let db = CreationDb2::checked(reader, CreationTable::Requirement).unwrap();
    assert_eq!(db.bits(7, 1, 0).unwrap(), 21);
}

#[test]
fn palette_array_checks_cardinality_and_bounds() {
    let mut fields = requirement_fields();
    fields[8] = field(CompressionType::PalletArray, 8, 0, 2, 2);
    let (mut pallet_data, common_data) = empty_blobs();
    pallet_data[8] = vec![10, 11];
    let valid = reader(
        CreationTable::Requirement,
        &[1],
        1,
        vec![0],
        fields.clone(),
        pallet_data.clone(),
        common_data.clone(),
        Vec::new(),
        vec![None],
        0,
        0,
        8,
    );
    let db = CreationDb2::checked(valid, CreationTable::Requirement).unwrap();
    assert_eq!(db.bits(1, 8, 0).unwrap(), 10);
    assert_eq!(db.bits(1, 8, 1).unwrap(), 11);

    let out_of_range = reader(
        CreationTable::Requirement,
        &[1],
        1,
        vec![2],
        fields.clone(),
        pallet_data,
        common_data.clone(),
        Vec::new(),
        vec![None],
        0,
        0,
        8,
    );
    let db = CreationDb2::checked(out_of_range, CreationTable::Requirement).unwrap();
    assert!(db.bits(1, 8, 0).is_err());

    let mut bad_cardinality = fields;
    bad_cardinality[8].val3 = 3;
    let (pallet_data, common_data) = empty_blobs();
    let mut pallet_data = pallet_data;
    pallet_data[8] = vec![10, 11];
    assert!(
        CreationDb2::checked(
            reader(
                CreationTable::Requirement,
                &[1],
                1,
                vec![0],
                bad_cardinality,
                pallet_data,
                common_data,
                Vec::new(),
                vec![None],
                0,
                0,
                8,
            ),
            CreationTable::Requirement,
        )
        .is_err()
    );
}

#[test]
fn missing_packed_and_none_record_bits_are_rejected() {
    let mut packed_fields = requirement_fields();
    packed_fields[1] = field(CompressionType::Bitpacked, 0, 0, 16, 0);
    let (pallet_data, common_data) = empty_blobs();
    let db = CreationDb2::checked(
        reader(
            CreationTable::Requirement,
            &[1],
            1,
            vec![0],
            packed_fields,
            pallet_data,
            common_data,
            Vec::new(),
            vec![None],
            0,
            0,
            0,
        ),
        CreationTable::Requirement,
    )
    .unwrap();
    assert!(db.bits(1, 1, 0).is_err());

    let mut none_fields = requirement_fields();
    none_fields[2].field_offset_bits = 8;
    let (pallet_data, common_data) = empty_blobs();
    let db = CreationDb2::checked(
        reader(
            CreationTable::Requirement,
            &[1],
            1,
            vec![0],
            none_fields,
            pallet_data,
            common_data,
            Vec::new(),
            vec![None],
            0,
            0,
            0,
        ),
        CreationTable::Requirement,
    )
    .unwrap();
    assert!(db.bits(1, 2, 0).is_err());
}

#[test]
fn common_default_and_source_id_copy_are_distinct() {
    let mut fields = requirement_fields();
    fields[1] = field(CompressionType::Common, 8, 77, 0, 0);
    let (pallet_data, mut common_data) = empty_blobs();
    common_data[1].insert(42, 123);
    let db = CreationDb2::checked(
        reader(
            CreationTable::Requirement,
            &[42, 43],
            1,
            vec![0, 0],
            fields,
            pallet_data,
            common_data,
            vec![(44, 42)],
            vec![None, None],
            0,
            8,
            0,
        ),
        CreationTable::Requirement,
    )
    .unwrap();
    assert_eq!(db.bits(42, 1, 0).unwrap(), 123);
    assert_eq!(db.bits(43, 1, 0).unwrap(), 77);
    assert_eq!(db.bits(44, 1, 0).unwrap(), 123);
}

#[test]
fn parent_override_and_extra_parent_default_are_explicit() {
    let (pallet_data, common_data) = empty_blobs();
    let with_parent = reader(
        CreationTable::RequiredChoice,
        &[7],
        4,
        vec![0; 4],
        vec![field(CompressionType::None, 0, 0, 0, 0)],
        vec![pallet_data[0].clone()],
        vec![common_data[0].clone()],
        Vec::new(),
        vec![Some(99)],
        0,
        0,
        0,
    );
    let db = CreationDb2::checked(with_parent, CreationTable::RequiredChoice).unwrap();
    assert_eq!(db.bits(7, 1, 0).unwrap(), 99);

    // RequiredChoice's parent field is one past its one physical field.  The
    // source FillParentLookup zero-initializes that extra field when no
    // relationship record is present; this is the only current schema that
    // reaches the checked view's explicit extra-parent default branch.
    let (pallet_data, common_data) = empty_blobs();
    let without_parent = reader(
        CreationTable::RequiredChoice,
        &[7],
        4,
        vec![0xFF; 4],
        vec![field(CompressionType::None, 0, 0, 0, 0)],
        vec![pallet_data[0].clone()],
        vec![common_data[0].clone()],
        Vec::new(),
        vec![None],
        0,
        0,
        0,
    );
    let db = CreationDb2::checked(without_parent, CreationTable::RequiredChoice).unwrap();
    assert_eq!(db.bits(7, 1, 0).unwrap(), 0);
}

#[test]
fn copies_must_follow_sources_and_never_duplicate_ids() {
    let make = |copies| {
        let (pallet_data, common_data) = empty_blobs();
        CreationDb2::checked(
            reader(
                CreationTable::Requirement,
                &[10],
                1,
                vec![0],
                requirement_fields(),
                pallet_data,
                common_data,
                copies,
                vec![None],
                0,
                0,
                0,
            ),
            CreationTable::Requirement,
        )
    };
    let db = make(vec![(20, 10), (30, 20)]).unwrap();
    assert_eq!(db.ids().collect::<Vec<_>>(), vec![10, 20, 30]);
    assert!(make(vec![(20, 999)]).is_err());
    assert!(make(vec![(20, 10), (20, 10)]).is_err());
}

#[test]
fn maximum_includes_copies_and_must_fit_header() {
    let (palette, common) = empty_blobs();
    let mut input = reader(
        CreationTable::Requirement,
        &[10],
        1,
        vec![0],
        requirement_fields(),
        palette,
        common,
        vec![(20, 10)],
        vec![None],
        0,
        0,
        0,
    );
    input.header.max_id = 19;
    assert!(CreationDb2::checked(input, CreationTable::Requirement).is_err());
}

#[test]
fn truncated_or_duplicate_common_blob_ids_are_rejected() {
    let mut fields = requirement_fields();
    fields[1] = field(CompressionType::Common, 16, 7, 0, 0);
    let (pallet_data, mut common_data) = empty_blobs();
    common_data[1].insert(1, 2);
    // split_common_data stores common entries by ID.  A duplicate source ID
    // therefore leaves one map entry while the blob still declares two; a
    // typed consumer must fail closed instead of silently using its default.
    assert!(
        CreationDb2::checked(
            reader(
                CreationTable::Requirement,
                &[1],
                1,
                vec![0],
                fields.clone(),
                pallet_data.clone(),
                common_data,
                Vec::new(),
                vec![None],
                0,
                16,
                0,
            ),
            CreationTable::Requirement,
        )
        .is_err()
    );

    let (pallet_data, common_data) = empty_blobs();
    fields[1].additional_data_size = 8;
    assert!(
        CreationDb2::checked(
            reader(
                CreationTable::Requirement,
                &[1],
                1,
                vec![0],
                fields,
                pallet_data,
                common_data,
                Vec::new(),
                vec![None],
                0,
                8,
                0,
            ),
            CreationTable::Requirement,
        )
        .is_err()
    );
}

fn name_reader() -> Wdc4Reader {
    let mut input = reader(
        CreationTable::NameReserved,
        &[1, 2],
        4,
        [8u32.to_le_bytes(), 8u32.to_le_bytes()].concat(),
        vec![field(CompressionType::None, 0, 0, 0, 0)],
        vec![Vec::new()],
        vec![HashMap::new()],
        vec![(3, 2)],
        vec![None; 2],
        0,
        0,
        0,
    );
    input.header.string_table_size = 9;
    input.header.section_count = 2;
    input.string_tables = vec![b"One\0".to_vec(), b"Deux\0".to_vec()];
    input.record_string_table_indices = vec![Some(0), Some(1)];
    input
}

#[test]
fn checked_strings_use_global_physical_field_displacement_and_copy_source() {
    let db = CreationDb2::checked(name_reader(), CreationTable::NameReserved).unwrap();
    assert_eq!(db.string(1, 0).unwrap(), "One");
    assert_eq!(db.string(2, 0).unwrap(), "Deux");
    assert_eq!(db.string(3, 0).unwrap(), "Deux");
    assert!(db.string(1, 1).is_err());
    assert!(db.string(99, 0).is_err());
    let mut input = name_reader();
    input.record_data[..4].copy_from_slice(&12u32.to_le_bytes());
    let db = CreationDb2::checked(input, CreationTable::NameReserved).unwrap();
    assert_eq!(db.string(1, 0).unwrap(), "Deux");
    let mut input = name_reader();
    input.string_tables[0][3] = b'X';
    let db = CreationDb2::checked(input, CreationTable::NameReserved).unwrap();
    assert_eq!(db.string(1, 0).unwrap(), "OneXDeux");
}

#[test]
fn checked_strings_preserve_source_null_and_reject_bad_addresses_and_encoding() {
    for relative in [1u32, 17, u32::MAX] {
        let mut input = name_reader();
        input.record_data[..4].copy_from_slice(&relative.to_le_bytes());
        let db = CreationDb2::checked(input, CreationTable::NameReserved).unwrap();
        assert!(db.string(1, 0).is_err());
    }
    let mut input = name_reader();
    input.record_data[..4].copy_from_slice(&0u32.to_le_bytes());
    let db = CreationDb2::checked(input, CreationTable::NameReserved).unwrap();
    assert_eq!(db.string(1, 0).unwrap(), "");
    let mut input = name_reader();
    input.string_tables[0][0] = 0xFF;
    assert!(
        CreationDb2::checked(input, CreationTable::NameReserved)
            .unwrap()
            .string(1, 0)
            .is_err()
    );
    let mut input = name_reader();
    input.string_tables[1][4] = b'X';
    assert!(
        CreationDb2::checked(input, CreationTable::NameReserved)
            .unwrap()
            .string(2, 0)
            .is_err()
    );
    let mut input = name_reader();
    input.header.string_table_size += 1;
    assert!(
        CreationDb2::checked(input, CreationTable::NameReserved)
            .unwrap()
            .string(1, 0)
            .is_err()
    );
}

use super::{ItemStatsStore, field_layout};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const ITEM_SPARSE_FIELD_COUNT: usize = 73;
const WDC4_HEADER_SIZE: usize = 72;
const WDC4_SECTION_HEADER_SIZE: usize = 40;
const WDC4_FIELD_META_SIZE: usize = 4;
const WDC4_FIELD_STORAGE_INFO_SIZE: usize = 24;

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

struct ItemSparseFixture(PathBuf);

impl ItemSparseFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rustycore-item-sparse-load-{}-{}",
            std::process::id(),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(root.join("data/dbc/enUS"))
            .expect("create temporary ItemSparse fixture directory");
        Self(root)
    }

    fn data_dir(&self) -> PathBuf {
        self.0.join("data")
    }

    fn item_sparse_path(&self) -> PathBuf {
        self.data_dir().join("dbc/enUS/ItemSparse.db2")
    }
}

impl Drop for ItemSparseFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn item_sparse_loads_signed_allowable_class() {
    let fixture = ItemSparseFixture::new();
    let rows = [
        (
            500_001,
            sparse_record(128, 0x3456, 7, 40, [0x100, 0x22, 0x33, 0x44], 10),
        ),
        (
            500_002,
            sparse_record(-1, 0x4567, 5, 123, [0x200, 0x55, 0x66, 0x77], 20),
        ),
    ];
    write_sparse_wdc4(&fixture.item_sparse_path(), &rows);

    let data_dir = fixture.data_dir();
    let store = ItemStatsStore::load(data_dir.to_str().expect("temporary path is UTF-8"), "enUS")
        .expect("load synthetic ItemSparse WDC4 through production reader");

    let mage_mask = 128i16;
    let mage_template = store
        .sparse_template(500_001)
        .expect("load positive Mage-mask ItemSparse row");
    assert_eq!(mage_template.allowable_class, mage_mask);
    assert_eq!(mage_template.allowable_class & mage_mask, mage_mask);
    assert_eq!(mage_template.max_durability, 40);
    assert_eq!(mage_template.stackable, 10);
    assert_eq!(store.raw_flags(500_001), Some([0x100, 0x22, 0x33, 0x44]));

    let negative_class_template = store
        .sparse_template(500_002)
        .expect("load signed ItemSparse AllowableClass row");
    assert_eq!(negative_class_template.allowable_class, -1);
    assert_eq!(
        negative_class_template.allowable_class & mage_mask,
        mage_mask
    );
    assert_eq!(negative_class_template.max_durability, 123);
    assert_eq!(negative_class_template.stackable, 20);
    assert_eq!(store.raw_flags(500_002), Some([0x200, 0x55, 0x66, 0x77]));

    report_installed_item_data_if_requested();
}

fn sparse_record(
    allowable_class: i16,
    random_suffix_group_id: u16,
    random_select: u16,
    max_durability: u32,
    flags: [u32; 4],
    stackable: i32,
) -> Vec<u8> {
    let layout = field_layout();
    assert_eq!(layout.len(), ITEM_SPARSE_FIELD_COUNT);

    let mut offsets = [0usize; ITEM_SPARSE_FIELD_COUNT];
    let mut record = Vec::new();
    for (field, (byte_size, is_string)) in layout.into_iter().enumerate() {
        offsets[field] = record.len();
        if is_string {
            record.push(0);
        } else {
            record.resize(record.len() + usize::from(byte_size), 0);
        }
    }

    write_field(&mut record, offsets[14], stackable.to_le_bytes());
    for (index, flag) in flags.into_iter().enumerate() {
        write_field(&mut record, offsets[23] + index * 4, flag.to_le_bytes());
    }
    write_field(&mut record, offsets[28], max_durability.to_le_bytes());
    write_field(&mut record, offsets[46], allowable_class.to_le_bytes());
    write_field(
        &mut record,
        offsets[47],
        random_suffix_group_id.to_le_bytes(),
    );
    write_field(&mut record, offsets[48], random_select.to_le_bytes());
    record
}

fn write_field<const N: usize>(record: &mut [u8], offset: usize, value: [u8; N]) {
    record[offset..offset + N].copy_from_slice(&value);
}

fn write_sparse_wdc4(path: &Path, rows: &[(u32, Vec<u8>)]) {
    assert!(!rows.is_empty());

    let field_meta_size = ITEM_SPARSE_FIELD_COUNT * WDC4_FIELD_META_SIZE;
    let field_storage_info_size = ITEM_SPARSE_FIELD_COUNT * WDC4_FIELD_STORAGE_INFO_SIZE;
    let records_offset =
        WDC4_HEADER_SIZE + WDC4_SECTION_HEADER_SIZE + field_meta_size + field_storage_info_size;
    let records_size: usize = rows.iter().map(|(_, record)| record.len()).sum();
    let records_end = records_offset + records_size;
    let offset_map_offset = records_end;
    let ids_offset = offset_map_offset + rows.len() * 6;
    let mut bytes = vec![0; ids_offset + rows.len() * 4];

    put_u32(&mut bytes, 0, u32::from_le_bytes(*b"WDC4"));
    put_u32(&mut bytes, 4, rows.len() as u32);
    put_u32(&mut bytes, 8, ITEM_SPARSE_FIELD_COUNT as u32);
    put_u32(&mut bytes, 20, 0xD532_973D);
    put_u32(&mut bytes, 28, rows.first().expect("non-empty rows").0);
    put_u32(&mut bytes, 32, rows.last().expect("non-empty rows").0);
    put_u16(&mut bytes, 40, 0x01);
    put_u16(&mut bytes, 42, u16::MAX);
    put_u32(&mut bytes, 44, ITEM_SPARSE_FIELD_COUNT as u32);
    put_u32(&mut bytes, 56, field_storage_info_size as u32);
    put_u32(&mut bytes, 68, 1);

    let section = WDC4_HEADER_SIZE;
    put_u32(&mut bytes, section + 8, records_offset as u32);
    put_u32(&mut bytes, section + 12, rows.len() as u32);
    put_u32(&mut bytes, section + 20, records_end as u32);
    put_u32(&mut bytes, section + 32, rows.len() as u32);

    let mut record_offset = records_offset;
    for (index, (id, record)) in rows.iter().enumerate() {
        bytes[record_offset..record_offset + record.len()].copy_from_slice(record);

        let offset_entry = offset_map_offset + index * 6;
        put_u32(&mut bytes, offset_entry, record_offset as u32);
        put_u16(
            &mut bytes,
            offset_entry + 4,
            u16::try_from(record.len()).expect("fixture record fits WDC4 offset-map size"),
        );
        put_u32(&mut bytes, ids_offset + index * 4, *id);
        record_offset += record.len();
    }

    fs::write(path, bytes).expect("write temporary ItemSparse WDC4 fixture");
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn report_installed_item_data_if_requested() {
    let Ok(data_dir) = std::env::var("RUSTYCORE_QA_ITEM_DATA_DIR") else {
        return;
    };
    let items = crate::ItemStore::load(&data_dir, "enUS")
        .expect("read installed Item.db2 through ItemStore");
    let sparse = ItemStatsStore::load(&data_dir, "enUS")
        .expect("read installed ItemSparse.db2 through ItemStatsStore");

    for item_id in [2121, 17030] {
        let item = items
            .get(item_id)
            .unwrap_or_else(|| panic!("installed Item.db2 row {item_id} is missing"));
        let template = sparse
            .sparse_template(item_id)
            .unwrap_or_else(|| panic!("installed ItemSparse.db2 row {item_id} is missing"));
        let flags = sparse
            .raw_flags(item_id)
            .expect("loaded sparse row has flags");

        eprintln!(
            "installed item row: id={item_id} class={} subclass={} invtype={} max_durability={} flags={flags:?} stackable={} allowable_class={} bonding={}",
            item.class_id,
            item.subclass_id,
            item.inventory_type,
            template.max_durability,
            template.stackable,
            template.allowable_class,
            template.bonding,
        );
        eprintln!(
            "candidate Mage class-mask check: id={item_id} mask=128 allowable_class={} intersects={}",
            template.allowable_class,
            template.allowable_class & 128 != 0,
        );
    }
}

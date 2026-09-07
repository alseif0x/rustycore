//! C++ DB2Metadata.h FactionMeta: signed int32[4] fields 14/15.
//! DB2FileLoader.cpp RecordGetVarInt(PalletArray) indexes the palette, then
//! copies the signed payload bits; record bits are not the reputation value.
use std::path::PathBuf;
use wow_data::progression_rewards::FactionStore;

fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove this test's unique fixture");
    }
}

fn fixture(palette: bool) -> Fixture {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "rustycore-faction-arrays-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let root = Fixture(root);
    let dir = root.0.join("dbc/enUS");
    std::fs::create_dir_all(&dir).unwrap();
    let palette_size = if palette { 64 } else { 0 };
    let records = 72 + 40 + 18 * 4 + 18 * 24 + palette_size;
    let mut data = vec![0; records + 2 * 128 + 8];
    data[..4].copy_from_slice(b"WDC4");
    for (offset, value) in [
        (4, 2),
        (8, 18),
        (12, 128),
        (28, 910),
        (32, 946),
        (44, 18),
        (56, 18 * 24),
        (64, palette_size),
        (68, 1),
        (80, records),
        (84, 2),
        (96, 8),
    ] {
        put32(&mut data, offset, value as u32);
    }
    data[40] = 4; // External record IDs.
    for field in 0..18 {
        let info = 72 + 40 + 18 * 4 + field * 24;
        data[info + 2..info + 4].copy_from_slice(&32u16.to_le_bytes());
        if field == 14 || field == 15 {
            let bit_offset = (64 + (field - 14) * 16) * 8;
            data[info..info + 2].copy_from_slice(&(bit_offset as u16).to_le_bytes());
            data[info + 2..info + 4]
                .copy_from_slice(&(if palette { 8u16 } else { 128 }).to_le_bytes());
            if palette {
                put32(&mut data, info + 4, 32);
                put32(&mut data, info + 8, 4); // PalletArray.
                put32(&mut data, info + 20, 4); // Four signed values per index.
            }
        }
    }
    let rows = [
        [-42000i32, 0, -1200, i32::MIN],
        [42999, -42000, 0, i32::MAX],
    ];
    for field in 0..2 {
        for record in 0..2 {
            for (element, value) in rows[record].iter().enumerate() {
                let offset = if palette {
                    records - 64 + field * 32 + record * 16 + element * 4
                } else {
                    records + record * 128 + 64 + field * 16 + element * 4
                };
                put32(&mut data, offset, *value as u32);
            }
            if palette {
                data[records + record * 128 + 64 + field * 16] = record as u8;
            }
        }
    }
    put32(&mut data, records + 256, 910);
    put32(&mut data, records + 260, 946);
    std::fs::write(dir.join("Faction.db2"), data).unwrap();
    root
}

#[test]
fn faction_reputation_arrays_preserve_signed_palette_and_uncompressed_values() {
    for palette in [false, true] {
        let fixture = fixture(palette);
        let store = FactionStore::load(fixture.0.to_str().unwrap(), "enUS").unwrap();
        for (id, expected) in [
            (910, [-42000, 0, -1200, i32::MIN]),
            (946, [42999, -42000, 0, i32::MAX]),
        ] {
            let faction = store.get(id).unwrap();
            assert_eq!(faction.reputation_base, expected, "base: palette={palette}");
            assert_eq!(faction.reputation_max, expected, "max: palette={palette}");
        }
    }
}

#[test]
#[ignore = "requires the development host's real 3.4.3 Faction.db2"]
fn real_faction_reputation_bases_preserve_hostile_values() {
    let store = FactionStore::load("/home/server/woltk-server-core/Data", "enUS").unwrap();
    for (id, expected) in [
        (910, [-42000, 0, 0, 0]),
        (946, [0, -42000, 0, 0]),
        (978, [-1200, -42000, 0, 0]),
        (1119, [-42000, 0, 0, 0]),
        (1126, [0, -42000, 0, 0]),
    ] {
        assert_eq!(store.get(id).unwrap().reputation_base, expected);
    }
}

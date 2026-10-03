use super::{SparseItemRecord, SparseItemRecords, decode};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, ops::Range};

const PREFIX_BYTES: usize = 6_971_318;
const FULL_BYTES: u32 = 6_999_130;
const COUNTS: [u32; 8] = [19167, 1, 1, 1, 1, 63, 1, 1];
const OFFSETS: [u32; 8] = [
    2732, 6971318, 6971702, 6972058, 6972438, 6972806, 6998406, 6998758,
];
const COPIES: [u32; 8] = [57, 0, 0, 0, 0, 0, 0, 0];

fn bytes<const N: usize>(data: &[u8], at: usize) -> Result<[u8; N]> {
    let end = at
        .checked_add(N)
        .context("Sparse metadata offset overflow")?;
    Ok(data
        .get(at..end)
        .context("Truncated sparse item metadata")?
        .try_into()?)
}
fn word(data: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes(data, at)?))
}

pub(super) fn validate(data: &[u8]) -> Result<()> {
    ensure!(
        data.len() == PREFIX_BYTES,
        "Wrong sparse item prefix extent"
    );
    for (at, expected) in [
        (0, 0x3543_4457),
        (4, 5),
        (136, 19236),
        (140, 68),
        (144, 356),
        (148, 0),
        (152, 0x919B_E54E),
        (156, 0x6FCC_3191),
        (168, 1 << 6), // actual esES header; Source AutoProduceStrings locale gate
        (172, 5),
        (176, 68),
        (184, 0),
        (188, 68 * 24),
        (192, 0),
        (196, 0),
        (200, 8),
    ] {
        ensure!(
            word(data, at)? == expected,
            "Wrong sparse item prefix schema"
        );
    }
    for index in 0..8 {
        let at = 204 + index * 40;
        let tact = u64::from_le_bytes(bytes(data, at)?);
        let end = if index == 7 {
            FULL_BYTES
        } else {
            OFFSETS[index + 1]
        };
        // Catalog IDs + copies + six-byte entries + separate IdTableSize.
        let catalog = end - COUNTS[index] * 14 - COPIES[index] * 8;
        ensure!(
            (tact == 0) == (index == 0),
            "Wrong sparse item section visibility"
        );
        for (field, value) in [
            (8, OFFSETS[index]),
            (12, COUNTS[index]),
            (16, 0),
            (20, catalog),
            (24, COUNTS[index] * 4),
            (28, 0),
            (32, COUNTS[index]),
            (36, COPIES[index]),
        ] {
            ensure!(
                word(data, at + field)? == value,
                "Wrong sparse item section metadata"
            );
        }
    }
    // Fresh target field entries + source GetFieldSize/offset rules. Do not
    // reuse the normal reader's compression offsets or a legacy sparse schema.
    for field in 0..68 {
        let unused = i16::from_le_bytes(bytes(data, 524 + field * 4)?);
        ensure!(
            unused == (4 - decode::width(field)) as i16 * 8,
            "Wrong target sparse item primitive width"
        );
    }
    Ok(())
}

pub(super) struct CatalogLayout {
    pub at: usize,
    pub count: usize,
    pub copies: usize,
    pub body: Range<usize>,
    pub max_id: u32,
}

pub(super) fn catalog(data: &[u8], layout: CatalogLayout) -> Result<Vec<SparseItemRecord>> {
    let copy_at = layout
        .at
        .checked_add(
            layout
                .count
                .checked_mul(4)
                .context("Sparse catalog ID extent overflow")?,
        )
        .context("Sparse catalog extent overflow")?;
    let entries_at = copy_at
        .checked_add(
            layout
                .copies
                .checked_mul(8)
                .context("Sparse catalog copy extent overflow")?,
        )
        .context("Sparse catalog extent overflow")?;
    let end = entries_at
        .checked_add(
            layout
                .count
                .checked_mul(6)
                .context("Sparse catalog entry extent overflow")?,
        )
        .context("Sparse catalog extent overflow")?;
    ensure!(
        end <= data.len() && layout.body.end <= layout.at && layout.body.start <= layout.body.end,
        "Sparse catalog exceeds admitted bytes"
    );
    let mut rows = BTreeMap::new();
    for index in 0..layout.count {
        let id = word(data, layout.at + index * 4)?;
        let start = word(data, entries_at + index * 6)? as usize;
        let size = u16::from_le_bytes(bytes(data, entries_at + index * 6 + 4)?) as usize;
        let record_end = start
            .checked_add(size)
            .context("Sparse record extent overflow")?;
        ensure!(
            size != 0 && start >= layout.body.start && record_end <= layout.body.end,
            "Sparse item record exceeds plaintext body"
        );
        ensure!(id <= layout.max_id, "Sparse item ID exceeds header maximum");
        let row = decode::record(id, &data[start..record_end])?;
        ensure!(
            rows.insert(id, row).is_none(),
            "Duplicate sparse item baseline ID"
        );
    }
    // Source AutoProduceRecordCopies: skip zero/missing/out-of-range source;
    // process file order, permit previous copies, overwrite an existing target.
    for index in 0..layout.copies {
        let new = word(data, copy_at + index * 8)?;
        let source = word(data, copy_at + index * 8 + 4)?;
        if source == 0 || source > layout.max_id || new > layout.max_id {
            continue;
        }
        if let Some(mut row) = rows.get(&source).cloned() {
            row.id = new;
            rows.insert(new, row);
        }
    }
    Ok(rows.into_values().collect())
}

pub(super) fn load(data: &[u8]) -> Result<SparseItemRecords> {
    validate(data)?;
    let at = word(data, 224)? as usize;
    let records = catalog(
        data,
        CatalogLayout {
            at,
            count: 19167,
            copies: 57,
            body: 2732..at,
            max_id: word(data, 164)?,
        },
    )?;
    Ok(SparseItemRecords {
        records,
        unknown_baseline_records: 69,
    })
}

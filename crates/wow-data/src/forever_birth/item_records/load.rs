use super::{ItemEffectRecord, ItemRecord, ItemRecords};
use crate::{
    forever_birth::{item_quantities::ItemEffectRelationRecord, item_sparse::SparseItemRecords},
    wdc4::creation::{CreationDb2, CreationTable},
};
use anyhow::Result;
use std::path::Path;

pub(super) fn available(directory: &Path) -> Result<ItemRecords> {
    let table = CreationDb2::open_item_prefix(directory, CreationTable::Item)?;
    let items = table
        .ids()
        .map(|id| {
            Ok(ItemRecord {
                id,
                class: table.bits(id, 0, 0)? as i32,
                subclass: table.bits(id, 1, 0)? as u8,
                material: table.bits(id, 2, 0)? as u8,
                inventory_type: table.bits(id, 3, 0)? as i8,
                sheathe: table.bits(id, 4, 0)? as u8,
                pet_food: table.bits(id, 5, 0)? as i32,
                sound_override: table.bits(id, 6, 0)? as i8,
                icon_file: table.bits(id, 7, 0)? as i32,
                group_sounds: table.bits(id, 8, 0)?,
                content_tuning: table.bits(id, 9, 0)? as i32,
                modified_crafting_reagent: table.bits(id, 10, 0)? as i32,
                unknown_1200: table.bits(id, 11, 0)? as u8,
                crafting_quality: table.bits(id, 12, 0)? as i32,
                squish_era: table.bits(id, 13, 0)? as i32,
                recraft_reagent_percentage: f32::from_bits(table.bits(id, 14, 0)?),
                order_source: table.bits(id, 15, 0)? as u8,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let sparse = SparseItemRecords::load_available(directory)?;
    let table = CreationDb2::open_item_prefix(directory, CreationTable::ItemEffect)?;
    let effects = table
        .ids()
        .map(|id| {
            Ok(ItemEffectRecord {
                id,
                legacy_slot: table.bits(id, 0, 0)? as u8,
                trigger: table.bits(id, 1, 0)? as u8,
                charges: table.bits(id, 2, 0)? as i16,
                cooldown: table.bits(id, 3, 0)? as i32,
                category_cooldown: table.bits(id, 4, 0)? as i32,
                spell_category: table.bits(id, 5, 0)? as u16,
                spell: table.bits(id, 6, 0)? as i32,
                specialization: table.bits(id, 7, 0)? as u16,
                player_condition: table.bits(id, 8, 0)? as i32,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let table = CreationDb2::open_item_prefix(directory, CreationTable::ItemEffectRelation)?;
    let relations = table
        .ids()
        .map(|id| {
            Ok(ItemEffectRelationRecord {
                id,
                effect: table.bits(id, 0, 0)? as i32,
                item: table.bits(id, 1, 0)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ItemRecords {
        items,
        sparse: sparse.records,
        effects,
        relations,
        unknown_baseline_records: [59, sparse.unknown_baseline_records, 40, 40],
    })
}

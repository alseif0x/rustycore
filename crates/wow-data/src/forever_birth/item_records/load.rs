use super::{
    AzeriteEmpoweredItemRecord, ItemBonusRecord, ItemEffectRecord, ItemOffsetCurveRecord,
    ItemRecord, ItemRecords, ItemScalingConfigRecord, ItemSquishEraRecord,
};
use crate::{
    forever_birth::{item_quantities::ItemEffectRelationRecord, item_sparse::SparseItemRecords},
    wdc4::creation::{CreationDb2, CreationTable},
};
use anyhow::Result;
use std::path::Path;

#[cfg(test)]
#[path = "load_tests.rs"]
mod tests;

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
    let table = CreationDb2::open(directory, CreationTable::ItemBonus)?;
    let item_bonuses = table
        .ids()
        .map(|id| {
            Ok(ItemBonusRecord {
                id,
                values: [
                    table.bits(id, 0, 0)? as i32,
                    table.bits(id, 0, 1)? as i32,
                    table.bits(id, 0, 2)? as i32,
                    table.bits(id, 0, 3)? as i32,
                ],
                parent: table.bits(id, 1, 0)? as u16,
                bonus_type: table.bits(id, 2, 0)? as u8,
                order: table.bits(id, 3, 0)? as u8,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let table = CreationDb2::open(directory, CreationTable::AzeriteEmpoweredItem)?;
    let azerite_empowered_items = table
        .ids()
        .map(|id| {
            Ok(AzeriteEmpoweredItemRecord {
                id,
                item: table.bits(id, 0, 0)? as i32,
                tier_unlock_set: table.bits(id, 1, 0)?,
                power_set: table.bits(id, 2, 0)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let table = CreationDb2::open(directory, CreationTable::ItemScalingConfig)?;
    let item_scaling_configs = table
        .ids()
        .map(|id| {
            Ok(ItemScalingConfigRecord {
                id,
                offset_curve: table.bits(id, 0, 0)? as i32,
                item_level: table.bits(id, 1, 0)? as i32,
                required_level: table.bits(id, 2, 0)? as i32,
                squish_era: table.bits(id, 3, 0)? as i32,
                flags: table.bits(id, 4, 0)? as i32,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let table = CreationDb2::open(directory, CreationTable::ItemOffsetCurve)?;
    let item_offset_curves = table
        .ids()
        .map(|id| {
            Ok(ItemOffsetCurveRecord {
                id,
                curve: table.bits(id, 0, 0)? as i32,
                offset: table.bits(id, 1, 0)? as i32,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let table = CreationDb2::open(directory, CreationTable::ItemSquishEra)?;
    let item_squish_era_file_last_index = Some(table.storage_last_index());
    let item_squish_eras = table
        .ids()
        .map(|id| {
            Ok(ItemSquishEraRecord {
                id,
                patch: table.bits(id, 1, 0)? as i32,
                curve: table.bits(id, 2, 0)? as i32,
                flags: table.bits(id, 3, 0)? as i32,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ItemRecords {
        items,
        sparse: sparse.records,
        effects,
        relations,
        item_bonuses,
        azerite_empowered_items,
        item_scaling_configs,
        item_offset_curves,
        item_squish_eras,
        item_squish_era_file_last_index,
        item_squish_era_sql_index_size: None,
        unknown_baseline_records: [59, sparse.unknown_baseline_records, 40, 40, 0, 0, 0, 0, 0],
    })
}

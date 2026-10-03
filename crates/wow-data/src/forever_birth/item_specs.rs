//! Complete target specialization/relic inputs, not an ItemTemplate/inventory.
//! 02245dcd DB2LoadInfo/Metadata/Structure; acquired 70170 hashes/layouts.
mod effective;
#[cfg(test)]
mod tests;
use crate::wdc4::creation::{CreationDb2, CreationTable};
use anyhow::Result;
pub use effective::{
    GEM_PROPERTIES_HASH, ITEM_SPEC_HASH, ITEM_SPEC_OVERRIDE_HASH, ItemSpecCatalog,
};
use std::path::Path;

#[derive(Clone, Copy)]
pub struct ItemSpecRecord {
    pub id: u32,
    pub min_level: u8,
    pub max_level: u8,
    pub item_type: u8,
    pub primary: u8,
    pub secondary: u8,
    pub specialization: u16,
}
#[derive(Clone, Copy)]
pub struct ItemSpecOverrideRecord {
    pub id: u32,
    pub specialization: u16,
    pub item: u32,
}
#[derive(Clone, Copy)]
pub struct GemPropertiesRecord {
    pub id: u32,
    pub enchantment: u16,
    pub kind: i32,
}
#[derive(Default)]
pub struct ItemSpecRecords {
    pub specs: Vec<ItemSpecRecord>,
    pub overrides: Vec<ItemSpecOverrideRecord>,
    pub gems: Vec<GemPropertiesRecord>,
}
impl ItemSpecRecords {
    /// All three complete reads finish before returning. Genuine empty stores
    /// include their full field metadata; encrypted files remain unsupported.
    pub fn load(directory: &Path) -> Result<Self> {
        let table = CreationDb2::open(directory, CreationTable::ItemSpec)?;
        let specs = table
            .ids()
            .map(|id| {
                Ok(ItemSpecRecord {
                    id,
                    min_level: table.bits(id, 0, 0)? as u8,
                    max_level: table.bits(id, 1, 0)? as u8,
                    item_type: table.bits(id, 2, 0)? as u8,
                    primary: table.bits(id, 3, 0)? as u8,
                    secondary: table.bits(id, 4, 0)? as u8,
                    specialization: table.bits(id, 5, 0)? as u16,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let table = CreationDb2::open(directory, CreationTable::ItemSpecOverride)?;
        let overrides = table
            .ids()
            .map(|id| {
                Ok(ItemSpecOverrideRecord {
                    id,
                    specialization: table.bits(id, 0, 0)? as u16,
                    item: table.bits(id, 1, 0)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let table = CreationDb2::open(directory, CreationTable::GemProperties)?;
        let gems = table
            .ids()
            .map(|id| {
                Ok(GemPropertiesRecord {
                    id,
                    enchantment: table.bits(id, 0, 0)? as u16,
                    kind: table.bits(id, 1, 0)? as i32,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            specs,
            overrides,
            gems,
        })
    }
}

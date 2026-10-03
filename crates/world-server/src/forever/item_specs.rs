//! Consuming SQL numeric rows. No raw retained SQL mirror or new state writer.
use wow_data::forever_birth::item_specs::{
    GemPropertiesRecord, ItemSpecOverrideRecord, ItemSpecRecord, ItemSpecRecords,
};
use wow_persistence::forever::item_specs::ItemSpecRows;
pub(super) fn records(rows: ItemSpecRows) -> ItemSpecRecords {
    ItemSpecRecords {
        specs: rows
            .specs
            .into_iter()
            .map(|r| ItemSpecRecord {
                id: r.id,
                min_level: r.min_level,
                max_level: r.max_level,
                item_type: r.item_type,
                primary: r.primary,
                secondary: r.secondary,
                specialization: r.specialization,
            })
            .collect(),
        overrides: rows
            .overrides
            .into_iter()
            .map(|r| ItemSpecOverrideRecord {
                id: r.id,
                specialization: r.specialization,
                item: r.item,
            })
            .collect(),
        gems: rows
            .gems
            .into_iter()
            .map(|r| GemPropertiesRecord {
                id: r.id,
                enchantment: r.enchantment,
                kind: r.kind,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::forever::item_specs::{
        GemPropertiesRow, ItemSpecOverrideRow, ItemSpecRow,
    };
    #[test]
    fn consumed_spec_override_and_gem_widths_preserve_signed_type_bits() {
        let rows = records(ItemSpecRows {
            specs: vec![ItemSpecRow {
                id: u32::MAX,
                min_level: 255,
                max_level: 255,
                item_type: 255,
                primary: 255,
                secondary: 255,
                specialization: u16::MAX,
            }],
            overrides: vec![ItemSpecOverrideRow {
                id: u32::MAX,
                specialization: u16::MAX,
                item: u32::MAX,
            }],
            gems: vec![GemPropertiesRow {
                id: u32::MAX,
                enchantment: u16::MAX,
                kind: i32::MIN,
            }],
        });
        assert_eq!(rows.specs[0].specialization, u16::MAX);
        assert_eq!(rows.specs[0].min_level, 255);
        assert_eq!(rows.overrides[0].item, u32::MAX);
        assert_eq!(rows.gems[0].kind, i32::MIN);
        assert_eq!(rows.gems[0].enchantment, u16::MAX);
    }
}

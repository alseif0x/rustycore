// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadVendors` represented data model.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use wow_database::{WorldDatabase, WorldStatements};

pub const ITEM_VENDOR_TYPE_ITEM_LIKE_CPP: u8 = 1;
pub const ITEM_VENDOR_TYPE_CURRENCY_LIKE_CPP: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorItemLikeCpp {
    pub item: u32,
    pub maxcount: u32,
    pub incrtime: u32,
    pub extended_cost: u32,
    pub vendor_type: u8,
    pub bonus_list_ids: Vec<i32>,
    pub player_condition_id: u32,
    pub ignore_filtering: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcVendorRowLikeCpp {
    pub entry: u32,
    pub item: i32,
    pub maxcount: u32,
    pub incrtime: u32,
    pub extended_cost: u32,
    pub vendor_type: u8,
    pub bonus_list_ids_raw: String,
    pub player_condition_id: u32,
    pub ignore_filtering: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VendorItemDataLikeCpp {
    items: Vec<VendorItemLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcVendorLoadReportLikeCpp {
    pub rows_seen: usize,
    pub reference_rows_seen: usize,
    pub loaded_items: usize,
    pub skipped_item_maxcount_without_incrtime: Vec<(u32, u32)>,
    pub skipped_item_incrtime_without_maxcount: Vec<(u32, u32)>,
    pub skipped_currency_without_maxcount: Vec<(u32, u32)>,
    pub skipped_duplicates: Vec<(u32, u32, u32, u8)>,
    pub skipped_reference_cycles: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Default)]
pub struct NpcVendorStoreLikeCpp {
    vendors: HashMap<u32, VendorItemDataLikeCpp>,
}

pub struct NpcVendorLoadOutcomeLikeCpp {
    pub store: NpcVendorStoreLikeCpp,
    pub report: NpcVendorLoadReportLikeCpp,
}

impl VendorItemDataLikeCpp {
    /// C++ `VendorItemData::GetItem`.
    pub fn get_item_like_cpp(&self, slot: u32) -> Option<&VendorItemLikeCpp> {
        self.items.get(slot as usize)
    }

    /// C++ `VendorItemData::FindItemCostPair`.
    pub fn find_item_cost_pair_like_cpp(
        &self,
        item: u32,
        extended_cost: u32,
        vendor_type: u8,
    ) -> Option<&VendorItemLikeCpp> {
        self.items.iter().find(|vendor_item| {
            vendor_item.item == item
                && vendor_item.extended_cost == extended_cost
                && vendor_item.vendor_type == vendor_type
        })
    }

    /// C++ `VendorItemData::AddItem`.
    pub fn add_item_like_cpp(&mut self, item: VendorItemLikeCpp) {
        self.items.push(item);
    }

    /// C++ `VendorItemData::RemoveItem`.
    pub fn remove_item_like_cpp(&mut self, item_id: u32, vendor_type: u8) -> bool {
        let old_len = self.items.len();
        self.items
            .retain(|item| item.item != item_id || item.vendor_type != vendor_type);
        old_len != self.items.len()
    }

    pub fn items_like_cpp(&self) -> &[VendorItemLikeCpp] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl NpcVendorStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = NpcVendorRowLikeCpp>,
    ) -> NpcVendorLoadOutcomeLikeCpp {
        let rows: Vec<NpcVendorRowLikeCpp> = rows.into_iter().collect();
        let mut rows_by_entry: HashMap<u32, Vec<usize>> = HashMap::new();
        for (index, row) in rows.iter().enumerate() {
            rows_by_entry.entry(row.entry).or_default().push(index);
        }

        let mut store = Self::default();
        let mut report = NpcVendorLoadReportLikeCpp {
            rows_seen: rows.len(),
            ..NpcVendorLoadReportLikeCpp::default()
        };

        for row in &rows {
            if row.item < 0 {
                report.reference_rows_seen += 1;
                store.load_reference_vendor_like_cpp(
                    row.entry,
                    row.item.unsigned_abs(),
                    &rows,
                    &rows_by_entry,
                    &mut report,
                    &mut HashSet::new(),
                );
                continue;
            }

            if let Some(vendor_item) = vendor_item_from_row_like_cpp(row) {
                store.add_validated_vendor_item_like_cpp(row.entry, vendor_item, &mut report);
            }
        }

        NpcVendorLoadOutcomeLikeCpp { store, report }
    }

    /// C++ `ObjectMgr::LoadVendors`.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<NpcVendorLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_NPC_VENDORS_ALL);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(NpcVendorRowLikeCpp {
                    entry: result.read(0),
                    item: result.read(1),
                    maxcount: result.read(2),
                    incrtime: result.read(3),
                    extended_cost: result.read(4),
                    vendor_type: result.read(5),
                    bonus_list_ids_raw: result.read_string(6),
                    player_condition_id: result.read(7),
                    ignore_filtering: result.try_read::<u8>(8).unwrap_or(0) != 0,
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows))
    }

    /// C++ `ObjectMgr::GetNpcVendorItemList`.
    pub fn get_npc_vendor_item_list_like_cpp(&self, entry: u32) -> Option<&VendorItemDataLikeCpp> {
        self.vendors.get(&entry)
    }

    /// C++ `ObjectMgr::AddVendorItem`.
    pub fn add_vendor_item_like_cpp(&mut self, entry: u32, item: VendorItemLikeCpp) {
        self.vendors
            .entry(entry)
            .or_default()
            .add_item_like_cpp(item);
    }

    /// C++ `ObjectMgr::RemoveVendorItem`.
    pub fn remove_vendor_item_like_cpp(&mut self, entry: u32, item: u32, vendor_type: u8) -> bool {
        let Some(items) = self.vendors.get_mut(&entry) else {
            return false;
        };
        items.remove_item_like_cpp(item, vendor_type)
    }

    pub fn len(&self) -> usize {
        self.vendors.len()
    }

    pub fn item_count_like_cpp(&self) -> usize {
        self.vendors.values().map(VendorItemDataLikeCpp::len).sum()
    }

    fn load_reference_vendor_like_cpp(
        &mut self,
        vendor_entry: u32,
        reference_entry: u32,
        rows: &[NpcVendorRowLikeCpp],
        rows_by_entry: &HashMap<u32, Vec<usize>>,
        report: &mut NpcVendorLoadReportLikeCpp,
        stack: &mut HashSet<u32>,
    ) {
        if !stack.insert(reference_entry) {
            report
                .skipped_reference_cycles
                .push((vendor_entry, reference_entry));
            return;
        }

        if let Some(row_indexes) = rows_by_entry.get(&reference_entry) {
            for row_index in row_indexes {
                let row = &rows[*row_index];
                if row.item < 0 {
                    report.reference_rows_seen += 1;
                    self.load_reference_vendor_like_cpp(
                        vendor_entry,
                        row.item.unsigned_abs(),
                        rows,
                        rows_by_entry,
                        report,
                        stack,
                    );
                    continue;
                }

                if let Some(vendor_item) = vendor_item_from_row_like_cpp(row) {
                    self.add_validated_vendor_item_like_cpp(vendor_entry, vendor_item, report);
                }
            }
        }

        stack.remove(&reference_entry);
    }

    fn add_validated_vendor_item_like_cpp(
        &mut self,
        entry: u32,
        item: VendorItemLikeCpp,
        report: &mut NpcVendorLoadReportLikeCpp,
    ) {
        if item.vendor_type == ITEM_VENDOR_TYPE_ITEM_LIKE_CPP {
            if item.maxcount > 0 && item.incrtime == 0 {
                report
                    .skipped_item_maxcount_without_incrtime
                    .push((entry, item.item));
                return;
            }
            if item.maxcount == 0 && item.incrtime > 0 {
                report
                    .skipped_item_incrtime_without_maxcount
                    .push((entry, item.item));
                return;
            }
        }

        if item.vendor_type == ITEM_VENDOR_TYPE_CURRENCY_LIKE_CPP && item.maxcount == 0 {
            report
                .skipped_currency_without_maxcount
                .push((entry, item.item));
            return;
        }

        if let Some(existing) = self.vendors.get(&entry) {
            if existing
                .find_item_cost_pair_like_cpp(item.item, item.extended_cost, item.vendor_type)
                .is_some()
            {
                report.skipped_duplicates.push((
                    entry,
                    item.item,
                    item.extended_cost,
                    item.vendor_type,
                ));
                return;
            }
        }

        self.add_vendor_item_like_cpp(entry, item);
        report.loaded_items += 1;
    }
}

fn vendor_item_from_row_like_cpp(row: &NpcVendorRowLikeCpp) -> Option<VendorItemLikeCpp> {
    Some(VendorItemLikeCpp {
        item: u32::try_from(row.item).ok()?,
        maxcount: row.maxcount,
        incrtime: row.incrtime,
        extended_cost: row.extended_cost,
        vendor_type: row.vendor_type,
        bonus_list_ids: parse_bonus_list_ids_like_cpp(&row.bonus_list_ids_raw),
        player_condition_id: row.player_condition_id,
        ignore_filtering: row.ignore_filtering,
    })
}

fn parse_bonus_list_ids_like_cpp(raw: &str) -> Vec<i32> {
    raw.split_whitespace()
        .filter_map(|token| token.parse::<i32>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(entry: u32, item: i32) -> NpcVendorRowLikeCpp {
        NpcVendorRowLikeCpp {
            entry,
            item,
            maxcount: 0,
            incrtime: 0,
            extended_cost: 0,
            vendor_type: ITEM_VENDOR_TYPE_ITEM_LIKE_CPP,
            bonus_list_ids_raw: String::new(),
            player_condition_id: 0,
            ignore_filtering: false,
        }
    }

    #[test]
    fn vendor_store_preserves_order_and_lookup_like_cpp() {
        let outcome = NpcVendorStoreLikeCpp::from_rows_like_cpp([
            row(10, 1000),
            row(10, 1001),
            row(11, 2000),
        ]);

        let vendor = outcome.store.get_npc_vendor_item_list_like_cpp(10).unwrap();
        assert_eq!(vendor.len(), 2);
        assert_eq!(vendor.get_item_like_cpp(0).unwrap().item, 1000);
        assert_eq!(vendor.get_item_like_cpp(1).unwrap().item, 1001);
        assert_eq!(outcome.report.loaded_items, 3);
    }

    #[test]
    fn vendor_store_expands_negative_reference_rows_like_cpp() {
        let outcome =
            NpcVendorStoreLikeCpp::from_rows_like_cpp([row(10, -20), row(20, 2000), row(20, 2001)]);

        let vendor = outcome.store.get_npc_vendor_item_list_like_cpp(10).unwrap();
        assert_eq!(
            vendor
                .items_like_cpp()
                .iter()
                .map(|item| item.item)
                .collect::<Vec<_>>(),
            vec![2000, 2001]
        );
        assert_eq!(outcome.report.reference_rows_seen, 1);
        assert_eq!(outcome.report.loaded_items, 4);
    }

    #[test]
    fn vendor_store_skips_duplicate_item_cost_type_like_cpp() {
        let mut duplicate = row(10, 1000);
        duplicate.extended_cost = 5;

        let mut original = row(10, 1000);
        original.extended_cost = 5;

        let outcome = NpcVendorStoreLikeCpp::from_rows_like_cpp([original, duplicate]);

        assert_eq!(outcome.report.loaded_items, 1);
        assert_eq!(
            outcome.report.skipped_duplicates,
            vec![(10, 1000, 5, ITEM_VENDOR_TYPE_ITEM_LIKE_CPP)]
        );
    }

    #[test]
    fn vendor_store_skips_item_stock_mismatches_like_cpp() {
        let mut max_without_incr = row(10, 1000);
        max_without_incr.maxcount = 5;

        let mut incr_without_max = row(10, 1001);
        incr_without_max.incrtime = 60;

        let outcome =
            NpcVendorStoreLikeCpp::from_rows_like_cpp([max_without_incr, incr_without_max]);

        assert_eq!(outcome.report.loaded_items, 0);
        assert_eq!(
            outcome.report.skipped_item_maxcount_without_incrtime,
            vec![(10, 1000)]
        );
        assert_eq!(
            outcome.report.skipped_item_incrtime_without_maxcount,
            vec![(10, 1001)]
        );
    }

    #[test]
    fn vendor_store_skips_currency_without_maxcount_like_cpp() {
        let mut currency = row(10, 395);
        currency.vendor_type = ITEM_VENDOR_TYPE_CURRENCY_LIKE_CPP;

        let outcome = NpcVendorStoreLikeCpp::from_rows_like_cpp([currency]);

        assert_eq!(outcome.report.loaded_items, 0);
        assert_eq!(
            outcome.report.skipped_currency_without_maxcount,
            vec![(10, 395)]
        );
    }

    #[test]
    fn vendor_bonus_list_ids_parse_like_cpp() {
        assert_eq!(
            parse_bonus_list_ids_like_cpp("7 bad -9 7 0x10 12"),
            vec![7, -9, 7, 12]
        );
    }
}

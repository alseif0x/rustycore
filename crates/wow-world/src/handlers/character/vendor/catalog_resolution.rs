//! catalog resolution for the existing vendor owner.

use super::*;

impl WorldSession {
}

impl WorldSession {
    pub(super) async fn resolve_vendor_buy_item_by_cpp_slot(
        &self,
        port: Option<&dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>,
        root_entry: u32,
        vendor_slot: u32,
        expected_item_id: u32,
    ) -> Option<VendorBuyItem> {
        #[cfg(test)]
        if let Some(item) = self.vendor_buy_item_test_override_like_cpp() {
            if vendor_slot != 0 || item.item_id != expected_item_id {
                return None;
            }
            return Some(VendorBuyItem {
                item_id: item.item_id,
                item_type: item.item_type,
                max_count: item.max_count,
                incr_time: item.incr_time,
                player_condition_id: item.player_condition_id,
                has_vendor_conditions: item.has_vendor_conditions,
                extended_cost: item.extended_cost,
                buy_price: item.buy_price,
                max_durability: item.max_durability,
                buy_count: item.buy_count,
            });
        }
        let port = port?;
        let mut raw_slot = 0u32;
        let mut expanded = std::collections::HashSet::<u32>::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(root_entry);
        while let Some(vendor_entry) = queue.pop_front() {
            if !expanded.insert(vendor_entry) {
                continue;
            }
            let rows = match port
                .load_vendor_rows_like_cpp(root_entry, vendor_entry)
                .await
            {
                wow_persistence::VendorCatalogOutcomeLikeCpp::Loaded(rows) => rows,
                wow_persistence::VendorCatalogOutcomeLikeCpp::Missing => Vec::new(),
                wow_persistence::VendorCatalogOutcomeLikeCpp::Failed { reason } => {
                    warn!("BuyItem: vendor item query failed for entry {vendor_entry}: {reason}");
                    continue;
                }
            };
            for row in rows {
                let item_id = row.item_id;
                if item_id > 0 {
                    let current_slot = raw_slot;
                    raw_slot = raw_slot.saturating_add(1);
                    let item_type = i32::from(row.item_type);
                    let item_known = self
                        .item_store()
                        .map_or(true, |store| store.get(item_id as u32).is_some());
                    let currency_known = item_type == ItemVendorType::Currency as i32
                        && vendor_currency_type_is_known(
                            self.currency_types_store().map(|store| store.as_ref()),
                            item_id as u32,
                        );
                    if (item_known || currency_known) && current_slot == vendor_slot {
                        let row_item_id = item_id as u32;
                        if row_item_id != expected_item_id {
                            return None;
                        }
                        return Some(VendorBuyItem {
                            item_id: row_item_id,
                            item_type,
                            max_count: row.max_count.max(0) as u32,
                            incr_time: row.incr_time,
                            player_condition_id: row.player_condition_id,
                            has_vendor_conditions: row.has_vendor_conditions,
                            extended_cost: row.extended_cost,
                            buy_price: row.buy_price,
                            max_durability: row.max_durability,
                            buy_count: row.buy_count,
                        });
                    }
                } else if item_id < 0 {
                    queue.push_back((-item_id) as u32);
                }
            }
        }
        None
    }
}

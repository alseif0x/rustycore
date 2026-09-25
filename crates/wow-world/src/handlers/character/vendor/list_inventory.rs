// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vendor inventory listing, reference expansion, filtering, and publication.

use super::rules::{
    vendor_list_item_refundable, vendor_list_reaches_cpp_item_limit,
    vendor_list_should_skip_allowed_class, vendor_list_should_skip_currency_row,
    vendor_list_should_skip_faction_flags, vendor_list_should_skip_sold_out,
    vendor_player_condition_failed_id_like_cpp,
};
use super::*;

impl WorldSession {
    /// Handle CMSG_LIST_INVENTORY — player opens vendor window.
    ///
    /// Queries npc_vendor for the creature's items (including reference vendors, item_id < 0)
    /// and sends SMSG_VENDOR_INVENTORY. Entry is resolved from the visibility tracker or,
    /// if missing, from world.creature by GUID (fallback when NPC not in tracker).
    pub async fn handle_list_inventory(&mut self, hello: Hello) {
        let vendor_guid = hello.unit;
        info!(
            "ListInventory for {:?} from account {}",
            vendor_guid, self.account_id
        );

        let vendor_catalog = match self.vendor_catalog_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        // Resolve creature entry: first from map-owned creature state, then fallback from DB by spawn GUID.
        let entry = match self.mutate_world_creature(vendor_guid, |creature| {
            creature.pause_interaction_movement_like_cpp();
            creature.entry()
        }) {
            Some(entry) => entry,
            None => {
                let fallback = match tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    vendor_catalog
                        .load_creature_entry_by_spawn_like_cpp(vendor_guid.low_value() as u64),
                )
                .await
                {
                    Ok(wow_persistence::VendorCatalogOutcomeLikeCpp::Loaded(entry)) => Some(entry),
                    _ => None,
                };
                match fallback {
                    Some(e) => {
                        info!("Vendor entry {} resolved from DB (GUID not in tracker)", e);
                        e
                    }
                    None => {
                        info!(
                            "Vendor GUID {:?} not in tracker and not found in creature table",
                            vendor_guid
                        );
                        self.send_packet(&VendorInventory {
                            vendor_guid,
                            reason: 0,
                            items: vec![],
                        });
                        return;
                    }
                }
            }
        };

        // Load all items: direct rows + expand reference vendors (npc_vendor.item < 0).
        let mut items = Vec::new();
        let mut raw_slot = 0i32;
        let mut expanded = std::collections::HashSet::<u32>::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(entry);
        let condition_store = self.condition_store().cloned();
        let player_condition_store = self.player_condition_store().cloned();
        let Some(player_condition_context) = self.represented_player_condition_context_like_cpp()
        else {
            return;
        };
        let player_condition_object = self.build_condition_player_object_like_cpp();
        let vendor_condition_object = self.build_condition_creature_object_like_cpp(vendor_guid);
        let Some(player_unit_snapshot) = self.condition_player_unit_snapshot_like_cpp() else {
            self.send_packet(&VendorInventory {
                vendor_guid,
                reason: 0,
                items: vec![],
            });
            return;
        };
        let player_snapshot = self.condition_player_snapshot_like_cpp();

        'vendor_expansion: while let Some(vendor_entry) = queue.pop_front() {
            if !expanded.insert(vendor_entry) {
                continue; // already expanded (avoid cycles)
            }
            let rows = match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                vendor_catalog.load_vendor_rows_like_cpp(entry, vendor_entry),
            )
            .await
            {
                Ok(wow_persistence::VendorCatalogOutcomeLikeCpp::Loaded(rows)) => rows,
                Ok(wow_persistence::VendorCatalogOutcomeLikeCpp::Missing) => Vec::new(),
                Ok(wow_persistence::VendorCatalogOutcomeLikeCpp::Failed { reason }) => {
                    warn!("Vendor query failed for entry {vendor_entry}: {reason}");
                    continue;
                }
                Err(_) => {
                    warn!("Vendor query timed out for entry {vendor_entry}");
                    continue;
                }
            };

            for row in rows {
                let item_id = row.item_id;
                let maxcount = row.max_count;
                let extended_cost = row.extended_cost as i32;
                let item_type = i32::from(row.item_type);
                let buy_price = row.buy_price;
                let durability = row.max_durability as i32;
                let stack_count = row.buy_count as i32;
                let do_not_filter = row.do_not_filter;
                let incr_time = row.incr_time;
                let player_condition_id = row.player_condition_id;
                let has_vendor_conditions = row.has_vendor_conditions;

                // Only send items with a valid ID; the client renders zero or negative IDs as "?".
                // Also filter items missing from this client's Item.db2, matching C++
                // `SendListInventory` item-template validation
                // (`Handlers/ItemHandler.cpp:617-626`). Entries such as 58260 and 58274 are omitted.
                if item_id > 0 {
                    let muid = raw_slot.saturating_add(1);
                    raw_slot = raw_slot.saturating_add(1);
                    if item_type == ItemVendorType::Currency as i32 {
                        if vendor_list_should_skip_currency_row(
                            self.currency_types_store().map(|store| store.as_ref()),
                            item_id,
                            extended_cost,
                        ) {
                            continue;
                        }
                        items.push(VendorItem {
                            muid,
                            item_id,
                            item_type,
                            quantity: 0,
                            price: 0,
                            durability: 0,
                            stack_count: maxcount,
                            extended_cost,
                            player_condition_failed: vendor_player_condition_failed_id_like_cpp(
                                player_condition_id,
                                player_condition_store.as_deref(),
                                player_condition_context.as_context(self),
                            ),
                            locked: false,
                            do_not_filter,
                            refundable: false,
                        });
                        if vendor_list_reaches_cpp_item_limit(items.len()) {
                            break 'vendor_expansion;
                        }
                        continue;
                    }
                    let item_known = self
                        .item_store()
                        .map_or(true, |s| s.get(item_id as u32).is_some());
                    if !item_known {
                        info!(
                            "Vendor item {} not in Item.db2 (entry {}), skipping",
                            item_id, vendor_entry
                        );
                        continue;
                    }
                    let current_count = self.vendor_item_current_count(
                        vendor_guid,
                        item_id as u32,
                        maxcount.max(0) as u32,
                        incr_time,
                        stack_count.max(1) as u32,
                    );
                    if vendor_list_should_skip_sold_out(maxcount, current_count, self.security > 0)
                    {
                        continue;
                    }
                    let template = self.item_storage_template(item_id as u32);
                    let sparse_template = self
                        .item_stats_store()
                        .and_then(|store| store.sparse_template(item_id as u32));
                    if vendor_list_should_skip_allowed_class(
                        sparse_template.map(|template| template.allowable_class),
                        sparse_template.map(|template| template.bonding),
                        self.player_class_like_cpp(),
                        self.security > 0,
                    ) {
                        continue;
                    }
                    if vendor_list_should_skip_faction_flags(
                        sparse_template.map(|template| template.flags[1]),
                        player_team_for_race_cpp(self.player_race_like_cpp()),
                        self.security > 0,
                    ) {
                        continue;
                    }
                    if has_vendor_conditions {
                        let Some(store) = condition_store.as_ref() else {
                            continue;
                        };
                        let (vendor_object, vendor_unit_snapshot) = vendor_condition_object
                            .as_ref()
                            .map(|(object, snapshot)| (Some(object), Some(*snapshot)))
                            .unwrap_or((None, None));
                        if !wow_conditions::is_vendor_item_conditions_with_snapshots_like_cpp(
                            store.as_ref(),
                            entry,
                            item_id as u32,
                            player_condition_object.as_ref(),
                            vendor_object,
                            player_unit_snapshot,
                            player_snapshot,
                            vendor_unit_snapshot,
                            player_condition_store.as_deref(),
                            player_condition_context.as_context(self),
                        ) {
                            warn!(
                                "Vendor item condition not met for creature entry {} item {}",
                                entry, item_id
                            );
                            continue;
                        }
                    }
                    let refundable = vendor_list_item_refundable(
                        template.as_ref().map(|template| template.flags),
                        template.as_ref().map(|template| template.max_stack_size),
                        extended_cost,
                    );
                    items.push(VendorItem {
                        muid,
                        item_id,
                        item_type,
                        quantity: if maxcount == 0 {
                            -1
                        } else {
                            current_count as i32
                        },
                        price: buy_price,
                        durability,
                        stack_count: stack_count.max(1),
                        extended_cost,
                        player_condition_failed: vendor_player_condition_failed_id_like_cpp(
                            player_condition_id,
                            player_condition_store.as_deref(),
                            player_condition_context.as_context(self),
                        ),
                        locked: false,
                        do_not_filter,
                        refundable,
                    });
                    if vendor_list_reaches_cpp_item_limit(items.len()) {
                        break 'vendor_expansion;
                    }
                } else if item_id < 0 {
                    let ref_entry = (-item_id) as u32;
                    queue.push_back(ref_entry);
                }
            }
        }

        let item_ids: Vec<i32> = items.iter().map(|i| i.item_id).collect();
        info!(
            "Sending vendor inventory: {} items for entry {} (item_ids: {:?})",
            items.len(),
            entry,
            item_ids
        );
        self.send_packet(&VendorInventory {
            vendor_guid,
            reason: 0,
            items,
        });
    }
}

//! repair cost for the existing durability owner.

use super::*;

impl WorldSession {
    /// C++ `Item::CalculateDurabilityRepairCost`.
    pub(crate) fn item_durability_repair_cost_like_cpp(
        &self,
        item_id: u32,
        current_durability: u32,
        max_durability: u32,
        discount: f32,
        repair_cost_rate: f32,
    ) -> u64 {
        if max_durability == 0 {
            return 0;
        }

        debug_assert!(
            max_durability >= current_durability,
            "C++ Item::CalculateDurabilityRepairCost asserts max durability >= current durability"
        );
        if current_durability >= max_durability {
            return 0;
        }

        let item = match self
            .items
            .store
            .as_ref()
            .and_then(|store| store.get(item_id))
        {
            Some(item) => item,
            None => return 0,
        };
        let stats = match self
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.random_property_template(item_id))
        {
            Some(stats) => stats,
            None => return 0,
        };
        if stats.quality < 0 {
            return 0;
        }

        let durability_cost = match self
            .items
            .durability_costs_store()
            .and_then(|store| store.get(u32::from(stats.item_level)))
        {
            Some(cost) => cost,
            None => return 0,
        };
        let durability_quality_entry_id = (stats.quality as u32 + 1) * 2;
        let durability_quality = match self
            .items
            .durability_quality_store()
            .and_then(|store| store.get(durability_quality_entry_id))
        {
            Some(quality) => quality,
            None => return 0,
        };

        let subclass = item.subclass_id as usize;
        let multiplier = if item.class_id == ItemClass::Weapon as u8 {
            durability_cost
                .weapon_sub_class_cost
                .get(subclass)
                .copied()
                .unwrap_or(0)
        } else if item.class_id == ItemClass::Armor as u8 {
            durability_cost
                .armor_sub_class_cost
                .get(subclass)
                .copied()
                .unwrap_or(0)
        } else {
            0
        };

        let lost_durability = max_durability - current_durability;
        let rounded =
            (lost_durability as f32 * multiplier as f32 * durability_quality.data * 1.0f32).round();
        let cost = (rounded * discount * repair_cost_rate) as u64;

        if cost == 0 { 1 } else { cost }
    }
    pub(in crate::session) fn repairable_inventory_item_costs_like_cpp(
        &self,
        discount: f32,
        repair_cost_rate: f32,
    ) -> Option<Vec<(ObjectGuid, u64)>> {
        let mut repair_items = Vec::new();
        let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(INVENTORY_DEFAULT_SIZE)
            .min(PLAYER_SLOT_END as u8);

        for (&slot, inventory_item) in &inventory_items {
            if !((slot < EQUIPMENT_SLOT_END)
                || (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
                || (INVENTORY_SLOT_ITEM_START..inventory_end).contains(&slot))
            {
                continue;
            }
            let Some(item_object) = item_objects.get(&inventory_item.guid) else {
                continue;
            };
            let cost = self.item_durability_repair_cost_like_cpp(
                inventory_item.entry_id,
                item_object.data().durability,
                item_object.data().max_durability,
                discount,
                repair_cost_rate,
            );
            if cost != 0 {
                repair_items.push((inventory_item.guid, cost));
            }
        }

        let represented_bag_guids: HashSet<_> = inventory_items
            .iter()
            .filter_map(|(&slot, item)| {
                ((INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot))
                    .then_some(item.guid)
            })
            .collect();
        for item_object in item_objects.values() {
            if !represented_bag_guids.contains(&item_object.container_guid())
                || item_object.slot() as usize >= MAX_BAG_SIZE
            {
                continue;
            }
            let cost = self.item_durability_repair_cost_like_cpp(
                item_object.object().entry(),
                item_object.data().durability,
                item_object.data().max_durability,
                discount,
                repair_cost_rate,
            );
            if cost != 0 {
                repair_items.push((item_object.object().guid(), cost));
            }
        }

        Some(repair_items)
    }
}

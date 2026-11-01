use std::collections::HashMap;

use super::super::*;
use crate::PlayerCurrencyState;
use wow_constants::ItemUpdateState;
use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellCastItemDebitActionLikeCpp {
    item_guid: ObjectGuid,
    entry_id: u32,
    bag: u8,
    slot: u8,
    previous_count: u32,
    removed_count: u32,
    remaining_count: u32,
    destroy_stack: bool,
}

impl PlayerSpellCastItemDebitActionLikeCpp {
    pub fn item_guid(self) -> ObjectGuid {
        self.item_guid
    }
    pub fn entry_id(self) -> u32 {
        self.entry_id
    }
    pub fn bag(self) -> u8 {
        self.bag
    }
    pub fn slot(self) -> u8 {
        self.slot
    }
    pub fn removed_count(self) -> u32 {
        self.removed_count
    }
    pub fn remaining_count(self) -> u32 {
        self.remaining_count
    }
    pub fn destroys_stack(self) -> bool {
        self.destroy_stack
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellCastCurrencyDebitActionLikeCpp {
    currency_id: u32,
    amount: u32,
    previous_quantity: u32,
    remaining_quantity: u32,
}

impl PlayerSpellCastCurrencyDebitActionLikeCpp {
    pub fn currency_id(self) -> u32 {
        self.currency_id
    }
    pub fn amount(self) -> u32 {
        self.amount
    }
    pub fn remaining_quantity(self) -> u32 {
        self.remaining_quantity
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerSpellCastResourceDebitPlanLikeCpp {
    item_actions: Vec<PlayerSpellCastItemDebitActionLikeCpp>,
    currency_actions: Vec<PlayerSpellCastCurrencyDebitActionLikeCpp>,
}

impl PlayerSpellCastResourceDebitPlanLikeCpp {
    pub fn item_actions(&self) -> &[PlayerSpellCastItemDebitActionLikeCpp] {
        &self.item_actions
    }
    pub fn currency_actions(&self) -> &[PlayerSpellCastCurrencyDebitActionLikeCpp] {
        &self.currency_actions
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerSpellCastResourceDebitOutcomeLikeCpp {
    pub item_actions: Vec<PlayerSpellCastItemDebitActionLikeCpp>,
    pub currency_actions: Vec<PlayerSpellCastCurrencyDebitActionLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellCastItemUseOutcomeLikeCpp {
    pub item_guid: ObjectGuid,
    pub entry_id: u32,
    pub bag: u8,
    pub slot: u8,
    pub removed_one: bool,
    pub removed_stack: bool,
    pub remaining_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellCastItemUsePlanLikeCpp {
    pub item_guid: ObjectGuid,
    pub entry_id: u32,
    pub expected_charges: [i32; crate::MAX_ITEM_SPELLS],
    pub updated_charges: [i32; crate::MAX_ITEM_SPELLS],
    pub writes_charges: bool,
    pub marks_item_changed: bool,
    pub consumes_one_item: bool,
}

#[derive(Debug, Clone, Copy)]
struct SpellCostItemLikeCpp {
    guid: ObjectGuid,
    entry_id: u32,
    bag: u8,
    slot: u8,
    count: u32,
    in_trade: bool,
}

impl Player {
    pub fn spell_cast_item_position_like_cpp(&self, item_guid: ObjectGuid) -> Option<(u8, u8)> {
        self.spell_cost_items_in_destroy_order_like_cpp()?
            .into_iter()
            .find(|item| item.guid == item_guid)
            .map(|item| (item.bag, item.slot))
    }

    /// Plan C++ `DestroyItemCount` reagent removal and spell currency costs
    /// against the current canonical Player owners. The plan is immutable and
    /// can be checked before the preceding power debit; applying it verifies
    /// the same physical slots and quantities again.
    pub fn plan_spell_cast_resource_debits_like_cpp(
        &self,
        reagent_debits: &[(u32, u32)],
        currency_debits: &[(u32, u32)],
    ) -> Option<PlayerSpellCastResourceDebitPlanLikeCpp> {
        if !reagent_debits.is_empty()
            && !self
                .inventory_runtime_like_cpp()
                .equipment_inventory_authority_complete_like_cpp()
        {
            return None;
        }
        if !currency_debits.is_empty() && !self.gameplay_state().currencies_loaded_like_cpp {
            return None;
        }

        let item_counts = if reagent_debits.is_empty() {
            HashMap::new()
        } else {
            self.item_counts_for_location_like_cpp(
                ItemSearchLocation::EQUIPMENT
                    | ItemSearchLocation::INVENTORY
                    | ItemSearchLocation::REAGENT_BANK,
            )?
        };
        for &(entry_id, count) in reagent_debits {
            if item_counts.get(&entry_id).copied().unwrap_or(0) < count {
                return None;
            }
        }

        let mut items = if reagent_debits.is_empty() {
            Vec::new()
        } else {
            self.spell_cost_items_in_destroy_order_like_cpp()?
        };
        let mut plan = PlayerSpellCastResourceDebitPlanLikeCpp::default();
        for &(entry_id, mut remaining) in reagent_debits {
            if remaining == 0 {
                continue;
            }
            for item in &mut items {
                if remaining == 0 || item.entry_id != entry_id || item.in_trade || item.count == 0 {
                    continue;
                }
                let removed_count = remaining.min(item.count);
                let remaining_count = item.count - removed_count;
                plan.item_actions
                    .push(PlayerSpellCastItemDebitActionLikeCpp {
                        item_guid: item.guid,
                        entry_id,
                        bag: item.bag,
                        slot: item.slot,
                        previous_count: item.count,
                        removed_count,
                        remaining_count,
                        destroy_stack: remaining_count == 0,
                    });
                item.count = remaining_count;
                remaining -= removed_count;
            }
            if remaining != 0 {
                return None;
            }
        }

        let mut currency_quantities = self
            .gameplay_state()
            .currencies
            .iter()
            .map(|(&id, currency)| (id, currency.quantity))
            .collect::<HashMap<_, _>>();
        for &(currency_id, amount) in currency_debits {
            if amount == 0 {
                continue;
            }
            if !currency_quantities.contains_key(&currency_id) {
                return None;
            }
            let quantity = currency_quantities.get(&currency_id).copied().unwrap_or(0);
            if quantity < amount {
                return None;
            }
            let removed = amount;
            let remaining_quantity = quantity - removed;
            plan.currency_actions
                .push(PlayerSpellCastCurrencyDebitActionLikeCpp {
                    currency_id,
                    amount: removed,
                    previous_quantity: quantity,
                    remaining_quantity,
                });
            currency_quantities.insert(currency_id, remaining_quantity);
        }

        Some(plan)
    }

    /// Plan only the next physical stack mutation for one source reagent row.
    /// The caller repeats this after each quest/item callback so each step sees
    /// current inventory state and preserves source row/slot order.
    pub fn plan_spell_cast_reagent_step_like_cpp(
        &self,
        entry_id: u32,
        remaining_count: u32,
    ) -> Option<PlayerSpellCastItemDebitActionLikeCpp> {
        if remaining_count == 0 {
            return None;
        }
        self.plan_spell_cast_resource_debits_like_cpp(&[(entry_id, remaining_count)], &[])?
            .item_actions
            .into_iter()
            .next()
    }

    pub fn plan_spell_cast_currency_debit_like_cpp(
        &self,
        currency_id: u32,
        amount: u32,
    ) -> Option<PlayerSpellCastCurrencyDebitActionLikeCpp> {
        self.plan_spell_cast_resource_debits_like_cpp(&[], &[(currency_id, amount)])?
            .currency_actions
            .into_iter()
            .next()
    }

    /// Apply one owner-created reagent step after revalidating its complete
    /// physical identity and count relationship before changing either owner.
    pub fn apply_spell_cast_reagent_step_like_cpp(
        &mut self,
        action: PlayerSpellCastItemDebitActionLikeCpp,
    ) -> Option<()> {
        if !self
            .inventory_runtime_like_cpp()
            .equipment_inventory_authority_complete_like_cpp()
            || action.removed_count == 0
            || action.previous_count == 0
            || action.removed_count > action.previous_count
            || action.remaining_count != action.previous_count - action.removed_count
            || action.destroy_stack != (action.remaining_count == 0)
            || self.get_item_by_pos(action.bag, action.slot) != Some(action.item_guid)
        {
            return None;
        }
        let current = self
            .inventory_runtime_like_cpp()
            .item_objects()
            .get(&action.item_guid)?;
        if current.owner_guid() != self.guid()
            || current.object().entry() != action.entry_id
            || current.count() != action.previous_count
            || current.is_in_trade()
            || current.update_state() == ItemUpdateState::Removed
        {
            return None;
        }

        if action.destroy_stack {
            if action.bag == INVENTORY_SLOT_BAG_0 {
                self.remove_top_level_item(action.slot).ok()?;
            } else {
                self.remove_bag_item(action.bag, action.slot).ok()?;
            }
            self.soulbound_tradeable_items.remove(&action.item_guid);
            let item = self
                .inventory_runtime_mut_like_cpp()
                .item_objects_mut()
                .get_mut(&action.item_guid)?;
            item.set_not_refundable();
            item.clear_soulbound_tradeable();
            item.set_contained_in(ObjectGuid::EMPTY);
            item.set_slot(NULL_SLOT);
            item.set_container_guid(ObjectGuid::EMPTY);
            // A New item normally means "pretend it never existed" when
            // removed. Full-save capture may already have observed that New
            // row, so this consumer must retain a delete tombstone for the
            // later acknowledged save.
            item.force_state(ItemUpdateState::Removed);
        } else {
            let item = self
                .inventory_runtime_mut_like_cpp()
                .item_objects_mut()
                .get_mut(&action.item_guid)?;
            item.set_count(action.remaining_count);
            if item.update_state() != ItemUpdateState::New {
                item.set_state(ItemUpdateState::Changed);
            }
        }
        Some(())
    }

    pub fn apply_spell_cast_currency_debit_like_cpp(
        &mut self,
        action: PlayerSpellCastCurrencyDebitActionLikeCpp,
    ) -> Option<()> {
        if !self.gameplay_state().currencies_loaded_like_cpp
            || action.amount == 0
            || action.amount > action.previous_quantity
            || action.remaining_quantity != action.previous_quantity - action.amount
        {
            return None;
        }
        let currency = self
            .gameplay_state_mut()
            .currencies
            .get_mut(&action.currency_id)?;
        if currency.quantity != action.previous_quantity {
            return None;
        }
        currency.quantity = action.remaining_quantity;
        if currency.state != PlayerCurrencyState::New {
            currency.state = PlayerCurrencyState::Changed;
        }
        Some(())
    }

    /// Apply an immutable pre-GO spell resource plan without an intervening
    /// database operation. Removed persisted items stay as canonical dirty
    /// tombstones until the existing full-save acknowledgement retires them.
    pub fn apply_spell_cast_resource_debits_like_cpp(
        &mut self,
        plan: &PlayerSpellCastResourceDebitPlanLikeCpp,
    ) -> Option<PlayerSpellCastResourceDebitOutcomeLikeCpp> {
        if !plan.item_actions.is_empty()
            && !self
                .inventory_runtime_like_cpp()
                .equipment_inventory_authority_complete_like_cpp()
        {
            return None;
        }
        if !plan.currency_actions.is_empty() && !self.gameplay_state().currencies_loaded_like_cpp {
            return None;
        }

        let mut simulated_items = HashMap::<(u8, u8), (ObjectGuid, u32)>::new();
        for action in &plan.item_actions {
            if action.removed_count == 0
                || action.previous_count == 0
                || action.removed_count > action.previous_count
                || action.remaining_count != action.previous_count - action.removed_count
                || action.destroy_stack != (action.remaining_count == 0)
            {
                return None;
            }
            let state = simulated_items
                .entry((action.bag, action.slot))
                .or_insert_with(|| {
                    let guid = self
                        .get_item_by_pos(action.bag, action.slot)
                        .unwrap_or(ObjectGuid::EMPTY);
                    let count = self
                        .inventory_runtime_like_cpp()
                        .item_objects()
                        .get(&guid)
                        .map_or(0, Item::count);
                    (guid, count)
                });
            if state.0 != action.item_guid || state.1 != action.previous_count {
                return None;
            }
            let item = self
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&action.item_guid)?;
            if item.owner_guid() != self.guid()
                || item.object().entry() != action.entry_id
                || item.is_in_trade()
                || item.update_state() == ItemUpdateState::Removed
            {
                return None;
            }
            state.1 = action.remaining_count;
        }

        let mut simulated_currencies = HashMap::<u32, u32>::new();
        for action in &plan.currency_actions {
            if action.amount == 0
                || action.amount > action.previous_quantity
                || action.remaining_quantity != action.previous_quantity - action.amount
            {
                return None;
            }
            let quantity = simulated_currencies
                .entry(action.currency_id)
                .or_insert_with(|| {
                    self.gameplay_state()
                        .currencies
                        .get(&action.currency_id)
                        .map_or(0, |currency| currency.quantity)
                });
            if *quantity != action.previous_quantity {
                return None;
            }
            if !self
                .gameplay_state()
                .currencies
                .contains_key(&action.currency_id)
            {
                return None;
            }
            *quantity = action.remaining_quantity;
        }

        for action in &plan.item_actions {
            let guid = self.get_item_by_pos(action.bag, action.slot)?;
            if guid != action.item_guid {
                return None;
            }
            if action.destroy_stack {
                self.soulbound_tradeable_items.remove(&guid);
                if action.bag == INVENTORY_SLOT_BAG_0 {
                    self.remove_top_level_item(action.slot).ok()?;
                } else {
                    self.remove_bag_item(action.bag, action.slot).ok()?;
                }
                let runtime = self.inventory_runtime_mut_like_cpp();
                let item = runtime.item_objects_mut().get_mut(&guid)?;
                item.set_not_refundable();
                item.clear_soulbound_tradeable();
                item.set_contained_in(ObjectGuid::EMPTY);
                item.set_slot(NULL_SLOT);
                item.set_container_guid(ObjectGuid::EMPTY);
                item.force_state(ItemUpdateState::Removed);
            } else {
                let item = self
                    .inventory_runtime_mut_like_cpp()
                    .item_objects_mut()
                    .get_mut(&guid)?;
                item.set_count(action.remaining_count);
                if item.update_state() != ItemUpdateState::New {
                    item.set_state(ItemUpdateState::Changed);
                }
            }
        }

        for action in &plan.currency_actions {
            let Some(currency) = self
                .gameplay_state_mut()
                .currencies
                .get_mut(&action.currency_id)
            else {
                return None;
            };
            if currency.quantity != action.previous_quantity {
                return None;
            }
            currency.quantity = action.remaining_quantity;
            if currency.state != PlayerCurrencyState::New {
                currency.state = PlayerCurrencyState::Changed;
            }
        }

        Some(PlayerSpellCastResourceDebitOutcomeLikeCpp {
            item_actions: plan.item_actions.clone(),
            currency_actions: plan.currency_actions.clone(),
        })
    }

    /// Apply C++ `Spell::TakeCastItem` to the exact retained physical item.
    /// Charge writes and removal remain on the canonical Item and inventory
    /// owners, so the existing full-save capture sees both outcomes.
    pub fn apply_spell_cast_item_use_like_cpp(
        &mut self,
        plan: PlayerSpellCastItemUsePlanLikeCpp,
    ) -> Option<PlayerSpellCastItemUseOutcomeLikeCpp> {
        if !self
            .inventory_runtime_like_cpp()
            .equipment_inventory_authority_complete_like_cpp()
        {
            return None;
        }
        let position = self
            .spell_cost_items_in_destroy_order_like_cpp()?
            .into_iter()
            .find(|item| item.guid == plan.item_guid)?;
        let current = self
            .inventory_runtime_like_cpp()
            .item_objects()
            .get(&plan.item_guid)?;
        if current.owner_guid() != self.guid()
            || current.object().entry() != plan.entry_id
            || current.update_state() == ItemUpdateState::Removed
            || position.in_trade
            || current.data().spell_charges != plan.expected_charges
            || current.count() == 0
        {
            return None;
        }
        let (bag, slot, count) = (position.bag, position.slot, current.count());
        let removed_one = plan.consumes_one_item;
        let removed_stack = removed_one && count == 1;
        let remaining_count = if removed_one {
            count.saturating_sub(1)
        } else {
            count
        };

        if removed_stack {
            self.soulbound_tradeable_items.remove(&plan.item_guid);
            if bag == INVENTORY_SLOT_BAG_0 {
                self.remove_top_level_item(slot).ok()?;
            } else {
                self.remove_bag_item(bag, slot).ok()?;
            }
            let runtime = self.inventory_runtime_mut_like_cpp();
            let item = runtime.item_objects_mut().get_mut(&plan.item_guid)?;
            item.set_not_refundable();
            item.clear_soulbound_tradeable();
            item.set_contained_in(ObjectGuid::EMPTY);
            item.set_slot(NULL_SLOT);
            item.set_container_guid(ObjectGuid::EMPTY);
            item.force_state(ItemUpdateState::Removed);
        } else {
            let item = self
                .inventory_runtime_mut_like_cpp()
                .item_objects_mut()
                .get_mut(&plan.item_guid)?;
            if plan.writes_charges {
                for (index, (&before, &after)) in plan
                    .expected_charges
                    .iter()
                    .zip(plan.updated_charges.iter())
                    .enumerate()
                {
                    if before != after {
                        item.set_spell_charges(index, after);
                    }
                }
            }
            if plan.marks_item_changed || removed_one {
                if item.update_state() != ItemUpdateState::New {
                    item.set_state(ItemUpdateState::Changed);
                }
            }
            if removed_one {
                item.set_count(remaining_count);
                if item.update_state() != ItemUpdateState::New {
                    item.set_state(ItemUpdateState::Changed);
                }
            }
        }

        Some(PlayerSpellCastItemUseOutcomeLikeCpp {
            item_guid: plan.item_guid,
            entry_id: plan.entry_id,
            bag,
            slot,
            removed_one,
            removed_stack,
            remaining_count,
        })
    }

    fn spell_cost_items_in_destroy_order_like_cpp(&self) -> Option<Vec<SpellCostItemLikeCpp>> {
        let mut positions = Vec::new();
        let backpack_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(self.active_data.num_backpack_slots)
            .min(INVENTORY_SLOT_ITEM_END);
        positions.extend(
            (INVENTORY_SLOT_ITEM_START..backpack_end).map(|slot| (INVENTORY_SLOT_BAG_0, slot)),
        );
        positions.extend(
            (KEYRING_SLOT_START..KEYRING_SLOT_END).map(|slot| (INVENTORY_SLOT_BAG_0, slot)),
        );
        for bag in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
            positions.extend((0..MAX_BAG_SIZE as u8).map(|slot| (bag, slot)));
        }
        positions.extend(
            (EQUIPMENT_SLOT_HEAD..INVENTORY_SLOT_BAG_END).map(|slot| (INVENTORY_SLOT_BAG_0, slot)),
        );
        positions.extend(
            (BANK_SLOT_ITEM_START..BANK_SLOT_ITEM_END).map(|slot| (INVENTORY_SLOT_BAG_0, slot)),
        );
        for bag in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
            positions.extend((0..MAX_BAG_SIZE as u8).map(|slot| (bag, slot)));
        }
        positions.extend(
            (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).map(|slot| (INVENTORY_SLOT_BAG_0, slot)),
        );
        positions.extend(
            (CHILD_EQUIPMENT_SLOT_START..CHILD_EQUIPMENT_SLOT_END)
                .map(|slot| (INVENTORY_SLOT_BAG_0, slot)),
        );

        let items = self.inventory_runtime_like_cpp().item_objects();
        let mut ordered = Vec::new();
        for (bag, slot) in positions {
            let Some(guid) = self.get_item_by_pos(bag, slot) else {
                continue;
            };
            let item = items.get(&guid)?;
            if item.owner_guid() != self.guid() || item.update_state() == ItemUpdateState::Removed {
                return None;
            }
            ordered.push(SpellCostItemLikeCpp {
                guid,
                entry_id: item.object().entry(),
                bag,
                slot,
                count: item.count(),
                in_trade: item.is_in_trade(),
            });
        }
        Some(ordered)
    }
}

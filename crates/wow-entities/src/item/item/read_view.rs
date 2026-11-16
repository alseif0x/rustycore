use std::collections::HashSet;

use wow_constants::{
    EnchantmentSlot, InventoryResult, ItemBondingType, ItemFieldFlags, ItemFieldFlags2,
    ItemModifier, ItemUpdateState,
};
use wow_core::ObjectGuid;

use crate::{
    ItemDataUpdate, ItemDataValues, ItemStorageTemplate, ItemValuesUpdate, ObjectValuesSnapshot,
    UpdateMask, update_fields::TYPEID_ITEM,
};

use super::Item;

/// Borrowed read-only Item facts shared by live Items and captured values.
#[derive(Debug, Clone, Copy)]
pub struct ItemReadView<'a> {
    object: ObjectValuesSnapshot,
    data: &'a ItemDataValues,
    item_data_changes: &'a UpdateMask,
    slot: u8,
    container: ObjectGuid,
    container_slot: u8,
    bonding: ItemBondingType,
    update_state: ItemUpdateState,
    queue_pos: i16,
    loot_generated: bool,
    in_trade: bool,
    last_played_time_update: i64,
    refund_recipient: ObjectGuid,
    paid_money: u64,
    paid_extended_cost: u32,
    soulbound_trade_allowed_guids: &'a HashSet<ObjectGuid>,
    text: &'a str,
}

impl<'a> ItemReadView<'a> {
    pub(super) fn from_item(item: &'a Item) -> Self {
        Self {
            object: item.object.values_snapshot(),
            data: &item.data,
            item_data_changes: &item.item_data_changes,
            slot: item.slot,
            container: item.container,
            container_slot: item.container_slot,
            bonding: item.bonding,
            update_state: item.update_state,
            queue_pos: item.queue_pos,
            loot_generated: item.loot_generated,
            in_trade: item.in_trade,
            last_played_time_update: item.last_played_time_update,
            refund_recipient: item.refund_recipient,
            paid_money: item.paid_money,
            paid_extended_cost: item.paid_extended_cost,
            soulbound_trade_allowed_guids: &item.soulbound_trade_allowed_guids,
            text: &item.text,
        }
    }

    pub(super) fn from_snapshot(snapshot: &'a super::values_snapshot::ItemValuesSnapshot) -> Self {
        Self {
            object: *snapshot.object(),
            data: snapshot.data(),
            item_data_changes: snapshot.item_data_changes_mask(),
            slot: snapshot.slot(),
            container: snapshot.container_guid(),
            container_slot: snapshot.container_slot(),
            bonding: snapshot.bonding(),
            update_state: snapshot.update_state(),
            queue_pos: snapshot.queue_pos(),
            loot_generated: snapshot.loot_generated(),
            in_trade: snapshot.is_in_trade(),
            last_played_time_update: snapshot.last_played_time_update(),
            refund_recipient: snapshot.refund_recipient(),
            paid_money: snapshot.paid_money(),
            paid_extended_cost: snapshot.paid_extended_cost(),
            soulbound_trade_allowed_guids: snapshot.soulbound_trade_allowed_guids(),
            text: snapshot.text(),
        }
    }

    pub const fn object(&self) -> &ObjectValuesSnapshot {
        &self.object
    }

    pub const fn data(&self) -> &'a ItemDataValues {
        self.data
    }

    pub const fn item_data_changes_mask(&self) -> &'a UpdateMask {
        self.item_data_changes
    }

    pub const fn slot(&self) -> u8 {
        self.slot
    }

    pub const fn container_guid(&self) -> ObjectGuid {
        self.container
    }

    pub const fn container_slot(&self) -> u8 {
        self.container_slot
    }

    pub const fn bonding(&self) -> ItemBondingType {
        self.bonding
    }

    pub const fn update_state(&self) -> ItemUpdateState {
        self.update_state
    }

    pub const fn queue_pos(&self) -> i16 {
        self.queue_pos
    }

    pub const fn loot_generated(&self) -> bool {
        self.loot_generated
    }

    pub const fn is_in_trade(&self) -> bool {
        self.in_trade
    }

    pub const fn last_played_time_update(&self) -> i64 {
        self.last_played_time_update
    }

    pub const fn refund_recipient(&self) -> ObjectGuid {
        self.refund_recipient
    }

    pub const fn paid_money(&self) -> u64 {
        self.paid_money
    }

    pub const fn paid_extended_cost(&self) -> u32 {
        self.paid_extended_cost
    }

    pub const fn soulbound_trade_allowed_guids(&self) -> &'a HashSet<ObjectGuid> {
        self.soulbound_trade_allowed_guids
    }

    pub const fn text(&self) -> &'a str {
        self.text
    }

    pub fn is_in_bag(&self) -> bool {
        !self.container.is_empty()
    }

    pub fn bag_slot(&self) -> u8 {
        if self.is_in_bag() {
            self.container_slot
        } else {
            super::INVENTORY_SLOT_BAG_0
        }
    }

    pub fn position(&self) -> u16 {
        u16::from(self.bag_slot()) << 8 | u16::from(self.slot)
    }

    pub fn is_equipped(&self) -> bool {
        !self.is_in_bag()
            && (self.slot < super::EQUIPMENT_SLOT_END
                || (self.slot >= super::PROFESSION_SLOT_START
                    && self.slot < super::PROFESSION_SLOT_END))
    }

    pub fn can_be_merged_partly_with(&self, entry: u32, max_stack_size: u32) -> InventoryResult {
        if self.loot_generated {
            return InventoryResult::LootGone;
        }

        if self.object.entry() != entry || self.data.stack_count >= max_stack_size {
            return InventoryResult::CantStack;
        }

        InventoryResult::Ok
    }

    pub const fn item_flags_bits(&self) -> u32 {
        self.data.dynamic_flags
    }

    pub fn has_item_flag(&self, flag: ItemFieldFlags) -> bool {
        (self.data.dynamic_flags & flag.bits()) != 0
    }

    pub fn has_item_flag2(&self, flag: ItemFieldFlags2) -> bool {
        (self.data.dynamic_flags2 & flag.bits()) != 0
    }

    pub fn is_soul_bound(&self) -> bool {
        self.has_item_flag(ItemFieldFlags::SOULBOUND)
    }

    pub fn is_refundable(&self) -> bool {
        self.has_item_flag(ItemFieldFlags::REFUNDABLE)
    }

    pub fn is_bop_tradeable(&self) -> bool {
        self.has_item_flag(ItemFieldFlags::BOP_TRADEABLE)
    }

    pub fn is_soulbound_trade_allowed_for(&self, player_guid: ObjectGuid) -> bool {
        self.soulbound_trade_allowed_guids.contains(&player_guid)
    }

    pub fn is_binded_not_with(
        &self,
        player_guid: ObjectGuid,
        template: &ItemStorageTemplate,
        bop_trade_allowed_for_player: bool,
    ) -> bool {
        if !self.is_soul_bound() {
            return false;
        }

        if self.data.owner == player_guid {
            return false;
        }

        if self.is_bop_tradeable() && bop_trade_allowed_for_player {
            return false;
        }

        if template.is_bound_account_wide() {
            return false;
        }

        true
    }

    pub fn is_binded_not_with_using_allowed_guids(
        &self,
        player_guid: ObjectGuid,
        template: &ItemStorageTemplate,
    ) -> bool {
        self.is_binded_not_with(
            player_guid,
            template,
            self.is_soulbound_trade_allowed_for(player_guid),
        )
    }

    pub fn played_time(&self, now_secs: i64) -> u32 {
        let elapsed = now_secs
            .saturating_sub(self.last_played_time_update)
            .try_into()
            .unwrap_or(u32::MAX);
        self.data.create_played_time.saturating_add(elapsed)
    }

    pub fn is_refund_expired_at(&self, now_secs: i64) -> bool {
        self.played_time(now_secs) > super::BOP_TRADEABLE_DURATION_SECS
    }

    pub fn is_soulbound_trade_expired(&self, owner_total_played_time: u32) -> bool {
        self.data
            .create_played_time
            .saturating_add(super::BOP_TRADEABLE_DURATION_SECS)
            < owner_total_played_time
    }

    pub fn is_wrapped(&self) -> bool {
        self.has_item_flag(ItemFieldFlags::WRAPPED)
    }

    pub fn is_locked(&self) -> bool {
        !self.has_item_flag(ItemFieldFlags::UNLOCKED)
    }

    pub fn is_broken(&self) -> bool {
        self.data.max_durability > 0 && self.data.durability == 0
    }

    pub fn get_modifier(&self, modifier: ItemModifier) -> u32 {
        self.data.modifiers[modifier as usize]
    }

    fn visible_modified_appearance_modifier(&self, active_talent_group: usize) -> u32 {
        let mut item_modified_appearance_id = self.get_modifier(super::spec_modifier(
            active_talent_group,
            &super::APPEARANCE_MODIFIER_SLOT_BY_SPEC,
        ));
        if item_modified_appearance_id == 0 {
            item_modified_appearance_id =
                self.get_modifier(ItemModifier::TransmogAppearanceAllSpecs);
        }
        item_modified_appearance_id
    }

    /// Item.cpp:1985 resolves the selected transmog display directly, then
    /// falls back to this physical item's appearance independently of entry.
    pub fn display_id_like_cpp(
        &self,
        active_talent_group: usize,
        modified_appearance_display: impl Fn(u32) -> Option<u32>,
        physical_item_display: impl Fn(u32, u32) -> Option<u32>,
    ) -> Option<u32> {
        modified_appearance_display(self.visible_modified_appearance_modifier(active_talent_group))
            .or_else(|| {
                physical_item_display(
                    self.object.entry(),
                    u32::from(self.data.item_appearance_mod_id),
                )
            })
    }

    pub fn visible_entry(
        &self,
        active_talent_group: usize,
        item_modified_appearance: impl Fn(u32) -> Option<(u32, u16)>,
    ) -> u32 {
        let item_modified_appearance_id =
            self.visible_modified_appearance_modifier(active_talent_group);

        if let Some((item_id, _)) = item_modified_appearance(item_modified_appearance_id) {
            item_id
        } else {
            self.object.entry()
        }
    }

    pub fn visible_appearance_mod_id(
        &self,
        active_talent_group: usize,
        item_modified_appearance: impl Fn(u32) -> Option<(u32, u16)>,
    ) -> u16 {
        let item_modified_appearance_id =
            self.visible_modified_appearance_modifier(active_talent_group);

        if let Some((_, item_appearance_modifier_id)) =
            item_modified_appearance(item_modified_appearance_id)
        {
            item_appearance_modifier_id
        } else {
            u16::from(self.data.item_appearance_mod_id)
        }
    }

    pub fn visible_enchantment_id(&self, active_talent_group: usize) -> u32 {
        let mut enchantment_id = self.get_modifier(super::spec_modifier(
            active_talent_group,
            &super::ILLUSION_MODIFIER_SLOT_BY_SPEC,
        ));
        if enchantment_id == 0 {
            enchantment_id = self.get_modifier(ItemModifier::EnchantIllusionAllSpecs);
        }
        if enchantment_id == 0 {
            enchantment_id = u32::try_from(
                self.data.enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
            )
            .unwrap_or(0);
        }
        enchantment_id
    }

    pub fn visible_item_visual(
        &self,
        active_talent_group: usize,
        spell_item_enchantment_visual: impl Fn(u32) -> Option<u16>,
    ) -> u16 {
        spell_item_enchantment_visual(self.visible_enchantment_id(active_talent_group)).unwrap_or(0)
    }

    pub fn visible_secondary_modified_appearance_id(&self, active_talent_group: usize) -> u32 {
        let mut item_modified_appearance_id = self.get_modifier(super::spec_modifier(
            active_talent_group,
            &super::SECONDARY_APPEARANCE_MODIFIER_SLOT_BY_SPEC,
        ));
        if item_modified_appearance_id == 0 {
            item_modified_appearance_id =
                self.get_modifier(ItemModifier::TransmogSecondaryAppearanceAllSpecs);
        }
        item_modified_appearance_id
    }

    pub fn changed_object_type_mask(&self) -> u32 {
        self.object.changed_object_type_mask()
            | if self.item_data_changes.is_any_set() {
                1 << TYPEID_ITEM
            } else {
                0
            }
    }

    pub fn values_update(&self) -> ItemValuesUpdate {
        let object_update = self.object.values_update();
        ItemValuesUpdate {
            changed_object_type_mask: self.changed_object_type_mask(),
            object_data: object_update.object_data,
            item_data: self.item_data_changes.is_any_set().then(|| ItemDataUpdate {
                mask: self.item_data_changes.clone(),
                values: self.data.clone(),
            }),
        }
    }
}

impl Item {
    pub fn read_view(&self) -> ItemReadView<'_> {
        ItemReadView::from_item(self)
    }

    pub fn can_be_merged_partly_with(&self, entry: u32, max_stack_size: u32) -> InventoryResult {
        self.read_view()
            .can_be_merged_partly_with(entry, max_stack_size)
    }

    pub fn is_in_bag(&self) -> bool {
        self.read_view().is_in_bag()
    }

    pub fn bag_slot(&self) -> u8 {
        self.read_view().bag_slot()
    }

    pub fn position(&self) -> u16 {
        self.read_view().position()
    }

    pub fn is_equipped(&self) -> bool {
        self.read_view().is_equipped()
    }

    pub fn has_item_flag(&self, flag: ItemFieldFlags) -> bool {
        self.read_view().has_item_flag(flag)
    }

    pub fn has_item_flag2(&self, flag: ItemFieldFlags2) -> bool {
        self.read_view().has_item_flag2(flag)
    }

    pub fn is_soul_bound(&self) -> bool {
        self.read_view().is_soul_bound()
    }

    pub fn is_refundable(&self) -> bool {
        self.read_view().is_refundable()
    }

    pub fn is_bop_tradeable(&self) -> bool {
        self.read_view().is_bop_tradeable()
    }

    pub fn is_soulbound_trade_allowed_for(&self, player_guid: ObjectGuid) -> bool {
        self.read_view().is_soulbound_trade_allowed_for(player_guid)
    }

    pub fn is_binded_not_with(
        &self,
        player_guid: ObjectGuid,
        template: &ItemStorageTemplate,
        bop_trade_allowed_for_player: bool,
    ) -> bool {
        self.read_view()
            .is_binded_not_with(player_guid, template, bop_trade_allowed_for_player)
    }

    pub fn is_binded_not_with_using_allowed_guids(
        &self,
        player_guid: ObjectGuid,
        template: &ItemStorageTemplate,
    ) -> bool {
        self.read_view()
            .is_binded_not_with_using_allowed_guids(player_guid, template)
    }

    pub fn played_time(&self, now_secs: i64) -> u32 {
        self.read_view().played_time(now_secs)
    }

    pub fn is_refund_expired_at(&self, now_secs: i64) -> bool {
        self.read_view().is_refund_expired_at(now_secs)
    }

    pub fn is_soulbound_trade_expired(&self, owner_total_played_time: u32) -> bool {
        self.read_view()
            .is_soulbound_trade_expired(owner_total_played_time)
    }

    pub fn is_wrapped(&self) -> bool {
        self.read_view().is_wrapped()
    }

    pub fn is_locked(&self) -> bool {
        self.read_view().is_locked()
    }

    pub fn is_broken(&self) -> bool {
        self.read_view().is_broken()
    }

    pub fn display_id_like_cpp(
        &self,
        active_talent_group: usize,
        modified_appearance_display: impl Fn(u32) -> Option<u32>,
        physical_item_display: impl Fn(u32, u32) -> Option<u32>,
    ) -> Option<u32> {
        self.read_view().display_id_like_cpp(
            active_talent_group,
            modified_appearance_display,
            physical_item_display,
        )
    }

    pub fn visible_entry(
        &self,
        active_talent_group: usize,
        item_modified_appearance: impl Fn(u32) -> Option<(u32, u16)>,
    ) -> u32 {
        self.read_view()
            .visible_entry(active_talent_group, item_modified_appearance)
    }

    pub fn visible_appearance_mod_id(
        &self,
        active_talent_group: usize,
        item_modified_appearance: impl Fn(u32) -> Option<(u32, u16)>,
    ) -> u16 {
        self.read_view()
            .visible_appearance_mod_id(active_talent_group, item_modified_appearance)
    }

    pub fn visible_enchantment_id(&self, active_talent_group: usize) -> u32 {
        self.read_view().visible_enchantment_id(active_talent_group)
    }

    pub fn visible_item_visual(
        &self,
        active_talent_group: usize,
        spell_item_enchantment_visual: impl Fn(u32) -> Option<u16>,
    ) -> u16 {
        self.read_view()
            .visible_item_visual(active_talent_group, spell_item_enchantment_visual)
    }

    pub fn visible_secondary_modified_appearance_id(&self, active_talent_group: usize) -> u32 {
        self.read_view()
            .visible_secondary_modified_appearance_id(active_talent_group)
    }

    pub fn changed_object_type_mask(&self) -> u32 {
        self.read_view().changed_object_type_mask()
    }

    pub fn values_update(&self) -> ItemValuesUpdate {
        self.read_view().values_update()
    }
}

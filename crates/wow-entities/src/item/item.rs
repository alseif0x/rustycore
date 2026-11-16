//! Item packets.
//!
//! Separated from item.rs under #693.

use super::*;

mod save_ack;
pub(crate) use save_ack::ItemPersistentSaveStateLikeCpp;
mod read_view;
pub use read_view::ItemReadView;
mod values_map_snapshot;
pub use values_map_snapshot::ItemValuesMapSnapshot;
mod values_snapshot;
pub use values_snapshot::ItemValuesSnapshot;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    object: EntityObject,
    data: ItemDataValues,
    item_data_changes: UpdateMask,
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
    soulbound_trade_allowed_guids: HashSet<ObjectGuid>,
    text: String,
}

impl Default for Item {
    fn default() -> Self {
        Self::new(0)
    }
}

fn set_stack_count_value_and_mask(
    data: &mut ItemDataValues,
    item_data_changes: &mut UpdateMask,
    count: u32,
) {
    if data.stack_count != count {
        data.stack_count = count;
        item_data_changes.set(ITEM_DATA_PARENT_BIT);
        item_data_changes.set(ITEM_DATA_STACK_COUNT_BIT);
    }
}

impl Item {
    pub fn new(last_played_time_update: i64) -> Self {
        Self {
            object: EntityObject::new(TypeId::Item, TypeMask::OBJECT | TypeMask::ITEM),
            data: ItemDataValues::default(),
            item_data_changes: UpdateMask::new(ITEM_DATA_BITS),
            slot: 0,
            container: ObjectGuid::EMPTY,
            container_slot: INVENTORY_SLOT_BAG_0,
            bonding: ItemBondingType::None,
            update_state: ItemUpdateState::New,
            queue_pos: -1,
            loot_generated: false,
            in_trade: false,
            last_played_time_update,
            refund_recipient: ObjectGuid::EMPTY,
            paid_money: 0,
            paid_extended_cost: 0,
            soulbound_trade_allowed_guids: HashSet::new(),
            text: String::new(),
        }
    }

    pub const fn object(&self) -> &EntityObject {
        &self.object
    }

    pub fn object_mut(&mut self) -> &mut EntityObject {
        &mut self.object
    }

    pub const fn data(&self) -> &ItemDataValues {
        &self.data
    }

    pub const fn count(&self) -> u32 {
        self.data.stack_count
    }

    pub fn item_data_changes_mask(&self) -> &UpdateMask {
        &self.item_data_changes
    }

    pub fn clear_item_data_changes(&mut self) {
        self.item_data_changes.reset_all();
    }

    pub fn initialize_created_state(&mut self, create: ItemCreateInfo) {
        self.object.create(create.guid);
        self.object.set_entry(create.item_id);
        self.object.set_scale(1.0);

        if let Some(owner) = create.owner {
            self.set_owner_guid(owner);
            self.set_contained_in(owner);
        }

        self.set_count(1);
        self.set_max_durability(create.max_durability);
        self.set_durability(create.max_durability);
        for (index, charges) in create.spell_charges.into_iter().enumerate() {
            self.set_spell_charges(index, charges);
        }
        self.set_expiration(create.expiration);
        self.set_create_played_time(0);
        self.set_context(create.context);
    }

    pub fn clone_item_for_store(
        &self,
        guid: ObjectGuid,
        owner: Option<ObjectGuid>,
        count: u32,
    ) -> Self {
        let mut item = Self::new(self.last_played_time_update);
        item.object.create(guid);
        item.object.set_entry(self.object.entry());
        item.object.set_scale(1.0);

        if let Some(owner) = owner {
            item.set_owner_guid(owner);
            item.set_contained_in(owner);
        }

        item.set_count(1);
        item.set_max_durability(self.data.max_durability);
        item.set_durability(self.data.max_durability);
        for (index, charges) in self.data.spell_charges.into_iter().enumerate() {
            item.set_spell_charges(index, charges);
        }
        item.set_expiration(self.data.expiration);
        item.set_create_played_time(0);
        item.set_context_value(self.data.context);
        item.set_count(count);
        item.set_creator(self.data.creator);
        item.set_gift_creator(self.data.gift_creator);
        item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
            self.data.dynamic_flags
                & !(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE).bits(),
        ));
        item.bonding = self.bonding;
        item
    }

    pub const fn slot(&self) -> u8 {
        self.slot
    }

    pub fn set_slot(&mut self, slot: u8) {
        self.slot = slot;
    }

    pub const fn container_guid(&self) -> ObjectGuid {
        self.container
    }

    pub fn set_container_guid(&mut self, container: ObjectGuid) {
        self.container = container;
        if container.is_empty() {
            self.container_slot = INVENTORY_SLOT_BAG_0;
        }
    }

    pub fn set_container_guid_and_slot(&mut self, container: ObjectGuid, container_slot: u8) {
        self.container = container;
        self.container_slot = if container.is_empty() {
            INVENTORY_SLOT_BAG_0
        } else {
            container_slot
        };
    }

    pub const fn bonding(&self) -> ItemBondingType {
        self.bonding
    }

    pub fn set_bonding(&mut self, bonding: ItemBondingType) {
        self.bonding = bonding;
    }

    pub fn bind_if_visualized(&mut self) {
        if matches!(
            self.bonding,
            ItemBondingType::OnEquip | ItemBondingType::OnAcquire | ItemBondingType::Quest
        ) {
            self.set_binding(true);
        }
    }

    pub fn bind_if_stored(&mut self, is_bag_pos: bool) {
        if matches!(
            self.bonding,
            ItemBondingType::OnAcquire | ItemBondingType::Quest
        ) || (self.bonding == ItemBondingType::OnEquip && is_bag_pos)
        {
            self.set_binding(true);
        }
    }

    pub const fn update_state(&self) -> ItemUpdateState {
        self.update_state
    }

    pub const fn queue_pos(&self) -> i16 {
        self.queue_pos
    }

    pub const fn is_in_update_queue(&self) -> bool {
        self.queue_pos != -1
    }

    pub fn set_queue_pos(&mut self, queue_pos: i16) {
        self.queue_pos = queue_pos;
    }

    pub fn set_state(&mut self, state: ItemUpdateState) -> ItemStateTransition {
        if self.update_state == ItemUpdateState::New && state == ItemUpdateState::Removed {
            return ItemStateTransition::PretendNeverExisted;
        }

        if state != ItemUpdateState::Unchanged {
            if self.update_state != ItemUpdateState::New {
                self.update_state = state;
            }
        } else {
            self.queue_pos = -1;
            self.update_state = ItemUpdateState::Unchanged;
        }

        ItemStateTransition::Updated
    }

    pub fn force_state(&mut self, state: ItemUpdateState) {
        self.update_state = state;
    }

    pub const fn loot_generated(&self) -> bool {
        self.loot_generated
    }

    pub fn set_loot_generated(&mut self, loot_generated: bool) {
        self.loot_generated = loot_generated;
    }

    pub const fn is_in_trade(&self) -> bool {
        self.in_trade
    }

    pub fn set_in_trade(&mut self, in_trade: bool) {
        self.in_trade = in_trade;
    }

    pub const fn last_played_time_update(&self) -> i64 {
        self.last_played_time_update
    }

    pub fn set_last_played_time_update(&mut self, timestamp_secs: i64) {
        self.last_played_time_update = timestamp_secs;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub const fn refund_recipient(&self) -> ObjectGuid {
        self.refund_recipient
    }

    pub fn set_refund_recipient(&mut self, recipient: ObjectGuid) {
        self.refund_recipient = recipient;
    }

    pub const fn paid_money(&self) -> u64 {
        self.paid_money
    }

    pub fn set_paid_money(&mut self, money: u64) {
        self.paid_money = money;
    }

    pub const fn paid_extended_cost(&self) -> u32 {
        self.paid_extended_cost
    }

    pub fn set_paid_extended_cost(&mut self, extended_cost: u32) {
        self.paid_extended_cost = extended_cost;
    }

    pub fn set_not_refundable(&mut self) {
        self.remove_item_flag(ItemFieldFlags::REFUNDABLE);
        self.set_refund_recipient(ObjectGuid::EMPTY);
        self.set_paid_money(0);
        self.set_paid_extended_cost(0);
    }

    pub const fn owner_guid(&self) -> ObjectGuid {
        self.data.owner
    }

    pub fn set_owner_guid(&mut self, guid: ObjectGuid) {
        self.set_guid_field(ITEM_DATA_OWNER_BIT, guid, |data| &mut data.owner);
    }

    pub fn set_contained_in(&mut self, guid: ObjectGuid) {
        self.set_guid_field(ITEM_DATA_CONTAINED_IN_BIT, guid, |data| {
            &mut data.contained_in
        });
    }

    pub fn set_creator(&mut self, guid: ObjectGuid) {
        self.set_guid_field(ITEM_DATA_CREATOR_BIT, guid, |data| &mut data.creator);
    }

    pub fn set_gift_creator(&mut self, guid: ObjectGuid) {
        self.set_guid_field(ITEM_DATA_GIFT_CREATOR_BIT, guid, |data| {
            &mut data.gift_creator
        });
    }

    pub fn set_count(&mut self, count: u32) {
        set_stack_count_value_and_mask(&mut self.data, &mut self.item_data_changes, count);
    }

    pub fn set_expiration(&mut self, expiration: u32) {
        self.set_u32_field(ITEM_DATA_EXPIRATION_BIT, expiration, |data| {
            &mut data.expiration
        });
    }

    pub fn set_item_flag(&mut self, flags: ItemFieldFlags) {
        self.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
            self.data.dynamic_flags | flags.bits(),
        ));
    }

    pub fn remove_item_flag(&mut self, flags: ItemFieldFlags) {
        self.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
            self.data.dynamic_flags & !flags.bits(),
        ));
    }

    pub fn replace_all_item_flags(&mut self, flags: ItemFieldFlags) {
        self.set_u32_field(ITEM_DATA_DYNAMIC_FLAGS_BIT, flags.bits(), |data| {
            &mut data.dynamic_flags
        });
    }

    pub const fn item_flags_bits(&self) -> u32 {
        self.data.dynamic_flags
    }

    pub fn set_binding(&mut self, bind: bool) {
        if bind {
            self.set_item_flag(ItemFieldFlags::SOULBOUND);
        } else {
            self.remove_item_flag(ItemFieldFlags::SOULBOUND);
        }
    }

    pub fn set_item_flag2(&mut self, flags: ItemFieldFlags2) {
        self.replace_all_item_flags2(ItemFieldFlags2::from_bits_retain(
            self.data.dynamic_flags2 | flags.bits(),
        ));
    }

    pub fn remove_item_flag2(&mut self, flags: ItemFieldFlags2) {
        self.replace_all_item_flags2(ItemFieldFlags2::from_bits_retain(
            self.data.dynamic_flags2 & !flags.bits(),
        ));
    }

    pub fn replace_all_item_flags2(&mut self, flags: ItemFieldFlags2) {
        self.set_u32_field(ITEM_DATA_DYNAMIC_FLAGS2_BIT, flags.bits(), |data| {
            &mut data.dynamic_flags2
        });
    }

    pub fn soulbound_trade_allowed_guids(&self) -> &HashSet<ObjectGuid> {
        &self.soulbound_trade_allowed_guids
    }

    pub fn set_soulbound_tradeable<I>(&mut self, allowed_looters: I)
    where
        I: IntoIterator<Item = ObjectGuid>,
    {
        self.set_item_flag(ItemFieldFlags::BOP_TRADEABLE);
        self.soulbound_trade_allowed_guids.clear();
        self.soulbound_trade_allowed_guids.extend(allowed_looters);
    }

    pub fn clear_soulbound_tradeable(&mut self) -> bool {
        self.remove_item_flag(ItemFieldFlags::BOP_TRADEABLE);
        let had_allowed_guids = !self.soulbound_trade_allowed_guids.is_empty();
        self.soulbound_trade_allowed_guids.clear();
        had_allowed_guids
    }

    pub fn set_property_seed(&mut self, seed: i32) {
        self.set_i32_field(ITEM_DATA_PROPERTY_SEED_BIT, seed, |data| {
            &mut data.property_seed
        });
    }

    pub fn set_random_properties_id(&mut self, id: i32) {
        self.set_i32_field(ITEM_DATA_RANDOM_PROPERTIES_ID_BIT, id, |data| {
            &mut data.random_properties_id
        });
    }

    pub fn set_durability(&mut self, durability: u32) {
        self.set_u32_field(ITEM_DATA_DURABILITY_BIT, durability, |data| {
            &mut data.durability
        });
    }

    pub fn set_max_durability(&mut self, max_durability: u32) {
        self.set_u32_field(ITEM_DATA_MAX_DURABILITY_BIT, max_durability, |data| {
            &mut data.max_durability
        });
    }

    pub fn set_create_played_time(&mut self, create_played_time: u32) {
        self.set_u32_field(
            ITEM_DATA_CREATE_PLAYED_TIME_BIT,
            create_played_time,
            |data| &mut data.create_played_time,
        );
    }

    pub fn set_context(&mut self, context: ItemContext) {
        self.set_i32_field(ITEM_DATA_CONTEXT_BIT, context as i32, |data| {
            &mut data.context
        });
    }

    pub fn set_context_value(&mut self, context: i32) {
        self.set_i32_field(ITEM_DATA_CONTEXT_BIT, context, |data| &mut data.context);
    }

    pub fn set_create_time(&mut self, create_time: i64) {
        self.set_i64_field(ITEM_DATA_CREATE_TIME_BIT, create_time, |data| {
            &mut data.create_time
        });
    }

    pub fn set_artifact_xp(&mut self, artifact_xp: u64) {
        self.set_u64_field(ITEM_DATA_ARTIFACT_XP_BIT, artifact_xp, |data| {
            &mut data.artifact_xp
        });
    }

    pub fn set_appearance_mod_id(&mut self, appearance_mod_id: u8) {
        self.set_u8_field(
            ITEM_DATA_ITEM_APPEARANCE_MOD_ID_BIT,
            appearance_mod_id,
            |data| &mut data.item_appearance_mod_id,
        );
    }

    pub fn set_debug_item_level(&mut self, item_level: u16) {
        self.set_u16_field(ITEM_DATA_DEBUG_ITEM_LEVEL_BIT, item_level, |data| {
            &mut data.debug_item_level
        });
    }

    pub fn set_item_bonus_key(&mut self, item_bonus_key: ItemBonusKey) {
        if self.data.item_bonus_key != item_bonus_key {
            self.data.item_bonus_key = item_bonus_key;
            self.mark_item_data(ITEM_DATA_ITEM_BONUS_KEY_BIT);
        }
    }

    pub fn set_spell_charges(&mut self, index: usize, value: i32) {
        assert!(index < MAX_ITEM_SPELLS);
        if self.data.spell_charges[index] != value {
            self.data.spell_charges[index] = value;
            self.mark_item_data_array(
                ITEM_DATA_SPELL_CHARGES_PARENT_BIT,
                ITEM_DATA_SPELL_CHARGES_FIRST_BIT,
                index,
            );
        }
    }

    pub fn set_enchantment(&mut self, slot: EnchantmentSlot, id: i32, duration: u32, charges: i16) {
        let index = slot as usize;
        assert!(index < MAX_ENCHANTMENT_SLOT);
        let target = &mut self.data.enchantments[index];
        if target.id == id && target.duration == duration && target.charges == charges {
            return;
        }
        target.id = id;
        target.duration = duration;
        target.charges = charges;
        self.mark_item_data_array(
            ITEM_DATA_ENCHANTMENT_PARENT_BIT,
            ITEM_DATA_ENCHANTMENT_FIRST_BIT,
            index,
        );
    }

    pub fn set_enchantment_duration(&mut self, slot: EnchantmentSlot, duration: u32) {
        let index = slot as usize;
        assert!(index < MAX_ENCHANTMENT_SLOT);
        if self.data.enchantments[index].duration != duration {
            self.data.enchantments[index].duration = duration;
            self.mark_item_data_array(
                ITEM_DATA_ENCHANTMENT_PARENT_BIT,
                ITEM_DATA_ENCHANTMENT_FIRST_BIT,
                index,
            );
        }
    }

    pub fn set_enchantment_charges(&mut self, slot: EnchantmentSlot, charges: i16) {
        let index = slot as usize;
        assert!(index < MAX_ENCHANTMENT_SLOT);
        if self.data.enchantments[index].charges != charges {
            self.data.enchantments[index].charges = charges;
            self.mark_item_data_array(
                ITEM_DATA_ENCHANTMENT_PARENT_BIT,
                ITEM_DATA_ENCHANTMENT_FIRST_BIT,
                index,
            );
        }
    }

    pub fn clear_enchantment(&mut self, slot: EnchantmentSlot) {
        self.set_enchantment(slot, 0, 0, 0);
    }

    pub fn set_petition_id(&mut self, petition_id: u32) {
        self.set_enchantment(
            EnchantmentSlot::EnhancementPermanent,
            petition_id as i32,
            0,
            0,
        );
    }

    pub fn set_petition_num_signatures(&mut self, signatures: u32) {
        self.set_enchantment_duration(EnchantmentSlot::EnhancementPermanent, signatures);
    }

    pub fn set_artifact_powers(&mut self, artifact_powers: Vec<ArtifactPower>) {
        if self.data.artifact_powers != artifact_powers {
            self.data.artifact_powers = artifact_powers;
            self.mark_item_data(ITEM_DATA_ARTIFACT_POWERS_BIT);
        }
    }

    pub fn set_gems(&mut self, gems: Vec<SocketedGem>) {
        if self.data.gems != gems {
            self.data.gems = gems;
            self.mark_item_data(ITEM_DATA_GEMS_BIT);
        }
    }

    pub fn mark_modifiers_changed(&mut self) {
        self.mark_item_data(ITEM_DATA_MODIFIERS_BIT);
    }

    pub fn get_modifier(&self, modifier: ItemModifier) -> u32 {
        self.read_view().get_modifier(modifier)
    }

    pub fn set_modifier(&mut self, modifier: ItemModifier, value: u32) {
        let target = &mut self.data.modifiers[modifier as usize];
        if *target != value {
            *target = value;
            self.mark_modifiers_changed();
        }
    }

    fn set_guid_field(
        &mut self,
        bit: usize,
        value: ObjectGuid,
        field: impl FnOnce(&mut ItemDataValues) -> &mut ObjectGuid,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_item_data(bit);
        }
    }

    fn set_u64_field(
        &mut self,
        bit: usize,
        value: u64,
        field: impl FnOnce(&mut ItemDataValues) -> &mut u64,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_item_data(bit);
        }
    }

    fn set_i64_field(
        &mut self,
        bit: usize,
        value: i64,
        field: impl FnOnce(&mut ItemDataValues) -> &mut i64,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_item_data(bit);
        }
    }

    fn set_u32_field(
        &mut self,
        bit: usize,
        value: u32,
        field: impl FnOnce(&mut ItemDataValues) -> &mut u32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_item_data(bit);
        }
    }

    fn set_i32_field(
        &mut self,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut ItemDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_item_data(bit);
        }
    }

    fn set_u16_field(
        &mut self,
        bit: usize,
        value: u16,
        field: impl FnOnce(&mut ItemDataValues) -> &mut u16,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_item_data(bit);
        }
    }

    fn set_u8_field(
        &mut self,
        bit: usize,
        value: u8,
        field: impl FnOnce(&mut ItemDataValues) -> &mut u8,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_item_data(bit);
        }
    }

    fn mark_item_data(&mut self, bit: usize) {
        self.item_data_changes.set(ITEM_DATA_PARENT_BIT);
        self.item_data_changes.set(bit);
    }

    fn mark_item_data_array(&mut self, parent_bit: usize, first_element_bit: usize, index: usize) {
        self.item_data_changes.set(parent_bit);
        self.item_data_changes.set(first_element_bit + index);
    }
}

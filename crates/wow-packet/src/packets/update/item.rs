// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item and container update blocks.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemEnchantmentValuesUpdate {
    pub item_enchantment_mask: u32,
    pub id: i32,
    pub duration: u32,
    pub charges: i16,
    pub field_a: u8,
    pub field_b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketedGemValuesUpdate {
    pub socketed_gem_mask: u32,
    pub item_id: i32,
    pub context: u8,
    pub bonus_list_ids: [u16; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemModValuesUpdate {
    pub value: i32,
    pub item_mod_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemModListValuesUpdate {
    pub item_mod_list_mask: u32,
    pub values: Vec<ItemModValuesUpdate>,
    pub values_update_mask: Option<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemBonusKeyValuesUpdate {
    pub item_id: i32,
    pub bonus_list_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDataValuesDeltaUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub item_data_mask: u64,
    pub artifact_powers: Vec<ArtifactPowerValuesUpdate>,
    pub artifact_powers_update_mask: Option<Vec<u32>>,
    pub gems: Vec<SocketedGemValuesUpdate>,
    pub gems_update_mask: Option<Vec<u32>>,
    pub owner: ObjectGuid,
    pub contained_in: ObjectGuid,
    pub creator: ObjectGuid,
    pub gift_creator: ObjectGuid,
    pub stack_count: u32,
    pub expiration: u32,
    pub dynamic_flags: u32,
    pub property_seed: i32,
    pub random_properties_id: i32,
    pub durability: u32,
    pub max_durability: u32,
    pub create_played_time: u32,
    pub context: i32,
    pub create_time: i64,
    pub artifact_xp: u64,
    pub item_appearance_mod_id: u8,
    pub modifiers: ItemModListValuesUpdate,
    pub dynamic_flags2: u32,
    pub item_bonus_key: ItemBonusKeyValuesUpdate,
    pub debug_item_level: u16,
    pub spell_charges: [i32; 5],
    pub enchantments: [ItemEnchantmentValuesUpdate; 13],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub item_data: Option<ItemDataValuesDeltaUpdate>,
    pub container_data_mask: u64,
    pub num_slots: u32,
    pub slots: [ObjectGuid; 36],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisibleItemValuesUpdate {
    pub visible_item_mask: u32,
    pub item_id: i32,
    pub appearance_mod_id: u16,
    pub item_visual: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PerksVendorItemValuesUpdate {
    pub vendor_item_id: i32,
    pub mount_id: i32,
    pub battle_pet_species_id: i32,
    pub transmog_set_id: i32,
    pub item_modified_appearance_id: i32,
    pub field_14: i32,
    pub field_18: i32,
    pub price: i32,
    pub available_until: i64,
    pub disabled: bool,
}

/// Data needed to build an Item CREATE_OBJECT block for the client.
///
/// Each equipped item must exist as a separate game object so the client
/// can display it in the character panel / inventory UI.
pub struct ItemCreateData {
    pub item_guid: ObjectGuid,
    pub entry_id: i32,
    pub owner_guid: ObjectGuid,
    pub contained_in: ObjectGuid,
    pub stack_count: u32,
    pub dynamic_flags: u32,
    pub durability: u32,
    pub max_durability: u32,
    pub random_properties_seed: i32,
    pub random_properties_id: i32,
    pub enchantments: [ItemEnchantmentValuesUpdate; 13],
    pub gems: Vec<SocketedGemValuesUpdate>,
    pub context: u8,
    /// Non-zero for `Bag` objects. C++ writes those as TYPEID_CONTAINER
    /// and appends `ContainerData::WriteCreate` after `ItemData::WriteCreate`.
    pub container_slots: u32,
    pub container_item_guids: [ObjectGuid; 36],
}

// ── PlayerStatChanges ──────────────────────────────────────────────

pub(super) fn debug_item_create_values_len_like_cpp(data: &ItemCreateData) -> usize {
    let mut block = WorldPacket::new_empty();
    write_item_create_block(&mut block, UpdateType::CreateObject, &data.item_guid, data);
    let type_id = if data.container_slots > 0 {
        TypeId::Container
    } else {
        TypeId::Item
    };
    block.into_data().len().saturating_sub(
        debug_create_header_len_like_cpp(UpdateType::CreateObject, &data.item_guid, type_id) + 7, // CreateObjectBits (18 bits flushed to 3 bytes) + PauseTimes count.
    )
}

/// Write a single CreateObject block for an Item (TypeId::Item).
///
/// Items have NO movement block, NO stationary, and all 18 bits are false.
/// Values = ObjectData + ItemData (with Owner conditional fields).
pub(super) fn write_item_create_block(
    buf: &mut WorldPacket,
    update_type: UpdateType,
    guid: &ObjectGuid,
    data: &ItemCreateData,
) {
    let is_container = data.container_slots > 0;

    // C++ `Object::BuildCreateUpdateBlockForPlayer` uses CreateObject for
    // existing login/inventory objects and CreateObject2 only while
    // `m_isNewObject` is set.
    buf.write_uint8(update_type as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = Item (1) or Container (2) for bags.
    buf.write_uint8(if is_container {
        TypeId::Container as u8
    } else {
        TypeId::Item as u8
    });

    // ── 18-bit CreateObjectBits (all false for items) ────
    for _ in 0..18 {
        buf.write_bit(false);
    }
    buf.flush_bits();

    // PauseTimes count (i32) — always 0
    buf.write_int32(0);

    // ── Values block ─────────────────────────────────────
    let mut val_buf = WorldPacket::new_empty();
    let flags: u8 = 0x01; // Owner
    val_buf.write_uint8(flags);

    // -- ObjectData (3 fields) --
    val_buf.write_int32(data.entry_id); // EntryId
    val_buf.write_uint32(0); // DynamicFlags
    val_buf.write_float(1.0); // Scale

    // -- ItemData --
    // Owner, ContainedIn, Creator, GiftCreator
    val_buf.write_packed_guid(&data.owner_guid);
    val_buf.write_packed_guid(&data.contained_in);
    write_empty_guid(&mut val_buf); // Creator
    write_empty_guid(&mut val_buf); // GiftCreator

    // Owner conditional block 1
    val_buf.write_int32(data.stack_count as i32); // StackCount
    val_buf.write_int32(0); // Expiration
    for _ in 0..5 {
        val_buf.write_int32(0); // SpellCharges[5]
    }

    // DynamicFlags
    val_buf.write_uint32(data.dynamic_flags);

    // 13 x ItemEnchantment
    for enchantment in data.enchantments {
        val_buf.write_int32(enchantment.id);
        val_buf.write_uint32(enchantment.duration);
        val_buf.write_int16(enchantment.charges);
        val_buf.write_uint8(enchantment.field_a);
        val_buf.write_uint8(enchantment.field_b);
    }

    // PropertySeed, RandomPropertiesID
    val_buf.write_int32(data.random_properties_seed);
    val_buf.write_int32(data.random_properties_id);

    // Owner conditional block 2
    val_buf.write_int32(data.durability as i32); // Durability
    val_buf.write_int32(data.max_durability as i32); // MaxDurability

    // CreatePlayedTime, Context, CreateTime
    val_buf.write_int32(0);
    val_buf.write_int32(i32::from(data.context));
    val_buf.write_int64(0);

    // Owner conditional block 3
    val_buf.write_int64(0); // ArtifactXP
    val_buf.write_uint8(0); // ItemAppearanceModID

    // ArtifactPowers.Size, Gems.Size
    val_buf.write_int32(0);
    val_buf.write_int32(data.gems.len() as i32);

    // Owner conditional block 4
    val_buf.write_uint32(0); // DynamicFlags2

    // ItemBonusKey: ItemID + BonusCount
    val_buf.write_int32(0); // ItemID
    val_buf.write_int32(0); // BonusListIDs.Count

    // Owner conditional block 5
    val_buf.write_uint16(0); // DEBUGItemLevel

    // C++ `SocketedGem::WriteCreate`. Its CREATE order deliberately differs
    // from `WriteUpdate`: ItemID, all 16 BonusListIDs, then Context.
    for gem in &data.gems {
        write_socketed_gem_create_like_cpp(&mut val_buf, gem);
    }

    // ItemModList (dynamic) — 6 bits for size = 0, then FlushBits
    val_buf.write_bits(0, 6);
    val_buf.flush_bits();

    if is_container {
        // C++ `ContainerData::WriteCreate`: Slots[36], then NumSlots.
        for slot_guid in data.container_item_guids {
            val_buf.write_packed_guid(&slot_guid);
        }
        val_buf.write_uint32(data.container_slots);
    }

    // Write values block with size prefix
    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

// ── VALUES update (UpdateType::Values) ─────────────────────────────

/// Write an ItemData VALUES update containing StackCount and, when the store
/// binds an existing stack, DynamicFlags in the same update block.
///
/// C++ refs:
/// - `Item::SetCount`
/// - `Object::BuildValuesUpdate`
/// - `UF::ItemData::WriteUpdate`
pub(super) fn write_item_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    stack_count: u32,
    dynamic_flags: Option<u32>,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(1 << 1); // TypeId::Item

    // ItemData has 43 bits: two 32-bit field blocks and a 2-bit blocks mask.
    // Parent bit 0 and StackCount bit 7 are always set. DynamicFlags bit 9
    // joins the same mask when `_StoreItem` binds the destination stack.
    val_buf.write_bits(0x01, 2);
    let mut item_mask = (1 << 0) | (1 << 7);
    if dynamic_flags.is_some() {
        item_mask |= 1 << 9;
    }
    val_buf.write_bits(item_mask, 32);
    val_buf.flush_bits();
    val_buf.write_int32(stack_count as i32);
    if let Some(dynamic_flags) = dynamic_flags {
        val_buf.write_uint32(dynamic_flags);
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

pub(super) const VALUES_TYPE_ITEM: u32 = 1 << 1;

pub(super) const VALUES_TYPE_CONTAINER: u32 = 1 << 2;

pub(super) fn write_socketed_gem_create_like_cpp(
    buf: &mut WorldPacket,
    data: &SocketedGemValuesUpdate,
) {
    buf.write_int32(data.item_id);
    for bonus in data.bonus_list_ids {
        buf.write_uint16(bonus);
    }
    buf.write_uint8(data.context);
}

fn write_socketed_gem_values_update(buf: &mut WorldPacket, data: &SocketedGemValuesUpdate) {
    let mask = u64::from(data.socketed_gem_mask & 0x000F_FFFF);
    write_update_field_blocks_mask(buf, mask, 1);
    buf.flush_bits();

    if field_mask_has(mask, 0) {
        if field_mask_has(mask, 1) {
            buf.write_int32(data.item_id);
        }
        if field_mask_has(mask, 2) {
            buf.write_uint8(data.context);
        }
    }

    if field_mask_has(mask, 3) {
        for (index, bonus) in data.bonus_list_ids.iter().enumerate() {
            if field_mask_has(mask, 4 + index) {
                buf.write_uint16(*bonus);
            }
        }
    }
}

fn write_item_enchantment_values_update(buf: &mut WorldPacket, data: &ItemEnchantmentValuesUpdate) {
    let mask = data.item_enchantment_mask & 0x3F;
    buf.write_bits(mask, 6);
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_int32(data.id);
        }
        if mask & 0x04 != 0 {
            buf.write_uint32(data.duration);
        }
        if mask & 0x08 != 0 {
            buf.write_int16(data.charges);
        }
        if mask & 0x10 != 0 {
            buf.write_uint8(data.field_a);
        }
        if mask & 0x20 != 0 {
            buf.write_uint8(data.field_b);
        }
    }
}

fn write_item_mod_values_update(buf: &mut WorldPacket, data: &ItemModValuesUpdate) {
    buf.write_int32(data.value);
    buf.write_uint8(data.item_mod_type);
}

fn write_item_mod_list_values_update(buf: &mut WorldPacket, data: &ItemModListValuesUpdate) {
    let mask = data.item_mod_list_mask & 0x01;
    buf.write_bits(mask, 1);

    if mask & 0x01 != 0 {
        write_dynamic_field_update_mask_bits(
            buf,
            data.values.len(),
            data.values_update_mask.as_deref(),
            6,
        );
    }
    buf.flush_bits();

    if mask & 0x01 != 0 {
        for (index, value) in data.values.iter().enumerate() {
            if dynamic_mask_has_index(data.values_update_mask.as_deref(), index) {
                write_item_mod_values_update(buf, value);
            }
        }
    }
    buf.flush_bits();
}

fn write_item_bonus_key_values_update(buf: &mut WorldPacket, data: &ItemBonusKeyValuesUpdate) {
    buf.write_int32(data.item_id);
    buf.write_uint32(data.bonus_list_ids.len() as u32);
    for bonus in &data.bonus_list_ids {
        buf.write_int32(*bonus);
    }
}

fn write_item_data_values_update_section(buf: &mut WorldPacket, data: &ItemDataValuesDeltaUpdate) {
    let mask = data.item_data_mask & ((1u64 << 43) - 1);
    write_update_field_blocks_mask(buf, mask, 2);

    if field_mask_has(mask, 0) {
        if field_mask_has(mask, 1) {
            write_dynamic_field_update_mask(
                buf,
                data.artifact_powers.len(),
                data.artifact_powers_update_mask.as_deref(),
            );
        }
        if field_mask_has(mask, 2) {
            write_dynamic_field_update_mask(buf, data.gems.len(), data.gems_update_mask.as_deref());
        }
    }
    buf.flush_bits();

    if field_mask_has(mask, 0) {
        if field_mask_has(mask, 1) {
            for (index, artifact_power) in data.artifact_powers.iter().enumerate() {
                if dynamic_mask_has_index(data.artifact_powers_update_mask.as_deref(), index) {
                    write_artifact_power_values_update(buf, artifact_power);
                }
            }
        }
        if field_mask_has(mask, 2) {
            for (index, gem) in data.gems.iter().enumerate() {
                if dynamic_mask_has_index(data.gems_update_mask.as_deref(), index) {
                    write_socketed_gem_values_update(buf, gem);
                }
            }
        }
        if field_mask_has(mask, 3) {
            buf.write_packed_guid(&data.owner);
        }
        if field_mask_has(mask, 4) {
            buf.write_packed_guid(&data.contained_in);
        }
        if field_mask_has(mask, 5) {
            buf.write_packed_guid(&data.creator);
        }
        if field_mask_has(mask, 6) {
            buf.write_packed_guid(&data.gift_creator);
        }
        if field_mask_has(mask, 7) {
            buf.write_uint32(data.stack_count);
        }
        if field_mask_has(mask, 8) {
            buf.write_uint32(data.expiration);
        }
        if field_mask_has(mask, 9) {
            buf.write_uint32(data.dynamic_flags);
        }
        if field_mask_has(mask, 10) {
            buf.write_int32(data.property_seed);
        }
        if field_mask_has(mask, 11) {
            buf.write_int32(data.random_properties_id);
        }
        if field_mask_has(mask, 12) {
            buf.write_uint32(data.durability);
        }
        if field_mask_has(mask, 13) {
            buf.write_uint32(data.max_durability);
        }
        if field_mask_has(mask, 14) {
            buf.write_uint32(data.create_played_time);
        }
        if field_mask_has(mask, 15) {
            buf.write_int32(data.context);
        }
        if field_mask_has(mask, 16) {
            buf.write_int64(data.create_time);
        }
        if field_mask_has(mask, 17) {
            buf.write_uint64(data.artifact_xp);
        }
        if field_mask_has(mask, 18) {
            buf.write_uint8(data.item_appearance_mod_id);
        }
        if field_mask_has(mask, 20) {
            buf.write_uint32(data.dynamic_flags2);
        }
        if field_mask_has(mask, 21) {
            write_item_bonus_key_values_update(buf, &data.item_bonus_key);
        }
        if field_mask_has(mask, 22) {
            buf.write_uint16(data.debug_item_level);
        }
        if field_mask_has(mask, 19) {
            write_item_mod_list_values_update(buf, &data.modifiers);
        }
    }

    if field_mask_has(mask, 23) {
        for (index, charge) in data.spell_charges.iter().enumerate() {
            if field_mask_has(mask, 24 + index) {
                buf.write_int32(*charge);
            }
        }
    }

    if field_mask_has(mask, 29) {
        for (index, enchantment) in data.enchantments.iter().enumerate() {
            if field_mask_has(mask, 30 + index) {
                write_item_enchantment_values_update(buf, enchantment);
            }
        }
    }
}

pub(super) fn write_full_item_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &ItemDataValuesDeltaUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_ITEM != 0 {
        write_item_data_values_update_section(&mut val_buf, data);
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn write_container_data_values_update_section(
    buf: &mut WorldPacket,
    data: &ContainerDataValuesUpdate,
) {
    let mask = data.container_data_mask & ((1u64 << 39) - 1);
    write_update_field_blocks_mask(buf, mask, 2);
    buf.flush_bits();

    if field_mask_has(mask, 0) && field_mask_has(mask, 1) {
        buf.write_uint32(data.num_slots);
    }

    if field_mask_has(mask, 2) {
        for (index, slot) in data.slots.iter().enumerate() {
            if field_mask_has(mask, 3 + index) {
                buf.write_packed_guid(slot);
            }
        }
    }
}

pub(super) fn write_container_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &ContainerDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_ITEM != 0 {
        if let Some(item_data) = &data.item_data {
            write_item_data_values_update_section(&mut val_buf, item_data);
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_CONTAINER != 0 {
        write_container_data_values_update_section(&mut val_buf, data);
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

pub(super) fn write_visible_item_values_update(
    buf: &mut WorldPacket,
    data: &VisibleItemValuesUpdate,
) {
    let mask = data.visible_item_mask & 0x0F;
    buf.write_bits(mask, 4);
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_int32(data.item_id);
        }
        if mask & 0x04 != 0 {
            buf.write_uint16(data.appearance_mod_id);
        }
        if mask & 0x08 != 0 {
            buf.write_uint16(data.item_visual);
        }
    }
}

pub fn write_perks_vendor_item_values_update(
    buf: &mut WorldPacket,
    data: PerksVendorItemValuesUpdate,
) {
    buf.write_int32(data.vendor_item_id);
    buf.write_int32(data.mount_id);
    buf.write_int32(data.battle_pet_species_id);
    buf.write_int32(data.transmog_set_id);
    buf.write_int32(data.item_modified_appearance_id);
    buf.write_int32(data.field_14);
    buf.write_int32(data.field_18);
    buf.write_int32(data.price);
    buf.write_int64(data.available_until);
    buf.write_bit(data.disabled);
    buf.flush_bits();
}

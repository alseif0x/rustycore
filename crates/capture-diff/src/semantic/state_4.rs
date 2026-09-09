//! Semantic capture diff state definitions, part 4 of 5.
//!
//! Separated from the semantic.rs root under #658. Behaviour is preserved.

use super::*;

pub(super) fn validate_issue_106_request_removal(
    request: DecodedSingleLootItemRequest,
    removed: DecodedLootRemovedBody,
) -> Result<(), String> {
    if !removed.is_issue_106_reviewed_shape() {
        return Err(removed.issue_106_shape_error().unwrap_or_else(|| {
            format!(
                "loot-removed owner {:?} is not the reviewed issue-#106 Doctor identity",
                removed.body.owner
            )
        }));
    }
    if request.loot_obj != removed.body.loot_obj {
        return Err(format!(
            "CMSG_LOOT_ITEM LootObj {:?} does not match SMSG_LOOT_REMOVED {:?}",
            request.loot_obj, removed.body.loot_obj
        ));
    }
    if request.loot_list_id != removed.body.loot_list_id {
        return Err(format!(
            "CMSG_LOOT_ITEM LootListID {} does not match SMSG_LOOT_REMOVED {}",
            request.loot_list_id, removed.body.loot_list_id
        ));
    }
    Ok(())
}

pub(super) fn validate_issue_106_created_grant(
    created: DecodedIssue106ItemCreate,
    pushed: DecodedIssue106ItemPushResult,
) -> Result<(), String> {
    if created.map_id != ISSUE_106_CREATURE_IDENTITY.map_id {
        return Err(format!(
            "item CreateObject map {} does not match loot owner map {}",
            created.map_id, ISSUE_106_CREATURE_IDENTITY.map_id
        ));
    }
    if created.owner != pushed.player || created.contained_in != pushed.player {
        return Err(format!(
            "item CreateObject owner/contained {:?}/{:?} does not match ItemPushResult player {:?}",
            created.owner, created.contained_in, pushed.player
        ));
    }
    if created.item_guid != pushed.item_guid {
        return Err(format!(
            "item CreateObject GUID {:?} does not match ItemPushResult {:?}",
            created.item_guid, pushed.item_guid
        ));
    }
    if created.item_entry != pushed.item_entry {
        return Err(format!(
            "item CreateObject entry {} does not match ItemPushResult {}",
            created.item_entry, pushed.item_entry
        ));
    }
    if created.stack_count != pushed.quantity as u32 {
        return Err(format!(
            "item CreateObject stack {} does not match ItemPushResult quantity {}",
            created.stack_count, pushed.quantity
        ));
    }
    if pushed.quantity != 1 || pushed.item_entry != ISSUE_106_ITEM_ENTRY {
        return Err(format!(
            "ItemPushResult is item {}/quantity {}, expected fixture item {ISSUE_106_ITEM_ENTRY}/quantity 1",
            pushed.item_entry, pushed.quantity
        ));
    }
    Ok(())
}

pub(super) fn validate_issue_106_inventory_update(
    body: &[u8],
    pushed: DecodedIssue106ItemPushResult,
) -> Result<(), String> {
    let inventory_update = match decode_update_object_inv_slots_candidate(body) {
        UpdateObjectInvSlotsDecode::Candidate(decoded) => decoded,
        UpdateObjectInvSlotsDecode::NotEligible(reason) => {
            return Err(format!(
                "post-claim SMSG_UPDATE_OBJECT is not the reviewed one-slot InvSlots update: {reason}"
            ));
        }
        UpdateObjectInvSlotsDecode::Malformed(error) => {
            return Err(format!(
                "post-claim SMSG_UPDATE_OBJECT is malformed: {error}"
            ));
        }
    };
    if inventory_update.body.map_id != ISSUE_106_CREATURE_IDENTITY.map_id {
        return Err(format!(
            "post-claim map {} does not match loot owner map {}",
            inventory_update.body.map_id, ISSUE_106_CREATURE_IDENTITY.map_id
        ));
    }
    if inventory_update.body.player != pushed.player {
        return Err(format!(
            "ItemPushResult player {:?} does not match InvSlots player {:?}",
            pushed.player, inventory_update.body.player
        ));
    }
    let [slot] = inventory_update.body.inv_slots.as_slice() else {
        return Err(format!(
            "post-claim InvSlots update contains {} child values, expected one",
            inventory_update.body.inv_slots.len()
        ));
    };
    if i32::from(slot.slot) != pushed.slot_in_bag {
        return Err(format!(
            "ItemPushResult SlotInBag {} does not match InvSlots child {}",
            pushed.slot_in_bag, slot.slot
        ));
    }
    if slot.item != pushed.item_guid {
        return Err(format!(
            "ItemPushResult ItemGUID {:?} does not match InvSlots item {:?}",
            pushed.item_guid, slot.item
        ));
    }
    Ok(())
}

/// Decode only the deterministic one-item CreateObject shape captured for
/// issue #106.
///
/// C++ anchors:
///
/// - `UpdateData::BuildPacket`: one update, map, destroy bit, data length;
/// - `Object::BuildCreateUpdateBlockForPlayer`: type, GUID, TypeID, movement,
///   values;
/// - `Item::BuildValuesCreate`: value length and Owner visibility flag;
/// - `ObjectData::WriteCreate` / `ItemData::WriteCreate`: entry, owner,
///   contained-in GUID and StackCount in that order.
pub(super) fn decode_issue_106_item_create(
    body: &[u8],
) -> Result<DecodedIssue106ItemCreate, String> {
    let mut cursor = 0usize;
    let num_updates = read_u32(body, &mut cursor, "NumObjUpdates")?;
    if num_updates != 1 {
        return Err(format!(
            "item CreateObject contains {num_updates} object updates, expected exactly one"
        ));
    }
    let map_id = read_u16(body, &mut cursor, "MapID")?;
    let destroy_or_out_of_range = read_u8(body, &mut cursor, "HasDestroyOrOutOfRange byte")?;
    if destroy_or_out_of_range != 0 {
        return Err(format!(
            "item CreateObject HasDestroyOrOutOfRange byte is 0x{destroy_or_out_of_range:02X}, expected canonical false"
        ));
    }

    let declared_blocks_len = read_u32(body, &mut cursor, "update blocks length")? as usize;
    let actual_blocks_len = body.len().saturating_sub(cursor);
    if declared_blocks_len != actual_blocks_len {
        return Err(format!(
            "item CreateObject update blocks length declares {declared_blocks_len} bytes but {actual_blocks_len} remain"
        ));
    }

    let update_type = read_u8(body, &mut cursor, "UpdateType")?;
    if update_type != 1 {
        return Err(format!(
            "item CreateObject UpdateType is {update_type}, expected CreateObject (1)"
        ));
    }
    let (item_low, item_high) = read_packed_guid(body, &mut cursor, "created Item")?;
    let item_guid = ExactObjectGuid {
        low: item_low,
        high: item_high,
    };
    if let Some(error) = issue_106_item_guid_error(item_guid) {
        return Err(format!("item CreateObject {error}"));
    }

    let type_id = read_u8(body, &mut cursor, "TypeID")?;
    if type_id != 1 {
        return Err(format!(
            "item CreateObject TypeID is {type_id}, expected Item (1)"
        ));
    }
    let create_bits: [u8; 3] = read_array(body, &mut cursor, "CreateObjectBits")?;
    if create_bits != [0; 3] {
        return Err(format!(
            "item CreateObject flags are {create_bits:02X?}, expected all 18 Item flags clear"
        ));
    }
    let pause_times = read_i32(body, &mut cursor, "PauseTimes count")?;
    if pause_times != 0 {
        return Err(format!(
            "item CreateObject PauseTimes count is {pause_times}, expected zero"
        ));
    }

    let declared_values_len = read_u32(body, &mut cursor, "values length")? as usize;
    let actual_values_len = body.len().saturating_sub(cursor);
    if declared_values_len != actual_values_len {
        return Err(format!(
            "item CreateObject values length declares {declared_values_len} bytes but {actual_values_len} remain"
        ));
    }
    decode_issue_106_item_create_values(&body[cursor..], map_id, item_guid)
}

pub(super) fn decode_issue_106_item_create_values(
    values: &[u8],
    map_id: u16,
    item_guid: ExactObjectGuid,
) -> Result<DecodedIssue106ItemCreate, String> {
    let mut values_cursor = 0usize;
    let visibility_flags = read_u8(values, &mut values_cursor, "UpdateFieldFlags")?;
    let item_entry = read_i32(values, &mut values_cursor, "ObjectData.EntryID")?;
    let object_dynamic_flags = read_u32(values, &mut values_cursor, "ObjectData.DynamicFlags")?;
    let object_scale_bits = read_u32(values, &mut values_cursor, "ObjectData.Scale")?;
    if visibility_flags != 0x01
        || object_dynamic_flags != 0
        || object_scale_bits != 1.0_f32.to_bits()
    {
        return Err(format!(
            "item CreateObject ObjectData is not the reviewed owner-visible shape: flags=0x{visibility_flags:02X} dynamic=0x{object_dynamic_flags:08X} scale_bits=0x{object_scale_bits:08X}"
        ));
    }
    if item_entry != ISSUE_106_ITEM_ENTRY {
        return Err(format!(
            "item CreateObject entry is {item_entry}, expected fixture item {ISSUE_106_ITEM_ENTRY}"
        ));
    }

    let (owner_low, owner_high) = read_packed_guid(values, &mut values_cursor, "ItemData.Owner")?;
    let owner = ExactObjectGuid {
        low: owner_low,
        high: owner_high,
    };
    let (contained_low, contained_high) =
        read_packed_guid(values, &mut values_cursor, "ItemData.ContainedIn")?;
    let contained_in = ExactObjectGuid {
        low: contained_low,
        high: contained_high,
    };
    let creator = read_packed_guid(values, &mut values_cursor, "ItemData.Creator")?;
    let gift_creator = read_packed_guid(values, &mut values_cursor, "ItemData.GiftCreator")?;
    let expected_player = ExactObjectGuid {
        low: ISSUE_106_CAPTURE_PLAYER_LOW,
        high: ISSUE_106_CAPTURE_PLAYER_HIGH,
    };
    if owner != expected_player || contained_in != expected_player {
        return Err(format!(
            "item CreateObject Owner/ContainedIn is {owner:?}/{contained_in:?}, expected capture player {expected_player:?}"
        ));
    }
    if creator != (0, 0) || gift_creator != (0, 0) {
        return Err(format!(
            "item CreateObject Creator/GiftCreator is {creator:?}/{gift_creator:?}, expected empty GUIDs"
        ));
    }

    let stack_count = read_u32(values, &mut values_cursor, "ItemData.StackCount")?;
    let expiration = read_u32(values, &mut values_cursor, "ItemData.Expiration")?;
    let mut spell_charges = [0_i32; 5];
    for charge in &mut spell_charges {
        *charge = read_i32(values, &mut values_cursor, "ItemData.SpellCharges")?;
    }
    let item_dynamic_flags = read_u32(values, &mut values_cursor, "ItemData.DynamicFlags")?;
    if stack_count != 1
        || expiration != 0
        || spell_charges != [0; 5]
        || item_dynamic_flags != ISSUE_106_ITEM_DYNAMIC_FLAGS
    {
        return Err(format!(
            "item CreateObject ItemData is not the deterministic one-key shape: stack={stack_count} expiration={expiration} charges={spell_charges:?} dynamic=0x{item_dynamic_flags:08X}"
        ));
    }

    let zero_tail = &values[values_cursor..];
    if zero_tail.len() != ISSUE_106_ITEM_CREATE_ZERO_TAIL_LEN
        || zero_tail.iter().any(|byte| *byte != 0)
    {
        return Err(format!(
            "item CreateObject deterministic ItemData tail has length {} and {} nonzero byte(s), expected {} zero bytes",
            zero_tail.len(),
            zero_tail.iter().filter(|byte| **byte != 0).count(),
            ISSUE_106_ITEM_CREATE_ZERO_TAIL_LEN
        ));
    }

    Ok(DecodedIssue106ItemCreate {
        map_id,
        item_guid,
        owner,
        contained_in,
        item_entry,
        stack_count,
    })
}

pub(super) fn decode_single_loot_item_request(
    body: &[u8],
) -> Result<DecodedSingleLootItemRequest, String> {
    let mut cursor = 0usize;
    let count = read_u32(body, &mut cursor, "Loot request count")?;
    if count != 1 {
        return Err(format!(
            "CMSG_LOOT_ITEM contains {count} request(s), expected exactly one"
        ));
    }
    let (loot_obj_low, loot_obj_high) = read_packed_guid(body, &mut cursor, "Loot Object")?;
    let loot_obj = ExactObjectGuid {
        low: loot_obj_low,
        high: loot_obj_high,
    };
    if let Some(error) = issue_106_loot_object_error(loot_obj) {
        return Err(format!("CMSG_LOOT_ITEM {error}"));
    }
    let loot_list_id = read_u8(body, &mut cursor, "LootListID")?;
    if loot_list_id != 0 {
        return Err(format!(
            "CMSG_LOOT_ITEM LootListID is {loot_list_id}, expected deterministic slot 0"
        ));
    }
    let soft_interact = read_u8(body, &mut cursor, "IsSoftInteract bit byte")?;
    if soft_interact != 0 {
        return Err(format!(
            "CMSG_LOOT_ITEM IsSoftInteract/padding byte is 0x{soft_interact:02X}, expected 0"
        ));
    }
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after CMSG_LOOT_ITEM: decoded {cursor} of {} bytes",
            body.len()
        ));
    }
    Ok(DecodedSingleLootItemRequest {
        loot_obj,
        loot_list_id,
    })
}

pub(super) fn decode_issue_106_item_push_result(
    body: &[u8],
) -> Result<DecodedIssue106ItemPushResult, String> {
    let mut cursor = 0usize;
    let (player_low, player_high) = read_packed_guid(body, &mut cursor, "PlayerGUID")?;
    let player = ExactObjectGuid {
        low: player_low,
        high: player_high,
    };
    if player.low != ISSUE_106_CAPTURE_PLAYER_LOW || player.high != ISSUE_106_CAPTURE_PLAYER_HIGH {
        return Err(format!(
            "ItemPushResult PlayerGUID is {player:?}, expected capture character low={ISSUE_106_CAPTURE_PLAYER_LOW} high=0x{ISSUE_106_CAPTURE_PLAYER_HIGH:016X}"
        ));
    }

    let slot = read_u8(body, &mut cursor, "Slot")?;
    let slot_in_bag = read_i32(body, &mut cursor, "SlotInBag")?;
    let quest_log_item_id = read_i32(body, &mut cursor, "QuestLogItemID")?;
    let quantity = read_i32(body, &mut cursor, "Quantity")?;
    let quantity_in_inventory = read_i32(body, &mut cursor, "QuantityInInventory")?;
    let dungeon_encounter_id = read_i32(body, &mut cursor, "DungeonEncounterID")?;
    let battle_pet_species_id = read_i32(body, &mut cursor, "BattlePetSpeciesID")?;
    let battle_pet_breed_id = read_i32(body, &mut cursor, "BattlePetBreedID")?;
    let battle_pet_breed_quality = read_u32(body, &mut cursor, "BattlePetBreedQuality")?;
    let battle_pet_level = read_i32(body, &mut cursor, "BattlePetLevel")?;
    let (item_low, item_high) = read_packed_guid(body, &mut cursor, "ItemGUID")?;
    let item_guid = ExactObjectGuid {
        low: item_low,
        high: item_high,
    };
    if let Some(error) = issue_106_item_guid_error(item_guid) {
        return Err(format!("ItemPushResult {error}"));
    }

    let result_flags = read_u8(body, &mut cursor, "ItemPushResult bit flags")?;
    if result_flags != 0x08 {
        return Err(format!(
            "ItemPushResult flags are 0x{result_flags:02X}, expected non-created/non-pushed normal-display loot flags 0x08"
        ));
    }
    let item_entry = read_i32(body, &mut cursor, "ItemInstance.ItemID")?;
    let random_properties_seed = read_i32(body, &mut cursor, "ItemInstance.RandomPropertiesSeed")?;
    let random_properties_id = read_i32(body, &mut cursor, "ItemInstance.RandomPropertiesID")?;
    let item_bonus_bits = read_u8(body, &mut cursor, "ItemInstance ItemBonus bit byte")?;
    let modifications_bits = read_u8(body, &mut cursor, "ItemInstance modifications bit byte")?;
    if slot != u8::MAX
        || slot_in_bag != ISSUE_106_ITEM_SLOT
        || quest_log_item_id != 0
        || quantity != 1
        || quantity_in_inventory != 1
        || dungeon_encounter_id != 0
        || battle_pet_species_id != 0
        || battle_pet_breed_id != 0
        || battle_pet_breed_quality != 0
        || battle_pet_level != 0
        || random_properties_seed != 0
        || random_properties_id != 0
        || item_bonus_bits != 0
        || modifications_bits != 0
    {
        return Err(format!(
            "ItemPushResult is not the deterministic one-item fixture shape: slot={slot} slot_in_bag={slot_in_bag} quest={quest_log_item_id} quantity={quantity}/{quantity_in_inventory} encounter={dungeon_encounter_id} battle_pet={battle_pet_species_id}/{battle_pet_breed_id}/{battle_pet_breed_quality}/{battle_pet_level} random={random_properties_seed}/{random_properties_id} bonus_bits=0x{item_bonus_bits:02X} mod_bits=0x{modifications_bits:02X}"
        ));
    }
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after ItemPushResult ItemInstance: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    Ok(DecodedIssue106ItemPushResult {
        player,
        slot_in_bag,
        quantity,
        item_guid,
        item_entry,
    })
}

pub(super) fn issue_106_item_guid_error(item_guid: ExactObjectGuid) -> Option<String> {
    if item_guid.low == 0
        || item_guid.low & !OBJECT_GUID_COUNTER_MASK != 0
        || item_guid.high != ISSUE_106_CAPTURE_ITEM_HIGH
    {
        return Some(format!(
            "ItemGUID {item_guid:?} is not the canonical non-empty realm-1 Item GUID with server id 0"
        ));
    }
    None
}

/// Decode the complete opcode-less body emitted by C++
/// `WorldPackets::Movement::MonsterMove::Write`.
///
/// This follows `MovementPackets.cpp` field order exactly and rejects
/// non-canonical packed GUIDs, nonzero bit padding, non-finite floats, trailing
/// bytes, and truncated optional sections. The returned allocation fields are
/// retained for the bot/report contract. The fixture mover counter must equal
/// the persistent spawn GUID; only the process-global spline ID is normalized.
pub fn decode_monster_move_body(body: &[u8]) -> Result<DecodedMonsterMoveBody, String> {
    let mut cursor = 0usize;
    let (mover_low, mover_high) = read_packed_guid(body, &mut cursor, "MoverGUID")?;
    if mover_low == 0 && mover_high == 0 {
        return Err("MoverGUID is empty".to_string());
    }
    let current_position = read_wire_position(body, &mut cursor, "Pos")?;
    let spline_id = read_u32(body, &mut cursor, "SplineData.ID")?;
    let destination = read_wire_position(body, &mut cursor, "SplineData.Destination")?;

    let spline_bits = read_u8(body, &mut cursor, "CrzTeleport/tolerance bit byte")?;
    if spline_bits & 0x0F != 0 {
        return Err(format!(
            "CrzTeleport/tolerance byte has non-canonical padding bits: 0x{spline_bits:02X}"
        ));
    }
    let crz_teleport = spline_bits & 0x80 != 0;
    let stop_distance_tolerance = (spline_bits >> 4) & 0x07;

    let flags = read_u32(body, &mut cursor, "Move.Flags")?;
    let elapsed = read_i32(body, &mut cursor, "Move.Elapsed")?;
    let move_time = read_u32(body, &mut cursor, "Move.MoveTime")?;
    let fade_object_time = read_u32(body, &mut cursor, "Move.FadeObjectTime")?;
    let mode = read_u8(body, &mut cursor, "Move.Mode")?;
    let (transport_low, transport_high) =
        read_packed_guid(body, &mut cursor, "Move.TransportGUID")?;
    let vehicle_seat = read_i8(body, &mut cursor, "Move.VehicleSeat")?;

    let header_bytes: [u8; 5] = read_array(body, &mut cursor, "Move bit header")?;
    let header = header_bytes
        .into_iter()
        .fold(0u64, |value, byte| (value << 8) | u64::from(byte));
    let face_kind = ((header >> 38) & 0x03) as u8;
    let point_count = ((header >> 22) & 0xFFFF) as usize;
    let vehicle_exit_voluntary = header & (1 << 21) != 0;
    let interpolate = header & (1 << 20) != 0;
    let packed_delta_count = ((header >> 4) & 0xFFFF) as usize;
    let has_spline_filter = header & (1 << 3) != 0;
    let has_spell_effect_extra = header & (1 << 2) != 0;
    let has_jump_extra = header & (1 << 1) != 0;
    let has_anim_tier_transition = header & 1 != 0;

    let spline_filter = if has_spline_filter {
        let key_count = read_u32(body, &mut cursor, "SplineFilter key count")? as usize;
        let base_speed_bits = read_f32_bits(body, &mut cursor, "SplineFilter BaseSpeed")?;
        let start_offset = read_i16(body, &mut cursor, "SplineFilter StartOffset")?;
        let distance_to_previous_key_bits =
            read_f32_bits(body, &mut cursor, "SplineFilter DistToPrevFilterKey")?;
        let added_to_start = read_i16(body, &mut cursor, "SplineFilter AddedToStart")?;
        let mut keys = Vec::with_capacity(key_count.min(body.len() / 4));
        for index in 0..key_count {
            keys.push(MonsterSplineFilterKeyBody {
                index: read_i16(body, &mut cursor, &format!("SplineFilter key[{index}].Idx"))?,
                speed: read_u16(
                    body,
                    &mut cursor,
                    &format!("SplineFilter key[{index}].Speed"),
                )?,
            });
        }
        let flag_byte = read_u8(body, &mut cursor, "SplineFilter flags bit byte")?;
        if flag_byte & 0x3F != 0 {
            return Err(format!(
                "SplineFilter flags byte has non-canonical padding bits: 0x{flag_byte:02X}"
            ));
        }
        Some(MonsterSplineFilterBody {
            base_speed_bits,
            start_offset,
            distance_to_previous_key_bits,
            added_to_start,
            keys,
            flags: flag_byte >> 6,
        })
    } else {
        None
    };

    let face = match face_kind {
        0 => MonsterMoveFaceBody::Normal,
        1 => MonsterMoveFaceBody::Spot {
            position: read_wire_position(body, &mut cursor, "Move.FaceSpot")?,
        },
        2 => {
            let direction_bits = read_f32_bits(body, &mut cursor, "Move.FaceDirection")?;
            let (low, high) = read_packed_guid(body, &mut cursor, "Move.FaceGUID")?;
            MonsterMoveFaceBody::Target {
                direction_bits,
                target: ExactObjectGuid { low, high },
            }
        }
        3 => MonsterMoveFaceBody::Angle {
            direction_bits: read_f32_bits(body, &mut cursor, "Move.FaceDirection")?,
        },
        _ => unreachable!("two-bit face kind"),
    };

    let mut points = Vec::with_capacity(point_count.min(body.len() / 12));
    for index in 0..point_count {
        points.push(read_wire_position(
            body,
            &mut cursor,
            &format!("Move.Points[{index}]"),
        )?);
    }
    let mut packed_deltas = Vec::with_capacity(packed_delta_count.min(body.len() / 4));
    for index in 0..packed_delta_count {
        packed_deltas.push(read_u32(
            body,
            &mut cursor,
            &format!("Move.PackedDeltas[{index}]"),
        )?);
    }

    let spell_effect_extra = if has_spell_effect_extra {
        let (low, high) = read_packed_guid(body, &mut cursor, "SpellEffectExtra.TargetGUID")?;
        Some(MonsterSplineSpellEffectExtraBody {
            target: ExactObjectGuid { low, high },
            spell_visual_id: read_u32(body, &mut cursor, "SpellEffectExtra.SpellVisualID")?,
            progress_curve_id: read_u32(body, &mut cursor, "SpellEffectExtra.ProgressCurveID")?,
            parabolic_curve_id: read_u32(body, &mut cursor, "SpellEffectExtra.ParabolicCurveID")?,
            jump_gravity_bits: read_f32_bits(body, &mut cursor, "SpellEffectExtra.JumpGravity")?,
        })
    } else {
        None
    };
    let jump_extra = if has_jump_extra {
        Some(MonsterSplineJumpExtraBody {
            jump_gravity_bits: read_f32_bits(body, &mut cursor, "JumpExtra.JumpGravity")?,
            start_time: read_u32(body, &mut cursor, "JumpExtra.StartTime")?,
            duration: read_u32(body, &mut cursor, "JumpExtra.Duration")?,
        })
    } else {
        None
    };
    let anim_tier_transition = if has_anim_tier_transition {
        Some(MonsterSplineAnimTierTransitionBody {
            tier_transition_id: read_i32(body, &mut cursor, "AnimTier.TierTransitionID")?,
            start_time: read_u32(body, &mut cursor, "AnimTier.StartTime")?,
            end_time: read_u32(body, &mut cursor, "AnimTier.EndTime")?,
            animation_tier: read_u8(body, &mut cursor, "AnimTier.AnimTier")?,
        })
    } else {
        None
    };

    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after MovementMonsterSpline: decoded {cursor} of {} bytes",
            body.len()
        ));
    }

    Ok(DecodedMonsterMoveBody {
        body: MonsterMoveBody {
            mover: stable_object_guid(mover_low, mover_high),
            current_position,
            destination,
            crz_teleport,
            stop_distance_tolerance,
            flags,
            elapsed,
            move_time,
            fade_object_time,
            mode,
            transport: ExactObjectGuid {
                low: transport_low,
                high: transport_high,
            },
            vehicle_seat,
            face,
            vehicle_exit_voluntary,
            interpolate,
            points,
            packed_deltas,
            spline_filter,
            spell_effect_extra,
            jump_extra,
            anim_tier_transition,
        },
        mover_runtime_counter: mover_low & OBJECT_GUID_COUNTER_MASK,
        spline_id,
    })
}

/// Decode TrinityCore's signed quarter-yard `PackedXYZ` representation.
#[must_use]
pub fn unpack_monster_move_delta(packed: u32) -> [f32; 3] {
    fn sign_extend(value: u32, bits: u32) -> i32 {
        let shift = 32 - bits;
        ((value << shift) as i32) >> shift
    }

    [
        sign_extend(packed & 0x7FF, 11) as f32 * 0.25,
        sign_extend((packed >> 11) & 0x7FF, 11) as f32 * 0.25,
        sign_extend((packed >> 22) & 0x3FF, 10) as f32 * 0.25,
    ]
}

/// Decode the opcode-less body emitted by C++
/// `WorldPackets::Character::LogXPGain::Write`.
///
/// Source anchors (identical in both legacy references):
///
/// - `CharacterPackets.cpp`: packed Victim, Original, Reason, Amount,
///   GroupBonus in that order;
/// - `ObjectGuid.cpp::operator<<`: low mask, high mask, packed low bytes,
///   packed high bytes;
/// - `ObjectGuidFactory::CreateWorldObject`: lower 40 low-word bits are the
///   runtime counter.
pub fn decode_log_xp_gain_body(body: &[u8]) -> Result<LogXpGainBody, String> {
    decode_log_xp_gain_body_with_counter(body).map(|decoded| decoded.body)
}

/// Decode the opcode-less body emitted by C++
/// `WorldPackets::Loot::LootRemoved::Write`.
///
/// Source anchors:
///
/// - `LootPackets.cpp`: packed Owner, packed LootObj, LootListID;
/// - `ObjectGuid.cpp::operator<<`: low mask, high mask, packed low bytes,
///   packed high bytes;
/// - `ObjectGuidFactory::CreateWorldObject`: lower 40 low-word bits are the
///   map-runtime counter.
pub fn decode_loot_removed_body(body: &[u8]) -> Result<LootRemovedBody, String> {
    decode_loot_removed_body_with_counter(body).map(|decoded| decoded.body)
}

/// Decode the opcode-less body emitted by C++
/// `WorldPackets::Item::BuySucceeded::Write`.
pub fn decode_buy_succeeded_body(body: &[u8]) -> Result<BuySucceededBody, String> {
    decode_buy_succeeded_body_with_counter(body).map(|decoded| decoded.body)
}

/// Decode and canonicalize the opcode-less body emitted by C++
/// `WorldPackets::Spells::SendKnownSpells::Write`.
///
/// The first bit and both counts remain exact. Known and favorite spell IDs
/// are sorted only after canonical body decoding so the comparator mirrors
/// C++'s unordered `PlayerSpellMap` iteration without accepting a different
/// set. Duplicate IDs, favorite IDs absent from the known set, nonzero bit
/// padding, count/length mismatches, and spell ID zero are rejected.
pub fn decode_send_known_spells_body(body: &[u8]) -> Result<SendKnownSpellsBody, String> {
    if body.len() < 9 {
        return Err(format!(
            "SendKnownSpells body is {} bytes; need at least 9",
            body.len()
        ));
    }
    let bit_byte = body[0];
    if bit_byte & 0x7F != 0 {
        return Err(format!(
            "SendKnownSpells InitialLogin byte has non-canonical padding bits: 0x{bit_byte:02X}"
        ));
    }
    let initial_login = bit_byte & 0x80 != 0;
    let known_count = u32::from_le_bytes(body[1..5].try_into().expect("four-byte slice")) as usize;
    let favorite_count =
        u32::from_le_bytes(body[5..9].try_into().expect("four-byte slice")) as usize;
    let spell_count = known_count
        .checked_add(favorite_count)
        .ok_or_else(|| "SendKnownSpells spell counts overflow usize".to_string())?;
    let expected_len = spell_count
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(9))
        .ok_or_else(|| "SendKnownSpells body length overflows usize".to_string())?;
    if body.len() != expected_len {
        return Err(format!(
            "SendKnownSpells counts require {expected_len} bytes but body has {}",
            body.len()
        ));
    }

    let mut cursor = 9;
    let mut read_spells = |count: usize, label: &str| -> Result<Vec<u32>, String> {
        let mut spells = Vec::with_capacity(count);
        for index in 0..count {
            let end = cursor + 4;
            let spell = u32::from_le_bytes(
                body[cursor..end]
                    .try_into()
                    .expect("validated exact body length"),
            );
            cursor = end;
            if spell == 0 {
                return Err(format!("{label}[{index}] has invalid spell ID 0"));
            }
            spells.push(spell);
        }
        spells.sort_unstable();
        if let Some(duplicate) = spells.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(format!(
                "{label} contains duplicate spell ID {}",
                duplicate[0]
            ));
        }
        Ok(spells)
    };

    let known_spells = read_spells(known_count, "KnownSpells")?;
    let favorite_spells = read_spells(favorite_count, "FavoriteSpells")?;
    if let Some(spell) = favorite_spells
        .iter()
        .find(|spell| known_spells.binary_search(spell).is_err())
    {
        return Err(format!(
            "FavoriteSpells contains spell ID {spell} absent from KnownSpells"
        ));
    }

    Ok(SendKnownSpellsBody {
        initial_login,
        known_spells,
        favorite_spells,
    })
}

/// Decode the complete opcode-less C++ `WorldPackets::Spells::SpellGo` body.
///
/// This follows `operator<<(ByteBuffer&, SpellCastData const&)` field for
/// field, including both packed-bit headers and every optional vector. The
/// trailing `CombatLogServerPacket` bit must select the basic packet: a full
/// advanced-combat-log payload is deliberately rejected because accepting it
/// without decoding `SpellCastLogData` would ignore stable bytes.
///
/// The returned stable body normalizes only the lower 40-bit counters of the
/// correlated Creature caster and Cast GUID plus the wrapping CastTime. Exact
/// GUIDs and CastTime remain available on [`DecodedSpellGoBody`] for contract
/// validation and diagnostics.
pub fn decode_spell_go_body(body: &[u8]) -> Result<DecodedSpellGoBody, String> {
    decode_spell_cast_data_body(body, true)
}

/// Decode the complete opcode-less C++ `WorldPackets::Spells::SpellStart`
/// body. START has no combat-log suffix and its CastTime is retained as the
/// exact cast duration.
pub fn decode_spell_start_body(body: &[u8]) -> Result<DecodedSpellStartBody, String> {
    let decoded = decode_spell_cast_data_body(body, false)?;
    Ok(DecodedSpellStartBody {
        body: SpellStartBody {
            cast: decoded.body,
            cast_time: decoded.cast_time,
        },
        exact_caster_guid: decoded.exact_caster_guid,
        exact_caster_unit: decoded.exact_caster_unit,
        cast_id: decoded.cast_id,
    })
}

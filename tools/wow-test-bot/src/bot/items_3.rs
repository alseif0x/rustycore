//! Items operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn find_issue20_item_create_in_update_object(
    payload: &[u8],
    item_db_guid: u64,
    item_entry: u32,
    owner_db_guid: u64,
    runtime_realm_id: u16,
) -> Result<Option<Issue20ItemCreateEvidence>> {
    use sha2::{Digest, Sha256};

    let expected_item = item_guid_raw(item_db_guid, runtime_realm_id);
    let expected_owner = create_player_guid_raw(owner_db_guid, u32::from(runtime_realm_id));

    for block_start in 0..payload.len().saturating_sub(2) {
        if !matches!(payload[block_start], 1 | 2) {
            continue;
        }
        let mut cursor = block_start + 1;
        let Some((guid_len, item_low, item_high)) = parse_packed_guid(&payload[cursor..]) else {
            continue;
        };
        if (item_low, item_high) != expected_item {
            continue;
        }
        cursor += guid_len;

        let update_type = payload[block_start];
        if update_type != 1 {
            bail!("issue #20 loaded item used CreateObject2 instead of C++ existing-object CreateObject");
        }
        if issue20_take_u8(payload, &mut cursor, "TypeID")? != 1 {
            bail!("issue #20 loaded fixture was not serialized as TYPEID_ITEM");
        }
        let create_bits_end = cursor
            .checked_add(3)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| anyhow!("truncated issue #20 item CreateObject flags"))?;
        if payload[cursor..create_bits_end] != [0, 0, 0] {
            bail!("issue #20 loaded item CreateObject carried nonzero movement flags");
        }
        cursor = create_bits_end;
        if issue20_take_i32(payload, &mut cursor, "PauseTimes")? != 0 {
            bail!("issue #20 loaded item CreateObject carried pause times");
        }

        let values_len = issue20_take_u32(payload, &mut cursor, "values length")? as usize;
        let values_end = cursor
            .checked_add(values_len)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| anyhow!("truncated issue #20 item CreateObject values"))?;
        if issue20_take_u8(payload, &mut cursor, "UpdateFieldFlags")? != 0x01 {
            bail!("issue #20 loaded item CreateObject was not owner-visible");
        }
        if issue20_take_i32(payload, &mut cursor, "EntryID")? != item_entry as i32
            || issue20_take_u32(payload, &mut cursor, "ObjectData.DynamicFlags")? != 0
            || issue20_take_u32(payload, &mut cursor, "ObjectData.Scale")? != 1.0_f32.to_bits()
        {
            bail!("issue #20 loaded item ObjectData did not match the isolated fixture");
        }
        let owner = issue20_take_packed_guid(payload, &mut cursor, "Owner")?;
        let contained_in = issue20_take_packed_guid(payload, &mut cursor, "ContainedIn")?;
        let creator = issue20_take_packed_guid(payload, &mut cursor, "Creator")?;
        let gift_creator = issue20_take_packed_guid(payload, &mut cursor, "GiftCreator")?;
        if owner != expected_owner
            || contained_in != expected_owner
            || creator != (0, 0)
            || gift_creator != (0, 0)
        {
            bail!("issue #20 loaded item ownership GUIDs did not match the fixture player");
        }
        if issue20_take_i32(payload, &mut cursor, "StackCount")? != 1
            || issue20_take_i32(payload, &mut cursor, "Expiration")? != 0
        {
            bail!("issue #20 loaded item count/expiration did not match the fixture");
        }
        for _ in 0..5 {
            if issue20_take_i32(payload, &mut cursor, "SpellCharges")? != 0 {
                bail!("issue #20 loaded item carried unexpected spell charges");
            }
        }
        let _dynamic_flags = issue20_take_u32(payload, &mut cursor, "ItemData.DynamicFlags")?;

        let mut enchantments = [0; ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT];
        for (slot, enchantment) in enchantments.iter_mut().enumerate() {
            *enchantment = issue20_take_i32(payload, &mut cursor, "Enchantment.ID")?;
            let duration = issue20_take_u32(payload, &mut cursor, "Enchantment.Duration")?;
            let charges = issue20_take_u16(payload, &mut cursor, "Enchantment.Charges")?;
            let field_a = issue20_take_u8(payload, &mut cursor, "Enchantment.FieldA")?;
            let field_b = issue20_take_u8(payload, &mut cursor, "Enchantment.FieldB")?;
            if duration != 0 || charges != 0 || field_a != 0 || field_b != 0 {
                bail!(
                    "issue #20 loaded item enchantment slot {slot} carried unexpected auxiliary fields: duration={duration} charges={charges} fields={field_a}/{field_b}"
                );
            }
        }
        let random_properties_seed = issue20_take_i32(payload, &mut cursor, "PropertySeed")?;
        let random_properties_id = issue20_take_i32(payload, &mut cursor, "RandomPropertiesID")?;
        if enchantments != issue20_expected_enchantment_ids()
            || random_properties_seed != 0
            || random_properties_id != ISSUE20_ITEM_RANDOM_PROPERTY_ID
        {
            bail!(
                "issue #20 loaded item lost enchant/random metadata: enchantments={enchantments:?} random={random_properties_seed}/{random_properties_id}"
            );
        }

        let durability = issue20_take_u32(payload, &mut cursor, "Durability")?;
        let max_durability = issue20_take_u32(payload, &mut cursor, "MaxDurability")?;
        let create_played_time = issue20_take_u32(payload, &mut cursor, "CreatePlayedTime")?;
        let context = issue20_take_i32(payload, &mut cursor, "Context")?;
        let create_time = issue20_take_u64(payload, &mut cursor, "CreateTime")?;
        let artifact_xp = issue20_take_u64(payload, &mut cursor, "ArtifactXP")?;
        let item_appearance_mod_id = issue20_take_u8(payload, &mut cursor, "ItemAppearanceModID")?;
        let artifact_power_count = issue20_take_u32(payload, &mut cursor, "ArtifactPowers.Size")?;
        let gem_count = issue20_take_u32(payload, &mut cursor, "Gems.Size")?;
        let dynamic_flags2 = issue20_take_u32(payload, &mut cursor, "DynamicFlags2")?;
        let bonus_key_item_id = issue20_take_i32(payload, &mut cursor, "ItemBonusKey.ItemID")?;
        let bonus_list_count =
            issue20_take_u32(payload, &mut cursor, "ItemBonusKey.BonusListIDs.Size")?;
        let debug_item_level = issue20_take_u16(payload, &mut cursor, "DEBUGItemLevel")?;
        let modifier_count_bits = issue20_take_u8(payload, &mut cursor, "ItemModList.Values.Size")?;
        if durability != 0
            || max_durability != 0
            || create_played_time != 0
            || context != 0
            || create_time != 0
            || artifact_xp != 0
            || item_appearance_mod_id != 0
            || artifact_power_count != 0
            || gem_count != 0
            || dynamic_flags2 != 0
            || bonus_key_item_id != 0
            || bonus_list_count != 0
            || debug_item_level != 0
            || modifier_count_bits != 0
        {
            bail!("issue #20 loaded item CreateObject tail did not match the isolated fixture");
        }
        if cursor != values_end {
            bail!(
                "issue #20 loaded item CreateObject did not consume its complete values block: parsed={cursor} declared_end={values_end}"
            );
        }

        let block_sha256 = hex::encode(Sha256::digest(&payload[block_start..values_end]));
        return Ok(Some(Issue20ItemCreateEvidence {
            block_sha256,
            enchantments,
            random_properties_seed,
            random_properties_id,
        }));
    }
    Ok(None)
}
pub(crate) fn record_issue20_item_create_evidence(
    payload: &[u8],
    options: &InventorySwapSmokeOptions,
    owner_db_guid: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    let runtime_realm_id = u16::try_from(realm_id())
        .map_err(|_| anyhow!("runtime realm ID does not fit issue #20 item GUID"))?;
    let Some(evidence) = find_issue20_item_create_in_update_object(
        payload,
        options.item_guid_a,
        options.item_entry_a,
        owner_db_guid,
        runtime_realm_id,
    )?
    else {
        return Ok(());
    };
    if let Some(previous) = result.inventory_swap_item_create_sha256.as_deref() {
        if previous != evidence.block_sha256 {
            bail!("issue #20 login published two different CreateObject blocks for one item");
        }
    } else {
        info!(
            "Issue #20 loaded item CREATE_OBJECT exact block SHA-256: {}",
            evidence.block_sha256
        );
        result.inventory_swap_item_create_sha256 = Some(evidence.block_sha256);
    }
    Ok(())
}

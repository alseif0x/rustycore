// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The player-item spell paths (#1363 slice 12b2), moved from the World
//! `session/player_items/{durability,storage}.rs`: C++
//! `Spell::EffectDurabilityDamage` and `Player::ApplyItemObtainSpells`.

use wow_constants::{ItemFlags, ItemSpelltriggerType};
use wow_core::ObjectGuid;
use wow_entities::{INVENTORY_SLOT_BAG_END, SpellCastMetadata};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;
use wow_world_inventory::VendorSessionHostLikeCpp;

use crate::PlayerCastHostLikeCpp;
use crate::SpellEffectHostLikeCpp;

/// C++ `Spell::EffectDurabilityDamage` (`SpellEffects.cpp:4316-4352`).
pub fn apply_durability_damage_effect_like_cpp<
    H: SpellEffectHostLikeCpp + VendorSessionHostLikeCpp,
>(
    host: &mut H,
    effect: u32,
    damage: i32,
    slot: i32,
    target_guid: ObjectGuid,
) {
    if host.player_guid() != Some(target_guid) {
        return;
    }
    let effect = i32::try_from(effect).unwrap_or(0);
    if slot < 0 {
        wow_world_inventory::apply_represented_durability_points_loss_all_like_cpp(
            host,
            damage,
            slot < -1,
        );
        // C++ logs `-1`/`-1` for the all-items branch
        // (`SpellEffects.cpp:4328`).
        crate::record_spell_execute_log_durability_damage_like_cpp(
            host,
            effect,
            target_guid,
            -1,
            -1,
        );
        return;
    }
    let Ok(slot) = u8::try_from(slot) else {
        return;
    };
    if slot >= INVENTORY_SLOT_BAG_END {
        return;
    }
    let item_entry = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_like_cpp(hub, slot)
    })
    .map(|item| item.entry_id);
    if wow_world_inventory::apply_represented_durability_points_loss_at_slot_like_cpp(
        host, slot, damage,
    ) && let Some(item_entry) = item_entry
    {
        // C++ logs the item entry and the slot as `ItemID`/`Amount`
        // (`SpellEffects.cpp:4339`).
        crate::record_spell_execute_log_durability_damage_like_cpp(
            host,
            effect,
            target_guid,
            i32::try_from(item_entry).unwrap_or(i32::MAX),
            i32::from(slot),
        );
    }
}

/// C++ `Player::ApplyItemObtainSpells(item, true)` after `_StoreItem`.
pub async fn apply_inventory_item_obtain_spells_with_generator_like_cpp<
    H: PlayerCastHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    item_guid: ObjectGuid,
) -> Result<Vec<i32>, &'static str> {
    let (item_entry, item_level) = host
        .server_normal_physical_item_facts_like_cpp(item_guid)
        .ok_or("physical obtain item owner or effective item level unavailable")?;
    let item_flags = host
        .hub_ref()
        .catalogs
        .item_template_flags(item_entry)
        .ok_or("physical obtain item template unavailable")?;
    if item_flags.contains(ItemFlags::LEGACY) {
        return Ok(Vec::new());
    }
    let effect_store = host
        .hub_ref()
        .catalogs
        .items
        .effect_store
        .as_ref()
        .ok_or("ItemEffect catalog unavailable for obtain spell")?;
    let spell_ids = effect_store
        .item_effects_for_item_id_like_cpp(item_entry)
        .into_iter()
        .filter(|effect| {
            effect.trigger_type == ItemSpelltriggerType::OnPickup as i8 && effect.spell_id > 0
        })
        .map(|effect| effect.spell_id)
        .collect::<Vec<_>>();
    let Some(player_guid) = host.player_guid() else {
        return Err("Player identity unavailable for obtain spell");
    };
    let mut applied = Vec::new();
    for spell_id in spell_ids {
        let has_aura = {
            let hub = host.hub_ref();
            if hub.player_aura_authority_ready_like_cpp() != Some(true) {
                return Err("complete untombstoned Player aura authority unavailable");
            }
            hub.resolved_player_visible_auras_like_cpp()
                .ok_or("Player aura applications unavailable")?
                .values()
                .any(|aura| aura.spell_id == spell_id)
        };
        if has_aura {
            continue;
        }
        let metadata = SpellCastMetadata {
            cast_item_entry: Some(item_entry),
            cast_item_guid: Some(item_guid),
            cast_item_level: Some(item_level),
            ..SpellCastMetadata::default()
        };
        // C++ `ApplyItemObtainSpells` uses normal `CastSpell` with a physical
        // cast item and default CastSpellExtraArgs trigger flags.
        if crate::cast_server_normal_player_spell_with_generator_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            player_guid,
            metadata,
        )
        .await
        {
            applied.push(spell_id);
        }
    }
    Ok(applied)
}

/// C++ `Player::CastAllObtainSpells` over the live backpack and equipped-bag
/// positions. Each physical item and each `HasAura` query is resolved again
/// from the current owner after the preceding cast returns.
pub async fn cast_all_obtain_spells_like_cpp<H: PlayerCastHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
) -> Result<(), &'static str> {
    let positions = host
        .core()
        .with_owned_player_like_cpp(|player| player.cast_all_obtain_item_positions_like_cpp())
        .ok_or("canonical inventory owner unavailable for obtain scan")?
        .ok_or("complete backpack and bag inventory authority unavailable")?;

    for (bag, slot) in positions {
        let item_guid = host
            .core()
            .with_owned_player_like_cpp(|player| {
                if !player
                    .inventory_runtime_like_cpp()
                    .equipment_inventory_authority_complete_like_cpp()
                {
                    return Err("inventory authority became unavailable during obtain scan");
                }
                let Some(item_guid) = player.get_item_by_pos(bag, slot) else {
                    return Ok(None);
                };
                let Some(item) = player
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .get(&item_guid)
                else {
                    return Err("live obtain slot has no canonical physical item");
                };
                if item.owner_guid() != player.guid()
                    || item.update_state() == wow_constants::ItemUpdateState::Removed
                {
                    return Err("live obtain slot physical item identity is invalid");
                }
                Ok(Some(item_guid))
            })
            .ok_or("canonical inventory owner disappeared during obtain scan")??;
        let Some(item_guid) = item_guid else {
            continue;
        };
        Box::pin(apply_inventory_item_obtain_spells_with_generator_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            item_guid,
        ))
        .await?;
    }
    Ok(())
}

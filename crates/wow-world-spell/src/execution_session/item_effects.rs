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
    let owner = obtain_player_residence_like_cpp(host);
    if host.core().player_handle_like_cpp.is_some() && owner.is_none() {
        return Err("active canonical Player residence unavailable for item obtain cast");
    }
    // C++ checks the physical item's template flags and ItemEffect rows
    // before a possible CastSpell constructs a Spell and reads item level.
    let physical_item_entry = |player: &wow_entities::Player| {
        player
            .inventory_runtime_like_cpp()
            .item_objects()
            .get(&item_guid)
            .filter(|item| {
                item.owner_guid() == player.guid()
                    && item.update_state() != wow_constants::ItemUpdateState::Removed
            })
            .map(|item| item.object().entry())
    };
    let item_entry = host
        .core()
        .with_owned_player_like_cpp(physical_item_entry);
    #[cfg(any(test, feature = "test-fixtures"))]
    let item_entry = item_entry.or_else(|| {
        host.core()
            .with_owned_player_for_rest_like_cpp(physical_item_entry)
    });
    let item_entry = item_entry
        .ok_or("physical obtain item owner or effective item level unavailable")?
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
        if let Some(owner) = owner {
            ensure_obtain_player_residence_like_cpp(host, owner)?;
        }
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
        // C++ `Spell::prepare` reads cast-item identity and item level for
        // every CastSpell call; an earlier effect may have changed the live
        // item or Player level while the selected ItemEffect sequence remains
        // fixed for this invocation.
        let (cast_item_entry, cast_item_level) = host
            .server_normal_physical_item_facts_like_cpp(item_guid)
            .ok_or("physical obtain item owner or effective item level unavailable")?;
        let metadata = SpellCastMetadata {
            cast_item_entry: Some(cast_item_entry),
            cast_item_guid: Some(item_guid),
            cast_item_level: Some(cast_item_level),
            ..SpellCastMetadata::default()
        };
        // C++ `ApplyItemObtainSpells` uses normal `CastSpell` with a physical
        // cast item and default CastSpellExtraArgs trigger flags.
        let casted = crate::try_cast_server_normal_player_spell_with_generator_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            player_guid,
            metadata,
        )
        .await?;
        if let Some(owner) = owner {
            ensure_obtain_player_residence_like_cpp(host, owner)?;
        }
        if casted {
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
    let owner = obtain_player_residence_like_cpp(host)
        .ok_or("active canonical Player residence unavailable for obtain scan")?;
    let backpack_end = host
        .core()
        .with_owned_player_like_cpp(|player| player.cast_all_obtain_backpack_end_like_cpp())
        .ok_or("canonical inventory owner unavailable for obtain scan")?
        .ok_or("complete backpack and bag inventory authority unavailable")?;

    // C++ freezes INVENTORY_SLOT_ITEM_START + GetInventorySlotCount() once.
    for slot in wow_entities::INVENTORY_SLOT_ITEM_START..backpack_end {
        ensure_obtain_player_residence_like_cpp(host, owner)?;
        let item_guid = host
            .core()
            .with_owned_player_like_cpp(|player| {
                player.cast_all_obtain_item_at_like_cpp(
                    wow_entities::INVENTORY_SLOT_BAG_0,
                    slot,
                    None,
                )
            })
            .ok_or("canonical inventory owner disappeared during obtain scan")?
            .ok_or("inventory authority became unavailable during obtain scan")?;
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
        ensure_obtain_player_residence_like_cpp(host, owner)?;
    }

    // C++ resolves each bag only after all earlier backpack/bag casts return.
    for bag_slot in wow_entities::INVENTORY_SLOT_BAG_START..wow_entities::INVENTORY_SLOT_BAG_END {
        ensure_obtain_player_residence_like_cpp(host, owner)?;
        let bag = host
            .core()
            .with_owned_player_like_cpp(|player| player.cast_all_obtain_bag_like_cpp(bag_slot))
            .ok_or("canonical inventory owner disappeared during obtain bag scan")?
            .ok_or("inventory authority became unavailable during obtain bag scan")?;
        let Some((bag_guid, bag_size)) = bag else {
            continue;
        };

        for slot in 0..bag_size {
            ensure_obtain_player_residence_like_cpp(host, owner)?;
            let item_guid = host
                .core()
                .with_owned_player_like_cpp(|player| {
                    player.cast_all_obtain_item_at_like_cpp(bag_slot, slot, Some(bag_guid))
                })
                .ok_or("canonical inventory owner disappeared during obtain bag scan")?
                .ok_or("inventory authority became unavailable during obtain bag scan")?;
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
            ensure_obtain_player_residence_like_cpp(host, owner)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ObtainPlayerResidenceLikeCpp {
    handle: wow_map::PlayerHandle,
    guid: ObjectGuid,
    map_key: wow_map::MapKey,
    revision: u64,
}

fn obtain_player_residence_like_cpp<H: PlayerCastHostLikeCpp>(
    host: &H,
) -> Option<ObtainPlayerResidenceLikeCpp> {
    let handle = host.core().player_handle_like_cpp?;
    let guid = host.player_guid()?;
    if handle.guid() != guid {
        return None;
    }
    let manager = host.core().canonical_map_manager.as_ref()?.lock().ok()?;
    let (map_key, revision) = manager.player_active_residence_revision_like_cpp(handle)?;
    drop(manager);
    host.core()
        .with_owned_player_like_cpp(|player| player.guid() == guid)
        .filter(|matches| *matches)?;
    Some(ObtainPlayerResidenceLikeCpp {
        handle,
        guid,
        map_key,
        revision,
    })
}

fn ensure_obtain_player_residence_like_cpp<H: PlayerCastHostLikeCpp>(
    host: &H,
    expected: ObtainPlayerResidenceLikeCpp,
) -> Result<(), &'static str> {
    if obtain_player_residence_like_cpp(host) == Some(expected) {
        Ok(())
    } else {
        Err("canonical Player incarnation or residence changed during obtain scan")
    }
}

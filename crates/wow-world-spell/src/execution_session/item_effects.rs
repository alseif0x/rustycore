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

use super::SpellExecutionHostLikeCpp;
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
    H: SpellExecutionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    item_entry: u32,
) -> Vec<i32> {
    if host
        .hub_ref()
        .catalogs
        .item_template_flags(item_entry)
        .is_some_and(|flags| flags.contains(ItemFlags::LEGACY))
    {
        return Vec::new();
    }
    let spell_ids = host
        .hub_ref()
        .catalogs
        .items
        .effect_store
        .as_ref()
        .map(|store| {
            store
                .item_effects_for_item_id_like_cpp(item_entry)
                .into_iter()
                .filter(|effect| {
                    effect.trigger_type == ItemSpelltriggerType::OnPickup as i8
                        && effect.spell_id > 0
                })
                .map(|effect| effect.spell_id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let Some(player_guid) = host.player_guid() else {
        return Vec::new();
    };
    let Some(visible_auras) = host.hub_ref().resolved_player_visible_auras_like_cpp() else {
        return Vec::new();
    };
    let mut applied = Vec::new();
    for spell_id in spell_ids {
        if visible_auras.values().any(|aura| aura.spell_id == spell_id) {
            continue;
        }
        // C++ `Player::ApplyItemObtainSpells` uses
        // `CastSpellExtraArgs().SetCastItem(item)`, whose default trigger
        // flags are TRIGGERED_NONE: not a triggered cast, and the global
        // cooldown applies.
        if crate::execute_server_triggered_spell_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            player_guid,
            SpellCastMetadata::default(),
        )
        .await
        .is_ok()
        {
            applied.push(spell_id);
        }
    }
    applied
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented item durability loss (#1363 slice 12b1; moved from the World
//! `session/player_items/durability.rs`): `Player::DurabilityLossAll`,
//! `DurabilityPointsLossAll`, `DurabilityLoss`, `DurabilityPointsLoss`, the
//! `Spell::EffectDurabilityDamagePCT` slot and all-items branches and the
//! `Unit::Kill` death-loss branches.

use wow_constants::ItemUpdateState;
use wow_core::ObjectGuid;
use wow_entities::{
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, is_equipment_packed_pos, make_item_pos,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::{
    ITEM_DATA_BITS, ITEM_DATA_DURABILITY_BIT, ItemDataUpdate, ItemValuesUpdate, TYPEID_ITEM,
    UpdateMask,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::entity_update_bridge::item_values_update_to_update_object;

use super::item_mods::{
    record_represented_item_mods_like_cpp, send_represented_item_bonus_player_stat_update_like_cpp,
};
use crate::VendorSessionHostLikeCpp;

/// C++ `Player::DurabilityLossAll` (`Player.cpp:4522-4544`).
///
/// Equipment is always processed; `inventory` additionally walks the
/// backpack slots and every bag's contents, matching C++.
pub fn apply_represented_durability_loss_all_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    percent: f64,
    inventory: bool,
) -> usize {
    let targets = {
        let (s, h) = host.split_inventory_ref();
        s.represented_durability_targets_like_cpp(h, inventory)
    };
    let mut affected = 0;
    for (guid, slot, equipped) in targets {
        if apply_represented_durability_loss_item_like_cpp(host, guid, slot, equipped, percent) {
            affected += 1;
        }
    }
    affected
}

/// Spirit-healer `Player::DurabilityLossAll(0.25f, true)`; native item VALUES
/// are batched by the owner's Map, while stat callbacks retain their order.
pub fn apply_spirit_healer_durability_loss<H: VendorSessionHostLikeCpp>(host: &mut H) -> usize {
    let targets = {
        let (state, hub) = host.split_inventory_ref();
        state.represented_durability_targets_like_cpp(hub, true)
    };
    let mut affected = 0;
    let mut equipped_became_broken = false;
    for (item_guid, slot, equipped) in targets {
        let Some(was_broken) = ({
            let (state, hub) = host.split_inventory_ref();
            state
                .resolved_inventory_item_object_like_cpp(hub, item_guid)
                .map(|item| item.is_broken())
        }) else {
            continue;
        };
        if !apply_represented_durability_loss_item_like_cpp(host, item_guid, slot, equipped, 0.25) {
            continue;
        }
        affected += 1;

        if let Some(item) = {
            let (state, hub) = host.split_inventory_ref();
            state.resolved_inventory_item_object_like_cpp(hub, item_guid)
        } {
            equipped_became_broken |= equipped && !was_broken && item.is_broken();
            // Handle-less legacy fixtures have no native Map publication owner.
            #[cfg(any(test, feature = "test-fixtures"))]
            if host.core().player_handle_like_cpp.is_none() {
                let mut item_data_mask = UpdateMask::new(ITEM_DATA_BITS);
                item_data_mask.set(ITEM_DATA_DURABILITY_BIT);
                let durability_update = ItemValuesUpdate {
                    changed_object_type_mask: 1 << TYPEID_ITEM,
                    object_data: None,
                    item_data: Some(ItemDataUpdate {
                        mask: item_data_mask,
                        values: item.data().clone(),
                    }),
                };
                if let Some(update) = item_values_update_to_update_object(
                    item_guid,
                    host.core().player_map_id_like_cpp(),
                    &durability_update,
                ) {
                    host.send_packet(&update);
                }
            }
        }
    }
    if equipped_became_broken {
        send_represented_item_bonus_player_stat_update_like_cpp(host);
    }
    affected
}

/// C++ `Player::DurabilityPointsLossAll` (`Player.cpp:4566-4588`).
pub fn apply_represented_durability_points_loss_all_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    points: i32,
    inventory: bool,
) -> usize {
    let targets = {
        let (s, h) = host.split_inventory_ref();
        s.represented_durability_targets_like_cpp(h, inventory)
    };
    let mut affected = 0;
    for (guid, slot, equipped) in targets {
        if apply_represented_durability_points_loss_like_cpp(
            host,
            guid,
            slot,
            equipped,
            i64::from(points),
        ) {
            affected += 1;
        }
    }
    affected
}

/// C++ `Spell::EffectDurabilityDamage` slot branch: one top-level
/// `INVENTORY_SLOT_BAG_0` slot, resolved through `GetItemByPos`.
pub fn apply_represented_durability_points_loss_at_slot_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    slot: u8,
    points: i32,
) -> bool {
    let Some(item) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_like_cpp(hub, slot)
    }) else {
        return false;
    };
    let equipped = is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot));
    apply_represented_durability_points_loss_like_cpp(
        host,
        item.guid,
        slot,
        equipped,
        i64::from(points),
    )
}

/// C++ `Spell::EffectDurabilityDamagePCT` slot branch.
fn apply_represented_durability_loss_at_slot_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    slot: u8,
    percent: f64,
) -> bool {
    let Some(item) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_like_cpp(hub, slot)
    }) else {
        return false;
    };
    let equipped = is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot));
    apply_represented_durability_loss_item_like_cpp(host, item.guid, slot, equipped, percent)
}

/// C++ `Player::DurabilityLoss` (`Player.cpp:4546-4562`).
fn apply_represented_durability_loss_item_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
    slot: u8,
    equipped: bool,
    percent: f64,
) -> bool {
    let Some(item) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, item_guid)
    }) else {
        return false;
    };
    let max_durability = item.data().max_durability;
    if max_durability == 0 {
        return false;
    }
    let percent = percent / f64::from(represented_durability_loss_aura_multiplier_like_cpp(host));
    let points = ((f64::from(max_durability) * percent) as u32).max(1);
    apply_represented_durability_points_loss_like_cpp(
        host,
        item_guid,
        slot,
        equipped,
        i64::from(points),
    )
}

/// C++ `Player::DurabilityPointsLoss` (`Player.cpp:4590-4620`).
///
/// Mods are removed *before* the durability write so the represented
/// `_ApplyItemMods` gate still sees the item as unbroken, then restored
/// after, exactly like C++.
fn apply_represented_durability_points_loss_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
    slot: u8,
    equipped: bool,
    points: i64,
) -> bool {
    if represented_prevent_durability_loss_like_cpp(host) {
        return false;
    }
    let Some(item) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, item_guid)
    }) else {
        return false;
    };
    let max_durability = i64::from(item.data().max_durability);
    let old_durability = i64::from(item.data().durability);
    let new_durability = (old_durability - points).clamp(0, max_durability);
    if old_durability == new_durability {
        return false;
    }
    if new_durability == 0 && old_durability > 0 && equipped {
        record_represented_item_mods_like_cpp(host, item_guid, slot, false);
    }
    let updated = {
        let (state, mut hub) = host.split_inventory_mut();
        // Player.cpp:4610-4616 commits the durability and persistent Changed state together.
        state.apply_inventory_item_object_updates_like_cpp(
            &mut hub,
            item_guid,
            &[
                wow_entities::ItemObjectUpdateLikeCpp::SetDurability(
                    u32::try_from(new_durability).unwrap_or(0),
                ),
                wow_entities::ItemObjectUpdateLikeCpp::SetState(ItemUpdateState::Changed),
                wow_entities::ItemObjectUpdateLikeCpp::QueueDurabilityUpdate,
            ],
        )
    };
    if new_durability > 0 && old_durability == 0 && equipped {
        record_represented_item_mods_like_cpp(host, item_guid, slot, true);
    }
    updated
}

/// C++ `Spell::EffectDurabilityDamagePCT` (`SpellEffects.cpp:4354-4373`).
///
/// C++ has no `ExecuteLogEffectDurabilityDamage` call on this branch, so
/// the percent variant publishes no execute-log row.
pub fn apply_durability_damage_pct_effect_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    damage: i32,
    slot: i32,
    target_guid: ObjectGuid,
) {
    if host.player_guid() != Some(target_guid) {
        return;
    }
    if slot < 0 {
        apply_represented_durability_loss_all_like_cpp(host, f64::from(damage) / 100.0, slot < -1);
        return;
    }
    let Ok(slot) = u8::try_from(slot) else {
        return;
    };
    if slot >= INVENTORY_SLOT_BAG_END || damage <= 0 {
        return;
    }
    apply_represented_durability_loss_at_slot_like_cpp(host, slot, f64::from(damage) / 100.0);
}

fn represented_durability_loss_aura_multiplier_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &H,
) -> f32 {
    let (state, hub) = host.split_inventory_ref();
    state.represented_durability_loss_aura_multiplier_like_cpp(hub)
}

fn represented_prevent_durability_loss_like_cpp<H: VendorSessionHostLikeCpp>(host: &H) -> bool {
    let (state, hub) = host.split_inventory_ref();
    state.represented_prevent_durability_loss_like_cpp(hub)
}

/// C++ `Unit::Kill` player-victim durability branch (`Unit.cpp:10639-10648`).
///
/// Applies `Player::DurabilityLossAll(baseLoss, false)` and returns the
/// `SMSG_DURABILITY_DAMAGE_DEATH` percent C++ derives as
/// `baseLoss - baseLoss * GetTotalAuraMultiplier(MOD_DURABILITY_LOSS)`.
/// C++ truncates that value to `uint32`, so with no aura the message percent
/// is 0; that legacy behaviour is reproduced rather than repaired.
pub fn apply_represented_durability_loss_on_death_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
) -> u32 {
    let base_loss = f64::from(
        host.hub_ref()
            .config
            .durability_loss_on_death_rate_like_cpp(),
    );
    let multiplier = f64::from(represented_durability_loss_aura_multiplier_like_cpp(host));
    let loss = (base_loss - base_loss * multiplier) as u32;
    apply_represented_durability_loss_all_like_cpp(host, base_loss, false);
    loss
}

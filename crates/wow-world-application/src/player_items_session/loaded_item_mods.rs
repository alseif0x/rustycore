// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The loaded item-mod replay of C++ `Player::_ApplyAllItemMods` with its
//! item-set auras, equip auras and loaded enchantments (#1363 slice 12b2b),
//! moved from the World `session/player_items/{modifiers,enchantment,
//! persistence_load}.rs`. C++ anchors stay on each body.
//!
//! Auras are applied and removed through the application aura-application
//! context, built here from the World session's disjoint fields
//! ([`crate::TrainerSessionHostLikeCpp::split_trainer_mut`]) with the same
//! participants as the World `player_aura_application_cx_like_cpp`.

use wow_constants::EnchantmentSlot;
use wow_core::ObjectGuid;
use wow_entities::{
    ApplyEnchantmentArgs, ApplyEnchantmentDurationAction, ApplyEnchantmentEffectAction,
    ApplyEnchantmentPlan, ApplyEnchantmentResult,
};
use wow_world_core::session::AFLAG_NOCASTER_LIKE_CPP;
use wow_world_inventory::{
    BattlePetSessionHostLikeCpp, InitialLoadedItemModsOutcomeLikeCpp,
    LoadedEquippedItemEnchantmentsOutcomeLikeCpp, RepresentedItemBonusActionLikeCpp,
    VendorSessionHostLikeCpp,
};
use wow_world_quest::QuestItemStorageHostLikeCpp;
use wow_world_spell::unit_owned_apply_aura_effect_mask_like_cpp;

use crate::{TrainerSessionFieldsLikeCpp, TrainerSessionHostLikeCpp};

/// The World operation the loaded item-mod replay calls beyond its hosts.
pub trait LoadedItemModsHostLikeCpp:
    TrainerSessionHostLikeCpp
    + VendorSessionHostLikeCpp
    + BattlePetSessionHostLikeCpp
    + QuestItemStorageHostLikeCpp
{
    /// The World item-enchantment context's current-player plan
    /// (`Player::ApplyEnchantment` planning over the canonical Player).
    fn apply_current_player_item_enchantment_plan_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        args: ApplyEnchantmentArgs,
    ) -> Option<ApplyEnchantmentPlan>;
}

/// The aura-application context over the World session's fields.
fn session_aura_application_cx_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
) -> crate::PlayerAuraApplicationCxLikeCpp<'_> {
    let world_test = host.world_test_consumer_like_cpp();
    let TrainerSessionFieldsLikeCpp {
        hub,
        spell_state,
        inventory,
        loot,
        #[cfg(any(test, feature = "test-fixtures"))]
        quest_state,
        ..
    } = host.split_trainer_mut();
    crate::player_aura_application_cx_like_cpp(
        hub,
        spell_state,
        inventory,
        loot,
        #[cfg(any(test, feature = "test-fixtures"))]
        quest_state,
        world_test,
    )
}

/// Replays the aura-producing part of C++ `Player::_ApplyAllItemMods` after
/// `Player::_LoadAuras`. C++ walks equipment slots and, for each item,
/// applies its item-set effect, regular equip spell, and enchantments before
/// advancing to the next slot; preserve that exact order for aura slots.
pub fn apply_initial_loaded_item_mods_like_cpp<H: LoadedItemModsHostLikeCpp>(
    host: &mut H,
    loaded_equipped_item_guids: &[ObjectGuid],
) -> InitialLoadedItemModsOutcomeLikeCpp {
    // This is the initial C++ `_ApplyAllItemMods` replay for a newly
    // constructed Player. Start from the same empty modifier state even
    // after a failed/retried login that did not reach normal teardown.
    {
        let (s, mut h) = host.split_inventory_mut();
        s.reset_represented_item_bonus_runtime_like_cpp(&mut h)
    };

    let mut equipped = loaded_equipped_item_guids
        .iter()
        .filter_map(|&item_guid| {
            host.resolved_inventory_item_object_like_cpp(item_guid)
                .map(|item| (item.slot(), item_guid))
        })
        .collect::<Vec<_>>();
    equipped.sort_by_key(|(slot, guid)| (*slot, guid.counter()));

    let mut item_set_auras = 0usize;
    let mut item_equip_auras = 0usize;
    let mut enchantments = LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
    for (slot, item_guid) in equipped {
        // C++ `_ApplyAllItemMods` starts each equipped item with
        // `_ApplyItemBonuses`. Seed the canonical Player accumulator here
        // so login and a later equip/swap use exactly the same contribution
        // path. Broken items are rejected before both static/scaling
        // bonuses and aura/enchantment effects are considered.
        if host
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| !item.is_broken())
        {
            wow_world_inventory::record_represented_item_mods_like_cpp(host, item_guid, slot, true);
        }
        item_set_auras += apply_initial_item_set_auras_like_cpp(host, item_guid);
        item_equip_auras += session_aura_application_cx_like_cpp(host)
            .apply_initial_item_equip_auras_like_cpp(item_guid);
        enchantments.append(apply_loaded_equipped_item_enchantments_like_cpp(
            host, item_guid,
        ));
    }

    InitialLoadedItemModsOutcomeLikeCpp {
        item_set_auras,
        item_equip_auras,
        enchantments,
    }
}
fn apply_initial_item_set_auras_like_cpp<H: LoadedItemModsHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
) -> usize {
    let Some(player_guid) = host.player_guid() else {
        return 0;
    };
    let events = wow_world_inventory::record_represented_items_set_item_events_like_cpp(
        host, item_guid, true,
    );
    let mut applied = 0usize;
    for event in events {
        if !event.apply {
            continue;
        }
        let Ok(spell_id) = i32::try_from(event.spell_id) else {
            continue;
        };
        if host
            .hub_ref()
            .player_has_visible_aura_spell_like_cpp(spell_id)
            != Some(false)
        {
            continue;
        }
        let effect_mask = host
            .hub_ref()
            .catalogs
            .spell_store()
            .and_then(|store| store.get(spell_id))
            .map(unit_owned_apply_aura_effect_mask_like_cpp)
            .unwrap_or(0x0000_0001)
            .max(0x0000_0001);
        if session_aura_application_cx_like_cpp(host)
            .apply_aura_with_effect_mask_like_cpp(
                spell_id,
                player_guid,
                0,
                AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                effect_mask,
            )
            .is_ok()
        {
            applied += 1;
        }
    }
    applied
}
/// C++ `Player::_LoadInventory` finishes by `_ApplyAllItemMods`, which in
/// turn calls `ApplyEnchantment(m_items[i], true)` for equipped items.
pub fn apply_loaded_equipped_item_enchantments_like_cpp<H: LoadedItemModsHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
) -> LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
    let Some(item) = host.resolved_inventory_item_object_like_cpp(item_guid) else {
        return LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
    };
    // `_ApplyAllItemMods` visits the broad top-level range, but C++
    // `ApplyEnchantment` immediately returns unless `Item::IsEquipped`.
    if !item.is_equipped() {
        return LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
    }
    if !initial_loaded_item_mods_can_apply_like_cpp(host, item_guid) {
        return LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
    }
    let slots = item
        .data()
        .enchantments
        .iter()
        .enumerate()
        .filter_map(|(slot_index, enchantment)| {
            if enchantment.id == 0 {
                return None;
            }
            <EnchantmentSlot as num_traits::FromPrimitive>::from_usize(slot_index)
        })
        .collect::<Vec<_>>();

    let mut outcome = LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
    for slot in slots {
        if let Some(plan) = host.apply_current_player_item_enchantment_plan_like_cpp(
            item_guid,
            slot,
            ApplyEnchantmentArgs::apply(),
        ) {
            if let ApplyEnchantmentResult::Applied {
                enchantment_id,
                apply,
                effects_allowed,
                update_permanent_visible_item,
                duration_action,
                ..
            } = plan.result
            {
                if let Some(ApplyEnchantmentDurationAction::Added(duration_update)) =
                    duration_action
                {
                    outcome.duration_updates.push(duration_update);
                }
                if effects_allowed {
                    let (changed_stats, mut effect_actions, mut unrepresented_effect_actions) =
                        apply_loaded_equipped_item_enchantment_effects_like_cpp(
                            host,
                            item_guid,
                            slot,
                            enchantment_id,
                            apply,
                        );
                    outcome.send_stat_update |= changed_stats;
                    outcome.effect_actions.append(&mut effect_actions);
                    outcome
                        .unrepresented_effect_actions
                        .append(&mut unrepresented_effect_actions);
                }
                if update_permanent_visible_item
                    && let Some(visible_item_update) =
                        loaded_inventory_item_visible_update_like_cpp(host, item_guid)
                {
                    outcome.visible_item_changes.push(visible_item_update);
                }
            }
            outcome.plans.push(plan);
        }
    }
    outcome
}
fn apply_loaded_equipped_item_enchantment_effects_like_cpp<H: LoadedItemModsHostLikeCpp>(
    host: &mut H,
    item_guid: ObjectGuid,
    slot: EnchantmentSlot,
    enchantment_id: i32,
    apply: bool,
) -> (
    bool,
    Vec<RepresentedItemBonusActionLikeCpp>,
    Vec<RepresentedItemBonusActionLikeCpp>,
) {
    let Some(item) = host.resolved_inventory_item_object_like_cpp(item_guid) else {
        return (false, Vec::new(), Vec::new());
    };
    let Some(effects) = u32::try_from(enchantment_id)
        .ok()
        .and_then(|enchantment_id| {
            host.hub_ref()
                .catalogs
                .apply_enchantment_effect_refs(enchantment_id)
        })
    else {
        return (false, Vec::new(), Vec::new());
    };
    let item_template = host
        .hub_ref()
        .catalogs
        .item_storage_template(item.object().entry());
    let random_suffix = host
        .hub_ref()
        .catalogs
        .apply_enchantment_random_suffix_ref(item.data().random_properties_id);
    let actions = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            player.apply_enchantment_effect_actions_for_enchantment(
                &item,
                item_template.as_ref(),
                slot,
                enchantment_id,
                random_suffix,
                apply,
                &effects,
            )
        })
        .unwrap_or_default();

    let mut changed_stats = false;
    let mut represented_actions = Vec::new();
    let mut unrepresented_actions = Vec::new();
    for action in actions {
        if matches!(action, ApplyEnchantmentEffectAction::Noop) {
            continue;
        }
        changed_stats |= wow_entities::represented_item_bonus_action_updates_stats_like_cpp(action);
        let represented_action = RepresentedItemBonusActionLikeCpp {
            item_guid,
            slot: slot as u8,
            action,
        };
        #[cfg(any(test, feature = "test-fixtures"))]
        if TrainerSessionHostLikeCpp::world_test_consumer_like_cpp(host) {
            host.split_inventory_mut()
                .0
                .record_represented_item_bonus_actions_for_test_like_cpp(std::slice::from_ref(
                    &represented_action,
                ));
        }
        let spell_action_applied = apply_loaded_enchantment_spell_action_like_cpp(host, action);
        if !spell_action_applied
            && wow_entities::loaded_enchantment_effect_action_is_unrepresented_like_cpp(action)
        {
            unrepresented_actions.push(represented_action.clone());
        }
        represented_actions.push(represented_action);
        apply_represented_item_bonus_action_state_like_cpp(host, action);
    }
    (changed_stats, represented_actions, unrepresented_actions)
}
fn apply_loaded_enchantment_spell_action_like_cpp<H: LoadedItemModsHostLikeCpp>(
    host: &mut H,
    action: ApplyEnchantmentEffectAction,
) -> bool {
    match action {
        ApplyEnchantmentEffectAction::CastEquipSpell {
            spell_id,
            item_guid,
        } => {
            let Ok(spell_id) = i32::try_from(spell_id) else {
                return false;
            };
            let Some(spell_info) = host
                .hub_ref()
                .catalogs
                .spell_store()
                .and_then(|store| store.get(spell_id))
                .cloned()
            else {
                return false;
            };
            let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
            if effect_mask == 0 {
                return false;
            }
            session_aura_application_cx_like_cpp(host)
                .apply_aura_with_effect_mask_provenance_and_update_like_cpp(
                    spell_id,
                    item_guid,
                    0,
                    AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                    effect_mask,
                    wow_entities::AuraCastProvenanceLikeCpp::default(),
                    false,
                )
                .is_ok()
        }
        ApplyEnchantmentEffectAction::RemoveEquipSpellAura {
            spell_id,
            item_guid,
        } => {
            let Ok(spell_id) = i32::try_from(spell_id) else {
                return false;
            };
            let Some(visible_auras) = host.hub_ref().resolved_player_visible_auras_like_cpp()
            else {
                return false;
            };
            let slots = visible_auras
                .values()
                .filter_map(|aura| {
                    (aura.spell_id == spell_id && aura.caster_guid == item_guid)
                        .then_some(aura.slot)
                })
                .collect::<Vec<_>>();
            let removed = !slots.is_empty();
            for slot in slots {
                let _ = session_aura_application_cx_like_cpp(host).remove_aura_like_cpp(slot);
            }
            removed
        }
        _ => false,
    }
}
fn initial_loaded_item_mods_can_apply_like_cpp<H: LoadedItemModsHostLikeCpp>(
    host: &H,
    item_guid: ObjectGuid,
) -> bool {
    let (state, hub) = host.split_inventory_ref();
    state.initial_loaded_item_mods_can_apply_like_cpp(hub, item_guid)
}
fn loaded_inventory_item_visible_update_like_cpp<H: LoadedItemModsHostLikeCpp>(
    host: &H,
    item_guid: ObjectGuid,
) -> Option<(u8, i32, u16, u16)> {
    let (state, hub) = host.split_inventory_ref();
    state.loaded_inventory_item_visible_update_like_cpp(hub, item_guid)
}
pub fn apply_represented_item_bonus_action_state_like_cpp<H: LoadedItemModsHostLikeCpp>(
    host: &mut H,
    action: ApplyEnchantmentEffectAction,
) -> bool {
    let (state, mut hub) = host.split_inventory_mut();
    state.apply_represented_item_bonus_action_state_like_cpp(&mut hub, action)
}

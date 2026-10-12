// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Single-caller helpers of the moved gameobject use (#1363 slice 4a): the
//! battleground-object gates, the item forge, the fishing skill base and the
//! stealth/invisibility removal, each moved from its World session module.

use wow_constants::unit::UnitFlags;
use wow_core::ObjectGuid;
use wow_entities::RepresentedAuraEffectLikeCpp;

use crate::gameobject_use_host::GameObjectUseHostLikeCpp;
use crate::{RepresentedBattlegroundObjectUseRejection, RepresentedGameObjectUseEffect};

pub fn represented_player_can_use_battleground_object_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> bool {
    let gameobject_faction = host
        .world_entities()
        .represented_gameobject_use_state_like_cpp(gameobject_guid)
        .and_then(|state| state.faction_template);
    if let (Some(player_faction), Some(gameobject_faction), Some(store)) = (
        host.hub_ref().player_faction_template_id_like_cpp(),
        gameobject_faction,
        host.hub_ref().catalogs.factions.template_store.as_ref(),
    ) && let (Some(player_entry), Some(gameobject_entry)) =
        (store.get(player_faction), store.get(gameobject_faction))
        && !player_entry.is_friendly_to_like_cpp(gameobject_entry)
    {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::UnfriendlyFaction,
                },
            );
        return false;
    }

    let Some((player_unit_flags, _, _)) =
        host.hub_ref().player_unit_presentation_snapshot_like_cpp()
    else {
        return false;
    };
    if player_unit_flags.contains(UnitFlags::IMMUNE) {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::DamageImmune,
                },
            );
        return false;
    }

    let Some(has_recently_dropped_flag_debuff) =
        host.hub_ref().has_recently_dropped_flag_debuff_like_cpp()
    else {
        return false;
    };
    if has_recently_dropped_flag_debuff {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::RecentlyDroppedFlag,
                },
            );
        return false;
    }

    let Some(player_is_alive) = host.hub_ref().resolved_player_is_alive_like_cpp() else {
        return false;
    };
    if !player_is_alive {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::Dead,
                },
            );
        return false;
    }

    true
}

pub fn represented_player_battleground_type_id_or_reject_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> Option<u32> {
    let state = host
        .hub_ref()
        .player_battleground_state_snapshot_like_cpp()?;
    if let Some(bg_type_id) = state.battleground_type_id_like_cpp() {
        return Some(bg_type_id);
    }

    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::NotInBattleground,
            },
        );
    None
}

pub fn represented_player_reject_battleground_object_vehicle_like_cpp<
    H: GameObjectUseHostLikeCpp,
>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> bool {
    if host
        .hub_ref()
        .player_vehicle_seat_state_like_cpp()
        .and_then(|(flags, _)| flags)
        .is_none()
    {
        return false;
    }

    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::Vehicle,
            },
        );
    true
}

pub fn use_represented_gameobject_item_forge_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    source: wow_entities::ItemForgeUseSource,
) -> bool {
    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::ItemForgeUsed {
                gameobject_guid,
                player_guid,
                condition_id: source.condition_id,
                forge_type: source.forge_type,
            },
        );

    true
}

pub fn represented_fishing_base_skill_level_like_cpp<H: GameObjectUseHostLikeCpp>(
    host: &H,
    gameobject_guid: ObjectGuid,
) -> Option<i32> {
    let area_id = host
        .world_entities()
        .represented_gameobject_use_state_like_cpp(gameobject_guid)
        .and_then(|state| state.area_id)?;
    let area_store = host.hub_ref().catalogs.area_table_store()?;
    let fishing_store = host.hub_ref().catalogs.fishing_base_skill_store()?;
    Some(fishing_store.base_skill_level_like_cpp(area_store, area_id))
}

pub fn remove_represented_stealth_or_invisibility_auras_by_type_like_cpp<
    H: GameObjectUseHostLikeCpp,
>(
    host: &mut H,
) -> Option<usize> {
    let visible_auras = host.hub_ref().resolved_player_visible_auras_like_cpp()?;
    let slots: Vec<u8> = visible_auras
        .iter()
        .filter_map(|(slot, aura)| {
            matches!(
                aura.represented_effect,
                Some(RepresentedAuraEffectLikeCpp::Stealth)
                    | Some(RepresentedAuraEffectLikeCpp::Invisibility)
            )
            .then_some(*slot)
        })
        .collect();
    let removed = slots.len();
    for slot in slots {
        let _ = host.remove_aura(slot);
    }
    Some(removed)
}

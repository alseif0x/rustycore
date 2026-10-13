// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot permission, recipient eligibility and master-loot authority (#1363
//! slice 5a; moved from the World `handlers/loot/authority.rs`).

use super::*;

pub fn represented_loot_authority_pools_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    loot: CreatureLoot,
    personal: bool,
) -> Option<(Option<CreatureLoot>, HashMap<ObjectGuid, CreatureLoot>)> {
    // The admitted application owner holds this transition; the World
    // shell only builds the borrowed loot-release context (#1263 F6).
    host.loot_release_cx_like_cpp()
        .represented_loot_authority_pools_like_cpp(owner_guid, player_guid, loot, personal)
}

pub fn mark_represented_master_loot_item_removed_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    owner_guid: ObjectGuid,
    loot_obj: ObjectGuid,
    loot_list_id: u8,
    target: ObjectGuid,
) {
    {
        let Some(loot) = host
            .loot_mut()
            .cached_loot_for_owner_mut_like_cpp(owner_guid)
        else {
            return;
        };

        let Some(entry) = loot.items.get_mut(loot_list_id as usize) else {
            return;
        };

        let was_unlooted = !entry.is_looted_for_player_like_cpp(target);
        if !was_unlooted {
            return;
        }

        entry.quantity = 0;
        entry.mark_looted_for_player_like_cpp(target);
        loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
    }

    refresh_represented_loot_owner_canonical_summary_like_cpp(host, owner_guid, target);
    host.send_packet(&LootRemoved {
        owner: owner_guid,
        loot_obj,
        loot_list_id,
    });
}

pub fn represented_master_loot_can_store_error_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    target: ObjectGuid,
    item_id: u32,
    count: u32,
) -> Option<u8> {
    if host.player_guid() != Some(target) {
        return None;
    }

    let Some((result, _, _)) = host.plan_store_new_direct_inventory_item(item_id, count) else {
        return Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP);
    };

    master_loot_error_for_inventory_result_like_cpp(result)
}

pub fn represented_master_loot_candidate_list_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> Option<MasterLootCandidateList> {
    let is_master_looter = if let (Some(group_guid), Some(registry)) =
        (host.resolved_group_guid_like_cpp(), host.group_registry())
    {
        registry.get(&group_guid).is_some_and(|group| {
            group.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
                && group.master_looter_guid == player_guid
        })
    } else {
        false
    };

    let loot = host.loot().cached_loot_for_owner_like_cpp(owner_guid)?;
    if loot.loot_method != LOOT_METHOD_MASTER_LIKE_CPP || !is_master_looter {
        return None;
    }

    Some(MasterLootCandidateList {
        loot_obj: loot.loot_guid,
        players: loot.allowed_looters.clone(),
    })
}

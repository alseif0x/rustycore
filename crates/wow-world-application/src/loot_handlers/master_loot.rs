// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handler family: `CMSG_MASTER_LOOT_ITEM` (C++
//! `WorldSession::HandleLootMasterGiveOpcode`), with its private member gate.

use super::*;

/// CMSG_MASTER_LOOT_ITEM — master looter assigns loot to a target.
///
/// C++ first rejects players that are not in a group or are not the group's
/// master looter with `LOOT_ERROR_DIDNT_KILL`. Current Rust group state has
/// loot method `MASTER_LOOT` and the stored master-looter GUID matching the
/// current player.
pub(super) async fn handle_master_loot_item_with_generator_like_cpp<H, C>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    master_loot_item: MasterLootItem,
) where
    H: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    let Some(player_guid) = host.loot_unit_hub_ref_like_cpp().core.player_guid() else {
        return;
    };

    let is_represented_master_looter = if let (Some(group_guid), Some(registry)) = (
        host.master_loot_resolved_group_guid_like_cpp(),
        host.loot_unit_hub_ref_like_cpp().core.group_registry(),
    ) {
        registry.get(&group_guid).is_some_and(|group| {
            group.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
                && group.master_looter_guid == player_guid
        })
    } else {
        false
    };

    if !is_represented_master_looter {
        host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
            host.loot_unit_hub_ref_like_cpp(),
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            LOOT_ERROR_DIDNT_KILL_LIKE_CPP,
        );
        return;
    }

    if !host
        .loot_unit_loot_ref_like_cpp()
        .represented_master_loot_target_exists_like_cpp(
            host.loot_unit_hub_ref_like_cpp(),
            master_loot_item.target,
        )
    {
        host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
            host.loot_unit_hub_ref_like_cpp(),
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP,
        );
        return;
    }

    let mut current_session_assignments = 0_u32;

    for req in &master_loot_item.loot {
        let Some(owner_guid) = host
            .loot_unit_loot_ref_like_cpp()
            .active_loot_owner_for_loot_object_like_cpp(req.object)
        else {
            return;
        };

        if !represented_master_loot_target_eligible_like_cpp(host, master_loot_item.target) {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            );
            return;
        }

        let owned_authority =
            host.master_loot_prepare_owned_authority_outcome_like_cpp(owner_guid, player_guid);
        let authority = match &owned_authority {
            OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority)
                if authority
                    .snapshot_for_player_like_cpp(master_loot_item.target)
                    .is_some() =>
            {
                Some(authority.clone())
            }
            _ => None,
        };
        // Only a readable "absent" answer may still fall back to the local
        // fixture; an unreadable designated owner always refuses.
        if authority.is_none()
            && (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
            && (!matches!(
                owned_authority,
                OwnedLootAuthorityLookupOutcomeLikeCpp::Absent
            ) || !host.master_loot_local_fixture_allowed_like_cpp())
        {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            );
            return;
        }
        if let Some(authority) = authority.as_ref() {
            if !host
                .loot_unit_loot_ref_like_cpp()
                .represented_active_loot_generation_matches_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    owner_guid,
                    authority,
                )
            {
                host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }
            let _ =
                host.master_loot_reconcile_loot_cache_like_cpp(owner_guid, master_loot_item.target);
        }

        let Some(loot) = host
            .loot_unit_loot_ref_like_cpp()
            .cached_loot_for_owner_like_cpp(owner_guid)
        else {
            return;
        };
        let dungeon_encounter_id = loot.dungeon_encounter_id;

        if loot.loot_method != LOOT_METHOD_MASTER_LIKE_CPP {
            return;
        }

        if !loot.allowed_looters.contains(&master_loot_item.target) {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            );
            return;
        }

        if req.loot_list_id as usize >= loot.items.len() {
            return;
        }

        let item = &loot.items[req.loot_list_id as usize];
        if !item.allowed_looters.is_empty()
            && !item.allowed_looters.contains(&master_loot_item.target)
        {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            );
            return;
        }

        if let Some(error) = host.master_loot_can_store_error_like_cpp(
            master_loot_item.target,
            item.item_id,
            item.quantity,
        ) {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                error,
            );
            return;
        }

        let mut entry = item.clone();
        let claim = if let Some(authority) = authority {
            let Some(expected_generation) = host
                .loot_unit_loot_ref_like_cpp()
                .active_loot_view_generation_like_cpp(owner_guid)
                .copied()
            else {
                host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            };
            let claim = match authority
                .reserve_item_for_award_generation_like_cpp(
                    master_loot_item.target,
                    req.loot_list_id,
                    expected_generation,
                )
                .await
            {
                Ok(claim) => claim,
                Err(_) => {
                    host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                        host.loot_unit_hub_ref_like_cpp(),
                        req.object,
                        owner_guid,
                        LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                    );
                    return;
                }
            };
            if !host
                .loot_unit_loot_ref_like_cpp()
                .represented_active_loot_claim_generation_matches_like_cpp(owner_guid, &claim)
            {
                claim.rollback_like_cpp();
                host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }
            if let LootClaimPayload::Item(reserved_entry) = claim.payload_like_cpp() {
                entry = reserved_entry.clone();
            }
            Some(claim)
        } else {
            None
        };
        if master_loot_item.target == player_guid {
            let stored = if let Some(claim) = claim.as_ref() {
                host.master_loot_store_claimed_direct_item_like_cpp(
                    item_guid_generator,
                    &entry,
                    dungeon_encounter_id,
                    owner_guid,
                    req.object,
                    claim,
                )
                .await
            } else {
                host.master_loot_store_direct_item_like_cpp(
                    item_guid_generator,
                    &entry,
                    dungeon_encounter_id,
                    owner_guid,
                )
                .await
            };
            if !stored {
                return;
            }
            if claim.is_none() {
                host.master_loot_mark_item_removed_like_cpp(
                    owner_guid,
                    req.object,
                    req.loot_list_id,
                    master_loot_item.target,
                );
            }
            current_session_assignments = current_session_assignments.saturating_add(1);
        } else {
            let authoritative_claim = claim.is_some();
            match host
                .loot_unit_loot_ref_like_cpp()
                .request_represented_remote_master_loot_give_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    master_loot_item.target,
                    owner_guid,
                    req.object,
                    req.loot_list_id,
                    dungeon_encounter_id,
                    entry,
                    claim,
                )
                .await
            {
                MasterLootGiveResult::Stored if !authoritative_claim => {
                    host.master_loot_mark_item_removed_like_cpp(
                        owner_guid,
                        req.object,
                        req.loot_list_id,
                        master_loot_item.target,
                    );
                }
                MasterLootGiveResult::Stored => {}
                MasterLootGiveResult::StoreFailed(error) => {
                    host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                        host.loot_unit_hub_ref_like_cpp(),
                        req.object,
                        owner_guid,
                        error,
                    );
                    return;
                }
                MasterLootGiveResult::TargetMismatch => {
                    host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                        host.loot_unit_hub_ref_like_cpp(),
                        ObjectGuid::EMPTY,
                        ObjectGuid::EMPTY,
                        LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP,
                    );
                    return;
                }
            }
        }
    }

    debug!(
        account = host.loot_unit_hub_ref_like_cpp().core.account_id,
        target = ?master_loot_item.target,
        request_count = master_loot_item.loot.len(),
        current_session_assignments,
        "CMSG_MASTER_LOOT_ITEM accepted; represented self and connected remote target assignments route through target session state"
    );
}

/// C++ `Group::IsMember`-style target gate of `HandleLootMasterGiveOpcode`;
/// moved with the handler from the World loot tree, whose only caller it was.
fn represented_master_loot_target_eligible_like_cpp<H, C>(host: &H, target: ObjectGuid) -> bool
where
    H: LootHandlerHostLikeCpp<C>,
{
    let Some(group_guid) = host.master_loot_resolved_group_guid_like_cpp() else {
        return false;
    };

    let Some(group_registry) = host.loot_unit_hub_ref_like_cpp().core.group_registry() else {
        return false;
    };

    group_registry
        .get(&group_guid)
        .is_some_and(|group| group.members.contains(&target))
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void storage: `CMSG_UNLOCK_VOID_STORAGE`, `CMSG_QUERY_VOID_STORAGE`, the unlock commit (moved from the World `session/persistence/commit.rs`) and the loaded-state read (#1363 slice 4c).

use super::*;

pub async fn handle_void_storage_unlock_with_generator_like_cpp<
    H: VoidStorageSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    mut pkt: WorldPacket,
) {
    let Ok(unlock) = UnlockVoidStorage::read(&mut pkt) else {
        return;
    };
    if host
        .hub_ref()
        .represented_npc_can_interact_with_like_cpp(unlock.npc, NPCFlags1::VAULT_KEEPER.bits(), 0)
        .is_none()
        || host.void_storage_is_unlocked_like_cpp()
    {
        return;
    }

    let Some(player_guid) = host.player_guid() else {
        return;
    };
    let Some(port) =
        BattlePetSessionHostLikeCpp::lifecycle(host).void_storage_persistence_port_like_cpp()
    else {
        return;
    };
    let Some(money_persistence) = host
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
    else {
        return;
    };

    // C++ ModifyMoney clamps at zero; unlocking does not have a separate
    // HasEnoughMoney gate in this audited branch.
    let Some(old_money) = host.resolved_player_money_like_cpp() else {
        return;
    };
    let new_money = old_money.saturating_sub(VOID_STORAGE_UNLOCK_COST_LIKE_CPP);
    let Some(new_flags) = host.represented_player_flags_value_like_cpp() else {
        return;
    };
    let new_flags = new_flags | PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP;
    let request = wow_persistence::VoidStorageUnlockWriteRequestLikeCpp {
        player_guid: player_guid.counter() as u64,
        money_before: old_money,
        money_after: new_money,
        player_flags_after: new_flags,
    };
    let Some(money_persistence) = host
        .await_exclusive_player_money_transaction_outcome_like_cpp(
            money_persistence,
            port.persist_void_storage_unlock_like_cpp(request),
            old_money,
            new_money,
            "void-storage unlock",
        )
        .await
    else {
        return;
    };

    if !host.stage_player_money_change_like_cpp(old_money, new_money) {
        host.core_mut().kick(
            "canonical Player money owner became unavailable after void-storage unlock COMMIT",
        );
        return;
    }
    apply_committed_void_storage_unlock_like_cpp(host);
    VendorSessionHostLikeCpp::sync_player_registry_state_like_cpp(host);
    drop(money_persistence);

    host.drain_represented_quest_objective_progress_with_generator_like_cpp(item_guid_generator)
        .await;
    if old_money != new_money {
        BattlePetPurchaseHostLikeCpp::send_player_values_update_from_entity_bridge(
            host,
            &[],
            &[],
            &[],
            &[],
            Some(new_money),
        );
    }
}

pub async fn handle_void_storage_query<H: VoidStorageSessionHostLikeCpp>(
    host: &mut H,
    mut pkt: WorldPacket,
) {
    let Ok(query) = QueryVoidStorage::read(&mut pkt) else {
        return;
    };
    if host
        .hub_ref()
        .represented_npc_can_interact_with_like_cpp(
            query.npc,
            (NPCFlags1::TRANSMOGRIFIER | NPCFlags1::VAULT_KEEPER).bits(),
            0,
        )
        .is_none()
        || !host.void_storage_is_unlocked_like_cpp()
        || represented_void_storage_loaded_like_cpp(host) != Some(true)
    {
        host.send_packet(&VoidStorageFailed::default());
        return;
    }

    let Some(contents) = represented_void_storage_contents_like_cpp(host) else {
        host.send_packet(&VoidStorageFailed::default());
        return;
    };
    host.send_packet(&contents);
}

/// Apply the already-committed void-storage unlock to runtime state and
/// emit the same PlayerData::Flags values delta that C++ SetPlayerFlag does.
pub fn apply_committed_void_storage_unlock_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &mut H,
) {
    let values_update = host.player_values_update_snapshot().and_then(|mut player| {
        player.set_player_flag_like_cpp(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP);
        Some(player.values_update_like_cpp(true))
    });

    let Some(_current_flags) = host.represented_player_flags_value_like_cpp() else {
        return;
    };
    let _canonical = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP);
        })
        .is_some();
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        host.split_lifecycle_mut()
            .0
            .set_represented_loaded_player_flags_for_test_like_cpp(Some(
                _current_flags | PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP,
            ));
        host.split_lifecycle_mut()
            .0
            .set_represented_loaded_player_flags_applied_for_test_like_cpp(_canonical);
    }

    if let Some(update) = values_update {
        host.core().send_player_values_update_like_cpp(&update);
    }
}

pub fn represented_void_storage_loaded_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
) -> Option<bool> {
    let (inventory, hub) = host.split_inventory_ref();
    inventory.with_owned_void_storage_like_cpp(hub, |_, loaded| loaded)
}

use super::*;
use wow_persistence::WorldLootMoneySource;

enum ChestReceipt<T> {
    Ready { spent: bool },
    Read(T),
    Unavailable,
}

/// Returns the recovered spent flag. A missing result defers this loot window.
pub(super) async fn reattach<H: LootSessionHostLikeCpp>(
    host: &mut H,
    guid: ObjectGuid,
    player: ObjectGuid,
) -> Option<bool> {
    let realm_id = host.hub_ref().core.realm_id;
    let observed = host.mutate_canonical_gameobject_by_guid_like_cpp(guid, |chest| {
        if chest.chest_money_restock_waiting() {
            return ChestReceipt::Unavailable;
        }
        if chest.spawn_id() == 0 {
            return ChestReceipt::Ready { spent: false };
        }
        let owner = chest.loot_authority_like_cpp();
        let Some(pool) = owner.snapshot_for_player_like_cpp(player) else {
            return ChestReceipt::Unavailable;
        };
        if let Some(spent) = owner.recovered_world_money(pool.scope, pool.generation) {
            return ChestReceipt::Ready { spent };
        }
        let source = WorldLootMoneySource {
            realm_id,
            map_id: chest.world().map_id(),
            instance_id: chest.world().instance_id(),
            spawn_id: chest.spawn_id(),
        };
        ChestReceipt::Read((
            source,
            owner.clone(),
            pool.scope,
            pool.generation,
            chest.loot_lifecycle_revision_like_cpp(),
        ))
    });
    let (source, owner, scope, generation, revision) = match observed {
        Some(ChestReceipt::Read(state)) => state,
        Some(ChestReceipt::Ready { spent }) => return Some(spent),
        Some(ChestReceipt::Unavailable) => return None,
        None => {
            return host
                .represented_local_loot_fixture_allowed_like_cpp()
                .then_some(false);
        }
    };
    let Some(port) = host
        .lifecycle()
        .group_loot_money_persistence_port_like_cpp()
        .cloned()
    else {
        return host
            .represented_local_loot_fixture_allowed_like_cpp()
            .then_some(false);
    };
    let personal_owner = match scope {
        OwnedLootScope::Shared => 0,
        OwnedLootScope::Personal(looter) => looter.counter() as u64,
    };
    let binding = match port.bind_world_chest_money(source, personal_owner).await {
        Ok(Some(binding)) => binding,
        Ok(None) => return None,
        Err(error) => {
            tracing::warn!(?source, %error, "Unable to read chest money generation");
            return None;
        }
    };
    let installed = host.mutate_canonical_gameobject_by_guid_like_cpp(guid, |chest| {
        if chest.loot_lifecycle_revision_like_cpp() != revision
            || !owner.shares_storage_like_cpp(chest.loot_authority_like_cpp())
            || !chest.bind_chest_money_generation(realm_id, binding.generation, |authority| {
                authority.bind_world_money(scope, generation, binding.claim_id, binding.spent)
            })
        {
            return false;
        }
        chest.sync_loot_summaries_from_authority_like_cpp();
        true
    });
    (installed == Some(true) && reconcile_represented_loot_cache_like_cpp(host, guid, player))
        .then_some(binding.spent)
}

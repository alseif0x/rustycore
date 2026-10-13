// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject loot-authority install observation and personal-pool upsert
//! (#1363 slice 5a; moved from the World `handlers/loot/sources/gameobject_authority.rs`).

use super::*;

pub fn upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    loot: CreatureLoot,
    replace: bool,
    observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
) -> Option<()> {
    upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
        host,
        gameobject_guid,
        player_guid,
        loot,
        replace,
        false,
        observation,
    )
}

pub fn upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    loot: CreatureLoot,
    replace: bool,
    discard_empty_pool: bool,
    observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
) -> Option<()> {
    let (_, mut personal) =
        represented_loot_authority_pools_like_cpp(host, gameobject_guid, player_guid, loot, true)?;
    let pool = personal.remove(&player_guid)?;
    if discard_empty_pool && loot_is_looted_like_cpp(&pool) {
        host.loot_mut()
            .discard_represented_personal_loot_cache_for_player_like_cpp(
                gameobject_guid,
                player_guid,
            );
        return None;
    }
    let installed =
        host.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, move |gameobject| {
            gameobject.install_personal_loot_if_lifecycle_like_cpp(
                &observation.authority,
                observation.object_generation,
                observation.loot_lifecycle_revision,
                player_guid,
                pool,
                replace,
            )
        });
    if installed != Some(true) {
        host.loot_mut()
            .discard_represented_personal_loot_cache_for_player_like_cpp(
                gameobject_guid,
                player_guid,
            );
        return None;
    }
    if !reconcile_represented_loot_cache_like_cpp(host, gameobject_guid, player_guid) {
        return None;
    }
    Some(())
}

pub(super) fn represented_gameobject_exists_for_loot_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    guid: ObjectGuid,
) -> bool {
    if !guid.is_game_object() {
        return false;
    }

    let canonical_position = host.loot().canonical_map_object_position_for_loot_like_cpp(
        host.hub_ref(),
        guid,
        &[
            AccessorObjectKind::GameObject,
            AccessorObjectKind::Transport,
        ],
    );
    let represented_state = host
        .world_entities()
        .represented_gameobject_use_state_like_cpp(guid);
    canonical_position.is_some()
        || represented_state.and_then(|state| state.position).is_some()
        || host
            .hub_ref()
            .core
            .client_visible_guids_like_cpp
            .contains(&guid)
}

pub fn represented_gameobject_loot_install_observation_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
) -> Option<RepresentedGameObjectLootInstallObservationLikeCpp> {
    represented_gameobject_loot_install_observation_result_like_cpp(host, gameobject_guid)?
}

/// Preserves the distinction between a missing canonical owner (`None`)
/// and an owner whose current lifecycle rejects generation (`Some(None)`).
/// Test-only packet fixtures may fall back only for the former.
pub(super) fn represented_gameobject_loot_install_observation_result_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
) -> Option<Option<RepresentedGameObjectLootInstallObservationLikeCpp>> {
    let (world_entities, mut hub) = host.split_world_entities_mut();
    world_entities.mutate_canonical_gameobject_by_guid_like_cpp(
        &mut hub,
        gameobject_guid,
        |gameobject| {
            (gameobject.loot_state() != LootState::JustDeactivated).then(|| {
                let authority = gameobject.loot_authority_like_cpp().clone();
                RepresentedGameObjectLootInstallObservationLikeCpp {
                    object_generation: authority.generation_like_cpp(),
                    authority,
                    loot_lifecycle_revision: gameobject.loot_lifecycle_revision_like_cpp(),
                }
            })
        },
    )
}

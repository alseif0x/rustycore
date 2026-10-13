// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Owned loot claims and their leases (#1363 slice 5a; moved from the World
//! `handlers/loot/claims.rs`).

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use super::*;

pub fn creature_loot_release_values_for_viewer_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    creature_guid: ObjectGuid,
    viewer_guid: ObjectGuid,
    viewer_has_pending_bind: bool,
    authority: Option<&OwnedLootAuthority>,
    update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
) -> wow_packet::packets::update::UnitDataValuesDeltaUpdate {
    let (state, hub) = host.split_loot_ref();
    state.creature_loot_release_values_for_viewer_like_cpp(
        hub,
        creature_guid,
        viewer_guid,
        viewer_has_pending_bind,
        authority,
        update,
    )
}

/// The session's loot-authority lookup. It resolves the owner through its
/// one designated owner and keeps a genuinely missing authority distinct
/// from an unreadable one (F6-7 R2/R4). This is the only shape production
/// uses; the historical `Option` wrapper no longer exists.
pub fn represented_owned_loot_authority_outcome_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    owner_guid: ObjectGuid,
) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
    host.core_mut()
        .loot_release_owner_access_like_cpp()
        .represented_owned_loot_authority_outcome_like_cpp(owner_guid)
}

/// Owned by the App release context
/// (`wow-world-loot-ops/src/loot_release/authority.rs`); the shell only
/// lends the borrowed participants.
pub fn prepare_owned_loot_authority_for_active_request_outcome_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    owner_guid: ObjectGuid,
    scope_player: ObjectGuid,
) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
    host.loot_release_cx_like_cpp()
        .prepare_owned_loot_authority_for_active_request_outcome_like_cpp(owner_guid, scope_player)
}

pub(super) fn sync_represented_gameobject_loot_to_canonical_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> Option<()> {
    host.loot_release_cx_like_cpp()
        .sync_represented_gameobject_loot_to_canonical_like_cpp(gameobject_guid, player_guid)
}

pub fn sync_represented_creature_loot_to_canonical_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> Option<()> {
    host.loot_release_cx_like_cpp()
        .sync_represented_creature_loot_to_canonical_like_cpp(creature_guid, player_guid)
}

pub fn refresh_owned_loot_summary_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    owner_guid: ObjectGuid,
) {
    host.core_mut()
        .loot_release_owner_access_like_cpp()
        .refresh_owned_loot_summary_like_cpp(owner_guid);
}

pub async fn store_claimed_direct_loot_item_from_owner_with_generator_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    loot_entry: &LootEntry,
    dungeon_encounter_id: u32,
    owner_guid: ObjectGuid,
    loot_obj: ObjectGuid,
    claim: &LootClaimLease,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    store_direct_loot_item_with_source_and_generator_like_cpp(
        host,
        item_guid_generator,
        loot_entry,
        dungeon_encounter_id,
        owner_guid.is_item().then_some(owner_guid),
        Some(claim),
        Some(LootItemClaimCommitContextLikeCpp {
            owner_guid,
            loot_obj,
            loot_list_id: loot_entry.loot_list_id,
            player_guid,
            free_for_all: loot_entry.flags.freeforall,
        }),
    )
    .await
}

pub fn record_represented_gameobject_chest_release_metadata_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    source: GameObjectLootSource,
) {
    let state = host
        .world_entities_mut()
        .ensure_represented_gameobject_use_state_like_cpp(gameobject_guid);
    state.go_type = Some(GAMEOBJECT_TYPE_CHEST as u8);
    state.chest_restock_time_secs = Some(source.chest_restock_time_secs);
    state.chest_consumable = Some(source.chest_consumable);
    state.despawn_at_action = source.chest_consumable;
    state.chest_loot_source = Some(source);
    state.chest_personal_loot_id = Some(source.personal_loot_id);
    state.linked_trap_entry = (source.linked_trap_entry != 0).then_some(source.linked_trap_entry);
}

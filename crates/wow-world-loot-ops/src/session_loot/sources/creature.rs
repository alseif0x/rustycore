// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature loot generation, persistence observations, and source snapshots (#1363 slice 5a;
//! moved from the World `handlers/loot/sources/creature.rs`).

use super::*;

/// C++ `AELootCreatureCheck::LootDistance` (`LootHandler.cpp:44`).
const AE_LOOT_DISTANCE_LIKE_CPP: f32 = 30.0;

pub async fn represented_ae_loot_creature_targets_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    main_loot_target: ObjectGuid,
    player_guid: ObjectGuid,
) -> Vec<ObjectGuid> {
    let Some(player_position) = host.hub_ref().player_position_like_cpp() else {
        return Vec::new();
    };

    // C++ `AELootCreatureCheck::LootDistance` over the nearby grid cells
    // only, each corpse read in place (no per-creature clone).
    let mut candidates: Vec<ObjectGuid> = host
        .core()
        .nearby_dead_creature_guids_like_cpp(player_position, AE_LOOT_DISTANCE_LIKE_CPP)
        .into_iter()
        .filter(|guid| *guid != main_loot_target && guid.is_creature_or_vehicle())
        .collect();
    candidates.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

    let mut result = Vec::new();
    for owner_guid in candidates {
        let Some(creature) = represented_creature_loot_state_like_cpp(host, owner_guid) else {
            continue;
        };
        if !creature.tappers.is_empty() && !creature.tappers.contains(&player_guid) {
            continue;
        }
        // C++ `CMSG_LOOT_UNIT` only reads the Loot created by
        // `Unit::Kill`; it never regenerates a corpse pool. Reconcile the
        // active object-owned generation and fail closed if kill-time
        // generation is absent or the corpse lifetime was retired.
        if !reconcile_represented_loot_cache_like_cpp(host, owner_guid, player_guid) {
            host.loot_mut()
                .remove_cached_loot_for_owner_like_cpp(owner_guid);
            continue;
        }

        if host
            .loot()
            .cached_loot_for_owner_like_cpp(owner_guid)
            .is_some_and(|loot| {
                host.loot()
                    .represented_loot_can_be_opened_by_player_like_cpp(
                        owner_guid,
                        loot,
                        player_guid,
                    )
            })
        {
            result.push(owner_guid);
        }
    }

    result
}

pub async fn ensure_represented_creature_kill_loot_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
) {
    let Some(creature) = represented_creature_loot_state_like_cpp(host, creature_guid) else {
        return;
    };
    let Some(loot_owner_guid) = creature.tappers.first().copied() else {
        return;
    };
    let loot_scope_player_guid = if host.current_map_dungeon_state_like_cpp() == Some(false) {
        let connected_tappers = {
            let (s, h) = host.split_loot_ref();
            s.represented_connected_creature_tappers_like_cpp(h, &creature.tappers)
        };
        host.player_guid()
            .filter(|player_guid| connected_tappers.contains(player_guid))
            .or_else(|| connected_tappers.first().copied())
            .unwrap_or(loot_owner_guid)
    } else {
        loot_owner_guid
    };

    ensure_represented_creature_loot_like_cpp(
        host,
        creature_guid,
        loot_owner_guid,
        creature.level,
        creature.entry,
        creature.loot_id,
        creature.gold_min,
        creature.gold_max,
        creature.dungeon_encounter_id,
        &creature.tappers,
        creature.loot_lifecycle_revision,
    )
    .await;
    if sync_represented_creature_loot_to_canonical_like_cpp(
        host,
        creature_guid,
        loot_scope_player_guid,
    )
    .is_none()
    {
        host.loot_mut()
            .remove_cached_loot_for_owner_like_cpp(creature_guid);
    }
}

pub fn install_represented_creature_kill_loot_if_current_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
    expected_authority: &OwnedLootAuthority,
    expected_object_generation: u64,
    expected_loot_lifecycle_revision: u64,
    shared: Option<CreatureLoot>,
    personal: HashMap<ObjectGuid, CreatureLoot>,
) -> bool {
    let (state, mut hub) = host.split_loot_mut();
    state.install_represented_creature_kill_loot_if_current_like_cpp(
        &mut hub,
        creature_guid,
        expected_authority,
        expected_object_generation,
        expected_loot_lifecycle_revision,
        shared,
        personal,
    )
}

async fn ensure_represented_creature_loot_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
    loot_owner_guid: ObjectGuid,
    level: u8,
    entry: u32,
    loot_id: u32,
    gold_min: u32,
    gold_max: u32,
    dungeon_encounter_id: u32,
    allowed_looters: &[ObjectGuid],
    expected_loot_lifecycle_revision: u64,
) {
    // An unreadable designated owner abandons the attempt without
    // disposing the cache; genuine absence keeps the historical fixture
    // rule below.
    let authority = match represented_owned_loot_authority_outcome_like_cpp(host, creature_guid) {
        OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority) => Some(authority),
        OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable => return,
        OwnedLootAuthorityLookupOutcomeLikeCpp::Absent => None,
    };
    if authority.is_none() && !host.represented_local_loot_fixture_allowed_like_cpp() {
        host.loot_mut()
            .remove_cached_loot_for_owner_like_cpp(creature_guid);
        return;
    }
    let mut retired_object_generation = None;
    if let Some(authority) = authority.as_ref() {
        #[cfg(any(test, feature = "test-fixtures"))]
        if host.test_fixtures_like_cpp().is_some()
            && authority.is_pristine_like_cpp()
            && host
                .loot()
                .cached_loot_contains_owner_like_cpp(creature_guid)
        {
            if !host.loot().is_personal_loot_owner_like_cpp(creature_guid)
                && let Some(loot) = host
                    .loot_mut()
                    .cached_loot_for_owner_mut_like_cpp(creature_guid)
            {
                prepare_represented_shared_creature_loot_generation_like_cpp(loot, allowed_looters);
            }
            if sync_represented_creature_loot_to_canonical_like_cpp(
                host,
                creature_guid,
                loot_owner_guid,
            )
            .is_some()
            {
                // Legacy packet fixtures pre-populate the former session
                // cache. Install that value once into the typed object-owned
                // authority instead of silently replacing it with generated
                // empty loot. This branch does not exist in production.
                return;
            }
        }
        let snapshot = host
            .player_guid()
            .and_then(|player_guid| authority.snapshot_for_player_like_cpp(player_guid))
            .or_else(|| authority.snapshot_for_player_like_cpp(loot_owner_guid));
        if let Some(snapshot) = snapshot {
            let cache_player = match snapshot.scope {
                OwnedLootScope::Personal(player_guid) => player_guid,
                OwnedLootScope::Shared => loot_owner_guid,
            };
            host.loot_mut()
                .cache_represented_owned_loot_snapshot_like_cpp(
                    creature_guid,
                    cache_player,
                    snapshot,
                );
            return;
        }
        if !authority.is_retired_like_cpp() {
            host.loot_mut()
                .remove_cached_loot_for_owner_like_cpp(creature_guid);
            return;
        }
        retired_object_generation = Some(authority.generation_like_cpp());
        host.loot_mut()
            .remove_cached_loot_for_owner_like_cpp(creature_guid);
        host.loot_mut()
            .remove_cached_loot_generation_like_cpp(creature_guid);
    }

    let map_is_dungeon = host.current_map_dungeon_state_like_cpp();
    let connected_tappers = {
        let (s, h) = host.split_loot_ref();
        s.represented_connected_creature_tappers_like_cpp(h, allowed_looters)
    };

    // C++ `Unit::Kill` has three distinct ownership shapes:
    // - overworld: one independently generated personal pool per tapper;
    // - dungeon encounter/boss: one independent, lockout-filtered pool per
    //   tapper (`GenerateDungeonEncounterPersonalLoot`);
    // - dungeon trash: exactly one personal pool, keyed by the group's
    //   selected looter (or the first tapper without a group).
    if map_is_dungeon == Some(false) || (map_is_dungeon == Some(true) && dungeon_encounter_id != 0)
    {
        let personal_tappers = connected_tappers
            .into_iter()
            .filter(|tapper| {
                dungeon_encounter_id == 0
                    || host.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
                        *tapper,
                        dungeon_encounter_id,
                    )
            })
            .collect::<Vec<_>>();
        if personal_tappers.is_empty() {
            host.loot_mut()
                .remove_cached_loot_for_owner_like_cpp(creature_guid);
            return;
        }

        let Some(personal) = generate_represented_creature_personal_loot_like_cpp(
            host,
            creature_guid,
            level,
            entry,
            loot_id,
            gold_min,
            gold_max,
            dungeon_encounter_id,
            &personal_tappers,
        )
        .await
        else {
            return;
        };
        let cache_player = host
            .player_guid()
            .filter(|player_guid| personal.contains_key(player_guid))
            .unwrap_or(personal_tappers[0]);

        if let (Some(authority), Some(expected_generation)) =
            (authority.as_ref(), retired_object_generation)
        {
            if install_represented_creature_kill_loot_if_current_like_cpp(
                host,
                creature_guid,
                authority,
                expected_generation,
                expected_loot_lifecycle_revision,
                None,
                personal,
            ) {
                let _ =
                    reconcile_represented_loot_cache_like_cpp(host, creature_guid, cache_player);
            } else {
                host.loot_mut()
                    .remove_cached_loot_for_owner_like_cpp(creature_guid);
            }
        } else if host.represented_local_loot_fixture_allowed_like_cpp()
            && let Some(pool) = personal.get(&cache_player).cloned()
        {
            host.loot_mut()
                .insert_cached_loot_for_owner_like_cpp(creature_guid, pool);
        }
        return;
    }

    if map_is_dungeon == Some(true) {
        if connected_tappers.is_empty() {
            host.loot_mut()
                .remove_cached_loot_for_owner_like_cpp(creature_guid);
            return;
        }
        let selected_looter = represented_dungeon_trash_looter_like_cpp(host, &connected_tappers);
        let Some(personal) = generate_represented_creature_personal_loot_like_cpp(
            host,
            creature_guid,
            level,
            entry,
            loot_id,
            gold_min,
            gold_max,
            0,
            &[selected_looter],
        )
        .await
        else {
            return;
        };
        let has_loot = personal
            .get(&selected_looter)
            .is_some_and(|loot| !loot_is_looted_like_cpp(loot));

        if let (Some(authority), Some(expected_generation)) =
            (authority.as_ref(), retired_object_generation)
        {
            if install_represented_creature_kill_loot_if_current_like_cpp(
                host,
                creature_guid,
                authority,
                expected_generation,
                expected_loot_lifecycle_revision,
                None,
                personal,
            ) {
                let _ =
                    reconcile_represented_loot_cache_like_cpp(host, creature_guid, selected_looter);
                if has_loot {
                    advance_represented_dungeon_trash_looter_like_cpp(host, &connected_tappers);
                }
            } else {
                host.loot_mut()
                    .remove_cached_loot_for_owner_like_cpp(creature_guid);
            }
        } else if host.represented_local_loot_fixture_allowed_like_cpp()
            && let Some(pool) = personal.get(&selected_looter).cloned()
        {
            host.loot_mut()
                .insert_cached_loot_for_owner_like_cpp(creature_guid, pool);
        }
        return;
    }

    // Missing Map.db2 metadata is not proof of either overworld or
    // dungeon. Preserve the represented shared fallback for legacy test
    // fixtures, but still bind its async install to the exact death token.
    if !host
        .loot()
        .cached_loot_contains_owner_like_cpp(creature_guid)
    {
        let Some(mut loot) = generate_represented_creature_loot_like_cpp(
            host,
            creature_guid,
            loot_owner_guid,
            level,
            entry,
            loot_id,
            gold_min,
            gold_max,
            dungeon_encounter_id,
        )
        .await
        else {
            return;
        };
        prepare_represented_shared_creature_loot_generation_like_cpp(&mut loot, allowed_looters);
        if let (Some(authority), Some(expected_generation)) =
            (authority.as_ref(), retired_object_generation)
        {
            if install_represented_creature_kill_loot_if_current_like_cpp(
                host,
                creature_guid,
                authority,
                expected_generation,
                expected_loot_lifecycle_revision,
                Some(loot),
                HashMap::new(),
            ) {
                let _ =
                    reconcile_represented_loot_cache_like_cpp(host, creature_guid, loot_owner_guid);
            } else {
                host.loot_mut()
                    .remove_cached_loot_for_owner_like_cpp(creature_guid);
            }
        } else if host.represented_local_loot_fixture_allowed_like_cpp() {
            host.loot_mut()
                .insert_cached_loot_for_owner_like_cpp(creature_guid, loot);
        }
    }
}

pub async fn generate_represented_creature_loot_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
    loot_owner_guid: ObjectGuid,
    _level: u8,
    entry: u32,
    loot_id: u32,
    gold_min: u32,
    gold_max: u32,
    dungeon_encounter_id: u32,
) -> Option<CreatureLoot> {
    let (loot_method, loot_master, round_robin_player) =
        represented_creature_loot_group_state_like_cpp(host, loot_owner_guid);
    let coins = host.represented_money_loot_with_rate_like_cpp(
        gold_min,
        gold_max,
        host.loot_drop_rates_like_cpp().money,
    );

    let items = generate_represented_creature_loot_items_like_cpp(host, loot_id)
        .await
        .unwrap_or_else(|| {
            if loot_id != 0 {
                debug!(
                    entry,
                    loot_id, "creature loot template unavailable for represented corpse"
                );
            }
            Vec::new()
        });

    let loot_guid = next_represented_loot_object_guid_like_cpp(host, creature_guid)?;
    Some(CreatureLoot {
        loot_guid,
        coins,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id,
        loot_method,
        loot_master,
        round_robin_player,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items,
        looted_by_player: false,
    })
}

/// Generate one independently rolled C++ personal `Loot` per supplied
/// player. The caller chooses the ownership set for overworld tappers,
/// encounter-eligible dungeon tappers, or the single dungeon-trash
/// selected looter. Every pool is constructed without a Group and remains
/// an object-owned per-view source of truth.
#[allow(clippy::too_many_arguments)]
async fn generate_represented_creature_personal_loot_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
    _level: u8,
    entry: u32,
    loot_id: u32,
    gold_min: u32,
    gold_max: u32,
    dungeon_encounter_id: u32,
    tappers: &[ObjectGuid],
) -> Option<HashMap<ObjectGuid, CreatureLoot>> {
    let mut personal = HashMap::with_capacity(tappers.len());
    for tapper in tappers {
        let coins = host.represented_money_loot_with_rate_like_cpp(
            gold_min,
            gold_max,
            host.loot_drop_rates_like_cpp().money,
        );
        let items = generate_represented_creature_loot_items_for_player_like_cpp(
            host, loot_id, *tapper,
        )
        .await
        .unwrap_or_else(|| {
            if loot_id != 0 {
                debug!(
                    entry,
                    loot_id,
                    tapper = ?tapper,
                    "creature personal loot template unavailable for represented overworld corpse"
                );
            }
            Vec::new()
        });
        let loot_guid = next_represented_loot_object_guid_like_cpp(host, creature_guid)?;
        let mut loot = CreatureLoot {
            loot_guid,
            coins,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items,
            looted_by_player: false,
        };
        mark_loot_allowed_for_player_like_cpp(&mut loot, *tapper);
        rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(&mut loot);
        personal.insert(*tapper, loot);
    }
    Some(personal)
}

fn represented_creature_loot_group_state_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    loot_owner_guid: ObjectGuid,
) -> (u8, ObjectGuid, ObjectGuid) {
    let Some(group_guid) = host.resolved_group_guid_like_cpp() else {
        return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
    };
    let Some(registry) = host.group_registry() else {
        return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
    };
    let Some(group) = registry.get(&group_guid) else {
        return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
    };

    (group.loot_method, group.master_looter_guid, loot_owner_guid)
}

async fn generate_represented_creature_loot_items_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    loot_id: u32,
) -> Option<Vec<LootEntry>> {
    let player_guid = host.player_guid().unwrap_or(ObjectGuid::EMPTY);
    generate_represented_creature_loot_items_for_player_like_cpp(host, loot_id, player_guid).await
}

pub async fn generate_represented_creature_loot_items_for_player_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    loot_id: u32,
    player_guid: ObjectGuid,
) -> Option<Vec<LootEntry>> {
    if loot_id == 0 {
        return Some(Vec::new());
    }

    let mut rng = host.core_mut().represented_runtime_subrng_like_cpp();
    let stores = host.loot_stores()?;
    let store = stores.get(&LootStoreKind::Creature)?;
    let rates = host.loot_drop_rates_like_cpp();
    let condition_ids =
        store.condition_ids_for_fill_like_cpp(loot_id, LootStoreKind::Creature, stores);
    let condition_rows =
        load_represented_creature_loot_condition_rows_like_cpp(host, &condition_ids).await;
    let condition_references =
        load_represented_creature_loot_condition_reference_rows_like_cpp(host, &condition_rows)
            .await;
    let addon_metadata = load_item_template_addon_loot_metadata_for_item_ids_like_cpp(
        host,
        condition_ids.iter().map(|id| id.source_entry),
    )
    .await;
    let generated = {
        store
            .fill_loot_with_context_like_cpp(
                loot_id,
                LootStoreKind::Creature,
                stores,
                LootFillOptions {
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    rates_allowed: true,
                    referenced_amount_rate: rates.item_referenced_amount,
                    item_context: ItemContext::None as u8,
                },
                &mut rng,
                |item_id| {
                    host.item_storage_template(item_id)
                        .map(|template| LootItemTemplateMetadata {
                            max_stack: template.max_stack_size.max(1),
                            has_multi_drop_flag: template.flags.contains(ItemFlags::MULTI_DROP),
                            has_follow_loot_rules_flag: false,
                        })
                },
                |item| host.item_drop_rate_like_cpp(item.item_id),
                |context| {
                    represented_creature_loot_item_allowed_for_player_like_cpp(
                        host,
                        context,
                        player_guid,
                        &condition_rows,
                        &condition_references,
                        &addon_metadata,
                    )
                },
                |item_id, rng| {
                    let random_properties =
                        generate_loot_store_random_properties_with_rng_like_cpp(host, item_id, rng);
                    LootItemRandomProperties {
                        id: random_properties.id,
                        seed: random_properties.seed,
                    }
                },
            )
            .ok()?
    };

    Some(
        generated
            .into_iter()
            .map(|item| {
                let metadata = addon_metadata
                    .get(&item.item_id)
                    .copied()
                    .unwrap_or_default();
                generated_creature_loot_item_to_entry_like_cpp(item, metadata)
            })
            .collect(),
    )
}

pub fn represented_creature_loot_state_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    guid: ObjectGuid,
) -> Option<RepresentedCreatureLootStateLikeCpp> {
    let (state, mut hub) = host.split_loot_mut();
    state.represented_creature_loot_state_like_cpp(&mut hub, guid)
}

pub fn represented_creature_position_for_loot_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    guid: ObjectGuid,
) -> Option<wow_core::Position> {
    let (state, mut hub) = host.split_loot_mut();
    state.represented_creature_position_for_loot_like_cpp(&mut hub, guid)
}

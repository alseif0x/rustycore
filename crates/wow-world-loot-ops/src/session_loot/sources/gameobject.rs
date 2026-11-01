// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject loot-source interaction and runtime state publication (#1363 slice 5a;
//! moved from the World `handlers/loot/sources/gameobject.rs`).

use super::*;

/// Mirrors the small gathering-node state subset that C++ keeps on the
/// shared GameObject before asking this session to recompute its visible
/// GameObject dynamic-flag deltas.
pub fn handle_sync_gathering_node_gameobject_state_and_refresh_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    command: SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn {
        return;
    }
    if command.map_id != host.core().player_map_id_like_cpp() {
        return;
    }
    let current_instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    if command.instance_id != current_instance_id {
        return;
    }
    if u32::from(command.go_type) != GAMEOBJECT_TYPE_GATHERING_NODE {
        return;
    }
    let loot_state = match command.loot_state {
        Some(0) => Some(LootState::NotReady),
        Some(1) => Some(LootState::Ready),
        Some(2) => Some(LootState::Activated),
        Some(3) => Some(LootState::JustDeactivated),
        Some(_) => return,
        None => None,
    };
    let go_state = match command.go_state {
        Some(0) => Some(GoState::Active),
        Some(1) => Some(GoState::Ready),
        Some(2) => Some(GoState::Destroyed),
        Some(24) => Some(GoState::TransportActive),
        Some(25) => Some(GoState::TransportStopped),
        Some(_) => return,
        None => None,
    };

    {
        let state = host
            .world_entities_mut()
            .ensure_represented_gameobject_use_state_like_cpp(command.gameobject_guid);
        state.map_id = Some(command.map_id);
        state.go_type = Some(command.go_type);
        state.loot_state = loot_state;
        state.loot_state_unit_guid = command.loot_state_unit_guid;
        state.go_state = go_state;
        state.dynamic_flags = command.dynamic_flags;
        state.gathering_node_loot_id = command.gathering_node_loot_id;
        state.personal_loot_uses = command.personal_loot_uses;
        state.linked_trap_entry = command.linked_trap_entry;
        state.linked_trap_guid = command.linked_trap_guid;
    }

    let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
}

/// Mirrors the small chest state subset that C++ keeps on the shared
/// GameObject before asking this session to recompute visible GameObject
/// dynamic-flag deltas.
pub fn handle_sync_chest_gameobject_state_and_refresh_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    command: SyncChestGameobjectStateAndRefreshLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn {
        return;
    }
    if command.map_id != host.core().player_map_id_like_cpp() {
        return;
    }
    let current_instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    if command.instance_id != current_instance_id {
        return;
    }
    if u32::from(command.go_type) != GAMEOBJECT_TYPE_CHEST {
        return;
    }
    let loot_state = match command.loot_state {
        Some(0) => Some(LootState::NotReady),
        Some(1) => Some(LootState::Ready),
        Some(2) => Some(LootState::Activated),
        Some(3) => Some(LootState::JustDeactivated),
        Some(_) => return,
        None => None,
    };

    {
        let state = host
            .world_entities_mut()
            .ensure_represented_gameobject_use_state_like_cpp(command.gameobject_guid);
        state.map_id = Some(command.map_id);
        state.go_type = Some(command.go_type);
        state.loot_state = loot_state;
        state.loot_state_unit_guid = command.loot_state_unit_guid;
        state.chest_loot_source = Some(GameObjectLootSource {
            loot_id: command.chest_loot_id,
            use_group_loot_rules: false,
            dungeon_encounter_id: 0,
            personal_loot_id: command.chest_personal_loot_id,
            push_loot_id: command.chest_push_loot_id,
            triggered_event_id: 0,
            linked_trap_entry: command.linked_trap_entry.unwrap_or_default(),
            chest_restock_time_secs: command.chest_restock_time_secs,
            chest_consumable: command.chest_consumable,
            chest_quest_id: command.chest_quest_id,
        });
        state.chest_restock_time_secs = Some(command.chest_restock_time_secs);
        state.chest_consumable = Some(command.chest_consumable);
        state.chest_personal_loot_id = Some(command.chest_personal_loot_id);
        state.linked_trap_entry = command.linked_trap_entry;
        state.linked_trap_guid = command.linked_trap_guid;
    }

    let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
}

/// Mirrors the small shared goober state subset that C++ keeps on the
/// shared GameObject before asking this session to recompute visible
/// GameObject dynamic-flag deltas. This intentionally does not import the
/// cooldown/source ownership fields; the map-owned close/despawn path is a
/// later runtime slice.
pub fn handle_sync_goober_gameobject_state_and_refresh_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    command: SyncGooberGameobjectStateAndRefreshLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn {
        return;
    }
    if command.map_id != host.core().player_map_id_like_cpp() {
        return;
    }
    let current_instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    if command.instance_id != current_instance_id {
        return;
    }
    if u32::from(command.go_type) != GAMEOBJECT_TYPE_GOOBER {
        return;
    }
    let loot_state = match command.loot_state {
        Some(0) => Some(LootState::NotReady),
        Some(1) => Some(LootState::Ready),
        Some(2) => Some(LootState::Activated),
        Some(3) => Some(LootState::JustDeactivated),
        Some(_) => return,
        None => None,
    };
    let go_state = match command.go_state {
        Some(0) => Some(GoState::Active),
        Some(1) => Some(GoState::Ready),
        Some(2) => Some(GoState::Destroyed),
        Some(24) => Some(GoState::TransportActive),
        Some(25) => Some(GoState::TransportStopped),
        Some(_) => return,
        None => None,
    };

    {
        let state = host
            .world_entities_mut()
            .ensure_represented_gameobject_use_state_like_cpp(command.gameobject_guid);
        state.map_id = Some(command.map_id);
        state.go_type = Some(command.go_type);
        state.gameobject_flags = command.gameobject_flags;
        state.loot_state = loot_state;
        state.loot_state_unit_guid = command.loot_state_unit_guid;
        state.go_state = go_state;
        state.dynamic_flags = command.dynamic_flags;
        state.linked_trap_entry = command.linked_trap_entry;
        state.linked_trap_guid = command.linked_trap_guid;
    }

    let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
}

pub async fn open_represented_gameobject_chest_with_template_money_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    gameobject_guid: ObjectGuid,
    source: GameObjectLootSource,
    template_money: (u32, u32),
) {
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true) {
        return;
    }
    if !represented_gameobject_exists_for_loot_like_cpp(host, gameobject_guid) {
        return;
    }

    record_represented_gameobject_chest_release_metadata_like_cpp(host, gameobject_guid, source);

    let is_first_represented_unique_use = !host
        .loot()
        .represented_unique_gameobject_use_contains_like_cpp(gameobject_guid);
    if source.loot_id == 0 && is_first_represented_unique_use {
        host.loot_mut()
            .insert_represented_unique_gameobject_use_like_cpp(gameobject_guid);
        host.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
            gameobject.add_unique_use_like_cpp(player_guid);
        });
        if source.should_autostore_push_loot_like_cpp() {
            autostore_represented_gameobject_chest_push_loot_like_cpp(
                host,
                item_guid_generator,
                gameobject_guid,
                source,
            )
            .await;
        }
        record_represented_gameobject_use_effects_like_cpp(
            host,
            gameobject_guid,
            player_guid,
            source.triggered_event_id,
            source.linked_trap_entry,
        );
    }
    let activated_now = set_represented_gameobject_loot_state_activated_like_cpp(
        host,
        gameobject_guid,
        player_guid,
    );
    if activated_now {
        let _ = queue_session_chest_gameobject_state_refresh_for_same_map_like_cpp(
            host,
            gameobject_guid,
        );
    }
    if !source.has_open_loot_like_cpp() {
        return;
    }

    let should_record_generation_effects = source.loot_id != 0
        && !host
            .loot()
            .cached_loot_contains_owner_like_cpp(gameobject_guid);
    let allowed_looters = if source.is_personal_encounter_loot_like_cpp() {
        Vec::new()
    } else if source.uses_personal_loot_like_cpp() {
        // C++ creates only `m_personalLoot[player]` for a personal chest
        // without a DungeonEncounter; group loot rules never widen it.
        vec![player_guid]
    } else if source.use_group_loot_rules {
        represented_group_looters_at_reward_distance_like_cpp(host, player_guid)
    } else {
        vec![player_guid]
    };
    ensure_represented_gameobject_chest_loot_like_cpp(
        host,
        gameobject_guid,
        player_guid,
        source,
        &allowed_looters,
        template_money,
    )
    .await;
    if should_record_generation_effects
        && host
            .loot()
            .cached_loot_contains_owner_like_cpp(gameobject_guid)
    {
        record_represented_gameobject_use_effects_like_cpp(
            host,
            gameobject_guid,
            player_guid,
            source.triggered_event_id,
            source.linked_trap_entry,
        );
    }

    if sync_represented_gameobject_loot_to_canonical_like_cpp(host, gameobject_guid, player_guid)
        .is_none()
    {
        host.loot_mut()
            .remove_cached_loot_for_owner_like_cpp(gameobject_guid);
        return;
    }

    let Some(loot) = host.loot().cached_loot_for_owner_like_cpp(gameobject_guid) else {
        return;
    };
    // C++ keeps and sends an empty non-encounter
    // `m_personalLoot[player]`. Encounter generation instead discards
    // empty pools in `GenerateDungeonEncounterPersonalLoot`, so only the
    // former bypasses the generic item/money availability gate.
    let empty_non_encounter_personal_pool = source.uses_personal_loot_like_cpp()
        && !source.is_personal_encounter_loot_like_cpp()
        && loot.allowed_looters.contains(&player_guid);
    if !empty_non_encounter_personal_pool
        && !host
            .loot()
            .represented_loot_can_be_opened_by_player_like_cpp(gameobject_guid, loot, player_guid)
    {
        return;
    }

    let response = LootResponse {
        owner: gameobject_guid,
        loot_obj: loot.loot_guid,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
        loot_method: loot.loot_method,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: host.loot().represented_loot_money_for_player_like_cpp(
            gameobject_guid,
            loot,
            player_guid,
        ),
        items: represented_loot_response_items_like_cpp(loot, player_guid),
        currencies: vec![],
        acquired: true,
        ae_looting: false,
    };

    if host.has_active_non_item_loot_views_like_cpp() {
        host.do_loot_release_all_like_cpp(player_guid).await;
    }
    host.loot_mut().set_active_loot_guid(gameobject_guid);
    represented_on_loot_opened_with_catalogs_like_cpp(
        host,
        item_valuation,
        gameobject_guid,
        player_guid,
        response,
    );
}

pub async fn open_represented_fishing_hole_with_catalogs_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    gameobject_guid: ObjectGuid,
    gameobject_entry: u32,
    loot_id: u32,
) {
    let player_guid = host.player_guid();
    let should_update_criteria = player_guid.is_some()
        && loot_id != 0
        && host.hub_ref().resolved_player_is_alive_like_cpp() == Some(true)
        && represented_gameobject_exists_for_loot_like_cpp(host, gameobject_guid);
    open_represented_gameobject_personal_loot_like_cpp(
        host,
        item_valuation,
        gameobject_guid,
        loot_id,
        LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
        true,
    )
    .await;
    if should_update_criteria {
        let player_guid = player_guid.expect("checked above");
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::FishingHoleCatchCriteriaUpdated {
                    gameobject_guid,
                    player_guid,
                    gameobject_entry,
                },
            );
    }
}

pub async fn open_represented_fishing_node_loot_with_catalogs_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    gameobject_guid: ObjectGuid,
    area_id: u32,
    junk: bool,
) {
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true) {
        return;
    }
    if !represented_gameobject_exists_for_loot_like_cpp(host, gameobject_guid) {
        return;
    }
    let install_observation =
        represented_gameobject_loot_install_observation_like_cpp(host, gameobject_guid);
    if install_observation.is_none() && !host.represented_local_loot_fixture_allowed_like_cpp() {
        return;
    }

    let loot_type = if junk {
        LOOT_TYPE_FISHING_JUNK_LIKE_CPP
    } else {
        LOOT_TYPE_FISHING_LIKE_CPP
    };
    let loot_mode = if junk {
        LOOT_MODE_JUNK_FISH_LIKE_CPP
    } else {
        LOOT_MODE_DEFAULT_LIKE_CPP
    };
    let items = generate_represented_fishing_loot_items_like_cpp(host, area_id, loot_mode)
        .await
        .unwrap_or_else(|| {
            debug!(
                area_id,
                gameobject = ?gameobject_guid,
                junk,
                "fishing loot template unavailable"
            );
            Vec::new()
        });

    let Some(loot_guid) = next_represented_loot_object_guid_like_cpp(host, gameobject_guid) else {
        return;
    };
    host.loot_mut().insert_cached_loot_for_owner_like_cpp(
        gameobject_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items,
            looted_by_player: false,
        },
    );

    if let Some(loot) = host
        .loot_mut()
        .cached_loot_for_owner_mut_like_cpp(gameobject_guid)
    {
        mark_loot_allowed_for_player_like_cpp(loot, player_guid);
    }
    let upserted = host
        .loot()
        .cached_loot_for_owner_like_cpp(gameobject_guid)
        .cloned()
        .and_then(|loot| {
            install_observation.as_ref().and_then(|observation| {
                upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
                    host,
                    gameobject_guid,
                    player_guid,
                    loot,
                    false,
                    observation,
                )
            })
        });
    if upserted.is_none() && !host.represented_local_loot_fixture_allowed_like_cpp() {
        host.loot_mut()
            .remove_cached_loot_for_owner_like_cpp(gameobject_guid);
        return;
    }

    let Some(loot) = host.loot().cached_loot_for_owner_like_cpp(gameobject_guid) else {
        return;
    };
    if !host
        .loot()
        .represented_loot_can_be_opened_by_player_like_cpp(gameobject_guid, loot, player_guid)
    {
        return;
    }

    let response = LootResponse {
        owner: gameobject_guid,
        loot_obj: loot.loot_guid,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
        loot_method: loot.loot_method,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: loot.coins,
        items: represented_loot_response_items_like_cpp(loot, player_guid),
        currencies: vec![],
        acquired: true,
        ae_looting: false,
    };

    if host.has_active_non_item_loot_views_like_cpp() {
        host.do_loot_release_all_like_cpp(player_guid).await;
    }
    host.loot_mut().set_active_loot_guid(gameobject_guid);
    represented_on_loot_opened_with_catalogs_like_cpp(
        host,
        item_valuation,
        gameobject_guid,
        player_guid,
        response,
    );
}

pub async fn open_represented_gathering_node_with_catalogs_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    gameobject_guid: ObjectGuid,
    gameobject_entry: u32,
    source: GatheringNodeUseSource,
) {
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true) {
        return;
    }
    if !represented_gameobject_exists_for_loot_like_cpp(host, gameobject_guid) {
        return;
    }

    let is_first_represented_use = !host
        .loot()
        .represented_unique_gameobject_use_contains_like_cpp(gameobject_guid);
    if is_first_represented_use {
        host.loot_mut()
            .insert_represented_unique_gameobject_use_like_cpp(gameobject_guid);
        host.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
            gameobject.add_unique_use_like_cpp(player_guid);
        });
    }

    open_represented_gameobject_personal_loot_like_cpp(
        host,
        item_valuation,
        gameobject_guid,
        source.loot_id,
        LOOT_TYPE_CHEST_LIKE_CPP,
        false,
    )
    .await;

    if is_first_represented_use {
        let xp = {
            let (s, h) = host.split_loot_ref();
            s.represented_gathering_node_xp_like_cpp(h, source.xp_difficulty)
        };
        if xp != 0 {
            host.give_xp(xp, ObjectGuid::EMPTY, 1.0).await;
        }
        record_represented_gameobject_use_effects_like_cpp(
            host,
            gameobject_guid,
            player_guid,
            source.triggered_event_id,
            source.linked_trap_entry,
        );
    }
    record_represented_gathering_node_runtime_state_like_cpp(
        host,
        gameobject_guid,
        gameobject_entry,
        player_guid,
        source,
        is_first_represented_use,
    );
    let _ =
        queue_gathering_node_gameobject_state_refresh_for_same_map_like_cpp(host, gameobject_guid);
}

fn record_represented_gathering_node_runtime_state_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    gameobject_entry: u32,
    player_guid: ObjectGuid,
    source: GatheringNodeUseSource,
    is_first_represented_use: bool,
) {
    {
        let state = host
            .world_entities_mut()
            .ensure_represented_gameobject_use_state_like_cpp(gameobject_guid);
        if is_first_represented_use {
            state.personal_loot_uses = state.personal_loot_uses.saturating_add(1);
        }
        state.go_type = Some(GAMEOBJECT_TYPE_GATHERING_NODE as u8);
        state.gathering_node_loot_id = Some(source.loot_id);
        if state.personal_loot_uses >= source.max_loots {
            state.go_state = Some(GoState::Active);
            state.dynamic_flags |= GO_DYNFLAG_LO_NO_INTERACT;
        }
        state.linked_trap_entry =
            (source.linked_trap_entry != 0).then_some(source.linked_trap_entry);
    }

    let activated_now = set_represented_gameobject_loot_state_activated_like_cpp(
        host,
        gameobject_guid,
        player_guid,
    );
    if activated_now && source.despawn_delay_secs != 0 {
        if let Some(state) = host
            .world_entities_mut()
            .represented_gameobject_use_state_mut_like_cpp(gameobject_guid)
        {
            state.despawn_delay_secs = Some(source.despawn_delay_secs);
            state.despawn_delay_until =
                Some(Instant::now() + Duration::from_secs(u64::from(source.despawn_delay_secs)));
        }
    }

    if is_first_represented_use && source.spell_id != 0 {
        wow_world_entities::apply_represented_gameobject_post_use_spell_like_cpp(
            host,
            gameobject_guid,
            player_guid,
            gameobject_entry,
            GAMEOBJECT_TYPE_GATHERING_NODE,
            source.spell_id,
            false,
            RepresentedGameObjectSpellCaster::User,
            player_guid,
        );
    }
}

fn set_represented_gameobject_loot_state_activated_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> bool {
    let state = host
        .world_entities_mut()
        .ensure_represented_gameobject_use_state_like_cpp(gameobject_guid);
    if state.loot_state == Some(LootState::Activated) {
        return false;
    }

    state.loot_state = Some(LootState::Activated);
    state.loot_state_unit_guid = player_guid;
    true
}

fn record_represented_gameobject_use_effects_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    triggered_event_id: u32,
    linked_trap_entry: u32,
) {
    if triggered_event_id != 0 {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: triggered_event_id,
                },
            );
    }
    if linked_trap_entry != 0 {
        host.world_entities_mut()
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                    gameobject_guid,
                    player_guid,
                    trap_entry: linked_trap_entry,
                },
            );
    }
}

fn gathering_node_gameobject_state_refresh_command_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    gameobject_guid: ObjectGuid,
) -> Option<SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand> {
    let state = host
        .world_entities()
        .represented_gameobject_use_state_like_cpp(gameobject_guid)?;
    Some(SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand {
        gameobject_guid,
        map_id: host.hub_ref().core.player_map_id_like_cpp(),
        instance_id: host
            .hub_ref()
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0),
        go_type: state.go_type?,
        loot_state: state.loot_state.map(|loot_state| loot_state as u8),
        loot_state_unit_guid: state.loot_state_unit_guid,
        go_state: state.go_state.map(|go_state| go_state as i8),
        dynamic_flags: state.dynamic_flags,
        gathering_node_loot_id: state.gathering_node_loot_id,
        personal_loot_uses: state.personal_loot_uses,
        linked_trap_entry: state.linked_trap_entry,
        linked_trap_guid: state.linked_trap_guid,
    })
}

fn goober_gameobject_state_refresh_command_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    gameobject_guid: ObjectGuid,
) -> Option<SyncGooberGameobjectStateAndRefreshLikeCppCommand> {
    let state = host
        .world_entities()
        .represented_gameobject_use_state_like_cpp(gameobject_guid)?;
    Some(SyncGooberGameobjectStateAndRefreshLikeCppCommand {
        gameobject_guid,
        map_id: host.hub_ref().core.player_map_id_like_cpp(),
        instance_id: host
            .hub_ref()
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0),
        go_type: state.go_type.unwrap_or(GAMEOBJECT_TYPE_GOOBER as u8),
        gameobject_flags: state.gameobject_flags,
        loot_state: state.loot_state.map(|loot_state| loot_state as u8),
        loot_state_unit_guid: state.loot_state_unit_guid,
        go_state: state.go_state.map(|go_state| go_state as i8),
        dynamic_flags: state.dynamic_flags,
        linked_trap_entry: state.linked_trap_entry,
        linked_trap_guid: state.linked_trap_guid,
    })
}

pub fn queue_goober_gameobject_state_refresh_for_same_map_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    gameobject_guid: ObjectGuid,
) -> usize {
    let Some(player_guid) = host.hub_ref().core.player_guid() else {
        return 0;
    };
    let Some(registry) = host.hub_ref().core.player_registry() else {
        return 0;
    };
    let Some(command) = goober_gameobject_state_refresh_command_like_cpp(host, gameobject_guid)
    else {
        return 0;
    };
    let current_map_id = host.hub_ref().core.player_map_id_like_cpp();
    let current_instance_id = host
        .hub_ref()
        .core
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    let mut queued = 0;

    for registration in
        registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
    {
        if registry
            .try_send_current_command(
                registration,
                SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command.clone()),
            )
            .is_ok()
        {
            queued += 1;
        }
    }

    queued
}

fn queue_gathering_node_gameobject_state_refresh_for_same_map_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &H,
    gameobject_guid: ObjectGuid,
) -> usize {
    let Some(player_guid) = host.hub_ref().core.player_guid() else {
        return 0;
    };
    let Some(registry) = host.hub_ref().core.player_registry() else {
        return 0;
    };
    let Some(command) =
        gathering_node_gameobject_state_refresh_command_like_cpp(host, gameobject_guid)
    else {
        return 0;
    };
    let current_map_id = host.hub_ref().core.player_map_id_like_cpp();
    let current_instance_id = host
        .hub_ref()
        .core
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    let mut queued = 0;

    for registration in
        registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
    {
        if registry
            .try_send_current_command(
                registration,
                SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(command.clone()),
            )
            .is_ok()
        {
            queued += 1;
        }
    }

    queued
}

/// The session facade over the loot-release chest refresh (#1363 slice 5b;
/// moved from the World `LootCxRef` helper, renamed so it no longer collides
/// with the loot-release free function exported at the crate root).
pub fn queue_session_chest_gameobject_state_refresh_for_same_map_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &H,
    gameobject_guid: ObjectGuid,
) -> usize {
    crate::queue_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &host.hub_ref().core.loot_release_access_like_cpp(),
        host.world_entities(),
        gameobject_guid,
    )
}

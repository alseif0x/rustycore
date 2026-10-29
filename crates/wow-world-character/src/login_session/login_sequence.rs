// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The persisted transport resolve and the login packet sequence
//! (`HandlePlayerLogin` -> `SendInitialPacketsBeforeAddToMap` ->
//! `Map::AddPlayerToMap` -> `SendInitialPacketsAfterAddToMap`), moved from
//! `wow-world` `handlers/character/session_state.rs` (#1363 slice 8d).

use crate::character_login_support::CharacterLoginLocationLikeCpp;
use crate::login_session::initial_packets::send_initial_packets_before_add_to_map;
use crate::login_session::lifecycle::{
    load_map_corpse_data_like_cpp, load_player_customizations_like_cpp,
};
use crate::login_session::login_data::load_active_player_trait_configs_like_cpp;
use crate::login_session::login_recovery::{
    abort_partial_login_sequence_like_cpp, continue_login_after_grid_load_like_cpp,
};
use crate::login_session::world_entry::send_handle_player_login_packets_like_cpp;
use crate::{
    CharacterLoginHostLikeCpp, send_initial_packets_after_add_to_map_with_catalogs_like_cpp,
};
use std::collections::HashMap;
use tracing::{info, warn};
use wow_constants::PowerType;
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_data::{TaxiPathNodeEntry, TaxiPathNodeStore};
use wow_packet::packets::misc::{AccountMount, SpellChargeEntry, SpellHistoryEntry};
use wow_packet::packets::movement::TransportInfo;
use wow_packet::packets::update::{PlayerCombatStats, UpdateBlock, UpdateObject};
use wow_persistence::{
    PlayerCorpseLocationLoadRow, PlayerLoginAuxiliaryLoadOutcomeLikeCpp,
    PlayerLoginAuxiliaryLoadRequestLikeCpp, PlayerLoginAuxiliaryLoadedLikeCpp,
    PlayerLoginTransportLoadOutcomeLikeCpp, PlayerLoginTransportLoadRequestLikeCpp,
};
use wow_world_core::session::map_admission::PlayerGridLoadResolverLikeCpp;
use wow_world_core::session::{SessionState, SupportFeaturePolicyLikeCpp, game_time_ms_like_cpp};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;
use wow_world_inventory::primary_max_power_for_class_like_cpp;
use wow_world_lifecycle::login_transport::{
    PersistedTransportLoginLikeCpp, compose_init_self_create_blocks_like_cpp,
    map_transport_create_block_like_cpp, map_transport_create_from_load_row_like_cpp,
    player_visibility_create_update_from_snapshot_like_cpp, transport_position_for_login_like_cpp,
    transport_route_contains_saved_map_like_cpp, validate_persisted_transport_login_like_cpp,
};
use wow_world_visibility::InitTransportsPlanLikeCpp;

/// Resolve C++ `Player::LoadFromDB`'s persisted transport passenger state
/// against the currently materialized MO-transport path.
pub async fn resolve_persisted_transport_login_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &H,
    guid_low: u64,
    saved_map_id: u16,
    offset: Position,
) -> Option<PersistedTransportLoginLikeCpp> {
    let port = host.lifecycle().player_lifecycle_port_like_cpp()?;
    let rows = match port
        .load_login_transports_like_cpp(PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low })
        .await
    {
        PlayerLoginTransportLoadOutcomeLikeCpp::Loaded(rows) => rows,
        PlayerLoginTransportLoadOutcomeLikeCpp::Failed { .. } => return None,
    };
    let transport_create = map_transport_create_from_load_row_like_cpp(*rows.first()?);

    let data_dir = host
        .hub_ref()
        .config
        .mmap_runtime_config_like_cpp()
        .data_dir
        .clone();
    let taxi_path_nodes = TaxiPathNodeStore::load(&data_dir, &host.core().locale).ok()?;
    let nodes: Vec<TaxiPathNodeEntry> = taxi_path_nodes
        .entries()
        .filter(|node| node.path_id == transport_create.taxi_path_id)
        .cloned()
        .collect();
    // TransportMgr creates one same-GUID transport object for every map in
    // the template route. C++ first asks the character's saved map for that
    // object before it may follow GetExpectedMapId() to the current leg.
    if !transport_route_contains_saved_map_like_cpp(
        nodes.iter().map(|node| node.continent_id),
        saved_map_id,
    ) {
        return None;
    }
    let transport_position = transport_position_for_login_like_cpp(
        &nodes,
        transport_create.move_speed,
        transport_create.accel_rate,
        game_time_ms_like_cpp(),
    )?;
    let guid = ObjectGuid::create_transport(HighGuid::Transport, guid_low as i64);
    validate_persisted_transport_login_like_cpp(guid, offset, transport_position, transport_create)
}

/// C++ `Map::SendInitSelf` appends other passengers on the player's
/// current transport after the player's own CREATE block, but only when
/// `HaveAtClient(passenger)` was already true.
pub(super) fn init_self_fellow_transport_passenger_blocks_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &H,
    map_id: u16,
    transport_guid: ObjectGuid,
) -> Vec<UpdateBlock> {
    let (Some(player_guid), Some(registry)) = (host.player_guid(), host.core().player_registry())
    else {
        return Vec::new();
    };
    let instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    let mut passengers: Vec<_> = registry
        .fellow_transport_passengers(player_guid, map_id, instance_id, transport_guid)
        .into_iter()
        .filter(|passenger| {
            host.core()
                .client_visible_guids_like_cpp
                .contains(&passenger.guid)
        })
        .collect();
    passengers.sort_by_key(|passenger| passenger.guid);
    passengers
        .into_iter()
        .filter_map(|passenger| {
            player_visibility_create_update_from_snapshot_like_cpp(&passenger, map_id)
                .blocks
                .pop()
        })
        .collect()
}

/// Plan C++ `Map::SendInitSelf`'s current transport plus
/// `Map::SendInitTransports`' remaining map transports from one stable
/// path-time snapshot.
pub(super) async fn plan_init_transports_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    map_id: u16,
    persisted_transport: Option<PersistedTransportLoginLikeCpp>,
) -> Box<InitTransportsPlanLikeCpp> {
    host.visibility_mut()
        .clear_client_visible_transports_like_cpp();
    let mut plan = Box::new(InitTransportsPlanLikeCpp::default());
    let now_ms = game_time_ms_like_cpp();
    if let Some(snapshot) = persisted_transport {
        if snapshot.map_id == map_id {
            plan.own_transport = Some((
                snapshot.guid,
                map_transport_create_block_like_cpp(
                    snapshot.transport_create,
                    snapshot.transport_position,
                    now_ms,
                ),
            ));
            plan.considered += 1;
        } else {
            // A validated attachment and the selected login map must be
            // one snapshot. Fail closed instead of sending a player whose
            // nested transport reference has no preceding CREATE block.
            host.hub_mut().set_player_transport_info_like_cpp(None);
        }
    }
    let Some(port) = host.lifecycle().player_lifecycle_port_like_cpp().cloned() else {
        return plan;
    };

    let data_dir = host
        .hub_ref()
        .config
        .mmap_runtime_config_like_cpp()
        .data_dir
        .clone();
    let locale = host.core().locale.clone();
    let taxi_path_nodes = match TaxiPathNodeStore::load(&data_dir, &locale) {
        Ok(store) => store,
        Err(error) => {
            warn!(
                map_id,
                data_dir,
                locale,
                %error,
                "RUST_LOGIN send_init_transports skipped: TaxiPathNode.db2 load failed"
            );
            return plan;
        }
    };

    let mut nodes_by_path: HashMap<u16, Vec<TaxiPathNodeEntry>> = HashMap::new();
    for node in taxi_path_nodes.entries() {
        nodes_by_path
            .entry(node.path_id)
            .or_default()
            .push(node.clone());
    }

    let transports = match port
        .load_login_transports_like_cpp(PlayerLoginTransportLoadRequestLikeCpp::All)
        .await
    {
        PlayerLoginTransportLoadOutcomeLikeCpp::Loaded(rows) => rows
            .into_iter()
            .map(map_transport_create_from_load_row_like_cpp)
            .collect::<Vec<_>>(),
        PlayerLoginTransportLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                map_id,
                %reason,
                "RUST_LOGIN send_init_transports skipped: DB query failed"
            );
            return plan;
        }
    };

    if transports.is_empty() {
        return plan;
    }

    let player_transport_guid = host.hub_ref().player_transport_guid_like_cpp();

    for transport in transports {
        let transport_guid =
            ObjectGuid::create_transport(HighGuid::Transport, transport.guid_low as i64);
        if player_transport_guid == Some(transport_guid) {
            // The validated own transport was materialized above without a
            // second fallible query/path load.
            continue;
        }
        let Some(nodes) = nodes_by_path.get(&transport.taxi_path_id) else {
            plan.skipped_missing_path += 1;
            continue;
        };
        let Some(path_position) = transport_position_for_login_like_cpp(
            nodes,
            transport.move_speed,
            transport.accel_rate,
            now_ms,
        ) else {
            plan.skipped_missing_path += 1;
            continue;
        };

        if path_position.map_id != map_id {
            plan.skipped_other_map += 1;
            continue;
        }

        let (target_phase_shift, _) = host.hub_ref().catalogs.db_spawn_phase_shift_like_cpp(
            map_id,
            transport.phase_use_flags,
            transport.phase_id,
            transport.phase_group_id,
            -1,
        );
        if !host.should_send_init_transport_like_cpp(transport_guid, &target_phase_shift) {
            plan.skipped_phase += 1;
            continue;
        }

        plan.considered += 1;
        plan.other_blocks.push(map_transport_create_block_like_cpp(
            transport,
            path_position,
            now_ms,
        ));
        plan.other_visible_guids.push(transport_guid);
    }

    info!(
        map_id,
        own_transport = plan.own_transport.is_some(),
        other_blocks = plan.other_blocks.len(),
        considered = plan.considered,
        skipped_other_map = plan.skipped_other_map,
        skipped_missing_path = plan.skipped_missing_path,
        skipped_phase = plan.skipped_phase,
        "RUST_LOGIN send_init_transports plan"
    );

    plan
}

/// Send the player login packet sequence to the client.
///
/// Follows the C++ login phases:
/// HandlePlayerLogin → SendInitialPacketsBeforeAddToMap → AddToMap →
/// SendInitialPacketsAfterAddToMap.
///
/// AuthResponse, SetTimeZone, FeatureSystemStatusGlueScreen,
/// AccountDataTimes(global), and TutorialFlags are first sent during
/// session init. C++ intentionally resends the account-data times,
/// tutorials, and time-zone packets during `HandlePlayerLogin`.
pub async fn send_login_sequence<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    trait_node_entries: &wow_data::trait_tree::TraitNodeEntryStore,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    feature_policy: &SupportFeaturePolicyLikeCpp,
    player_grid_loader: &PlayerGridLoadResolverLikeCpp,
    guid: ObjectGuid,
    race: u8,
    class: u8,
    sex: u8,
    level: u8,
    display_id: u32,
    position: &Position,
    map_id: i32,
    zone_id: i32,
    homebind: CharacterLoginLocationLikeCpp,
    persisted_transport_login: Option<PersistedTransportLoginLikeCpp>,
    visible_items: [(i32, u16, u16); 19],
    inv_slots: [ObjectGuid; 141],
    item_creates: Vec<wow_packet::packets::update::ItemCreateData>,
    combat: PlayerCombatStats,
    current_power0: i32,
    base_mana: i32,
    known_spells: Vec<i32>,
    favorite_spells: Vec<i32>,
    spell_history_entries: Vec<SpellHistoryEntry>,
    spell_charge_entries: Vec<SpellChargeEntry>,
    action_buttons: [i64; 180],
    skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
    account_mounts: Vec<AccountMount>,
) -> bool {
    let updateobject_trace_enabled = std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some();
    let authoritative_grid_map_key = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .filter(|key| u32::try_from(map_id).ok() == Some(key.map_id));
    let grid_instance_id = authoritative_grid_map_key
        .map(|key| key.instance_id)
        .unwrap_or(0);
    // Rust's loaded-grid bridge also validates that the canonical map is
    // usable. C++ finishes all fallible `Player::LoadFromDB` map selection
    // before emitting successful-login packets, so run this bridge before
    // Phase 1 and retain its outcome for the Map::AddPlayerToMap trace.
    let grid_load_outcome = Some(player_grid_loader(
        map_id as u16,
        authoritative_grid_map_key.map(|key| key.instance_id),
        *position,
    ));
    if !continue_login_after_grid_load_like_cpp(
        host,
        guid,
        map_id,
        grid_instance_id,
        grid_load_outcome,
    ) {
        return false;
    }
    // C++ `LoginQueryHolder` carries `SEL_CORPSE_LOCATION` into
    // `Player::LoadCorpse`; keep its row local until the Player is in the map.
    let persisted_corpse_location = load_corpse_location_for_login(host, guid).await;
    let corpse_load_outcome =
        load_map_corpse_data_like_cpp(host, map_id as u16, grid_instance_id).await;
    if corpse_load_outcome.rows_seen != 0
        || corpse_load_outcome.already_loaded
        || corpse_load_outcome.invalid_type_rows != 0
        || corpse_load_outcome.invalid_race_rows != 0
        || corpse_load_outcome.invalid_position_rows != 0
        || corpse_load_outcome.add_to_map_errors != 0
    {
        info!(
            map_id,
            instance_id = grid_instance_id,
            already_loaded = corpse_load_outcome.already_loaded,
            rows_seen = corpse_load_outcome.rows_seen,
            corpses_added = corpse_load_outcome.corpses_added,
            invalid_type_rows = corpse_load_outcome.invalid_type_rows,
            invalid_race_rows = corpse_load_outcome.invalid_race_rows,
            invalid_position_rows = corpse_load_outcome.invalid_position_rows,
            add_to_map_errors = corpse_load_outcome.add_to_map_errors,
            "Loaded canonical map corpse data like C++ Map::LoadCorpseData"
        );
    }

    // ── Phase 1: HandlePlayerLogin packets ──
    let motd = wow_config::get_value_default::<String>("Motd", DEFAULT_MOTD_LIKE_CPP.to_string());
    let account_mount_login_partials = crate::account_mount_login_partial_rows_like_cpp(host);
    if !send_handle_player_login_packets_like_cpp(
        host,
        item_guid_generator,
        feature_policy,
        guid,
        position,
        map_id,
        &account_mount_login_partials,
        &motd,
    )
    .await
    {
        return false;
    }

    // ── Phase 2: SendInitialPacketsBeforeAddToMap ──
    if !send_initial_packets_before_add_to_map(
        host,
        guid,
        position,
        map_id,
        zone_id,
        homebind,
        known_spells,
        favorite_spells,
        spell_history_entries,
        spell_charge_entries,
        action_buttons,
        account_mounts,
        updateobject_trace_enabled,
    )
    .await
    {
        return false;
    }

    // ── C++ Map::AddPlayerToMap ──
    if updateobject_trace_enabled {
        info!(
            guid = ?guid,
            map_id,
            instance_id = grid_instance_id,
            init_player = 1u8,
            player_x = position.x,
            player_y = position.y,
            player_z = position.z,
            player_o = position.orientation,
            "RUST_LOGIN map_add start"
        );
    }

    // C++ `Map::AddPlayerToMap` performs EnsureGridLoadedForActiveObject
    // before AddToWorld/SendInitSelf (`Map.cpp:443-470`). The Rust bridge
    // was preflighted before Phase 1 above because it also contains
    // fallible map validation; report the retained result at this
    // equivalent map-add point.
    if let Some(outcome) = grid_load_outcome {
        info!(
            map_id,
            instance_id = grid_instance_id,
            map_unavailable = outcome.map_unavailable,
            map_created = outcome.map_created,
            grid_loaded_now = outcome.grid_loaded_now,
            metadata_entries = outcome.metadata_entries,
            skipped_already_loaded = outcome.skipped_already_loaded,
            skipped_should_not_spawn = outcome.skipped_should_not_spawn,
            skipped_difficulty_mismatch = outcome.skipped_difficulty_mismatch,
            stale_index_entries = outcome.stale_index_entries,
            creature_records_added = outcome.creature_records_added,
            gameobject_records_added = outcome.gameobject_records_added,
            area_trigger_records_added = outcome.area_trigger_records_added,
            pre_add_records_added = outcome.pre_add_records_added,
            add_to_map_errors = outcome.add_to_map_errors,
            load_record_missing = outcome.load_record_missing,
            creature_load_record_missing = outcome.creature_load_record_missing,
            gameobject_load_record_missing = outcome.gameobject_load_record_missing,
            area_trigger_load_record_missing = outcome.area_trigger_load_record_missing,
            "RUST_LOGIN grid_load"
        );
    }
    if updateobject_trace_enabled {
        info!(guid = ?guid, "RUST_LOGIN map_add after_ensure_grid");
    }

    info!(
        guid = ?guid,
        map_id,
        "RUST_LOGIN map_add before_add_to_world"
    );
    let attached_controller = host.ensure_login_player_controller_like_cpp(
        guid,
        host.hub_ref()
            .player_name_like_cpp()
            .unwrap_or_else(|| format!("Player{}", guid.counter())),
        *position,
        map_id as u16,
        race,
        class,
        level,
        sex,
    );
    if attached_controller {
        let _ = host.ensure_canonical_world_map_for_current_player_like_cpp();
    }
    // #1263 C2: the login attach result, reported at the C++
    // `Map::AddPlayerToMap` equivalent point. Every guard taken by the two
    // helpers above is released before the event is emitted, so no I/O
    // runs under a guard.
    {
        let canonical_key = host.core().current_canonical_player_map_key_like_cpp();
        let (resolved_map_id, resolved_instance_id) =
            host.core().current_runtime_map_key_like_cpp();
        let requested_map_id = u16::try_from(map_id).unwrap_or(u16::MAX);
        host.core().emit_runtime_location_capture_like_cpp(
            wow_world_core::session::RuntimeLocationCaptureLikeCpp {
                capture_id: "C2",
                root: "WorldSession::send_login_sequence::login_attach",
                phase: "login_attach",
                player_guid_counter: Some(guid.counter() as u32),
                session_state: wow_world_core::session::session_state_name_like_cpp(
                    host.core().state,
                ),
                requested_map_id,
                requested_instance_id: Some(grid_instance_id),
                canonical_key_present: canonical_key.is_some(),
                resolved_map_id,
                resolved_instance_id,
                detail: if attached_controller {
                    "login_player_controller_attached"
                } else {
                    "login_player_controller_preexisting"
                },
            },
        );
    }
    host.hub_mut().sync_canonical_player_health_like_cpp(
        combat.health.max(0).min(u32::MAX as i64) as u32,
        combat.max_health.max(1).min(u32::MAX as i64) as u32,
    );
    let primary_power_type = primary_power_type_for_class_like_cpp(class);
    let primary_max_power = primary_max_power_for_class_like_cpp(class, combat.max_mana);
    let primary_base_mana = if primary_power_type == PowerType::Mana {
        base_mana
    } else {
        0
    };
    host.hub_mut().sync_canonical_player_primary_power_like_cpp(
        primary_power_type,
        current_power0,
        primary_max_power,
        primary_base_mana,
    );
    if std::env::var_os("RUSTYCORE_SPELL_POWER_TRACE").is_some() {
        info!(
            guid = ?guid,
            class,
            power_type = ?primary_power_type,
            current_power0,
            max_power0 = primary_max_power,
            base_mana = primary_base_mana,
            "RUST_LOGIN_POWER_SYNC"
        );
    }
    host.lifecycle_mut()
        .set_login_time_like_cpp(Some(std::time::Instant::now()));
    host.world_entities_mut()
        .set_suppress_creature_movement_queued_at_or_before_like_cpp(None);
    // Clear per-session loot/combat state as part of the Rust AddToWorld
    // equivalent, before C++ would build `Map::SendInitSelf`.
    host.loot_mut().clear_cached_loot_like_cpp();
    host.loot_mut().set_active_loot_guid(ObjectGuid::EMPTY);
    host.hub_mut().set_combat_target_like_cpp(None);
    host.hub_mut().set_in_combat_like_cpp(false);
    info!(
        guid = ?guid,
        map_id,
        "RUST_LOGIN map_add after_add_to_world"
    );
    let mut init_transports_plan =
        plan_init_transports_like_cpp(host, map_id as u16, persisted_transport_login).await;

    // C++ `Map::SendInitSelf` — current transport + items + player in a
    // single packet. The transport precedes the player's nested
    // MovementInfo::TransportInfo reference; items precede InvSlots.
    {
        // Build quest log for the UpdateObject (25 slots max).
        // C++ Player::BuildValuesCreate sends quest log fields in the
        // host-view player create block.
        // StateFlags: 0=None, 1=Complete (QuestSlotStateMask)
        let quest_log: Vec<(u32, u32, i64, [u16; 24])> = host.quest_log_create_entries_like_cpp();

        let account_toys = {
            let (state, hub) = host.split_lifecycle_ref();
            state.account_toy_active_player_rows_like_cpp(hub)
        };
        let account_heirlooms = {
            let (state, hub) = host.split_lifecycle_ref();
            state.account_heirloom_active_player_rows_like_cpp(hub)
        };
        let account_transmog = host
            .hub_ref()
            .account_transmog_active_player_rows_like_cpp();
        let trait_configs =
            load_active_player_trait_configs_like_cpp(host, trait_node_entries, guid).await;
        let player_customizations = load_player_customizations_like_cpp(host, guid).await;
        {
            let (state, mut hub) = host.split_lifecycle_mut();
            state.set_loaded_player_customizations_like_cpp(&mut hub, player_customizations.clone())
        };
        let (Some(player_xp), Some(player_next_level_xp), Some(scaling_level_delta)) = (
            host.hub_ref().resolved_player_xp_like_cpp(),
            host.hub_ref().resolved_player_next_level_xp_like_cpp(),
            host.hub_ref()
                .resolved_player_scaling_level_delta_like_cpp(),
        ) else {
            return false;
        };
        let Some(player_money) = host.resolved_player_money_like_cpp() else {
            return false;
        };
        let Some((self_res_spells, no_reagent_cost_mask, summon)) =
            host.core().with_owned_player_like_cpp(|player| {
                (
                    player.self_res_spells().to_vec(),
                    player.no_reagent_cost_mask_like_cpp(),
                    player
                        .unit()
                        .minion_guid_like_cpp()
                        .unwrap_or(ObjectGuid::EMPTY),
                )
            })
        else {
            return false;
        };
        info!(
            toys = account_toys.len(),
            heirlooms = account_heirlooms.len(),
            transmog = account_transmog.len(),
            trait_configs = trait_configs.len(),
            customizations = player_customizations.len(),
            "Building player CREATE collection dynamic fields"
        );

        let mut player_pkt = UpdateObject::create_player_with_party_type(
            guid,
            race,
            class,
            sex,
            level,
            display_id,
            position,
            map_id as u16,
            zone_id as u32,
            true,
            visible_items,
            inv_slots,
            combat,
            skill_info,
            player_money,
            quest_log,
            host.core().party_member_party_type_like_cpp(),
        );
        let Some((player_flags, player_flags_ex)) =
            host.resolved_player_flags_for_create_like_cpp()
        else {
            return false;
        };
        let Some(aura_state) = host.core().canonical_player_snapshot_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .build_aura_state_update_for_target(guid)
        }) else {
            return false;
        };
        player_pkt.set_player_aura_state_like_cpp(aura_state);
        player_pkt.set_player_flags_like_cpp(player_flags, player_flags_ex);
        player_pkt.set_player_current_power0_like_cpp(current_power0);
        player_pkt.set_player_xp_like_cpp(player_xp.min(i32::MAX as u32) as i32);
        player_pkt
            .set_player_next_level_xp_like_cpp(player_next_level_xp.min(i32::MAX as u32) as i32);
        player_pkt.set_player_max_level_like_cpp(
            host.hub_ref().player_active_max_level_like_cpp() as i32,
        );
        player_pkt.set_player_scaling_level_delta_like_cpp(scaling_level_delta);
        let (Some(rest_threshold), Some(rest_state)) = (
            host.hub_ref().resolved_xp_rest_threshold_like_cpp(),
            host.hub_ref().resolved_xp_rest_state_like_cpp(),
        ) else {
            return false;
        };
        player_pkt.set_player_rest_info_like_cpp(0, rest_threshold, rest_state);
        if std::env::var_os("RUSTYCORE_SPELL_POWER_TRACE").is_some() {
            info!(
                guid = ?guid,
                current_power0,
                max_power0 = primary_max_power,
                base_mana = primary_base_mana,
                "RUST_LOGIN_POWER_CREATE"
            );
        }
        player_pkt.set_player_account_guids_like_cpp(
            ObjectGuid::create_global(HighGuid::WowAccount, 0, host.core().account_id as i64),
            ObjectGuid::create_global(
                HighGuid::BNetAccount,
                0,
                host.core().battlenet_account_id() as i64,
            ),
        );
        player_pkt.set_player_collection_dynamic_fields_like_cpp(
            account_toys,
            account_heirlooms,
            account_transmog,
            trait_configs,
        );
        player_pkt.set_player_self_res_spells(self_res_spells);
        player_pkt.set_player_no_reagent_cost_mask_like_cpp(no_reagent_cost_mask);
        player_pkt.set_player_summon_guid_like_cpp(summon);
        let Some(action_buttons) = host
            .hub_ref()
            .represented_action_buttons_snapshot_like_cpp()
        else {
            return false;
        };
        player_pkt.set_player_action_buttons_like_cpp(action_buttons);
        player_pkt.set_player_customizations_like_cpp(player_customizations);

        if let (Some((transport_guid, _)), Some(transport_position)) = (
            init_transports_plan.own_transport.as_ref(),
            host.hub_ref().player_transport_position_like_cpp(),
        ) {
            player_pkt.set_player_movement_transport_like_cpp(TransportInfo {
                guid: *transport_guid,
                x: transport_position.x,
                y: transport_position.y,
                z: transport_position.z,
                o: transport_position.orientation,
                seat: -1,
                time: 0,
                prev_time: None,
                vehicle_id: None,
            });
        }

        let fellow_passenger_blocks = init_transports_plan
            .own_transport
            .as_ref()
            .map(|(transport_guid, _)| {
                init_self_fellow_transport_passenger_blocks_like_cpp(
                    host,
                    map_id as u16,
                    *transport_guid,
                )
            })
            .unwrap_or_default();
        if !item_creates.is_empty()
            || init_transports_plan.own_transport.is_some()
            || !fellow_passenger_blocks.is_empty()
        {
            info!(
                items = item_creates.len(),
                own_transport = init_transports_plan.own_transport.is_some(),
                fellow_passengers = fellow_passenger_blocks.len(),
                "Sending C++ Map::SendInitSelf CREATE blocks"
            );
            if let Some(transport_guid) = compose_init_self_create_blocks_like_cpp(
                &mut player_pkt,
                item_creates,
                init_transports_plan.own_transport.take(),
                fellow_passenger_blocks,
            ) {
                host.visibility_mut()
                    .insert_client_visible_transport_like_cpp(transport_guid);
            }
        }

        if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
            for line in player_pkt.debug_create_summary_like_cpp() {
                info!("RUST_UPDATEOBJECT login_self {line}");
            }
        }
        host.send_packet(&player_pkt);
    }
    if updateobject_trace_enabled {
        info!(guid = ?guid, "RUST_LOGIN map_add after_send_init_self");
    }
    // C++ Map::AddPlayerToMap sends transports immediately after
    // SendInitSelf, before clearing the normal visible GUID cache.
    {
        let (s, mut h) = host.split_visibility_mut();
        s.send_init_transports_like_cpp(&mut h, map_id as u16, init_transports_plan)
    };
    if updateobject_trace_enabled {
        info!(guid = ?guid, "RUST_LOGIN map_add after_send_init_transports");
    }

    // C++ Map::AddPlayerToMap clears the visible GUID cache immediately
    // after SendInitSelf and SendInitTransports.
    if updateobject_trace_enabled {
        info!(
            guid = ?guid,
            count = host.core().client_visible_guids_like_cpp.len(),
            "RUST_LOGIN map_add before_clear_client_guids"
        );
    }
    host.core().client_visible_guids_like_cpp.clear();
    // C++ clears m_clientGUIDs here, then Player::SendInitialPacketsAfterAddToMap
    // starts with UpdateVisibilityForPlayer. Do not let Rust's movement-distance
    // throttle reuse the previous login/logout position after the clear.
    host.visibility_mut().clear_last_visibility_pos_like_cpp();
    if updateobject_trace_enabled {
        info!(
            guid = ?guid,
            count = host.core().client_visible_guids_like_cpp.len(),
            "RUST_LOGIN map_add after_clear_client_guids"
        );
    }

    // C++ `Map::AddPlayerToMap` clears `m_clientGUIDs`, then calls
    // `Player::UpdateObjectVisibility(false)`. In the traced 3.4.3 login
    // this only schedules `NOTIFY_VISIBILITY_CHANGED`; it does not emit
    // the nearby object create batch in this map-add phase.
    if updateobject_trace_enabled {
        info!(
            guid = ?guid,
            count = host.core().client_visible_guids_like_cpp.len(),
            "RUST_LOGIN map_add after_update_object_visibility"
        );
    }

    // ── Phase 4: SendInitialPacketsAfterAddToMap ──
    if !send_initial_packets_after_add_to_map_with_catalogs_like_cpp(
        host,
        creature_spawn_catalogs,
        guid,
        position,
        map_id,
        updateobject_trace_enabled,
    )
    .await
    {
        abort_partial_login_sequence_like_cpp(host);
        return false;
    }

    // C++ `Player::LoadCorpse` runs after map insertion and before login is
    // published. Corpse restoration consumes only resurrection catalogs.
    if let Err(reason) = wow_world_entities::restore_player_corpse_after_login(
        host,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator,
            creature_spawns: creature_spawn_catalogs,
            graveyards: None,
        },
        persisted_corpse_location,
    )
    .await
    {
        warn!(?guid, reason, "Login corpse restoration did not complete");
        abort_partial_login_sequence_like_cpp(host);
        return false;
    }

    // C++ does not deliver SMSG_ON_MONSTER_MOVE inside the initial
    // enter-world packet burst. Rust fan-out commands are queued from a
    // sessionless world tick, so remember the burst boundary and drop only
    // movement commands that were queued at or before it.
    host.world_entities_mut()
        .set_suppress_creature_movement_queued_at_or_before_like_cpp(Some(
            std::time::Instant::now(),
        ));

    // Rust keeps the session status flip after the initial after-add packet
    // subset so the network loop cannot process normal movement/gameplay
    // opcodes while the login stream is still being emitted. The represented
    // player itself was already installed above at the AddToWorld point.
    host.set_state(SessionState::LoggedIn);

    // 31. Existing nearby sessions run the same C++-style visibility diff
    //     so their client GUID caches gain this player symmetrically.
    host.notify_other_players_visibility_changed_like_cpp();

    // 32. Send full stat VALUES update so all character panel tabs
    //     (Melee, Ranged, Spell, Defense) display correct values on login.
    //     C++ has already applied loaded item enchantments at this point;
    //     merge their represented modifiers into this absolute snapshot.
    //     A second bonus-only packet would write its default fields as
    //     zero and corrupt unrelated client-visible stats.
    host.send_login_stat_update_with_represented_item_bonuses_like_cpp();

    info!(
        "Login sequence complete for {:?} (38 packets including broadcasts)",
        guid
    );
    true
}

async fn load_corpse_location_for_login<H: CharacterLoginHostLikeCpp>(
    host: &H,
    guid: ObjectGuid,
) -> Option<PlayerCorpseLocationLoadRow> {
    let Some(port) = host.lifecycle().player_lifecycle_port_like_cpp() else {
        warn!(
            player_guid = guid.counter(),
            "Cannot load persisted Player corpse location without the lifecycle port"
        );
        return None;
    };
    let outcome = port
        .load_login_auxiliary_like_cpp(PlayerLoginAuxiliaryLoadRequestLikeCpp::CorpseLocation {
            player_guid: guid.counter() as u64,
        })
        .await;
    match corpse_location_from_load_outcome(outcome) {
        Ok(row) => row,
        Err(reason) => {
            warn!(player_guid = guid.counter(), %reason, "Failed to load persisted Player corpse location during login");
            None
        }
    }
}

fn corpse_location_from_load_outcome(
    outcome: PlayerLoginAuxiliaryLoadOutcomeLikeCpp,
) -> Result<Option<PlayerCorpseLocationLoadRow>, String> {
    match outcome {
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::CorpseLocation(row),
        ) => Ok(row),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
            Err("unexpected auxiliary login payload for corpse location".to_owned())
        }
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => Err(reason),
    }
}

#[cfg(test)]
mod corpse_location_load_tests {
    use super::corpse_location_from_load_outcome;
    use wow_persistence::{
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadedLikeCpp,
    };

    #[test]
    fn empty_corpse_result_is_success_and_stays_distinct_from_query_failure() {
        let empty =
            corpse_location_from_load_outcome(PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                PlayerLoginAuxiliaryLoadedLikeCpp::CorpseLocation(None),
            ));
        let failed =
            corpse_location_from_load_outcome(PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "driver failure".to_owned(),
            });

        assert_eq!(empty, Ok(None));
        assert_eq!(failed, Err("driver failure".to_owned()));
    }
}

const DEFAULT_MOTD_LIKE_CPP: &str = "Welcome to a Trinity Core Server.";

fn primary_power_type_for_class_like_cpp(class_id: u8) -> PowerType {
    wow_world_core::session::primary_power_type_for_player_class_like_cpp(class_id)
}

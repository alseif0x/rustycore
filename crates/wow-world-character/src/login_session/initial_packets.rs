// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordered login packet bursts before and after canonical map insertion.

use crate::character_login_support::{
    CharacterLoginLocationLikeCpp, LoginWorldStateTemplateLikeCpp,
    apply_pvp_season_world_states_like_cpp, build_initial_world_states_like_cpp,
    login_bind_point_update_like_cpp, parse_login_world_state_area_ids_like_cpp,
    parse_login_world_state_map_ids_like_cpp,
};
use crate::{CharacterLoginHostLikeCpp, apply_post_add_zone_from_terrain_like_cpp};
use std::sync::Arc;
use tracing::{info, warn};
use wow_core::{ObjectGuid, Position};
use wow_packet::packets::misc::{
    AccountMount, AccountMountUpdate, AllAchievementData, InitWorldStates, InitialSetup,
    LoginSetTimeSpeed, MoveSetActiveMover, PhaseShiftChange, SendKnownSpells, SendSpellCharges,
    SendSpellHistory, SendUnlearnSpells, SetSpellModifier, SetupCurrency, SpellChargeEntry,
    SpellHistoryEntry, UpdateActionButtons, WorldServerInfo,
};
use wow_persistence::PlayerInitialWorldStateRowsLikeCpp;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

/// C++ `Player::SendInitialPacketsBeforeAddToMap` (Player.cpp:23479-23590): the init
/// packets sent before the player is added to the map, ending with `SetMovedUnit`
/// (SMSG_MOVE_SET_ACTIVE_MOVER). Currently called by login only; far-teleport
/// replay remains open (#NEXT.R8.ENTITIES.1229). The per-character items that
/// the caller already has on hand (known/favorite spells, spell history/charges, action
/// buttons, account mounts) plus the destination guid/position/map/zone are passed in.
#[allow(clippy::too_many_arguments)]
pub async fn send_initial_packets_before_add_to_map<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    guid: ObjectGuid,
    _position: &Position,
    _map_id: i32,
    _zone_id: i32,
    homebind: CharacterLoginLocationLikeCpp,
    known_spells: Vec<i32>,
    favorite_spells: Vec<i32>,
    spell_history_entries: Vec<SpellHistoryEntry>,
    spell_charge_entries: Vec<SpellChargeEntry>,
    action_buttons: [i64; 180],
    account_mounts: Vec<AccountMount>,
    updateobject_trace_enabled: bool,
) -> bool {
    if updateobject_trace_enabled {
        info!(guid = ?guid, "RUST_LOGIN before_initial_packets_before_add");
    }

    // C++ `Player::SendInitialPacketsBeforeAddToMap` resets m_movementCounter to 0 for a
    // non-seamless add (login / far teleport; Player.cpp:23483) before any control packets.
    host.hub_mut().reset_movement_counter_like_cpp();

    // 6. TimeSyncRequest (critical — client needs time sync)
    //    Also initializes the periodic timer (5s first, then 10s).
    host.core_mut().reset_time_sync_like_cpp();
    host.core_mut().send_time_sync();

    // 7. ContactList — C++ `GetSocial()->SendSocialList(this, SOCIAL_FLAG_ALL)`.
    if !host
        .core()
        .wait_for_instance_send_before_realm_send_like_cpp()
        .await
    {
        return false;
    }
    host.send_contact_list_like_cpp(7).await;

    // 8. BindPointUpdate — C++ `Player::SendBindPointUpdate` always uses
    // `m_homebind`/`m_homebindAreaId`, independently of the current login
    // location selected by `UpdatePositionData`.
    if !host
        .core()
        .wait_for_realm_send_before_instance_update_like_cpp()
        .await
    {
        return false;
    }
    host.send_packet(&login_bind_point_update_like_cpp(homebind));

    // 9. UpdateTalentData — C++ `Player::SendTalentsInfoData`.
    let Some(talent_data) = host.hub_ref().resolved_update_talent_data_packet_like_cpp() else {
        return false;
    };
    host.send_packet(&talent_data);

    // 10. SendKnownSpells — populated from character_spell table
    info!("Sending {} known spells for {:?}", known_spells.len(), guid);
    host.send_packet(&SendKnownSpells {
        initial_login: true,
        known_spells,
        favorite_spells,
    });

    // 11. SendUnlearnSpells (empty)
    host.send_packet(&SendUnlearnSpells);

    // 12. SendSpellHistory — C++ `SpellHistory::WritePacket`.
    host.send_packet(&SendSpellHistory {
        entries: spell_history_entries,
    });

    // 13. SendSpellCharges — C++ `SpellHistory::WritePacket`.
    host.send_packet(&SendSpellCharges {
        entries: spell_charge_entries,
    });

    // 14. ActiveGlyphs — full update; bindable spell mapping is still pending.
    if !host
        .core()
        .wait_for_instance_send_before_realm_send_like_cpp()
        .await
    {
        return false;
    }
    host.core()
        .send_packet_realm(&host.hub_ref().represented_active_glyphs_packet_like_cpp());

    // 15. UpdateActionButtons — populated from character_action table
    if !host
        .core()
        .wait_for_realm_send_before_instance_update_like_cpp()
        .await
    {
        return false;
    }
    host.send_packet(&UpdateActionButtons {
        buttons: action_buttons,
        reason: 0, // Initialization
    });

    // 16. InitializeFactions (1000 factions, all neutral)
    let Some(initialize_factions) = host
        .hub_mut()
        .mutate_reputation_mgr_like_cpp(|mgr| mgr.initialize_factions_packet_like_cpp())
    else {
        return false;
    };
    host.send_packet(&initialize_factions);

    // 17. SetupCurrency (empty)
    host.send_packet(&SetupCurrency::empty());

    // 18. LoadEquipmentSet
    if let Some(packet) = {
        let (state, hub) = host.split_inventory_ref();
        state.represented_load_equipment_set_packet_like_cpp(hub)
    } {
        host.send_packet(&packet);
    }

    // 19. AllAchievementData — C++ `AchievementMgr::SendAllData`.
    // `QuestObjectiveCriteriaMgr::SendAllData` does not emit
    // `AllAccountCriteria` in the traced 3.4.3 login when there is no
    // progress; do not synthesize an empty packet here.
    host.send_packet(&AllAchievementData);

    // 20. LoginSetTimeSpeed
    host.send_packet(&LoginSetTimeSpeed::now());

    // 21. WorldServerInfo
    host.send_packet(&WorldServerInfo::default_open_world());

    // 22. SetFlatSpellModifier + SetPctSpellModifier.
    //      C++ `Player::SendInitialPacketsBeforeAddToMap` calls
    //      `Player::SendSpellModifiers()` immediately after
    //      `WorldServerInfo` (`Player.cpp:23562-23563`). Fresh characters
    //      have empty modifier maps, but C++ still sends the packets.
    host.send_raw_packet(&SetSpellModifier::flat_empty().to_bytes());
    host.send_raw_packet(&SetSpellModifier::pct_empty().to_bytes());

    // 23. AccountMountUpdate
    host.send_packet(&AccountMountUpdate::full(account_mounts));

    // 24. AccountToyUpdate
    {
        let (state, hub) = host.split_lifecycle_ref();
        state.send_account_toys_like_cpp(hub)
    };

    // 25. AccountHeirloomUpdate
    {
        let (state, hub) = host.split_lifecycle_ref();
        state.send_account_heirlooms_like_cpp(hub)
    };

    // 26. AccountTransmogUpdate favorite appearances
    {
        let (state, hub) = host.split_inventory_ref();
        state.send_favorite_appearances_like_cpp(hub)
    };

    // 27. InitialSetup (expansion level)
    host.send_packet(&InitialSetup::wotlk());

    // C++ `Player::SendInitialPacketsBeforeAddToMap` ends with
    // `SetMovedUnit(this)`. `Unit::SetMovedUnit` updates server-side mover
    // state and sends `SMSG_MOVE_SET_ACTIVE_MOVER` to bind client input to
    // the player before the create block.
    host.set_player_moved_unit_guid_like_cpp(guid);
    host.send_packet(&MoveSetActiveMover { mover_guid: guid });
    if updateobject_trace_enabled {
        info!(guid = ?guid, "RUST_LOGIN after_initial_packets_before_add");
    }
    true
}

/// C++ `Player::SendInitialPacketsAfterAddToMap` (Player.cpp:23592-23685): the packets
/// sent after the player is added to the map — the post-add phase shift, visibility
/// mirror, `UpdateZone` -> SMSG_INIT_WORLD_STATES (resolved for the destination map),
/// CUF profiles, auras and the `PhasingHandler::OnMapChange` phase shift. Shared by login
/// and far teleport (#NEXT.R8.ENTITIES.1229). Reads all data from self; the destination
/// guid/position/map are passed in.
pub async fn send_initial_packets_after_add_to_map_with_catalogs_like_cpp<
    H: CharacterLoginHostLikeCpp,
>(
    host: &mut H,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    guid: ObjectGuid,
    position: &Position,
    map_id: i32,
    updateobject_trace_enabled: bool,
) {
    if updateobject_trace_enabled {
        info!(guid = ?guid, "RUST_LOGIN before_initial_packets_after_add");
    }

    // C++ Map::AddPlayerToMap sends the phase shift after
    // UpdateObjectVisibility(false), before post-add world-state packets.
    host.send_packet(&PhaseShiftChange::default_for(guid));

    // C++ `HandlePlayerLogin` calls `ObjectAccessor::AddObject` after
    // `Map::AddPlayerToMap` returns and before
    // `Player::SendInitialPacketsAfterAddToMap`
    // (`CharacterHandler.cpp:1241-1262`).
    host.register_in_player_registry();

    // C++ `HandlePlayerLogin` calls `ObjectAccessor::AddObject`, then
    // `Player::SendInitialPacketsAfterAddToMap`; that method starts with
    // `UpdateVisibilityForPlayer()`. Rust must force the same rebuild here
    // because Map::AddPlayerToMap just cleared the client-visible GUID cache;
    // after logout/relogin at the same position the normal movement-distance
    // throttle can otherwise leave the client with no visible creatures.
    host.sync_current_player_session_visibility_detection_like_cpp();
    host.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
        .await;
    if updateobject_trace_enabled {
        info!(
            guid = ?guid,
            count = host.core().client_visible_guids_like_cpp.len(),
            "RUST_LOGIN after_initial_update_visibility_for_player"
        );
    }

    if !apply_post_add_zone_from_terrain_like_cpp(host, map_id, position) {
        return;
    }
    let _ = {
        let (state, mut hub) = host.split_lifecycle_mut();
        state.advance_worldport_post_add_like_cpp(
            &mut hub,
            wow_entities::PlayerWorldportPostAddPhaseLikeCpp::ZoneApplied,
        )
    };
    // 27. InitWorldStates — C++ `Player::SendInitWorldStates` delegates to
    // `WorldStateMgr::FillInitialWorldStates`: realm values first, then map
    // values filtered by AreaIDs.
    let Some((represented_zone_id, represented_area_id)) =
        host.hub_ref().player_zone_area_like_cpp()
    else {
        return;
    };
    let world_states =
        load_initial_world_states_for_login_like_cpp(host, map_id, represented_area_id).await;
    host.send_packet(&InitWorldStates::with_world_states(
        map_id,
        represented_zone_id as i32,
        represented_area_id as i32,
        world_states,
    ));

    // 28. LoadCufProfiles — C++ sends this immediately after InitWorldStates.
    // Keeping the CUF profile application at that exact point in the login burst is
    // client-significant: the later phase refresh must not overtake it.
    if let Some(packet) = {
        let (state, hub) = host.split_lifecycle_ref();
        state.represented_load_cuf_profiles_packet_like_cpp(hub)
    } {
        host.send_packet(&packet);
    }
    // C++ `Player::SendInitialPacketsAfterAddToMap` calls
    // `SendAurasForTarget(this)` after movement aura state setup.
    {
        let (s, h) = host.split_spell_state_ref();
        s.send_initial_player_auras_like_cpp(h)
    };
    // C++ calls PhasingHandler::OnMapChange(this) only after CUF profiles, the
    // login-effect/movement-aura work and SendAurasForTarget (Player.cpp:23600-23672).
    // This re-sends SMSG_PHASE_SHIFT_CHANGE (the second phase-shift of login,
    // byte-identical to the AddToMap one). #NEXT.R8.ENTITIES.1228.
    host.send_packet(&PhaseShiftChange::default_for(guid));
    // C++ Player.cpp:23648-23650 applies destination item scaling after
    // PhasingHandler::OnMapChange, shared by login and far worldport.
    if host
        .update_represented_item_level_area_based_scaling_with_publication_like_cpp(true)
        .is_none()
    {
        return;
    }
    let _ = {
        let (state, mut hub) = host.split_lifecycle_mut();
        state.advance_worldport_post_add_like_cpp(
            &mut hub,
            wow_entities::PlayerWorldportPostAddPhaseLikeCpp::ScalingApplied,
        )
    };
    // C++ RestMgr only dirties PLAYER_FLAGS_RESTING during UpdateZone; the
    // map object-update owner flushes that field after post-add packets.
    // Keep the marker on Player across the world-state await; only channel acceptance
    // retires it. This is not a client acknowledgement or a restart durability claim.
    if host
        .hub_ref()
        .player_rest_state_snapshot_like_cpp()
        .is_some_and(|rest| rest.deferred_flag_update_dirty_like_cpp())
        && ({
            let (state, hub) = host.split_inventory_ref();
            state.send_represented_resting_player_flag_update_like_cpp(hub)
        })
    {
        host.take_deferred_rest_flag_update_dirty_like_cpp();
    }
    if updateobject_trace_enabled {
        info!(guid = ?guid, "RUST_LOGIN after_initial_packets_after_add");
    }
}

pub async fn load_initial_world_states_for_login_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &H,
    map_id: i32,
    player_area_id: u32,
) -> Vec<(i32, i32)> {
    let Some(port) = host.lifecycle().player_lifecycle_port_like_cpp() else {
        warn!("InitWorldStates: missing Player lifecycle persistence port");
        return Vec::new();
    };
    let loaded = port.load_initial_world_states_like_cpp().await;

    let area_store = host.hub_ref().catalogs.area_table_store().map(Arc::as_ref);
    let map_store = host.hub_ref().catalogs.map_store().map(Arc::as_ref);

    let mut templates = Vec::new();
    match loaded.templates {
        PlayerInitialWorldStateRowsLikeCpp::Loaded(rows) => {
            for row in rows {
                let map_ids_csv = row.map_ids_csv;
                let area_ids_csv = row.area_ids_csv;
                let map_ids = parse_login_world_state_map_ids_like_cpp(&map_ids_csv, |map_id| {
                    u32::try_from(map_id).ok().is_some_and(|map_id| {
                        map_store.is_some_and(|store| store.get(map_id).is_some())
                    })
                });
                if !map_ids_csv.is_empty() && map_ids.is_empty() {
                    continue;
                }

                let area_ids =
                    parse_login_world_state_area_ids_like_cpp(&area_ids_csv, &map_ids, area_store);
                if !area_ids_csv.is_empty() && !map_ids.is_empty() && area_ids.is_empty() {
                    continue;
                }

                templates.push(LoginWorldStateTemplateLikeCpp {
                    id: row.id,
                    default_value: row.default_value,
                    map_ids,
                    area_ids,
                });
            }
        }
        PlayerInitialWorldStateRowsLikeCpp::Failed { reason } => {
            warn!(
                reason,
                "InitWorldStates: failed to load C++ world_state templates"
            );
        }
    }

    let saved_values = match loaded.saved_values {
        PlayerInitialWorldStateRowsLikeCpp::Loaded(rows) => {
            rows.into_iter().map(|row| (row.id, row.value)).collect()
        }
        PlayerInitialWorldStateRowsLikeCpp::Failed { reason } => {
            warn!(
                reason,
                "InitWorldStates: failed to load C++ world_state_value overlay"
            );
            Vec::new()
        }
    };

    let mut states = build_initial_world_states_like_cpp(
        templates,
        saved_values,
        map_id,
        player_area_id,
        area_store,
    );
    // C++ World.cpp:1363-1364 / 2300-2301: the WorldStateMgr seeds realm-wide PvP-season
    // world states that FillInitialWorldStates always includes. CONFIG_ARENA_SEASON_ID
    // defaults to 32, CONFIG_ARENA_SEASON_IN_PROGRESS to false. #NEXT.R8.ENTITIES.1232.
    apply_pvp_season_world_states_like_cpp(
        &mut states,
        wow_config::get_value_default::<i32>("Arena.ArenaSeason.ID", 32),
        wow_config::get_value_default::<i32>("Arena.ArenaSeason.InProgress", 0) != 0,
    );
    info!(
        map_id,
        player_area_id,
        count = states.len(),
        "InitWorldStates loaded like C++"
    );
    states
}

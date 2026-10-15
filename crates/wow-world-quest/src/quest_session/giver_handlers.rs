// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest-giver packet entry points moved from the World
//! `handlers/quest/handlers.rs` and `handlers/quest/handlers/queries.rs`
//! (#1363 slice 6c): `HandleAdventureMapStartQuest`,
//! `HandleQuestgiverStatusQueryOpcode`, `HandleQuestgiverHelloOpcode`,
//! `HandleQuestgiverQueryQuestOpcode` and `HandleQuestPOIQuery` with the quest
//! POI store load and builder, the query-quest packet reader and the
//! represented quest-giver source checks (`ObjectAccessor::GetObjectByTypeMask`,
//! `hasQuest`/`hasInvolvedQuest` with `CanInteractWithQuestGiver`).

use super::*;

/// CMSG_ADVENTURE_MAP_START_QUEST.
///
/// C++ `HandleAdventureMapStartQuest`:
/// `QuestTemplate` lookup -> `sAdventureMapPOIStore` QuestID + PlayerCondition gate ->
/// `Player::CanTakeQuest(quest, true)` -> `AddQuestAndCheckCompletion(quest, player)`.
///
/// Rust keeps the same silent-return gates and records the accepted request until
/// Adventure Map quest starts can call the same live AddQuestAndCheckCompletion path.
pub async fn handle_adventure_map_start_quest_with_catalog_like_cpp<
    H: QuestHandlerSessionHostLikeCpp,
>(
    host: &mut H,
    adventure_map_poi_store: &wow_data::AdventureMapPoiStore,
    mut pkt: wow_packet::WorldPacket,
) {
    let request = match AdventureMapStartQuest::read(&mut pkt) {
        Ok(request) => request,
        Err(error) => {
            warn!("AdventureMapStartQuest: bad packet: {error}");
            return;
        }
    };
    let Ok(quest_id) = u32::try_from(request.quest_id) else {
        return;
    };

    let Some(quest_store) = host.hub_ref().catalogs.quests.store.clone() else {
        return;
    };
    let Some(quest) = quest_store.get(quest_id) else {
        return;
    };
    let Some(poi) = adventure_map_poi_store.find_start_quest_poi_like_cpp(quest_id, |id| {
        wow_world_player_conditions::meets_player_condition_id_like_cpp(
            &host.player_condition_projection_cx_like_cpp(),
            host.hub_ref().catalogs.player_condition_store.as_deref(),
            id,
        )
    }) else {
        return;
    };

    if !crate::can_take_quest(host, quest) {
        return;
    }

    host.record_represented_adventure_map_start_quest_like_cpp(
        RepresentedAdventureMapStartQuestLikeCpp {
            quest_id,
            adventure_map_poi_id: poi.id,
            player_condition_id: poi.player_condition_id,
        },
    );
}

/// CMSG_QUEST_GIVER_STATUS_QUERY — returns the quest status icon for an NPC.
/// C++ QuestHandler.cpp: HandleQuestgiverStatusQueryOpcode -> Player::GetQuestDialogStatus.
pub async fn handle_quest_giver_status_query_with_catalog_like_cpp<
    H: QuestHandlerSessionHostLikeCpp,
>(
    host: &mut H,
    quest_info: &wow_data::progression_rewards::QuestInfoStore,
    mut pkt: wow_packet::WorldPacket,
) {
    let guid = match pkt.read_packed_guid() {
        Ok(g) => g,
        Err(_) => {
            warn!("QuestGiverStatusQuery: failed to read GUID");
            return;
        }
    };

    let Some(source) = represented_quest_giver_status_query_source_like_cpp(host, guid) else {
        debug!(
            account = host.core().account_id,
            ?guid,
            "QuestGiverStatusQuery: represented ObjectAccessor mask UNIT|GAMEOBJECT miss"
        );
        return;
    };
    let status = crate::get_represented_quest_giver_status_with_catalog_like_cpp(
        host,
        Some(quest_info),
        source,
    );

    debug!(
        account = host.core().account_id,
        ?guid,
        source_entry = source.entry(),
        source_kind = source.kind_name(),
        status = status,
        "QuestGiverStatus represented source resolved"
    );

    host.send_packet(&QuestGiverStatus { guid, status });
}

/// CMSG_QUEST_GIVER_HELLO — player right-clicks a quest NPC.
/// Opens the represented quest list dialog for an interactable questgiver Creature.
/// C++ refs:
/// - `WorldSession::HandleQuestgiverHelloOpcode`, `QuestHandler.cpp:76-103`.
/// - `Player::PrepareQuestMenu`, `Player.cpp:13947-14004`.
/// Remaining represented gaps: fake-death aura removal, `AI()->OnGossipHello`,
/// full PlayerTalkClass ownership.
pub async fn handle_quest_giver_hello<H: QuestHandlerSessionHostLikeCpp>(
    host: &mut H,
    mut pkt: wow_packet::WorldPacket,
) {
    let guid = match pkt.read_packed_guid() {
        Ok(g) => g,
        Err(_) => {
            warn!("QuestGiverHello: failed to read GUID");
            return;
        }
    };

    let Some(access) = host.hub_ref().represented_npc_can_interact_with_like_cpp(
        guid,
        NPCFlags1::QUEST_GIVER.bits(),
        0,
    ) else {
        debug!(
            account = host.core().account_id,
            ?guid,
            "QuestGiverHello: NPC not found or not interactable as questgiver"
        );
        return;
    };

    {
        let (s, mut h) = host.split_world_entities_mut();
        s.pause_interacted_creature_movement_like_cpp(&mut h, guid)
    };

    if (access.npc_flags & NPCFlags1::GOSSIP.bits()) != 0
        && let Some(msg) = host
            .build_gossip_menu(access.entry, access.npc_flags, guid)
            .await
    {
        debug!(
            account = host.core().account_id,
            creature_entry = access.entry,
            "QuestGiverHello sent catalog-backed prepared gossip menu like C++"
        );
        host.send_packet(&msg);
        return;
    }

    if host.send_represented_creature_trainer_gossip_menu_like_cpp(
        guid,
        access.entry,
        access.npc_flags,
    ) {
        debug!(
            account = host.core().account_id,
            creature_entry = access.entry,
            "QuestGiverHello sent trainer fallback prepared gossip menu like C++"
        );
        return;
    }

    if crate::use_represented_creature_questgiver_like_cpp(host, guid, access.entry) {
        debug!(
            account = host.core().account_id,
            creature_entry = access.entry,
            "QuestGiverHello represented Creature questgiver seam consumed"
        );
    }
}

/// CMSG_QUEST_GIVER_QUERY_QUEST — player clicks a quest name in the list.
/// Shows full quest details (objectives, rewards) before acceptkng.
/// Legacy non-canonical note: QuestHandler.HandleQuestGiverQueryQuest
pub async fn handle_quest_giver_query_quest<H: QuestHandlerSessionHostLikeCpp>(
    host: &mut H,
    mut pkt: wow_packet::WorldPacket,
) {
    let (guid, quest_id, respond_to_giver) = match read_quest_giver_query_quest_like_cpp(&mut pkt) {
        Ok(packet) => packet,
        Err(_) => {
            warn!("QuestGiverQueryQuest: failed to read packet");
            return;
        }
    };

    info!(
        account = host.core().account_id,
        ?guid,
        quest_id,
        respond_to_giver,
        "Received QuestGiverQueryQuest like C++"
    );
    if !crate::send_represented_quest_giver_query_quest_like_cpp(host, guid, quest_id) {
        warn!(
            account = host.core().account_id,
            ?guid,
            quest_id,
            "QuestGiverQueryQuest produced no represented response"
        );
    }
}

/// CMSG_QUEST_GIVER_CLOSE_QUEST — acknowledged client close for auto-accept quest flow.
/// C++ ref: `WorldSession::HandleQuestgiverCloseQuest`, `QuestHandler.cpp:591-601`.
/// Represented seam only: records local `ScriptMgr::OnQuestAcknowledgeAutoAccept` evidence.

/// CMSG_QUEST_POI_QUERY — client asks for tracker POI blobs.
///
/// C++ refs:
/// - `QuestPOIQuery::Read`, QueryPackets.cpp:418-423.
/// - `WorldSession::HandleQuestPOIQuery`, QueryHandler.cpp:280-298.
/// - `ObjectMgr::LoadQuestPOI`, ObjectMgr.cpp:8337-8415.
pub async fn handle_quest_poi_query<H: QuestHandlerSessionHostLikeCpp>(
    host: &mut H,
    query: QuestPoiQuery,
) {
    if query.missing_quest_count > i32::from(MAX_QUEST_LOG_SIZE_LIKE_CPP) {
        return;
    }

    let requested_count = query.missing_quest_count.max(0) as usize;
    let requested_count = requested_count.min(query.missing_quest_pois.len());
    let mut quest_ids = std::collections::HashSet::new();
    for quest_id in query.missing_quest_pois.iter().take(requested_count) {
        quest_ids.insert(*quest_id);
    }

    let poi_store = quest_poi_store_like_cpp(host).await;
    let mut quest_poi_data_stats = Vec::new();
    for quest_id in quest_ids {
        if quest_id <= 0 {
            continue;
        }

        let quest_id_u32 = quest_id as u32;
        let owner = host.core().quest_objective_access_like_cpp();
        if crate::find_quest_slot_like_cpp(
            &owner,
            host.quest_state(),
            quest_id_u32,
            host.world_test_consumer_like_cpp(),
        )
        .is_none()
        {
            continue;
        }

        if let Some(poi_data) = poi_store.get(&quest_id) {
            quest_poi_data_stats.push(poi_data.clone());
        }
    }

    host.core().send_packet_realm(&QuestPoiQueryResponse {
        quest_poi_data_stats,
    });
}

pub async fn quest_poi_store_like_cpp<H: QuestHandlerSessionHostLikeCpp>(
    host: &mut H,
) -> Arc<HashMap<i32, QuestPoiData>> {
    if let Some(store) = &host.hub_ref().catalogs.quest_poi_store_like_cpp {
        return Arc::clone(store);
    }

    let Some(port) = host
        .lifecycle()
        .quest_poi_persistence_port_like_cpp()
        .cloned()
    else {
        warn!("QuestPOIQuery: quest POI persistence port unavailable; sending empty C++ response");
        let store = Arc::new(HashMap::new());
        host.catalogs_mut().quest_poi_store_like_cpp = Some(Arc::clone(&store));
        return store;
    };

    let store = match port.load_quest_poi_rows_like_cpp().await {
        wow_persistence::QuestPoiLoadOutcomeLikeCpp::Loaded { points, blobs } => {
            Arc::new(build_quest_poi_store_like_cpp(points, blobs))
        }
        wow_persistence::QuestPoiLoadOutcomeLikeCpp::Failed { stage, reason } => {
            warn!(?stage, error = %reason, "QuestPOIQuery: failed to load quest POI store like C++");
            Arc::new(HashMap::new())
        }
    };

    host.catalogs_mut().quest_poi_store_like_cpp = Some(Arc::clone(&store));
    store
}

pub fn build_quest_poi_store_like_cpp(
    point_rows: Vec<wow_persistence::QuestPoiPointLoadRowLikeCpp>,
    poi_rows: Vec<wow_persistence::QuestPoiBlobLoadRowLikeCpp>,
) -> HashMap<i32, QuestPoiData> {
    let mut all_points: HashMap<(i32, i32), Vec<QuestPoiBlobPoint>> = HashMap::new();
    for row in point_rows {
        all_points
            .entry((row.quest_id, row.idx1))
            .or_default()
            .push(QuestPoiBlobPoint {
                x: row.x,
                y: row.y,
                z: row.z,
            });
    }

    let mut store: HashMap<i32, QuestPoiData> = HashMap::new();
    for row in poi_rows {
        let Some(points) = all_points.get(&(row.quest_id, row.idx1)).cloned() else {
            debug!(
                quest_id = row.quest_id,
                blob_index = row.blob_index,
                "quest_poi references unknown quest points like C++; skipping blob"
            );
            continue;
        };

        store
            .entry(row.quest_id)
            .or_insert_with(|| QuestPoiData {
                quest_id: row.quest_id,
                blobs: Vec::new(),
            })
            .blobs
            .push(QuestPoiBlobData {
                blob_index: row.blob_index,
                objective_index: row.objective_index,
                quest_objective_id: row.quest_objective_id,
                quest_object_id: row.quest_object_id,
                map_id: row.map_id,
                ui_map_id: row.ui_map_id,
                priority: row.priority,
                flags: row.flags,
                world_effect_id: row.world_effect_id,
                player_condition_id: row.player_condition_id,
                navigation_player_condition_id: row.navigation_player_condition_id,
                spawn_tracking_id: row.spawn_tracking_id,
                points,
                always_allow_merging_blobs: row.always_allow_merging_blobs,
            });
    }

    store
}

pub fn read_quest_giver_query_quest_like_cpp(
    pkt: &mut wow_packet::WorldPacket,
) -> Result<(ObjectGuid, u32, bool), wow_packet::PacketError> {
    let guid = pkt.read_packed_guid()?;
    let quest_id = pkt.read_uint32().unwrap_or(0);
    let respond_to_giver = pkt.read_bit().unwrap_or(false);
    Ok((guid, quest_id, respond_to_giver))
}

/// Resolves CMSG_QUEST_GIVER_STATUS_QUERY through the represented equivalent of
/// C++ `ObjectAccessor::GetObjectByTypeMask(*_player, guid, TYPEMASK_UNIT | TYPEMASK_GAMEOBJECT)`.
/// Missing canonical objects and unsupported Player/Item/other GUID types fail closed with no packet.
pub fn represented_quest_giver_status_query_source_like_cpp<H: QuestHandlerSessionHostLikeCpp>(
    host: &H,
    guid: wow_core::ObjectGuid,
) -> Option<RepresentedQuestGiverStatusSourceLikeCpp> {
    if guid.is_any_type_creature() {
        // C++ TYPEID_UNIT branch also checks Creature::IsHostileTo before computing
        // dialog status. Exact faction/hostility is not represented here yet; a
        // resolved canonical Creature is treated as non-hostile only for this
        // bounded represented status calculation.
        let access = host
            .world_entities()
            .canonical_creature_access_like_cpp(host.hub_ref(), guid)?;
        return Some(RepresentedQuestGiverStatusSourceLikeCpp::Creature {
            entry: access.entry,
        });
    }

    if guid.is_game_object() {
        let access = host
            .hub_ref()
            .core
            .canonical_gameobject_access_like_cpp(guid)?;
        return Some(RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
            entry: access.entry,
        });
    }

    None
}

pub fn represented_quest_giver_involved_source_allows_quest_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    source_guid: ObjectGuid,
    quest_id: u32,
    quest_store: &wow_data::quest::QuestStore,
) -> bool {
    if source_guid.is_any_type_creature() {
        let Some(access) = host.hub_ref().represented_npc_can_interact_with_like_cpp(
            source_guid,
            NPCFlags1::QUEST_GIVER.bits(),
            0,
        ) else {
            debug!(
                account = host.hub_ref().core.account_id,
                ?source_guid,
                quest_id,
                "QuestGiverCompleteReward: represented Creature source missing or not interactable"
            );
            return false;
        };

        return quest_store.creature_has_ender_relation_like_cpp(access.entry, quest_id);
    }

    if source_guid.is_game_object() {
        let Some(access) =
            host.represented_gameobject_questgiver_can_interact_with_like_cpp(source_guid)
        else {
            debug!(
                account = host.hub_ref().core.account_id,
                ?source_guid,
                quest_id,
                "QuestGiverCompleteReward: represented GameObject source missing or not interactable"
            );
            return false;
        };

        return quest_store.gameobject_has_ender_relation_like_cpp(access.entry, quest_id);
    }

    // Player and Item questgiver branches are not involved-quest sources in this represented slice.
    // Match the C++ early-return shape by failing closed with no packet and no mutation.
    debug!(
        account = host.hub_ref().core.account_id,
        ?source_guid,
        quest_id,
        "QuestGiverCompleteReward: unsupported represented source type"
    );
    false
}

pub fn represented_quest_giver_accept_source_allows_quest_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    source_guid: ObjectGuid,
    quest_id: u32,
    quest_store: &wow_data::quest::QuestStore,
) -> bool {
    if source_guid.is_any_type_creature() {
        let Some(access) = host.hub_ref().represented_npc_can_interact_with_like_cpp(
            source_guid,
            NPCFlags1::QUEST_GIVER.bits(),
            0,
        ) else {
            debug!(
                account = host.hub_ref().core.account_id,
                ?source_guid,
                quest_id,
                "QuestGiverAcceptQuest: represented Creature source missing or not interactable"
            );
            return false;
        };

        return quest_store.creature_has_starter_relation_like_cpp(access.entry, quest_id);
    }

    if source_guid.is_game_object() {
        let Some(access) =
            host.represented_gameobject_questgiver_can_interact_with_like_cpp(source_guid)
        else {
            debug!(
                account = host.hub_ref().core.account_id,
                ?source_guid,
                quest_id,
                "QuestGiverAcceptQuest: represented GameObject source missing or not interactable"
            );
            return false;
        };

        return quest_store.gameobject_has_starter_relation_like_cpp(access.entry, quest_id);
    }

    // Player quest sharing and Item questgiver branches are not represented in this slice.
    // Match the C++ early-return shape by failing closed with no packet and no mutation.
    debug!(
        account = host.hub_ref().core.account_id,
        ?source_guid,
        quest_id,
        "QuestGiverAcceptQuest: unsupported represented source type"
    );
    false
}

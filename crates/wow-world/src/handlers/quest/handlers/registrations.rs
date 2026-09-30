//! Canonical Quest opcode registrations and thunks.
use super::*;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AdventureMapStartQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_adventure_map_start_quest",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_adventure_map_start_quest_with_catalog_like_cpp(
                        catalogs.adventure_map_pois.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_status_query",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_status_query_with_catalog_like_cpp(catalogs.quest_info.as_ref(), pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverHello,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_hello",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_hello(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverQueryQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_query_quest",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_query_quest(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverAcceptQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_accept_quest",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_quest_giver_accept_quest_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestLogRemoveQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_log_remove_quest",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_log_remove_quest(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_quest_info",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_query_quest_info(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestCompletionNpcs,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_quest_completion_npcs",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryQuestCompletionNpcs::read(&mut pkt) {
                    Ok(query) => session.handle_query_quest_completion_npcs(query).await,
                    Err(e) => tracing::warn!("Failed to read QueryQuestCompletionNpcs: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestPoiQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_poi_query",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QuestPoiQuery::read(&mut pkt) {
                    Ok(query) => session.handle_quest_poi_query(query).await,
                    Err(e) => tracing::warn!("Failed to read QuestPoiQuery: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverRequestReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_request_reward",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_quest_giver_request_reward_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverCompleteQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_complete_quest",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_complete_quest(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverChooseReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_choose_reward",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_quest_giver_choose_reward_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverCloseQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_close_quest",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_close_quest(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestWorldQuestUpdate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_world_quest_update",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_request_world_quest_update(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestConfirmAccept,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_confirm_accept",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_quest_confirm_accept_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestPushResult,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_push_result",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_push_result(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PushQuestToParty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_push_quest_to_party",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_push_quest_to_party(pkt).await })
        },
    }
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest packet entry points and their handler registrations.

use super::*;
use wow_packet::ClientPacket;

mod acceptance;
mod queries;
mod reward_flow;
mod sharing;

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverCompleteQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_complete_quest",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move {
                wow_world_application::handle_quest_giver_complete_quest_like_cpp(session, pkt)
                    .await
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
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

// ── Handler implementations ──────────────────────────────────────────────────

/// TrinityCore `MAX_QUEST_LOG_SIZE`; explicit quest-log slots are 0..24.

#[cfg(test)]
mod test_shims;

impl WorldSession {
    /// CMSG_ADVENTURE_MAP_START_QUEST.
    ///
    /// C++ `HandleAdventureMapStartQuest`:
    /// `QuestTemplate` lookup -> `sAdventureMapPOIStore` QuestID + PlayerCondition gate ->
    /// `Player::CanTakeQuest(quest, true)` -> `AddQuestAndCheckCompletion(quest, player)`.
    ///
    /// Rust keeps the same silent-return gates and records the accepted request until
    /// Adventure Map quest starts can call the same live AddQuestAndCheckCompletion path.
    pub(crate) async fn handle_adventure_map_start_quest_with_catalog_like_cpp(
        &mut self,
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

        let Some(quest_store) = self.catalogs.quests.store.clone() else {
            return;
        };
        let Some(quest) = quest_store.get(quest_id) else {
            return;
        };
        let Some(poi) = adventure_map_poi_store.find_start_quest_poi_like_cpp(quest_id, |id| {
            self.represented_meets_player_condition_id_like_cpp(id)
        }) else {
            return;
        };

        if !self.can_take_quest(quest) {
            return;
        }

        self.record_represented_adventure_map_start_quest_like_cpp(
            RepresentedAdventureMapStartQuestLikeCpp {
                quest_id,
                adventure_map_poi_id: poi.id,
                player_condition_id: poi.player_condition_id,
            },
        );
    }

    #[cfg(test)]
    pub async fn handle_adventure_map_start_quest(&mut self, pkt: wow_packet::WorldPacket) {
        let store = self.adventure_map_poi_store().cloned().unwrap_or_else(|| {
            std::sync::Arc::new(wow_data::AdventureMapPoiStore::from_entries([]))
        });
        self.handle_adventure_map_start_quest_with_catalog_like_cpp(store.as_ref(), pkt)
            .await;
    }
}

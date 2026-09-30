//! Quest poi operation at the application boundary.
use super::*;

impl WorldSession {

    pub(in crate::handlers::quest) async fn quest_poi_store_like_cpp(&mut self) -> Arc<HashMap<i32, QuestPoiData>> {
        if let Some(store) = &self.quest_poi_store_like_cpp {
            return Arc::clone(store);
        }

        let Some(port) = self.quest_poi_persistence_port_like_cpp() else {
            warn!(
                "QuestPOIQuery: quest POI persistence port unavailable; sending empty C++ response"
            );
            let store = Arc::new(HashMap::new());
            self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
            return store;
        };

        let store = match port.load_quest_poi_rows_like_cpp().await {
            wow_persistence::QuestPoiLoadOutcomeLikeCpp::Loaded { points, blobs } => {
                Arc::new(wow_data::build_quest_poi_store(
                    points.into_iter().map(|row| {
                        (row.quest_id, row.idx1, QuestPoiBlobPoint {
                            x: row.x,
                            y: row.y,
                            z: row.z,
                        })
                    }),
                    blobs.into_iter().map(|row| {
                        (row.quest_id, row.idx1, QuestPoiBlobData {
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
                            points: Vec::new(),
                            always_allow_merging_blobs: row.always_allow_merging_blobs,
                        })
                    }),
                ))
            }
            wow_persistence::QuestPoiLoadOutcomeLikeCpp::Failed { stage, reason } => {
                warn!(?stage, error = %reason, "QuestPOIQuery: failed to load quest POI store like C++");
                Arc::new(HashMap::new())
            }
        };

        self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
        store
    }
}

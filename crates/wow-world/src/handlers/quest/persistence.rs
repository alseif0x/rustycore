// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest status persistence and load.

use super::*;

impl WorldSession {
    pub(super) async fn quest_poi_store_like_cpp(&mut self) -> Arc<HashMap<i32, QuestPoiData>> {
        if let Some(store) = &self.catalogs.quest_poi_store_like_cpp {
            return Arc::clone(store);
        }

        let Some(port) =
            crate::session::cx_quest_state_ref(self).quest_poi_persistence_port_like_cpp()
        else {
            warn!(
                "QuestPOIQuery: quest POI persistence port unavailable; sending empty C++ response"
            );
            let store = Arc::new(HashMap::new());
            self.catalogs.quest_poi_store_like_cpp = Some(Arc::clone(&store));
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

        self.catalogs.quest_poi_store_like_cpp = Some(Arc::clone(&store));
        store
    }
}

impl crate::session::QuestStateCxRef<'_> {}

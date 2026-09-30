//! Quest POI point grouping and catalog assembly.
//!
//! C++ a5f8da2e: ObjectMgr.cpp:8337–8415. SQL row decoding and async
//! loading remain with the persistence adapter and its World caller.

use std::collections::HashMap;

use tracing::debug;
use wow_data_model::quest_poi::{QuestPoiBlobData, QuestPoiBlobPoint, QuestPoiData};

/// Assemble POIs in input order, retaining duplicate points and blobs.
///
/// Blob inputs contain metadata with empty points; matching point groups
/// supply the points. Each blob receives its own clone of the group, retaining
/// the existing Rust behavior rather than C++'s `std::move(*points)`.
pub fn build_quest_poi_store(
    point_rows: impl IntoIterator<Item = (i32, i32, QuestPoiBlobPoint)>,
    poi_rows: impl IntoIterator<Item = (i32, i32, QuestPoiBlobData)>,
) -> HashMap<i32, QuestPoiData> {
    let mut all_points: HashMap<(i32, i32), Vec<QuestPoiBlobPoint>> = HashMap::new();
    for (quest_id, idx1, point) in point_rows {
        all_points.entry((quest_id, idx1)).or_default().push(point);
    }

    let mut store: HashMap<i32, QuestPoiData> = HashMap::new();
    for (quest_id, idx1, mut blob) in poi_rows {
        let Some(points) = all_points.get(&(quest_id, idx1)).cloned() else {
            debug!(
                target: "wow_world::handlers::quest",
                quest_id = quest_id,
                blob_index = blob.blob_index,
                "quest_poi references unknown quest points like C++; skipping blob"
            );
            continue;
        };

        blob.points = points;
        store
            .entry(quest_id)
            .or_insert_with(|| QuestPoiData {
                quest_id,
                blobs: Vec::new(),
            })
            .blobs
            .push(blob);
    }

    store
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob_row(quest_id: i32, idx1: i32) -> (i32, i32, QuestPoiBlobData) {
        (
            quest_id,
            idx1,
            QuestPoiBlobData {
                blob_index: 1,
                objective_index: -1,
                quest_objective_id: 2,
                quest_object_id: 3,
                map_id: 571,
                ui_map_id: 486,
                priority: 4,
                flags: 5,
                world_effect_id: 6,
                player_condition_id: 7,
                navigation_player_condition_id: 8,
                spawn_tracking_id: 9,
                points: Vec::new(),
                always_allow_merging_blobs: false,
            },
        )
    }

    #[test]
    fn quest_poi_typed_rows_join_points_and_skip_unknown_groups_like_cpp() {
        let store = build_quest_poi_store(
            vec![(
                77,
                3,
                QuestPoiBlobPoint {
                    x: 10,
                    y: 11,
                    z: 12,
                },
            )],
            vec![blob_row(77, 3), blob_row(88, 9)],
        );

        assert_eq!(store.len(), 1);
        assert_eq!(store[&77].blobs[0].points[0].x, 10);
        assert!(!store.contains_key(&88));
    }

    #[test]
    fn quest_poi_preserves_negative_ids_metadata_and_raw_coordinates() {
        let point = QuestPoiBlobPoint {
            x: i32::MIN,
            y: i32::MAX,
            z: 65_536,
        };
        let blob = QuestPoiBlobData {
            blob_index: -1,
            objective_index: -2,
            quest_objective_id: -3,
            quest_object_id: -4,
            map_id: -5,
            ui_map_id: -6,
            priority: -7,
            flags: -8,
            world_effect_id: -9,
            player_condition_id: -10,
            navigation_player_condition_id: -11,
            spawn_tracking_id: -12,
            points: Vec::new(),
            always_allow_merging_blobs: true,
        };
        let mut expected = blob.clone();
        expected.points.push(point.clone());

        let store = build_quest_poi_store([(-77, -3, point)], [(-77, -3, blob)]);

        assert_eq!(store.len(), 1);
        assert_eq!(store[&-77].quest_id, -77);
        assert_eq!(store[&-77].blobs, vec![expected]);
    }

    #[test]
    fn quest_poi_preserves_duplicate_points_and_blob_order_for_shared_group() {
        let first = QuestPoiBlobPoint {
            x: 10,
            y: -11,
            z: 12,
        };
        let last = QuestPoiBlobPoint {
            x: 20,
            y: -21,
            z: 22,
        };
        let expected_points = vec![first.clone(), first.clone(), last.clone()];
        let mut first_blob = blob_row(77, 3);
        first_blob.2.blob_index = 9;
        let mut middle_blob = blob_row(77, 3);
        middle_blob.2.blob_index = 3;
        let duplicate_blob = first_blob.clone();

        let store = build_quest_poi_store(
            [(77, 3, first.clone()), (77, 3, first), (77, 3, last)],
            [first_blob, middle_blob, duplicate_blob],
        );
        let blobs = &store[&77].blobs;

        assert_eq!(
            blobs.iter().map(|blob| blob.blob_index).collect::<Vec<_>>(),
            vec![9, 3, 9]
        );
        for blob in blobs {
            assert_eq!(blob.points, expected_points);
        }
        assert_eq!(blobs[0], blobs[2]);
    }

    #[test]
    fn quest_poi_skips_missing_groups_without_creating_quest_entries() {
        let store = build_quest_poi_store(
            [(
                77,
                3,
                QuestPoiBlobPoint {
                    x: 10,
                    y: 11,
                    z: 12,
                },
            )],
            [
                blob_row(77, 9),
                blob_row(77, 9),
                blob_row(88, 3),
                blob_row(77, 3),
            ],
        );

        assert_eq!(store.len(), 1);
        assert_eq!(store[&77].blobs.len(), 1);
        assert!(!store.contains_key(&88));
    }
}

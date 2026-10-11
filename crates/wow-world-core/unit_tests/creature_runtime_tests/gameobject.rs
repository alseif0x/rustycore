//! Gameobject scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn persisted_respawn_restart_load_expired_ready_future_queued_and_gameobject_timer_loaded() {
    let mut manager = MapManager::new();
    let now = Instant::now();
    let now_secs = 2_000_000;
    let rows = vec![
        PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 10,
            respawn_time: now_secs - 5,
            map_id: 571,
            instance_id: 0,
        },
        PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 11,
            respawn_time: now_secs + 60,
            map_id: 571,
            instance_id: 0,
        },
        PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id: 12,
            respawn_time: now_secs + 90,
            map_id: 571,
            instance_id: 0,
        },
    ];

    let report =
        manager.load_persisted_respawns_into_queue_like_cpp(rows, now, now_secs, |_row, at| {
            Some(make_pending_respawn(at))
        });

    assert_eq!(report.rows, 3);
    assert_eq!(report.timers_loaded, 3);
    assert_eq!(report.creature_queued, 2);
    assert_eq!(report.gameobject_loaded, 1);
    assert_eq!(
        manager.persisted_respawn_time_like_cpp(571, 0, SpawnObjectType::GameObject, 12),
        Some(now_secs + 90)
    );

    let ready = manager.drain_ready_respawns(571, 0, now);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].spawn_id, 10);
    assert_eq!(manager.respawn_queue_len(571, 0), 1);

    let delete = manager
        .remove_persisted_respawn_time_like_cpp(571, 0, SpawnObjectType::Creature, 10)
        .expect("processed C++ respawn should queue CHAR_DEL_RESPAWN");
    assert_eq!(
        delete,
        RespawnPersistenceMutationLikeCpp::Delete {
            key: RespawnPersistenceKeyLikeCpp {
                object_type_raw: 0,
                spawn_id: 10,
                map_id: 571,
                instance_id: 0,
            },
        }
    );
    assert_eq!(
        manager.persisted_respawn_time_like_cpp(571, 0, SpawnObjectType::Creature, 10),
        None
    );

    let future = manager.drain_ready_respawns(571, 0, now + Duration::from_secs(59));
    assert!(future.is_empty());
    assert_eq!(manager.respawn_queue_len(571, 0), 1);
}

//! Composition boundaries between SQLx-free vehicle rows and `wow-data`.

use anyhow::{Result, bail};
use tracing::info;
use wow_entities::{VehicleAccessory, VehicleTemplate};
use wow_persistence::{
    VehicleHotfixLoadOutcomeLikeCpp, VehicleHotfixPersistencePortLikeCpp,
    VehicleWorldCatalogLoadOutcomeLikeCpp, VehicleWorldCatalogPersistencePortLikeCpp,
};

fn hotfix_rows_like_cpp<T>(outcome: VehicleHotfixLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        VehicleHotfixLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        VehicleHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

fn world_rows_like_cpp<T>(outcome: VehicleWorldCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        VehicleWorldCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        VehicleWorldCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

async fn overlay_vehicle_store_like_cpp(
    mut store: wow_data::VehicleStore,
    persistence: &dyn VehicleHotfixPersistencePortLikeCpp,
) -> Result<(wow_data::VehicleStore, usize)> {
    let rows = hotfix_rows_like_cpp(persistence.load_vehicle_hotfix_rows_like_cpp().await)?;
    let count =
        store.apply_hotfix_entries_like_cpp(rows.into_iter().map(|row| wow_data::VehicleEntry {
            id: row.id,
            flags: row.flags,
            flags_b: row.flags_b,
            seat_ids: row.seat_ids,
        }));
    Ok((store, count))
}

pub(super) async fn load_vehicle_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn VehicleHotfixPersistencePortLikeCpp,
) -> Result<wow_data::VehicleStore> {
    let (store, count) = overlay_vehicle_store_like_cpp(
        wow_data::VehicleStore::load(data_dir, locale)?,
        persistence,
    )
    .await?;
    if count != 0 {
        info!("Loaded {count} Vehicle hotfix rows");
    }
    Ok(store)
}

async fn overlay_vehicle_seat_store_like_cpp(
    mut store: wow_data::VehicleSeatStore,
    persistence: &dyn VehicleHotfixPersistencePortLikeCpp,
) -> Result<(wow_data::VehicleSeatStore, usize)> {
    let rows = hotfix_rows_like_cpp(persistence.load_vehicle_seat_hotfix_rows_like_cpp().await)?;
    let count = store.apply_hotfix_entries_like_cpp(rows.into_iter().map(|row| {
        wow_data::VehicleSeatEntry {
            id: row.id,
            attachment_offset_x: row.attachment_offset_x,
            attachment_offset_y: row.attachment_offset_y,
            attachment_offset_z: row.attachment_offset_z,
            flags: row.flags,
            flags_b: row.flags_b,
            flags_c: row.flags_c,
        }
    }));
    Ok((store, count))
}

pub(super) async fn load_vehicle_seat_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn VehicleHotfixPersistencePortLikeCpp,
) -> Result<wow_data::VehicleSeatStore> {
    let (store, count) = overlay_vehicle_seat_store_like_cpp(
        wow_data::VehicleSeatStore::load(data_dir, locale)?,
        persistence,
    )
    .await?;
    if count != 0 {
        info!("Loaded {count} VehicleSeat hotfix rows");
    }
    Ok(store)
}

pub(super) async fn load_vehicle_template_store_like_cpp(
    persistence: &dyn VehicleWorldCatalogPersistencePortLikeCpp,
) -> Result<wow_data::VehicleTemplateStoreLikeCpp> {
    let rows = world_rows_like_cpp(persistence.load_vehicle_template_rows_like_cpp().await)?;
    let store = wow_data::VehicleTemplateStoreLikeCpp::from_entries(rows.into_iter().map(|row| {
        (
            row.creature_entry,
            VehicleTemplate {
                despawn_delay_ms: row.despawn_delay_ms,
            },
        )
    }));
    info!("Loaded {} Vehicle Template entries", store.len());
    Ok(store)
}

pub(super) async fn load_vehicle_accessory_store_like_cpp(
    persistence: &dyn VehicleWorldCatalogPersistencePortLikeCpp,
) -> Result<wow_data::VehicleAccessoryStoreLikeCpp> {
    let template_rows = world_rows_like_cpp(
        persistence
            .load_vehicle_template_accessory_rows_like_cpp()
            .await,
    )?;
    let spawn_rows = world_rows_like_cpp(
        persistence
            .load_vehicle_spawn_accessory_rows_like_cpp()
            .await,
    )?;
    let template_count = template_rows.len();
    let spawn_count = spawn_rows.len();

    let mut by_creature_entry = std::collections::HashMap::<u32, Vec<VehicleAccessory>>::new();
    for row in template_rows {
        by_creature_entry
            .entry(row.creature_entry)
            .or_default()
            .push(VehicleAccessory {
                accessory_entry: row.accessory_entry,
                seat_id: row.seat_id,
                is_minion: row.is_minion,
                summoned_type: row.summoned_type,
                summon_time_ms: row.summon_time_ms,
            });
    }
    let mut by_spawn_guid = std::collections::HashMap::<u64, Vec<VehicleAccessory>>::new();
    for row in spawn_rows {
        by_spawn_guid
            .entry(row.spawn_guid)
            .or_default()
            .push(VehicleAccessory {
                accessory_entry: row.accessory_entry,
                seat_id: row.seat_id,
                is_minion: row.is_minion,
                summoned_type: row.summoned_type,
                summon_time_ms: row.summon_time_ms,
            });
    }
    let store =
        wow_data::VehicleAccessoryStoreLikeCpp::from_parts(by_spawn_guid, by_creature_entry);
    info!(
        "Loaded {template_count} vehicle template accessories and {spawn_count} vehicle accessories"
    );
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::{
        PersistenceFutureLikeCpp, VehicleHotfixPersistenceRowLikeCpp,
        VehicleSeatHotfixPersistenceRowLikeCpp, VehicleSpawnAccessoryPersistenceRowLikeCpp,
        VehicleTemplateAccessoryPersistenceRowLikeCpp, VehicleTemplatePersistenceRowLikeCpp,
    };

    #[derive(Clone)]
    struct FakeHotfixPortLikeCpp {
        vehicles: VehicleHotfixLoadOutcomeLikeCpp<VehicleHotfixPersistenceRowLikeCpp>,
        seats: VehicleHotfixLoadOutcomeLikeCpp<VehicleSeatHotfixPersistenceRowLikeCpp>,
    }

    impl VehicleHotfixPersistencePortLikeCpp for FakeHotfixPortLikeCpp {
        fn load_vehicle_hotfix_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            VehicleHotfixLoadOutcomeLikeCpp<VehicleHotfixPersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.vehicles.clone() })
        }

        fn load_vehicle_seat_hotfix_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            VehicleHotfixLoadOutcomeLikeCpp<VehicleSeatHotfixPersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.seats.clone() })
        }
    }

    #[derive(Clone)]
    struct FakeWorldPortLikeCpp {
        templates: VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleTemplatePersistenceRowLikeCpp>,
        template_accessories:
            VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleTemplateAccessoryPersistenceRowLikeCpp>,
        spawn_accessories:
            VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleSpawnAccessoryPersistenceRowLikeCpp>,
    }

    impl VehicleWorldCatalogPersistencePortLikeCpp for FakeWorldPortLikeCpp {
        fn load_vehicle_template_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleTemplatePersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.templates.clone() })
        }

        fn load_vehicle_template_accessory_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleTemplateAccessoryPersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.template_accessories.clone() })
        }

        fn load_vehicle_spawn_accessory_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleSpawnAccessoryPersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.spawn_accessories.clone() })
        }
    }

    #[tokio::test]
    async fn typed_hotfix_rows_replace_db2_entries_without_reordering_seats() {
        let base = wow_data::VehicleStore::from_entries([wow_data::VehicleEntry {
            id: 7,
            flags: 1,
            flags_b: 2,
            seat_ids: [1, 2, 3, 4, 5, 6, 7, 8],
        }]);
        let port = FakeHotfixPortLikeCpp {
            vehicles: VehicleHotfixLoadOutcomeLikeCpp::Loaded(vec![
                VehicleHotfixPersistenceRowLikeCpp {
                    id: 7,
                    flags: 10,
                    flags_b: 20,
                    seat_ids: [80, 70, 60, 50, 40, 30, 20, 10],
                },
            ]),
            seats: VehicleHotfixLoadOutcomeLikeCpp::Loaded(vec![
                VehicleSeatHotfixPersistenceRowLikeCpp {
                    id: 9,
                    attachment_offset_x: 1.5,
                    attachment_offset_y: 2.5,
                    attachment_offset_z: 3.5,
                    flags: 11,
                    flags_b: 12,
                    flags_c: 13,
                },
            ]),
        };

        let (store, count) = overlay_vehicle_store_like_cpp(base, &port).await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(store.get(7).unwrap().flags, 10);
        assert_eq!(
            store.get(7).unwrap().seat_ids,
            [80, 70, 60, 50, 40, 30, 20, 10]
        );

        let (seats, seat_count) = overlay_vehicle_seat_store_like_cpp(
            wow_data::VehicleSeatStore::from_entries([]),
            &port,
        )
        .await
        .unwrap();
        assert_eq!(seat_count, 1);
        let seat = seats.get(9).unwrap();
        assert_eq!(seat.attachment_offset_x, 1.5);
        assert_eq!(seat.attachment_offset_y, 2.5);
        assert_eq!(seat.attachment_offset_z, 3.5);
        assert_eq!((seat.flags, seat.flags_b, seat.flags_c), (11, 12, 13));
    }

    #[tokio::test]
    async fn empty_and_failure_remain_distinct_before_vehicle_publication() {
        let empty = FakeHotfixPortLikeCpp {
            vehicles: VehicleHotfixLoadOutcomeLikeCpp::Loaded(Vec::new()),
            seats: VehicleHotfixLoadOutcomeLikeCpp::Loaded(Vec::new()),
        };
        let (store, count) =
            overlay_vehicle_store_like_cpp(wow_data::VehicleStore::from_entries([]), &empty)
                .await
                .unwrap();
        assert!(store.is_empty());
        assert_eq!(count, 0);

        let failed = FakeHotfixPortLikeCpp {
            vehicles: VehicleHotfixLoadOutcomeLikeCpp::Failed {
                reason: "vehicle hotfix failed".to_string(),
            },
            seats: VehicleHotfixLoadOutcomeLikeCpp::Loaded(Vec::new()),
        };
        let result =
            overlay_vehicle_store_like_cpp(wow_data::VehicleStore::from_entries([]), &failed).await;
        let Err(error) = result else {
            panic!("failed hotfix query must not publish a vehicle store");
        };
        assert_eq!(error.to_string(), "vehicle hotfix failed");
    }

    #[tokio::test]
    async fn world_rows_keep_group_order_and_spawn_specific_precedence() {
        let port = FakeWorldPortLikeCpp {
            templates: VehicleWorldCatalogLoadOutcomeLikeCpp::Loaded(vec![
                VehicleTemplatePersistenceRowLikeCpp {
                    creature_entry: 10,
                    despawn_delay_ms: -12,
                },
            ]),
            template_accessories: VehicleWorldCatalogLoadOutcomeLikeCpp::Loaded(vec![
                VehicleTemplateAccessoryPersistenceRowLikeCpp {
                    creature_entry: 10,
                    accessory_entry: 100,
                    seat_id: -1,
                    is_minion: true,
                    summoned_type: 2,
                    summon_time_ms: 300,
                },
                VehicleTemplateAccessoryPersistenceRowLikeCpp {
                    creature_entry: 10,
                    accessory_entry: 101,
                    seat_id: 1,
                    is_minion: false,
                    summoned_type: 3,
                    summon_time_ms: 400,
                },
            ]),
            spawn_accessories: VehicleWorldCatalogLoadOutcomeLikeCpp::Loaded(vec![
                VehicleSpawnAccessoryPersistenceRowLikeCpp {
                    spawn_guid: 55,
                    accessory_entry: 200,
                    seat_id: 2,
                    is_minion: false,
                    summoned_type: 4,
                    summon_time_ms: 500,
                },
            ]),
        };

        let templates = load_vehicle_template_store_like_cpp(&port).await.unwrap();
        assert_eq!(templates.despawn_delay_ms_like_cpp(10), -12);
        assert_eq!(templates.despawn_delay_ms_like_cpp(999), 1);

        let accessories = load_vehicle_accessory_store_like_cpp(&port).await.unwrap();
        let generic = accessories
            .accessories_for_vehicle_like_cpp(None, 10)
            .unwrap();
        assert_eq!(
            generic
                .iter()
                .map(|row| row.accessory_entry)
                .collect::<Vec<_>>(),
            [100, 101]
        );
        assert_eq!(generic[0].seat_id, -1);
        assert!(generic[0].is_minion);
        assert_eq!(generic[0].summoned_type, 2);
        assert_eq!(generic[0].summon_time_ms, 300);
        let specific = accessories
            .accessories_for_vehicle_like_cpp(Some(55), 10)
            .unwrap();
        assert_eq!(specific.len(), 1);
        assert_eq!(specific[0].accessory_entry, 200);
        assert_eq!(specific[0].seat_id, 2);
        assert!(!specific[0].is_minion);
        assert_eq!(specific[0].summoned_type, 4);
        assert_eq!(specific[0].summon_time_ms, 500);
    }

    #[tokio::test]
    async fn spawn_accessory_failure_stops_before_accessory_store_publication() {
        let port = FakeWorldPortLikeCpp {
            templates: VehicleWorldCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
            template_accessories: VehicleWorldCatalogLoadOutcomeLikeCpp::Loaded(vec![
                VehicleTemplateAccessoryPersistenceRowLikeCpp {
                    creature_entry: 10,
                    accessory_entry: 100,
                    seat_id: 0,
                    is_minion: false,
                    summoned_type: 0,
                    summon_time_ms: 0,
                },
            ]),
            spawn_accessories: VehicleWorldCatalogLoadOutcomeLikeCpp::Failed {
                reason: "spawn accessory query failed".to_string(),
            },
        };

        let result = load_vehicle_accessory_store_like_cpp(&port).await;
        let Err(error) = result else {
            panic!("a failed second query must not publish the partially assembled store");
        };
        assert_eq!(error.to_string(), "spawn accessory query failed");
    }
}

//! Composition boundary for C++ Hotfix client-delivery metadata.

use anyhow::{Result, bail};
use wow_persistence::{
    HotfixBlobPersistenceRowLikeCpp, HotfixDataPersistenceRowLikeCpp,
    HotfixDeliveryMetadataLoadOutcomeLikeCpp, HotfixDeliveryMetadataPersistencePortLikeCpp,
    HotfixOptionalDataPersistenceRowLikeCpp,
};

fn loaded_rows_like_cpp<T>(outcome: HotfixDeliveryMetadataLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        HotfixDeliveryMetadataLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_db2_hotfix_removals_like_cpp(
    persistence: &dyn HotfixDeliveryMetadataPersistencePortLikeCpp,
) -> Result<wow_data::Db2HotfixRemovalStoreLikeCpp> {
    let rows = loaded_rows_like_cpp(persistence.load_hotfix_data_rows_like_cpp().await)?;
    Ok(
        wow_data::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(
            rows.into_iter()
                .map(|row| (row.table_hash, row.record_id, row.status)),
        ),
    )
}

fn apply_hotfix_blob_outcome_like_cpp(
    cache: &mut wow_data::HotfixBlobCache,
    outcome: HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixBlobPersistenceRowLikeCpp>,
    locale: &str,
) -> Result<usize> {
    let rows = loaded_rows_like_cpp(outcome)?;
    Ok(cache.apply_hotfix_blob_rows_like_cpp(
        rows.into_iter()
            .map(|row| (row.table_hash, row.record_id, row.locale, row.blob)),
        locale,
    ))
}

fn apply_hotfix_data_outcome_like_cpp(
    cache: &mut wow_data::HotfixBlobCache,
    outcome: HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixDataPersistenceRowLikeCpp>,
    locale: &str,
) -> Result<usize> {
    let rows = loaded_rows_like_cpp(outcome)?;
    Ok(cache.apply_hotfix_data_rows_like_cpp(
        rows.into_iter().map(|row| {
            (
                row.push_id,
                row.unique_id,
                row.table_hash,
                row.record_id,
                row.status,
            )
        }),
        locale,
    ))
}

fn apply_hotfix_optional_data_outcome_like_cpp(
    cache: &mut wow_data::HotfixBlobCache,
    outcome: HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixOptionalDataPersistenceRowLikeCpp>,
    locale: &str,
) -> Result<usize> {
    let rows = loaded_rows_like_cpp(outcome)?;
    Ok(cache.apply_hotfix_optional_data_rows_like_cpp(
        rows.into_iter()
            .map(|row| (row.table_hash, row.record_id, row.locale, row.key, row.data)),
        locale,
    ))
}

pub(super) async fn load_hotfix_delivery_metadata_like_cpp(
    cache: &mut wow_data::HotfixBlobCache,
    persistence: &dyn HotfixDeliveryMetadataPersistencePortLikeCpp,
    locale: &str,
) -> [Result<usize>; 3] {
    // C++ `World::SetInitialWorldSettings` keeps these three stages ordered.
    // Each result remains independent so startup preserves Rust's existing
    // warn-and-continue behavior instead of turning this into one eager bundle.
    let blobs = apply_hotfix_blob_outcome_like_cpp(
        cache,
        persistence.load_hotfix_blob_rows_like_cpp().await,
        locale,
    );
    let data = apply_hotfix_data_outcome_like_cpp(
        cache,
        persistence.load_hotfix_data_rows_like_cpp().await,
        locale,
    );
    let optional_data = apply_hotfix_optional_data_outcome_like_cpp(
        cache,
        persistence.load_hotfix_optional_data_rows_like_cpp().await,
        locale,
    );
    [blobs, data, optional_data]
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct RecordingPort {
        calls: Mutex<Vec<&'static str>>,
    }

    impl HotfixDeliveryMetadataPersistencePortLikeCpp for RecordingPort {
        fn load_hotfix_blob_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixBlobPersistenceRowLikeCpp>,
        > {
            self.calls.lock().unwrap().push("blob");
            Box::pin(async {
                HotfixDeliveryMetadataLoadOutcomeLikeCpp::Failed {
                    reason: "blob read failed".into(),
                }
            })
        }

        fn load_hotfix_data_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixDataPersistenceRowLikeCpp>,
        > {
            self.calls.lock().unwrap().push("data");
            Box::pin(async { HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(Vec::new()) })
        }

        fn load_hotfix_optional_data_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixOptionalDataPersistenceRowLikeCpp>,
        > {
            self.calls.lock().unwrap().push("optional");
            Box::pin(async {
                HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(vec![
                    HotfixOptionalDataPersistenceRowLikeCpp {
                        table_hash: 1,
                        record_id: 2,
                        locale: "enUS".into(),
                        key: 3,
                        data: vec![4],
                    },
                ])
            })
        }
    }

    #[tokio::test]
    async fn stages_remain_ordered_and_fail_independently_before_mutation() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
        };
        let mut cache = wow_data::HotfixBlobCache::new();
        let [blobs, data, optional] =
            load_hotfix_delivery_metadata_like_cpp(&mut cache, &port, "enUS").await;

        assert_eq!(*port.calls.lock().unwrap(), ["blob", "data", "optional"]);
        assert_eq!(blobs.unwrap_err().to_string(), "blob read failed");
        assert_eq!(data.unwrap(), 0);
        assert_eq!(optional.unwrap(), 1);
        assert_eq!(cache.total_hotfix_blobs(), 0);
        assert_eq!(cache.get_optional_data(1, 2, "enUS").unwrap()[0].data, [4]);
    }

    struct RemovalPort {
        fail_data: bool,
    }

    impl HotfixDeliveryMetadataPersistencePortLikeCpp for RemovalPort {
        fn load_hotfix_blob_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixBlobPersistenceRowLikeCpp>,
        > {
            panic!("the early removal stage must not query hotfix_blob")
        }

        fn load_hotfix_data_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixDataPersistenceRowLikeCpp>,
        > {
            if self.fail_data {
                return Box::pin(async {
                    HotfixDeliveryMetadataLoadOutcomeLikeCpp::Failed {
                        reason: "hotfix_data unavailable".to_owned(),
                    }
                });
            }
            Box::pin(async {
                HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(vec![
                    HotfixDataPersistenceRowLikeCpp {
                        push_id: 1,
                        unique_id: 10,
                        table_hash: 0xAAAA,
                        record_id: 7,
                        status: 2,
                    },
                    HotfixDataPersistenceRowLikeCpp {
                        push_id: 2,
                        unique_id: 11,
                        table_hash: 0xAAAA,
                        record_id: 7,
                        status: 1,
                    },
                    HotfixDataPersistenceRowLikeCpp {
                        push_id: 3,
                        unique_id: 12,
                        table_hash: 0xBBBB,
                        record_id: -8,
                        status: 2,
                    },
                ])
            })
        }

        fn load_hotfix_optional_data_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixOptionalDataPersistenceRowLikeCpp>,
        > {
            panic!("the early removal stage must not query hotfix_optional_data")
        }
    }

    #[tokio::test]
    async fn early_removal_stage_uses_only_typed_data_and_keeps_last_status() {
        let removals = load_db2_hotfix_removals_like_cpp(&RemovalPort { fail_data: false })
            .await
            .unwrap();

        assert!(!removals.contains_like_cpp(0xAAAA, 7));
        assert!(removals.contains_like_cpp(0xBBBB, -8));
    }

    #[tokio::test]
    async fn early_removal_failure_publishes_no_store() {
        let result = load_db2_hotfix_removals_like_cpp(&RemovalPort { fail_data: true }).await;

        assert_eq!(result.unwrap_err().to_string(), "hotfix_data unavailable");
    }

    #[test]
    fn app_composes_one_adapter_after_local_db2_and_before_ordered_stages() {
        let source = include_str!("app.rs");
        assert_eq!(
            source
                .matches("MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp::new")
                .count(),
            1
        );
        let adapter = source
            .find("let hotfix_delivery_metadata_persistence")
            .unwrap();
        let removals = source.find("load_db2_hotfix_removals_like_cpp").unwrap();
        let local_db2 = source.find("build_hotfix_blob_cache").unwrap();
        let staged_sql = source
            .find("load_hotfix_delivery_metadata_like_cpp")
            .unwrap();
        assert!(adapter < removals);
        assert!(removals < local_db2);
        assert!(local_db2 < staged_sql);
    }
}

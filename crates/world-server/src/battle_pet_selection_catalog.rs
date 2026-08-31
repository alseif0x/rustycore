//! Composition boundary for C++ battle-pet breed and quality World catalogs.

use tracing::warn;
use wow_persistence::{
    BattlePetBreedPersistenceRowLikeCpp, BattlePetQualityPersistenceRowLikeCpp,
    BattlePetSelectionCatalogLoadOutcomeLikeCpp, BattlePetSelectionCatalogPersistencePortLikeCpp,
};

async fn load_rows_like_cpp(
    persistence: &dyn BattlePetSelectionCatalogPersistencePortLikeCpp,
) -> (
    Vec<BattlePetBreedPersistenceRowLikeCpp>,
    Vec<BattlePetQualityPersistenceRowLikeCpp>,
) {
    let breeds = match persistence.load_breed_rows_like_cpp().await {
        BattlePetSelectionCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        BattlePetSelectionCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                target: "sql.sql",
                error = %reason,
                ">> Loaded 0 battle pet breeds. DB table `battle_pet_breeds` could not be read."
            );
            Vec::new()
        }
    };
    let qualities = match persistence.load_quality_rows_like_cpp().await {
        BattlePetSelectionCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        BattlePetSelectionCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                target: "sql.sql",
                error = %reason,
                ">> Loaded 0 battle pet qualities. DB table `battle_pet_quality` could not be read."
            );
            Vec::new()
        }
    };
    (breeds, qualities)
}

pub(super) async fn load_battle_pet_selection_store_like_cpp<SpeciesFlags>(
    persistence: &dyn BattlePetSelectionCatalogPersistencePortLikeCpp,
    species_flags: SpeciesFlags,
) -> wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp
where
    SpeciesFlags: FnMut(u32) -> Option<i32>,
{
    let (breeds, qualities) = load_rows_like_cpp(persistence).await;
    wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp::from_sources_like_cpp(
        breeds.into_iter().map(
            |row| wow_data::battle_pet_selection::BattlePetBreedRowLikeCpp {
                species_id: row.species_id,
                breed_id: row.breed_id,
            },
        ),
        qualities.into_iter().map(|row| {
            wow_data::battle_pet_selection::BattlePetQualityRowLikeCpp {
                species_id: row.species_id,
                quality: row.quality,
            }
        }),
        species_flags,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct RecordingPort {
        calls: Mutex<Vec<&'static str>>,
        fail_breeds: bool,
        fail_qualities: bool,
    }

    impl BattlePetSelectionCatalogPersistencePortLikeCpp for RecordingPort {
        fn load_breed_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            BattlePetSelectionCatalogLoadOutcomeLikeCpp<BattlePetBreedPersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.calls.lock().unwrap().push("breeds");
                if self.fail_breeds {
                    BattlePetSelectionCatalogLoadOutcomeLikeCpp::Failed {
                        reason: "breed read failed".into(),
                    }
                } else {
                    BattlePetSelectionCatalogLoadOutcomeLikeCpp::Loaded(vec![
                        BattlePetBreedPersistenceRowLikeCpp {
                            species_id: 10,
                            breed_id: 3,
                        },
                    ])
                }
            })
        }

        fn load_quality_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            BattlePetSelectionCatalogLoadOutcomeLikeCpp<BattlePetQualityPersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.calls.lock().unwrap().push("qualities");
                if self.fail_qualities {
                    BattlePetSelectionCatalogLoadOutcomeLikeCpp::Failed {
                        reason: "quality read failed".into(),
                    }
                } else {
                    BattlePetSelectionCatalogLoadOutcomeLikeCpp::Loaded(vec![
                        BattlePetQualityPersistenceRowLikeCpp {
                            species_id: 10,
                            quality: 2,
                        },
                    ])
                }
            })
        }
    }

    #[tokio::test]
    async fn success_preserves_breed_then_quality_order_and_rows() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_breeds: false,
            fail_qualities: false,
        };
        let (breeds, qualities) = load_rows_like_cpp(&port).await;
        assert_eq!(breeds[0].breed_id, 3);
        assert_eq!(qualities[0].quality, 2);
        assert_eq!(*port.calls.lock().unwrap(), ["breeds", "qualities"]);
    }

    #[tokio::test]
    async fn breed_failure_becomes_empty_without_suppressing_quality() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_breeds: true,
            fail_qualities: false,
        };
        let (breeds, qualities) = load_rows_like_cpp(&port).await;
        assert!(breeds.is_empty());
        assert_eq!(qualities.len(), 1);
        assert_eq!(*port.calls.lock().unwrap(), ["breeds", "qualities"]);
    }

    #[tokio::test]
    async fn quality_failure_becomes_empty_after_preserving_breeds() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_breeds: false,
            fail_qualities: true,
        };
        let (breeds, qualities) = load_rows_like_cpp(&port).await;
        assert_eq!(breeds.len(), 1);
        assert!(qualities.is_empty());
        assert_eq!(*port.calls.lock().unwrap(), ["breeds", "qualities"]);
    }
}

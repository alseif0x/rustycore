//! Composition boundary for represented C++ Player-creation World sources.

use anyhow::{Result, bail};
use wow_core::Position;
use wow_persistence::{
    PlayerCreateCastSpellPersistenceRowLikeCpp, PlayerCreateCustomSpellPersistenceRowLikeCpp,
    PlayerCreateInfoPersistenceRowLikeCpp, PlayerCreationCatalogLoadOutcomeLikeCpp,
    PlayerCreationCatalogPersistencePortLikeCpp,
};

fn player_create_info_row_like_cpp(
    row: PlayerCreateInfoPersistenceRowLikeCpp,
    npe_transport_template_valid: bool,
) -> wow_data::PlayerCreateInfoRowLikeCpp {
    let create_position_npe = match (
        row.npe_map_id,
        row.npe_position_x,
        row.npe_position_y,
        row.npe_position_z,
        row.npe_orientation,
    ) {
        (Some(map_id), Some(x), Some(y), Some(z), Some(orientation)) => {
            Some(wow_data::PlayerCreatePositionLikeCpp {
                map_id,
                position: Position::new(x, y, z, orientation),
                transport_guid: row.npe_transport_guid,
            })
        }
        _ => None,
    };

    wow_data::PlayerCreateInfoRowLikeCpp {
        race: row.race,
        class: row.class,
        create_position: wow_data::PlayerCreatePositionLikeCpp {
            map_id: u32::from(row.map_id),
            position: Position::new(
                row.position_x,
                row.position_y,
                row.position_z,
                row.orientation,
            ),
            transport_guid: None,
        },
        create_position_npe,
        npe_transport_template_valid,
    }
}

fn player_create_cast_spell_row_like_cpp(
    row: PlayerCreateCastSpellPersistenceRowLikeCpp,
) -> wow_data::PlayerCreateInfoCastSpellRowLikeCpp {
    wow_data::PlayerCreateInfoCastSpellRowLikeCpp {
        race_mask: row.race_mask,
        class_mask: row.class_mask,
        spell_id: row.spell_id,
        create_mode: row.create_mode,
    }
}

fn player_create_custom_spell_row_like_cpp(
    row: PlayerCreateCustomSpellPersistenceRowLikeCpp,
) -> wow_data::PlayerCreateInfoCustomSpellRowLikeCpp {
    wow_data::PlayerCreateInfoCustomSpellRowLikeCpp {
        race_mask: row.race_mask,
        class_mask: row.class_mask,
        spell_id: row.spell_id,
    }
}

fn loaded_rows_like_cpp<T>(outcome: PlayerCreationCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        PlayerCreationCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        PlayerCreationCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

async fn load_required_player_create_info_rows_like_cpp(
    persistence: &dyn PlayerCreationCatalogPersistencePortLikeCpp,
) -> Result<Vec<PlayerCreateInfoPersistenceRowLikeCpp>> {
    let rows = loaded_rows_like_cpp(persistence.load_player_create_info_rows_like_cpp().await)?;
    if rows.is_empty() {
        bail!("playercreateinfo is empty");
    }
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn load_player_create_info_store_like_cpp(
    persistence: &dyn PlayerCreationCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    chr_races_store: &wow_data::character_progression::ChrRacesStore,
    chr_classes_store: &wow_data::character_progression::ChrClassesStore,
    chr_model_store: &wow_data::character_progression::ChrModelStore,
    chr_race_x_chr_model_store: &wow_data::character_progression::ChrRaceXChrModelStore,
    gameobject_template_store: &wow_data::GameObjectTemplateLifecycleStoreLikeCpp,
    taxi_path_store: &wow_data::TaxiPathStore,
    taxi_path_node_store: &wow_data::TaxiPathNodeStore,
) -> Result<wow_data::PlayerCreateInfoStoreLikeCpp> {
    let rows = load_required_player_create_info_rows_like_cpp(persistence).await?;

    Ok(wow_data::PlayerCreateInfoStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter().map(|row| {
            let npe_transport_template_valid = row.npe_transport_guid.is_none()
                || row.npe_transport_entry.is_some_and(|entry| {
                    wow_data::player_create_npe_transport_template_valid_like_cpp(
                        entry,
                        gameobject_template_store,
                        taxi_path_store,
                        taxi_path_node_store,
                        map_store,
                    )
                });
            player_create_info_row_like_cpp(row, npe_transport_template_valid)
        }),
        map_store,
        |race| chr_races_store.get(u32::from(race)).is_some(),
        |class| chr_classes_store.get(u32::from(class)).is_some(),
        |race| {
            [0, 1].into_iter().all(|sex| {
                chr_race_x_chr_model_store.entries().any(|race_model| {
                    race_model.chr_races_id == u32::from(race)
                        && race_model.sex == sex
                        && u32::try_from(race_model.chr_model_id)
                            .ok()
                            .is_some_and(|model_id| chr_model_store.get(model_id).is_some())
                })
            })
        },
    ))
}

pub(super) async fn load_player_create_cast_spell_store_like_cpp(
    persistence: &dyn PlayerCreationCatalogPersistencePortLikeCpp,
) -> Result<wow_data::PlayerCreateInfoCastSpellStoreLikeCpp> {
    Ok(
        wow_data::PlayerCreateInfoCastSpellStoreLikeCpp::from_rows_like_cpp(
            loaded_rows_like_cpp(
                persistence
                    .load_player_create_cast_spell_rows_like_cpp()
                    .await,
            )?
            .into_iter()
            .map(player_create_cast_spell_row_like_cpp),
        ),
    )
}

pub(super) async fn load_player_create_custom_spell_store_like_cpp(
    persistence: &dyn PlayerCreationCatalogPersistencePortLikeCpp,
) -> Result<wow_data::PlayerCreateInfoCustomSpellStoreLikeCpp> {
    Ok(
        wow_data::PlayerCreateInfoCustomSpellStoreLikeCpp::from_rows_like_cpp(
            loaded_rows_like_cpp(
                persistence
                    .load_player_create_custom_spell_rows_like_cpp()
                    .await,
            )?
            .into_iter()
            .map(player_create_custom_spell_row_like_cpp),
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    #[test]
    fn typed_base_row_preserves_every_consumed_field_and_complete_npe_group() {
        let row = player_create_info_row_like_cpp(
            PlayerCreateInfoPersistenceRowLikeCpp {
                race: 7,
                class: 3,
                map_id: 530,
                position_x: 1.0,
                position_y: 2.0,
                position_z: 3.0,
                orientation: 4.0,
                npe_map_id: Some(1),
                npe_position_x: Some(5.0),
                npe_position_y: Some(6.0),
                npe_position_z: Some(7.0),
                npe_orientation: Some(8.0),
                npe_transport_guid: Some(9),
                npe_transport_entry: Some(10),
            },
            true,
        );

        assert_eq!(row.race, 7);
        assert_eq!(row.class, 3);
        assert_eq!(row.create_position.map_id, 530);
        assert_eq!(
            row.create_position.position,
            Position::new(1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(
            row.create_position_npe,
            Some(wow_data::PlayerCreatePositionLikeCpp {
                map_id: 1,
                position: Position::new(5.0, 6.0, 7.0, 8.0),
                transport_guid: Some(9),
            })
        );
        assert!(row.npe_transport_template_valid);
    }

    #[test]
    fn incomplete_npe_group_stays_absent_instead_of_fabricating_a_position() {
        let row = player_create_info_row_like_cpp(
            PlayerCreateInfoPersistenceRowLikeCpp {
                race: 1,
                class: 1,
                map_id: 0,
                position_x: 1.0,
                position_y: 2.0,
                position_z: 3.0,
                orientation: 4.0,
                npe_map_id: Some(1),
                npe_position_x: Some(5.0),
                npe_position_y: None,
                npe_position_z: Some(7.0),
                npe_orientation: Some(8.0),
                npe_transport_guid: Some(9),
                npe_transport_entry: None,
            },
            false,
        );

        assert!(row.create_position_npe.is_none());
        assert!(!row.npe_transport_template_valid);
    }

    #[test]
    fn typed_spell_rows_preserve_masks_spell_and_signed_mode() {
        assert_eq!(
            player_create_cast_spell_row_like_cpp(PlayerCreateCastSpellPersistenceRowLikeCpp {
                race_mask: 0x1234,
                class_mask: 0x5678,
                spell_id: 42,
                create_mode: -1,
            }),
            wow_data::PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 0x1234,
                class_mask: 0x5678,
                spell_id: 42,
                create_mode: -1,
            }
        );
        assert_eq!(
            player_create_custom_spell_row_like_cpp(PlayerCreateCustomSpellPersistenceRowLikeCpp {
                race_mask: 0x9ABC,
                class_mask: 0xDEF0,
                spell_id: 84,
            }),
            wow_data::PlayerCreateInfoCustomSpellRowLikeCpp {
                race_mask: 0x9ABC,
                class_mask: 0xDEF0,
                spell_id: 84,
            }
        );
    }

    struct RecordingPort {
        base_calls: AtomicUsize,
        cast_calls: AtomicUsize,
        custom_calls: AtomicUsize,
        cast_fails: bool,
        custom_fails: bool,
    }

    impl PlayerCreationCatalogPersistencePortLikeCpp for RecordingPort {
        fn load_player_create_info_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PlayerCreationCatalogLoadOutcomeLikeCpp<PlayerCreateInfoPersistenceRowLikeCpp>,
        > {
            self.base_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { PlayerCreationCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()) })
        }

        fn load_player_create_cast_spell_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PlayerCreationCatalogLoadOutcomeLikeCpp<PlayerCreateCastSpellPersistenceRowLikeCpp>,
        > {
            self.cast_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if self.cast_fails {
                    PlayerCreationCatalogLoadOutcomeLikeCpp::Failed {
                        reason: "cast read failed".into(),
                    }
                } else {
                    PlayerCreationCatalogLoadOutcomeLikeCpp::Loaded(Vec::new())
                }
            })
        }

        fn load_player_create_custom_spell_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PlayerCreationCatalogLoadOutcomeLikeCpp<PlayerCreateCustomSpellPersistenceRowLikeCpp>,
        > {
            self.custom_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if self.custom_fails {
                    PlayerCreationCatalogLoadOutcomeLikeCpp::Failed {
                        reason: "custom read failed".into(),
                    }
                } else {
                    PlayerCreationCatalogLoadOutcomeLikeCpp::Loaded(Vec::new())
                }
            })
        }
    }

    #[tokio::test]
    async fn empty_base_stage_is_fatal_without_calling_later_sources() {
        let port = RecordingPort {
            base_calls: AtomicUsize::new(0),
            cast_calls: AtomicUsize::new(0),
            custom_calls: AtomicUsize::new(0),
            cast_fails: false,
            custom_fails: false,
        };

        let error = load_required_player_create_info_rows_like_cpp(&port)
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "playercreateinfo is empty");
        assert_eq!(port.base_calls.load(Ordering::SeqCst), 1);
        assert_eq!(port.cast_calls.load(Ordering::SeqCst), 0);
        assert_eq!(port.custom_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn each_spell_stage_calls_only_its_semantic_operation_and_propagates_failure() {
        let port = RecordingPort {
            base_calls: AtomicUsize::new(0),
            cast_calls: AtomicUsize::new(0),
            custom_calls: AtomicUsize::new(0),
            cast_fails: true,
            custom_fails: false,
        };

        let cast = load_player_create_cast_spell_store_like_cpp(&port).await;
        assert_eq!(cast.unwrap_err().to_string(), "cast read failed");
        assert_eq!(port.base_calls.load(Ordering::SeqCst), 0);
        assert_eq!(port.cast_calls.load(Ordering::SeqCst), 1);
        assert_eq!(port.custom_calls.load(Ordering::SeqCst), 0);

        let custom = load_player_create_custom_spell_store_like_cpp(&port)
            .await
            .unwrap();
        assert!(custom.custom_spells_like_cpp(1, 1).is_empty());
        assert_eq!(port.custom_calls.load(Ordering::SeqCst), 1);

        let custom_failure_port = RecordingPort {
            base_calls: AtomicUsize::new(0),
            cast_calls: AtomicUsize::new(0),
            custom_calls: AtomicUsize::new(0),
            cast_fails: false,
            custom_fails: true,
        };
        let custom = load_player_create_custom_spell_store_like_cpp(&custom_failure_port).await;
        assert_eq!(custom.unwrap_err().to_string(), "custom read failed");
        assert_eq!(custom_failure_port.base_calls.load(Ordering::SeqCst), 0);
        assert_eq!(custom_failure_port.cast_calls.load(Ordering::SeqCst), 0);
        assert_eq!(custom_failure_port.custom_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn app_composes_one_adapter_and_keeps_the_three_existing_publication_points() {
        let source = include_str!("app.rs");
        assert_eq!(
            source
                .matches("MariaDbPlayerCreationCatalogPersistenceAdapterLikeCpp::new")
                .count(),
            1
        );
        let base = source
            .find("load_player_create_info_store_like_cpp")
            .unwrap();
        let cast = source
            .find("load_player_create_cast_spell_store_like_cpp")
            .unwrap();
        let custom = source
            .find("load_player_create_custom_spell_store_like_cpp")
            .unwrap();
        assert!(base < cast && cast < custom);
    }
}

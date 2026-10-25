//! Composition boundary for small immutable DB2/rule overlays.

use anyhow::{Result, bail};
use wow_persistence::{
    AreaTableHotfixRowLikeCpp, ChrCustomizationDisplayInfoHotfixRow,
    PowerTypeHotfixRowLikeCpp, SpellEnchantProcPersistenceRowLikeCpp,
    StaticDataOverlayPersistencePortLikeCpp, StaticDataRowsLoadOutcomeLikeCpp,
    UiMapXMapArtHotfixRowLikeCpp,
};

fn loaded<T>(outcome: StaticDataRowsLoadOutcomeLikeCpp<T>) -> Result<T> {
    match outcome {
        StaticDataRowsLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        StaticDataRowsLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn shapeshift_display_links_absent(
    db2: &wow_data::character_progression::ChrCustomizationDisplayInfoStore,
    persistence: &dyn StaticDataOverlayPersistencePortLikeCpp,
) -> Result<bool> {
    let rows = loaded(persistence.load_chr_customization_display_info_rows().await)?;
    Ok(display_link_gate(db2, &rows))
}

fn display_link_gate(
    db2: &wow_data::character_progression::ChrCustomizationDisplayInfoStore,
    overlays: &[ChrCustomizationDisplayInfoHotfixRow],
) -> bool {
    !db2.entries().any(|entry| entry.shapeshift_form_id != 0) && overlays.is_empty()
}

pub(super) async fn load_area_table_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn StaticDataOverlayPersistencePortLikeCpp,
) -> Result<wow_data::AreaTableStore> {
    let mut store = wow_data::AreaTableStore::load(data_dir, locale)?;
    let rows = loaded(persistence.load_area_table_hotfix_rows_like_cpp().await)?;
    store.apply_hotfix_rows_like_cpp(rows.into_iter().map(|row| {
        let AreaTableHotfixRowLikeCpp {
            id,
            continent_id,
            parent_area_id,
            area_bit,
            exploration_level,
            faction_group_mask,
            mount_flags,
            flags,
        } = row;
        (
            wow_data::AreaTableEntry {
                id,
                continent_id,
                parent_area_id,
                area_bit,
                exploration_level,
                mount_flags,
                flags,
            },
            faction_group_mask,
        )
    }));
    Ok(store)
}

pub(super) async fn load_ui_map_x_map_art_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn StaticDataOverlayPersistencePortLikeCpp,
) -> Result<wow_data::UiMapXMapArtStore> {
    let mut store = wow_data::UiMapXMapArtStore::load(data_dir, locale)?;
    let rows = loaded(
        persistence
            .load_ui_map_x_map_art_hotfix_rows_like_cpp()
            .await,
    )?;
    store.apply_hotfix_rows_like_cpp(rows.into_iter().map(
        |UiMapXMapArtHotfixRowLikeCpp {
             id,
             phase_id,
             ui_map_art_id,
             ui_map_id,
         }| wow_data::UiMapXMapArtEntry {
            id,
            phase_id,
            ui_map_art_id,
            ui_map_id,
        },
    ));
    Ok(store)
}

fn power_entry(row: PowerTypeHotfixRowLikeCpp) -> wow_data::character_progression::PowerTypeEntry {
    wow_data::character_progression::PowerTypeEntry {
        id: row.id,
        name_global_string_tag: row.name_global_string_tag,
        cost_global_string_tag: row.cost_global_string_tag,
        power_type_enum: row.power_type_enum,
        min_power: row.min_power,
        max_base_power: row.max_base_power,
        center_power: row.center_power,
        default_power: row.default_power,
        display_modifier: row.display_modifier,
        regen_interrupt_time_ms: row.regen_interrupt_time_ms,
        regen_peace: row.regen_peace,
        regen_combat: row.regen_combat,
        flags: row.flags,
    }
}

pub(super) async fn load_power_type_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn StaticDataOverlayPersistencePortLikeCpp,
) -> Result<wow_data::character_progression::PowerTypeStore> {
    let mut store = wow_data::character_progression::PowerTypeStore::load(data_dir, locale)?;
    let (official, custom) = loaded(persistence.load_power_type_hotfix_rows_like_cpp().await)?;
    store.apply_hotfix_overlays_like_cpp(
        official.into_iter().map(power_entry),
        custom.into_iter().map(power_entry),
    );
    Ok(store)
}

pub(super) async fn load_spell_enchant_proc_store_like_cpp(
    persistence: &dyn StaticDataOverlayPersistencePortLikeCpp,
    enchantment_store: &wow_data::SpellItemEnchantmentStore,
) -> Result<wow_data::SpellEnchantProcLoadOutcomeLikeCpp> {
    let rows = loaded(persistence.load_spell_enchant_proc_rows_like_cpp().await)?;
    Ok(wow_data::SpellEnchantProcStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter().map(
            |SpellEnchantProcPersistenceRowLikeCpp {
                 enchant_id,
                 chance,
                 procs_per_minute,
                 hit_mask,
                 attributes_mask,
             }| wow_data::SpellEnchantProcRowLikeCpp {
                enchant_id,
                chance,
                procs_per_minute,
                hit_mask,
                attributes_mask,
            },
        ),
        enchantment_store,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct DisplayInfoOverlayPort {
        rows: StaticDataRowsLoadOutcomeLikeCpp<Vec<ChrCustomizationDisplayInfoHotfixRow>>,
    }

    fn port_error<T>() -> StaticDataRowsLoadOutcomeLikeCpp<T> {
        StaticDataRowsLoadOutcomeLikeCpp::Failed {
            reason: "port unavailable".into(),
        }
    }

    impl StaticDataOverlayPersistencePortLikeCpp for DisplayInfoOverlayPort {
        fn load_area_table_hotfix_rows_like_cpp(
            &self,
        ) -> wow_persistence::PersistenceFutureLikeCpp<
            '_ ,
            StaticDataRowsLoadOutcomeLikeCpp<Vec<AreaTableHotfixRowLikeCpp>>,
        > {
            Box::pin(async { port_error() })
        }

        fn load_power_type_hotfix_rows_like_cpp(
            &self,
        ) -> wow_persistence::PersistenceFutureLikeCpp<
            '_ ,
            StaticDataRowsLoadOutcomeLikeCpp<(
                Vec<PowerTypeHotfixRowLikeCpp>,
                Vec<PowerTypeHotfixRowLikeCpp>,
            )>,
        > {
            Box::pin(async { port_error() })
        }

        fn load_ui_map_x_map_art_hotfix_rows_like_cpp(
            &self,
        ) -> wow_persistence::PersistenceFutureLikeCpp<
            '_ ,
            StaticDataRowsLoadOutcomeLikeCpp<Vec<UiMapXMapArtHotfixRowLikeCpp>>,
        > {
            Box::pin(async { port_error() })
        }

        fn load_chr_customization_display_info_rows(
            &self,
        ) -> wow_persistence::PersistenceFutureLikeCpp<
            '_ ,
            StaticDataRowsLoadOutcomeLikeCpp<Vec<ChrCustomizationDisplayInfoHotfixRow>>,
        > {
            let rows = self.rows.clone();
            Box::pin(async move { rows })
        }

        fn load_spell_enchant_proc_rows_like_cpp(
            &self,
        ) -> wow_persistence::PersistenceFutureLikeCpp<
            '_ ,
            StaticDataRowsLoadOutcomeLikeCpp<Vec<SpellEnchantProcPersistenceRowLikeCpp>>,
        > {
            Box::pin(async { port_error() })
        }
    }

    fn display_info_port(
        rows: StaticDataRowsLoadOutcomeLikeCpp<Vec<ChrCustomizationDisplayInfoHotfixRow>>,
    ) -> DisplayInfoOverlayPort {
        DisplayInfoOverlayPort { rows }
    }

    #[tokio::test]
    async fn sql_form_link_fails_the_startup_gate_without_database_writes() {
        let db2 = wow_data::character_progression::ChrCustomizationDisplayInfoStore::from_entries([]);
        let port = display_info_port(StaticDataRowsLoadOutcomeLikeCpp::Loaded(vec![
            ChrCustomizationDisplayInfoHotfixRow {
                id: 17,
                shapeshift_form_id: 1,
                display_id: 900,
            },
        ]));
        assert!(!shapeshift_display_links_absent(&db2, &port)
            .await
            .unwrap());
    }

    #[test]
    fn db2_form_link_fails_the_fallback_gate() {
        use wow_data::character_progression::{
            ChrCustomizationDisplayInfoEntry as Entry,
            ChrCustomizationDisplayInfoStore as Store,
        };
        let db2 = Store::from_entries([Entry {
            id: 17,
            shapeshift_form_id: 1,
            display_id: 900,
            barber_shop_min_camera_distance: 0.0,
            barber_shop_height_offset: 0.0,
        }]);
        assert!(!display_link_gate(&db2, &[]));
    }

    #[tokio::test]
    async fn empty_effective_inputs_certify_only_the_fallback_dataset() {
        let db2 = wow_data::character_progression::ChrCustomizationDisplayInfoStore::from_entries([]);
        let port = display_info_port(StaticDataRowsLoadOutcomeLikeCpp::Loaded(vec![]));
        assert!(shapeshift_display_links_absent(&db2, &port)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn failed_hotfix_read_is_not_treated_as_empty_by_startup_consumer() {
        let db2 = wow_data::character_progression::ChrCustomizationDisplayInfoStore::from_entries([]);
        let port = display_info_port(port_error());
        assert!(shapeshift_display_links_absent(&db2, &port).await.is_err());
    }

    #[test]
    fn failure_is_not_converted_into_an_empty_overlay() {
        let error =
            loaded::<Vec<AreaTableHotfixRowLikeCpp>>(StaticDataRowsLoadOutcomeLikeCpp::Failed {
                reason: "read failed".into(),
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "read failed");
    }

    #[test]
    fn power_conversion_preserves_signed_and_float_domains() {
        let entry = power_entry(PowerTypeHotfixRowLikeCpp {
            id: 1,
            name_global_string_tag: "A".into(),
            cost_global_string_tag: "B".into(),
            power_type_enum: -2,
            min_power: -3,
            max_base_power: 4,
            center_power: -5,
            default_power: 6,
            display_modifier: -7,
            regen_interrupt_time_ms: 8,
            regen_peace: f32::from_bits(0x7fc0_0001),
            regen_combat: -0.0,
            flags: -9,
        });
        assert_eq!(entry.power_type_enum, -2);
        assert_eq!(entry.min_power, -3);
        assert_eq!(entry.regen_peace.to_bits(), 0x7fc0_0001);
        assert_eq!(entry.regen_combat.to_bits(), (-0.0f32).to_bits());
        assert_eq!(entry.flags, -9);
    }
}

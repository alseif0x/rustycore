//! Composition boundary for the represented startup gossip catalogs.

use anyhow::{Context, Result, bail};
use wow_persistence::{
    GossipMenuOptionCatalogRowLikeCpp, GossipStartupCatalogLoadOutcomeLikeCpp,
    GossipStartupCatalogPersistencePortLikeCpp,
};

fn loaded_rows_like_cpp<T>(outcome: GossipStartupCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        GossipStartupCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        GossipStartupCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

fn menu_item_like_cpp(row: GossipMenuOptionCatalogRowLikeCpp) -> wow_data::GossipMenuItem {
    wow_data::GossipMenuItem {
        menu_id: row.menu_id,
        gossip_option_id: row.gossip_option_id,
        order_index: row.option_id,
        option_npc: row.option_npc,
        option_text: row.option_text,
        option_broadcast_text_id: row.option_broadcast_text_id,
        language: row.language,
        flags: row.flags,
        action_menu_id: row.action_menu_id,
        action_poi_id: row.action_poi_id,
        gossip_npc_option_id: row.gossip_npc_option_id,
        box_coded: row.box_coded,
        box_money: row.box_money,
        box_text: row.box_text,
        box_broadcast_text_id: row.box_broadcast_text_id,
        spell_id: row.spell_id,
        override_icon_id: row.override_icon_id,
        conditions: wow_data::ConditionsReference::default(),
    }
}

pub(super) async fn load_gossip_startup_catalog_like_cpp(
    persistence: &dyn GossipStartupCatalogPersistencePortLikeCpp,
) -> Result<(wow_data::GossipStore, wow_data::GossipLoadReport)> {
    // Preserve Rust's existing production sequence. C++ loads the locale rows
    // in an earlier localization phase; reconciling that drift is behavior
    // work, not part of this persistence-boundary move.
    let menus = loaded_rows_like_cpp(persistence.load_menu_rows_like_cpp().await)
        .context("Failed to load C++ gossip_menu rows")?;
    let options = loaded_rows_like_cpp(persistence.load_menu_option_rows_like_cpp().await)
        .context("Failed to load C++ gossip_menu_option rows")?;
    let locales = loaded_rows_like_cpp(persistence.load_menu_option_locale_rows_like_cpp().await)
        .context("Failed to load C++ gossip_menu_option locale rows")?;
    let addons = loaded_rows_like_cpp(persistence.load_menu_addon_rows_like_cpp().await)
        .context("Failed to load C++ gossip_menu_addon rows")?;

    Ok(wow_data::GossipStore::from_rows_like_cpp(
        menus.into_iter().map(|row| wow_data::GossipMenuRowLikeCpp {
            menu_id: row.menu_id,
            text_id: row.text_id,
        }),
        options.into_iter().map(menu_item_like_cpp),
        locales
            .into_iter()
            .map(|row| wow_data::GossipMenuItemsLocaleRowLikeCpp {
                menu_id: row.menu_id,
                option_id: row.option_id,
                locale: row.locale,
                option_text: row.option_text,
                box_text: row.box_text,
            }),
        addons
            .into_iter()
            .map(|row| wow_data::GossipMenuAddonRowLikeCpp {
                menu_id: row.menu_id,
                friendship_faction_id: row.friendship_faction_id,
            }),
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use wow_persistence::{
        GossipMenuAddonPersistenceRowLikeCpp, GossipMenuOptionLocalePersistenceRowLikeCpp,
        GossipMenuPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
    };

    struct RecordingPort {
        calls: Mutex<Vec<&'static str>>,
        fail_at: Option<&'static str>,
    }

    impl RecordingPort {
        fn outcome<T>(&self, stage: &'static str) -> GossipStartupCatalogLoadOutcomeLikeCpp<T> {
            self.calls.lock().unwrap().push(stage);
            if self.fail_at == Some(stage) {
                GossipStartupCatalogLoadOutcomeLikeCpp::Failed {
                    reason: format!("{stage} failed"),
                }
            } else {
                GossipStartupCatalogLoadOutcomeLikeCpp::Loaded(Vec::new())
            }
        }
    }

    impl GossipStartupCatalogPersistencePortLikeCpp for RecordingPort {
        fn load_menu_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuPersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.outcome("menus") })
        }

        fn load_menu_option_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuOptionCatalogRowLikeCpp>,
        > {
            Box::pin(async { self.outcome("options") })
        }

        fn load_menu_option_locale_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuOptionLocalePersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.outcome("locales") })
        }

        fn load_menu_addon_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuAddonPersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.outcome("addons") })
        }
    }

    #[tokio::test]
    async fn empty_success_keeps_the_existing_four_stage_order() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
        };

        let (store, report) = load_gossip_startup_catalog_like_cpp(&port).await.unwrap();

        assert_eq!(
            *port.calls.lock().unwrap(),
            ["menus", "options", "locales", "addons"]
        );
        assert_eq!(store.menu_row_count(), 0);
        assert_eq!(report, wow_data::GossipLoadReport::default());
    }

    #[tokio::test]
    async fn first_failure_stops_later_reads_before_publication() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: Some("options"),
        };

        let error = load_gossip_startup_catalog_like_cpp(&port)
            .await
            .unwrap_err();

        assert_eq!(*port.calls.lock().unwrap(), ["menus", "options"]);
        assert!(error.to_string().contains("gossip_menu_option"));
    }

    #[test]
    fn typed_option_preserves_every_consumed_domain_field() {
        let item = menu_item_like_cpp(GossipMenuOptionCatalogRowLikeCpp {
            menu_id: 1,
            gossip_option_id: -2,
            option_id: 3,
            option_npc: 4,
            option_text: "option".to_owned(),
            option_broadcast_text_id: 5,
            language: 6,
            flags: -7,
            action_menu_id: 8,
            action_poi_id: 9,
            gossip_npc_option_id: Some(-10),
            box_coded: true,
            box_money: 11,
            box_text: "box".to_owned(),
            box_broadcast_text_id: 12,
            spell_id: Some(-13),
            override_icon_id: Some(14),
        });

        assert_eq!(item.menu_id, 1);
        assert_eq!(item.gossip_option_id, -2);
        assert_eq!(item.order_index, 3);
        assert_eq!(item.option_npc, 4);
        assert_eq!(item.option_text, "option");
        assert_eq!(item.option_broadcast_text_id, 5);
        assert_eq!(item.language, 6);
        assert_eq!(item.flags, -7);
        assert_eq!(item.action_menu_id, 8);
        assert_eq!(item.action_poi_id, 9);
        assert_eq!(item.gossip_npc_option_id, Some(-10));
        assert!(item.box_coded);
        assert_eq!(item.box_money, 11);
        assert_eq!(item.box_text, "box");
        assert_eq!(item.box_broadcast_text_id, 12);
        assert_eq!(item.spell_id, Some(-13));
        assert_eq!(item.override_icon_id, Some(14));
    }

    #[test]
    fn app_composes_one_gossip_adapter_for_startup_and_runtime() {
        let source = include_str!("app.rs");
        assert_eq!(
            source
                .matches("MariaDbGossipCatalogPersistenceAdapterLikeCpp::new")
                .count(),
            1
        );
        assert!(source.contains("load_gossip_startup_catalog_like_cpp"));
        assert!(source.contains("gossip_catalog_adapter.clone()"));
    }
}

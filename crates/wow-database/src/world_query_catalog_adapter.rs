//! MariaDB startup adapter for immutable C++ `ObjectMgr` query catalogs.
//!
//! The projections and table order follow `ObjectMgr.cpp` startup loaders:
//! creature templates/locales at 349/255, page texts/locales at 6143/6189,
//! and gameobject templates/addons/locales at 7556/7768/7461.

use std::sync::Arc;

use wow_persistence::*;

use crate::WorldDatabase;

pub struct MariaDbWorldQueryCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbWorldQueryCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn creature_rows(&self) -> anyhow::Result<CreatureQueryCatalogPersistenceRowsLikeCpp> {
        let mut result = self.world_db.direct_query(concat!(
            "SELECT ct.entry, ct.name, ct.femaleName, ct.subname, ct.TitleAlt, ct.IconName, ",
            "ct.type, ct.family, ct.Classification, ct.KillCredit1, ct.KillCredit2, ",
            "ct.Civilian, ct.RacialLeader, ct.movementId, ct.RequiredExpansion, ct.VignetteID, ",
            "ct.unit_class, ct.WidgetSetID, ct.WidgetSetUnitConditionID, ",
            "ctdiff.HealthModifier, ctdiff.ManaModifier, ctdiff.CreatureDifficultyID, ",
            "ctdiff.TypeFlags, ctdiff.TypeFlags2 FROM creature_template ct ",
            "LEFT JOIN creature_template_difficulty ctdiff ON ct.entry = ctdiff.Entry AND ctdiff.DifficultyID = 0"
        )).await?;
        let mut templates = Vec::new();
        if !result.is_empty() {
            loop {
                templates.push(CreatureQueryTemplatePersistenceRowLikeCpp {
                    entry: result.try_read(0).unwrap_or(0),
                    name: result.try_read(1).unwrap_or_default(),
                    subname: result.try_read(3).unwrap_or_default(),
                    title_alt: result.try_read(4).unwrap_or_default(),
                    icon_name: result.try_read(5).unwrap_or_default(),
                    creature_type: result.try_read(6).unwrap_or(0),
                    creature_family: result.try_read(7).unwrap_or(0),
                    classification: result.try_read(8).unwrap_or(0),
                    kill_credits: [
                        result.try_read(9).unwrap_or(0),
                        result.try_read(10).unwrap_or(0),
                    ],
                    civilian: result.try_read::<u8>(11).unwrap_or(0) != 0,
                    racial_leader: result.try_read::<u8>(12).unwrap_or(0) != 0,
                    movement_id: result.try_read(13).unwrap_or(0),
                    required_expansion: result.try_read(14).unwrap_or(0),
                    vignette_id: result.try_read(15).unwrap_or(0),
                    unit_class: result.try_read::<u8>(16).unwrap_or(1) as i32,
                    widget_set_id: result.try_read(17).unwrap_or(0),
                    widget_set_unit_condition_id: result.try_read(18).unwrap_or(0),
                    hp_multi: result.try_read::<Option<f32>>(19).flatten().unwrap_or(1.0),
                    energy_multi: result.try_read::<Option<f32>>(20).flatten().unwrap_or(1.0),
                    creature_difficulty_id: result
                        .try_read::<Option<i32>>(21)
                        .flatten()
                        .unwrap_or(0),
                    type_flags: [
                        result.try_read::<Option<u32>>(22).flatten().unwrap_or(0),
                        result.try_read::<Option<u32>>(23).flatten().unwrap_or(0),
                    ],
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        let mut result = self.world_db.direct_query(
            "SELECT CreatureID, CreatureDisplayID, DisplayScale, Probability FROM creature_template_model ORDER BY CreatureID, Idx"
        ).await?;
        let mut displays = Vec::new();
        if !result.is_empty() {
            loop {
                displays.push(CreatureQueryDisplayPersistenceRowLikeCpp {
                    entry: result.try_read(0).unwrap_or(0),
                    display_id: result.try_read(1).unwrap_or(0),
                    scale: result.try_read(2).unwrap_or(1.0),
                    probability: result.try_read(3).unwrap_or(1.0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        let mut result = self.world_db.direct_query(
            "SELECT entry, locale, Name, NameAlt, Title, TitleAlt FROM creature_template_locale"
        ).await?;
        let mut locales = Vec::new();
        if !result.is_empty() {
            loop {
                locales.push(CreatureQueryLocalePersistenceRowLikeCpp {
                    entry: result.try_read(0).unwrap_or(0),
                    locale: result.try_read(1).unwrap_or_default(),
                    name: result.try_read(2).unwrap_or_default(),
                    subname: result.try_read(4).unwrap_or_default(),
                    title_alt: result.try_read(5).unwrap_or_default(),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(CreatureQueryCatalogPersistenceRowsLikeCpp {
            templates,
            displays,
            locales,
        })
    }

    async fn gameobject_rows(
        &self,
    ) -> anyhow::Result<GameObjectQueryCatalogPersistenceRowsLikeCpp> {
        let mut result = self.world_db.direct_query(concat!(
            "SELECT gt.entry, gt.type, gt.displayId, gt.name, gt.IconName, gt.castBarCaption, gt.unk1, gt.size, ",
            "gt.Data0, gt.Data1, gt.Data2, gt.Data3, gt.Data4, gt.Data5, gt.Data6, gt.Data7, gt.Data8, gt.Data9, ",
            "gt.Data10, gt.Data11, gt.Data12, gt.Data13, gt.Data14, gt.Data15, gt.Data16, gt.Data17, gt.Data18, ",
            "gt.Data19, gt.Data20, gt.Data21, gt.Data22, gt.Data23, gt.Data24, gt.Data25, gt.Data26, gt.Data27, ",
            "gt.Data28, gt.Data29, gt.Data30, gt.Data31, gt.Data32, gt.Data33, gt.Data34, gt.ContentTuningId, ",
            "COALESCE(gta.mingold, 0), COALESCE(gta.maxgold, 0) FROM gameobject_template gt ",
            "LEFT JOIN gameobject_template_addon gta ON gt.entry = gta.entry"
        )).await?;
        let mut templates = Vec::new();
        if !result.is_empty() {
            loop {
                let mut data = [0_i32; WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP];
                for (index, value) in data.iter_mut().enumerate() {
                    *value = result.try_read(8 + index).unwrap_or(0);
                }
                templates.push(GameObjectQueryTemplatePersistenceRowLikeCpp {
                    entry: result.try_read(0).unwrap_or(0),
                    go_type: result.try_read(1).unwrap_or(0),
                    display_id: result.try_read(2).unwrap_or(0),
                    name: result.try_read(3).unwrap_or_default(),
                    icon_name: result.try_read(4).unwrap_or_default(),
                    cast_bar_caption: result.try_read(5).unwrap_or_default(),
                    unk_string: result.try_read(6).unwrap_or_default(),
                    size: result.try_read(7).unwrap_or(1.0),
                    data,
                    content_tuning_id: result.try_read(43).unwrap_or(0),
                    min_money: result.try_read(44).unwrap_or(0),
                    max_money: result.try_read(45).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        let mut result = self
            .world_db
            .direct_query(
                "SELECT entry, locale, Name, CastBarCaption, Unk1 FROM gameobject_template_locale",
            )
            .await?;
        let mut locales = Vec::new();
        if !result.is_empty() {
            loop {
                locales.push(GameObjectQueryLocalePersistenceRowLikeCpp {
                    entry: result.try_read(0).unwrap_or(0),
                    locale: result.try_read(1).unwrap_or_default(),
                    name: result.try_read(2).unwrap_or_default(),
                    cast_bar_caption: result.try_read(3).unwrap_or_default(),
                    unk_string: result.try_read(4).unwrap_or_default(),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(GameObjectQueryCatalogPersistenceRowsLikeCpp { templates, locales })
    }

    async fn page_rows(&self) -> anyhow::Result<PageTextCatalogPersistenceRowsLikeCpp> {
        let mut result = self
            .world_db
            .direct_query("SELECT ID, `Text`, NextPageID, PlayerConditionID, Flags FROM page_text")
            .await?;
        let mut pages = Vec::new();
        if !result.is_empty() {
            loop {
                pages.push(PageTextPersistenceRowLikeCpp {
                    id: result.try_read(0).unwrap_or(0),
                    text: result.try_read(1).unwrap_or_default(),
                    next_page_id: result.try_read(2).unwrap_or(0),
                    player_condition_id: result.try_read(3).unwrap_or(0),
                    flags: result.try_read(4).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        let mut result = self
            .world_db
            .direct_query("SELECT ID, locale, `Text` FROM page_text_locale")
            .await?;
        let mut locales = Vec::new();
        if !result.is_empty() {
            loop {
                locales.push(PageTextLocalePersistenceRowLikeCpp {
                    id: result.try_read(0).unwrap_or(0),
                    locale: result.try_read(1).unwrap_or_default(),
                    text: result.try_read(2).unwrap_or_default(),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(PageTextCatalogPersistenceRowsLikeCpp { pages, locales })
    }
}

impl WorldQueryCatalogPersistencePortLikeCpp for MariaDbWorldQueryCatalogPersistenceAdapterLikeCpp {
    fn load_creature_query_catalog_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldQueryCatalogLoadOutcomeLikeCpp<CreatureQueryCatalogPersistenceRowsLikeCpp>,
    > {
        Box::pin(async move {
            self.creature_rows()
                .await
                .map(WorldQueryCatalogLoadOutcomeLikeCpp::Loaded)
                .unwrap_or_else(|error| WorldQueryCatalogLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                })
        })
    }

    fn load_gameobject_query_catalog_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldQueryCatalogLoadOutcomeLikeCpp<GameObjectQueryCatalogPersistenceRowsLikeCpp>,
    > {
        Box::pin(async move {
            self.gameobject_rows()
                .await
                .map(WorldQueryCatalogLoadOutcomeLikeCpp::Loaded)
                .unwrap_or_else(|error| WorldQueryCatalogLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                })
        })
    }

    fn load_page_text_catalog_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldQueryCatalogLoadOutcomeLikeCpp<PageTextCatalogPersistenceRowsLikeCpp>,
    > {
        Box::pin(async move {
            self.page_rows()
                .await
                .map(WorldQueryCatalogLoadOutcomeLikeCpp::Loaded)
                .unwrap_or_else(|error| WorldQueryCatalogLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                })
        })
    }
}

//! MariaDB startup adapter for immutable C++ `ObjectMgr` query catalogs.
//!
//! The projections and table order follow `ObjectMgr.cpp` startup loaders:
//! creature templates/locales at 349/255, page texts/locales at 6143/6189,
//! and gameobject templates/addons/locales at 7556/7768/7461.

use std::sync::Arc;

use anyhow::Result;
use wow_persistence::*;

use crate::WorldDatabase;
use crate::field_like_cpp::FieldRowLikeCpp;
use crate::result::collect_rows_like_cpp;

/// C++ `ObjectMgr::LoadCreatureTemplate` (ObjectMgr.cpp:403-470) columns the
/// query catalog needs, plus the difficulty-0 `LoadCreatureTemplateDifficulty`
/// (ObjectMgr.cpp:940-1035) modifiers.
const CREATURE_TEMPLATE_QUERY_CATALOG_SQL_LIKE_CPP: &str = concat!(
    "SELECT ct.entry, ct.name, ct.femaleName, ct.subname, ct.TitleAlt, ct.IconName, ",
    "ct.type, ct.family, ct.Classification, ct.KillCredit1, ct.KillCredit2, ",
    "ct.Civilian, ct.RacialLeader, ct.movementId, ct.RequiredExpansion, ct.VignetteID, ",
    "ct.unit_class, ct.WidgetSetID, ct.WidgetSetUnitConditionID, ",
    "ctdiff.HealthModifier, ctdiff.ManaModifier, ctdiff.CreatureDifficultyID, ",
    "ctdiff.TypeFlags, ctdiff.TypeFlags2 FROM creature_template ct ",
    "LEFT JOIN creature_template_difficulty ctdiff ON ct.entry = ctdiff.Entry AND ctdiff.DifficultyID = 0"
);

/// C++ `ObjectMgr::LoadGameObjectTemplate` (ObjectMgr.cpp:7556-7606) and the
/// `gameobject_template_addon` gold (7775-7801) in one startup projection.
const GAMEOBJECT_TEMPLATE_QUERY_CATALOG_SQL_LIKE_CPP: &str = concat!(
    "SELECT gt.entry, gt.type, gt.displayId, gt.name, gt.IconName, gt.castBarCaption, gt.unk1, gt.size, ",
    "gt.Data0, gt.Data1, gt.Data2, gt.Data3, gt.Data4, gt.Data5, gt.Data6, gt.Data7, gt.Data8, gt.Data9, ",
    "gt.Data10, gt.Data11, gt.Data12, gt.Data13, gt.Data14, gt.Data15, gt.Data16, gt.Data17, gt.Data18, ",
    "gt.Data19, gt.Data20, gt.Data21, gt.Data22, gt.Data23, gt.Data24, gt.Data25, gt.Data26, gt.Data27, ",
    "gt.Data28, gt.Data29, gt.Data30, gt.Data31, gt.Data32, gt.Data33, gt.Data34, gt.ContentTuningId, ",
    "gta.mingold, gta.maxgold FROM gameobject_template gt ",
    "LEFT JOIN gameobject_template_addon gta ON gt.entry = gta.entry"
);

/// One `CreatureTemplate` query row with the C++ getter widths
/// (ObjectMgr.cpp:405-466). `type`/`Classification` are TINYINT UNSIGNED
/// (`GetUInt8`) and `KillCredit1/2`/`movementId` INT UNSIGNED (`GetUInt32`);
/// sqlx rejected all five as i32 and they silently became 0 (#1436).
fn creature_query_template_row_like_cpp<R: FieldRowLikeCpp + ?Sized>(
    row: &R,
) -> Result<CreatureQueryTemplatePersistenceRowLikeCpp> {
    Ok(CreatureQueryTemplatePersistenceRowLikeCpp {
        entry: row.get_like_cpp::<u32>(0, "creature_template.entry")?,
        name: row.get_string_like_cpp(1, "creature_template.name")?,
        subname: row.get_string_like_cpp(3, "creature_template.subname")?,
        title_alt: row.get_string_like_cpp(4, "creature_template.TitleAlt")?,
        icon_name: row.get_string_like_cpp(5, "creature_template.IconName")?,
        creature_type: i32::from(row.get_like_cpp::<u8>(6, "creature_template.type")?),
        creature_family: row.get_like_cpp::<i32>(7, "creature_template.family")?,
        classification: i32::from(row.get_like_cpp::<u8>(8, "creature_template.Classification")?),
        kill_credits: [
            row.get_like_cpp::<u32>(9, "creature_template.KillCredit1")?
                .cast_signed(),
            row.get_like_cpp::<u32>(10, "creature_template.KillCredit2")?
                .cast_signed(),
        ],
        civilian: row.get_bool_like_cpp(11, "creature_template.Civilian")?,
        racial_leader: row.get_bool_like_cpp(12, "creature_template.RacialLeader")?,
        movement_id: row
            .get_like_cpp::<u32>(13, "creature_template.movementId")?
            .cast_signed(),
        required_expansion: row
            .get_like_cpp::<u32>(14, "creature_template.RequiredExpansion")?
            .cast_signed(),
        vignette_id: row
            .get_like_cpp::<u32>(15, "creature_template.VignetteID")?
            .cast_signed(),
        unit_class: i32::from(row.get_like_cpp::<u8>(16, "creature_template.unit_class")?),
        widget_set_id: row.get_like_cpp::<i32>(17, "creature_template.WidgetSetID")?,
        widget_set_unit_condition_id: row
            .get_like_cpp::<i32>(18, "creature_template.WidgetSetUnitConditionID")?,
        // No difficulty-0 row keeps the C++ `CreatureDifficulty` defaults.
        hp_multi: row
            .get_nullable_like_cpp::<f32>(19, "creature_template_difficulty.HealthModifier")?
            .unwrap_or(1.0),
        energy_multi: row
            .get_nullable_like_cpp::<f32>(20, "creature_template_difficulty.ManaModifier")?
            .unwrap_or(1.0),
        creature_difficulty_id: row
            .get_like_cpp::<i32>(21, "creature_template_difficulty.CreatureDifficultyID")?,
        type_flags: [
            row.get_like_cpp::<u32>(22, "creature_template_difficulty.TypeFlags")?,
            row.get_like_cpp::<u32>(23, "creature_template_difficulty.TypeFlags2")?,
        ],
    })
}

/// One `GameObjectTemplate` row with the C++ field widths
/// (ObjectMgr.cpp:7591-7606 and 7800-7801).
fn gameobject_query_template_row_like_cpp<R: FieldRowLikeCpp + ?Sized>(
    row: &R,
) -> Result<GameObjectQueryTemplatePersistenceRowLikeCpp> {
    let mut data = [0_i32; WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP];
    for (index, value) in data.iter_mut().enumerate() {
        // `got.raw.data[i] = fields[8 + i].GetUInt32()` over a signed INT.
        *value = row
            .get_like_cpp::<u32>(8 + index, "gameobject_template.Data")?
            .cast_signed();
    }
    Ok(GameObjectQueryTemplatePersistenceRowLikeCpp {
        entry: row.get_like_cpp::<u32>(0, "gameobject_template.entry")?,
        // `type` is TINYINT UNSIGNED (`GetUInt8`), `displayId` INT UNSIGNED;
        // sqlx rejected both as i32 and every template became a DOOR (#1430).
        go_type: i32::from(row.get_like_cpp::<u8>(1, "gameobject_template.type")?),
        display_id: row
            .get_like_cpp::<u32>(2, "gameobject_template.displayId")?
            .cast_signed(),
        name: row.get_string_like_cpp(3, "gameobject_template.name")?,
        icon_name: row.get_string_like_cpp(4, "gameobject_template.IconName")?,
        cast_bar_caption: row.get_string_like_cpp(5, "gameobject_template.castBarCaption")?,
        unk_string: row.get_string_like_cpp(6, "gameobject_template.unk1")?,
        size: row.get_like_cpp::<f32>(7, "gameobject_template.size")?,
        data,
        content_tuning_id: row.get_like_cpp::<i32>(43, "gameobject_template.ContentTuningId")?,
        // The addon gold is read directly: COALESCE(INT UNSIGNED, 0) is
        // DECIMAL, and a missing addon row (NULL) keeps the C++ default 0.
        min_money: row.get_like_cpp::<u32>(44, "gameobject_template_addon.mingold")?,
        max_money: row.get_like_cpp::<u32>(45, "gameobject_template_addon.maxgold")?,
    })
}

pub struct MariaDbWorldQueryCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbWorldQueryCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn creature_rows(&self) -> Result<CreatureQueryCatalogPersistenceRowsLikeCpp> {
        let templates = collect_rows_like_cpp(
            self.world_db
                .direct_query(CREATURE_TEMPLATE_QUERY_CATALOG_SQL_LIKE_CPP)
                .await?,
            "creature_template",
            creature_query_template_row_like_cpp,
        )?;
        // ObjectMgr::LoadCreatureTemplateModels (ObjectMgr.cpp:620-650).
        let displays = collect_rows_like_cpp(
            self.world_db.direct_query(
                "SELECT CreatureID, CreatureDisplayID, DisplayScale, Probability FROM creature_template_model ORDER BY CreatureID, Idx"
            ).await?,
            "creature_template_model",
            |row| {
                Ok(CreatureQueryDisplayPersistenceRowLikeCpp {
                    entry: row.get_like_cpp(0, "creature_template_model.CreatureID")?,
                    display_id: row.get_like_cpp(1, "creature_template_model.CreatureDisplayID")?,
                    scale: row.get_like_cpp(2, "creature_template_model.DisplayScale")?,
                    probability: row.get_like_cpp(3, "creature_template_model.Probability")?,
                })
            },
        )?;
        let locales = collect_rows_like_cpp(
            self.world_db.direct_query(
                "SELECT entry, locale, Name, NameAlt, Title, TitleAlt FROM creature_template_locale"
            ).await?,
            "creature_template_locale",
            |row| {
                Ok(CreatureQueryLocalePersistenceRowLikeCpp {
                    entry: row.get_like_cpp(0, "creature_template_locale.entry")?,
                    locale: row.get_string_like_cpp(1, "creature_template_locale.locale")?,
                    name: row.get_string_like_cpp(2, "creature_template_locale.Name")?,
                    subname: row.get_string_like_cpp(4, "creature_template_locale.Title")?,
                    title_alt: row.get_string_like_cpp(5, "creature_template_locale.TitleAlt")?,
                })
            },
        )?;
        Ok(CreatureQueryCatalogPersistenceRowsLikeCpp {
            templates,
            displays,
            locales,
        })
    }

    async fn gameobject_rows(&self) -> Result<GameObjectQueryCatalogPersistenceRowsLikeCpp> {
        let templates = collect_rows_like_cpp(
            self.world_db
                .direct_query(GAMEOBJECT_TEMPLATE_QUERY_CATALOG_SQL_LIKE_CPP)
                .await?,
            "gameobject_template",
            gameobject_query_template_row_like_cpp,
        )?;
        let locales = collect_rows_like_cpp(
            self.world_db
                .direct_query(
                    "SELECT entry, locale, Name, CastBarCaption, Unk1 FROM gameobject_template_locale",
                )
                .await?,
            "gameobject_template_locale",
            |row| {
                Ok(GameObjectQueryLocalePersistenceRowLikeCpp {
                    entry: row.get_like_cpp(0, "gameobject_template_locale.entry")?,
                    locale: row.get_string_like_cpp(1, "gameobject_template_locale.locale")?,
                    name: row.get_string_like_cpp(2, "gameobject_template_locale.name")?,
                    cast_bar_caption: row
                        .get_string_like_cpp(3, "gameobject_template_locale.castBarCaption")?,
                    unk_string: row.get_string_like_cpp(4, "gameobject_template_locale.unk1")?,
                })
            },
        )?;
        Ok(GameObjectQueryCatalogPersistenceRowsLikeCpp { templates, locales })
    }

    /// ObjectMgr::LoadPageTexts / LoadPageTextLocales (ObjectMgr.cpp:6143-6205).
    async fn page_rows(&self) -> Result<PageTextCatalogPersistenceRowsLikeCpp> {
        let pages = collect_rows_like_cpp(
            self.world_db
                .direct_query(
                    "SELECT ID, `Text`, NextPageID, PlayerConditionID, Flags FROM page_text",
                )
                .await?,
            "page_text",
            |row| {
                Ok(PageTextPersistenceRowLikeCpp {
                    id: row.get_like_cpp(0, "page_text.ID")?,
                    text: row.get_string_like_cpp(1, "page_text.Text")?,
                    next_page_id: row.get_like_cpp(2, "page_text.NextPageID")?,
                    player_condition_id: row.get_like_cpp(3, "page_text.PlayerConditionID")?,
                    flags: row.get_like_cpp(4, "page_text.Flags")?,
                })
            },
        )?;
        let locales = collect_rows_like_cpp(
            self.world_db
                .direct_query("SELECT ID, locale, `Text` FROM page_text_locale")
                .await?,
            "page_text_locale",
            |row| {
                Ok(PageTextLocalePersistenceRowLikeCpp {
                    id: row.get_like_cpp(0, "page_text_locale.ID")?,
                    locale: row.get_string_like_cpp(1, "page_text_locale.locale")?,
                    text: row.get_string_like_cpp(2, "page_text_locale.Text")?,
                })
            },
        )?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_like_cpp::NativeFieldLikeCpp as Native;
    use crate::world::live_tests::live_world_probe_rows_like_cpp;

    /// A `creature_template` query row as MariaDB types it (TDB schema).
    fn creature_template_row(
        creature_type: u8,
        classification: u8,
        kill_credit: u32,
        movement_id: u32,
    ) -> Vec<Native> {
        vec![
            Native::UInt32(448),           // entry INT UNSIGNED
            Native::Text("Hogger".into()), // name MEDIUMTEXT
            Native::Null,                  // femaleName
            Native::Null,                  // subname
            Native::Null,                  // TitleAlt
            Native::Null,                  // IconName
            Native::UInt8(creature_type),  // type TINYINT UNSIGNED
            Native::Int32(0),              // family INT
            Native::UInt8(classification), // Classification TINYINT UNSIGNED
            Native::UInt32(kill_credit),   // KillCredit1 INT UNSIGNED
            Native::UInt32(0),             // KillCredit2 INT UNSIGNED
            Native::UInt8(0),              // Civilian TINYINT UNSIGNED
            Native::UInt8(1),              // RacialLeader TINYINT UNSIGNED
            Native::UInt32(movement_id),   // movementId INT UNSIGNED
            Native::Int32(0),              // RequiredExpansion INT
            Native::Int32(0),              // VignetteID INT
            Native::UInt8(1),              // unit_class TINYINT UNSIGNED
            Native::Int32(0),              // WidgetSetID INT
            Native::Int32(0),              // WidgetSetUnitConditionID INT
            Native::Null,                  // HealthModifier FLOAT (no difficulty row)
            Native::Null,                  // ManaModifier FLOAT
            Native::Null,                  // CreatureDifficultyID INT
            Native::Null,                  // TypeFlags INT UNSIGNED
            Native::Null,                  // TypeFlags2 INT UNSIGNED
        ]
    }

    #[test]
    fn creature_query_row_reads_unsigned_type_classification_credit_and_movement_like_cpp() {
        let row = creature_template_row(7, 1, 17_839, 1_693);
        let template = creature_query_template_row_like_cpp(&row[..]).unwrap();
        assert_eq!(template.entry, 448);
        assert_eq!(template.name, "Hogger");
        assert_eq!(template.subname, "");
        assert_eq!(template.creature_type, 7);
        assert_eq!(template.classification, 1);
        assert_eq!(template.kill_credits, [17_839, 0]);
        assert_eq!(template.movement_id, 1_693);
        assert!(!template.civilian);
        assert!(template.racial_leader);
        assert_eq!(template.unit_class, 1);
        // No difficulty-0 row: C++ `CreatureDifficulty` defaults.
        assert_eq!((template.hp_multi, template.energy_multi), (1.0, 1.0));
        assert_eq!(template.creature_difficulty_id, 0);
        assert_eq!(template.type_flags, [0, 0]);
    }

    #[test]
    fn creature_query_row_fails_closed_instead_of_defaulting_a_bad_column() {
        let mut row = creature_template_row(7, 1, 0, 0);
        row[6] = Native::Unsupported("DECIMAL".into());
        let error = creature_query_template_row_like_cpp(&row[..]).unwrap_err();
        assert!(
            error.to_string().contains("creature_template.type"),
            "{error}"
        );

        let mut row = creature_template_row(7, 1, 0, 0);
        row[16] = Native::UInt32(300);
        let error = creature_query_template_row_like_cpp(&row[..]).unwrap_err();
        assert!(error.to_string().contains("Field::GetUInt8"), "{error}");
    }

    #[test]
    fn gameobject_query_row_reads_signed_data_and_missing_addon_gold_like_cpp() {
        let mut row = vec![
            Native::UInt32(1721),                         // entry
            Native::UInt8(10),                            // type TINYINT UNSIGNED
            Native::UInt32(259),                          // displayId INT UNSIGNED
            Native::Text("Locked ball and chain".into()), // name
            Native::Text(String::new()),                  // IconName
            Native::Text(String::new()),                  // castBarCaption
            Native::Text(String::new()),                  // unk1
            Native::Float(1.0),                           // size
        ];
        row.extend(
            (0..WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP).map(|index| match index {
                0 => Native::Int32(36),
                1 => Native::Int32(-1),
                _ => Native::Int32(0),
            }),
        );
        row.extend([Native::Int32(-3), Native::Null, Native::Null]);
        let template = gameobject_query_template_row_like_cpp(&row[..]).unwrap();
        assert_eq!((template.entry, template.go_type), (1721, 10));
        assert_eq!(template.display_id, 259);
        assert_eq!(template.name, "Locked ball and chain");
        assert_eq!(template.data[..3], [36, -1, 0]);
        assert_eq!(template.content_tuning_id, -3);
        assert_eq!((template.min_money, template.max_money), (0, 0));
    }

    #[test]
    fn query_catalog_projections_do_not_widen_columns_with_coalesce() {
        // COALESCE(INT UNSIGNED, 0) is DECIMAL in MariaDB, which no C++
        // numeric getter accepts; C++ reads the columns directly (#1430).
        for sql in [
            CREATURE_TEMPLATE_QUERY_CATALOG_SQL_LIKE_CPP,
            GAMEOBJECT_TEMPLATE_QUERY_CATALOG_SQL_LIKE_CPP,
        ] {
            assert!(!sql.contains("COALESCE"), "{sql}");
        }
        let sql = GAMEOBJECT_TEMPLATE_QUERY_CATALOG_SQL_LIKE_CPP;
        assert!(sql.starts_with("SELECT gt.entry, gt.type, gt.displayId, "));
        assert!(sql.contains("gt.ContentTuningId, gta.mingold, gta.maxgold FROM"));
    }

    /// #1430/#1436: run the startup row mappers over real `gameobject_template`
    /// and `creature_template` rows inside a READ ONLY transaction.
    #[tokio::test]
    #[ignore = "read-only MariaDB probe; set RUSTYCORE_DB_IT_USER, RUSTYCORE_DB_IT_WORLD_DB and optional HOST/PORT/PASS/SOCKET"]
    async fn live_gameobject_template_rows_keep_cpp_type_display_and_gold() -> anyhow::Result<()> {
        let rows = live_world_probe_rows_like_cpp(&format!(
            "{GAMEOBJECT_TEMPLATE_QUERY_CATALOG_SQL_LIKE_CPP} WHERE gt.entry IN (1721, 2846, 181366) ORDER BY gt.entry"
        ))
        .await?;
        assert_eq!(rows.len(), 3);

        // Locked ball and chain: signed `Data1 = -1`.
        let chain = gameobject_query_template_row_like_cpp(&rows[0][..])?;
        assert_eq!((chain.entry, chain.go_type), (1721, 10));
        assert_eq!(chain.data[..2], [36, -1]);

        // Tattered Chest: GAMEOBJECT_TYPE_CHEST, lock 57, loot 2278, no addon row.
        let chest = gameobject_query_template_row_like_cpp(&rows[1][..])?;
        assert_eq!(chest.entry, 2846);
        assert_eq!(chest.go_type, 3);
        assert_eq!(chest.display_id, 259);
        assert_eq!(chest.data[1], 2278);
        assert_eq!((chest.min_money, chest.max_money), (0, 0));

        let gold_chest = gameobject_query_template_row_like_cpp(&rows[2][..])?;
        assert_eq!(gold_chest.entry, 181_366);
        assert_eq!(gold_chest.go_type, 3);
        assert_eq!(
            (gold_chest.min_money, gold_chest.max_money),
            (1_700_000, 1_800_000)
        );
        Ok(())
    }

    #[tokio::test]
    #[ignore = "read-only MariaDB probe; set RUSTYCORE_DB_IT_USER, RUSTYCORE_DB_IT_WORLD_DB and optional HOST/PORT/PASS/SOCKET"]
    async fn live_creature_template_rows_keep_cpp_unsigned_query_fields() -> anyhow::Result<()> {
        let rows = live_world_probe_rows_like_cpp(&format!(
            "{CREATURE_TEMPLATE_QUERY_CATALOG_SQL_LIKE_CPP} WHERE ct.entry IN (448, 21140) ORDER BY ct.entry"
        ))
        .await?;
        assert_eq!(rows.len(), 2);

        // Hogger: humanoid elite.
        let hogger = creature_query_template_row_like_cpp(&rows[0][..])?;
        assert_eq!(hogger.entry, 448);
        assert_eq!((hogger.creature_type, hogger.classification), (7, 1));
        assert_eq!(hogger.movement_id, 1693);

        // Rift Lord: demon elite with a kill credit.
        let rift_lord = creature_query_template_row_like_cpp(&rows[1][..])?;
        assert_eq!(rift_lord.entry, 21140);
        assert_eq!((rift_lord.creature_type, rift_lord.classification), (2, 1));
        assert_eq!(rift_lord.kill_credits, [17_839, 0]);
        assert_eq!(rift_lord.movement_id, 1694);
        Ok(())
    }
}

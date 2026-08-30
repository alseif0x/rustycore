//! MariaDB adapter for Rust's transitional on-demand gossip catalog reads.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    GossipBroadcastTextLocaleRequestLikeCpp, GossipCatalogPersistencePortLikeCpp,
    GossipCatalogReadOutcomeLikeCpp, GossipCreatureMenuRequestLikeCpp,
    GossipMenuAddonPersistenceRowLikeCpp, GossipMenuCatalogRequestLikeCpp,
    GossipMenuOptionCatalogRowLikeCpp, GossipMenuOptionLocalePersistenceRowLikeCpp,
    GossipMenuPersistenceRowLikeCpp, GossipNpcTextCatalogRequestLikeCpp,
    GossipStartupCatalogLoadOutcomeLikeCpp, GossipStartupCatalogPersistencePortLikeCpp,
    PersistenceFutureLikeCpp,
};

use crate::{PreparedStatement, SqlResult, WorldDatabase, WorldStatements};

fn read_integer_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<i128> {
    result
        .try_read::<i64>(column)
        .map(i128::from)
        .or_else(|| result.try_read::<u64>(column).map(i128::from))
        .or_else(|| result.try_read::<i32>(column).map(i128::from))
        .or_else(|| result.try_read::<u32>(column).map(i128::from))
        .or_else(|| result.try_read::<i16>(column).map(i128::from))
        .or_else(|| result.try_read::<u16>(column).map(i128::from))
        .or_else(|| result.try_read::<i8>(column).map(i128::from))
        .or_else(|| result.try_read::<u8>(column).map(i128::from))
        .with_context(|| format!("missing or non-integer {field} SQL column {column}"))
}

fn u32_field_like_cpp(value: i128, field: &'static str) -> Result<u32> {
    if let Ok(value) = u32::try_from(value) {
        return Ok(value);
    }
    i32::try_from(value)
        .map(|value| value as u32)
        .with_context(|| format!("{field} SQL value {value} is outside the C++ uint32 field range"))
}

fn i32_field_like_cpp(value: i128, field: &'static str) -> Result<i32> {
    i32::try_from(value)
        .or_else(|_| u32::try_from(value).map(|value| value as i32))
        .with_context(|| format!("{field} SQL value {value} is outside the C++ int32 field range"))
}

fn u8_field_like_cpp(value: i128, field: &'static str) -> Result<u8> {
    if let Ok(value) = u8::try_from(value) {
        return Ok(value);
    }
    i8::try_from(value)
        .map(|value| value as u8)
        .with_context(|| format!("{field} SQL value {value} is outside the C++ uint8 field range"))
}

fn read_u32_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<u32> {
    u32_field_like_cpp(read_integer_checked_like_cpp(result, column, field)?, field)
}

fn read_i32_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<i32> {
    i32_field_like_cpp(read_integer_checked_like_cpp(result, column, field)?, field)
}

fn read_nullable_i32_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<Option<i32>> {
    if result.is_null(column) {
        Ok(None)
    } else {
        read_i32_checked_like_cpp(result, column, field).map(Some)
    }
}

fn menu_values_like_cpp(values: (i128, i128)) -> Result<GossipMenuPersistenceRowLikeCpp> {
    Ok(GossipMenuPersistenceRowLikeCpp {
        menu_id: u32_field_like_cpp(values.0, "GossipMenu.MenuID")?,
        text_id: u32_field_like_cpp(values.1, "GossipMenu.TextID")?,
    })
}

fn menu_row_like_cpp(result: &SqlResult) -> Result<GossipMenuPersistenceRowLikeCpp> {
    menu_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "GossipMenu.MenuID")?,
        read_integer_checked_like_cpp(result, 1, "GossipMenu.TextID")?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn menu_option_values_like_cpp(
    menu_id: i128,
    gossip_option_id: i128,
    option_id: i128,
    option_npc: i128,
    option_text: String,
    option_broadcast_text_id: i128,
    language: i128,
    flags: i128,
    action_menu_id: i128,
    action_poi_id: i128,
    gossip_npc_option_id: Option<i128>,
    box_coded: i128,
    box_money: i128,
    box_text: String,
    box_broadcast_text_id: i128,
    spell_id: Option<i128>,
    override_icon_id: Option<i128>,
) -> Result<GossipMenuOptionCatalogRowLikeCpp> {
    Ok(GossipMenuOptionCatalogRowLikeCpp {
        menu_id: u32_field_like_cpp(menu_id, "GossipMenuOption.MenuID")?,
        gossip_option_id: i32_field_like_cpp(gossip_option_id, "GossipMenuOption.GossipOptionID")?,
        option_id: u32_field_like_cpp(option_id, "GossipMenuOption.OptionID")?,
        option_npc: u8_field_like_cpp(option_npc, "GossipMenuOption.OptionNpc")?,
        option_text,
        option_broadcast_text_id: u32_field_like_cpp(
            option_broadcast_text_id,
            "GossipMenuOption.OptionBroadcastTextID",
        )?,
        language: u32_field_like_cpp(language, "GossipMenuOption.Language")?,
        flags: i32_field_like_cpp(flags, "GossipMenuOption.Flags")?,
        action_menu_id: u32_field_like_cpp(action_menu_id, "GossipMenuOption.ActionMenuID")?,
        action_poi_id: u32_field_like_cpp(action_poi_id, "GossipMenuOption.ActionPoiID")?,
        gossip_npc_option_id: gossip_npc_option_id
            .map(|value| i32_field_like_cpp(value, "GossipMenuOption.GossipNpcOptionID"))
            .transpose()?,
        box_coded: u8_field_like_cpp(box_coded, "GossipMenuOption.BoxCoded")? != 0,
        box_money: u32_field_like_cpp(box_money, "GossipMenuOption.BoxMoney")?,
        box_text,
        box_broadcast_text_id: u32_field_like_cpp(
            box_broadcast_text_id,
            "GossipMenuOption.BoxBroadcastTextID",
        )?,
        spell_id: spell_id
            .map(|value| i32_field_like_cpp(value, "GossipMenuOption.SpellID"))
            .transpose()?,
        override_icon_id: override_icon_id
            .map(|value| i32_field_like_cpp(value, "GossipMenuOption.OverrideIconID"))
            .transpose()?,
    })
}

fn menu_option_row_like_cpp(result: &SqlResult) -> Result<GossipMenuOptionCatalogRowLikeCpp> {
    menu_option_values_like_cpp(
        read_integer_checked_like_cpp(result, 0, "GossipMenuOption.MenuID")?,
        read_integer_checked_like_cpp(result, 1, "GossipMenuOption.GossipOptionID")?,
        read_integer_checked_like_cpp(result, 2, "GossipMenuOption.OptionID")?,
        read_integer_checked_like_cpp(result, 3, "GossipMenuOption.OptionNpc")?,
        result.read_string(4),
        read_integer_checked_like_cpp(result, 5, "GossipMenuOption.OptionBroadcastTextID")?,
        read_integer_checked_like_cpp(result, 6, "GossipMenuOption.Language")?,
        read_integer_checked_like_cpp(result, 7, "GossipMenuOption.Flags")?,
        read_integer_checked_like_cpp(result, 8, "GossipMenuOption.ActionMenuID")?,
        read_integer_checked_like_cpp(result, 9, "GossipMenuOption.ActionPoiID")?,
        read_nullable_i32_like_cpp(result, 10, "GossipMenuOption.GossipNpcOptionID")?
            .map(i128::from),
        read_integer_checked_like_cpp(result, 11, "GossipMenuOption.BoxCoded")?,
        read_integer_checked_like_cpp(result, 12, "GossipMenuOption.BoxMoney")?,
        result.read_string(13),
        read_integer_checked_like_cpp(result, 14, "GossipMenuOption.BoxBroadcastTextID")?,
        read_nullable_i32_like_cpp(result, 15, "GossipMenuOption.SpellID")?.map(i128::from),
        read_nullable_i32_like_cpp(result, 16, "GossipMenuOption.OverrideIconID")?.map(i128::from),
    )
}

fn locale_row_like_cpp(result: &SqlResult) -> Result<GossipMenuOptionLocalePersistenceRowLikeCpp> {
    Ok(GossipMenuOptionLocalePersistenceRowLikeCpp {
        menu_id: read_u32_checked_like_cpp(result, 0, "GossipMenuOptionLocale.MenuID")?,
        option_id: read_u32_checked_like_cpp(result, 1, "GossipMenuOptionLocale.OptionID")?,
        locale: result.read_string(2),
        option_text: result.read_string(3),
        box_text: result.read_string(4),
    })
}

fn addon_values_like_cpp(values: (i128, i128)) -> Result<GossipMenuAddonPersistenceRowLikeCpp> {
    Ok(GossipMenuAddonPersistenceRowLikeCpp {
        menu_id: u32_field_like_cpp(values.0, "GossipMenuAddon.MenuID")?,
        friendship_faction_id: i32_field_like_cpp(values.1, "GossipMenuAddon.FriendshipFactionID")?,
    })
}

fn addon_row_like_cpp(result: &SqlResult) -> Result<GossipMenuAddonPersistenceRowLikeCpp> {
    addon_values_like_cpp((
        read_integer_checked_like_cpp(result, 0, "GossipMenuAddon.MenuID")?,
        read_integer_checked_like_cpp(result, 1, "GossipMenuAddon.FriendshipFactionID")?,
    ))
}

async fn query_startup_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> Result<T>,
) -> Result<Vec<T>> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::with_capacity(result.count());
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(decode(&result)?);
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn classify_startup_rows_like_cpp<T>(
    result: Result<Vec<T>>,
) -> GossipStartupCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => GossipStartupCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => GossipStartupCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

fn u32_statement_like_cpp(statement: WorldStatements, value: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(statement);
    statement.set_u32(0, value);
    statement
}

fn broadcast_text_locale_statement_like_cpp(
    broadcast_text_id: u32,
    locale: &str,
) -> PreparedStatement {
    let mut statement = u32_statement_like_cpp(
        WorldStatements::SEL_BROADCAST_TEXT_LOCALE,
        broadcast_text_id,
    );
    statement.set_string(1, locale);
    statement
}

pub struct MariaDbGossipCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbGossipCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl GossipStartupCatalogPersistencePortLikeCpp for MariaDbGossipCatalogPersistenceAdapterLikeCpp {
    fn load_menu_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_startup_rows_like_cpp(
                query_startup_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_GOSSIP_MENUS,
                    menu_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_menu_option_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuOptionCatalogRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_startup_rows_like_cpp(
                query_startup_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_GOSSIP_MENU_OPTIONS_ALL,
                    menu_option_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_menu_option_locale_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuOptionLocalePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_startup_rows_like_cpp(
                query_startup_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_GOSSIP_MENU_OPTION_LOCALES,
                    locale_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_menu_addon_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuAddonPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_startup_rows_like_cpp(
                query_startup_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_GOSSIP_MENU_ADDON,
                    addon_row_like_cpp,
                )
                .await,
            )
        })
    }
}

impl GossipCatalogPersistencePortLikeCpp for MariaDbGossipCatalogPersistenceAdapterLikeCpp {
    fn load_creature_gossip_menu_id_like_cpp<'a>(
        &'a self,
        request: GossipCreatureMenuRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<u32>> {
        Box::pin(async move {
            let result = match self
                .world_db
                .query(&u32_statement_like_cpp(
                    WorldStatements::SEL_CREATURE_GOSSIP_MENU,
                    request.creature_entry,
                ))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return GossipCatalogReadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            result
                .try_read::<u32>(0)
                .map(GossipCatalogReadOutcomeLikeCpp::Found)
                .unwrap_or(GossipCatalogReadOutcomeLikeCpp::Missing)
        })
    }

    fn load_gossip_menu_text_ids_like_cpp<'a>(
        &'a self,
        request: GossipMenuCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<Vec<u32>>> {
        Box::pin(async move {
            let mut result = match self
                .world_db
                .query(&u32_statement_like_cpp(
                    WorldStatements::SEL_GOSSIP_MENU_TEXTS,
                    request.menu_id,
                ))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return GossipCatalogReadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            if result.is_empty() {
                return GossipCatalogReadOutcomeLikeCpp::Missing;
            }

            let mut text_ids = Vec::new();
            loop {
                text_ids.push(result.try_read::<u32>(0).unwrap_or(1));
                if !result.next_row() {
                    break;
                }
            }
            GossipCatalogReadOutcomeLikeCpp::Found(text_ids)
        })
    }

    fn load_npc_text_broadcast_id_like_cpp<'a>(
        &'a self,
        request: GossipNpcTextCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<i32>> {
        Box::pin(async move {
            let result = match self
                .world_db
                .query(&u32_statement_like_cpp(
                    WorldStatements::SEL_NPC_TEXT,
                    request.npc_text_id,
                ))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return GossipCatalogReadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            result
                .try_read::<u32>(0)
                .map(|value| GossipCatalogReadOutcomeLikeCpp::Found(value as i32))
                .unwrap_or(GossipCatalogReadOutcomeLikeCpp::Missing)
        })
    }

    fn load_gossip_menu_options_like_cpp<'a>(
        &'a self,
        request: GossipMenuCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        'a,
        GossipCatalogReadOutcomeLikeCpp<Vec<GossipMenuOptionCatalogRowLikeCpp>>,
    > {
        Box::pin(async move {
            let mut result = match self
                .world_db
                .query(&u32_statement_like_cpp(
                    WorldStatements::SEL_GOSSIP_MENU_OPTIONS,
                    request.menu_id,
                ))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return GossipCatalogReadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            if result.is_empty() {
                return GossipCatalogReadOutcomeLikeCpp::Missing;
            }

            let mut options = Vec::new();
            loop {
                options.push(GossipMenuOptionCatalogRowLikeCpp {
                    menu_id: result.try_read(0).unwrap_or(request.menu_id),
                    gossip_option_id: result.try_read(1).unwrap_or(0),
                    option_id: result.try_read(2).unwrap_or(0),
                    option_npc: result.try_read(3).unwrap_or(0),
                    option_text: result.read_string(4),
                    option_broadcast_text_id: result.try_read::<u32>(5).unwrap_or(0),
                    language: result.try_read::<u32>(6).unwrap_or(0),
                    flags: result.try_read::<i32>(7).unwrap_or(0),
                    action_menu_id: result.try_read(8).unwrap_or(0),
                    action_poi_id: result.try_read(9).unwrap_or(0),
                    gossip_npc_option_id: result.try_read(10),
                    box_coded: result.try_read::<u8>(11).unwrap_or(0) != 0,
                    box_money: result.try_read(12).unwrap_or(0),
                    box_text: result.read_string(13),
                    box_broadcast_text_id: result.try_read::<u32>(14).unwrap_or(0),
                    spell_id: result.try_read(15),
                    override_icon_id: result.try_read(16),
                });
                if !result.next_row() {
                    break;
                }
            }
            GossipCatalogReadOutcomeLikeCpp::Found(options)
        })
    }

    fn load_broadcast_text_locale_like_cpp<'a>(
        &'a self,
        request: GossipBroadcastTextLocaleRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<String>> {
        Box::pin(async move {
            let result = match self
                .world_db
                .query(&broadcast_text_locale_statement_like_cpp(
                    request.broadcast_text_id,
                    &request.locale,
                ))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return GossipCatalogReadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            if result.is_empty() {
                GossipCatalogReadOutcomeLikeCpp::Missing
            } else {
                GossipCatalogReadOutcomeLikeCpp::Found(result.read_string(0))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn gossip_catalog_statements_preserve_identity_and_binds() {
        let creature =
            u32_statement_like_cpp(WorldStatements::SEL_CREATURE_GOSSIP_MENU, 0x0102_0304);
        assert_eq!(
            creature.sql(),
            WorldStatements::SEL_CREATURE_GOSSIP_MENU.sql()
        );
        assert_eq!(creature.params(), [SqlParam::U32(0x0102_0304)]);

        let texts = u32_statement_like_cpp(WorldStatements::SEL_GOSSIP_MENU_TEXTS, 41);
        assert_eq!(texts.sql(), WorldStatements::SEL_GOSSIP_MENU_TEXTS.sql());
        assert_eq!(texts.params(), [SqlParam::U32(41)]);

        let npc_text = u32_statement_like_cpp(WorldStatements::SEL_NPC_TEXT, 42);
        assert_eq!(npc_text.sql(), WorldStatements::SEL_NPC_TEXT.sql());
        assert_eq!(npc_text.params(), [SqlParam::U32(42)]);

        let options = u32_statement_like_cpp(WorldStatements::SEL_GOSSIP_MENU_OPTIONS, 43);
        assert_eq!(
            options.sql(),
            WorldStatements::SEL_GOSSIP_MENU_OPTIONS.sql()
        );
        assert_eq!(options.params(), [SqlParam::U32(43)]);

        let locale = broadcast_text_locale_statement_like_cpp(44, "esES");
        assert_eq!(
            locale.sql(),
            WorldStatements::SEL_BROADCAST_TEXT_LOCALE.sql()
        );
        assert_eq!(
            locale.params(),
            [SqlParam::U32(44), SqlParam::String("esES".to_owned())]
        );

        assert_eq!(
            WorldStatements::SEL_GOSSIP_MENUS.sql(),
            "SELECT MenuID, TextID FROM gossip_menu"
        );
        assert!(
            WorldStatements::SEL_GOSSIP_MENU_OPTIONS_ALL
                .sql()
                .ends_with("FROM gossip_menu_option ORDER BY MenuID, OptionID")
        );
        assert_eq!(
            WorldStatements::SEL_GOSSIP_MENU_OPTION_LOCALES.sql(),
            "SELECT MenuID, OptionID, Locale, OptionText, BoxText FROM gossip_menu_option_locale"
        );
        assert_eq!(
            WorldStatements::SEL_GOSSIP_MENU_ADDON.sql(),
            "SELECT MenuID, FriendshipFactionID FROM gossip_menu_addon"
        );
    }

    #[test]
    fn startup_rows_preserve_all_consumed_fields_widths_and_nulls() {
        assert_eq!(
            menu_values_like_cpp((7, 9)).unwrap(),
            GossipMenuPersistenceRowLikeCpp {
                menu_id: 7,
                text_id: 9,
            }
        );
        assert_eq!(
            addon_values_like_cpp((7, -11)).unwrap(),
            GossipMenuAddonPersistenceRowLikeCpp {
                menu_id: 7,
                friendship_faction_id: -11,
            }
        );

        let option = menu_option_values_like_cpp(
            1,
            -2,
            3,
            4,
            "option".to_owned(),
            5,
            6,
            -7,
            8,
            9,
            None,
            1,
            10,
            "box".to_owned(),
            11,
            Some(-12),
            Some(13),
        )
        .unwrap();
        assert_eq!(
            option,
            GossipMenuOptionCatalogRowLikeCpp {
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
                gossip_npc_option_id: None,
                box_coded: true,
                box_money: 10,
                box_text: "box".to_owned(),
                box_broadcast_text_id: 11,
                spell_id: Some(-12),
                override_icon_id: Some(13),
            }
        );

        assert!(menu_values_like_cpp((i128::from(u32::MAX) + 1, 0)).is_err());
        assert!(
            menu_option_values_like_cpp(
                1,
                2,
                3,
                256,
                String::new(),
                0,
                0,
                0,
                0,
                0,
                None,
                0,
                0,
                String::new(),
                0,
                None,
                None,
            )
            .is_err()
        );
    }
}

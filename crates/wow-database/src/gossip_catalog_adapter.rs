//! MariaDB adapter for Rust's transitional on-demand gossip catalog reads.

use std::sync::Arc;

use wow_persistence::{
    GossipBroadcastTextLocaleRequestLikeCpp, GossipCatalogPersistencePortLikeCpp,
    GossipCatalogReadOutcomeLikeCpp, GossipCreatureMenuRequestLikeCpp,
    GossipMenuCatalogRequestLikeCpp, GossipMenuOptionCatalogRowLikeCpp,
    GossipNpcTextCatalogRequestLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{PreparedStatement, WorldDatabase, WorldStatements};

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
    }
}

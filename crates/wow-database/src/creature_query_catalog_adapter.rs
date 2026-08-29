//! MariaDB adapter for Rust's transitional on-demand creature query catalog.

use std::sync::Arc;

use wow_persistence::{
    CreatureQueryCatalogOutcomeLikeCpp, CreatureQueryCatalogPersistencePortLikeCpp,
    CreatureQueryCatalogRequestLikeCpp, CreatureQueryCatalogRowLikeCpp,
    CreatureQueryDisplayRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{PreparedStatement, WorldDatabase, WorldStatements};

fn statement_like_cpp(statement: WorldStatements, entry: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(statement);
    statement.set_u32(0, entry);
    statement
}

pub struct MariaDbCreatureQueryCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbCreatureQueryCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl CreatureQueryCatalogPersistencePortLikeCpp
    for MariaDbCreatureQueryCatalogPersistenceAdapterLikeCpp
{
    fn load_creature_query_catalog_like_cpp<'a>(
        &'a self,
        request: CreatureQueryCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, CreatureQueryCatalogOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match self
                .world_db
                .query(&statement_like_cpp(
                    WorldStatements::SEL_CREATURE_QUERY_RESPONSE,
                    request.entry,
                ))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return CreatureQueryCatalogOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            if result.is_empty() {
                return CreatureQueryCatalogOutcomeLikeCpp::Missing;
            }

            let mut row = CreatureQueryCatalogRowLikeCpp {
                name: result.read_string(1),
                subname: result.read_string(3),
                title_alt: result.read_string(4),
                icon_name: result.read_string(5),
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
                creature_difficulty_id: result.try_read::<Option<i32>>(21).flatten().unwrap_or(0),
                type_flags: [
                    result.try_read::<Option<u32>>(22).flatten().unwrap_or(0),
                    result.try_read::<Option<u32>>(23).flatten().unwrap_or(0),
                ],
                displays: Vec::new(),
            };

            let mut locale_error = None;
            if !request.locale.is_empty() && request.locale != "enUS" {
                let mut statement = statement_like_cpp(
                    WorldStatements::SEL_CREATURE_TEMPLATE_LOCALE,
                    request.entry,
                );
                statement.set_string(1, &request.locale);
                match self.world_db.query(&statement).await {
                    Ok(locale) if !locale.is_empty() => {
                        let name = locale.read_string(0);
                        let subname = locale.read_string(2);
                        let title_alt = locale.read_string(3);
                        if !name.is_empty() {
                            row.name = name;
                        }
                        if !subname.is_empty() {
                            row.subname = subname;
                        }
                        if !title_alt.is_empty() {
                            row.title_alt = title_alt;
                        }
                    }
                    Ok(_) => {}
                    Err(error) => locale_error = Some(error.to_string()),
                }
            }

            if let Ok(mut displays) = self
                .world_db
                .query(&statement_like_cpp(
                    WorldStatements::SEL_CREATURE_DISPLAY_MODELS,
                    request.entry,
                ))
                .await
            {
                if !displays.is_empty() {
                    loop {
                        row.displays.push(CreatureQueryDisplayRowLikeCpp {
                            display_id: displays.try_read(0).unwrap_or(0),
                            scale: displays.try_read(1).unwrap_or(1.0),
                            probability: displays.try_read(2).unwrap_or(1.0),
                        });
                        if !displays.next_row() {
                            break;
                        }
                    }
                }
            }

            CreatureQueryCatalogOutcomeLikeCpp::Found { row, locale_error }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn creature_query_statements_preserve_identity_and_entry_bind() {
        for identity in [
            WorldStatements::SEL_CREATURE_QUERY_RESPONSE,
            WorldStatements::SEL_CREATURE_DISPLAY_MODELS,
        ] {
            let statement = statement_like_cpp(identity, 0xA1B2_C3D4);
            assert_eq!(statement.sql(), identity.sql());
            assert_eq!(statement.params(), [SqlParam::U32(0xA1B2_C3D4)]);
        }
    }
}

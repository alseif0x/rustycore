// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! MariaDB adapter for canonical-map corpse hydration.

use std::sync::Arc;

use wow_persistence::{
    MapCorpseAuxiliaryLoadOutcomeLikeCpp, MapCorpseCustomizationLoadRowLikeCpp,
    MapCorpseLoadOutcomeLikeCpp, MapCorpseLoadRequestLikeCpp, MapCorpseLoadRowLikeCpp,
    MapCorpsePersistencePortLikeCpp, MapCorpsePhaseLoadRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::CharacterDatabase;
use crate::params::PreparedStatement;
use crate::statements::CharStatements;

fn map_corpse_load_statements_like_cpp(
    request: MapCorpseLoadRequestLikeCpp,
) -> [PreparedStatement; 3] {
    let mut corpses = PreparedStatement::for_statement(CharStatements::SEL_CORPSES);
    corpses.set_u32(0, request.map_id);
    corpses.set_u32(1, request.instance_id);

    let mut phases = PreparedStatement::for_statement(CharStatements::SEL_CORPSE_PHASES);
    phases.set_u32(0, request.map_id);
    phases.set_u32(1, request.instance_id);

    let mut customizations =
        PreparedStatement::for_statement(CharStatements::SEL_CORPSE_CUSTOMIZATIONS);
    customizations.set_u32(0, request.map_id);
    customizations.set_u32(1, request.instance_id);

    [corpses, phases, customizations]
}

pub struct MariaDbMapCorpsePersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbMapCorpsePersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl MapCorpsePersistencePortLikeCpp for MariaDbMapCorpsePersistenceAdapterLikeCpp {
    fn load_map_corpses_like_cpp<'a>(
        &'a self,
        request: MapCorpseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, MapCorpseLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let [corpse_stmt, phase_stmt, customization_stmt] =
                map_corpse_load_statements_like_cpp(request);
            let mut corpse_result = match self.character_db.query(&corpse_stmt).await {
                Ok(result) => result,
                Err(error) => {
                    return MapCorpseLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            if corpse_result.is_empty() {
                return MapCorpseLoadOutcomeLikeCpp::Loaded {
                    corpses: Vec::new(),
                    phases: MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
                    customizations: MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
                };
            }

            let mut corpses = Vec::with_capacity(corpse_result.row_count_like_cpp());
            loop {
                corpses.push(MapCorpseLoadRowLikeCpp {
                    pos_x: corpse_result.try_read::<f32>(0).unwrap_or(f32::NAN),
                    pos_y: corpse_result.try_read::<f32>(1).unwrap_or(f32::NAN),
                    pos_z: corpse_result.try_read::<f32>(2).unwrap_or(f32::NAN),
                    orientation: corpse_result.try_read::<f32>(3).unwrap_or(f32::NAN),
                    map_id: corpse_result
                        .try_read::<u16>(4)
                        .unwrap_or(request.map_id as u16),
                    display_id: corpse_result.try_read::<u32>(5).unwrap_or(0),
                    item_cache: corpse_result.read_string(6),
                    race: corpse_result.try_read::<u8>(7).unwrap_or(0),
                    class: corpse_result.try_read::<u8>(8).unwrap_or(0),
                    sex: corpse_result.try_read::<u8>(9).unwrap_or(0),
                    flags: corpse_result.try_read::<u8>(10).unwrap_or(0),
                    dynamic_flags: corpse_result.try_read::<u8>(11).unwrap_or(0),
                    ghost_time: corpse_result.try_read::<u32>(12).unwrap_or(0),
                    corpse_type: corpse_result.try_read::<u8>(13).unwrap_or(u8::MAX),
                    instance_id: corpse_result
                        .try_read::<u32>(14)
                        .unwrap_or(request.instance_id),
                    owner_guid: corpse_result.try_read::<u64>(15).unwrap_or(0),
                });
                if !corpse_result.next_row() {
                    break;
                }
            }

            let phases = match self.character_db.query(&phase_stmt).await {
                Ok(mut result) => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(MapCorpsePhaseLoadRowLikeCpp {
                                owner_guid: result.try_read::<u64>(0).unwrap_or(0),
                                phase_id: result.try_read::<u32>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(rows)
                }
                Err(error) => MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            };

            let customizations = match self.character_db.query(&customization_stmt).await {
                Ok(mut result) => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(MapCorpseCustomizationLoadRowLikeCpp {
                                owner_guid: result.try_read::<u64>(0).unwrap_or(0),
                                option_id: result.try_read::<u32>(1).unwrap_or(0),
                                choice_id: result.try_read::<u32>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(rows)
                }
                Err(error) => MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            };

            MapCorpseLoadOutcomeLikeCpp::Loaded {
                corpses,
                phases,
                customizations,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqlParam;
    use crate::statements::StatementDef;

    #[test]
    fn map_corpse_request_maps_to_cpp_statement_order_and_exact_binds() {
        let statements = map_corpse_load_statements_like_cpp(MapCorpseLoadRequestLikeCpp {
            map_id: 571,
            instance_id: 9,
        });

        assert_eq!(
            statements.each_ref().map(|statement| statement.sql()),
            [
                CharStatements::SEL_CORPSES.sql(),
                CharStatements::SEL_CORPSE_PHASES.sql(),
                CharStatements::SEL_CORPSE_CUSTOMIZATIONS.sql(),
            ]
        );
        for statement in statements {
            assert_eq!(
                statement.params(),
                vec![SqlParam::U32(571), SqlParam::U32(9)]
            );
        }
    }
}

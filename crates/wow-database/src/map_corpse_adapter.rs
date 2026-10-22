// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! MariaDB adapter for canonical-map corpse hydration.

use std::sync::Arc;

use wow_persistence::{
    CorpsePersistenceMutationOutcomeLikeCpp, CorpseSaveRequestLikeCpp,
    MapCorpseAuxiliaryLoadOutcomeLikeCpp, MapCorpseCustomizationLoadRowLikeCpp,
    MapCorpseLoadOutcomeLikeCpp, MapCorpseLoadRequestLikeCpp, MapCorpseLoadRowLikeCpp,
    MapCorpsePersistencePortLikeCpp, MapCorpsePhaseLoadRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::CharacterDatabase;
use crate::SqlTransaction;
use crate::params::PreparedStatement;
use crate::statements::CharStatements;

/// C++ `Corpse::DeleteFromDB(ownerGuid, trans)`: corpse, phases and
/// customizations, in that order.
fn corpse_delete_statements_like_cpp(owner_guid: u64) -> [PreparedStatement; 3] {
    let mut corpse = PreparedStatement::for_statement(CharStatements::DEL_CORPSE);
    corpse.set_u64(0, owner_guid);
    let mut phases = PreparedStatement::for_statement(CharStatements::DEL_CORPSE_PHASES);
    phases.set_u64(0, owner_guid);
    let mut customizations =
        PreparedStatement::for_statement(CharStatements::DEL_CORPSE_CUSTOMIZATIONS);
    customizations.set_u64(0, owner_guid);
    [corpse, phases, customizations]
}

/// C++ `Corpse::SaveToDB`: `DeleteFromDB` first, then the corpse row, one
/// phase row per phase and one customization row per choice.
fn corpse_save_statements_like_cpp(request: &CorpseSaveRequestLikeCpp) -> Vec<PreparedStatement> {
    let mut statements = corpse_delete_statements_like_cpp(request.owner_guid).to_vec();

    let mut corpse = PreparedStatement::for_statement(CharStatements::INS_CORPSE);
    corpse.set_u64(0, request.owner_guid);
    corpse.set_f32(1, request.pos_x);
    corpse.set_f32(2, request.pos_y);
    corpse.set_f32(3, request.pos_z);
    corpse.set_f32(4, request.orientation);
    corpse.set_u16(5, request.map_id);
    corpse.set_u32(6, request.display_id);
    corpse.set_string(7, request.item_cache.clone());
    corpse.set_u8(8, request.race);
    corpse.set_u8(9, request.class);
    corpse.set_u8(10, request.sex);
    corpse.set_u8(11, request.flags);
    corpse.set_u8(12, request.dynamic_flags);
    corpse.set_u32(13, request.ghost_time);
    corpse.set_u8(14, request.corpse_type);
    corpse.set_u32(15, request.instance_id);
    statements.push(corpse);

    for phase_id in &request.phases {
        let mut phase = PreparedStatement::for_statement(CharStatements::INS_CORPSE_PHASES);
        phase.set_u64(0, request.owner_guid);
        phase.set_u32(1, *phase_id);
        statements.push(phase);
    }

    for (option_id, choice_id) in &request.customizations {
        let mut customization =
            PreparedStatement::for_statement(CharStatements::INS_CORPSE_CUSTOMIZATIONS);
        customization.set_u64(0, request.owner_guid);
        customization.set_u32(1, *option_id);
        customization.set_u32(2, *choice_id);
        statements.push(customization);
    }

    statements
}

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

    fn save_corpse_like_cpp<'a>(
        &'a self,
        request: CorpseSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, CorpsePersistenceMutationOutcomeLikeCpp> {
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            for statement in corpse_save_statements_like_cpp(&request) {
                transaction.append(statement);
            }
            match self.character_db.commit_transaction(transaction).await {
                Ok(()) => CorpsePersistenceMutationOutcomeLikeCpp::Committed,
                Err(error) => CorpsePersistenceMutationOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn delete_corpse_like_cpp<'a>(
        &'a self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<'a, CorpsePersistenceMutationOutcomeLikeCpp> {
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            for statement in corpse_delete_statements_like_cpp(owner_guid) {
                transaction.append(statement);
            }
            match self.character_db.commit_transaction(transaction).await {
                Ok(()) => CorpsePersistenceMutationOutcomeLikeCpp::Committed,
                Err(error) => CorpsePersistenceMutationOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
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

    #[test]
    fn corpse_save_runs_delete_then_inserts_in_cpp_order_and_binds() {
        let request = CorpseSaveRequestLikeCpp {
            owner_guid: 42,
            pos_x: 1.0,
            pos_y: 2.0,
            pos_z: 3.0,
            orientation: 0.5,
            map_id: 571,
            display_id: 49,
            item_cache: "0 1 ".into(),
            race: 1,
            class: 2,
            sex: 0,
            flags: 4,
            dynamic_flags: 0,
            ghost_time: 1_700_000_000,
            corpse_type: 1,
            instance_id: 0,
            phases: vec![169, 170],
            customizations: vec![(7, 8)],
        };

        let statements = corpse_save_statements_like_cpp(&request);
        assert_eq!(
            statements
                .iter()
                .map(|statement| statement.sql())
                .collect::<Vec<_>>(),
            vec![
                CharStatements::DEL_CORPSE.sql(),
                CharStatements::DEL_CORPSE_PHASES.sql(),
                CharStatements::DEL_CORPSE_CUSTOMIZATIONS.sql(),
                CharStatements::INS_CORPSE.sql(),
                CharStatements::INS_CORPSE_PHASES.sql(),
                CharStatements::INS_CORPSE_PHASES.sql(),
                CharStatements::INS_CORPSE_CUSTOMIZATIONS.sql(),
            ]
        );
        for statement in &statements[..3] {
            assert_eq!(statement.params(), vec![SqlParam::U64(42)]);
        }
        assert_eq!(
            statements[3].params(),
            vec![
                SqlParam::U64(42),
                SqlParam::F32(1.0),
                SqlParam::F32(2.0),
                SqlParam::F32(3.0),
                SqlParam::F32(0.5),
                SqlParam::U16(571),
                SqlParam::U32(49),
                SqlParam::String("0 1 ".into()),
                SqlParam::U8(1),
                SqlParam::U8(2),
                SqlParam::U8(0),
                SqlParam::U8(4),
                SqlParam::U8(0),
                SqlParam::U32(1_700_000_000),
                SqlParam::U8(1),
                SqlParam::U32(0),
            ]
        );
        assert_eq!(
            statements[4].params(),
            vec![SqlParam::U64(42), SqlParam::U32(169)]
        );
        assert_eq!(
            statements[6].params(),
            vec![SqlParam::U64(42), SqlParam::U32(7), SqlParam::U32(8)]
        );
    }
}

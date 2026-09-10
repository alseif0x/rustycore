//! MariaDB adapter for C++ `SpellMgr` World catalogs.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, SpellAreaPersistenceRowLikeCpp, SpellGroupPersistenceRowLikeCpp,
    SpellGroupStackRulePersistenceRowLikeCpp, SpellLinkedPersistenceRowLikeCpp,
    SpellPetAuraPersistenceRowLikeCpp, SpellProcPersistenceRowLikeCpp,
    SpellRequiredPersistenceRowLikeCpp, SpellTargetPositionPersistenceRowLikeCpp,
    SpellThreatPersistenceRowLikeCpp, SpellTotemModelPersistenceRowLikeCpp,
    SpellWorldCatalogLoadOutcomeLikeCpp, SpellWorldCatalogPersistencePortLikeCpp,
};

use crate::{DatabaseError, SqlResult, WorldDatabase, WorldStatements};

const SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP: [WorldStatements; 10] = [
    WorldStatements::SEL_SPELL_AREA,
    WorldStatements::SEL_SPELL_TARGET_POSITION,
    WorldStatements::SEL_SPELL_PROC,
    WorldStatements::SEL_SPELL_REQUIRED,
    WorldStatements::SEL_SPELL_GROUP,
    WorldStatements::SEL_SPELL_GROUP_STACK_RULES,
    WorldStatements::SEL_SPELL_THREATS,
    WorldStatements::SEL_SPELL_LINKED,
    WorldStatements::SEL_SPELL_TOTEM_MODEL,
    WorldStatements::SEL_SPELL_PET_AURAS,
];

async fn query_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> T,
) -> Result<Vec<T>, DatabaseError> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::new();
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(decode(&result));
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn classify_rows_like_cpp<T>(
    result: Result<Vec<T>, DatabaseError>,
) -> SpellWorldCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => SpellWorldCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => SpellWorldCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

fn required_row_like_cpp(values: (u32, u32)) -> SpellRequiredPersistenceRowLikeCpp {
    SpellRequiredPersistenceRowLikeCpp {
        spell_id: values.0,
        req_spell: values.1,
    }
}

fn threat_row_like_cpp(values: (u32, i32, f32, f32)) -> SpellThreatPersistenceRowLikeCpp {
    SpellThreatPersistenceRowLikeCpp {
        spell_id: values.0,
        flat_mod: values.1,
        pct_mod: values.2,
        ap_pct_mod: values.3,
    }
}

fn linked_row_like_cpp(values: (i32, i32, u8)) -> SpellLinkedPersistenceRowLikeCpp {
    SpellLinkedPersistenceRowLikeCpp {
        spell_trigger: values.0,
        spell_effect: values.1,
        link_type: values.2,
    }
}

fn totem_model_row_like_cpp(values: (u32, u8, u32)) -> SpellTotemModelPersistenceRowLikeCpp {
    SpellTotemModelPersistenceRowLikeCpp {
        spell_id: values.0,
        race_id: values.1,
        display_id: values.2,
    }
}

fn pet_aura_row_like_cpp(values: (u32, u8, u32, u32)) -> SpellPetAuraPersistenceRowLikeCpp {
    SpellPetAuraPersistenceRowLikeCpp {
        spell_id: values.0,
        effect_index: values.1,
        pet_entry: values.2,
        aura_id: values.3,
    }
}

fn area_row_like_cpp(
    values: (u32, u32, u32, u32, u32, u32, i32, u64, u8, u8),
) -> SpellAreaPersistenceRowLikeCpp {
    SpellAreaPersistenceRowLikeCpp {
        spell_id: values.0,
        area_id: values.1,
        quest_start: values.2,
        quest_start_status: values.3,
        quest_end_status: values.4,
        quest_end: values.5,
        aura_spell: values.6,
        race_mask: values.7,
        gender: values.8,
        flags: values.9,
    }
}

fn group_row_like_cpp(values: (u32, i32)) -> SpellGroupPersistenceRowLikeCpp {
    SpellGroupPersistenceRowLikeCpp {
        group_id: values.0,
        spell_id: values.1,
    }
}

fn group_stack_rule_row_like_cpp(values: (u32, u8)) -> SpellGroupStackRulePersistenceRowLikeCpp {
    SpellGroupStackRulePersistenceRowLikeCpp {
        group_id: values.0,
        stack_rule: values.1,
    }
}

fn target_position_row_like_cpp(
    values: (u32, u8, u16, f32, f32, f32, Option<f32>),
) -> SpellTargetPositionPersistenceRowLikeCpp {
    SpellTargetPositionPersistenceRowLikeCpp {
        spell_id: values.0,
        effect_index: u32::from(values.1),
        target_map_id: values.2,
        x: values.3,
        y: values.4,
        z: values.5,
        orientation: values.6,
    }
}

fn proc_row_like_cpp(row: &SqlResult) -> SpellProcPersistenceRowLikeCpp {
    SpellProcPersistenceRowLikeCpp {
        spell_id: row.try_read(0).unwrap_or(0),
        school_mask: row.try_read(1).unwrap_or(0),
        spell_family_name: row.try_read(2).unwrap_or(0),
        spell_family_mask: [
            row.try_read(3).unwrap_or(0),
            row.try_read(4).unwrap_or(0),
            row.try_read(5).unwrap_or(0),
            row.try_read(6).unwrap_or(0),
        ],
        proc_flags: [row.try_read(7).unwrap_or(0), row.try_read(8).unwrap_or(0)],
        spell_type_mask: row.try_read(9).unwrap_or(0),
        spell_phase_mask: row.try_read(10).unwrap_or(0),
        hit_mask: row.try_read(11).unwrap_or(0),
        attributes_mask: row.try_read(12).unwrap_or(0),
        disable_effects_mask: row.try_read(13).unwrap_or(0),
        procs_per_minute: row.try_read(14).unwrap_or(0.0),
        chance: row.try_read(15).unwrap_or(0.0),
        cooldown_ms: row.try_read(16).unwrap_or(0),
        charges: row.try_read(17).unwrap_or(0),
    }
}

pub struct MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl SpellWorldCatalogPersistencePortLikeCpp for MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp {
    fn load_spell_area_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellAreaPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[0],
                    |row| {
                        area_row_like_cpp((
                            row.try_read(0).unwrap_or(0),
                            row.try_read(1).unwrap_or(0),
                            row.try_read(2).unwrap_or(0),
                            row.try_read(3).unwrap_or(0),
                            row.try_read(4).unwrap_or(0),
                            row.try_read(5).unwrap_or(0),
                            row.try_read(6).unwrap_or(0),
                            row.try_read(7).unwrap_or(0),
                            row.try_read(8).unwrap_or(2),
                            row.try_read(9).unwrap_or(0),
                        ))
                    },
                )
                .await,
            )
        })
    }

    fn load_spell_target_position_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellTargetPositionPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[1],
                    |row| {
                        target_position_row_like_cpp((
                            row.try_read(0).unwrap_or(0),
                            row.try_read(1).unwrap_or(0),
                            row.try_read(2).unwrap_or(0),
                            row.try_read(3).unwrap_or(0.0),
                            row.try_read(4).unwrap_or(0.0),
                            row.try_read(5).unwrap_or(0.0),
                            row.try_read::<Option<f32>>(6).unwrap_or(None),
                        ))
                    },
                )
                .await,
            )
        })
    }

    fn load_spell_proc_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellProcPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[2],
                    proc_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_spell_required_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellRequiredPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[3],
                    |row| {
                        required_row_like_cpp((
                            row.try_read(0).unwrap_or(0),
                            row.try_read(1).unwrap_or(0),
                        ))
                    },
                )
                .await,
            )
        })
    }

    fn load_spell_group_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellGroupPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[4],
                    |row| {
                        group_row_like_cpp((
                            row.try_read(0).unwrap_or(0),
                            row.try_read(1).unwrap_or(0),
                        ))
                    },
                )
                .await,
            )
        })
    }

    fn load_spell_group_stack_rule_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellGroupStackRulePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[5],
                    |row| {
                        group_stack_rule_row_like_cpp((
                            row.try_read(0).unwrap_or(0),
                            row.try_read(1).unwrap_or(0),
                        ))
                    },
                )
                .await,
            )
        })
    }

    fn load_spell_threat_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellThreatPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[6],
                    |row| {
                        threat_row_like_cpp((
                            row.try_read(0).unwrap_or(0),
                            row.try_read(1).unwrap_or(0),
                            row.try_read(2).unwrap_or(0.0),
                            row.try_read(3).unwrap_or(0.0),
                        ))
                    },
                )
                .await,
            )
        })
    }

    fn load_spell_linked_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellLinkedPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[7],
                    |row| {
                        linked_row_like_cpp((
                            row.try_read(0).unwrap_or(0),
                            row.try_read(1).unwrap_or(0),
                            row.try_read(2).unwrap_or(0),
                        ))
                    },
                )
                .await,
            )
        })
    }

    fn load_spell_totem_model_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellTotemModelPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[8],
                    |row| {
                        totem_model_row_like_cpp((
                            row.try_read(0).unwrap_or(0),
                            row.try_read(1).unwrap_or(0),
                            row.try_read(2).unwrap_or(0),
                        ))
                    },
                )
                .await,
            )
        })
    }

    fn load_spell_pet_aura_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellPetAuraPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP[9],
                    |row| {
                        pet_aura_row_like_cpp((
                            row.try_read(0).unwrap_or(0),
                            row.try_read(1).unwrap_or(0),
                            row.try_read(2).unwrap_or(0),
                            row.try_read(3).unwrap_or(0),
                        ))
                    },
                )
                .await,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn spell_world_catalog_statement_manifest_matches_cpp_queries() {
        assert_eq!(
            SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_SPELL_AREA,
                WorldStatements::SEL_SPELL_TARGET_POSITION,
                WorldStatements::SEL_SPELL_PROC,
                WorldStatements::SEL_SPELL_REQUIRED,
                WorldStatements::SEL_SPELL_GROUP,
                WorldStatements::SEL_SPELL_GROUP_STACK_RULES,
                WorldStatements::SEL_SPELL_THREATS,
                WorldStatements::SEL_SPELL_LINKED,
                WorldStatements::SEL_SPELL_TOTEM_MODEL,
                WorldStatements::SEL_SPELL_PET_AURAS,
            ]
        );
        assert_eq!(
            SPELL_WORLD_CATALOG_STATEMENTS_LIKE_CPP.map(WorldStatements::sql),
            [
                "SELECT spell, area, quest_start, quest_start_status, quest_end_status, quest_end, aura_spell, racemask, gender, flags FROM spell_area",
                "SELECT ID, EffectIndex, MapID, PositionX, PositionY, PositionZ, Orientation FROM spell_target_position",
                "SELECT SpellId, SchoolMask, SpellFamilyName, SpellFamilyMask0, SpellFamilyMask1, SpellFamilyMask2, SpellFamilyMask3, ProcFlags, ProcFlags2, SpellTypeMask, SpellPhaseMask, HitMask, AttributesMask, DisableEffectsMask, ProcsPerMinute, Chance, Cooldown, Charges FROM spell_proc",
                "SELECT spell_id, req_spell from spell_required",
                "SELECT id, spell_id FROM spell_group",
                "SELECT group_id, stack_rule FROM spell_group_stack_rules",
                "SELECT entry, flatMod, pctMod, apPctMod FROM spell_threat",
                "SELECT spell_trigger, spell_effect, type FROM spell_linked_spell",
                "SELECT SpellID, RaceID, DisplayID from spell_totem_model",
                "SELECT spell, effectId, pet, aura FROM spell_pet_auras",
            ]
        );
    }

    #[test]
    fn successful_empty_query_keeps_loaded_classification() {
        assert_eq!(
            classify_rows_like_cpp::<u8>(Ok(Vec::new())),
            SpellWorldCatalogLoadOutcomeLikeCpp::Loaded(Vec::new())
        );
    }

    #[test]
    fn every_statement_column_maps_to_the_typed_contract() {
        assert_eq!(
            area_row_like_cpp((1, 2, 3, 4, 5, 6, -7, 8, 2, 9)),
            SpellAreaPersistenceRowLikeCpp {
                spell_id: 1,
                area_id: 2,
                quest_start: 3,
                quest_start_status: 4,
                quest_end_status: 5,
                quest_end: 6,
                aura_spell: -7,
                race_mask: 8,
                gender: 2,
                flags: 9,
            }
        );
        assert_eq!(
            required_row_like_cpp((1, 2)),
            SpellRequiredPersistenceRowLikeCpp {
                spell_id: 1,
                req_spell: 2,
            }
        );
        assert_eq!(
            threat_row_like_cpp((3, -4, 5.5, 6.5)),
            SpellThreatPersistenceRowLikeCpp {
                spell_id: 3,
                flat_mod: -4,
                pct_mod: 5.5,
                ap_pct_mod: 6.5,
            }
        );
        assert_eq!(
            linked_row_like_cpp((-7, 8, 2)),
            SpellLinkedPersistenceRowLikeCpp {
                spell_trigger: -7,
                spell_effect: 8,
                link_type: 2,
            }
        );
        assert_eq!(
            totem_model_row_like_cpp((9, 10, 11)),
            SpellTotemModelPersistenceRowLikeCpp {
                spell_id: 9,
                race_id: 10,
                display_id: 11,
            }
        );
        assert_eq!(
            pet_aura_row_like_cpp((12, 13, 14, 15)),
            SpellPetAuraPersistenceRowLikeCpp {
                spell_id: 12,
                effect_index: 13,
                pet_entry: 14,
                aura_id: 15,
            }
        );
        assert_eq!(
            group_row_like_cpp((16, -17)),
            SpellGroupPersistenceRowLikeCpp {
                group_id: 16,
                spell_id: -17,
            }
        );
        assert_eq!(
            group_stack_rule_row_like_cpp((18, 3)),
            SpellGroupStackRulePersistenceRowLikeCpp {
                group_id: 18,
                stack_rule: 3,
            }
        );
        assert_eq!(
            target_position_row_like_cpp((19, 2, 20, 1.0, 2.0, 3.0, Some(4.0))),
            SpellTargetPositionPersistenceRowLikeCpp {
                spell_id: 19,
                effect_index: 2,
                target_map_id: 20,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: Some(4.0),
            }
        );
    }
}

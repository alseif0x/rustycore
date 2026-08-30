//! MariaDB adapter for foundational C++ `SpellMgr` World catalogs.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, SpellLinkedPersistenceRowLikeCpp, SpellPetAuraPersistenceRowLikeCpp,
    SpellRequiredPersistenceRowLikeCpp, SpellThreatPersistenceRowLikeCpp,
    SpellTotemModelPersistenceRowLikeCpp, SpellWorldCatalogLoadOutcomeLikeCpp,
    SpellWorldCatalogPersistencePortLikeCpp,
};

use crate::{DatabaseError, SqlResult, WorldDatabase, WorldStatements};

const FOUNDATIONAL_SPELL_STATEMENTS_LIKE_CPP: [WorldStatements; 5] = [
    WorldStatements::SEL_SPELL_REQUIRED,
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

pub struct MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl SpellWorldCatalogPersistencePortLikeCpp for MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp {
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
                    FOUNDATIONAL_SPELL_STATEMENTS_LIKE_CPP[0],
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
                    FOUNDATIONAL_SPELL_STATEMENTS_LIKE_CPP[1],
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
                    FOUNDATIONAL_SPELL_STATEMENTS_LIKE_CPP[2],
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
                    FOUNDATIONAL_SPELL_STATEMENTS_LIKE_CPP[3],
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
                    FOUNDATIONAL_SPELL_STATEMENTS_LIKE_CPP[4],
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
    fn foundational_spell_statement_manifest_matches_cpp_queries() {
        assert_eq!(
            FOUNDATIONAL_SPELL_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_SPELL_REQUIRED,
                WorldStatements::SEL_SPELL_THREATS,
                WorldStatements::SEL_SPELL_LINKED,
                WorldStatements::SEL_SPELL_TOTEM_MODEL,
                WorldStatements::SEL_SPELL_PET_AURAS,
            ]
        );
        assert_eq!(
            FOUNDATIONAL_SPELL_STATEMENTS_LIKE_CPP.map(WorldStatements::sql),
            [
                "SELECT spell_id, req_spell from spell_required",
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
    }
}

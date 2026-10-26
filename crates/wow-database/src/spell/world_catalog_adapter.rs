//! MariaDB adapter for C++ `SpellMgr` World catalogs.

use std::sync::Arc;

use anyhow::{Context, Result as AnyResult};

use wow_persistence::{
    PersistenceFutureLikeCpp, SpellAreaPersistenceRowLikeCpp, SpellGroupPersistenceRowLikeCpp,
    SpellGroupStackRulePersistenceRowLikeCpp, SpellLinkedPersistenceRowLikeCpp,
    SpellPetAuraPersistenceRowLikeCpp, SpellProcPersistenceRowLikeCpp,
    SpellRequiredPersistenceRowLikeCpp, SpellTargetPositionPersistenceRowLikeCpp,
    SpellThreatPersistenceRowLikeCpp, SpellTotemModelPersistenceRowLikeCpp,
    SpellWorldCatalogLoadOutcomeLikeCpp, SpellWorldCatalogPersistencePortLikeCpp,
};

use crate::field_like_cpp::{FieldRowLikeCpp, NativeFieldLikeCpp};
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

fn proc_row_like_cpp(row: &[NativeFieldLikeCpp]) -> AnyResult<SpellProcPersistenceRowLikeCpp> {
    Ok(SpellProcPersistenceRowLikeCpp {
        spell_id: row.get_like_cpp(0, "spell_proc.SpellID")?,
        school_mask: row.get_like_cpp(1, "spell_proc.SchoolMask")?,
        spell_family_name: row.get_like_cpp(2, "spell_proc.SpellFamilyName")?,
        spell_family_mask: [
            row.get_like_cpp(3, "spell_proc.SpellFamilyMask1")?,
            row.get_like_cpp(4, "spell_proc.SpellFamilyMask2")?,
            row.get_like_cpp(5, "spell_proc.SpellFamilyMask3")?,
            row.get_like_cpp(6, "spell_proc.SpellFamilyMask4")?,
        ],
        proc_flags: [
            row.get_like_cpp(7, "spell_proc.ProcFlags")?,
            row.get_like_cpp(8, "spell_proc.ProcFlags2")?,
        ],
        spell_type_mask: row.get_like_cpp(9, "spell_proc.SpellTypeMask")?,
        spell_phase_mask: row.get_like_cpp(10, "spell_proc.SpellPhaseMask")?,
        hit_mask: row.get_like_cpp(11, "spell_proc.HitMask")?,
        attributes_mask: row.get_like_cpp(12, "spell_proc.AttributesMask")?,
        disable_effects_mask: row.get_like_cpp(13, "spell_proc.DisableEffectsMask")?,
        procs_per_minute: row.get_like_cpp(14, "spell_proc.ProcsPerMinute")?,
        chance: row.get_like_cpp(15, "spell_proc.Chance")?,
        cooldown_ms: row.get_like_cpp(16, "spell_proc.Cooldown")?,
        charges: row.get_like_cpp(17, "spell_proc.Charges")?,
    })
}

async fn query_checked_proc_rows(db: &WorldDatabase) -> AnyResult<Vec<SpellProcPersistenceRowLikeCpp>> {
    let mut result = db
        .query(&db.prepare(WorldStatements::SEL_SPELL_PROC))
        .await
        .context("query spell_proc")?;
    let mut rows = Vec::with_capacity(result.count());
    if result.is_empty() {
        return Ok(rows);
    }
    let mut row_index = 0;
    loop {
        let native = result
            .native_row_like_cpp()
            .with_context(|| format!("spell_proc row {row_index} native fields"))?;
        append_checked_proc(&mut rows, row_index, &native)?;
        row_index += 1;
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn append_checked_proc(
    rows: &mut Vec<SpellProcPersistenceRowLikeCpp>,
    row_index: usize,
    native: &[NativeFieldLikeCpp],
) -> AnyResult<()> {
    rows.push(proc_row_like_cpp(native).with_context(|| format!("spell_proc row {row_index}"))?);
    Ok(())
}

fn classify_checked_rows<T>(
    result: AnyResult<Vec<T>>,
) -> SpellWorldCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => SpellWorldCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => SpellWorldCatalogLoadOutcomeLikeCpp::Failed {
            reason: format!("{error:#}"),
        },
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
            classify_checked_rows(query_checked_proc_rows(&self.world_db).await)
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
    use wow_persistence::SpellWorldCatalogLoadOutcomeLikeCpp;
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

    #[test]
    fn proc_row_uses_exact_widths_and_preserves_full_payloads() {
        let mut fields = vec![NativeFieldLikeCpp::UInt32(0); 18];
        fields[0] = NativeFieldLikeCpp::Int32(-1);
        fields[1] = NativeFieldLikeCpp::UInt8(u8::MAX);
        fields[2] = NativeFieldLikeCpp::UInt16(u16::MAX);
        fields[3] = NativeFieldLikeCpp::UInt32(u32::MAX);
        fields[4] = NativeFieldLikeCpp::UInt32(0x8000_0000);
        fields[5] = NativeFieldLikeCpp::UInt32(0x1234_5678);
        fields[6] = NativeFieldLikeCpp::UInt32(0xABCD_EF01);
        fields[7] = NativeFieldLikeCpp::UInt32(u32::MAX);
        fields[8] = NativeFieldLikeCpp::UInt32(0x8000_0000);
        fields[14] = NativeFieldLikeCpp::Float(1.25);
        fields[15] = NativeFieldLikeCpp::Float(0.125);
        fields[16] = NativeFieldLikeCpp::UInt32(u32::MAX);
        fields[17] = NativeFieldLikeCpp::UInt8(u8::MAX);

        let decoded = proc_row_like_cpp(&fields).unwrap();
        assert_eq!(decoded.spell_id, -1);
        assert_eq!(decoded.school_mask, u8::MAX);
        assert_eq!(decoded.spell_family_name, u16::MAX);
        assert_eq!(
            decoded.spell_family_mask,
            [u32::MAX, 0x8000_0000, 0x1234_5678, 0xABCD_EF01]
        );
        assert_eq!(decoded.proc_flags, [u32::MAX, 0x8000_0000]);
        assert_eq!(decoded.procs_per_minute, 1.25);
        assert_eq!(decoded.chance, 0.125);
        assert_eq!(decoded.cooldown_ms, u32::MAX);
        assert_eq!(decoded.charges, u8::MAX);

        let null = proc_row_like_cpp(&vec![NativeFieldLikeCpp::Null; 18]).unwrap();
        assert_eq!(null.spell_id, 0);
        assert_eq!(null.spell_family_mask, [0; 4]);
        assert_eq!(null.procs_per_minute, 0.0);
        assert_eq!(null.charges, 0);

        fields[2] = NativeFieldLikeCpp::UInt32(65_536);
        assert!(proc_row_like_cpp(&fields).is_err());
        fields[2] = NativeFieldLikeCpp::UInt16(0);
        fields[17] = NativeFieldLikeCpp::UInt16(256);
        assert!(proc_row_like_cpp(&fields).is_err());
        fields[17] = NativeFieldLikeCpp::UInt8(0);
        fields[0] = NativeFieldLikeCpp::UInt64(u64::from(u32::MAX));
        assert!(proc_row_like_cpp(&fields).is_err());
    }

    #[test]
    fn malformed_proc_row_fails_whole_batch_with_field_context() {
        let valid = vec![NativeFieldLikeCpp::UInt32(0); 18];
        let mut malformed = vec![NativeFieldLikeCpp::UInt32(0); 18];
        malformed[17] = NativeFieldLikeCpp::UInt16(256);
        let mut rows = Vec::new();
        append_checked_proc(&mut rows, 0, &valid).unwrap();
        let error = append_checked_proc(&mut rows, 1, &malformed).unwrap_err();
        let reason = match classify_checked_rows::<SpellProcPersistenceRowLikeCpp>(Err(error)) {
            SpellWorldCatalogLoadOutcomeLikeCpp::Failed { reason } => reason,
            SpellWorldCatalogLoadOutcomeLikeCpp::Loaded(rows) => {
                panic!("malformed proc row unexpectedly loaded {} rows", rows.len())
            }
        };
        assert!(reason.contains("spell_proc row 1"), "{reason}");
        assert!(reason.contains("spell_proc.Charges"), "{reason}");
        assert!(reason.contains("GetUInt8"), "{reason}");
    }
}

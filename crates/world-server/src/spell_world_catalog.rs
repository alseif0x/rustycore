//! Composition boundary for foundational C++ `SpellMgr` World catalogs.

use anyhow::{Result, bail};
use wow_persistence::{
    SpellAreaPersistenceRowLikeCpp, SpellGroupPersistenceRowLikeCpp,
    SpellGroupStackRulePersistenceRowLikeCpp, SpellLinkedPersistenceRowLikeCpp,
    SpellPetAuraPersistenceRowLikeCpp, SpellProcPersistenceRowLikeCpp,
    SpellRequiredPersistenceRowLikeCpp, SpellTargetPositionPersistenceRowLikeCpp,
    SpellThreatPersistenceRowLikeCpp, SpellTotemModelPersistenceRowLikeCpp,
    SpellWorldCatalogLoadOutcomeLikeCpp, SpellWorldCatalogPersistencePortLikeCpp,
};

fn loaded_rows_like_cpp<T>(outcome: SpellWorldCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        SpellWorldCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        SpellWorldCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

fn required_row_like_cpp(
    row: SpellRequiredPersistenceRowLikeCpp,
) -> wow_data::SpellRequiredRowLikeCpp {
    wow_data::SpellRequiredRowLikeCpp {
        spell_id: row.spell_id,
        req_spell: row.req_spell,
    }
}

fn threat_row_like_cpp(row: SpellThreatPersistenceRowLikeCpp) -> wow_data::SpellThreatRowLikeCpp {
    wow_data::SpellThreatRowLikeCpp {
        spell_id: row.spell_id,
        flat_mod: row.flat_mod,
        pct_mod: row.pct_mod,
        ap_pct_mod: row.ap_pct_mod,
    }
}

fn linked_row_like_cpp(row: SpellLinkedPersistenceRowLikeCpp) -> wow_data::SpellLinkedRowLikeCpp {
    wow_data::SpellLinkedRowLikeCpp {
        spell_trigger: row.spell_trigger,
        spell_effect: row.spell_effect,
        link_type: row.link_type,
    }
}

fn totem_model_row_like_cpp(
    row: SpellTotemModelPersistenceRowLikeCpp,
) -> wow_data::SpellTotemModelRowLikeCpp {
    wow_data::SpellTotemModelRowLikeCpp {
        spell_id: row.spell_id,
        race_id: row.race_id,
        display_id: row.display_id,
    }
}

fn pet_aura_row_like_cpp(
    row: SpellPetAuraPersistenceRowLikeCpp,
) -> wow_data::SpellPetAuraRowLikeCpp {
    wow_data::SpellPetAuraRowLikeCpp {
        spell_id: row.spell_id,
        effect_index: row.effect_index,
        pet_entry: row.pet_entry,
        aura_id: row.aura_id,
    }
}

fn area_row_like_cpp(row: SpellAreaPersistenceRowLikeCpp) -> wow_data::SpellAreaRowLikeCpp {
    wow_data::SpellAreaRowLikeCpp {
        spell_id: row.spell_id,
        area_id: row.area_id,
        quest_start: row.quest_start,
        quest_start_status: row.quest_start_status,
        quest_end_status: row.quest_end_status,
        quest_end: row.quest_end,
        aura_spell: row.aura_spell,
        race_mask: row.race_mask,
        gender: row.gender,
        flags: row.flags,
    }
}

fn group_row_like_cpp(row: SpellGroupPersistenceRowLikeCpp) -> wow_data::SpellGroupRowLikeCpp {
    wow_data::SpellGroupRowLikeCpp {
        group_id: row.group_id,
        spell_id: row.spell_id,
    }
}

fn group_stack_rule_row_like_cpp(
    row: SpellGroupStackRulePersistenceRowLikeCpp,
) -> wow_data::SpellGroupStackRuleRowLikeCpp {
    wow_data::SpellGroupStackRuleRowLikeCpp {
        group_id: row.group_id,
        stack_rule: row.stack_rule,
    }
}

fn target_position_row_like_cpp(
    row: SpellTargetPositionPersistenceRowLikeCpp,
) -> wow_data::SpellTargetPositionRowLikeCpp {
    wow_data::SpellTargetPositionRowLikeCpp {
        spell_id: row.spell_id,
        effect_index: row.effect_index,
        target_map_id: row.target_map_id,
        x: row.x,
        y: row.y,
        z: row.z,
        orientation: row.orientation,
    }
}

fn proc_row_like_cpp(row: SpellProcPersistenceRowLikeCpp) -> wow_data::SpellProcRowLikeCpp {
    wow_data::SpellProcRowLikeCpp {
        spell_id: row.spell_id,
        school_mask: row.school_mask,
        spell_family_name: row.spell_family_name,
        spell_family_mask: row.spell_family_mask,
        proc_flags: row.proc_flags,
        spell_type_mask: row.spell_type_mask,
        spell_phase_mask: row.spell_phase_mask,
        hit_mask: row.hit_mask,
        attributes_mask: row.attributes_mask,
        disable_effects_mask: row.disable_effects_mask,
        procs_per_minute: row.procs_per_minute,
        chance: row.chance,
        cooldown_ms: row.cooldown_ms,
        charges: row.charges,
    }
}

pub(super) async fn load_spell_area_like_cpp<SpellExists, AreaExists, QuestExists>(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spell_exists: SpellExists,
    area_exists: AreaExists,
    quest_exists: QuestExists,
) -> Result<wow_data::SpellAreaLoadOutcomeLikeCpp>
where
    SpellExists: FnMut(u32) -> bool,
    AreaExists: FnMut(u32) -> bool,
    QuestExists: FnMut(u32) -> bool,
{
    let rows = loaded_rows_like_cpp(persistence.load_spell_area_rows_like_cpp().await)?;
    Ok(wow_data::SpellAreaStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter().map(area_row_like_cpp),
        spell_exists,
        area_exists,
        quest_exists,
    ))
}

pub(super) async fn load_spell_target_position_like_cpp<MapExists>(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spells: &wow_data::SpellStore,
    map_exists: MapExists,
) -> Result<wow_data::SpellTargetPositionStoreLikeCpp>
where
    MapExists: FnMut(u16) -> bool,
{
    let rows = loaded_rows_like_cpp(persistence.load_spell_target_position_rows_like_cpp().await)?;
    Ok(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            rows.into_iter().map(target_position_row_like_cpp),
            spells,
            map_exists,
        ),
    )
}

pub(super) async fn load_spell_proc_like_cpp(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spells: &wow_data::SpellStore,
    spell_chains: &wow_data::SpellChainStoreLikeCpp,
    spell_aura_options: &wow_data::SpellAuraOptionsStore,
    spell_misc: &wow_data::SpellMiscStore,
    spell_class_options: &wow_data::SpellClassOptionsStore,
    spell_procs_per_minute: &wow_data::SpellProcsPerMinuteStore,
) -> Result<wow_data::SpellProcLoadOutcomeLikeCpp> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_proc_rows_like_cpp().await)?;
    Ok(
        wow_data::SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
            rows.into_iter().map(proc_row_like_cpp),
            spells,
            spell_chains,
            spell_aura_options,
            spell_misc,
            spell_class_options,
            spell_procs_per_minute,
        ),
    )
}

pub(super) async fn load_spell_group_like_cpp(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spells: &wow_data::SpellStore,
    spell_chains: &wow_data::SpellChainStoreLikeCpp,
) -> Result<wow_data::SpellGroupLoadOutcomeLikeCpp> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_group_rows_like_cpp().await)?;
    Ok(wow_data::SpellGroupStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter().map(group_row_like_cpp),
        |spell_id| spells.get(spell_id as i32).is_some(),
        |spell_id| u32::from(spell_chains.spell_rank_like_cpp(spell_id)),
    ))
}

pub(super) async fn load_spell_group_stack_rule_like_cpp(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spell_groups: &wow_data::SpellGroupStoreLikeCpp,
    spells: &wow_data::SpellStore,
    spell_chains: &wow_data::SpellChainStoreLikeCpp,
) -> Result<wow_data::SpellGroupStackRuleLoadOutcomeLikeCpp> {
    let rows = loaded_rows_like_cpp(
        persistence
            .load_spell_group_stack_rule_rows_like_cpp()
            .await,
    )?;
    Ok(
        wow_data::SpellGroupStackRuleStoreLikeCpp::from_rows_like_cpp(
            rows.into_iter().map(group_stack_rule_row_like_cpp),
            spell_groups,
            |spell_id| spells.get(spell_id as i32).cloned(),
            |spell_id| {
                let next_spell_id = spell_chains.next_spell_in_chain_like_cpp(spell_id);
                (next_spell_id != 0).then_some(next_spell_id)
            },
        ),
    )
}

pub(super) async fn load_spell_required_like_cpp(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spells: &wow_data::SpellStore,
    spell_chains: &wow_data::SpellChainStoreLikeCpp,
) -> Result<wow_data::SpellRequiredLoadOutcomeLikeCpp> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_required_rows_like_cpp().await)?;
    Ok(
        wow_data::SpellRequiredStoreLikeCpp::from_rows_and_stores_like_cpp(
            rows.into_iter().map(required_row_like_cpp),
            spells,
            spell_chains,
        ),
    )
}

pub(super) async fn load_spell_threat_like_cpp(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spells: &wow_data::SpellStore,
) -> Result<wow_data::SpellThreatLoadOutcomeLikeCpp> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_threat_rows_like_cpp().await)?;
    Ok(
        wow_data::SpellThreatStoreLikeCpp::from_rows_and_spell_store_like_cpp(
            rows.into_iter().map(threat_row_like_cpp),
            spells,
        ),
    )
}

pub(super) async fn load_spell_linked_like_cpp(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spells: &wow_data::SpellStore,
) -> Result<wow_data::SpellLinkedLoadOutcomeLikeCpp> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_linked_rows_like_cpp().await)?;
    Ok(
        wow_data::SpellLinkedStoreLikeCpp::from_rows_and_spell_store_like_cpp(
            rows.into_iter().map(linked_row_like_cpp),
            spells,
        ),
    )
}

pub(super) async fn load_spell_totem_model_like_cpp<SpellExists, RaceExists, DisplayExists>(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spell_exists: SpellExists,
    race_exists: RaceExists,
    display_exists: DisplayExists,
) -> Result<wow_data::SpellTotemModelLoadOutcomeLikeCpp>
where
    SpellExists: FnMut(u32) -> bool,
    RaceExists: FnMut(u8) -> bool,
    DisplayExists: FnMut(u32) -> bool,
{
    let rows = loaded_rows_like_cpp(persistence.load_spell_totem_model_rows_like_cpp().await)?;
    Ok(
        wow_data::SpellTotemModelStoreLikeCpp::from_rows_and_stores_like_cpp(
            rows.into_iter().map(totem_model_row_like_cpp),
            spell_exists,
            race_exists,
            display_exists,
        ),
    )
}

pub(super) async fn load_spell_pet_aura_like_cpp(
    persistence: &dyn SpellWorldCatalogPersistencePortLikeCpp,
    spells: &wow_data::SpellStore,
) -> Result<wow_data::SpellPetAuraLoadOutcomeLikeCpp> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_pet_aura_rows_like_cpp().await)?;
    Ok(
        wow_data::SpellPetAuraStoreLikeCpp::from_rows_and_spell_store_like_cpp(
            rows.into_iter().map(pet_aura_row_like_cpp),
            spells,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_typed_boundary_row_preserves_its_fields() {
        assert_eq!(
            required_row_like_cpp(SpellRequiredPersistenceRowLikeCpp {
                spell_id: 7,
                req_spell: 8,
            }),
            wow_data::SpellRequiredRowLikeCpp {
                spell_id: 7,
                req_spell: 8,
            }
        );
        assert_eq!(
            threat_row_like_cpp(SpellThreatPersistenceRowLikeCpp {
                spell_id: 9,
                flat_mod: -10,
                pct_mod: 1.25,
                ap_pct_mod: 2.5,
            }),
            wow_data::SpellThreatRowLikeCpp {
                spell_id: 9,
                flat_mod: -10,
                pct_mod: 1.25,
                ap_pct_mod: 2.5,
            }
        );
        assert_eq!(
            linked_row_like_cpp(SpellLinkedPersistenceRowLikeCpp {
                spell_trigger: -11,
                spell_effect: 12,
                link_type: 3,
            }),
            wow_data::SpellLinkedRowLikeCpp {
                spell_trigger: -11,
                spell_effect: 12,
                link_type: 3,
            }
        );
        assert_eq!(
            totem_model_row_like_cpp(SpellTotemModelPersistenceRowLikeCpp {
                spell_id: 13,
                race_id: 14,
                display_id: 15,
            }),
            wow_data::SpellTotemModelRowLikeCpp {
                spell_id: 13,
                race_id: 14,
                display_id: 15,
            }
        );
        assert_eq!(
            pet_aura_row_like_cpp(SpellPetAuraPersistenceRowLikeCpp {
                spell_id: 16,
                effect_index: 17,
                pet_entry: 18,
                aura_id: 19,
            }),
            wow_data::SpellPetAuraRowLikeCpp {
                spell_id: 16,
                effect_index: 17,
                pet_entry: 18,
                aura_id: 19,
            }
        );
        assert_eq!(
            area_row_like_cpp(SpellAreaPersistenceRowLikeCpp {
                spell_id: 20,
                area_id: 21,
                quest_start: 22,
                quest_start_status: 23,
                quest_end_status: 24,
                quest_end: 25,
                aura_spell: -26,
                race_mask: 27,
                gender: 2,
                flags: 28,
            }),
            wow_data::SpellAreaRowLikeCpp {
                spell_id: 20,
                area_id: 21,
                quest_start: 22,
                quest_start_status: 23,
                quest_end_status: 24,
                quest_end: 25,
                aura_spell: -26,
                race_mask: 27,
                gender: 2,
                flags: 28,
            }
        );
        assert_eq!(
            group_row_like_cpp(SpellGroupPersistenceRowLikeCpp {
                group_id: 29,
                spell_id: -30,
            }),
            wow_data::SpellGroupRowLikeCpp {
                group_id: 29,
                spell_id: -30,
            }
        );
        assert_eq!(
            group_stack_rule_row_like_cpp(SpellGroupStackRulePersistenceRowLikeCpp {
                group_id: 31,
                stack_rule: 3,
            }),
            wow_data::SpellGroupStackRuleRowLikeCpp {
                group_id: 31,
                stack_rule: 3,
            }
        );
        assert_eq!(
            target_position_row_like_cpp(SpellTargetPositionPersistenceRowLikeCpp {
                spell_id: 32,
                effect_index: 2,
                target_map_id: 33,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: None,
            }),
            wow_data::SpellTargetPositionRowLikeCpp {
                spell_id: 32,
                effect_index: 2,
                target_map_id: 33,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: None,
            }
        );
        let proc_row = SpellProcPersistenceRowLikeCpp {
            spell_id: -34,
            school_mask: 1,
            spell_family_name: 2,
            spell_family_mask: [3, 4, 5, 6],
            proc_flags: [7, 8],
            spell_type_mask: 9,
            spell_phase_mask: 10,
            hit_mask: 11,
            attributes_mask: 12,
            disable_effects_mask: 13,
            procs_per_minute: 14.5,
            chance: 15.5,
            cooldown_ms: 16,
            charges: 17,
        };
        assert_eq!(
            proc_row_like_cpp(proc_row),
            wow_data::SpellProcRowLikeCpp {
                spell_id: -34,
                school_mask: 1,
                spell_family_name: 2,
                spell_family_mask: [3, 4, 5, 6],
                proc_flags: [7, 8],
                spell_type_mask: 9,
                spell_phase_mask: 10,
                hit_mask: 11,
                attributes_mask: 12,
                disable_effects_mask: 13,
                procs_per_minute: 14.5,
                chance: 15.5,
                cooldown_ms: 16,
                charges: 17,
            }
        );
    }

    #[test]
    fn failed_read_stops_before_domain_publication() {
        let result = loaded_rows_like_cpp::<SpellRequiredPersistenceRowLikeCpp>(
            SpellWorldCatalogLoadOutcomeLikeCpp::Failed {
                reason: "world read failed".to_string(),
            },
        );
        assert_eq!(result.unwrap_err().to_string(), "world read failed");
    }
}

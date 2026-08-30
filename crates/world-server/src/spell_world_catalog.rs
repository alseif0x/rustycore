//! Composition boundary for foundational C++ `SpellMgr` World catalogs.

use anyhow::{Result, bail};
use wow_persistence::{
    SpellLinkedPersistenceRowLikeCpp, SpellPetAuraPersistenceRowLikeCpp,
    SpellRequiredPersistenceRowLikeCpp, SpellThreatPersistenceRowLikeCpp,
    SpellTotemModelPersistenceRowLikeCpp, SpellWorldCatalogLoadOutcomeLikeCpp,
    SpellWorldCatalogPersistencePortLikeCpp,
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

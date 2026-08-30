//! Composition boundary for the exact regular `SpellInfo` key Hotfix overlays.

use anyhow::{Result, anyhow};
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, SpellInfoKeyContributorLikeCpp,
    SpellInfoKeyHotfixOverlayBatchLikeCpp, SpellInfoKeyHotfixOverlayRowLikeCpp,
    SpellInfoKeyHotfixOverlaysLikeCpp, SpellInfoPowerDifficultyHotfixOverlayRowLikeCpp,
    SpellNameStore, SpellStore,
};
use wow_persistence::{
    SpellInfoKeyContributorHotfixBatchLikeCpp as PersistenceBatch,
    SpellInfoKeyContributorHotfixRowLikeCpp as PersistenceRow,
    SpellInfoKeyContributorLikeCpp as PersistenceContributor, SpellInfoKeyHotfixLoadOutcomeLikeCpp,
    SpellInfoKeyHotfixPersistencePortLikeCpp, SpellInfoKeyHotfixRowsLikeCpp as PersistenceRows,
    SpellInfoPowerDifficultyHotfixRowLikeCpp as PersistencePowerDifficultyRow,
};

fn contributor_like_cpp(contributor: PersistenceContributor) -> SpellInfoKeyContributorLikeCpp {
    match contributor {
        PersistenceContributor::SpellEffect => SpellInfoKeyContributorLikeCpp::SpellEffect,
        PersistenceContributor::SpellAuraOptions => {
            SpellInfoKeyContributorLikeCpp::SpellAuraOptions
        }
        PersistenceContributor::SpellAuraRestrictions => {
            SpellInfoKeyContributorLikeCpp::SpellAuraRestrictions
        }
        PersistenceContributor::SpellCastingRequirements => {
            SpellInfoKeyContributorLikeCpp::SpellCastingRequirements
        }
        PersistenceContributor::SpellCategories => SpellInfoKeyContributorLikeCpp::SpellCategories,
        PersistenceContributor::SpellClassOptions => {
            SpellInfoKeyContributorLikeCpp::SpellClassOptions
        }
        PersistenceContributor::SpellCooldowns => SpellInfoKeyContributorLikeCpp::SpellCooldowns,
        PersistenceContributor::SpellEquippedItems => {
            SpellInfoKeyContributorLikeCpp::SpellEquippedItems
        }
        PersistenceContributor::SpellInterrupts => SpellInfoKeyContributorLikeCpp::SpellInterrupts,
        PersistenceContributor::SpellLabel => SpellInfoKeyContributorLikeCpp::SpellLabel,
        PersistenceContributor::SpellLevels => SpellInfoKeyContributorLikeCpp::SpellLevels,
        PersistenceContributor::SpellMisc => SpellInfoKeyContributorLikeCpp::SpellMisc,
        PersistenceContributor::SpellPower => SpellInfoKeyContributorLikeCpp::SpellPower,
        PersistenceContributor::SpellReagents => SpellInfoKeyContributorLikeCpp::SpellReagents,
        PersistenceContributor::SpellReagentsCurrency => {
            SpellInfoKeyContributorLikeCpp::SpellReagentsCurrency
        }
        PersistenceContributor::SpellScaling => SpellInfoKeyContributorLikeCpp::SpellScaling,
        PersistenceContributor::SpellShapeshift => SpellInfoKeyContributorLikeCpp::SpellShapeshift,
        PersistenceContributor::SpellTargetRestrictions => {
            SpellInfoKeyContributorLikeCpp::SpellTargetRestrictions
        }
        PersistenceContributor::SpellTotems => SpellInfoKeyContributorLikeCpp::SpellTotems,
        PersistenceContributor::SpellXSpellVisual => {
            SpellInfoKeyContributorLikeCpp::SpellXSpellVisual
        }
    }
}

fn row_like_cpp(row: PersistenceRow) -> SpellInfoKeyHotfixOverlayRowLikeCpp {
    SpellInfoKeyHotfixOverlayRowLikeCpp {
        record_id: row.record_id,
        spell_id: row.spell_id,
        difficulty_id: row.difficulty_id,
    }
}

fn batch_like_cpp(batch: PersistenceBatch) -> SpellInfoKeyHotfixOverlayBatchLikeCpp {
    SpellInfoKeyHotfixOverlayBatchLikeCpp {
        contributor: contributor_like_cpp(batch.contributor),
        rows: batch.rows.into_iter().map(row_like_cpp).collect(),
    }
}

fn power_difficulty_row_like_cpp(
    row: PersistencePowerDifficultyRow,
) -> SpellInfoPowerDifficultyHotfixOverlayRowLikeCpp {
    SpellInfoPowerDifficultyHotfixOverlayRowLikeCpp {
        power_record_id: row.power_record_id,
        difficulty_id: row.difficulty_id,
    }
}

fn overlays_like_cpp(
    outcome: SpellInfoKeyHotfixLoadOutcomeLikeCpp,
) -> Result<SpellInfoKeyHotfixOverlaysLikeCpp> {
    let PersistenceRows {
        contributor_batches,
        power_difficulty_rows,
    } = match outcome {
        SpellInfoKeyHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        SpellInfoKeyHotfixLoadOutcomeLikeCpp::Failed { reason } => return Err(anyhow!(reason)),
    };
    Ok(SpellInfoKeyHotfixOverlaysLikeCpp {
        contributor_batches: contributor_batches
            .into_iter()
            .map(batch_like_cpp)
            .collect(),
        power_difficulty_rows: power_difficulty_rows
            .into_iter()
            .map(power_difficulty_row_like_cpp)
            .collect(),
    })
}

pub(super) async fn load_spell_store_seed_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellInfoKeyHotfixPersistencePortLikeCpp,
    spell_name_store: &SpellNameStore,
    hotfix_removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellStore> {
    let overlays = overlays_like_cpp(persistence.load_spell_info_key_rows_like_cpp().await)?;
    SpellStore::load_spell_info_key_seed_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        spell_name_store,
        hotfix_removals,
        overlays,
    )
}

#[cfg(test)]
mod tests {
    use super::overlays_like_cpp;
    use wow_data::SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP as DATA_ORDER;
    use wow_persistence::{
        SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP as PERSISTENCE_ORDER,
        SpellInfoKeyContributorHotfixBatchLikeCpp, SpellInfoKeyContributorHotfixRowLikeCpp,
        SpellInfoKeyHotfixLoadOutcomeLikeCpp, SpellInfoKeyHotfixRowsLikeCpp,
        SpellInfoPowerDifficultyHotfixRowLikeCpp,
    };

    #[test]
    fn every_typed_row_and_contributor_crosses_the_composition_boundary() {
        let rows = SpellInfoKeyHotfixRowsLikeCpp {
            contributor_batches: PERSISTENCE_ORDER
                .into_iter()
                .enumerate()
                .map(
                    |(index, contributor)| SpellInfoKeyContributorHotfixBatchLikeCpp {
                        contributor,
                        rows: vec![SpellInfoKeyContributorHotfixRowLikeCpp {
                            record_id: index as u32 + 1,
                            spell_id: index as u32 + 101,
                            difficulty_id: index as u8,
                        }],
                    },
                )
                .collect(),
            power_difficulty_rows: vec![SpellInfoPowerDifficultyHotfixRowLikeCpp {
                power_record_id: 77,
                difficulty_id: 3,
            }],
        };

        let mapped = overlays_like_cpp(SpellInfoKeyHotfixLoadOutcomeLikeCpp::Loaded(rows))
            .expect("loaded rows map");
        assert_eq!(
            mapped
                .contributor_batches
                .iter()
                .map(|batch| batch.contributor)
                .collect::<Vec<_>>(),
            DATA_ORDER
        );
        for (index, batch) in mapped.contributor_batches.iter().enumerate() {
            assert_eq!(batch.rows[0].record_id, index as u32 + 1);
            assert_eq!(batch.rows[0].spell_id, index as u32 + 101);
            assert_eq!(batch.rows[0].difficulty_id, index as u8);
        }
        assert_eq!(mapped.power_difficulty_rows[0].power_record_id, 77);
        assert_eq!(mapped.power_difficulty_rows[0].difficulty_id, 3);
    }

    #[test]
    fn empty_and_failure_remain_distinct_before_publication() {
        let empty = overlays_like_cpp(SpellInfoKeyHotfixLoadOutcomeLikeCpp::Loaded(
            SpellInfoKeyHotfixRowsLikeCpp {
                contributor_batches: Vec::new(),
                power_difficulty_rows: Vec::new(),
            },
        ))
        .expect("empty is a successful database outcome");
        assert!(empty.contributor_batches.is_empty());

        let error = overlays_like_cpp(SpellInfoKeyHotfixLoadOutcomeLikeCpp::Failed {
            reason: "hotfix unavailable".to_owned(),
        })
        .expect_err("failure must stop composition");
        assert_eq!(error.to_string(), "hotfix unavailable");
    }
}

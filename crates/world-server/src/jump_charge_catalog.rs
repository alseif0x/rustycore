//! Composition boundary for C++ `ObjectMgr::LoadJumpChargeParams`.

use anyhow::{Result, bail};
use wow_persistence::{
    JumpChargeCatalogLoadOutcomeLikeCpp, JumpChargeCatalogPersistencePortLikeCpp,
    JumpChargeParamsPersistenceRowLikeCpp,
};

fn domain_row_like_cpp(
    row: JumpChargeParamsPersistenceRowLikeCpp,
) -> wow_data::JumpChargeParamsRowLikeCpp {
    wow_data::JumpChargeParamsRowLikeCpp {
        id: row.id,
        speed: row.speed,
        treat_speed_as_move_time_seconds: row.treat_speed_as_move_time_seconds,
        jump_gravity: row.jump_gravity,
        spell_visual_id: row.spell_visual_id,
        progress_curve_id: row.progress_curve_id,
        parabolic_curve_id: row.parabolic_curve_id,
    }
}

pub(super) async fn load_jump_charge_catalog_like_cpp(
    persistence: &dyn JumpChargeCatalogPersistencePortLikeCpp,
    spell_visual_exists: impl Fn(u32) -> bool,
    curve_exists: impl Fn(u32) -> bool,
) -> Result<wow_data::JumpChargeParamsLoadOutcomeLikeCpp> {
    let rows = match persistence.load_rows_like_cpp().await {
        JumpChargeCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        JumpChargeCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    Ok(wow_data::JumpChargeParamsStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter().map(domain_row_like_cpp),
        spell_visual_exists,
        curve_exists,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_movement::JumpChargeSpec;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct FixedPort {
        outcome: JumpChargeCatalogLoadOutcomeLikeCpp,
    }

    impl JumpChargeCatalogPersistencePortLikeCpp for FixedPort {
        fn load_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, JumpChargeCatalogLoadOutcomeLikeCpp> {
            Box::pin(async move { self.outcome.clone() })
        }
    }

    fn persistence_row_like_cpp() -> JumpChargeParamsPersistenceRowLikeCpp {
        JumpChargeParamsPersistenceRowLikeCpp {
            id: 17,
            speed: 12.5,
            treat_speed_as_move_time_seconds: true,
            jump_gravity: 8.5,
            spell_visual_id: Some(21),
            progress_curve_id: Some(31),
            parabolic_curve_id: Some(32),
        }
    }

    #[tokio::test]
    async fn typed_row_preserves_fields_and_domain_validation() {
        let outcome = load_jump_charge_catalog_like_cpp(
            &FixedPort {
                outcome: JumpChargeCatalogLoadOutcomeLikeCpp::Loaded(vec![
                    persistence_row_like_cpp(),
                ]),
            },
            |id| id == 21,
            |id| id == 31,
        )
        .await
        .unwrap();

        let params = outcome.store.get_jump_charge_params_like_cpp(17).unwrap();
        assert_eq!(params.spec, JumpChargeSpec::MoveTimeSeconds(12.5));
        assert_eq!(params.jump_gravity, 8.5);
        assert_eq!(params.spell_visual_id, Some(21));
        assert_eq!(params.progress_curve_id, Some(31));
        assert_eq!(params.parabolic_curve_id, None);
        assert_eq!(outcome.report.ignored_missing_parabolic_curves, [(17, 32)]);
    }

    #[tokio::test]
    async fn empty_success_remains_a_successful_empty_catalog() {
        let outcome = load_jump_charge_catalog_like_cpp(
            &FixedPort {
                outcome: JumpChargeCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
            },
            |_| false,
            |_| false,
        )
        .await
        .unwrap();
        assert!(outcome.store.is_empty());
        assert_eq!(outcome.report.rows_seen, 0);
    }

    #[tokio::test]
    async fn failure_preserves_existing_startup_fatal_policy() {
        let error = load_jump_charge_catalog_like_cpp(
            &FixedPort {
                outcome: JumpChargeCatalogLoadOutcomeLikeCpp::Failed {
                    reason: "world read failed".into(),
                },
            },
            |_| false,
            |_| false,
        )
        .await
        .err()
        .expect("failed persistence must abort startup publication");
        assert_eq!(error.to_string(), "world read failed");
    }
}

//! MariaDB adapter for C++ `ObjectMgr::LoadJumpChargeParams`.

use std::sync::Arc;

use wow_persistence::{
    JumpChargeCatalogLoadOutcomeLikeCpp, JumpChargeCatalogPersistencePortLikeCpp,
    JumpChargeParamsPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{WorldDatabase, WorldStatements};

pub struct MariaDbJumpChargeCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbJumpChargeCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl JumpChargeCatalogPersistencePortLikeCpp for MariaDbJumpChargeCatalogPersistenceAdapterLikeCpp {
    fn load_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, JumpChargeCatalogLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = async {
                let mut result = self
                    .world_db
                    .query(
                        &self
                            .world_db
                            .prepare(WorldStatements::SEL_JUMP_CHARGE_PARAMS),
                    )
                    .await?;
                let mut rows = Vec::with_capacity(result.count());
                if result.is_empty() {
                    return Ok(rows);
                }

                loop {
                    rows.push(JumpChargeParamsPersistenceRowLikeCpp {
                        id: result.read(0),
                        speed: result.read(1),
                        treat_speed_as_move_time_seconds: result.read(2),
                        jump_gravity: result.read(3),
                        spell_visual_id: (!result.is_null(4)).then(|| result.read(4)),
                        progress_curve_id: (!result.is_null(5)).then(|| result.read(5)),
                        parabolic_curve_id: (!result.is_null(6)).then(|| result.read(6)),
                    });
                    if !result.next_row() {
                        break;
                    }
                }
                Ok::<_, anyhow::Error>(rows)
            }
            .await;

            match result {
                Ok(rows) => JumpChargeCatalogLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => JumpChargeCatalogLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn statement_matches_cpp_columns_and_order() {
        assert_eq!(
            WorldStatements::SEL_JUMP_CHARGE_PARAMS.sql(),
            "SELECT id, speed, treatSpeedAsMoveTimeSeconds, jumpGravity, spellVisualId, progressCurveId, parabolicCurveId FROM jump_charge_params"
        );
    }
}

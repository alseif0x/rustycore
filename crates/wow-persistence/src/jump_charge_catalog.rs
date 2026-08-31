//! SQLx-free World source contract for C++ jump-charge parameters.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq)]
pub struct JumpChargeParamsPersistenceRowLikeCpp {
    pub id: i32,
    pub speed: f32,
    pub treat_speed_as_move_time_seconds: bool,
    pub jump_gravity: f32,
    pub spell_visual_id: Option<i32>,
    pub progress_curve_id: Option<i32>,
    pub parabolic_curve_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JumpChargeCatalogLoadOutcomeLikeCpp {
    Loaded(Vec<JumpChargeParamsPersistenceRowLikeCpp>),
    Failed { reason: String },
}

pub trait JumpChargeCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, JumpChargeCatalogLoadOutcomeLikeCpp>;
}

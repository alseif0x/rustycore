// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadJumpChargeParams` represented store.

use std::collections::HashMap;

use anyhow::Result;
use wow_database::{WorldDatabase, WorldStatements};
use wow_movement::{JumpChargeParams, JumpChargeSpec};

pub const SPEED_CHARGE_LIKE_CPP: f32 = 42.0;
pub const MOVEMENT_GRAVITY_LIKE_CPP: f32 = 19.291_105_270_385_74_f32;

#[derive(Debug, Clone, PartialEq)]
pub struct JumpChargeParamsRowLikeCpp {
    pub id: i32,
    pub speed: f32,
    pub treat_speed_as_move_time_seconds: bool,
    pub jump_gravity: f32,
    pub spell_visual_id: Option<i32>,
    pub progress_curve_id: Option<i32>,
    pub parabolic_curve_id: Option<i32>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct JumpChargeParamsLoadReportLikeCpp {
    pub rows_seen: usize,
    pub loaded_params: usize,
    pub corrected_invalid_speeds: Vec<(i32, f32)>,
    pub corrected_invalid_jump_gravities: Vec<(i32, f32)>,
    pub ignored_missing_spell_visuals: Vec<(i32, i32)>,
    /// C++ logs the SpellVisual column value here by mistake; the second value is the bad Curve id.
    pub ignored_missing_progress_curves: Vec<(i32, i32, Option<i32>)>,
    pub ignored_missing_parabolic_curves: Vec<(i32, i32)>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct JumpChargeParamsStoreLikeCpp {
    params_by_id: HashMap<i32, JumpChargeParams>,
}

pub struct JumpChargeParamsLoadOutcomeLikeCpp {
    pub store: JumpChargeParamsStoreLikeCpp,
    pub report: JumpChargeParamsLoadReportLikeCpp,
}

impl JumpChargeParamsStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = JumpChargeParamsRowLikeCpp>,
        spell_visual_exists: impl Fn(u32) -> bool,
        curve_exists: impl Fn(u32) -> bool,
    ) -> JumpChargeParamsLoadOutcomeLikeCpp {
        let mut params_by_id = HashMap::new();
        let mut report = JumpChargeParamsLoadReportLikeCpp::default();

        for row in rows {
            report.rows_seen += 1;

            let mut value = row.speed;
            if value <= 0.0 {
                report.corrected_invalid_speeds.push((row.id, row.speed));
                value = SPEED_CHARGE_LIKE_CPP;
            }

            let mut jump_gravity = row.jump_gravity;
            if jump_gravity <= 0.0 {
                report
                    .corrected_invalid_jump_gravities
                    .push((row.id, row.jump_gravity));
                jump_gravity = MOVEMENT_GRAVITY_LIKE_CPP;
            }

            let spec = if row.treat_speed_as_move_time_seconds {
                JumpChargeSpec::MoveTimeSeconds(value)
            } else {
                JumpChargeSpec::Speed(value)
            };
            let mut params = JumpChargeParams {
                spec,
                jump_gravity,
                spell_visual_id: None,
                progress_curve_id: None,
                parabolic_curve_id: None,
            };

            if let Some(spell_visual_id) = row.spell_visual_id {
                if u32::try_from(spell_visual_id)
                    .ok()
                    .is_some_and(&spell_visual_exists)
                {
                    params.spell_visual_id = Some(spell_visual_id as u32);
                } else {
                    report
                        .ignored_missing_spell_visuals
                        .push((row.id, spell_visual_id));
                }
            }

            if let Some(progress_curve_id) = row.progress_curve_id {
                if u32::try_from(progress_curve_id)
                    .ok()
                    .is_some_and(&curve_exists)
                {
                    params.progress_curve_id = Some(progress_curve_id as u32);
                } else {
                    report.ignored_missing_progress_curves.push((
                        row.id,
                        progress_curve_id,
                        row.spell_visual_id,
                    ));
                }
            }

            if let Some(parabolic_curve_id) = row.parabolic_curve_id {
                if u32::try_from(parabolic_curve_id)
                    .ok()
                    .is_some_and(&curve_exists)
                {
                    params.parabolic_curve_id = Some(parabolic_curve_id as u32);
                } else {
                    report
                        .ignored_missing_parabolic_curves
                        .push((row.id, parabolic_curve_id));
                }
            }

            params_by_id.insert(row.id, params);
        }

        report.loaded_params = params_by_id.len();
        JumpChargeParamsLoadOutcomeLikeCpp {
            store: Self { params_by_id },
            report,
        }
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spell_visual_exists: impl Fn(u32) -> bool,
        curve_exists: impl Fn(u32) -> bool,
    ) -> Result<JumpChargeParamsLoadOutcomeLikeCpp> {
        let mut result = db
            .query(&db.prepare(WorldStatements::SEL_JUMP_CHARGE_PARAMS))
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(JumpChargeParamsRowLikeCpp {
                    id: result.read(0),
                    speed: result.read(1),
                    treat_speed_as_move_time_seconds: result.read(2),
                    jump_gravity: result.read(3),
                    spell_visual_id: if result.is_null(4) {
                        None
                    } else {
                        Some(result.read(4))
                    },
                    progress_curve_id: if result.is_null(5) {
                        None
                    } else {
                        Some(result.read(5))
                    },
                    parabolic_curve_id: if result.is_null(6) {
                        None
                    } else {
                        Some(result.read(6))
                    },
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            spell_visual_exists,
            curve_exists,
        ))
    }

    pub fn get_jump_charge_params_like_cpp(&self, id: i32) -> Option<&JumpChargeParams> {
        self.params_by_id.get(&id)
    }

    pub fn len(&self) -> usize {
        self.params_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params_by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.000_01;

    fn row(id: i32) -> JumpChargeParamsRowLikeCpp {
        JumpChargeParamsRowLikeCpp {
            id,
            speed: 30.0,
            treat_speed_as_move_time_seconds: false,
            jump_gravity: 10.0,
            spell_visual_id: None,
            progress_curve_id: None,
            parabolic_curve_id: None,
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn jump_charge_params_load_valid_rows_like_cpp() {
        let mut valid = row(7);
        valid.treat_speed_as_move_time_seconds = true;
        valid.spell_visual_id = Some(10);
        valid.progress_curve_id = Some(20);
        valid.parabolic_curve_id = Some(21);

        let outcome =
            JumpChargeParamsStoreLikeCpp::from_rows_like_cpp([valid], |id| id == 10, |id| id >= 20);

        assert_eq!(outcome.report.rows_seen, 1);
        assert_eq!(outcome.report.loaded_params, 1);
        let loaded = outcome.store.get_jump_charge_params_like_cpp(7).unwrap();
        assert_eq!(loaded.spec, JumpChargeSpec::MoveTimeSeconds(30.0));
        assert_close(loaded.jump_gravity, 10.0);
        assert_eq!(loaded.spell_visual_id, Some(10));
        assert_eq!(loaded.progress_curve_id, Some(20));
        assert_eq!(loaded.parabolic_curve_id, Some(21));
    }

    #[test]
    fn jump_charge_params_correct_invalid_speed_and_gravity_like_cpp() {
        let mut invalid = row(8);
        invalid.speed = 0.0;
        invalid.jump_gravity = -1.0;

        let outcome =
            JumpChargeParamsStoreLikeCpp::from_rows_like_cpp([invalid], |_| true, |_| true);

        let loaded = outcome.store.get_jump_charge_params_like_cpp(8).unwrap();
        assert_eq!(loaded.spec, JumpChargeSpec::Speed(SPEED_CHARGE_LIKE_CPP));
        assert_close(loaded.jump_gravity, MOVEMENT_GRAVITY_LIKE_CPP);
        assert_eq!(outcome.report.corrected_invalid_speeds, [(8, 0.0)]);
        assert_eq!(outcome.report.corrected_invalid_jump_gravities, [(8, -1.0)]);
    }

    #[test]
    fn jump_charge_params_ignore_missing_optional_refs_like_cpp() {
        let mut invalid = row(9);
        invalid.spell_visual_id = Some(11);
        invalid.progress_curve_id = Some(22);
        invalid.parabolic_curve_id = Some(33);

        let outcome =
            JumpChargeParamsStoreLikeCpp::from_rows_like_cpp([invalid], |_| false, |_| false);

        let loaded = outcome.store.get_jump_charge_params_like_cpp(9).unwrap();
        assert_eq!(loaded.spell_visual_id, None);
        assert_eq!(loaded.progress_curve_id, None);
        assert_eq!(loaded.parabolic_curve_id, None);
        assert_eq!(outcome.report.ignored_missing_spell_visuals, [(9, 11)]);
        assert_eq!(
            outcome.report.ignored_missing_progress_curves,
            [(9, 22, Some(11))]
        );
        assert_eq!(outcome.report.ignored_missing_parabolic_curves, [(9, 33)]);
    }

    #[test]
    fn jump_charge_params_duplicate_id_overwrites_like_cpp() {
        let mut first = row(10);
        first.speed = 1.0;
        let mut second = row(10);
        second.speed = 2.0;

        let outcome =
            JumpChargeParamsStoreLikeCpp::from_rows_like_cpp([first, second], |_| true, |_| true);

        assert_eq!(outcome.report.rows_seen, 2);
        assert_eq!(outcome.report.loaded_params, 1);
        assert_eq!(
            outcome
                .store
                .get_jump_charge_params_like_cpp(10)
                .unwrap()
                .spec,
            JumpChargeSpec::Speed(2.0)
        );
    }
}

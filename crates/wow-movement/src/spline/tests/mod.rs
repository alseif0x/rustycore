//! Spline regressions.
//!
//! Separated from spline.rs under #685.

use super::*;

fn linear_args() -> MoveSplineInitArgs {
    MoveSplineInitArgs {
        path: vec![Position::xyz(0.0, 0.0, 0.0), Position::xyz(10.0, 0.0, 0.0)],
        velocity: 5.0,
        spline_id: 42,
        ..MoveSplineInitArgs::default()
    }
}

mod scenarios;

//! Curves packets.
//!
//! Separated from progression_rewards.rs under #691.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurveEntry {
    pub id: u32,
    pub curve_type: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurvePointEntry {
    pub id: u32,
    pub pos: [f32; 2],
    pub pre_sl_squish_pos: [f32; 2],
    pub curve_id: u32,
    pub order_index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurveInterpolationModeLikeCpp {
    Linear,
    Cosine,
    CatmullRom,
    Bezier3,
    Bezier4,
    Bezier,
    Constant,
}

impl CurveStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Curve.db2", |id, idx, r| CurveEntry {
            id,
            curve_type: r.get_field_u8(idx, 1),
            flags: r.get_field_u8(idx, 2),
        })
    }

    pub fn curve_value_at_like_cpp(
        &self,
        curve_points: &CurvePointStore,
        curve_id: u32,
        x: f32,
    ) -> f32 {
        let grouped_points = curve_points.points_by_curve_like_cpp(self);
        let Some(points) = grouped_points.get(&curve_id) else {
            return 0.0;
        };
        let Some(curve) = self.get(curve_id) else {
            return 0.0;
        };
        if points.is_empty() {
            return 0.0;
        }

        curve_value_at_points_like_cpp(determine_curve_type_like_cpp(curve, points), points, x)
    }

    /// C++ `DB2Manager::GetCurveXAxisRange`.
    pub fn curve_x_axis_range_like_cpp(
        &self,
        curve_points: &CurvePointStore,
        curve_id: u32,
    ) -> Option<(f32, f32)> {
        let grouped_points = curve_points.points_by_curve_like_cpp(self);
        let points = grouped_points.get(&curve_id)?;
        let first = points.first()?;
        let last = points.last()?;
        Some((first[0], last[0]))
    }
}

impl CurvePointStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "CurvePoint.db2", |id, idx, r| {
            CurvePointEntry {
                id,
                pos: f32_array::<2>(r, idx, 0),
                pre_sl_squish_pos: f32_array::<2>(r, idx, 1),
                curve_id: r.get_relationship_id(idx).unwrap_or(0),
                order_index: r.get_field_u8(idx, 4),
            }
        })
    }

    pub fn points_by_curve_like_cpp(
        &self,
        curve_store: &CurveStore,
    ) -> HashMap<u32, Vec<[f32; 2]>> {
        let mut unsorted_points: HashMap<u32, Vec<&CurvePointEntry>> = HashMap::new();
        for point in self.entries.values() {
            if curve_store.get(point.curve_id).is_some() {
                unsorted_points
                    .entry(point.curve_id)
                    .or_default()
                    .push(point);
            }
        }

        let mut points_by_curve = HashMap::with_capacity(unsorted_points.len());
        for (curve_id, mut points) in unsorted_points {
            points.sort_by_key(|point| point.order_index);
            points_by_curve.insert(
                curve_id,
                points.into_iter().map(|point| point.pos).collect(),
            );
        }
        points_by_curve
    }
}

fn determine_curve_type_like_cpp(
    curve: &CurveEntry,
    points: &[[f32; 2]],
) -> CurveInterpolationModeLikeCpp {
    match curve.curve_type {
        1 => {
            if points.len() < 4 {
                CurveInterpolationModeLikeCpp::Cosine
            } else {
                CurveInterpolationModeLikeCpp::CatmullRom
            }
        }
        2 => match points.len() {
            1 => CurveInterpolationModeLikeCpp::Constant,
            2 => CurveInterpolationModeLikeCpp::Linear,
            3 => CurveInterpolationModeLikeCpp::Bezier3,
            4 => CurveInterpolationModeLikeCpp::Bezier4,
            _ => CurveInterpolationModeLikeCpp::Bezier,
        },
        3 => CurveInterpolationModeLikeCpp::Cosine,
        _ => {
            if points.len() != 1 {
                CurveInterpolationModeLikeCpp::Linear
            } else {
                CurveInterpolationModeLikeCpp::Constant
            }
        }
    }
}

fn curve_value_at_points_like_cpp(
    mode: CurveInterpolationModeLikeCpp,
    points: &[[f32; 2]],
    x: f32,
) -> f32 {
    match mode {
        CurveInterpolationModeLikeCpp::Linear => {
            let mut point_index = 0usize;
            while point_index < points.len() && points[point_index][0] <= x {
                point_index += 1;
            }
            if point_index == 0 {
                return points[0][1];
            }
            if point_index >= points.len() {
                return points[points.len() - 1][1];
            }
            let x_diff = points[point_index][0] - points[point_index - 1][0];
            if x_diff == 0.0 {
                return points[point_index][1];
            }
            (((x - points[point_index - 1][0]) / x_diff)
                * (points[point_index][1] - points[point_index - 1][1]))
                + points[point_index - 1][1]
        }
        CurveInterpolationModeLikeCpp::Cosine => {
            let mut point_index = 0usize;
            while point_index < points.len() && points[point_index][0] <= x {
                point_index += 1;
            }
            if point_index == 0 {
                return points[0][1];
            }
            if point_index >= points.len() {
                return points[points.len() - 1][1];
            }
            let x_diff = points[point_index][0] - points[point_index - 1][0];
            if x_diff == 0.0 {
                return points[point_index][1];
            }
            ((points[point_index][1] - points[point_index - 1][1])
                * (1.0 - ((x - points[point_index - 1][0]) / x_diff * std::f32::consts::PI).cos())
                * 0.5)
                + points[point_index - 1][1]
        }
        CurveInterpolationModeLikeCpp::CatmullRom => {
            let mut point_index = 1usize;
            while point_index < points.len() && points[point_index][0] <= x {
                point_index += 1;
            }
            if point_index == 1 {
                return points[1][1];
            }
            if point_index >= points.len() - 1 {
                return points[points.len() - 2][1];
            }
            let x_diff = points[point_index][0] - points[point_index - 1][0];
            if x_diff == 0.0 {
                return points[point_index][1];
            }

            let mu = (x - points[point_index - 1][0]) / x_diff;
            let a0 = -0.5 * points[point_index - 2][1] + 1.5 * points[point_index - 1][1]
                - 1.5 * points[point_index][1]
                + 0.5 * points[point_index + 1][1];
            let a1 = points[point_index - 2][1] - 2.5 * points[point_index - 1][1]
                + 2.0 * points[point_index][1]
                - 0.5 * points[point_index + 1][1];
            let a2 = -0.5 * points[point_index - 2][1] + 0.5 * points[point_index][1];
            let a3 = points[point_index - 1][1];

            a0 * mu * mu * mu + a1 * mu * mu + a2 * mu + a3
        }
        CurveInterpolationModeLikeCpp::Bezier3 => {
            let x_diff = points[2][0] - points[0][0];
            if x_diff == 0.0 {
                return points[1][1];
            }
            let mu = (x - points[0][0]) / x_diff;
            ((1.0 - mu) * (1.0 - mu) * points[0][1])
                + (1.0 - mu) * 2.0 * mu * points[1][1]
                + mu * mu * points[2][1]
        }
        CurveInterpolationModeLikeCpp::Bezier4 => {
            let x_diff = points[3][0] - points[0][0];
            if x_diff == 0.0 {
                return points[1][1];
            }
            let mu = (x - points[0][0]) / x_diff;
            (1.0 - mu) * (1.0 - mu) * (1.0 - mu) * points[0][1]
                + 3.0 * mu * (1.0 - mu) * (1.0 - mu) * points[1][1]
                + 3.0 * mu * mu * (1.0 - mu) * points[2][1]
                + mu * mu * mu * points[3][1]
        }
        CurveInterpolationModeLikeCpp::Bezier => {
            let x_diff = points[points.len() - 1][0] - points[0][0];
            if x_diff == 0.0 {
                return points[points.len() - 1][1];
            }

            let mut tmp: Vec<f32> = points.iter().map(|point| point[1]).collect();
            let mu = (x - points[0][0]) / x_diff;
            let mut i = tmp.len() - 1;
            while i > 0 {
                for k in 0..i {
                    tmp[k] += mu * (tmp[k + 1] - tmp[k]);
                }
                i -= 1;
            }
            tmp[0]
        }
        CurveInterpolationModeLikeCpp::Constant => points[0][1],
    }
}

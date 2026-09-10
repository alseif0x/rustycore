//! Interpolation packets.
//!
//! Separated from spline.rs under #693.

use super::*;

pub(super) fn init_catmull_storage(
    path: &[Position],
    smooth: bool,
    initial_orientation: f32,
) -> SplineData {
    let count = path.len();
    let mut points = vec![Position::ZERO; count + 2];
    points[1..=count].copy_from_slice(path);
    points[0] = offset_initial_virtual_point(path[0], initial_orientation);
    points[count + 1] = path[count - 1];
    SplineData {
        points,
        lengths: Vec::new(),
        first: 1,
        last: i32::try_from(count).expect("spline point count fits i32"),
        cyclic: false,
        smooth,
    }
}

pub(super) fn init_cyclic_catmull_storage(
    path: &[Position],
    smooth: bool,
    cyclic_point: usize,
    initial_orientation: f32,
) -> SplineData {
    let count = path.len();
    let mut points = vec![Position::ZERO; count + 3];
    points[1..=count].copy_from_slice(path);
    points[0] = if cyclic_point == 0 {
        path[count - 1]
    } else {
        offset_initial_virtual_point(path[0], initial_orientation)
    };
    points[count + 1] = path[cyclic_point];
    points[count + 2] = path[cyclic_point + 1];
    SplineData {
        points,
        lengths: Vec::new(),
        first: 1,
        last: i32::try_from(count).expect("spline point count fits i32") + 1,
        cyclic: true,
        smooth,
    }
}

pub(super) fn init_lengths<F>(spline: &mut SplineData, mut next_length: F)
where
    F: FnMut(&SplineData, i32) -> i32,
{
    spline.lengths.resize(spline.last as usize + 1, 0);
    let mut prev_length = 0;
    for index in spline.first..spline.last {
        let new_length = next_length(spline, index).max(prev_length);
        spline.lengths[(index + 1) as usize] = new_length;
        prev_length = new_length;
    }
}

fn offset_initial_virtual_point(position: Position, orientation: f32) -> Position {
    Position::new(
        position.x - orientation.cos(),
        position.y - orientation.sin(),
        position.z,
        position.orientation,
    )
}

pub(super) fn segment_length(spline: &SplineData, index: i32) -> f32 {
    if spline.smooth {
        let mut current = spline.point(index);
        let mut length = 0.0;
        for step in 1..=3 {
            let next = evaluate_catmullrom(spline, index, step as f32 / 3.0);
            length += distance_3d(current, next);
            current = next;
        }
        length
    } else {
        distance_3d(spline.point(index), spline.point(index + 1))
    }
}

pub(super) fn evaluate_linear(spline: &SplineData, index: i32, u: f32) -> Position {
    let start = spline.point(index);
    let end = spline.point(index + 1);
    Position::new(
        start.x + (end.x - start.x) * u,
        start.y + (end.y - start.y) * u,
        start.z + (end.z - start.z) * u,
        start.orientation,
    )
}

pub(super) fn evaluate_derivative_linear(spline: &SplineData, index: i32) -> Position {
    let start = spline.point(index);
    let end = spline.point(index + 1);
    Position::xyz(end.x - start.x, end.y - start.y, end.z - start.z)
}

pub(super) fn evaluate_catmullrom(spline: &SplineData, index: i32, t: f32) -> Position {
    let p0 = spline.point(index - 1);
    let p1 = spline.point(index);
    let p2 = spline.point(index + 1);
    let p3 = spline.point(index + 2);
    let t2 = t * t;
    let t3 = t2 * t;
    let x = 0.5
        * ((2.0 * p1.x)
            + (-p0.x + p2.x) * t
            + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
            + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3);
    let y = 0.5
        * ((2.0 * p1.y)
            + (-p0.y + p2.y) * t
            + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2
            + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3);
    let z = 0.5
        * ((2.0 * p1.z)
            + (-p0.z + p2.z) * t
            + (2.0 * p0.z - 5.0 * p1.z + 4.0 * p2.z - p3.z) * t2
            + (-p0.z + 3.0 * p1.z - 3.0 * p2.z + p3.z) * t3);
    Position::xyz(x, y, z)
}

pub(super) fn evaluate_derivative_catmullrom(spline: &SplineData, index: i32, t: f32) -> Position {
    let p0 = spline.point(index - 1);
    let p1 = spline.point(index);
    let p2 = spline.point(index + 1);
    let p3 = spline.point(index + 2);
    let t2 = t * t;
    let x = 0.5
        * ((-p0.x + p2.x)
            + 2.0 * (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t
            + 3.0 * (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t2);
    let y = 0.5
        * ((-p0.y + p2.y)
            + 2.0 * (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t
            + 3.0 * (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t2);
    let z = 0.5
        * ((-p0.z + p2.z)
            + 2.0 * (2.0 * p0.z - 5.0 * p1.z + 4.0 * p2.z - p3.z) * t
            + 3.0 * (-p0.z + 3.0 * p1.z - 3.0 * p2.z + p3.z) * t2);
    Position::xyz(x, y, z)
}

pub(super) fn wrap_angle_0_2pi(angle: f32) -> f32 {
    angle.rem_euclid(2.0 * PI)
}

pub(super) fn distance_3d(left: Position, right: Position) -> f32 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    let dz = left.z - right.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub(super) fn ms_to_sec(ms: i32) -> f32 {
    ms as f32 / 1000.0
}

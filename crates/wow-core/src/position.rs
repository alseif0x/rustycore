use std::fmt;

/// A 3D position with orientation in the game world.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

impl Position {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        orientation: 0.0,
    };

    #[inline]
    pub fn new(x: f32, y: f32, z: f32, orientation: f32) -> Self {
        Self {
            x,
            y,
            z,
            orientation,
        }
    }

    #[inline]
    pub fn xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            orientation: 0.0,
        }
    }

    /// Squared distance to another position (avoids sqrt).
    #[inline]
    pub fn distance_sq(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    /// 3D distance to another position.
    #[inline]
    pub fn distance(&self, other: &Position) -> f32 {
        self.distance_sq(other).sqrt()
    }

    /// 2D squared distance (ignoring Z axis).
    #[inline]
    pub fn distance_2d_sq(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// 2D distance (ignoring Z axis).
    #[inline]
    pub fn distance_2d(&self, other: &Position) -> f32 {
        self.distance_2d_sq(other).sqrt()
    }

    /// Angle from this position to another position (in radians).
    #[inline]
    pub fn angle_to(&self, other: &Position) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        dy.atan2(dx)
    }

    /// Check if another position is within a given distance.
    #[inline]
    pub fn is_within_dist(&self, other: &Position, dist: f32) -> bool {
        self.distance_sq(other) <= dist * dist
    }

    /// Check if another position is within a given 2D distance.
    #[inline]
    pub fn is_within_dist_2d(&self, other: &Position, dist: f32) -> bool {
        self.distance_2d_sq(other) <= dist * dist
    }

    /// Check if a position is in front of this one (within PI/2 arc).
    pub fn is_in_front(&self, other: &Position, arc: f32) -> bool {
        self.has_in_arc(other, arc)
    }

    /// Check if target is within a given arc (in radians) centered on orientation.
    pub fn has_in_arc(&self, target: &Position, arc: f32) -> bool {
        let angle = self.angle_to(target);
        let diff = normalize_angle(angle - self.orientation);
        diff <= arc / 2.0 || diff >= std::f32::consts::TAU - arc / 2.0
    }

    /// Compute a new position at the given distance and angle from this one.
    pub fn point_at_distance(&self, dist: f32, angle: f32) -> Position {
        let total_angle = self.orientation + angle;
        Position::new(
            self.x + dist * total_angle.cos(),
            self.y + dist * total_angle.sin(),
            self.z,
            self.orientation,
        )
    }
}

/// Normalize an angle to [0, 2*PI).
#[inline]
fn normalize_angle(mut angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    angle %= tau;
    if angle < 0.0 {
        angle += tau;
    }
    angle
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "X: {:.3} Y: {:.3} Z: {:.3} O: {:.3}",
            self.x, self.y, self.z, self.orientation
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance() {
        let a = Position::xyz(0.0, 0.0, 0.0);
        let b = Position::xyz(3.0, 4.0, 0.0);
        assert!((a.distance(&b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_distance_sq() {
        let a = Position::xyz(0.0, 0.0, 0.0);
        let b = Position::xyz(3.0, 4.0, 0.0);
        assert!((a.distance_sq(&b) - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_distance_2d() {
        let a = Position::xyz(0.0, 0.0, 0.0);
        let b = Position::xyz(3.0, 4.0, 100.0);
        assert!((a.distance_2d(&b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_is_within_dist() {
        let a = Position::xyz(0.0, 0.0, 0.0);
        let b = Position::xyz(3.0, 4.0, 0.0);
        assert!(a.is_within_dist(&b, 6.0));
        assert!(!a.is_within_dist(&b, 4.0));
    }

    #[test]
    fn test_angle_to() {
        let a = Position::xyz(0.0, 0.0, 0.0);
        let b = Position::xyz(1.0, 0.0, 0.0);
        assert!((a.angle_to(&b) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_point_at_distance() {
        let a = Position::new(0.0, 0.0, 0.0, 0.0);
        let b = a.point_at_distance(5.0, 0.0);
        assert!((b.x - 5.0).abs() < 0.001);
        assert!(b.y.abs() < 0.001);
    }

    #[test]
    fn test_zero_position() {
        let p = Position::ZERO;
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
        assert_eq!(p.z, 0.0);
        assert_eq!(p.orientation, 0.0);
    }
}

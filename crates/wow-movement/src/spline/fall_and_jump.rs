//! Fall and jump packets.
//!
//! Separated from spline.rs under #693.

pub const GRAVITY_LIKE_CPP: f32 = 19.291_105;

const TERMINAL_VELOCITY_LIKE_CPP: f32 = 60.148_003;

const TERMINAL_SAFE_FALL_VELOCITY_LIKE_CPP: f32 = 7.0;

pub(super) const MINIMAL_DURATION_MS_LIKE_CPP: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpSpeeds {
    pub speed_xy: f32,
    pub speed_z: f32,
}

pub fn compute_jump_max_height_like_cpp(speed_z: f32) -> f32 {
    let move_time_half = speed_z / GRAVITY_LIKE_CPP;
    -compute_fall_elevation(move_time_half, false, -speed_z)
}

pub fn calculate_jump_speeds_like_cpp(
    distance: f32,
    base_speed: f32,
    current_speed: f32,
    speed_multiplier: f32,
    min_height: f32,
    max_height: f32,
) -> JumpSpeeds {
    let speed_xy = (base_speed * 3.0 * speed_multiplier).min(28.0_f32.max(current_speed * 4.0));
    let duration = distance / speed_xy;
    let duration_sqr = duration * duration;
    let height = if duration_sqr < min_height * 8.0 / GRAVITY_LIKE_CPP {
        min_height
    } else if duration_sqr > max_height * 8.0 / GRAVITY_LIKE_CPP {
        max_height
    } else {
        GRAVITY_LIKE_CPP * duration_sqr / 8.0
    };

    JumpSpeeds {
        speed_xy,
        speed_z: (2.0 * GRAVITY_LIKE_CPP * height).sqrt(),
    }
}

#[must_use]
pub fn compute_fall_time(path_length: f32, is_safe_fall: bool) -> f32 {
    if path_length < 0.0 {
        return 0.0;
    }

    let terminal_safe_fall_length = TERMINAL_SAFE_FALL_VELOCITY_LIKE_CPP
        * TERMINAL_SAFE_FALL_VELOCITY_LIKE_CPP
        / (2.0 * GRAVITY_LIKE_CPP);
    let terminal_length =
        TERMINAL_VELOCITY_LIKE_CPP * TERMINAL_VELOCITY_LIKE_CPP / (2.0 * GRAVITY_LIKE_CPP);
    let terminal_safe_fall_time = TERMINAL_SAFE_FALL_VELOCITY_LIKE_CPP / GRAVITY_LIKE_CPP;
    let terminal_fall_time = TERMINAL_VELOCITY_LIKE_CPP / GRAVITY_LIKE_CPP;

    if is_safe_fall {
        if path_length >= terminal_safe_fall_length {
            (path_length - terminal_safe_fall_length) / TERMINAL_SAFE_FALL_VELOCITY_LIKE_CPP
                + terminal_safe_fall_time
        } else {
            (2.0 * path_length / GRAVITY_LIKE_CPP).sqrt()
        }
    } else if path_length >= terminal_length {
        (path_length - terminal_length) / TERMINAL_VELOCITY_LIKE_CPP + terminal_fall_time
    } else {
        (2.0 * path_length / GRAVITY_LIKE_CPP).sqrt()
    }
}

#[must_use]
pub fn compute_fall_elevation(t_passed: f32, is_safe_fall: bool, start_velocity: f32) -> f32 {
    let terminal_velocity = if is_safe_fall {
        TERMINAL_SAFE_FALL_VELOCITY_LIKE_CPP
    } else {
        TERMINAL_VELOCITY_LIKE_CPP
    };
    let start_velocity = start_velocity.min(terminal_velocity);
    let terminal_time = if is_safe_fall {
        TERMINAL_SAFE_FALL_VELOCITY_LIKE_CPP / GRAVITY_LIKE_CPP
    } else {
        TERMINAL_VELOCITY_LIKE_CPP / GRAVITY_LIKE_CPP
    } - start_velocity / GRAVITY_LIKE_CPP;

    if t_passed > terminal_time {
        terminal_velocity * (t_passed - terminal_time)
            + start_velocity * terminal_time
            + GRAVITY_LIKE_CPP * terminal_time * terminal_time * 0.5
    } else {
        t_passed * (start_velocity + t_passed * GRAVITY_LIKE_CPP * 0.5)
    }
}

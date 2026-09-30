#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JumpChargeSpec {
    Speed(f32),
    MoveTimeSeconds(f32),
}

impl JumpChargeSpec {
    #[must_use]
    pub const fn value(self) -> f32 {
        match self {
            Self::Speed(value) | Self::MoveTimeSeconds(value) => value,
        }
    }

    #[must_use]
    pub const fn treat_speed_as_move_time_seconds(self) -> bool {
        matches!(self, Self::MoveTimeSeconds(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpChargeParams {
    pub spec: JumpChargeSpec,
    pub jump_gravity: f32,
    pub spell_visual_id: Option<u32>,
    pub progress_curve_id: Option<u32>,
    pub parabolic_curve_id: Option<u32>,
}

impl JumpChargeParams {
    #[must_use]
    pub const fn with_speed(speed: f32) -> Self {
        Self {
            spec: JumpChargeSpec::Speed(speed),
            jump_gravity: 0.0,
            spell_visual_id: None,
            progress_curve_id: None,
            parabolic_curve_id: None,
        }
    }

    #[must_use]
    pub const fn with_move_time_seconds(move_time_seconds: f32) -> Self {
        Self {
            spec: JumpChargeSpec::MoveTimeSeconds(move_time_seconds),
            jump_gravity: 0.0,
            spell_visual_id: None,
            progress_curve_id: None,
            parabolic_curve_id: None,
        }
    }

    #[must_use]
    pub const fn with_jump_gravity(mut self, jump_gravity: f32) -> Self {
        self.jump_gravity = jump_gravity;
        self
    }

    #[must_use]
    pub const fn with_spell_visual_id(mut self, spell_visual_id: u32) -> Self {
        self.spell_visual_id = Some(spell_visual_id);
        self
    }

    #[must_use]
    pub const fn with_progress_curve_id(mut self, progress_curve_id: u32) -> Self {
        self.progress_curve_id = Some(progress_curve_id);
        self
    }

    #[must_use]
    pub const fn with_parabolic_curve_id(mut self, parabolic_curve_id: u32) -> Self {
        self.parabolic_curve_id = Some(parabolic_curve_id);
        self
    }
}

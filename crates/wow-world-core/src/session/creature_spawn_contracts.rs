// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureCreateModelScalarsLikeCpp {
    pub display_scale: f32,
    pub native_x_display_scale: f32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub hover_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureCreateDisplaySelectionLikeCpp {
    pub display_id: u32,
    pub display_scale: f32,
}

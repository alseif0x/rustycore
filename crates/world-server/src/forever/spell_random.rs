//! Source thread-local SFMT/frand capability. No spell state or retained callback.
//! Ordered custom/positivity startup supplies this without speculative draws.
use wow_world::forever::spells::{SpellDiminishingError, SpellValueError};
#[cfg(test)]
mod tests;

#[link(name = "rustycore_forever_spell_random", kind = "static")]
unsafe extern "C" {
    fn rustycore_forever_spell_random(minimum: f32, maximum: f32, value: *mut f32) -> i32;
    fn rustycore_forever_spell_select(
        weights: *const f64,
        count: usize,
        selection: *mut u32,
    ) -> i32;
}

// Do not invoke this eagerly, including for a singleton visual candidate.
pub(super) fn select(weights: &[f64]) -> Result<usize, SpellDiminishingError> {
    let mut selection = 0;
    // Borrowed numeric inputs/output only. C++ owns no pointer after the call.
    match unsafe { rustycore_forever_spell_select(weights.as_ptr(), weights.len(), &mut selection) }
    {
        0 if (selection as usize) < weights.len() => Ok(selection as usize),
        0 | 1 => Err(SpellDiminishingError::InvalidSelection),
        _ => Err(SpellDiminishingError::SelectionUnavailable),
    }
}

// Not invoked as a speculative warmup: every source draw has observable order.
pub(super) fn draw(minimum: f32, maximum: f32) -> Result<f32, SpellValueError> {
    if !minimum.is_finite() || !maximum.is_finite() || maximum < minimum {
        return Err(SpellValueError::InvalidVarianceRange);
    }
    let mut value = 0.0;
    // C++ owns no pointer beyond this call, catches exceptions, and lazily
    // retains only its source TLS RNG. No game/map/entity guard or await here.
    match unsafe { rustycore_forever_spell_random(minimum, maximum, &mut value) } {
        0 if value.is_finite() && value >= minimum && value <= maximum => Ok(value),
        0 => Err(SpellValueError::InvalidVarianceDraw),
        1 => Err(SpellValueError::InvalidVarianceRange),
        _ => Err(SpellValueError::RandomSourceUnavailable),
    }
}

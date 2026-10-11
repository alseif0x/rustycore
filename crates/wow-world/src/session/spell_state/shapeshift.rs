//! Represented shapeshift form boosts.
//!
//! C++ `AuraEffect::HandleShapeshiftBoosts` (`SpellAuraEffects.cpp:1325-1464`)
//! owns the form's bonus spells and the stance-gated self-aura sweep. Form
//! ownership itself lives in `super::aura`.

use super::*;

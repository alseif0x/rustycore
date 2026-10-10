//! The creature movement body over canonical ownership.
//!
//! #1263 F6-8D3a-2: C++ keeps `Unit::movespline`, `Unit::i_motionMaster` and
//! the concrete generators on the live `Creature` object. Their state already
//! lives in the canonical `Creature` (`CreatureRuntimeLikeCpp`, F6-8A/D1); the
//! generator, spline and effect bodies that drive it are written once against
//! [`CreatureMovementLikeCpp`], a borrow of that canonical creature.
//!
//! The legacy `WorldCreature` bridge lends its canonical entity together with
//! its cached `CreatureCreateData` packet projection, so every movement-flag
//! change keeps mirroring into that projection exactly as before. The admitted
//! canonical executor lends the canonical incarnation alone: it has no cached
//! projection, so there is nothing to mirror.

use std::ops::{Deref, DerefMut};

use rand::rngs::StdRng;

use super::*;

/// A mutable borrow of one canonical creature for the movement bodies.
pub struct CreatureMovementLikeCpp<'a> {
    pub creature: &'a mut Creature,
    /// The legacy bridge's cached create projection, when the borrow comes
    /// from that bridge.
    pub(in crate::map_manager) projection: Option<&'a mut CreatureCreateData>,
}

impl<'a> CreatureMovementLikeCpp<'a> {
    /// Borrow a canonical incarnation that has no cached packet projection.
    pub fn canonical_like_cpp(creature: &'a mut Creature) -> Self {
        Self {
            creature,
            projection: None,
        }
    }

    /// Mirror the canonical movement flags into the cached projection, if any.
    pub(in crate::map_manager) fn mirror_projection_movement_flags_like_cpp(&mut self) {
        if let Some(projection) = self.projection.as_deref_mut() {
            projection.movement_flags = self.creature.movement_flags_like_cpp().bits();
        }
    }

    /// C++ `Unit::GetSpeedRate(mtype)`. The legacy bridge has always launched
    /// splines from its projection's template rate; a canonical incarnation
    /// reads `Unit::m_speed_rate` itself.
    fn speed_rate_like_cpp(&self, move_type: UnitMoveType) -> f32 {
        match self.projection.as_deref() {
            Some(projection) if move_type == UnitMoveType::Walk => projection.speed_walk_rate,
            Some(projection) => projection.speed_run_rate,
            None => self.creature.unit().speed_rate()[move_type as usize],
        }
    }

    /// Draw access to the creature-owned runtime RNG. The state has one owner
    /// (the canonical runtime state); this borrow only reaches it.
    pub(crate) fn runtime_rng_mut(&mut self) -> &mut StdRng {
        self.creature
            .runtime_like_cpp_mut()
            .runtime_rng_like_cpp_mut()
    }

    pub(in crate::map_manager) fn walk_speed_like_cpp(&self) -> f32 {
        (self.speed_rate_like_cpp(UnitMoveType::Walk) * 2.5).max(0.01)
    }

    pub(in crate::map_manager) fn run_speed_like_cpp(&self) -> f32 {
        (self.speed_rate_like_cpp(UnitMoveType::Run) * 7.0).max(0.01)
    }
}

impl Deref for CreatureMovementLikeCpp<'_> {
    type Target = Creature;

    fn deref(&self) -> &Creature {
        self.creature
    }
}

impl DerefMut for CreatureMovementLikeCpp<'_> {
    fn deref_mut(&mut self) -> &mut Creature {
        self.creature
    }
}

impl WorldCreature {
    /// Lend this bridge's canonical entity, with its cached projection, to the
    /// movement bodies.
    pub fn movement_like_cpp(&mut self) -> CreatureMovementLikeCpp<'_> {
        CreatureMovementLikeCpp {
            creature: &mut self.creature,
            projection: Some(&mut self.create_data),
        }
    }
}

//! Canonical Actor application operations, dormant until producer migration.

mod movement;
pub(in crate::session) mod spell;
pub use spell::{run_canonical_spell, CanonicalSpellError, CanonicalSpellFailure,
    CanonicalSpellAbandoned, CanonicalSpellAbandonment, CanonicalSpellLosResolver};
pub(in crate::session) mod aggro;
pub use aggro::{run_canonical_aggro, CanonicalAggroError, CanonicalAggroFailure};

pub use movement::{
    run_canonical_movement, CanonicalMovementAbandoned, CanonicalMovementAbandonment,
    CanonicalMovementError, CanonicalMovementOutcome,
};

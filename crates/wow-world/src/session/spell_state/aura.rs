//! Represented aura state owned at the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The cast-speed and polymorph
//! queries moved to `wow_world_spell` (#1363 slice 12b3).

use super::*;
use wow_world_spell::shapeshift_form_of_spell_like_cpp;

#[path = "aura/effect_queries.rs"]
mod effect_queries;
#[path = "aura/spell_hit_authority.rs"]
mod spell_hit_authority;

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura/f3_shims.rs"]
mod f3_shims;

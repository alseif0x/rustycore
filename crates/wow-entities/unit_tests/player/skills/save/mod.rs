//! Operation-level skill save capture and confirmed-commit reconciliation.
use crate::{
    PlayerSkills, ProfessionItemMove, SkillCriteria, SkillSetEffects, SkillSetSource, SkillUpdate,
    SkillUpdateState,
};

mod acknowledgement;
mod capture;
#[path = "../set/effects.rs"]
mod effects;
#[path = "../set/fixtures.rs"]
mod fixtures;
mod mutations;

use effects::{Effects, Event};
use fixtures::{Fixture, activate, allocated, line, update};

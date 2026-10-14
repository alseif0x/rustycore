//! Represented aura and spell-state responsibility, separated from the
//! Session root under #601. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_spell::PlayerSpellAndTraitTestFixtureLikeCpp;

mod acquisition;
mod aura;
mod aura_application;
mod aura_host;
mod aura_publication;
mod cast;
mod catalog;
mod cooldown;
mod effects;

mod spell;
mod spell_click_host;
mod spell_publication;
mod spellbook;
mod spellbook_host;

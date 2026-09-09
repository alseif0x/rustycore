//! Represented aura and spell-state responsibility, separated from the
//! Session root under #601. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod acquisition;
mod aura;
mod aura_application;
mod aura_publication;
mod cast;
mod catalog;
mod cooldown;
mod effects;
mod mount_aura;
mod spell;
mod spell_click;
mod spell_publication;
mod spellbook;

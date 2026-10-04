// Copyright (c) 2026 alseif0x

//! Receiver-free rules moved out of the Session under #676.
//!
//! The bodies now live in `wow-world-spell`; this module keeps the original
//! `crate::session_rules::…` paths working (#1263 F5).

pub(crate) use wow_world_spell::aura_effects::*;
pub(crate) use wow_world_spell::melee_damage::*;
pub(crate) use wow_world_spell::melee_rules::*;

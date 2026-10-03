// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Instance bind contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

pub(in crate::session) use wow_world_lifecycle::HomebindPersistenceJobLikeCpp;

pub(crate) use wow_world_entities::RepresentedGameObjectSpellCaster;

pub(crate) use wow_world_instances::RepresentedPendingBind;

pub(crate) type RepresentedHomebindLikeCpp = wow_entities::PlayerHomebindLikeCpp;

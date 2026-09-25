// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Instance bind contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, GameObject};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedGameObjectSpellCaster {
    User,
    GameObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPendingBind {
    pub map_id: u32,
    pub instance_id: u32,
    pub completed_mask: u32,
    pub time_until_lock_ms: u32,
}

pub(crate) type RepresentedHomebindLikeCpp = wow_entities::PlayerHomebindLikeCpp;

pub(in crate::session) struct HomebindPersistenceJobLikeCpp {
    pub(in crate::session) port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    pub(in crate::session) request: wow_persistence::PlayerHomebindPersistenceRequestLikeCpp,
    pub(in crate::session) guid_counter: u64,
}

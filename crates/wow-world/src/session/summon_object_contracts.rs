// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Summon object contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ObjectGuid, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ApplyEffectSummonObjectWildSessionStatusLikeCpp {
    InvalidTemplateEntry,
    MissingTemplateStore,
    MissingTemplate,
    MissingExplicitDestination,
    MissingCaster,
    MissingCasterPosition,
    MissingCanonicalMapManager,
    MissingCanonicalPlayerMap,
    MissingManagedMap,
    MapResolved,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub(crate) struct ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
    pub status: ApplyEffectSummonObjectWildSessionStatusLikeCpp,
    pub template_entry: Option<u32>,
    pub duration_ms: Option<i32>,
    pub explicit_destination_used: bool,
    pub close_point_fallback_represented: bool,
    pub map_outcome: Option<wow_map::map::SpellEffectSummonObjectWildOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ApplyEffectSummonObjectSlotSessionStatusLikeCpp {
    InvalidSlot,
    InvalidTemplateEntry,
    MissingTemplateStore,
    MissingTemplate,
    MissingCaster,
    MissingCasterPosition,
    MissingCanonicalMapManager,
    MissingCanonicalPlayerMap,
    MissingManagedMap,
    MapResolved,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub(crate) struct ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
    pub status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp,
    pub slot: Option<usize>,
    pub template_entry: Option<u32>,
    pub duration_ms: Option<i32>,
    pub explicit_destination_used: bool,
    pub close_point_fallback_represented: bool,
    pub cleanup_outcome: Option<wow_map::map::GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp>,
    pub map_outcome: Option<wow_map::map::GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RepresentedSpellFocusObjectLikeCpp {
    pub guid: ObjectGuid,
    pub map_key: wow_map::MapKey,
    pub position: Position,
    pub source: wow_entities::SpellFocusUseSource,
}

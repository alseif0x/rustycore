// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The summon-object contracts, from the World
//! `session/summon_object_contracts.rs`: the wild contracts (#1363 slice 9c)
//! and the slot contracts (#1363 slice 9d), each with its effect.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ApplyEffectSummonObjectWildSessionStatusLikeCpp {
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
pub struct ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
    pub status: ApplyEffectSummonObjectWildSessionStatusLikeCpp,
    pub template_entry: Option<u32>,
    pub duration_ms: Option<i32>,
    pub explicit_destination_used: bool,
    pub close_point_fallback_represented: bool,
    pub map_outcome: Option<wow_map::map::SpellEffectSummonObjectWildOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ApplyEffectSummonObjectSlotSessionStatusLikeCpp {
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
pub struct ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
    pub status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp,
    pub slot: Option<usize>,
    pub template_entry: Option<u32>,
    pub duration_ms: Option<i32>,
    pub explicit_destination_used: bool,
    pub close_point_fallback_represented: bool,
    pub cleanup_outcome: Option<wow_map::map::GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp>,
    pub map_outcome: Option<wow_map::map::GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp>,
}

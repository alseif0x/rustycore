// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! #1263 F6-8D2: the isolated admitted creature execution path.
//!
//! F6-8D1 ported the complete per-creature phase operations onto canonical
//! `Creature`/`CreatureRuntimeLikeCpp` ownership. This module connects that
//! engine to an **admitted** transition — the map incarnations, the selected
//! objects and the tick diff captured for one tick — and executes it in an
//! isolated path that production does not call. Production still runs the
//! externally-driven legacy phases (`RuntimeTickOwner::GlobalLegacy`) and the
//! canonical map still records `MapCreatureUpdateOwnerLikeCpp::ExternalRuntime`,
//! so nothing here changes production behaviour.
//!
//! Three contracts are asserted by the regressions:
//!
//! 1. **Admission is a fence, not a post-hoc count.** Every map incarnation and
//!    every selected object is re-checked against the canonical manager
//!    *before* the first mutation. A map key recreated with a new incarnation
//!    under the same GUID is refused with zero clock, timer, RNG, health and
//!    publication effects — the recreation/ABA falsifier of F6-8D2.
//! 2. **One execution owner per admitted transition.** A transition is claimed
//!    by exactly one owner; a second owner is refused before it touches the
//!    engine, so the diff cannot be advanced, the cast consumed or the swing
//!    committed twice.
//! 3. **One diff/clock advancement, one due cast, one swing commit and one
//!    deferred publication** per admitted transition, with the publication
//!    returned to the caller instead of being delivered under the map guard.
//!
//! The execution owner here is deliberately **not** wired into production: D3
//! owns the exclusive cutover, and E owns the retirement of the legacy store.

use super::*;

// The engine is one responsibility with four cohesive private parts: the
// single-owner claim (`lease`), the frozen admission and its fence
// (`admission`), the admitted execution itself (`execution`) and the
// three-owner composition the acceptance evaluates (`composition`).
mod admission;
mod composition;
mod execution;
mod lease;

pub use admission::*;
pub use composition::*;
pub use execution::*;
pub use lease::*;

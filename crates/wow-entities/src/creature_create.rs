// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Compatibility path for the creature CREATE projection schema.
//!
//! Historically moved out of wow-packet to remove the map manager's normal wire
//! dependency; the canonical schema now lives in wow-data-model. Projection and
//! gameplay operations remain with their existing owners.

pub use wow_data_model::creature_create::CreatureCreateData;

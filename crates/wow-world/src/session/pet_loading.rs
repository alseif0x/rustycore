// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pet loading row types. The pet mode state and the `PetsCx` row loaders moved
//! to `wow_world_lifecycle::pet_session` (#1363 slice 7a).

pub(crate) use wow_world_lifecycle::adjusted_represented_pet_aura_remain_time_like_cpp;
// The character aura rows moved down to `wow-world-spell` (#1363 slice 8d).
pub(crate) use wow_world_spell::{CharacterAuraEffectRowLikeCpp, CharacterAuraRowLikeCpp};

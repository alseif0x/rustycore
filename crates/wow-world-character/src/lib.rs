// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler operations (C++ `CharacterHandler.cpp` and the login
//! support around it), moved out of `wow-world-application` (#1363 A5).
//!
//! This crate sits high: only `wow-world-application` and `wow-world` may
//! depend on it (`tools/architecture/wow_world_closure.py`).

pub mod character_creation;
pub mod character_enumeration;
mod character_handler_families;
mod character_handlers;
pub mod character_login_support;
mod character_query_handlers;
mod login_session;

// The exact `ApplicationCharacter` owner facade the handler-contract checker
// requires: the three canonical exports as one use item. The remaining
// application-level re-exports of the same owner module follow separately so
// the facade item stays exact.
pub use character_handlers::{
    CharDeleteStepLikeCpp, CharRenameStepLikeCpp, ConnectToFailedStepLikeCpp,
    CreateCharacterStepLikeCpp, PlayerLoginStepLikeCpp, initial_character_rest_state_like_cpp,
    parse_equipment_cache, send_represented_cinematic_start_like_cpp,
};
pub use character_handlers::{
    CharacterHandlerCxLikeCpp, CharacterHandlerHostLikeCpp, register_character_handlers_like_cpp,
};
pub use character_query_handlers::realm_query_response_like_cpp;
pub use character_query_handlers::{
    CharacterQueryHandlerCxLikeCpp, CharacterQueryHandlerHostLikeCpp,
    register_character_query_handlers_like_cpp,
};
// The character login load (#1363 slice 8d).
pub use login_session::*;

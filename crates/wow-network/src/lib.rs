// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World server networking: TCP listener, per-client WorldSocket, and
//! the authentication handshake flow.

pub mod accept;
pub mod session_mgr;
pub mod world_socket;

pub use accept::{
    SocketTimeoutsLikeCpp, WorldListenerPolicyLikeCpp, start_instance_listener,
    start_world_listener,
};
pub use session_mgr::{InstanceLink, SessionManager};
pub use world_socket::{
    AccountInfo, SocketReader, SocketWriteFenceLikeCpp, SocketWriteFenceWaitResultLikeCpp,
    SocketWriter, WorldSocket, WorldSocketError,
};

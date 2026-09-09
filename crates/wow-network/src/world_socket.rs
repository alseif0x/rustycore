// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Per-client `WorldSocket` — handles the connection lifecycle from initial
//! handshake through encrypted packet I/O.
//!
//! ## Connection flow
//!
//! 1. Server sends: `"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n"`
//! 2. Client sends: `"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n"`
//! 3. Server sends `SMSG_AUTH_CHALLENGE`
//! 4. Client sends `CMSG_AUTH_SESSION`
//! 5. Server validates digest, derives keys
//! 6. Server sends `AuthResponse` + `EnterEncryptedMode` (Ed25519 signed)
//! 7. Client sends `EnterEncryptedModeAck`
//! 8. All subsequent packets are AES-128-GCM encrypted

use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::BytesMut;
use num_traits::{FromPrimitive, ToPrimitive};
use rand::Rng;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, info, trace, warn};

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::IpLocationStore;
use wow_crypto::{HmacSha256, SessionKeyGenerator256, WorldCrypt};
use wow_packet::header::{HEADER_SIZE, PacketHeader, TAG_SIZE};
use wow_packet::packets::auth::{AuthChallenge, AuthSession, EnterEncryptedMode, Ping, Pong};
use wow_packet::{ClientPacket, ServerPacket, WorldPacket, compression};

// ── Protocol constants ────────────────────────────────────────────

mod ops_1;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "world_socket/tests/mod.rs"]
mod tests;

// Build-specific auth seeds are loaded from the `build_info` DB table at startup,
// injected through the composition-owned `AccountLookup`, and carried by `AccountInfo`.

// ── Error type ────────────────────────────────────────────────────

// ── Socket state machine ─────────────────────────────────────────

// ── Account info from DB ─────────────────────────────────────────

// ── WorldSocket ──────────────────────────────────────────────────

// ── Split I/O types ─────────────────────────────────────────────

// ── Account lookup trait ─────────────────────────────────────────

// ── Helper functions ─────────────────────────────────────────────

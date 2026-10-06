// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The world-session adapter for the shared packet-handler registry.

#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
use wow_constants::ClientOpcodes;
use wow_handler::DuplicateHandlerRegistrationLikeCpp;

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};

/// A handler thunk specialized for the world session and its catalog view.
pub type PacketHandlerFn =
    wow_handler::PacketHandlerFn<WorldSession, SessionHandlerCatalogsLikeCpp>;

/// A registered handler specialized for the world session and its catalog view.
pub type PacketHandlerEntry =
    wow_handler::PacketHandlerEntry<WorldSession, SessionHandlerCatalogsLikeCpp>;

/// The immutable registry used by a world session.
pub type WorldPacketHandlerRegistry =
    wow_handler::PacketHandlerRegistry<WorldSession, SessionHandlerCatalogsLikeCpp>;

/// The builder used to compose world-session packet handlers.
pub type WorldPacketHandlerRegistryBuilder =
    wow_handler::RegistryBuilder<WorldSession, SessionHandlerCatalogsLikeCpp>;

/// Inventory's orphan-rule adapter for registrations declared in this crate.
pub(crate) struct LegacyPacketHandlerRegistrationLikeCpp {
    pub(crate) entry: &'static PacketHandlerEntry,
}

inventory::collect!(LegacyPacketHandlerRegistrationLikeCpp);

/// Register one static world-session packet handler with the legacy collector.
macro_rules! register_packet_handler_like_cpp {
    ($entry:expr) => {
        const _: () = {
            static ENTRY: $crate::session::registry::PacketHandlerEntry = $entry;
            inventory::submit! {
                $crate::session::registry::LegacyPacketHandlerRegistrationLikeCpp {
                    entry: &ENTRY,
                }
            }
        };
    };
}

pub(crate) use register_packet_handler_like_cpp;

/// Register every legacy inventory entry in a world-session registry builder.
pub fn register_remaining_handlers_like_cpp(
    builder: &mut WorldPacketHandlerRegistryBuilder,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp> {
    for registration in inventory::iter::<LegacyPacketHandlerRegistrationLikeCpp> {
        builder.register(*registration.entry)?;
    }
    Ok(())
}

/// Compose the immutable world-session dispatch table from registered handlers.
#[cfg(any(test, feature = "test-fixtures"))]
#[must_use]
pub fn build_dispatch_table() -> Arc<WorldPacketHandlerRegistry> {
    crate::handler_composition::compose_packet_handlers_like_cpp()
        .expect("invalid duplicate packet handler composition")
}

/// Snapshot the composed handler entries for callers outside the dispatch path.
#[cfg(any(test, feature = "test-fixtures"))]
#[must_use]
pub fn registered_handler_entries_like_cpp() -> std::vec::IntoIter<PacketHandlerEntry> {
    build_dispatch_table()
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
}

/// Check if a handler is registered for the given opcode.
#[cfg(any(test, feature = "test-fixtures"))]
#[must_use]
pub fn contains_handler(opcode: ClientOpcodes) -> bool {
    get_handler(opcode).is_some()
}

/// Get the composed handler entry for a specific opcode.
#[cfg(any(test, feature = "test-fixtures"))]
#[must_use]
pub fn get_handler(opcode: ClientOpcodes) -> Option<PacketHandlerEntry> {
    build_dispatch_table().get(opcode).copied()
}

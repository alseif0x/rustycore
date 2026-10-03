// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;
use std::collections::hash_map::Entry as HashMapEntry;
use std::fmt;

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;

use crate::{HandlerFuture, PacketProcessing, SessionStatus};

/// The call a registered opcode performs for a concrete session and catalog type.
pub type PacketHandlerFn<S, C> =
    for<'a> fn(&'a mut S, &'a C, WorldPacket) -> HandlerFuture<'a, ()>;

/// A registered packet handler: admission metadata and the call itself.
pub struct PacketHandlerEntry<S, C> {
    /// Opcode dispatched to this handler.
    pub opcode: ClientOpcodes,
    /// Session status required before invoking the handler.
    pub status: SessionStatus,
    /// Packet phase eligibility for this handler.
    pub processing: PacketProcessing,
    /// Stable diagnostic name for the handler.
    pub handler_name: &'static str,
    /// The handler thunk invoked by the dispatcher.
    pub handler: PacketHandlerFn<S, C>,
}

impl<S, C> Copy for PacketHandlerEntry<S, C> {}

impl<S, C> Clone for PacketHandlerEntry<S, C> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A duplicate opcode registration, preserving both declared handler names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateHandlerRegistrationLikeCpp {
    /// The opcode registered more than once.
    pub opcode: ClientOpcodes,
    /// Name of the entry already in the builder.
    pub previous_handler_name: &'static str,
    /// Name of the rejected entry.
    pub new_handler_name: &'static str,
}

impl fmt::Display for DuplicateHandlerRegistrationLikeCpp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "duplicate packet handler for {:?}: previous handler {:?}, new handler {:?}",
            self.opcode, self.previous_handler_name, self.new_handler_name
        )
    }
}

impl std::error::Error for DuplicateHandlerRegistrationLikeCpp {}

/// Collects opcode handlers before publishing an immutable registry.
pub struct RegistryBuilder<S, C> {
    entries: HashMap<ClientOpcodes, PacketHandlerEntry<S, C>>,
}

impl<S, C> Default for RegistryBuilder<S, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, C> RegistryBuilder<S, C> {
    /// Start an empty registry builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Add an entry, returning an error without replacing an existing opcode.
    pub fn register(
        &mut self,
        entry: PacketHandlerEntry<S, C>,
    ) -> Result<(), DuplicateHandlerRegistrationLikeCpp> {
        match self.entries.entry(entry.opcode) {
            HashMapEntry::Occupied(existing) => Err(DuplicateHandlerRegistrationLikeCpp {
                opcode: entry.opcode,
                previous_handler_name: existing.get().handler_name,
                new_handler_name: entry.handler_name,
            }),
            HashMapEntry::Vacant(vacant) => {
                vacant.insert(entry);
                Ok(())
            }
        }
    }

    /// Finish composition and publish an immutable registry.
    #[must_use]
    pub fn build(self) -> PacketHandlerRegistry<S, C> {
        PacketHandlerRegistry {
            entries: self.entries,
        }
    }
}

/// An immutable opcode-to-handler registry.
pub struct PacketHandlerRegistry<S, C> {
    entries: HashMap<ClientOpcodes, PacketHandlerEntry<S, C>>,
}

impl<S, C> PacketHandlerRegistry<S, C> {
    /// Look up the handler registered for `opcode`.
    #[must_use]
    pub fn get(&self, opcode: ClientOpcodes) -> Option<&PacketHandlerEntry<S, C>> {
        self.entries.get(&opcode)
    }

    /// Return whether `opcode` has a registered handler.
    #[must_use]
    pub fn contains(&self, opcode: ClientOpcodes) -> bool {
        self.entries.contains_key(&opcode)
    }

    /// Return the number of registered handlers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether the registry has no handlers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over registered handlers in unspecified order.
    pub fn iter(&self) -> impl Iterator<Item = &PacketHandlerEntry<S, C>> + '_ {
        self.entries.values()
    }
}

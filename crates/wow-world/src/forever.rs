//! Forever's canonical authenticated session; no legacy packet/Player mirror.
//! Native build-70170 transport hands off only after its encrypted-mode ACK.
//! The same PacketHandlerEntry declaration owns metadata and invocation for
//! this 32-bit protocol. No second opcode match is used as a dispatcher.

pub mod appearance;
mod catalog;
pub mod creation;
mod handlers;
pub mod name_rules;
pub mod permissions;
pub mod player;
mod presentation;
pub mod selection;
pub mod spells;
#[cfg(test)]
mod tests;

use crate::session::registry::PacketHandlerEntryFor;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use wow_data::forever_hotfix::ForeverHotfixCatalog;
use wow_handler::PacketProcessing;
use wow_persistence::forever::{AccountSnapshot, LoadError, SessionRepository};

pub use catalog::{CharacterCatalog, ClassAvailability, RaceAvailability};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// C++ connection-level STATUS_NEVER/PROCESS_INPLACE, before gameplay.
    ConnectionEarly,
    Authenticated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Encrypted,
    Initializing,
    Authenticated,
    Selecting,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    Phase,
    Protocol,
    Codec,
    Persistence(LoadError),
    Registry,
    NameRules(name_rules::NameRuleError),
}

/// Already admitted account identity, supplied by composition after proof.
pub struct Identity {
    pub account_id: u32,
    pub battlenet_id: u32,
    pub realm_address: u32,
    pub account_expansion: u8,
    pub dbc_locale: u8,
    /// Canonical immutable default-RBAC projection. Composition admits only
    /// security zero and no explicit grants/denials before supplying it.
    pub permissions: Arc<permissions::DefaultAccountPermissions>,
}

pub struct InitializationPolicy {
    pub realm_name: String,
    pub normalized_realm_name: String,
    pub timezone: String,
    pub cache_version: u32,
    pub content_set: i32,
    pub max_characters: i32,
    pub character_templates: Arc<creation::CharacterTemplates>,
}

/// Payload is deliberately not Debug. Metadata can be recorded without rows.
pub struct Request {
    pub opcode: u32,
    pub payload: Vec<u8>,
}

pub struct Outgoing {
    opcode: u32,
    payload: Vec<u8>,
}

impl Outgoing {
    pub fn opcode(&self) -> u32 {
        self.opcode
    }
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    fn new(opcode: u32, payload: Vec<u8>) -> Self {
        Self { opcode, payload }
    }
}

type HandlerResult = Result<Vec<Outgoing>, SessionError>;
type Entry =
    PacketHandlerEntryFor<Session, CharacterCatalog, u32, Request, HandlerResult, Admission>;
inventory::collect!(Entry);

pub struct Session {
    identity: Identity,
    phase: Phase,
    snapshot: Option<AccountSnapshot>,
    repository: Arc<dyn SessionRepository>,
    hotfixes: Arc<ForeverHotfixCatalog>,
    name_rules: Arc<name_rules::NameRules>,
    name_policy: name_rules::NamePolicy,
    selection_policy: selection::SelectionPolicy,
    legitimate_characters: HashSet<wow_core::ObjectGuid>,
    registry: HashMap<u32, &'static Entry>,
    latency: u32,
    enumerated: bool,
    character_activity: std::time::Instant,
}

impl Session {
    pub fn after_encryption(
        identity: Identity,
        repository: Arc<dyn SessionRepository>,
        hotfixes: Arc<ForeverHotfixCatalog>,
        name_rules: Arc<name_rules::NameRules>,
        name_policy: name_rules::NamePolicy,
        selection_policy: selection::SelectionPolicy,
    ) -> Result<Self, SessionError> {
        let mut registry = HashMap::new();
        for entry in inventory::iter::<Entry> {
            if registry.insert(entry.opcode, entry).is_some() {
                return Err(SessionError::Registry);
            }
        }
        if identity.account_id == 0
            || identity.battlenet_id == 0
            || identity.realm_address & 0xFFFF == 0
            || identity.dbc_locale >= 12
            || !(1..=12).contains(&name_policy.minimum_units)
        {
            return Err(SessionError::Protocol);
        }
        Ok(Self {
            identity,
            phase: Phase::Encrypted,
            snapshot: None,
            repository,
            hotfixes,
            name_rules,
            name_policy,
            selection_policy,
            legitimate_characters: HashSet::new(),
            registry,
            latency: 0,
            enumerated: false,
            character_activity: std::time::Instant::now(),
        })
    }

    pub async fn initialize(
        &mut self,
        catalog: &CharacterCatalog,
        policy: &InitializationPolicy,
        time: i64,
    ) -> HandlerResult {
        if self.phase != Phase::Encrypted {
            return Err(SessionError::Phase);
        }
        self.phase = Phase::Initializing;
        let snapshot = match self
            .repository
            .load_account(
                self.identity.account_id,
                self.identity.battlenet_id,
                self.identity.realm_address & 0xFFFF,
            )
            .await
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.phase = Phase::Closed;
                return Err(SessionError::Persistence(error));
            }
        };
        let output = match presentation::initialize(
            &self.identity,
            catalog,
            policy,
            &snapshot,
            self.hotfixes.metadata(),
            time,
        ) {
            Ok(output) => output,
            Err(error) => {
                self.phase = Phase::Closed;
                return Err(error);
            }
        };
        // One canonical snapshot; callers publish this ordered batch only
        // after both query-holder responsibilities and codec bounds succeed.
        self.snapshot = Some(snapshot);
        self.phase = Phase::Authenticated;
        self.character_activity = std::time::Instant::now();
        Ok(output)
    }

    pub async fn dispatch(
        &mut self,
        catalog: &CharacterCatalog,
        request: Request,
    ) -> Result<Option<Vec<Outgoing>>, SessionError> {
        if self.phase == Phase::Closed {
            return Err(SessionError::Phase);
        }
        let Some(entry) = self.registry.get(&request.opcode).copied() else {
            return Ok(None);
        };
        if entry.status == Admission::Authenticated && self.phase != Phase::Authenticated {
            return Err(SessionError::Phase);
        }
        // WorldSocket.cpp:438 resets inactive login-screen time only for
        // registered ordinary opcodes. Ping is socket-local and does not reset.
        if entry.status == Admission::Authenticated {
            self.character_activity = std::time::Instant::now();
        }
        let result = (entry.handler)(self, catalog, request).await;
        if result.is_err() {
            self.close();
        }
        result.map(Some)
    }

    /// Transport publication failure/cancellation closes this incarnation.
    pub fn close(&mut self) {
        self.phase = Phase::Closed;
        self.legitimate_characters.clear();
        self.enumerated = false;
    }
    pub fn is_closed(&self) -> bool {
        self.phase == Phase::Closed
    }
    pub fn has_enumerated(&self) -> bool {
        self.enumerated
    }
    /// One session owns the character-phase deadline. No player/queue policy
    /// is implied; those require their separate lifecycle port.
    pub fn character_idle_remaining(
        &self,
        timeout: std::time::Duration,
    ) -> Option<std::time::Duration> {
        timeout.checked_sub(self.character_activity.elapsed())
    }
}

// Target opcodes below are explicit 70009 source hypotheses until the native
// 70170 action is observed; they are not a blanket +1 translation fallback.
pub const ENUM_CHARACTERS: u32 = 0x440014;
pub const HOTFIX_REQUEST: u32 = 0x440011;
pub const LOG_DISCONNECT: u32 = 0x450007;

inventory::submit! { Entry { opcode: wow_network::forever::PING, status: Admission::ConnectionEarly, processing: PacketProcessing::Inplace, handler_name: "forever_ping", handler: |session, _, request| Box::pin(handlers::ping(session, request)) } }
inventory::submit! { Entry { opcode: LOG_DISCONNECT, status: Admission::ConnectionEarly, processing: PacketProcessing::Inplace, handler_name: "forever_disconnect", handler: |session, _, request| Box::pin(handlers::disconnect(session, request)) } }
inventory::submit! { Entry { opcode: ENUM_CHARACTERS, status: Admission::Authenticated, processing: PacketProcessing::ThreadUnsafe, handler_name: "forever_enum", handler: |session, catalog, request| Box::pin(handlers::enumerate(session, catalog, request)) } }
inventory::submit! { Entry { opcode: HOTFIX_REQUEST, status: Admission::Authenticated, processing: PacketProcessing::ThreadUnsafe, handler_name: "forever_hotfix", handler: |session, _, request| Box::pin(handlers::hotfix(session, request)) } }
inventory::submit! { Entry { opcode: wow_packet::forever::db_query::CLASSIC_QUERY_OPCODE, status: Admission::Authenticated, processing: PacketProcessing::Inplace, handler_name: "forever_db_query", handler: |session, _, request| Box::pin(handlers::db_query(session, request)) } }
inventory::submit! { Entry { opcode: wow_packet::forever::name_availability::CHECK_CHARACTER_NAME_AVAILABILITY_OPCODE, status: Admission::Authenticated, processing: PacketProcessing::ThreadUnsafe, handler_name: "forever_check_name", handler: |session, _, request| Box::pin(handlers::check_name(session, request)) } }

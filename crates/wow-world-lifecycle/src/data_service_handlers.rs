// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hotfix and DB2 bulk-query packet handlers.
//!
//! C++ source of truth: `WorldSession::HandleHotfixRequest` and
//! `HandleDBQueryBulk` (`src/server/game/Handlers/MiscHandler.cpp`), the
//! `HotfixBlobCache` delivery data and `DB2StorageBase::WriteRecord`. The
//! family owns the packet bodies and the data-cache reads; the World session
//! only builds the borrowed hub plus catalog context (#1263 F5).

use tracing::{debug, info};
use wow_constants::ClientOpcodes;
use wow_data::{HotfixBlobCache, HotfixRecordStatus, TactKeyStore, hotfix_locale_mask};
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::misc::{
    DBReply, DbQueryBulk, HotfixConnect, HotfixConnectData, HotfixId, HotfixRequest,
};
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

const TACT_KEY_TABLE_HASH_LIKE_CPP: u32 = 0xDF2F_53CF;

/// Borrowed inputs of one hotfix/DB2 handler invocation.
pub struct DataServiceHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    hotfixes: &'a HotfixBlobCache,
    tact_keys: &'a TactKeyStore,
}

impl<'a> DataServiceHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        hotfixes: &'a HotfixBlobCache,
        tact_keys: &'a TactKeyStore,
    ) -> Self {
        Self {
            hub,
            hotfixes,
            tact_keys,
        }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// C++ `WorldSession::HandleHotfixRequest`.
    pub async fn handle_hotfix_request(&mut self, req: HotfixRequest) {
        info!(
            "HotfixRequest: client_build={}, data_build={}, {} hotfixes for account {}, first={:?}, last={:?}",
            req.client_build,
            req.data_build,
            req.hotfixes.len(),
            self.hub.core.account_id,
            req.hotfixes.first(),
            req.hotfixes.last()
        );

        let mut response = HotfixConnect::empty();
        let locale_mask = hotfix_locale_mask(&self.hub.core.locale);
        for push_id in &req.hotfixes {
            let Some(push) = self.hotfixes.hotfix_push(*push_id) else {
                continue;
            };

            for record in &push.records {
                if record.available_locales_mask & locale_mask == 0 {
                    continue;
                }

                let mut status = record.status as u8;
                let mut size = 0u32;

                if record.status == HotfixRecordStatus::Valid {
                    if let Some(blob) = self
                        .hotfixes
                        .get_hotfix_blob(record.table_hash, record.record_id)
                    {
                        let start = response.content.len();
                        response.content.extend_from_slice(blob);
                        if let Some(optional_entries) = self.hotfixes.get_optional_data(
                            record.table_hash,
                            record.record_id,
                            &self.hub.core.locale,
                        ) {
                            for optional_data in optional_entries {
                                response
                                    .content
                                    .extend_from_slice(&optional_data.key.to_le_bytes());
                                response.content.extend_from_slice(&optional_data.data);
                            }
                        }
                        size = (response.content.len() - start) as u32;
                    } else {
                        // C++ known-store hotfixes use DB2StorageBase::WriteRecord, not raw WDC4
                        // bytes. Until Rust has that typed serializer, fail closed so the client
                        // keeps its local DB2 cache instead of parsing a malformed Valid payload.
                        status = HotfixRecordStatus::Invalid as u8;
                    }
                }

                response.hotfixes.push(HotfixConnectData {
                    id: HotfixId {
                        push_id: record.id.push_id,
                        unique_id: record.id.unique_id,
                    },
                    table_hash: record.table_hash,
                    record_id: record.record_id,
                    size,
                    status,
                });
            }
        }

        self.publication_like_cpp().send_packet(&response);
    }

    pub async fn handle_db_query_bulk(&mut self, query: DbQueryBulk) {
        info!(
            "DbQueryBulk: table=0x{:08X}, {} records {:?} for account {}",
            query.table_hash,
            query.queries.len(),
            query.queries,
            self.hub.core.account_id
        );
        for record_id in &query.queries {
            if query.table_hash == TACT_KEY_TABLE_HASH_LIKE_CPP {
                let tact_key = (*record_id)
                    .try_into()
                    .ok()
                    .and_then(|id| self.tact_keys.get(id));
                if let Some(entry) = tact_key {
                    debug!(
                        "DbQueryBulk: TactKey.db2 record={} -> Valid(1), 16-byte typed WriteRecord payload",
                        record_id
                    );
                    self.publication_like_cpp()
                        .send_packet_realm(&DBReply::found(
                            query.table_hash,
                            *record_id,
                            entry.key.to_vec(),
                        ));
                    continue;
                }
                debug!(
                    "DbQueryBulk: NOT_FOUND TactKey.db2 record={} -> Invalid(3), client may use local DB2 cache",
                    record_id
                );
            } else {
                info!(
                    "DbQueryBulk: table=0x{:08X} record={} -> Invalid(3), no typed DB2 storage serializer",
                    query.table_hash, record_id
                );
            }
            // RecordRemoved(2) would tell the client to delete the record from its cache,
            // which is wrong for client-local DB2 rows missing from server typed storage.
            self.publication_like_cpp()
                .send_packet_realm(&DBReply::not_found(query.table_hash, *record_id));
        }
    }
}

/// Builds a data-service handler context from a host's hub and catalogs.
pub trait DataServiceHandlerHostLikeCpp<C> {
    fn data_service_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> DataServiceHandlerCxLikeCpp<'a>;
}

fn handle_hotfix_request_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: DataServiceHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match HotfixRequest::read(&mut pkt) {
            Ok(req) => {
                session
                    .data_service_handler_cx_like_cpp(catalogs)
                    .handle_hotfix_request(req)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read HotfixRequest: {e}"),
        }
    })
}

fn handle_db_query_bulk_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: DataServiceHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match DbQueryBulk::read(&mut pkt) {
            Ok(query) => {
                session
                    .data_service_handler_cx_like_cpp(catalogs)
                    .handle_db_query_bulk(query)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read DbQueryBulk: {e}"),
        }
    })
}

/// Registers the hotfix and DB2 bulk-query handlers on the packet registry.
pub fn register_data_service_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: DataServiceHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::DbQueryBulk,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_db_query_bulk",
        handler: handle_db_query_bulk_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::HotfixRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_hotfix_request",
        handler: handle_hotfix_request_thunk::<S, C>,
    })?;
    Ok(())
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player query handlers that only need the hub and packet publication.
//!
//! C++ source of truth: `WorldSession::HandleQueryTimeOpcode`,
//! `WorldSession::HandleQueryNextMailTime` and `WorldSession::HandleSetSelection`
//! (`src/server/game/Handlers/{Query,Mail,Misc}Handler.cpp`). The family owns
//! these three packet bodies; the World session only builds the borrowed hub
//! context (#1263 F5). The remaining player handlers stay in the shell while
//! they call shell-owned seams (far sight visibility, live-intent stand state,
//! title persistence, item purchase data).

use tracing::info;
use wow_constants::ClientOpcodes;
use wow_core::{GameTime, ObjectGuid};
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::WorldPacket;
use wow_packet::packets::misc::{MailNextTimeEntry, MailQueryNextTimeResult, QueryTimeResponse};
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

/// Borrowed inputs of one player query handler invocation.
pub struct PlayerQueryHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
}

impl<'a> PlayerQueryHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>) -> Self {
        Self { hub }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// CMSG_QUERY_TIME — client requests current server time.
    pub async fn handle_query_time(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);

        self.publication_like_cpp()
            .send_packet(&QueryTimeResponse { current_time });
    }

    /// CMSG_QUERY_NEXT_MAIL_TIME — client asks for the next unread delivery.
    pub async fn handle_query_next_mail_time(&mut self) {
        const MAIL_CHECK_MASK_READ_LIKE_CPP: u8 = 0x01;
        const MAIL_NORMAL_LIKE_CPP: u8 = 0;

        let Some(rows) = self.hub.shared().owned_player_mails_like_cpp() else {
            self.publication_like_cpp()
                .send_packet_realm(&MailQueryNextTimeResult::no_mail());
            return;
        };
        let now = GameTime::now().as_secs() as i64;

        let mut packet = MailQueryNextTimeResult::no_mail();
        let mut sent_senders = std::collections::BTreeSet::new();

        for row in rows {
            if (row.checked_flags as u8 & MAIL_CHECK_MASK_READ_LIKE_CPP) == 0
                && now >= row.deliver_time as i64
                && sent_senders.insert(row.sender)
            {
                let sender_guid = if row.message_type == MAIL_NORMAL_LIKE_CPP {
                    ObjectGuid::create_player(self.hub.shared().core.realm_id(), row.sender as i64)
                } else {
                    ObjectGuid::EMPTY
                };

                packet.next_mail_time = 0.0;
                packet.next.push(MailNextTimeEntry {
                    sender_guid,
                    time_left: (row.deliver_time as i64 - now) as f32,
                    alt_sender_id: if row.message_type == MAIL_NORMAL_LIKE_CPP {
                        0
                    } else {
                        row.sender as i32
                    },
                    alt_sender_type: row.message_type as i8,
                    stationery_id: row.stationery_id,
                });

                if sent_senders.len() > 2 {
                    break;
                }
            }
        }

        self.publication_like_cpp().send_packet_realm(&packet);
    }

    /// CMSG_SET_SELECTION — client clicked/targeted an object.
    pub async fn handle_set_selection(&mut self, mut pkt: WorldPacket) {
        let target_guid = pkt.read_packed_guid().unwrap_or(ObjectGuid::EMPTY);
        self.hub.set_selection_guid_like_cpp(Some(target_guid));
        info!(
            "SetSelection: account {} → {:?}",
            self.hub.shared().core.account_id,
            target_guid
        );
    }
}

/// Builds a player query handler context from a host's hub.
pub trait PlayerQueryHandlerHostLikeCpp<C> {
    fn player_query_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> PlayerQueryHandlerCxLikeCpp<'a>;
}

fn handle_query_time_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: PlayerQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_query_handler_cx_like_cpp(catalogs)
            .handle_query_time()
            .await;
    })
}

fn handle_query_next_mail_time_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: PlayerQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_query_handler_cx_like_cpp(catalogs)
            .handle_query_next_mail_time()
            .await;
    })
}

fn handle_set_selection_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: PlayerQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_query_handler_cx_like_cpp(catalogs)
            .handle_set_selection(pkt)
            .await;
    })
}

/// Registers the player query handlers on the packet registry.
pub fn register_player_query_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: PlayerQueryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryTime,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_time",
        handler: handle_query_time_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryNextMailTime,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_next_mail_time",
        handler: handle_query_next_mail_time_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetSelection,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_selection",
        handler: handle_set_selection_thunk::<S, C>,
    })?;
    Ok(())
}

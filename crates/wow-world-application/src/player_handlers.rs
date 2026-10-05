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

use crate::SessionQuestState;
use tracing::info;
use wow_constants::ClientOpcodes;
use wow_core::{GameTime, ObjectGuid};
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::character::SetTitle;
use wow_packet::packets::item::{GetItemPurchaseData, SetItemPurchaseData};
use wow_packet::packets::misc::FarSight;
use wow_packet::packets::misc::{MailNextTimeEntry, MailQueryNextTimeResult, QueryTimeResponse};
use wow_packet::packets::spell::SetActionButton;
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::entity_update_bridge::player_values_update_to_update_object;
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};
use wow_world_instances::InstanceState;
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_visibility::VisibilityState;

/// C++ `ItemExtendedCostEntry` refund projection for `SMSG_SET_ITEM_PURCHASE_DATA`.
///
/// C++ excludes a season-earned column from the refunded currencies once its
/// `REQUIRE_SEASON_EARNED_n` flag is set; the item columns always mirror the
/// extended-cost row.
pub fn item_purchase_contents_from_extended_cost(
    extended_cost: &wow_data::item::extended_cost::ItemExtendedCostEntry,
    money: u64,
) -> wow_packet::packets::item::ItemPurchaseContents {
    use wow_constants::ItemExtendedCostFlags;
    use wow_packet::packets::item::{ItemPurchaseRefundCurrency, ItemPurchaseRefundItem};

    let mut contents = wow_packet::packets::item::ItemPurchaseContents {
        money,
        ..Default::default()
    };

    for i in 0..5 {
        contents.items[i] = ItemPurchaseRefundItem {
            item_id: extended_cost.item_id[i] as i32,
            item_count: extended_cost.item_count[i] as i32,
        };

        let season_earned = match i {
            0 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1),
            1 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2),
            2 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3),
            3 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4),
            4 => extended_cost
                .flags
                .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5),
            _ => false,
        };
        if !season_earned {
            contents.currencies[i] = ItemPurchaseRefundCurrency {
                currency_id: extended_cost.currency_id[i] as i32,
                currency_count: extended_cost.currency_count[i] as i32,
            };
        }
    }

    contents
}

/// Borrowed inputs of one player query handler invocation.
pub struct PlayerHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    quest_state: &'a mut SessionQuestState,
    inventory: &'a mut InventoryState,
    lifecycle: &'a SessionLifecycleState,
    visibility: &'a mut VisibilityState,
    instances: &'a InstanceState,
}

impl<'a> PlayerHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        quest_state: &'a mut SessionQuestState,
        inventory: &'a mut InventoryState,
        lifecycle: &'a SessionLifecycleState,
        visibility: &'a mut VisibilityState,
        instances: &'a InstanceState,
    ) -> Self {
        Self {
            hub,
            quest_state,
            inventory,
            lifecycle,
            visibility,
            instances,
        }
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

    /// CMSG_FAR_SIGHT — switch the represented seer; the forced visibility
    /// update itself stays with the World session's spawn catalogs and map
    /// providers and runs through the host seam.
    pub async fn handle_far_sight(&mut self, mut pkt: WorldPacket) {
        let far_sight = match FarSight::read(&mut pkt) {
            Ok(far_sight) => far_sight,
            Err(err) => {
                tracing::warn!("Failed to read FarSight: {err}");
                return;
            }
        };

        self.apply_far_sight_like_cpp(far_sight.enable);
    }

    /// C++ `WorldSession::HandleFarSightOpcode`: does not create or remove the
    /// viewpoint; it only switches the represented seer.
    fn apply_far_sight_like_cpp(&mut self, enable: bool) {
        if !enable {
            #[cfg(any(test, feature = "test-fixtures"))]
            if let Some(player_guid) = self.hub.shared().core.player_guid() {
                self.visibility
                    .set_represented_seer_guid_fixture_like_cpp(Some(player_guid));
            }
            return;
        }

        let Some(target) = self
            .visibility
            .current_canonical_farsight_object_like_cpp(self.hub.shared())
        else {
            tracing::debug!("CMSG_FAR_SIGHT enable requested with no current viewpoint");
            return;
        };
        if self
            .instances
            .canonical_map_has_seer_like_object_like_cpp(self.hub.shared(), target)
        {
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                self.visibility
                    .set_represented_seer_guid_fixture_like_cpp(Some(target));
            }
        } else {
            tracing::debug!("CMSG_FAR_SIGHT enable target {:?} is not resoluble", target);
        }
    }

    /// CMSG_GET_ITEM_PURCHASE_DATA — refund window data for one owned item.
    pub async fn handle_get_item_purchase_data(&mut self, mut pkt: WorldPacket) {
        let request = match GetItemPurchaseData::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                tracing::warn!("GetItemPurchaseData parse failed: {e}");
                return;
            }
        };
        let Some(player_guid) = self.hub.shared().core.player_guid() else {
            return;
        };
        let current_total_played_time = self.lifecycle.total_played_time_like_cpp().saturating_add(
            self.lifecycle
                .login_time_like_cpp()
                .map(|login_time| login_time.elapsed().as_secs() as u32)
                .unwrap_or(0),
        );

        let Some(packet) = (|| {
            let item = self
                .inventory
                .resolved_inventory_item_objects_like_cpp(self.hub.shared())
                .and_then(|items| items.get(&request.item_guid).cloned())?;
            if !item.is_refundable() || item.refund_recipient() != player_guid {
                return None;
            }

            let played_time = item.played_time(i64::from(current_total_played_time));
            if played_time > 2 * 60 * 60 {
                return None;
            }

            let extended_cost = self
                .hub
                .catalogs
                .item_extended_cost_store()
                .and_then(|store| store.get(item.paid_extended_cost()))?;
            let contents =
                item_purchase_contents_from_extended_cost(extended_cost, item.paid_money());
            Some(SetItemPurchaseData {
                item_guid: request.item_guid,
                contents,
                flags: 0,
                purchase_time: current_total_played_time.saturating_sub(played_time),
            })
        })() else {
            tracing::debug!(
                "GetItemPurchaseData ignored for non-refundable or unknown item {:?}",
                request.item_guid
            );
            return;
        };

        self.publication_like_cpp().send_packet(&packet);
    }

    /// C++ `Player::HasTitle` + `Player::SetChosenTitle` and the values update.
    pub async fn handle_set_title(&mut self, mut pkt: WorldPacket) {
        let mut packet = match SetTitle::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "SetTitle parse failed: {error}"
                );
                return;
            }
        };

        if packet.title_id > 0 {
            if !self.represented_has_title_like_cpp(packet.title_id as u32) {
                return;
            }
        } else {
            packet.title_id = 0;
        }

        self.represented_set_chosen_title_like_cpp(packet.title_id);
        if let Some(update) = self
            .hub
            .core
            .set_canonical_chosen_title_like_cpp(packet.title_id)
        {
            if let Some(player_guid) = self.hub.shared().core.player_guid() {
                if let Some(packet) = player_values_update_to_update_object(
                    player_guid,
                    self.hub.shared().core.player_map_id_like_cpp(),
                    &update,
                ) {
                    self.publication_like_cpp().send_packet(&packet);
                }
            }
        }
    }

    /// C++ `Player::HasTitle`; the represented fixture fallback stays available.
    fn represented_has_title_like_cpp(&self, title_id: u32) -> bool {
        let canonical = self
            .hub
            .core
            .with_owned_player_like_cpp(|player| player.has_title_like_cpp(title_id));
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.hub.shared().core.player_handle_like_cpp.is_none() {
            return self
                .quest_state
                .fixture_has_represented_known_title_like_cpp(title_id);
        }
        canonical.unwrap_or(false)
    }

    /// C++ `Player::SetChosenTitle`; the represented fixture fallback stays available.
    fn represented_set_chosen_title_like_cpp(&mut self, title_id: i32) {
        let canonical = self
            .hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_chosen_title_like_cpp(title_id))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.hub.shared().core.player_handle_like_cpp.is_none() {
            self.quest_state
                .fixture_set_represented_chosen_title_like_cpp(title_id);
        }
    }

    /// CMSG_SET_ACTION_BUTTON — client binds or clears one action button.
    pub async fn handle_set_action_button(&mut self, mut pkt: WorldPacket) {
        let packet = match SetActionButton::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                tracing::warn!(
                    account = self.hub.shared().core.account_id,
                    "SetActionButton parse failed: {error}"
                );
                return;
            }
        };

        self.hub
            .represented_set_action_button_like_cpp(packet.index, packet.action);
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
pub trait PlayerHandlerHostLikeCpp<C> {
    fn player_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> PlayerHandlerCxLikeCpp<'a>;

    /// Runs the forced visibility refresh after a far-sight switch; the World
    /// session still owns the spawn catalogs and the map providers.
    fn force_update_visibility_after_far_sight_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> HandlerFuture<'a, ()>;
}

fn handle_query_time_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: PlayerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_handler_cx_like_cpp(catalogs)
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
    S: PlayerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_handler_cx_like_cpp(catalogs)
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
    S: PlayerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_handler_cx_like_cpp(catalogs)
            .handle_set_selection(pkt)
            .await;
    })
}

fn handle_far_sight_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: PlayerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_handler_cx_like_cpp(catalogs)
            .handle_far_sight(pkt)
            .await;
        session
            .force_update_visibility_after_far_sight_like_cpp(catalogs)
            .await;
    })
}

fn handle_get_item_purchase_data_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: PlayerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_handler_cx_like_cpp(catalogs)
            .handle_get_item_purchase_data(pkt)
            .await;
    })
}

fn handle_set_title_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: PlayerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_handler_cx_like_cpp(catalogs)
            .handle_set_title(pkt)
            .await;
    })
}

fn handle_set_action_button_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: PlayerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .player_handler_cx_like_cpp(catalogs)
            .handle_set_action_button(pkt)
            .await;
    })
}

/// Registers the player handlers on the packet registry.
pub fn register_player_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: PlayerHandlerHostLikeCpp<C> + Send,
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
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetActionButton,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_action_button",
        handler: handle_set_action_button_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetTitle,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_title",
        handler: handle_set_title_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GetItemPurchaseData,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_get_item_purchase_data",
        handler: handle_get_item_purchase_data_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::FarSight,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_far_sight",
        handler: handle_far_sight_thunk::<S, C>,
    })?;
    Ok(())
}

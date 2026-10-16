// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Gossip menus and NPC interaction text (#1363 slice 8e): the binder
//! activation, the gossip hello with its direct-interaction and trainer and
//! quest-giver fallbacks, the DB gossip menu build and its conditions, the
//! gossip option select, and the prepared trainer gossip menu with its
//! helpers. Moved from `wow-world` `handlers/character/gossip.rs`, the trainer
//! gossip menu of `handlers/character/vendor.rs` and the gossip constants and
//! helpers of `handlers/character/mod.rs`.
//!
//! C++ anchors stay on each body (`WorldSession::HandleGossipHelloOpcode`,
//! `HandleGossipSelectOptionOpcode`, `HandleBinderActivateOpcode`,
//! `Player::PrepareGossipMenu`). The bodies run on the Player's session through
//! [`CharacterLoginHostLikeCpp`], verbatim except for the mechanical `self` ->
//! `host` rewrite; each host method is the same-named `WorldSession`
//! operation, called at the exact point the body called it before the move.

use tracing::{debug, info, warn};
use wow_constants::ConditionSourceType;
use wow_constants::unit::NPCFlags1;
use wow_core::ObjectGuid;
use wow_data::{ConditionEntriesByTypeStore, ConditionId};
use wow_entities::SpellCastMetadata;
use wow_packet::packets::gossip::{GossipMessage, Hello};
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};
use wow_persistence::{
    GossipBroadcastTextLocaleRequestLikeCpp, GossipCatalogReadOutcomeLikeCpp,
    GossipCreatureMenuRequestLikeCpp, GossipMenuCatalogRequestLikeCpp,
    GossipNpcTextCatalogRequestLikeCpp,
};
use wow_world_entities::{
    CreatureSpawnCatalogsLikeCpp, represented_gameobject_gossip_can_interact_with_like_cpp,
};
use wow_world_inventory::PlayerConditionContextSourceLikeCpp;

use crate::CharacterLoginHostLikeCpp;

const DIRECT_VENDOR_MASK_LIKE_CPP: u32 = 0x80 | 0x100 | 0x200 | 0x400 | 0x800;
const DIRECT_TRAINER_MASK_LIKE_CPP: u32 = 0x10 | 0x20 | 0x40;
const DIRECT_FLIGHT_MASTER_LIKE_CPP: u32 = 0x2000;
const DIRECT_AUCTIONEER_LIKE_CPP: u32 = 0x200000;
const DIRECT_BANKER_LIKE_CPP: u32 = 0x20000;
const DIRECT_TABARD_DESIGNER_LIKE_CPP: u32 = 0x80000;
const DIRECT_STABLE_MASTER_LIKE_CPP: u32 = 0x400000;
const DIRECT_GUILD_BANKER_LIKE_CPP: u32 = 0x800000;
const DIRECT_INTERACTION_MASK_LIKE_CPP: u32 = DIRECT_VENDOR_MASK_LIKE_CPP
    | DIRECT_TRAINER_MASK_LIKE_CPP
    | DIRECT_FLIGHT_MASTER_LIKE_CPP
    | DIRECT_AUCTIONEER_LIKE_CPP
    | DIRECT_BANKER_LIKE_CPP
    | DIRECT_TABARD_DESIGNER_LIKE_CPP
    | DIRECT_STABLE_MASTER_LIKE_CPP
    | DIRECT_GUILD_BANKER_LIKE_CPP;

fn npc_has_direct_interaction_like_cpp(npc_flags: u32) -> bool {
    npc_flags & DIRECT_INTERACTION_MASK_LIKE_CPP != 0
}
const DEFAULT_GOSSIP_MESSAGE_LIKE_CPP: i32 = 0x00FF_FFFF;
const TRAINER_NPC_FLAGS_MASK_LIKE_CPP: u32 = 0x10 | 0x20 | 0x40;
pub const GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP: i32 = -1;
pub const GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP: u8 = 3;
const GOSSIP_OPTION_TRAINER_TEXT_LIKE_CPP: &str = "I would like to train.";

fn creature_has_trainer_flag_like_cpp(npc_flags: u32) -> bool {
    (npc_flags & TRAINER_NPC_FLAGS_MASK_LIKE_CPP) != 0
}

fn represented_trainer_gossip_option_like_cpp() -> wow_packet::packets::gossip::ClientGossipOption {
    wow_packet::packets::gossip::ClientGossipOption {
        gossip_option_id: GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP,
        option_npc: GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP,
        option_flags: 0,
        option_cost: 0,
        option_language: 0,
        flags: 0,
        order_index: 0,
        status: 0,
        text: GOSSIP_OPTION_TRAINER_TEXT_LIKE_CPP.to_string(),
        confirm: String::new(),
        spell_id: None,
        override_icon_id: None,
    }
}

fn represented_trainer_gossip_option_info_like_cpp() -> wow_entities::PlayerGossipOptionLikeCpp {
    wow_entities::PlayerGossipOptionLikeCpp {
        gossip_option_id: GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP,
        menu_id: 0,
        order_index: 0,
        option_npc: GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP,
        action_menu_id: 0,
    }
}

fn add_represented_trainer_gossip_option_if_missing_like_cpp(
    gossip_options: &mut Vec<wow_packet::packets::gossip::ClientGossipOption>,
    stored_options: &mut Vec<wow_entities::PlayerGossipOptionLikeCpp>,
    npc_flags: u32,
) -> bool {
    if !creature_has_trainer_flag_like_cpp(npc_flags) {
        return false;
    }

    if gossip_options
        .iter()
        .any(|option| option.option_npc == GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP)
    {
        return false;
    }

    gossip_options.push(represented_trainer_gossip_option_like_cpp());
    stored_options.push(represented_trainer_gossip_option_info_like_cpp());
    true
}

pub fn send_represented_creature_trainer_gossip_menu_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    npc_guid: ObjectGuid,
    entry: u32,
    npc_flags: u32,
) -> bool {
    let mut gossip_options = Vec::new();
    let mut stored_options = Vec::new();
    if !add_represented_trainer_gossip_option_if_missing_like_cpp(
        &mut gossip_options,
        &mut stored_options,
        npc_flags,
    ) {
        return false;
    }

    let gossip_text = if npc_flags & NPCFlags1::QUEST_GIVER.bits() != 0 {
        host.represented_creature_gossip_text_like_cpp(entry)
    } else {
        Vec::new()
    };

    if !crate::replace_player_gossip_options_like_cpp(host, stored_options) {
        return false;
    }
    {
        let (state, hub) = host.split_interaction();
        state.set_player_interaction_source_like_cpp(hub, npc_guid)
    };
    host.send_packet(&GossipMessage {
        gossip_guid: npc_guid,
        gossip_id: 0,
        friendship_faction_id: 0,
        text_id: Some(DEFAULT_GOSSIP_MESSAGE_LIKE_CPP),
        broadcast_text_id: None,
        gossip_options,
        gossip_text,
    });
    true
}

/// CMSG_BINDER_ACTIVATE — player sets hearthstone at innkeeper.
/// C++ refs: `WorldSession::HandleBinderActivateOpcode` /
/// `WorldSession::SendBindPoint` (`Handlers/NPCHandler.cpp:373-402`).
pub async fn handle_binder_activate_with_generator_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    hello: Hello,
) {
    info!(
        "BinderActivate {:?} account {}",
        hello.unit,
        host.core().account_id
    );
    if !host.core().player_is_strictly_in_world_like_cpp()
        || host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true)
    {
        return;
    }
    let Some(_innkeeper) = host.hub_ref().represented_npc_can_interact_with_like_cpp(
        hello.unit,
        NPCFlags1::INNKEEPER.bits(),
        0,
    ) else {
        debug!(
            innkeeper_guid = ?hello.unit,
            account = host.core().account_id,
            "BinderActivate rejected: NPC missing, out of range, dead, or lacks INNKEEPER flag"
        );
        return;
    };
    // C++ HandleBinderActivateOpcode removes feign death before
    // SendBindPoint performs its instanceable-map rejection.
    host.remove_represented_feign_death_if_needed_like_cpp();
    if host.player_current_map_instanceable_like_cpp() {
        debug!(
            innkeeper_guid = ?hello.unit,
            map_id = host.core().player_map_id_like_cpp(),
            "BinderActivate rejected: current map is instanceable like C++ SendBindPoint"
        );
        return;
    }

    // C++ SendBindPoint calls innkeeper->CastSpell(player, 3286, true).
    // Route the triggered creature cast through the represented spell
    // pipeline so SpellGo, EffectBind, persistence, and bind packets keep
    // their C++ ordering and caster identity.
    const BIND_SPELL_ID_LIKE_CPP: i32 = 3286;
    const CAST_FLAG_PENDING_LIKE_CPP: u32 = 0x0000_0001;
    const CAST_FLAG_UNKNOWN_9_LIKE_CPP: u32 = 0x0000_0100;
    const CAST_FLAG_NO_GCD_LIKE_CPP: u32 = 0x0004_0000;
    const BIND_SPELL_GO_CAST_FLAGS_LIKE_CPP: u32 =
        CAST_FLAG_UNKNOWN_9_LIKE_CPP | CAST_FLAG_PENDING_LIKE_CPP | CAST_FLAG_NO_GCD_LIKE_CPP;
    if let Some(player_guid) = host.player_guid() {
        let Some(cast_id) = host.next_represented_spell_cast_guid_like_cpp(BIND_SPELL_ID_LIKE_CPP)
        else {
            return;
        };
        if let Err(error) = host
            .execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                BIND_SPELL_ID_LIKE_CPP,
                player_guid,
                cast_id,
                SpellCastVisual::default(),
                SpellTargetData {
                    flags: 0x2,
                    unit: player_guid,
                    item: ObjectGuid::EMPTY,
                    ..SpellTargetData::default()
                },
                SpellCastMetadata {
                    caster_guid_override: Some(hello.unit),
                    // C++ Spell::SendSpellGo starts with UNKNOWN_9, adds
                    // PENDING for this non-client triggered cast, and
                    // adds NO_GCD because spell 3286 has no
                    // StartRecoveryTime row.
                    cast_flags: BIND_SPELL_GO_CAST_FLAGS_LIKE_CPP,
                    ..SpellCastMetadata::default()
                },
            )
            .await
        {
            warn!(
                innkeeper_guid = ?hello.unit,
                account = host.core().account_id,
                error,
                "BinderActivate bind spell failed"
            );
        }
    }
    // C++ closes gossip after attempting the triggered cast, even if the
    // spell execution itself cannot complete.
    {
        let (s, h) = host.split_interaction();
        s.send_close_gossip_like_cpp(h)
    };
}

/// Direct interaction for NPCs without gossip menus (banker, auctioneer, etc.).
async fn handle_npc_direct_interaction<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    hello: Hello,
    npc_flags: u32,
) {
    use wow_packet::packets::misc::{AuctionHelloResponse, NpcInteractionOpenResult};

    // This is Rust's shortcut for C++ `PrepareGossipMenu` followed by a
    // built-in gossip option. C++ `SendGossipMenu` first replaces the
    // complete InteractionData with this validated source, even when the
    // service subsequently emits a dedicated packet.
    {
        let (state, hub) = host.split_interaction();
        state.set_player_interaction_source_like_cpp(hub, hello.unit)
    };

    // This shortcut represents selecting one of C++'s built-in gossip
    // options immediately after publishing it. `HandleGossipSelectOptionOpcode`
    // removes fake death after source validation and before dispatching the
    // service. Keep the empty-menu fallback out of that transition.
    if npc_has_direct_interaction_like_cpp(npc_flags) {
        host.remove_represented_feign_death_if_needed_like_cpp();
    }

    if npc_flags & DIRECT_VENDOR_MASK_LIKE_CPP != 0 {
        wow_world_inventory::handle_list_inventory(host, hello).await;
    } else if npc_flags & DIRECT_TRAINER_MASK_LIKE_CPP != 0 {
        host.handle_trainer_list(hello).await;
    } else if npc_flags & DIRECT_AUCTIONEER_LIKE_CPP != 0 {
        host.send_packet(&AuctionHelloResponse::open(hello.unit));
    } else if npc_flags & DIRECT_BANKER_LIKE_CPP != 0 {
        host.send_show_bank_like_cpp(hello.unit);
    } else if npc_flags & DIRECT_FLIGHT_MASTER_LIKE_CPP != 0 {
        host.send_packet(&NpcInteractionOpenResult::new(hello.unit, 6));
    } else if npc_flags & DIRECT_TABARD_DESIGNER_LIKE_CPP != 0 {
        host.send_packet(&NpcInteractionOpenResult::new(hello.unit, 14));
    } else if npc_flags & DIRECT_STABLE_MASTER_LIKE_CPP != 0 {
        host.send_packet(&NpcInteractionOpenResult::new(hello.unit, 22));
    } else if npc_flags & DIRECT_GUILD_BANKER_LIKE_CPP != 0 {
        host.send_packet(&NpcInteractionOpenResult::new(hello.unit, 10));
    } else {
        host.send_packet(&GossipMessage::empty(hello.unit, 0, 1));
    }
}

pub async fn handle_gossip_hello<H: CharacterLoginHostLikeCpp>(host: &mut H, hello: Hello) {
    info!(
        "GossipHello for {:?} from account {}",
        hello.unit,
        host.core().account_id
    );

    const GOSSIP_FLAG: u32 = 0x1;

    // C++ `HandleGossipHelloOpcode` resolves the creature through
    // `GetNPCIfCanInteractWith(..., UNIT_NPC_FLAG_GOSSIP, ...)` before
    // preparing DB-backed gossip, including quest text synthesized from a
    // gossip menu with no options.
    let gossip_access =
        host.hub_ref()
            .represented_npc_can_interact_with_like_cpp(hello.unit, GOSSIP_FLAG, 0);
    let trainer_access = match gossip_access {
        Some(access) if access.npc_flags & TRAINER_NPC_FLAGS_MASK_LIKE_CPP != 0 => Some(access),
        Some(_) => None,
        None => host.hub_ref().represented_npc_can_interact_with_like_cpp(
            hello.unit,
            TRAINER_NPC_FLAGS_MASK_LIKE_CPP,
            0,
        ),
    };
    let Some(validated_access) = gossip_access.as_ref().or(trainer_access.as_ref()) else {
        debug!(
            account = host.core().account_id,
            source = ?hello.unit,
            "GossipHello rejected before clearing or publishing player-menu state"
        );
        return;
    };
    let (resolved_npc_flags, resolved_entry) = (validated_access.npc_flags, validated_access.entry);
    info!(
        "GossipHello npc_flags=0x{:X} entry={} for {:?}",
        resolved_npc_flags, resolved_entry, hello.unit
    );

    // C++ pauses the creature and clears PlayerMenu only after
    // GetNPCIfCanInteractWith has accepted the source.
    host.core_mut()
        .mutate_world_creature(hello.unit, |creature| {
            creature.pause_interaction_movement_like_cpp();
        });
    crate::clear_player_gossip_options_like_cpp(host);

    if let Some(access) = gossip_access.as_ref() {
        if let Some(msg) = build_gossip_menu(host, access.entry, access.npc_flags, hello.unit).await
        {
            info!(
                "Sending GossipMessage with {} options and {} quests for entry {}",
                msg.gossip_options.len(),
                msg.gossip_text.len(),
                access.entry
            );
            host.send_packet(&msg);
            return;
        }
    }

    if let Some(access) = trainer_access {
        if send_represented_creature_trainer_gossip_menu_like_cpp(
            host,
            hello.unit,
            access.entry,
            access.npc_flags,
        ) {
            info!(
                "GossipHello trainer fallback sent prepared gossip menu for entry={} {:?}",
                access.entry, hello.unit
            );
            return;
        }
    }

    // No DB gossip menu found. C++ `HandleQuestgiverHelloOpcode` uses the
    // same prepared-gossip path as `HandleGossipHelloOpcode`; the represented
    // seam currently models the quest part of that prepared menu.
    if (resolved_npc_flags & NPCFlags1::QUEST_GIVER.bits()) != 0
        && !npc_has_direct_interaction_like_cpp(resolved_npc_flags)
        && resolved_entry != 0
    {
        if host
            .hub_ref()
            .represented_npc_can_interact_with_like_cpp(
                hello.unit,
                NPCFlags1::QUEST_GIVER.bits(),
                0,
            )
            .is_none()
        {
            debug!(
                "GossipHello questgiver fallback rejected by C++ interaction checks for {:?}",
                hello.unit
            );
            return;
        }
        if host.use_represented_creature_questgiver_like_cpp(hello.unit, resolved_entry) {
            info!(
                "GossipHello questgiver fallback consumed entry={} for {:?}",
                resolved_entry, hello.unit
            );
            return;
        }
        info!(
            "GossipHello questgiver fallback found no quest menu for entry={} {:?}",
            resolved_entry, hello.unit
        );
    }

    // No gossip or quest menu found — fall back to direct interaction based on NPC flags.
    handle_npc_direct_interaction(host, hello, resolved_npc_flags).await;
}

fn gossip_conditions_meet_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    condition_store: &ConditionEntriesByTypeStore,
    source_type: ConditionSourceType,
    source_group: u32,
    source_entry: i32,
    npc_guid: ObjectGuid,
) -> bool {
    let Some(conditions) = condition_store
        .conditions_for_like_cpp(source_type, ConditionId::new(source_group, source_entry, 0))
    else {
        return true;
    };

    let Some(player_object) = host.hub_ref().build_condition_player_object_like_cpp() else {
        warn!(
            "Gossip condition check failed closed: missing player object for {:?}",
            source_type
        );
        return false;
    };
    let Some((source_object, source_unit_snapshot)) =
        host.build_condition_creature_object_like_cpp(npc_guid)
    else {
        warn!(
            "Gossip condition check failed closed: missing source object for {:?}",
            source_type
        );
        return false;
    };

    let Some(player_unit_snapshot) = host.hub_ref().condition_player_unit_snapshot_like_cpp()
    else {
        return false;
    };
    let player_snapshot = host.hub_ref().condition_player_snapshot_like_cpp();
    let player_condition_store = host.hub_ref().catalogs.player_condition_store().cloned();
    let Some(player_condition_context) = host.represented_player_condition_context_like_cpp()
    else {
        return false;
    };

    let mut source_info = wow_conditions::ConditionSourceInfo::from_targets(
        Some(&player_object),
        Some(&source_object),
        None,
    );
    source_info.set_unit_target_snapshot(0, player_unit_snapshot);
    source_info.set_player_target_snapshot(0, player_snapshot);
    source_info.set_unit_target_snapshot(1, source_unit_snapshot);
    if let Some(store) = player_condition_store.as_ref() {
        source_info.set_player_condition_store(store.as_ref());
        if let Some(context) = player_condition_context.as_context(host) {
            source_info.set_player_condition_context(0, context);
        }
    }

    wow_conditions::is_object_meet_to_conditions_like_cpp(
        &mut source_info,
        conditions.as_slice(),
        condition_store,
        |condition, source_info| match wow_conditions::condition_meets_basic_like_cpp(
            condition,
            source_info,
            |current_area, required_area| current_area == required_area,
        ) {
            wow_conditions::ConditionMeetResult::Evaluated(value) => value,
            wow_conditions::ConditionMeetResult::Unsupported => {
                warn!(
                    "Gossip condition check failed closed: unsupported {:?} for {:?} {}:{}",
                    condition.condition_type, source_type, source_group, source_entry
                );
                false
            }
        },
    )
}

fn gossip_menu_text_conditions_meet_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    condition_store: &ConditionEntriesByTypeStore,
    menu_id: u32,
    text_id: u32,
    npc_guid: ObjectGuid,
) -> bool {
    if condition_store
        .conditions_for_like_cpp(
            ConditionSourceType::GossipMenu,
            ConditionId::new(menu_id, text_id as i32, 0),
        )
        .is_some()
    {
        return gossip_conditions_meet_like_cpp(
            host,
            condition_store,
            ConditionSourceType::GossipMenu,
            menu_id,
            text_id as i32,
            npc_guid,
        );
    }

    gossip_conditions_meet_like_cpp(
        host,
        condition_store,
        ConditionSourceType::GossipMenu,
        menu_id,
        0,
        npc_guid,
    )
}

/// Build a GossipMessage from the database for a creature entry.
/// Returns None if no gossip menu exists.
pub async fn build_gossip_menu<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    entry: u32,
    npc_flags: u32,
    npc_guid: wow_core::ObjectGuid,
) -> Option<GossipMessage> {
    use wow_entities::PlayerGossipOptionLikeCpp as GossipOptionInfo;
    use wow_packet::packets::gossip::ClientGossipOption;

    let catalog = host
        .lifecycle()
        .gossip_catalog_persistence_port_like_cpp()?;

    // 1. Get MenuID from creature_template_gossip
    let menu_id = match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        catalog.load_creature_gossip_menu_id_like_cpp(GossipCreatureMenuRequestLikeCpp {
            creature_entry: entry,
        }),
    )
    .await
    {
        Ok(GossipCatalogReadOutcomeLikeCpp::Found(menu_id)) => menu_id,
        _ => return None,
    };

    let condition_store = host.hub_ref().catalogs.condition_store().cloned();

    // 2. Get TextID from gossip_menu, then resolve BroadcastTextID from npc_text.
    // C++ Player::GetGossipTextId iterates every gossip_menu row and keeps the last row whose
    // attached GossipMenu conditions meet for (player, source).
    let text_ids = match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        catalog.load_gossip_menu_text_ids_like_cpp(GossipMenuCatalogRequestLikeCpp { menu_id }),
    )
    .await
    {
        Ok(GossipCatalogReadOutcomeLikeCpp::Found(text_ids)) => text_ids,
        Ok(GossipCatalogReadOutcomeLikeCpp::Missing) => Vec::new(),
        _ => return None,
    };
    let npc_text_id: u32 = if text_ids.is_empty() {
        1
    } else {
        let mut selected = 1;
        for text_id in text_ids {
            let meets = condition_store.as_ref().is_none_or(|store| {
                gossip_menu_text_conditions_meet_like_cpp(
                    host,
                    store.as_ref(),
                    menu_id,
                    text_id,
                    npc_guid,
                )
            });
            if meets {
                selected = text_id;
            }
        }
        selected
    };

    // Resolve BroadcastTextID from npc_text; C++ `GossipMessage::Write`
    // carries optional TextID and BroadcastTextID separately
    // (`Server/Packets/NPCPackets.cpp:106-130`).
    let broadcast_text_id = match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        catalog.load_npc_text_broadcast_id_like_cpp(GossipNpcTextCatalogRequestLikeCpp {
            npc_text_id,
        }),
    )
    .await
    {
        Ok(GossipCatalogReadOutcomeLikeCpp::Found(broadcast_text_id)) => Some(broadcast_text_id),
        _ => None,
    };
    info!(
        "Gossip menu_id={} npc_text_id={} broadcast_text_id={:?}",
        menu_id, npc_text_id, broadcast_text_id
    );

    // 3. Get options from gossip_menu_option
    let raw_options = match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        catalog.load_gossip_menu_options_like_cpp(GossipMenuCatalogRequestLikeCpp { menu_id }),
    )
    .await
    {
        Ok(GossipCatalogReadOutcomeLikeCpp::Found(options)) => options,
        Ok(GossipCatalogReadOutcomeLikeCpp::Missing) => Vec::new(),
        _ => return None,
    };

    // Resolve localized text for each option via OptionBroadcastTextID.
    let locale = host.core().locale.clone();
    info!(
        "Gossip locale='{}' for {} options",
        locale,
        raw_options.len()
    );
    let mut gossip_options = Vec::new();
    let mut stored_options = Vec::new();
    for opt in &raw_options {
        if let Some(store) = condition_store.as_ref()
            && !gossip_conditions_meet_like_cpp(
                host,
                store.as_ref(),
                ConditionSourceType::GossipMenuOption,
                menu_id,
                opt.option_id as i32,
                npc_guid,
            )
        {
            continue;
        }

        let mut text = opt.option_text.clone();

        if opt.option_broadcast_text_id != 0 && locale != "enUS" {
            if let Ok(GossipCatalogReadOutcomeLikeCpp::Found(localized)) = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                catalog.load_broadcast_text_locale_like_cpp(
                    GossipBroadcastTextLocaleRequestLikeCpp {
                        broadcast_text_id: opt.option_broadcast_text_id,
                        locale: locale.clone(),
                    },
                ),
            )
            .await
            {
                if !localized.is_empty() {
                    text = localized;
                }
            }
        }

        gossip_options.push(ClientGossipOption {
            gossip_option_id: opt.gossip_option_id,
            option_npc: opt.option_npc,
            option_flags: i8::from(opt.box_coded),
            option_cost: opt.box_money as i32,
            option_language: i32::try_from(opt.language).unwrap_or(i32::MAX),
            flags: opt.flags,
            order_index: opt.option_id as i32,
            status: 0,
            text,
            confirm: opt.box_text.clone(),
            spell_id: opt.spell_id,
            override_icon_id: opt.override_icon_id,
        });

        stored_options.push(GossipOptionInfo {
            gossip_option_id: opt.gossip_option_id,
            menu_id: opt.menu_id,
            order_index: opt.option_id,
            option_npc: opt.option_npc,
            action_menu_id: opt.action_menu_id,
        });

        if opt.action_poi_id != 0
            || opt.gossip_npc_option_id.is_some()
            || opt.box_broadcast_text_id != 0
        {
            debug!(
                account = host.core().account_id,
                menu_id = opt.menu_id,
                option_id = opt.option_id,
                action_poi_id = opt.action_poi_id,
                gossip_npc_option_id = ?opt.gossip_npc_option_id,
                box_broadcast_text_id = opt.box_broadcast_text_id,
                "Gossip option loaded C++ auxiliary fields for represented runtime"
            );
        }
    }

    // C++ `Player::PrepareGossipMenu` adds a trainer menu option automatically when
    // a creature has trainer flags but no DB gossip option for `GossipOptionNpc::Trainer`.
    // This is required for mixed questgiver+trainer NPCs such as Ranger Sallina.
    add_represented_trainer_gossip_option_if_missing_like_cpp(
        &mut gossip_options,
        &mut stored_options,
        npc_flags,
    );

    let gossip_text = if npc_flags & NPCFlags1::QUEST_GIVER.bits() != 0 {
        host.represented_creature_gossip_text_like_cpp(entry)
    } else {
        Vec::new()
    };

    if gossip_options.is_empty() && gossip_text.is_empty() {
        return None;
    }

    // Store gossip state for when the player selects an option.
    if !crate::replace_player_gossip_options_like_cpp(host, stored_options) {
        return None;
    }
    {
        let (state, hub) = host.split_interaction();
        state.set_player_interaction_source_like_cpp(hub, npc_guid)
    };

    Some(GossipMessage {
        gossip_guid: npc_guid,
        gossip_id: menu_id as i32,
        friendship_faction_id: 0,
        text_id: None,
        broadcast_text_id,
        gossip_options,
        gossip_text,
    })
}

/// Handle CMSG_GOSSIP_SELECT_OPTION — player selects a gossip menu option.
///
/// Routes to the appropriate handler based on the option's OptionNpc value:
/// 1=Vendor, 3=Trainer, 5=Binder, etc.
pub async fn handle_gossip_select_option<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    select: wow_packet::packets::gossip::GossipSelectOption,
) {
    use wow_packet::packets::misc::NpcInteractionOpenResult;

    info!(
        "GossipSelectOption: gossip_id={}, option_id={} from account {}",
        select.gossip_id,
        select.gossip_option_id,
        host.core().account_id
    );

    // Find the selected option in our stored gossip data.
    let opt = crate::player_gossip_option_like_cpp(host, select.gossip_option_id);
    let opt = match opt {
        Some(o) => o,
        None => {
            warn!(
                "GossipSelectOption: unknown gossip_option_id={} — ignoring like C++.",
                select.gossip_option_id
            );
            return;
        }
    };
    let (option_npc, _action_menu_id) = (opt.option_npc, opt.action_menu_id);

    if host.player_interaction_source_guid_like_cpp() != Some(select.gossip_unit) {
        warn!(
            account = host.core().account_id,
            requested_source = ?select.gossip_unit,
            active_source = ?host.player_interaction_source_guid_like_cpp(),
            "GossipSelectOption rejected: interaction source mismatch"
        );
        return;
    }
    let npc_guid = select.gossip_unit;
    let source_is_interactable = if npc_guid.is_any_type_creature() {
        let required_flags = if option_npc == GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP {
            NPCFlags1::GOSSIP.bits() | TRAINER_NPC_FLAGS_MASK_LIKE_CPP
        } else {
            NPCFlags1::GOSSIP.bits()
        };
        host.hub_ref()
            .represented_npc_can_interact_with_like_cpp(npc_guid, required_flags, 0)
            .is_some()
    } else if npc_guid.is_game_object() {
        represented_gameobject_gossip_can_interact_with_like_cpp(host, npc_guid).is_some()
    } else {
        false
    };
    if !source_is_interactable {
        warn!(
            account = host.core().account_id,
            source = ?npc_guid,
            option_npc = option_npc,
            "GossipSelectOption rejected: source no longer interactable"
        );
        return;
    }
    // C++ removes fake death after revalidating the interaction source and
    // before `Player::OnGossipSelect` validates the menu ID.
    host.remove_represented_feign_death_if_needed_like_cpp();
    // The C++ base gossip path rejects a packet menu that is not the
    // currently published menu before executing its built-in action.
    if opt.menu_id != select.gossip_id as u32 {
        warn!(
            account = host.core().account_id,
            requested_menu_id = select.gossip_id,
            active_menu_id = opt.menu_id,
            "GossipSelectOption rejected: active menu mismatch"
        );
        return;
    }
    if npc_guid.is_game_object() && option_npc != 0 {
        warn!(
            account = host.core().account_id,
            source = ?npc_guid,
            option_npc,
            "GossipSelectOption rejected: GameObject option is not C++ OptionNpc::None"
        );
        return;
    }
    info!(
        "GossipSelectOption: OptionNpc={} for {:?}",
        option_npc, npc_guid
    );

    let hello = Hello { unit: npc_guid };
    match option_npc {
        1 => {
            // Vendor
            wow_world_inventory::handle_list_inventory(host, hello).await;
        }
        2 => {
            // Taxinode / Flight Master
            host.send_packet(&NpcInteractionOpenResult::new(npc_guid, 6));
        }
        3 => {
            // Trainer
            host.handle_trainer_list_for_gossip_option_like_cpp(
                hello,
                opt.menu_id,
                opt.order_index,
            )
            .await;
        }
        5 => {
            // Binder (Innkeeper)
            host.send_packet(&NpcInteractionOpenResult::new(npc_guid, 20));
        }
        6 => {
            // Banker
            host.send_show_bank_like_cpp(npc_guid);
        }
        8 => {
            // Guild Tabard Vendor
            host.send_packet(&NpcInteractionOpenResult::new(npc_guid, 14));
        }
        9 => {
            // Battlemaster
            info!("Battlemaster interaction (stub)");
        }
        10 => {
            // Auctioneer
            use wow_packet::packets::misc::AuctionHelloResponse;
            host.send_packet(&AuctionHelloResponse::open(npc_guid));
        }
        12 => {
            // Stable Master
            host.send_packet(&NpcInteractionOpenResult::new(npc_guid, 22));
        }
        _ => {
            info!(
                "GossipSelectOption: unhandled OptionNpc={} — ignored",
                option_npc
            );
        }
    }
}

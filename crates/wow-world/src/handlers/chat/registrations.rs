// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Opcode registrations for this handler family (moved out of the root file to keep it
//! inside its physical budget; PacketHandlerEntry remains the single registration source).

use crate::session::registry::PacketHandlerEntry;
use wow_constants::ClientOpcodes;
use wow_handler::PacketProcessing;
use wow_handler::SessionStatus;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageSay,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_say",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Say, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageYell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_yell",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Yell, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageParty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_party",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Party, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageGuild,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_guild",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Guild, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageOfficer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_officer",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Officer, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageRaid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_raid",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::Raid, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageRaidWarning,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_raid_warning",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::RaidWarning, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageInstanceChat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_instance",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_message_with_policy_like_cpp(pkt, wow_packet::packets::chat::ChatMsg::InstanceChat, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageWhisper,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_whisper",
        handler: |session, catalogs, pkt| Box::pin(async move { session.handle_chat_whisper_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await }),
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageChannel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_channel_message",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_channel_message_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageAfk,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_afk",
        handler: |session, catalogs, pkt| Box::pin(async move { session.handle_chat_afk_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await }),
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageDnd,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_dnd",
        handler: |session, catalogs, pkt| Box::pin(async move { session.handle_chat_dnd_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await }),
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateAadcStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_update_aadc_status",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_update_aadc_status(pkt).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatReportIgnored,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_report_ignored",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_report_ignored(pkt).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatReportFiltered,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_report_filtered",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_report_filtered(pkt).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatMessageEmote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_emote",
        handler: |session, catalogs, pkt| Box::pin(async move { session.handle_chat_emote_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await }),
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Emote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_emote",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_emote(pkt).await }),
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SendTextEmote,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_text_emote",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_text_emote_with_catalogs_like_cpp(
                        catalogs.emotes_text.as_ref(),
                        catalogs.emotes.as_ref(),
                        catalogs.chat_policy.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatRegisterAddonPrefixes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_register_addon_prefixes",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_chat_register_addon_prefixes(pkt).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatAddonMessage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_addon_message",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_addon_message_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await })
        },
    }
}
inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChatAddonMessageWhisper,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_chat_addon_message_whisper",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_chat_addon_message_whisper_with_policy_like_cpp(pkt, catalogs.chat_policy.as_ref()).await })
        },
    }
}

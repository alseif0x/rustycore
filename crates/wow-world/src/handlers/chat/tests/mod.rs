//! Chat handlers regression scenarios.
//!
//! Separated from the chat.rs root under #654.

use super::*;
use crate::session::AuraApplication;
use crate::session::directory::{PlayerRegistry, PlayerSessionRegistrationLikeCpp};
use crate::session_policy::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use wow_social::group::PendingInvites;

const LANG_COMMON_LIKE_CPP: i32 = 7;

fn chat_message_packet(opcode: ClientOpcodes, text: &str) -> wow_packet::WorldPacket {
    chat_message_packet_with_language(opcode, LANG_COMMON_LIKE_CPP, text)
}

fn chat_message_packet_with_language(
    opcode: ClientOpcodes,
    language: i32,
    text: &str,
) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_uint16(opcode as u16);
    writer.write_int32(language);
    writer.write_bits(text.len() as u32, 11);
    writer.write_bit(false);
    writer.write_string(text);

    let mut reader = wow_packet::WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    reader
}

fn chat_channel_message_packet(target: &str, text: &str) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_int32(LANG_COMMON_LIKE_CPP);
    writer.write_packed_guid(&ObjectGuid::EMPTY);
    writer.write_bits(target.len() as u32, 9);
    writer.write_bits(text.len() as u32, 11);
    writer.write_bit(false);
    writer.write_string(target);
    writer.write_string(text);
    wow_packet::WorldPacket::from_bytes(writer.data())
}

fn chat_addon_packet(msg_type: ChatMsg, prefix: &str, text: &str) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_bits(prefix.len() as u32, 5);
    writer.write_bits(text.len() as u32, 8);
    writer.write_bit(false);
    writer.write_int32(msg_type as i32);
    writer.write_string(prefix);
    writer.write_string(text);
    wow_packet::WorldPacket::from_bytes(writer.data())
}

fn chat_addon_targeted_packet(
    msg_type: ChatMsg,
    prefix: &str,
    text: &str,
    target: &str,
    channel_guid: ObjectGuid,
    is_logged: bool,
) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_bits(target.len() as u32, 9);
    writer.write_bits(prefix.len() as u32, 5);
    writer.write_bits(text.len() as u32, 8);
    writer.write_bit(is_logged);
    writer.write_int32(msg_type as i32);
    writer.write_string(prefix);
    writer.write_string(text);
    writer.write_packed_guid(&channel_guid);
    writer.write_string(target);
    wow_packet::WorldPacket::from_bytes(writer.data())
}

fn chat_addon_whisper_packet(target: &str, prefix: &str, message: &str) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_bits(target.len() as u32, 9);
    writer.write_bits(prefix.len() as u32, 5);
    writer.write_bits(message.len() as u32, 8);
    writer.write_string(target);
    writer.write_string(prefix);
    writer.write_string(message);
    wow_packet::WorldPacket::from_bytes(writer.data())
}

fn chat_whisper_packet(target: &str, text: &str) -> wow_packet::WorldPacket {
    chat_whisper_packet_with_language(target, LANG_COMMON_LIKE_CPP, text)
}

fn chat_whisper_packet_with_language(
    target: &str,
    language: i32,
    text: &str,
) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_int32(language);
    writer.write_bits(target.len() as u32, 9);
    writer.write_bits(text.len() as u32, 11);
    writer.write_string(target);
    writer.write_string(text);
    wow_packet::WorldPacket::from_bytes(writer.data())
}

fn chat_away_packet(text: &str) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_bits(text.len() as u32, 11);
    writer.write_string(text);
    wow_packet::WorldPacket::from_bytes(writer.data())
}

fn chat_emote_packet(text: &str) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_bits(text.len() as u32, 11);
    writer.write_string(text);
    wow_packet::WorldPacket::from_bytes(writer.data())
}

fn text_emote_packet(emote_id: i32, sound_index: i32) -> wow_packet::WorldPacket {
    text_emote_packet_with_target_and_visuals(ObjectGuid::EMPTY, emote_id, sound_index, &[], 0)
}

fn text_emote_packet_with_visuals(
    emote_id: i32,
    sound_index: i32,
    spell_visual_kit_ids: &[i32],
    sequence_variation: i32,
) -> wow_packet::WorldPacket {
    text_emote_packet_with_target_and_visuals(
        ObjectGuid::EMPTY,
        emote_id,
        sound_index,
        spell_visual_kit_ids,
        sequence_variation,
    )
}

fn text_emote_packet_with_target(
    target: ObjectGuid,
    emote_id: i32,
    sound_index: i32,
) -> wow_packet::WorldPacket {
    text_emote_packet_with_target_and_visuals(target, emote_id, sound_index, &[], 0)
}

fn text_emote_packet_with_target_and_visuals(
    target: ObjectGuid,
    emote_id: i32,
    sound_index: i32,
    spell_visual_kit_ids: &[i32],
    sequence_variation: i32,
) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_packed_guid(&target);
    writer.write_int32(emote_id);
    writer.write_int32(sound_index);
    writer.write_uint32(spell_visual_kit_ids.len() as u32);
    writer.write_int32(sequence_variation);
    for &id in spell_visual_kit_ids {
        writer.write_int32(id);
    }
    wow_packet::WorldPacket::from_bytes(writer.data())
}

fn chat_register_addon_prefixes_packet(prefixes: &[&str]) -> wow_packet::WorldPacket {
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_uint32(prefixes.len() as u32);
    for prefix in prefixes {
        writer.write_bits(prefix.len() as u32, 5);
        writer.write_string(prefix);
    }
    wow_packet::WorldPacket::from_bytes(writer.data())
}

fn chat_slash_cmd(bytes: &[u8]) -> u8 {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    packet.skip_opcode();
    packet.read_uint8().expect("chat slash command")
}

fn chat_language(bytes: &[u8]) -> u32 {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    packet.skip_opcode();
    let _ = packet.read_uint8().expect("chat slash command");
    packet.read_uint32().expect("chat language")
}

fn chat_text(bytes: &[u8]) -> String {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    packet.skip_opcode();
    let _ = packet.read_uint8().expect("chat slash command");
    let _ = packet.read_uint32().expect("chat language");
    let _ = packet.read_packed_guid().expect("sender guid");
    let _ = packet.read_packed_guid().expect("sender guild guid");
    let _ = packet.read_packed_guid().expect("sender account guid");
    let _ = packet.read_packed_guid().expect("target guid");
    let _ = packet.read_uint32().expect("target virtual realm");
    let _ = packet.read_uint32().expect("sender virtual realm");
    let _ = packet.read_int32().expect("achievement id");
    let _ = packet.read_float().expect("display time");
    let _ = packet.read_int32().expect("spell id");
    let sender_len = packet.read_bits(11).expect("sender len") as usize;
    let target_len = packet.read_bits(11).expect("target len") as usize;
    let prefix_len = packet.read_bits(5).expect("prefix len") as usize;
    let channel_len = packet.read_bits(7).expect("channel len") as usize;
    let text_len = packet.read_bits(12).expect("text len") as usize;
    let _ = packet.read_bits(15).expect("chat flags");
    let _ = packet.read_bit().expect("hide chat log");
    let _ = packet.read_bit().expect("fake sender");
    let _ = packet.read_bit().expect("unused 801");
    let _ = packet.read_bit().expect("channel guid");
    packet.flush_bits();
    let _ = packet.read_string(sender_len).expect("sender name");
    let _ = packet.read_string(target_len).expect("target name");
    let _ = packet.read_string(prefix_len).expect("prefix");
    let _ = packet.read_string(channel_len).expect("channel");
    packet.read_string(text_len).expect("text")
}

fn print_notification_text(bytes: &[u8]) -> String {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        wow_constants::ServerOpcodes::PrintNotification as u16
    );
    let text_len = packet.read_bits(12).expect("notify text len") as usize;
    let text = packet.read_string(text_len).expect("notify text");
    assert!(packet.is_empty());
    text
}

fn emote_message_fields(bytes: &[u8]) -> (i32, Vec<i32>, i32) {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("emote opcode"),
        wow_constants::ServerOpcodes::Emote as u16
    );
    let _ = packet.read_packed_guid().expect("emote guid");
    let emote_id = packet.read_int32().expect("emote id");
    let visual_count = packet.read_int32().expect("visual count") as usize;
    let sequence_variation = packet.read_int32().expect("sequence variation");
    let mut visual_ids = Vec::with_capacity(visual_count);
    for _ in 0..visual_count {
        visual_ids.push(packet.read_int32().expect("visual kit id"));
    }
    assert!(packet.is_empty());
    (emote_id, visual_ids, sequence_variation)
}

fn text_emote_fields(bytes: &[u8]) -> (i32, i32) {
    let (emote_id, sound_index, _) = text_emote_fields_with_target(bytes);
    (emote_id, sound_index)
}

fn text_emote_fields_with_target(bytes: &[u8]) -> (i32, i32, ObjectGuid) {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("text emote opcode"),
        wow_constants::ServerOpcodes::TextEmote as u16
    );
    let _ = packet.read_packed_guid().expect("source guid");
    let _ = packet.read_packed_guid().expect("source account guid");
    let emote_id = packet.read_int32().expect("text emote id");
    let sound_index = packet.read_int32().expect("sound index");
    let target_guid = packet.read_packed_guid().expect("target guid");
    assert!(packet.is_empty());
    (emote_id, sound_index, target_guid)
}

fn set_emotes_text_entries(
    session: &mut WorldSession,
    entries: impl IntoIterator<Item = wow_data::EmotesTextEntry>,
) {
    session
        .set_emotes_text_store_like_cpp(Arc::new(wow_data::EmotesTextStore::from_entries(entries)));
}

fn set_emotes_entries(
    session: &mut WorldSession,
    entries: impl IntoIterator<Item = wow_data::EmotesEntry>,
) {
    session.set_emotes_store_like_cpp(Arc::new(wow_data::EmotesStore::from_entries(entries)));
}

fn emotes_text_entry(id: u32, emote_id: u16) -> wow_data::EmotesTextEntry {
    wow_data::EmotesTextEntry {
        id,
        name: format!("emote{id}"),
        emote_id,
    }
}

fn emotes_entry(id: u32, anim_id: i32) -> wow_data::EmotesEntry {
    wow_data::EmotesEntry {
        id,
        race_mask: 0,
        emote_slash_command: String::new(),
        anim_id,
        emote_flags: 0,
        emote_spec_proc: 0,
        emote_spec_proc_param: 0,
        event_sound_id: 0,
        spell_visual_kit_id: 0,
        class_mask: 0,
    }
}

fn mute_session_for_seconds_like_cpp(session: &mut WorldSession, seconds: u64) {
    let mute_until = wow_core::GameTime::now().as_secs().saturating_add(seconds) as i64;
    session.set_mute_time_like_cpp(mute_until);
}

fn broadcast_info(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> PlayerSessionRegistrationLikeCpp {
    let (command_tx, _command_rx) = flume::bounded(8);
    broadcast_info_with_command_tx(guid, send_tx, command_tx)
}

fn broadcast_info_at(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    position: wow_core::Position,
) -> PlayerSessionRegistrationLikeCpp {
    let mut info = broadcast_info(guid, send_tx);
    info.placement.position = position;
    info
}

fn broadcast_info_at_with_command_tx(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
    position: wow_core::Position,
) -> PlayerSessionRegistrationLikeCpp {
    let mut info = broadcast_info_with_command_tx(guid, send_tx, command_tx);
    info.placement.position = position;
    info
}

fn broadcast_info_with_command_tx(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
) -> PlayerSessionRegistrationLikeCpp {
    PlayerSessionRegistrationLikeCpp {
        identity: crate::session::directory::PlayerDirectoryIdentityLikeCpp {
            player_name: format!("Player{}", guid.counter()),
            account_id: guid.counter() as u32,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            active_expansion: 2,
        },
        placement: crate::session::directory::PlayerDirectoryPlacementLikeCpp {
            map_id: 571,
            instance_id: 0,
            position: wow_core::Position::ZERO,
            is_in_world: true,
            level: 80,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: crate::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

fn gm_silence_aura(slot: u8) -> AuraApplication {
    AuraApplication {
        spell_id: GM_SILENCE_AURA_LIKE_CPP,
        difficulty_id: 0,
        caster_guid: ObjectGuid::EMPTY,
        slot,
        duration_total: 30_000,
        duration_remaining: 30_000,
        stack_count: 1,
        aura_flags: 0x1,
        effect_mask: 0x1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    }
}

fn expect_addon_command(
    rx: &flume::Receiver<SessionCommand>,
) -> SendAddonIfRegisteredLikeCppCommand {
    match rx.try_recv().expect("addon command") {
        SessionCommand::SendAddonIfRegisteredLikeCpp(command) => command,
        other => panic!("expected SendAddonIfRegisteredLikeCpp, got {other:?}"),
    }
}

fn expect_send_if_visible_command(
    rx: &flume::Receiver<SessionCommand>,
) -> crate::session::mailbox::SendIfVisibleLikeCppCommand {
    match rx.try_recv().expect("visible command") {
        SessionCommand::SendIfVisibleLikeCpp(command) => command,
        other => panic!("expected SendIfVisibleLikeCpp, got {other:?}"),
    }
}

fn session_for_chat_routing_like_cpp(
    sender_guid: ObjectGuid,
) -> (WorldSession, Arc<PlayerRegistry>, flume::Receiver<Vec<u8>>) {
    let (packet_tx, packet_rx) = flume::bounded(8);
    drop(packet_tx);
    let (send_tx, send_rx) = flume::bounded(8);
    let mut session = WorldSession::new(
        1,
        "TestAccount".to_string(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "enUS".to_string(),
        packet_rx,
        send_tx.clone(),
    );
    session.set_player_guid(Some(sender_guid));
    session.set_loaded_player_name_like_cpp(format!("Player{}", sender_guid.counter()));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_map_position_like_cpp(571, wow_core::Position::ZERO);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        sender_guid,
        broadcast_info(sender_guid, send_tx),
        Default::default(),
    );
    session.set_player_registry(Arc::clone(&player_registry));
    (session, player_registry, send_rx)
}

fn bind_canonical_player_like_cpp(
    registry: &PlayerRegistry,
    guid: ObjectGuid,
    configure: impl FnOnce(&mut wow_entities::Player),
) -> crate::session::SharedCanonicalMapManager {
    let canonical = registry
        .fixture_canonical_map_manager_like_cpp()
        .expect("canonical player fixture manager");
    let mut manager = canonical.lock().unwrap();
    let map = manager.create_world_map(571, 0).map_mut();
    if map.get_typed_player(guid).is_none() {
        let mut player = wow_entities::Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_map(571, 0).unwrap();
        player.unit_mut().world_mut().object_mut().add_to_world();
        map.insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
    configure(map.get_typed_player_mut(guid).expect("canonical player"));
    drop(manager);
    canonical
}

fn bind_social_presence_like_cpp(
    registry: &PlayerRegistry,
    guid: ObjectGuid,
    flag: u32,
    message: &str,
) {
    bind_canonical_player_like_cpp(registry, guid, |player| {
        player.set_player_flag(flag);
        player.gameplay_state_mut().social.auto_reply_msg_like_cpp = message.to_string();
    });
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;

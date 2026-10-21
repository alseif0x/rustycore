// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `chat.message`, the linked-module pre-decision (#1444 H4), driven through
//! the real chat handlers: AzerothCore `OnPlayerCanUseChat`
//! (`PlayerScript.h:657-709`) after every C++ check, before the send.

use std::sync::Mutex;

use wow_module_api::{
    ChatDecision, ChatKind, ChatMessageSnapshot, ChatModule, ModuleDescriptor, ModuleHooks,
    ModuleRegistry, ModuleVersion, PlayerModule,
};

use super::*;

/// Denies any text with "gold", masks "darn", records every snapshot.
#[derive(Default)]
struct Censor {
    seen: Mutex<Vec<ChatMessageSnapshot>>,
    replacement: Option<&'static str>,
}

impl ChatModule for Censor {
    fn on_chat_message(&self, snapshot: &ChatMessageSnapshot) -> ChatDecision {
        self.seen.lock().unwrap().push(snapshot.clone());
        if snapshot.text.contains("gold") {
            ChatDecision::Deny
        } else if let Some(replacement) = self.replacement {
            ChatDecision::ReplaceText(replacement.to_owned())
        } else if snapshot.text.contains("darn") {
            ChatDecision::ReplaceText(snapshot.text.replace("darn", "****"))
        } else {
            ChatDecision::Allow
        }
    }
}

fn install(session: &mut WorldSession, censor: Arc<Censor>) {
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "alpha.censor",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Censor",
            )
            .unwrap(),
            ModuleHooks::new().chat(censor),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));
}

fn say_session(
    sender: ObjectGuid,
    nearby: ObjectGuid,
    censor: Arc<Censor>,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    install(&mut session, censor);
    (session, sender_rx, nearby_rx)
}

#[tokio::test]
async fn a_chat_module_denies_or_rewrites_a_say_before_it_is_sent() {
    let sender = ObjectGuid::create_player(1, 4401);
    let nearby = ObjectGuid::create_player(1, 4402);
    let censor = Arc::new(Censor::default());
    let (mut session, sender_rx, nearby_rx) = say_session(sender, nearby, Arc::clone(&censor));

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "cheap gold here"),
            ChatMsg::Say,
        )
        .await;
    assert!(sender_rx.try_recv().is_err(), "a denied say is not echoed");
    assert!(
        nearby_rx.try_recv().is_err(),
        "a denied say is not broadcast"
    );

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageYell, "darn it"),
            ChatMsg::Yell,
        )
        .await;
    let echo = sender_rx.try_recv().expect("sender echo");
    assert_eq!(chat_slash_cmd(&echo), ChatMsg::Yell as u8);
    assert_eq!(chat_text(&echo), "**** it");
    assert_eq!(
        chat_text(&nearby_rx.try_recv().expect("broadcast")),
        "**** it"
    );

    let seen = censor.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2, "one dispatch per message");
    assert_eq!(seen[0].kind, ChatKind::Say);
    assert_eq!(seen[1].kind, ChatKind::Yell);
    assert_eq!(seen[1].guid, sender);
    assert_eq!(seen[1].language, LANG_COMMON_LIKE_CPP as u32);
    assert_eq!(seen[1].receiver, None);
}

/// C++ asks the scripts only after its own checks: a message the level gate
/// refuses never reaches a module.
#[tokio::test]
async fn the_chat_hook_runs_after_the_cpp_checks() {
    let sender = ObjectGuid::create_player(1, 4403);
    let nearby = ObjectGuid::create_player(1, 4404);
    let censor = Arc::new(Censor::default());
    let (mut session, sender_rx, _nearby_rx) = say_session(sender, nearby, Arc::clone(&censor));
    session.set_player_level_like_cpp(1);
    session.set_chat_level_requirements_like_cpp(ChatLevelRequirementsLikeCpp {
        say: 2,
        ..ChatLevelRequirementsLikeCpp::default()
    });

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "too low"),
            ChatMsg::Say,
        )
        .await;

    let _level_notice = sender_rx.try_recv();
    assert!(censor.seen.lock().unwrap().is_empty());
}

/// A replaced text passes the client-text rules again; one they refuse drops
/// the message instead of falling back to the text the module replaced.
#[tokio::test]
async fn a_replaced_text_the_client_rules_refuse_drops_the_message() {
    for bad in ["bad\u{1}text", ""] {
        let sender = ObjectGuid::create_player(1, 4405);
        let nearby = ObjectGuid::create_player(1, 4406);
        let censor = Arc::new(Censor {
            replacement: Some(bad),
            ..Censor::default()
        });
        let (mut session, sender_rx, nearby_rx) = say_session(sender, nearby, censor);
        session
            .handle_chat_message(
                chat_message_packet(ClientOpcodes::ChatMessageSay, "hello"),
                ChatMsg::Say,
            )
            .await;
        assert!(sender_rx.try_recv().is_err(), "{bad:?} is not echoed");
        assert!(nearby_rx.try_recv().is_err(), "{bad:?} is not broadcast");
    }
    let long: &'static str = Box::leak("x".repeat(512).into_boxed_str());
    let (mut session, sender_rx, _nearby_rx) = say_session(
        ObjectGuid::create_player(1, 4407),
        ObjectGuid::create_player(1, 4408),
        Arc::new(Censor {
            replacement: Some(long),
            ..Censor::default()
        }),
    );
    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "hello"),
            ChatMsg::Say,
        )
        .await;
    assert!(sender_rx.try_recv().is_err(), "a 512-byte text is refused");
}

#[tokio::test]
async fn a_whisper_reports_its_receiver_and_sends_the_replaced_text() {
    let sender = ObjectGuid::create_player(1, 4409);
    let target = ObjectGuid::create_player(1, 4410);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());
    let censor = Arc::new(Censor::default());
    install(&mut session, Arc::clone(&censor));

    session
        .handle_chat_whisper(chat_whisper_packet("Target", "darn you"))
        .await;

    assert_eq!(
        chat_text(&target_rx.try_recv().expect("whisper")),
        "**** you"
    );
    assert_eq!(
        chat_text(&sender_rx.try_recv().expect("inform")),
        "**** you"
    );
    let seen = censor.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].kind, ChatKind::Whisper);
    assert_eq!(seen[0].receiver, Some(target));
    assert_eq!(seen[0].language, 0, "AzerothCore passes LANG_UNIVERSAL");
}

#[tokio::test]
async fn a_chat_module_can_deny_a_text_emote() {
    let sender = ObjectGuid::create_player(1, 4411);
    let nearby = ObjectGuid::create_player(1, 4412);
    let censor = Arc::new(Censor::default());
    let (mut session, sender_rx, nearby_rx) = say_session(sender, nearby, Arc::clone(&censor));

    session
        .handle_chat_emote(chat_emote_packet("waves gold"))
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
    assert_eq!(censor.seen.lock().unwrap()[0].kind, ChatKind::Emote);
}

/// A module that offers only the player family is never asked about chat,
/// and the text goes out untouched.
#[tokio::test]
async fn a_module_without_the_chat_family_leaves_chat_untouched() {
    struct OnlyPlayer;
    impl PlayerModule for OnlyPlayer {}
    let sender = ObjectGuid::create_player(1, 4413);
    let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "alpha.player",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Player",
            )
            .unwrap(),
            ModuleHooks::new().player(Arc::new(OnlyPlayer)),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "darn gold"),
            ChatMsg::Say,
        )
        .await;

    assert_eq!(chat_text(&sender_rx.try_recv().expect("echo")), "darn gold");
}

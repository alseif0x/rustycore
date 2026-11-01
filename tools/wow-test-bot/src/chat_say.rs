//! Login-only chat phase: record every `SMSG_CHAT`, say lines, and assert
//! which ones the server echoed back (#1444 H5, the `chat.message` veto).
//!
//! Inputs, all optional (`|` separates entries):
//! - `WOW_BOT_CHAT_SAY`: lines to say that the server must echo back;
//! - `WOW_BOT_CHAT_SAY_BLOCKED`: lines to say that must never be echoed (a
//!   module denied them; C++ `Player::Say` echoes an allowed line to its
//!   sender through `SendMessageToSetInRange(..., self = true)`);
//! - `WOW_BOT_EXPECT_SYSTEM_CHAT`: substrings that must each appear in a
//!   system message received since login (a module's greeting, say);
//! - `WOW_BOT_CHAT_LANGUAGE`: the `LANG_*` id to say in (default 7, Common).
//!
//! The blocked lines are said first and observed until the end of the phase,
//! so an allowed line's echo proves the pipeline delivered while the blocked
//! one stayed silent.

use super::{
    build_chat_message_say_payload, build_time_sync_response_payload, parse_chat_summary,
    parse_time_sync_request_sequence, read_encrypted_packet, send_encrypted_packet, BotRunResult,
    EncryptedWorldConnection, ServerPacketInflater, WorldCrypt, CHAT_MSG_SAY, CHAT_MSG_SYSTEM,
    CMSG_TIME_SYNC_RESPONSE, SMSG_TIME_SYNC_REQUEST,
};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::info;

pub(crate) const SMSG_CHAT: u16 = 0x2BAD;
pub(crate) const CMSG_CHAT_MESSAGE_SAY: u16 = 0x37E7;
const LANG_COMMON: i32 = 7;
/// How long each said line is observed before the next one.
const OBSERVE_EACH: Duration = Duration::from_secs(3);

/// One decoded `SMSG_CHAT`.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub(crate) struct ChatSeen {
    pub(crate) msg_type: u8,
    pub(crate) sender: String,
    pub(crate) text: String,
}

/// What the chat phase sent, saw and concluded.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Evidence {
    pub(crate) said: Vec<String>,
    pub(crate) said_blocked: Vec<String>,
    pub(crate) echoed: Vec<String>,
    pub(crate) blocked_echoed: Vec<String>,
    pub(crate) expected_system: Vec<String>,
    pub(crate) missing_system: Vec<String>,
    pub(crate) passed: bool,
}

fn list(name: &str) -> Vec<String> {
    std::env::var(name)
        .unwrap_or_default()
        .split('|')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Record an `SMSG_CHAT` in the report. The shared reader already logged it.
pub(crate) fn record(_bot_index: usize, payload: &[u8], result: &mut BotRunResult) {
    if let Some(chat) = parse_chat_summary(payload) {
        result.chat_received.push(ChatSeen {
            msg_type: chat.msg_type,
            sender: chat.sender_name,
            text: chat.text,
        });
    }
}

/// Whether the phase has anything to do.
pub(crate) fn requested() -> bool {
    [
        "WOW_BOT_CHAT_SAY",
        "WOW_BOT_CHAT_SAY_BLOCKED",
        "WOW_BOT_EXPECT_SYSTEM_CHAT",
    ]
    .iter()
    .any(|name| !list(name).is_empty())
}

/// Read packets from both sockets for `window`, recording chat.
async fn observe(
    bot_index: usize,
    window: Duration,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    result: &mut BotRunResult,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        let mut instance_peek = [0; 1];
        let mut realm_peek = [0; 1];
        // Peek is cancellation-safe: never race partial encrypted frame reads.
        let ready = tokio::time::timeout(remaining, async {
            tokio::select! {
                bytes = stream.peek(&mut instance_peek) => {
                    if bytes? == 0 { bail!("instance closed during the chat phase"); }
                    Ok::<bool, anyhow::Error>(true)
                }
                bytes = realm.stream.peek(&mut realm_peek) => {
                    if bytes? == 0 { bail!("realm closed during the chat phase"); }
                    Ok(false)
                }
            }
        })
        .await;
        let instance = match ready {
            Ok(ready) => ready?,
            Err(_) => return Ok(()),
        };
        let (opcode, payload) = tokio::time::timeout(Duration::from_secs(10), async {
            if instance {
                read_encrypted_packet(stream, crypt, inflater).await
            } else {
                read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater)
                    .await
            }
        })
        .await
        .context("chat phase frame deadline exceeded")??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_CHAT {
            record(bot_index, &payload, result);
        } else if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
        }
    }
}

fn echoed(result: &BotRunResult, text: &str) -> bool {
    result
        .chat_received
        .iter()
        .any(|chat| chat.msg_type == CHAT_MSG_SAY && chat.text == text)
}

/// Judge the evidence from what was received. Pure, so it is unit-tested.
pub(crate) fn judge(
    said: Vec<String>,
    said_blocked: Vec<String>,
    expected_system: Vec<String>,
    result: &BotRunResult,
) -> Evidence {
    let echoed_lines: Vec<String> = said
        .iter()
        .filter(|text| echoed(result, text))
        .cloned()
        .collect();
    let blocked_echoed: Vec<String> = said_blocked
        .iter()
        .filter(|text| echoed(result, text))
        .cloned()
        .collect();
    let missing_system: Vec<String> = expected_system
        .iter()
        .filter(|needle| {
            !result
                .chat_received
                .iter()
                .any(|chat| chat.msg_type == CHAT_MSG_SYSTEM && chat.text.contains(needle.as_str()))
        })
        .cloned()
        .collect();
    let passed =
        echoed_lines.len() == said.len() && blocked_echoed.is_empty() && missing_system.is_empty();
    Evidence {
        said,
        said_blocked,
        echoed: echoed_lines,
        blocked_echoed,
        expected_system,
        missing_system,
        passed,
    }
}

/// Run the phase after the login streams drained. Fails the run when a
/// requested assertion does not hold; the evidence is recorded first.
pub(crate) async fn run(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) -> Result<()> {
    let realm = realm
        .as_mut()
        .context("the chat phase requires both sockets")?;
    let language = std::env::var("WOW_BOT_CHAT_LANGUAGE")
        .ok()
        .map(|raw| raw.parse::<i32>())
        .transpose()
        .context("WOW_BOT_CHAT_LANGUAGE must be an integer")?
        .unwrap_or(LANG_COMMON);
    let said_blocked = list("WOW_BOT_CHAT_SAY_BLOCKED");
    let said = list("WOW_BOT_CHAT_SAY");
    for text in said_blocked.iter().chain(said.iter()) {
        info!("[Bot {}] 💬 CMSG_CHAT_MESSAGE_SAY {:?}", bot_index, text);
        send_encrypted_packet(
            stream,
            crypt,
            CMSG_CHAT_MESSAGE_SAY,
            &build_chat_message_say_payload(language, text),
        )
        .await?;
        observe(
            bot_index,
            OBSERVE_EACH,
            stream,
            crypt,
            inflater,
            realm,
            result,
        )
        .await?;
    }
    let evidence = judge(
        said,
        said_blocked,
        list("WOW_BOT_EXPECT_SYSTEM_CHAT"),
        result,
    );
    info!(
        "[Bot {}] 💬 chat phase passed={} echoed={:?} blocked_echoed={:?} missing_system={:?}",
        bot_index,
        evidence.passed,
        evidence.echoed,
        evidence.blocked_echoed,
        evidence.missing_system
    );
    let passed = evidence.passed;
    result.chat_say = Some(evidence);
    if !passed {
        bail!("chat phase assertions failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(msg_type: u8, text: &str) -> ChatSeen {
        ChatSeen {
            msg_type,
            sender: String::new(),
            text: text.to_owned(),
        }
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn allowed_lines_must_echo_blocked_lines_must_not_and_system_text_must_arrive() {
        let mut result = BotRunResult::default();
        result.chat_received = vec![
            seen(
                CHAT_MSG_SYSTEM,
                "Welcome back, Bot! A milestone every 1 kills.",
            ),
            seen(CHAT_MSG_SAY, "hello there"),
        ];
        let pass = judge(
            strings(&["hello there"]),
            strings(&["a spoiler here"]),
            strings(&["Welcome"]),
            &result,
        );
        assert!(pass.passed);

        let missing_echo = judge(strings(&["not echoed"]), vec![], vec![], &result);
        assert!(!missing_echo.passed);

        result
            .chat_received
            .push(seen(CHAT_MSG_SAY, "a spoiler here"));
        let leaked = judge(vec![], strings(&["a spoiler here"]), vec![], &result);
        assert!(!leaked.passed);
        assert_eq!(leaked.blocked_echoed, strings(&["a spoiler here"]));

        let say_is_not_system = judge(vec![], vec![], strings(&["hello there"]), &result);
        assert!(
            !say_is_not_system.passed,
            "a say line is not a system message"
        );
    }
}

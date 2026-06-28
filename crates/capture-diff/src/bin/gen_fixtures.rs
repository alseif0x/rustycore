//! Reproducible generator for the committed `login` golden fixtures.
//!
//! Run with `cargo run -p capture-diff --bin gen-fixtures`. It writes, under
//! `crates/capture-diff/flows/login/`:
//!
//! - `cpp.pkt` — a synthetic but **byte-faithful PKT 3.1** golden modelling the
//!   C++ login s2c burst;
//! - `rust/` — a synthetic RustyCore dump modelling the current Rust login
//!   output, with the divergences catalogued in
//!   `docs/migration/world-load-audit.md` baked in;
//! - `expected-divergences.json` — the accepted-divergence baseline;
//! - `flow.json` — the flow descriptor.
//!
//! The fixtures are **synthetic and clearly labelled**: they exist to exercise
//! and regression-lock the harness end to end without committing real
//! (PII-bearing, session-specific) captures. Before the login flow is declared
//! "capture-clean" per STATE.md §5, replace `cpp.pkt`/`rust/` with a live
//! capture pair (see `scripts/`), then `update-baseline`.
//!
//! Opcodes are the real RustyCore values so names resolve; bodies are short
//! placeholders chosen to surface each divergence class.

// Product names (RustyCore, TrinityCore) appear throughout the docs as prose.
#![allow(clippy::doc_markdown)]

use anyhow::Result;

use capture_diff::diff::DiffReport;
use capture_diff::model::{Capture, CapturedPacket, Direction};
use capture_diff::{flow, pkt, rustdump};

// ── Real s2c opcodes (wow-constants ServerOpcodes) ───────────────────
const AUTH_RESPONSE: u16 = 0x256d;
const FEATURE_SYSTEM_STATUS: u16 = 0x25bf;
const ACCOUNT_DATA_TIMES: u16 = 0x270a;
const TUTORIAL_FLAGS: u16 = 0x27be;
const BIND_POINT_UPDATE: u16 = 0x257d;
const SET_PROFICIENCY: u16 = 0x2735;
const SEND_KNOWN_SPELLS: u16 = 0x2c27;
const ACTIVE_GLYPHS: u16 = 0x2c51;
const UPDATE_TALENT_DATA: u16 = 0x25d7;
const CHAT_SERVER_MESSAGE: u16 = 0x2bc5;
const QUERY_PLAYER_NAMES_RESPONSE: u16 = 0x301b;
const LFG_LIST_UPDATE_BLACKLIST: u16 = 0x2a2a;
const LOGIN_SET_TIME_SPEED: u16 = 0x270d;

// ── Real c2s opcodes (ClientOpcodes) ─────────────────────────────────
const PLAYER_LOGIN: u16 = 0x35eb; // CMSG_PLAYER_LOGIN
const VIOLENCE_LEVEL: u16 = 0x3187;
const REQUEST_PLAYED_TIME: u16 = 0x327a;

fn s2c(opcode: u16, body: &[u8]) -> CapturedPacket {
    CapturedPacket {
        direction: Direction::S2C,
        opcode,
        body: body.to_vec(),
    }
}

fn c2s(opcode: u16, body: &[u8]) -> CapturedPacket {
    CapturedPacket {
        direction: Direction::C2S,
        opcode,
        body: body.to_vec(),
    }
}

/// The C++ login skeleton (the golden / reference behaviour).
fn cpp_capture() -> Capture {
    let mut p = Vec::new();
    // Client → server: the same client drives both servers, so c2s is identical.
    p.extend(client_stream());

    // Server → client login burst (the "correct" order/content).
    p.push(s2c(AUTH_RESPONSE, &[0x0d, 0x00, 0x00, 0x00]));
    p.push(s2c(FEATURE_SYSTEM_STATUS, &[0x02, 0x01, 0x01, 0x00, 0xa0])); // config-populated
    p.push(s2c(ACCOUNT_DATA_TIMES, &[0x00, 0x00])); // global resend (HandlePlayerLogin)
    p.push(s2c(TUTORIAL_FLAGS, &[0x00, 0x00, 0x00, 0x00])); // SendTutorialsData resend
    p.push(s2c(BIND_POINT_UPDATE, &[0x01, 0x00, 0x00, 0x00]));
    // 6 proficiency packets with accumulated masks (audit cpp-s2c-61..66).
    for mask in [
        [0x00u8, 0x04, 0x00, 0x00, 0x02],
        [0x10, 0x04, 0x00, 0x00, 0x02],
        [0x10, 0x04, 0x08, 0x00, 0x02],
        [0x02, 0x00, 0x00, 0x00, 0x04],
        [0x10, 0x44, 0x08, 0x00, 0x02],
        [0x03, 0x00, 0x00, 0x00, 0x04],
    ] {
        p.push(s2c(SET_PROFICIENCY, &mask));
    }
    p.push(s2c(SEND_KNOWN_SPELLS, &[0x01, 0x00])); // KnownSpells BEFORE ActiveGlyphs
    p.push(s2c(ACTIVE_GLYPHS, &[0x00, 0x00]));
    p.push(s2c(UPDATE_TALENT_DATA, &[0x00]));
    // MOTD: 4 configured lines.
    for line in 0..4u8 {
        p.push(s2c(CHAT_SERVER_MESSAGE, &[0x03, line]));
    }
    p.push(s2c(LOGIN_SET_TIME_SPEED, &[0x00, 0x00, 0x00, 0x00]));

    Capture::new("synthetic-cpp-login", p)
}

/// The Rust login output, with world-load-audit divergences baked in.
fn rust_capture() -> Capture {
    let mut p = Vec::new();
    p.extend(client_stream()); // identical c2s

    p.push(s2c(AUTH_RESPONSE, &[0x0d, 0x00, 0x00, 0x00])); // identical → clean match
    p.push(s2c(FEATURE_SYSTEM_STATUS, &[0x00, 0x00, 0x00, 0x00, 0x00])); // default_wotlk → VALUE diff
    // MISSING: global AccountDataTimes resend (#1202)
    // MISSING: TutorialFlags resend (#1203)
    p.push(s2c(BIND_POINT_UPDATE, &[0x01, 0x00, 0x00, 0x00])); // identical
    // 8 proficiency packets, different masks/order (#1201) → count + value diffs.
    for mask in [
        [0x10u8, 0x00, 0x00, 0x00, 0x02],
        [0x02, 0x00, 0x00, 0x00, 0x04],
        [0x10, 0x00, 0x08, 0x00, 0x02],
        [0x10, 0x04, 0x08, 0x00, 0x02],
        [0x10, 0x44, 0x08, 0x00, 0x02],
        [0x03, 0x00, 0x00, 0x00, 0x04],
        [0x03, 0x02, 0x00, 0x00, 0x04],
        [0x03, 0x06, 0x00, 0x00, 0x04],
    ] {
        p.push(s2c(SET_PROFICIENCY, &mask));
    }
    p.push(s2c(ACTIVE_GLYPHS, &[0x00, 0x00])); // ActiveGlyphs BEFORE KnownSpells (#1206)
    p.push(s2c(QUERY_PLAYER_NAMES_RESPONSE, &[0x00])); // injected mid-burst (#1207)
    p.push(s2c(SEND_KNOWN_SPELLS, &[0x01, 0x00]));
    p.push(s2c(UPDATE_TALENT_DATA, &[0x00]));
    p.push(s2c(CHAT_SERVER_MESSAGE, &[0x03, 0x00])); // single MOTD line (#1205)
    p.push(s2c(LFG_LIST_UPDATE_BLACKLIST, &[0x00])); // extra packet (#1208)
    p.push(s2c(LOGIN_SET_TIME_SPEED, &[0x00, 0x00, 0x00, 0x00]));

    Capture::new("synthetic-rust-login", p)
}

/// Identical client→server stream both servers receive.
fn client_stream() -> Vec<CapturedPacket> {
    vec![
        c2s(
            PLAYER_LOGIN,
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        ),
        c2s(VIOLENCE_LEVEL, &[0x02]),
        c2s(REQUEST_PLAYED_TIME, &[0x01]),
    ]
}

fn main() -> Result<()> {
    let login_dir = flow::flows_root().join("login");
    std::fs::create_dir_all(&login_dir)?;

    let cpp = cpp_capture();
    let rust = rust_capture();

    // cpp.pkt (real PKT 3.1 bytes)
    let pkt_bytes = pkt::write_pkt_bytes(&cpp);
    std::fs::write(login_dir.join("cpp.pkt"), &pkt_bytes)?;

    // rust/ dump dir
    let rust_dir = login_dir.join("rust");
    if rust_dir.exists() {
        std::fs::remove_dir_all(&rust_dir)?;
    }
    rustdump::write_rust_dump(&rust_dir, &rust)?;

    // expected-divergences.json baseline (s2c + c2s)
    let directions = [Direction::S2C, Direction::C2S];
    let report = DiffReport::compute(&cpp, &rust, &directions);
    let baseline = serde_json::to_string_pretty(&report.signatures())?;
    std::fs::write(login_dir.join("expected-divergences.json"), baseline)?;

    // flow.json descriptor
    let flow_json = serde_json::json!({
        "description": "Login → enter world (synthetic golden; replace with live capture per STATE.md §5)",
        "directions": ["s2c", "c2s"],
    });
    std::fs::write(
        login_dir.join("flow.json"),
        serde_json::to_string_pretty(&flow_json)?,
    )?;

    println!(
        "generated login fixtures in {} ({} divergences)",
        login_dir.display(),
        report.signatures().len()
    );
    println!("{}", report.render_text());
    Ok(())
}

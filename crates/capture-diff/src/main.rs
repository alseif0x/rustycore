//! `capture-diff` — CLI for the C++-vs-Rust packet capture diff harness.
//!
//! ```text
//!   capture-diff diff <flow> [--rust DIR] [--cpp PKT] [--direction s2c|c2s|both]
//!                            [--from-opcode ...] [--until-opcode ...]
//!                            [--ignore-opcode s2c:0xNNNN]
//!                            [--json] [--baseline FILE] [--strict]
//!   capture-diff diff --cpp A.pkt --rust DIR [...]        # ad-hoc, no flow
//!   capture-diff show <PKT|DUMPDIR>                       # list a capture
//!   capture-diff list                                     # list known flows
//!   capture-diff verify-required <flow>                   # require clean real artifacts
//!   capture-diff update-baseline <flow> [--rust DIR]      # rewrite baseline
//! ```

// Product names (RustyCore, TrinityCore) appear throughout the docs as prose.
#![allow(clippy::doc_markdown)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

use capture_diff::diff::{self, BaselineDelta, DivergenceSignature};
use capture_diff::{Capture, DiffReport, Direction, PacketBoundary, flow, lineage, pkt, rustdump};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("capture-diff: error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(ExitCode::FAILURE);
    };
    let rest: Vec<String> = args.collect();

    match cmd.as_str() {
        "diff" => cmd_diff(&rest),
        "show" => cmd_show(&rest),
        "list" => cmd_list(),
        "import" => cmd_import(&rest),
        "verify-required" => cmd_verify_required(&rest),
        "update-baseline" => cmd_update_baseline(&rest),
        "-h" | "--help" | "help" => {
            print_usage();
            Ok(ExitCode::SUCCESS)
        }
        other => bail!(
            "unknown command '{other}' (try: diff, show, list, verify-required, update-baseline)"
        ),
    }
}

/// Parsed flags shared by `diff` and `update-baseline`.
struct Opts {
    positional: Option<String>,
    cpp: Option<PathBuf>,
    cpp_manifest: Option<PathBuf>,
    rust: Option<PathBuf>,
    rust_manifest: Option<PathBuf>,
    direction: Option<Vec<Direction>>,
    baseline: Option<PathBuf>,
    from_opcode: Option<PacketBoundary>,
    until_opcode: Option<PacketBoundary>,
    ignored_opcodes: Vec<PacketBoundary>,
    json: bool,
    strict: bool,
}

fn parse_opts(args: &[String]) -> Result<Opts> {
    let mut opts = Opts {
        positional: None,
        cpp: None,
        cpp_manifest: None,
        rust: None,
        rust_manifest: None,
        direction: None,
        baseline: None,
        from_opcode: None,
        until_opcode: None,
        ignored_opcodes: Vec::new(),
        json: false,
        strict: false,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--cpp" => opts.cpp = Some(PathBuf::from(next(&mut it, "--cpp")?)),
            "--cpp-manifest" => {
                opts.cpp_manifest = Some(PathBuf::from(next(&mut it, "--cpp-manifest")?));
            }
            "--rust" => opts.rust = Some(PathBuf::from(next(&mut it, "--rust")?)),
            "--rust-manifest" => {
                opts.rust_manifest = Some(PathBuf::from(next(&mut it, "--rust-manifest")?));
            }
            "--baseline" => opts.baseline = Some(PathBuf::from(next(&mut it, "--baseline")?)),
            "--from-opcode" => {
                let raw = next(&mut it, "--from-opcode")?;
                let boundary = parse_packet_boundary(&raw, "--from-opcode")?;
                if boundary.direction.is_none() {
                    bail!("--from-opcode requires a direction (for example c2s:0x318C)");
                }
                opts.from_opcode = Some(boundary);
            }
            "--until-opcode" => {
                let raw = next(&mut it, "--until-opcode")?;
                opts.until_opcode = Some(parse_packet_boundary(&raw, "--until-opcode")?);
            }
            "--ignore-opcode" => {
                let raw = next(&mut it, "--ignore-opcode")?;
                let ignored = parse_packet_boundary(&raw, "--ignore-opcode")?;
                if ignored.direction.is_none() {
                    bail!("--ignore-opcode requires a direction (for example s2c:0x2DD4)");
                }
                opts.ignored_opcodes.push(ignored);
            }
            "--direction" => {
                opts.direction = Some(parse_directions(&next(&mut it, "--direction")?)?);
            }
            "--json" => opts.json = true,
            "--strict" => opts.strict = true,
            other if other.starts_with('-') => bail!("unknown flag '{other}'"),
            other => {
                if opts.positional.is_some() {
                    bail!("unexpected extra argument '{other}'");
                }
                opts.positional = Some(other.to_string());
            }
        }
    }
    Ok(opts)
}

fn parse_packet_boundary(raw: &str, flag: &str) -> Result<PacketBoundary> {
    let raw = raw.trim();
    let (direction, opcode) = match raw.split_once(':') {
        Some((direction, opcode)) => {
            let direction = match direction.trim().to_ascii_lowercase().as_str() {
                "c2s" => Direction::C2S,
                "s2c" => Direction::S2C,
                other => bail!("invalid {flag} direction '{other}' (use c2s:0xNNNN or s2c:0xNNNN)"),
            };
            (Some(direction), opcode)
        }
        None => (None, raw),
    };
    let hex = opcode
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    let opcode = u16::from_str_radix(hex, 16)
        .with_context(|| format!("invalid {flag} '{raw}' (expected 0xNNNN or direction:0xNNNN)"))?;
    Ok(PacketBoundary { direction, opcode })
}

fn next(it: &mut std::slice::Iter<'_, String>, flag: &str) -> Result<String> {
    it.next()
        .cloned()
        .with_context(|| format!("flag {flag} requires a value"))
}

fn parse_directions(spec: &str) -> Result<Vec<Direction>> {
    match spec {
        "s2c" => Ok(vec![Direction::S2C]),
        "c2s" => Ok(vec![Direction::C2S]),
        "both" => Ok(vec![Direction::S2C, Direction::C2S]),
        other => bail!("invalid --direction '{other}' (use s2c, c2s, or both)"),
    }
}

/// Reviewed periodic server traffic that may be removed from an isolated
/// action window. Keep this allowlist deliberately small: a free-form filter
/// could hide the request or response whose parity the flow is meant to prove.
const APPROVED_AMBIENT_IGNORES: &[PacketBoundary] = &[
    PacketBoundary {
        direction: Some(Direction::S2C),
        opcode: 0x2DD2, // SMSG_TIME_SYNC_REQUEST
    },
    PacketBoundary {
        direction: Some(Direction::C2S),
        opcode: 0x3A3D, // CMSG_TIME_SYNC_RESPONSE paired with the request above
    },
    PacketBoundary {
        direction: Some(Direction::S2C),
        opcode: 0x2DD4, // SMSG_ON_MONSTER_MOVE
    },
];

fn boundaries_overlap(left: PacketBoundary, right: PacketBoundary) -> bool {
    left.opcode == right.opcode
        && (left.direction.is_none()
            || right.direction.is_none()
            || left.direction == right.direction)
}

fn validate_ignored_opcodes(opts: &Opts) -> Result<()> {
    let ignores_time_request = opts.ignored_opcodes.contains(&PacketBoundary {
        direction: Some(Direction::S2C),
        opcode: 0x2DD2,
    });
    let ignores_time_response = opts.ignored_opcodes.contains(&PacketBoundary {
        direction: Some(Direction::C2S),
        opcode: 0x3A3D,
    });
    if ignores_time_request != ignores_time_response {
        bail!(
            "time-sync ambient traffic may be ignored only as the reviewed s2c:0x2DD2 + c2s:0x3A3D request/response pair"
        );
    }
    for ignored in &opts.ignored_opcodes {
        if opts
            .from_opcode
            .is_some_and(|boundary| boundaries_overlap(boundary, *ignored))
            || opts
                .until_opcode
                .is_some_and(|boundary| boundaries_overlap(boundary, *ignored))
        {
            bail!("--ignore-opcode {ignored} cannot remove an action boundary");
        }
        let reviewed_detour_combat_values = opts.positional.as_deref()
            == Some("detour-chase-around-obstacle")
            && *ignored
                == (PacketBoundary {
                    direction: Some(Direction::S2C),
                    opcode: 0x27CB,
                });
        if !APPROVED_AMBIENT_IGNORES.contains(ignored) && !reviewed_detour_combat_values {
            bail!(
                "--ignore-opcode {ignored} is not approved ambient traffic for this flow"
            );
        }
    }
    for (index, ignored) in opts.ignored_opcodes.iter().enumerate() {
        if opts.ignored_opcodes[..index].contains(ignored) {
            bail!("--ignore-opcode {ignored} is duplicated");
        }
    }
    Ok(())
}

/// Apply the same checked action boundaries when comparing or importing
/// captures. Keeping this centralized prevents a CLI from accepting a boundary
/// flag and silently comparing the full capture.
fn apply_capture_boundaries(capture: Capture, opts: &Opts) -> Result<Capture> {
    if opts.from_opcode.is_some() && opts.until_opcode.is_none() {
        bail!("--from-opcode requires --until-opcode");
    }
    match (opts.from_opcode, opts.until_opcode) {
        (Some(from), Some(until)) => capture.sliced_between(from, until),
        (None, Some(until)) => capture.truncated_after_first_boundary(until),
        (None, None) => Ok(capture),
        (Some(_), None) => unreachable!("validated above"),
    }
}

/// Drop explicitly declared ambient packets after selecting the action window.
/// The filter is applied symmetrically to C++ and Rust captures and requires a
/// direction, so an unrelated opcode in the opposite wire direction cannot be
/// hidden accidentally.
fn apply_ignored_opcodes(mut capture: Capture, opts: &Opts) -> Capture {
    capture.packets.retain(|packet| {
        !opts
            .ignored_opcodes
            .iter()
            .any(|ignored| ignored.matches(packet))
    });
    capture
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct AmbientIgnoreCounts {
    time_sync_request: usize,
    time_sync_response: usize,
    monster_move: usize,
}

fn read_le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn monster_take<'a>(body: &'a [u8], cursor: &mut usize, len: usize) -> Option<&'a [u8]> {
    let end = cursor.checked_add(len)?;
    let bytes = body.get(*cursor..end)?;
    *cursor = end;
    Some(bytes)
}

fn monster_read_u32(body: &[u8], cursor: &mut usize) -> Option<u32> {
    let bytes: [u8; 4] = monster_take(body, cursor, 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn monster_read_f32(body: &[u8], cursor: &mut usize) -> Option<()> {
    let value = f32::from_bits(monster_read_u32(body, cursor)?);
    value.is_finite().then_some(())
}

fn monster_read_packed_guid(body: &[u8], cursor: &mut usize) -> Option<bool> {
    let low_mask = *monster_take(body, cursor, 1)?.first()?;
    let high_mask = *monster_take(body, cursor, 1)?.first()?;
    let encoded_len = (low_mask.count_ones() + high_mask.count_ones()) as usize;
    let encoded = monster_take(body, cursor, encoded_len)?;
    // A set mask bit encoding zero is non-canonical; empty masks are valid for
    // optional GUID fields such as an absent transport.
    encoded.iter().all(|byte| *byte != 0).then_some(())?;
    Some(low_mask != 0 || high_mask != 0)
}

/// Parse the exact C++ `MonsterMove::Write` / `MovementMonsterSpline` layout
/// far enough to prove that an ignored `SMSG_ON_MONSTER_MOVE` is one complete,
/// canonical movement packet rather than arbitrary bytes under an allowlisted
/// opcode. Counts and optional sections are consumed to the exact body end.
fn valid_monster_move_body(body: &[u8]) -> bool {
    let mut cursor = 0usize;
    if monster_read_packed_guid(body, &mut cursor) != Some(true) {
        return false;
    }
    for _ in 0..3 {
        if monster_read_f32(body, &mut cursor).is_none() {
            return false;
        }
    }
    if monster_read_u32(body, &mut cursor).is_none() {
        return false;
    }
    for _ in 0..3 {
        if monster_read_f32(body, &mut cursor).is_none() {
            return false;
        }
    }
    // CrzTeleport(1) + StopDistanceTolerance(3) are MSB-first and flushed;
    // the low nibble is canonical zero padding.
    let Some(spline_bits) = monster_take(body, &mut cursor, 1)
        .and_then(|b| b.first())
        .copied()
    else {
        return false;
    };
    if spline_bits & 0x0F != 0 || monster_take(body, &mut cursor, 17).is_none() {
        return false;
    }
    if monster_read_packed_guid(body, &mut cursor).is_none()
        || monster_take(body, &mut cursor, 1).is_none()
    {
        return false;
    }
    let Some(bits) = monster_take(body, &mut cursor, 5) else {
        return false;
    };
    let header = bits
        .iter()
        .fold(0u64, |value, byte| (value << 8) | u64::from(*byte));
    let face = ((header >> 38) & 0x03) as u8;
    let point_count = ((header >> 22) & 0xFFFF) as usize;
    let packed_delta_count = ((header >> 4) & 0xFFFF) as usize;
    let has_filter = header & (1 << 3) != 0;
    let has_spell_extra = header & (1 << 2) != 0;
    let has_jump_extra = header & (1 << 1) != 0;
    let has_anim_transition = header & 1 != 0;

    if has_filter {
        let Some(filter_count) = monster_read_u32(body, &mut cursor).map(|v| v as usize) else {
            return false;
        };
        if monster_read_f32(body, &mut cursor).is_none()
            || monster_take(body, &mut cursor, 2).is_none()
            || monster_read_f32(body, &mut cursor).is_none()
            || monster_take(body, &mut cursor, 2).is_none()
            || monster_take(body, &mut cursor, filter_count.saturating_mul(4)).is_none()
        {
            return false;
        }
        let Some(filter_flags) = monster_take(body, &mut cursor, 1)
            .and_then(|bytes| bytes.first())
            .copied()
        else {
            return false;
        };
        if filter_flags & 0x3F != 0 {
            return false;
        }
    }

    let face_ok = match face {
        0 => true,
        1 => (0..3).all(|_| monster_read_f32(body, &mut cursor).is_some()),
        2 => {
            monster_read_f32(body, &mut cursor).is_some()
                && monster_read_packed_guid(body, &mut cursor).is_some()
        }
        3 => monster_read_f32(body, &mut cursor).is_some(),
        _ => false,
    };
    if !face_ok {
        return false;
    }
    for _ in 0..point_count.saturating_mul(3) {
        if monster_read_f32(body, &mut cursor).is_none() {
            return false;
        }
    }
    if monster_take(body, &mut cursor, packed_delta_count.saturating_mul(4)).is_none() {
        return false;
    }
    if has_spell_extra
        && (monster_read_packed_guid(body, &mut cursor).is_none()
            || monster_take(body, &mut cursor, 12).is_none()
            || monster_read_f32(body, &mut cursor).is_none())
    {
        return false;
    }
    if has_jump_extra
        && (monster_read_f32(body, &mut cursor).is_none()
            || monster_take(body, &mut cursor, 8).is_none())
    {
        return false;
    }
    if has_anim_transition && monster_take(body, &mut cursor, 13).is_none() {
        return false;
    }
    cursor == body.len()
}

fn validate_ambient_ignore_evidence(capture: &Capture, opts: &Opts) -> Result<AmbientIgnoreCounts> {
    let ignores_time = opts.ignored_opcodes.contains(&PacketBoundary {
        direction: Some(Direction::S2C),
        opcode: 0x2DD2,
    });
    let ignores_movement = opts.ignored_opcodes.contains(&PacketBoundary {
        direction: Some(Direction::S2C),
        opcode: 0x2DD4,
    });
    let mut counts = AmbientIgnoreCounts::default();
    let mut pending_time_sync = BTreeMap::<(u32, u32), usize>::new();
    let mut completed_time_sync = BTreeMap::<(u32, u32), usize>::new();

    for (index, packet) in capture.packets.iter().enumerate() {
        match (packet.direction, packet.opcode) {
            (Direction::S2C, 0x2DD2) if ignores_time => {
                if packet.connection_id != 1 {
                    bail!(
                        "{} packet {index} time-sync request is on connection {}, expected instance connection 1",
                        capture.source,
                        packet.connection_id
                    );
                }
                if packet.body.len() != 4 {
                    bail!(
                        "{} packet {index} time-sync request has malformed {}-byte body, expected 4",
                        capture.source,
                        packet.body.len()
                    );
                }
                let key = (packet.connection_id, read_le_u32(&packet.body));
                if pending_time_sync.contains_key(&key) || completed_time_sync.contains_key(&key) {
                    bail!(
                        "{} packet {index} duplicates time-sync request sequence {} on connection {}",
                        capture.source,
                        key.1,
                        key.0
                    );
                }
                pending_time_sync.insert(key, index);
                counts.time_sync_request += 1;
            }
            (Direction::C2S, 0x3A3D) if ignores_time => {
                if packet.connection_id != 1 {
                    bail!(
                        "{} packet {index} time-sync response is on connection {}, expected instance connection 1",
                        capture.source,
                        packet.connection_id
                    );
                }
                if packet.body.len() != 8 {
                    bail!(
                        "{} packet {index} time-sync response has malformed {}-byte body, expected 8",
                        capture.source,
                        packet.body.len()
                    );
                }
                let key = (packet.connection_id, read_le_u32(&packet.body));
                let Some(request_index) = pending_time_sync.remove(&key) else {
                    bail!(
                        "{} packet {index} has orphan or duplicate time-sync response sequence {} on connection {}",
                        capture.source,
                        key.1,
                        key.0
                    );
                };
                completed_time_sync.insert(key, request_index);
                counts.time_sync_response += 1;
            }
            (Direction::S2C, 0x2DD4) if ignores_movement => {
                if packet.connection_id != 1 {
                    bail!(
                        "{} packet {index} monster movement is on connection {}, expected instance connection 1",
                        capture.source,
                        packet.connection_id
                    );
                }
                if !valid_monster_move_body(&packet.body) {
                    bail!(
                        "{} packet {index} has malformed monster-movement body ({} bytes)",
                        capture.source,
                        packet.body.len()
                    );
                }
                counts.monster_move += 1;
            }
            _ => {}
        }
    }

    if let Some(((connection_id, sequence), request_index)) = pending_time_sync.into_iter().next() {
        bail!(
            "{} packet {request_index} has no matching time-sync response for sequence {sequence} on connection {connection_id}",
            capture.source
        );
    }
    Ok(counts)
}

#[cfg(test)]
fn apply_capture_selection(capture: Capture, opts: &Opts) -> Result<Capture> {
    validate_ignored_opcodes(opts)?;
    let bounded = apply_capture_boundaries(capture, opts)?;
    validate_ambient_ignore_evidence(&bounded, opts)?;
    Ok(apply_ignored_opcodes(bounded, opts))
}

fn apply_capture_pair_selection(
    cpp: Capture,
    rust: Capture,
    opts: &Opts,
) -> Result<(Capture, Capture)> {
    validate_ignored_opcodes(opts)?;
    let cpp = apply_capture_boundaries(cpp, opts)?;
    let rust = apply_capture_boundaries(rust, opts)?;
    let cpp_counts = validate_ambient_ignore_evidence(&cpp, opts)?;
    let rust_counts = validate_ambient_ignore_evidence(&rust, opts)?;
    if cpp_counts != rust_counts {
        bail!(
            "ambient ignore evidence count mismatch: C++ {cpp_counts:?}, Rust {rust_counts:?}; refusing asymmetric deletion"
        );
    }
    Ok((
        apply_ignored_opcodes(cpp, opts),
        apply_ignored_opcodes(rust, opts),
    ))
}

fn cmd_diff(args: &[String]) -> Result<ExitCode> {
    let opts = parse_opts(args)?;
    if opts.cpp_manifest.is_some() || opts.rust_manifest.is_some() {
        bail!("raw manifest flags are accepted only by import");
    }

    // Resolve cpp/rust/directions/baseline either from a flow or explicit paths.
    let (cpp_path, rust_path, directions, baseline) = resolve_sources(&opts)?;

    let (cpp, rust) =
        apply_capture_pair_selection(load_capture(&cpp_path)?, load_capture(&rust_path)?, &opts)?;
    let report = DiffReport::compute(&cpp, &rust, &directions);

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", report.render_text());
    }

    // Baseline comparison (regression gate).
    if let Some(baseline_path) = baseline {
        if baseline_path.is_file() {
            let cmp = compare_baseline(&report, &baseline_path)?;
            if !opts.json {
                print!("{}", render_baseline_delta(&cmp));
            }
            if opts.strict && !cmp.matches() {
                return Ok(ExitCode::FAILURE);
            }
            return Ok(ExitCode::SUCCESS);
        } else if opts.strict {
            bail!("baseline {} not found", baseline_path.display());
        }
    }

    // No baseline: --strict requires a fully clean diff.
    if opts.strict && !report.is_clean() {
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

#[allow(clippy::type_complexity)]
fn resolve_sources(opts: &Opts) -> Result<(PathBuf, PathBuf, Vec<Direction>, Option<PathBuf>)> {
    if let Some(name) = &opts.positional {
        let flow = flow::load_flow(name)?;
        let cpp = opts.cpp.clone().unwrap_or(flow.golden_pkt);
        let rust = opts.rust.clone().unwrap_or(flow.reference_rust);
        let directions = opts.direction.clone().unwrap_or(flow.directions);
        let baseline = opts.baseline.clone().or(Some(flow.expected));
        Ok((cpp, rust, directions, baseline))
    } else {
        let cpp = opts
            .cpp
            .clone()
            .context("provide a flow name or --cpp <PKT>")?;
        let rust = opts
            .rust
            .clone()
            .context("provide a flow name or --rust <DUMPDIR>")?;
        let directions = opts
            .direction
            .clone()
            .unwrap_or_else(|| vec![Direction::S2C, Direction::C2S]);
        Ok((cpp, rust, directions, opts.baseline.clone()))
    }
}

fn cmd_show(args: &[String]) -> Result<ExitCode> {
    if args.len() != 1 {
        bail!("usage: capture-diff show <PKT|DUMPDIR>");
    }
    let path = &args[0];
    let cap = load_capture(Path::new(path))?;
    println!("{} ({} packets)", cap.source, cap.packets.len());
    for (i, p) in cap.packets.iter().enumerate() {
        println!(
            "  {i:4} [{} conn={}] 0x{:04X} {} ({} body bytes)",
            p.direction,
            p.connection_id,
            p.opcode,
            p.opcode_name(),
            p.body.len()
        );
    }
    Ok(ExitCode::SUCCESS)
}

// Kept fallible for uniform dispatch with the other `cmd_*` handlers.
#[allow(clippy::unnecessary_wraps)]
fn cmd_list() -> Result<ExitCode> {
    let flows = flow::list_flows();
    let requirements = flow::list_requirements();
    if flows.is_empty() && requirements.is_empty() {
        println!("no flows found under {}", flow::flows_root().display());
        return Ok(ExitCode::SUCCESS);
    }
    if !flows.is_empty() {
        println!("known flows:");
        for name in &flows {
            match flow::load_flow(name) {
                Ok(f) => println!("  {name:<16} {}", f.description),
                Err(_) => println!("  {name}"),
            }
        }
    }
    if !requirements.is_empty() {
        println!("required flows:");
        for name in requirements {
            match flow::load_requirement(&name) {
                Ok(requirement) => println!(
                    "  {name:<24} {:?} ({})",
                    requirement.status, requirement.issue
                ),
                Err(_) => println!("  {name:<24} INVALID"),
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Enforce a milestone's capture contract, not merely its accepted-divergence
/// baseline. A required flow must have real artifacts, an empty baseline, the
/// reviewed packet topology/order, and a fully clean C++↔Rust diff.
fn cmd_verify_required(args: &[String]) -> Result<ExitCode> {
    if args.len() != 1 {
        bail!("usage: capture-diff verify-required <flow>");
    }
    let name = &args[0];
    let requirement = flow::load_requirement(name)?;
    requirement.require_ready()?;

    let pinned = flow::load_flow(name)?;
    if pinned.directions != requirement.directions {
        bail!(
            "required flow '{name}' directions {:?} do not match flow.json {:?}",
            requirement.directions,
            pinned.directions
        );
    }
    let reviewed_selection = lineage_selection_for_requirement(&requirement);
    lineage::verify_required_lineage(name, &requirement.directory, &reviewed_selection)?;

    let cpp = load_capture(&pinned.golden_pkt)?;
    let rust = load_capture(&pinned.reference_rust)?;
    requirement.validate_capture(&cpp)?;
    requirement.validate_capture(&rust)?;

    let expected_text = std::fs::read_to_string(&pinned.expected)
        .with_context(|| format!("reading baseline {}", pinned.expected.display()))?;
    let expected: Vec<DivergenceSignature> = serde_json::from_str(&expected_text)
        .with_context(|| format!("parsing baseline {}", pinned.expected.display()))?;
    if !expected.is_empty() {
        bail!(
            "required flow '{name}' pins {} accepted divergence(s); a milestone capture must be clean",
            expected.len()
        );
    }

    let report = DiffReport::compute(&cpp, &rust, &requirement.directions);
    if !report.is_clean() {
        print!("{}", report.render_text());
        return Ok(ExitCode::FAILURE);
    }

    println!(
        "required flow '{name}' CLEAN: {} matched packet(s), exact topology/order and correlated payload semantics present",
        report.counts.matched
    );
    Ok(ExitCode::SUCCESS)
}

/// Install a captured cpp/rust pair as a flow's golden fixture, optionally
/// trimming both to a flow boundary, and (re)write the divergence baseline.
fn cmd_import(args: &[String]) -> Result<ExitCode> {
    let opts = parse_opts(args)?;
    let name = opts.positional.as_deref().context(
        "usage: capture-diff import <flow> --cpp <PKT> --rust <DIR> [--cpp-manifest <JSON>] [--rust-manifest <JSON>] [--from-opcode c2s:0xNNNN --until-opcode s2c:0xNNNN] [--ignore-opcode s2c:0xNNNN]",
    )?;
    flow::validate_flow_name(name)?;
    let cpp_path = opts.cpp.clone().context("import requires --cpp <PKT>")?;
    let rust_path = opts
        .rust
        .clone()
        .context("import requires --rust <DUMPDIR>")?;
    let cpp_manifest_path = opts.cpp_manifest.clone().unwrap_or_else(|| {
        cpp_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("cpp.capture-manifest.json")
    });
    let rust_manifest_path = opts
        .rust_manifest
        .clone()
        .unwrap_or_else(|| rust_path.join("rust.capture-manifest.json"));
    let requirement_path = flow::flows_root().join(name).join("requirement.json");
    let is_required = requirement_path.is_file();
    if is_required && !opts.strict {
        bail!("required flow '{name}' can be imported only with --strict");
    }
    let raw = lineage::validate_raw_pair(
        name,
        &cpp_path,
        &cpp_manifest_path,
        &rust_path,
        &rust_manifest_path,
        is_required,
    )?;

    let (cpp, rust) =
        apply_capture_pair_selection(load_capture(&cpp_path)?, load_capture(&rust_path)?, &opts)?;

    let directions = opts
        .direction
        .clone()
        .unwrap_or_else(|| vec![Direction::S2C, Direction::C2S]);
    if opts.strict {
        validate_required_import(name, &directions, &opts, &cpp, &rust)?;
    }
    lineage::validate_bot_report_capture_binding(name, &raw, &cpp, &rust)?;
    let report = DiffReport::compute(&cpp, &rust, &directions);
    if opts.strict && !report.is_clean() {
        print!("{}", report.render_text());
        eprintln!(
            "capture-diff: refusing to import a non-clean flow with --strict (no files written)"
        );
        return Ok(ExitCode::FAILURE);
    }

    let transaction = lineage::AtomicFlowImport::prepare(&flow::flows_root(), name)?;
    let dir = transaction.staging_dir();
    std::fs::write(dir.join("cpp.pkt"), pkt::write_pkt_bytes(&cpp))
        .with_context(|| format!("writing staged {}/cpp.pkt", dir.display()))?;
    let rust_dir = dir.join("rust");
    rustdump::write_rust_dump(&rust_dir, &rust)?;

    std::fs::write(
        dir.join("expected-divergences.json"),
        serde_json::to_string_pretty(&report.signatures())?,
    )?;
    let raw_after_import = lineage::validate_raw_pair(
        name,
        &cpp_path,
        &cpp_manifest_path,
        &rust_path,
        &rust_manifest_path,
        is_required,
    )?;
    if raw != raw_after_import {
        bail!("raw capture manifests or artifacts changed while import was running");
    }
    let selection = lineage::ImportSelection::new(
        directions.clone(),
        opts.from_opcode,
        opts.until_opcode,
        &opts.ignored_opcodes,
        opts.strict,
    );
    lineage::write_derived_lineage(name, dir, &raw, selection)?;
    if is_required {
        let requirement = flow::load_requirement(name)?;
        lineage::verify_required_lineage(
            name,
            dir,
            &lineage_selection_for_requirement(&requirement),
        )?;
    }
    transaction.publish()?;
    let published_dir = flow::flows_root().join(name);

    println!(
        "imported flow '{name}': cpp={} rust={} packets, {} divergence(s) -> {}",
        cpp.packets.len(),
        rust.packets.len(),
        report.signatures().len(),
        published_dir.display()
    );
    print!("{}", report.render_text());
    Ok(ExitCode::SUCCESS)
}

/// A strict import for a milestone flow must satisfy its reviewed capture
/// contract before any fixture is written. `AwaitingRealCaptures` is accepted
/// here deliberately: importing the real pair is the operation that allows the
/// manifest to be reviewed and promoted to `ready` afterwards.
fn validate_required_import(
    name: &str,
    directions: &[Direction],
    opts: &Opts,
    cpp: &Capture,
    rust: &Capture,
) -> Result<()> {
    let requirement_path = flow::flows_root().join(name).join("requirement.json");
    if !requirement_path.is_file() {
        return Ok(());
    }

    let requirement = flow::load_requirement(name)?;
    requirement.validate_import_selection(
        directions,
        opts.from_opcode,
        opts.until_opcode,
        &opts.ignored_opcodes,
        opts.strict,
    )?;
    requirement.validate_capture(cpp)?;
    requirement.validate_capture(rust)?;
    Ok(())
}

fn lineage_selection_for_requirement(
    requirement: &flow::FlowRequirement,
) -> lineage::ImportSelection {
    let ignored = requirement
        .import_selection
        .ignored_opcodes
        .iter()
        .copied()
        .map(PacketBoundary::from)
        .collect::<Vec<_>>();
    lineage::ImportSelection::new(
        requirement.directions.clone(),
        Some(requirement.import_selection.from_opcode.into()),
        Some(requirement.import_selection.until_opcode.into()),
        &ignored,
        true,
    )
}

fn cmd_update_baseline(args: &[String]) -> Result<ExitCode> {
    let opts = parse_opts(args)?;
    if opts.cpp_manifest.is_some() || opts.rust_manifest.is_some() {
        bail!("raw manifest flags are accepted only by import");
    }
    if opts.from_opcode.is_some() || opts.until_opcode.is_some() || !opts.ignored_opcodes.is_empty()
    {
        bail!(
            "update-baseline does not accept capture boundaries or opcode filters; use import to install consistently selected fixtures"
        );
    }
    let name = opts
        .positional
        .as_deref()
        .context("usage: capture-diff update-baseline <flow> [--rust DIR]")?;
    let flow = flow::load_flow(name)?;
    let cpp = load_capture(&opts.cpp.clone().unwrap_or(flow.golden_pkt))?;
    let rust = load_capture(&opts.rust.clone().unwrap_or(flow.reference_rust))?;
    let directions = opts.direction.clone().unwrap_or(flow.directions);
    let report = DiffReport::compute(&cpp, &rust, &directions);
    let json = serde_json::to_string_pretty(&report.signatures())?;
    std::fs::write(&flow.expected, &json)
        .with_context(|| format!("writing baseline {}", flow.expected.display()))?;
    println!(
        "wrote {} divergence(s) to {}",
        report.signatures().len(),
        flow.expected.display()
    );
    Ok(ExitCode::SUCCESS)
}

/// Load a capture from either a `.pkt` file (C++) or a dump directory (Rust).
fn load_capture(path: &Path) -> Result<Capture> {
    if path.is_dir() {
        rustdump::parse_rust_dump(path)
    } else if path.is_file() {
        pkt::parse_pkt_file(path)
    } else {
        bail!("capture path does not exist: {}", path.display())
    }
}

/// Render a count-aware baseline delta for the terminal.
fn render_baseline_delta(delta: &BaselineDelta) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s);
    if delta.matches() {
        let _ = writeln!(s, "baseline: MATCHES accepted divergences ✓");
        return s;
    }
    let _ = writeln!(s, "baseline: CHANGED ✗ (vs accepted divergences)");
    for d in &delta.new {
        let _ = writeln!(
            s,
            "  NEW   {:?} [{}] {} {}",
            d.kind, d.direction, d.opcode, d.name
        );
    }
    for d in &delta.fixed {
        let _ = writeln!(
            s,
            "  FIXED {:?} [{}] {} {} (update baseline with `update-baseline`)",
            d.kind, d.direction, d.opcode, d.name
        );
    }
    s
}

fn compare_baseline(report: &DiffReport, baseline_path: &Path) -> Result<BaselineDelta> {
    let text = std::fs::read_to_string(baseline_path)
        .with_context(|| format!("reading baseline {}", baseline_path.display()))?;
    let expected: Vec<DivergenceSignature> = serde_json::from_str(&text)
        .with_context(|| format!("parsing baseline {}", baseline_path.display()))?;
    Ok(diff::baseline_delta(&report.signatures(), &expected))
}

fn print_usage() {
    eprintln!(
        "capture-diff — C++(PKT) vs Rust packet capture diff (issue [01]/#66)\n\
         \n\
         USAGE:\n\
         \x20 capture-diff diff <flow> [--rust DIR] [--cpp PKT] [--direction s2c|c2s|both] [--from-opcode ...] [--until-opcode ...] [--ignore-opcode ...] [--json] [--strict]\n\
         \x20 capture-diff diff --cpp A.pkt --rust DIR [...]\n\
         \x20 capture-diff show <PKT|DUMPDIR>\n\
         \x20 capture-diff list\n\
         \x20 capture-diff verify-required <flow>\n\
         \x20 capture-diff import <flow> --cpp PKT --rust DIR [--cpp-manifest JSON] [--rust-manifest JSON] [--from-opcode c2s:0xNNNN] [--until-opcode s2c:0xNNNN] [--ignore-opcode s2c:0xNNNN] [--strict]\n\
         \x20 capture-diff update-baseline <flow> [--rust DIR]\n\
         \n\
         A flow resolves its golden C++ capture, reference Rust dump, and accepted-divergence\n\
         baseline from crates/capture-diff/flows/<flow>/. --strict exits non-zero when the diff\n\
         deviates from that baseline. verify-required additionally requires an operator-marked\n\
         ready manifest and refuses missing fixtures, accepted divergences, or a capture that\n\
         violates its reviewed wire shape."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(direction: Direction, opcode: u16) -> capture_diff::CapturedPacket {
        capture_diff::CapturedPacket {
            direction,
            connection_id: 0,
            opcode,
            body: Vec::new(),
        }
    }

    fn routed_packet(
        direction: Direction,
        connection_id: u32,
        opcode: u16,
    ) -> capture_diff::CapturedPacket {
        capture_diff::CapturedPacket {
            direction,
            connection_id,
            opcode,
            body: Vec::new(),
        }
    }

    fn ambient_packet(
        direction: Direction,
        connection_id: u32,
        opcode: u16,
        body: Vec<u8>,
    ) -> capture_diff::CapturedPacket {
        capture_diff::CapturedPacket {
            direction,
            connection_id,
            opcode,
            body,
        }
    }

    fn minimal_monster_move_body() -> Vec<u8> {
        let mut body = vec![0x01, 0x00, 0x01]; // canonical non-empty mover GUID
        body.extend_from_slice(&[0; 12]); // current XYZ
        body.extend_from_slice(&[0; 4]); // spline id
        body.extend_from_slice(&[0; 12]); // destination XYZ
        body.push(0); // CrzTeleport + tolerance + zero padding
        body.extend_from_slice(&[0; 17]); // flags/elapsed/time/fade/mode
        body.extend_from_slice(&[0, 0]); // empty transport GUID
        body.push(0xFF); // vehicle seat
        body.extend_from_slice(&[0; 5]); // normal face, zero path/options
        assert!(valid_monster_move_body(&body));
        body
    }

    fn valid_required_loot_capture(source: &str) -> Capture {
        let pinned = flow::load_flow("loot-single-item-claim").expect("committed loot flow");
        let mut capture = load_capture(&pinned.reference_rust).expect("committed Rust fixture");
        capture.source = source.to_string();
        capture
    }

    fn reviewed_required_import_opts() -> Opts {
        parse_opts(&[
            "loot-single-item-claim".into(),
            "--from-opcode".into(),
            "c2s:0x3211".into(),
            "--until-opcode".into(),
            "c2s:0x3768".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD2".into(),
            "--ignore-opcode".into(),
            "c2s:0x3A3D".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD4".into(),
            "--direction".into(),
            "both".into(),
            "--strict".into(),
        ])
        .unwrap()
    }

    #[test]
    fn strict_import_accepts_awaiting_required_contract_with_valid_shape() {
        let cpp = valid_required_loot_capture("cpp");
        let rust = valid_required_loot_capture("rust");
        let opts = reviewed_required_import_opts();

        validate_required_import(
            "loot-single-item-claim",
            &[Direction::S2C, Direction::C2S],
            &opts,
            &cpp,
            &rust,
        )
        .unwrap();
    }

    #[test]
    fn strict_import_rejects_required_contract_direction_mismatch() {
        let cpp = valid_required_loot_capture("cpp");
        let rust = valid_required_loot_capture("rust");
        let opts = reviewed_required_import_opts();

        let error = validate_required_import(
            "loot-single-item-claim",
            &[Direction::S2C],
            &opts,
            &cpp,
            &rust,
        )
        .expect_err("required flow directions must be exact");

        assert!(error.to_string().contains("reviewed contract"));
    }

    #[test]
    fn strict_import_rejects_required_contract_wrong_route() {
        let cpp = valid_required_loot_capture("cpp");
        let mut rust = valid_required_loot_capture("rust");
        let opts = reviewed_required_import_opts();
        rust.packets[2].connection_id = 0;

        let error = validate_required_import(
            "loot-single-item-claim",
            &[Direction::S2C, Direction::C2S],
            &opts,
            &cpp,
            &rust,
        )
        .expect_err("required flow route must be exact");

        let message = error.to_string();
        assert!(message.contains("packet 2"));
        assert!(message.contains("s2c conn=1 0x2615"));
    }

    #[test]
    fn strict_import_rejects_required_contract_wrong_boundary() {
        let mut cpp = valid_required_loot_capture("cpp");
        let rust = valid_required_loot_capture("rust");
        let opts = reviewed_required_import_opts();
        cpp.packets
            .insert(0, routed_packet(Direction::S2C, 1, 0x2DD4));

        let error = validate_required_import(
            "loot-single-item-claim",
            &[Direction::S2C, Direction::C2S],
            &opts,
            &cpp,
            &rust,
        )
        .expect_err("required flow boundary must be exact");

        assert!(error.to_string().contains("required boundary"));
    }

    #[test]
    fn strict_import_rejects_an_extra_approved_ignore_for_required_flow() {
        let cpp = valid_required_loot_capture("cpp");
        let rust = valid_required_loot_capture("rust");
        let mut opts = reviewed_required_import_opts();
        opts.ignored_opcodes.push(PacketBoundary {
            direction: Some(Direction::S2C),
            opcode: 0x2DD4,
        });

        let error = validate_required_import(
            "loot-single-item-claim",
            &[Direction::S2C, Direction::C2S],
            &opts,
            &cpp,
            &rust,
        )
        .expect_err("an extra filter must not be able to hide required-flow traffic");
        assert!(error.to_string().contains("reviewed contract"));
    }

    #[test]
    fn parses_directional_packet_boundary() {
        assert_eq!(
            parse_packet_boundary("c2s:0x318C", "--from-opcode").unwrap(),
            PacketBoundary {
                direction: Some(Direction::C2S),
                opcode: 0x318C,
            }
        );
        assert_eq!(
            parse_packet_boundary("S2C:271c", "--until-opcode").unwrap(),
            PacketBoundary {
                direction: Some(Direction::S2C),
                opcode: 0x271C,
            }
        );
    }

    #[test]
    fn preserves_legacy_directionless_until_boundary() {
        assert_eq!(
            parse_packet_boundary("0x3A46", "--until-opcode").unwrap(),
            PacketBoundary {
                direction: None,
                opcode: 0x3A46,
            }
        );
    }

    #[test]
    fn rejects_unknown_boundary_direction() {
        let err = parse_packet_boundary("both:0x318C", "--from-opcode").unwrap_err();
        assert!(err.to_string().contains("use c2s:0xNNNN or s2c:0xNNNN"));
    }

    #[test]
    fn shared_diff_and_import_selection_slices_and_filters_ambient_packets() {
        let opts = parse_opts(&[
            "stand-state".into(),
            "--from-opcode".into(),
            "c2s:0x318C".into(),
            "--until-opcode".into(),
            "c2s:0x3768".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD4".into(),
        ])
        .unwrap();
        let capture = Capture::new(
            "full-session",
            vec![
                packet(Direction::S2C, 0x256D),
                packet(Direction::C2S, 0x318C),
                ambient_packet(Direction::S2C, 1, 0x2DD4, minimal_monster_move_body()),
                packet(Direction::S2C, 0x271C),
                packet(Direction::C2S, 0x3768),
                packet(Direction::S2C, 0x304E),
            ],
        );

        let sliced = apply_capture_selection(capture, &opts).unwrap();
        assert_eq!(
            sliced
                .packets
                .iter()
                .map(|packet| (packet.direction, packet.opcode))
                .collect::<Vec<_>>(),
            vec![
                (Direction::C2S, 0x318C),
                (Direction::S2C, 0x271C),
                (Direction::C2S, 0x3768),
            ]
        );
    }

    #[test]
    fn import_can_remove_only_the_reviewed_time_sync_request_response_pair() {
        let opts = parse_opts(&[
            "loot-single-item-claim".into(),
            "--from-opcode".into(),
            "c2s:0x3211".into(),
            "--until-opcode".into(),
            "c2s:0x3768".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD2".into(),
            "--ignore-opcode".into(),
            "c2s:0x3A3D".into(),
        ])
        .unwrap();
        let capture = Capture::new(
            "loot-window",
            vec![
                packet(Direction::C2S, 0x3211),
                ambient_packet(Direction::S2C, 1, 0x2DD2, 7_u32.to_le_bytes().to_vec()),
                ambient_packet(Direction::C2S, 1, 0x3A3D, {
                    let mut body = 7_u32.to_le_bytes().to_vec();
                    body.extend_from_slice(&1234_u32.to_le_bytes());
                    body
                }),
                packet(Direction::S2C, 0x2615),
                packet(Direction::C2S, 0x3768),
            ],
        );

        let selected = apply_capture_selection(capture, &opts).unwrap();
        assert_eq!(
            selected
                .packets
                .iter()
                .map(|packet| (packet.direction, packet.opcode))
                .collect::<Vec<_>>(),
            vec![
                (Direction::C2S, 0x3211),
                (Direction::S2C, 0x2615),
                (Direction::C2S, 0x3768),
            ]
        );
    }

    #[test]
    fn ignored_opcode_requires_an_explicit_direction() {
        let error = parse_opts(&[
            "stand-state".into(),
            "--ignore-opcode".into(),
            "0x2DD4".into(),
        ])
        .err()
        .expect("directionless ignore must fail closed");
        assert!(error.to_string().contains("requires a direction"));
    }

    #[test]
    fn ignored_opcode_rejects_functional_packets_outside_the_ambient_allowlist() {
        let opts = parse_opts(&[
            "stand-state".into(),
            "--ignore-opcode".into(),
            "s2c:0x271C".into(),
        ])
        .unwrap();
        let error = apply_capture_selection(Capture::new("capture", Vec::new()), &opts)
            .expect_err("stand-state ACK must never be filterable");

        assert!(error.to_string().contains("not approved ambient traffic"));
    }

    #[test]
    fn ignored_ambient_opcode_cannot_also_be_an_action_boundary() {
        let opts = parse_opts(&[
            "stand-state".into(),
            "--until-opcode".into(),
            "s2c:0x2DD4".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD4".into(),
        ])
        .unwrap();
        let error = apply_capture_selection(Capture::new("capture", Vec::new()), &opts)
            .expect_err("an action boundary must never be filterable");

        assert!(
            error
                .to_string()
                .contains("cannot remove an action boundary")
        );
    }

    #[test]
    fn ignored_time_sync_requires_a_well_formed_matched_pair_on_instance_socket() {
        let opts = parse_opts(&[
            "loot-single-item-claim".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD2".into(),
            "--ignore-opcode".into(),
            "c2s:0x3A3D".into(),
        ])
        .unwrap();
        let request = ambient_packet(Direction::S2C, 1, 0x2DD2, 9_u32.to_le_bytes().to_vec());
        let response = ambient_packet(Direction::C2S, 1, 0x3A3D, {
            let mut body = 9_u32.to_le_bytes().to_vec();
            body.extend_from_slice(&44_u32.to_le_bytes());
            body
        });
        assert!(
            apply_capture_selection(
                Capture::new("valid", vec![request.clone(), response.clone()]),
                &opts
            )
            .unwrap()
            .packets
            .is_empty()
        );

        let mut wrong_socket = request.clone();
        wrong_socket.connection_id = 0;
        let error = apply_capture_selection(
            Capture::new("wrong socket", vec![wrong_socket, response.clone()]),
            &opts,
        )
        .expect_err("wrong socket must fail");
        assert!(error.to_string().contains("expected instance connection 1"));

        let error = apply_capture_selection(Capture::new("orphan", vec![response.clone()]), &opts)
            .expect_err("orphan response must fail");
        assert!(error.to_string().contains("orphan or duplicate"));

        let error = apply_capture_selection(
            Capture::new(
                "duplicate",
                vec![request.clone(), request.clone(), response],
            ),
            &opts,
        )
        .expect_err("duplicate request must fail");
        assert!(error.to_string().contains("duplicates time-sync request"));

        let malformed = ambient_packet(Direction::S2C, 1, 0x2DD2, vec![9, 0, 0]);
        let error = apply_capture_selection(Capture::new("malformed", vec![malformed]), &opts)
            .expect_err("malformed request must fail");
        assert!(error.to_string().contains("malformed 3-byte body"));
    }

    #[test]
    fn paired_selection_rejects_asymmetric_ambient_counts() {
        let opts = parse_opts(&[
            "stand-state".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD4".into(),
        ])
        .unwrap();
        let body = minimal_monster_move_body();
        let cpp = Capture::new("cpp", vec![ambient_packet(Direction::S2C, 1, 0x2DD4, body)]);
        let rust = Capture::new("rust", Vec::new());
        let error = apply_capture_pair_selection(cpp, rust, &opts)
            .expect_err("different ignore cardinality must fail");
        assert!(error.to_string().contains("count mismatch"));
    }

    #[test]
    fn ignored_monster_move_requires_instance_route_and_structural_body() {
        let opts = parse_opts(&[
            "stand-state".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD4".into(),
        ])
        .unwrap();
        let malformed = ambient_packet(Direction::S2C, 1, 0x2DD4, vec![1; 12]);
        let error = apply_capture_selection(Capture::new("malformed", vec![malformed]), &opts)
            .expect_err("truncated movement must fail");
        assert!(
            error
                .to_string()
                .contains("malformed monster-movement body")
        );

        let valid_body = minimal_monster_move_body();
        let wrong_route = ambient_packet(Direction::S2C, 0, 0x2DD4, valid_body);
        let error = apply_capture_selection(Capture::new("wrong route", vec![wrong_route]), &opts)
            .expect_err("realm-routed movement must fail");
        assert!(error.to_string().contains("expected instance connection 1"));

        let mut trailing = minimal_monster_move_body();
        trailing.push(0);
        let error = apply_capture_selection(
            Capture::new(
                "trailing bytes",
                vec![ambient_packet(Direction::S2C, 1, 0x2DD4, trailing)],
            ),
            &opts,
        )
        .expect_err("trailing movement bytes must fail");
        assert!(error.to_string().contains("malformed monster-movement"));
    }

    #[test]
    fn time_sync_ignore_cannot_be_declared_one_sided_or_duplicated() {
        let one_sided =
            parse_opts(&["flow".into(), "--ignore-opcode".into(), "s2c:0x2DD2".into()]).unwrap();
        assert!(
            apply_capture_selection(Capture::new("capture", Vec::new()), &one_sided)
                .unwrap_err()
                .to_string()
                .contains("request/response pair")
        );

        let duplicate = parse_opts(&[
            "flow".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD4".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD4".into(),
        ])
        .unwrap();
        assert!(
            apply_capture_selection(Capture::new("capture", Vec::new()), &duplicate)
                .unwrap_err()
                .to_string()
                .contains("duplicated")
        );
    }

    #[test]
    fn combat_values_ignore_is_scoped_to_detour_chase_flow() {
        let detour = parse_opts(&[
            "detour-chase-around-obstacle".into(),
            "--ignore-opcode".into(),
            "s2c:0x27CB".into(),
        ])
        .unwrap();
        apply_capture_selection(Capture::new("capture", Vec::new()), &detour).unwrap();

        let unrelated = parse_opts(&[
            "stand-state".into(),
            "--ignore-opcode".into(),
            "s2c:0x27CB".into(),
        ])
        .unwrap();
        assert!(
            apply_capture_selection(Capture::new("capture", Vec::new()), &unrelated)
                .unwrap_err()
                .to_string()
                .contains("not approved ambient traffic")
        );
    }

    #[test]
    fn shared_boundaries_reject_start_without_end() {
        let opts = parse_opts(&[
            "stand-state".into(),
            "--from-opcode".into(),
            "c2s:0x318C".into(),
        ])
        .unwrap();
        let err = apply_capture_boundaries(Capture::new("capture", Vec::new()), &opts).unwrap_err();
        assert!(err.to_string().contains("requires --until-opcode"));
    }

    #[test]
    fn update_baseline_rejects_boundaries_that_would_desync_fixtures() {
        let error = cmd_update_baseline(&[
            "stand-state".into(),
            "--until-opcode".into(),
            "c2s:0x3768".into(),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("use import"));

        let filtered_error = cmd_update_baseline(&[
            "stand-state".into(),
            "--ignore-opcode".into(),
            "s2c:0x2DD4".into(),
        ])
        .unwrap_err();
        assert!(filtered_error.to_string().contains("use import"));
    }

    #[test]
    fn strict_ad_hoc_diff_fails_on_connection_mismatch() {
        let root = std::env::temp_dir().join(format!(
            "capture-diff-strict-connection-mismatch-{}",
            std::process::id()
        ));
        let rust_dir = root.join("rust");
        let cpp_path = root.join("cpp.pkt");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let cpp = Capture::new(
            "cpp",
            vec![capture_diff::CapturedPacket {
                direction: Direction::S2C,
                connection_id: 0,
                opcode: 0x271C,
                body: vec![0, 0, 0, 0, 1],
            }],
        );
        let mut rust_packet = cpp.packets[0].clone();
        rust_packet.connection_id = 1;
        let rust = Capture::new("rust", vec![rust_packet]);
        std::fs::write(&cpp_path, pkt::write_pkt_bytes(&cpp)).unwrap();
        rustdump::write_rust_dump(&rust_dir, &rust).unwrap();

        let result = cmd_diff(&[
            "--cpp".into(),
            cpp_path.display().to_string(),
            "--rust".into(),
            rust_dir.display().to_string(),
            "--direction".into(),
            "s2c".into(),
            "--strict".into(),
        ])
        .unwrap();
        assert_eq!(result, ExitCode::FAILURE);
    }
}

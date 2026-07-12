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
//!   capture-diff update-baseline <flow> [--rust DIR]      # rewrite baseline
//! ```

// Product names (RustyCore, TrinityCore) appear throughout the docs as prose.
#![allow(clippy::doc_markdown)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

use capture_diff::diff::{self, BaselineDelta, DivergenceSignature};
use capture_diff::{Capture, DiffReport, Direction, PacketBoundary, flow, pkt, rustdump};

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
        "update-baseline" => cmd_update_baseline(&rest),
        "-h" | "--help" | "help" => {
            print_usage();
            Ok(ExitCode::SUCCESS)
        }
        other => bail!("unknown command '{other}' (try: diff, show, list, update-baseline)"),
    }
}

/// Parsed flags shared by `diff` and `update-baseline`.
struct Opts {
    positional: Option<String>,
    cpp: Option<PathBuf>,
    rust: Option<PathBuf>,
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
        rust: None,
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
            "--rust" => opts.rust = Some(PathBuf::from(next(&mut it, "--rust")?)),
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
        if !APPROVED_AMBIENT_IGNORES.contains(ignored) {
            bail!(
                "--ignore-opcode {ignored} is not approved ambient traffic; reviewed filters are s2c:0x2DD2 and s2c:0x2DD4"
            );
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

fn apply_capture_selection(capture: Capture, opts: &Opts) -> Result<Capture> {
    validate_ignored_opcodes(opts)?;
    Ok(apply_ignored_opcodes(
        apply_capture_boundaries(capture, opts)?,
        opts,
    ))
}

fn cmd_diff(args: &[String]) -> Result<ExitCode> {
    let opts = parse_opts(args)?;

    // Resolve cpp/rust/directions/baseline either from a flow or explicit paths.
    let (cpp_path, rust_path, directions, baseline) = resolve_sources(&opts)?;

    let cpp = apply_capture_selection(load_capture(&cpp_path)?, &opts)?;
    let rust = apply_capture_selection(load_capture(&rust_path)?, &opts)?;
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
    if flows.is_empty() {
        println!("no flows found under {}", flow::flows_root().display());
    } else {
        println!("known flows:");
        for name in flows {
            match flow::load_flow(&name) {
                Ok(f) => println!("  {name:<16} {}", f.description),
                Err(_) => println!("  {name}"),
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Install a captured cpp/rust pair as a flow's golden fixture, optionally
/// trimming both to a flow boundary, and (re)write the divergence baseline.
fn cmd_import(args: &[String]) -> Result<ExitCode> {
    let opts = parse_opts(args)?;
    let name = opts.positional.as_deref().context(
        "usage: capture-diff import <flow> --cpp <PKT> --rust <DIR> [--from-opcode c2s:0xNNNN --until-opcode s2c:0xNNNN] [--ignore-opcode s2c:0xNNNN]",
    )?;
    flow::validate_flow_name(name)?;
    let cpp_path = opts.cpp.clone().context("import requires --cpp <PKT>")?;
    let rust_path = opts
        .rust
        .clone()
        .context("import requires --rust <DUMPDIR>")?;

    let cpp = apply_capture_selection(load_capture(&cpp_path)?, &opts)?;
    let rust = apply_capture_selection(load_capture(&rust_path)?, &opts)?;

    let directions = opts
        .direction
        .clone()
        .unwrap_or_else(|| vec![Direction::S2C, Direction::C2S]);
    let report = DiffReport::compute(&cpp, &rust, &directions);
    if opts.strict && !report.is_clean() {
        print!("{}", report.render_text());
        eprintln!(
            "capture-diff: refusing to import a non-clean flow with --strict (no files written)"
        );
        return Ok(ExitCode::FAILURE);
    }

    let dir = flow::flows_root().join(name);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("cpp.pkt"), pkt::write_pkt_bytes(&cpp))
        .with_context(|| format!("writing {}/cpp.pkt", dir.display()))?;
    let rust_dir = dir.join("rust");
    if rust_dir.exists() {
        std::fs::remove_dir_all(&rust_dir)?;
    }
    rustdump::write_rust_dump(&rust_dir, &rust)?;

    std::fs::write(
        dir.join("expected-divergences.json"),
        serde_json::to_string_pretty(&report.signatures())?,
    )?;

    println!(
        "imported flow '{name}': cpp={} rust={} packets, {} divergence(s) -> {}",
        cpp.packets.len(),
        rust.packets.len(),
        report.signatures().len(),
        dir.display()
    );
    print!("{}", report.render_text());
    Ok(ExitCode::SUCCESS)
}

fn cmd_update_baseline(args: &[String]) -> Result<ExitCode> {
    let opts = parse_opts(args)?;
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
         \x20 capture-diff import <flow> --cpp PKT --rust DIR [--from-opcode c2s:0xNNNN] [--until-opcode s2c:0xNNNN] [--ignore-opcode s2c:0xNNNN] [--strict]\n\
         \x20 capture-diff update-baseline <flow> [--rust DIR]\n\
         \n\
         A flow resolves its golden C++ capture, reference Rust dump, and accepted-divergence\n\
         baseline from crates/capture-diff/flows/<flow>/. --strict exits non-zero when the diff\n\
         deviates from that baseline (the milestone regression gate)."
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
                packet(Direction::S2C, 0x2DD4),
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

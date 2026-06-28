//! `capture-diff` — CLI for the C++-vs-Rust packet capture diff harness.
//!
//! ```text
//!   capture-diff diff <flow> [--rust DIR] [--cpp PKT] [--direction s2c|c2s|both]
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
use capture_diff::{Capture, DiffReport, Direction, flow, pkt, rustdump};

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
        json: false,
        strict: false,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--cpp" => opts.cpp = Some(PathBuf::from(next(&mut it, "--cpp")?)),
            "--rust" => opts.rust = Some(PathBuf::from(next(&mut it, "--rust")?)),
            "--baseline" => opts.baseline = Some(PathBuf::from(next(&mut it, "--baseline")?)),
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

fn cmd_diff(args: &[String]) -> Result<ExitCode> {
    let opts = parse_opts(args)?;

    // Resolve cpp/rust/directions/baseline either from a flow or explicit paths.
    let (cpp_path, rust_path, directions, baseline) = resolve_sources(&opts)?;

    let cpp = load_capture(&cpp_path)?;
    let rust = load_capture(&rust_path)?;
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
            "  {i:4} [{}] 0x{:04X} {} ({} body bytes)",
            p.direction,
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

fn cmd_update_baseline(args: &[String]) -> Result<ExitCode> {
    let opts = parse_opts(args)?;
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
         \x20 capture-diff diff <flow> [--rust DIR] [--cpp PKT] [--direction s2c|c2s|both] [--json] [--strict]\n\
         \x20 capture-diff diff --cpp A.pkt --rust DIR [...]\n\
         \x20 capture-diff show <PKT|DUMPDIR>\n\
         \x20 capture-diff list\n\
         \x20 capture-diff update-baseline <flow> [--rust DIR]\n\
         \n\
         A flow resolves its golden C++ capture, reference Rust dump, and accepted-divergence\n\
         baseline from crates/capture-diff/flows/<flow>/. --strict exits non-zero when the diff\n\
         deviates from that baseline (the milestone regression gate)."
    );
}

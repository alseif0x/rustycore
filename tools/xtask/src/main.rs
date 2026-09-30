//! Structure automation for the RustyCore workspace.
//!
//! Commands:
//!   structure-audit        source sizes, consumers, flags and navigation layers
//!   check-layers           validate canonical architecture dependency policy
//!   check-deps             compatibility alias for check-layers
//!   context <phase>        print the active slice for an agent
//!   check-scope <phase>    fail when the working tree touches paths outside the phase scope

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

fn layer_of(name: &str) -> Option<u8> {
    let l0 = [
        "wow-core",
        "wow-constants",
        "wow-config",
        "wow-crypto",
        "wow-logging",
        "wow-math",
        "wow-data-model",
        "wow-proto",
        "wow-util-collections",
        "wow-module-api",
    ];
    let l1 = [
        "wow-data",
        "wow-persistence",
        "wow-database",
        "rustycore-db",
        "wow-recastdetour",
    ];
    let l2 = [
        "wow-entities",
        "wow-map",
        "wow-movement",
        "wow-packet",
        "wow-network",
    ];
    let l3 = [
        "wow-combat",
        "wow-conditions",
        "wow-spell",
        "wow-spell-acquisition",
        "wow-loot",
        "wow-quest",
        "wow-items",
        "wow-pets",
        "wow-social",
        "wow-economy",
        "wow-instances",
        "wow-battlegrounds",
        "wow-dungeon-finding",
        "wow-progression",
        "wow-ai",
        "wow-account-collections",
        "wow-anticheat",
        "wow-chat",
        "wow-script",
        "wow-scripts",
        "wow-session",
        "wow-handler",
        "capture-diff",
        "xtask",
    ];
    if l0.contains(&name) {
        Some(0)
    } else if l1.contains(&name) {
        Some(1)
    } else if l2.contains(&name) {
        Some(2)
    } else if l3.contains(&name) {
        Some(3)
    } else if name == "wow-world" {
        Some(4)
    } else if ["world-server", "bnet-server", "world-modules"].contains(&name) {
        Some(5)
    } else {
        None
    }
}

struct Crate {
    name: String,
    #[allow(dead_code)]
    dir: PathBuf,
    src_lines: usize,
    test_lines: usize,
    lines: usize,
    files: usize,
    largest: usize,
    deps: Vec<String>,
    #[allow(dead_code)]
    dev_deps: Vec<String>,
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn members(root: &Path) -> Vec<String> {
    let text = read(&root.join("Cargo.toml"));
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.trim_start().starts_with("members") {
            inside = true;
            continue;
        }
        if inside {
            if line.trim_start().starts_with(']') {
                break;
            }
            let entry = line.trim().trim_end_matches(',').trim_matches('"');
            if !entry.is_empty() {
                out.push(entry.to_string());
            }
        }
    }
    out
}

/// Minimal manifest reader: enough for `package.name`, `[dependencies]` and `[dev-dependencies]`.
fn load_crate(root: &Path, member: &str) -> Option<Crate> {
    let dir = root.join(member);
    let manifest = dir.join("Cargo.toml");
    if !manifest.is_file() {
        return None;
    }
    let text = read(&manifest);
    let mut name = String::new();
    let mut deps = Vec::new();
    let mut dev_deps = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = trimmed.to_string();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name") {
            if name.is_empty() && rest.trim_start().starts_with('=') {
                name = rest
                    .trim()
                    .trim_start_matches('=')
                    .trim()
                    .trim_matches('"')
                    .to_string();
            }
        }
        if let Some(key) = trimmed.split('=').next() {
            let key = key.trim();
            if key.is_empty() || key.starts_with('#') || key.starts_with('"') {
                continue;
            }
            if section == "[dependencies]" || section == "[build-dependencies]" {
                deps.push(key.to_string());
            } else if section == "[dev-dependencies]" {
                dev_deps.push(key.to_string());
            }
        }
    }
    let (src_lines, src_files, src_largest) = walk(&dir.join("src"));
    let (test_lines, test_files, test_largest) = walk(&dir.join("tests"));
    Some(Crate {
        name,
        dir,
        src_lines,
        test_lines,
        lines: src_lines + test_lines,
        files: src_files + test_files,
        largest: src_largest.max(test_largest),
        deps,
        dev_deps,
    })
}

fn walk(dir: &Path) -> (usize, usize, usize) {
    let mut lines = 0;
    let mut files = 0;
    let mut largest = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let content = read(&path);
                let count = content.lines().count();
                lines += count;
                files += 1;
                largest = largest.max(count);
            }
        }
    }
    (lines, files, largest)
}

fn load_all(root: &Path) -> Vec<Crate> {
    let mut crates: Vec<Crate> = members(root)
        .iter()
        .filter_map(|m| load_crate(root, m))
        .collect();
    crates.sort_by(|a, b| a.name.cmp(&b.name));
    crates
}

fn consumers(crates: &[Crate]) -> BTreeMap<String, usize> {
    let names: BTreeSet<&str> = crates.iter().map(|c| c.name.as_str()).collect();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for c in crates {
        for d in &c.deps {
            if names.contains(d.as_str()) {
                *counts.entry(d.clone()).or_default() += 1;
            }
        }
    }
    counts
}

fn structure_audit(root: &Path) {
    let crates = load_all(root);
    let consumers = consumers(&crates);
    println!(
        "{:<24} {:>11} {:>11} {:>11} {:>7} {:>13} {:>6} {:>5}  layer  flags",
        "crate",
        "src-lines",
        "test-lines",
        "total-lines",
        "files",
        "largest-lines",
        "deps",
        "used"
    );
    for c in crates.iter().rev() {
        let mut flags = Vec::new();
        if c.src_lines <= 60 {
            flags.push("stub");
        }
        let layer = layer_of(&c.name);
        if consumers.get(&c.name).copied().unwrap_or(0) == 0 && layer.is_some_and(|l| l < 5) {
            flags.push("no-consumer");
        }
        if c.largest > 1000 {
            flags.push("file>1000");
        }
        if c.lines > 60_000 {
            flags.push("crate>60k");
        }
        if layer.is_none() {
            flags.push("no-layer");
        }
        println!(
            "{:<24} {:>11} {:>11} {:>11} {:>7} {:>13} {:>6} {:>5}  L{:<4} {}",
            c.name,
            c.src_lines,
            c.test_lines,
            c.lines,
            c.files,
            c.largest,
            c.deps.len(),
            consumers.get(&c.name).copied().unwrap_or(0),
            layer.map(|l| l.to_string()).unwrap_or("?".into()),
            flags.join(",")
        );
    }
    println!(
        "Layer labels are navigation; dependency policy: tools/architecture/dependency-policy.json (check-layers/check-deps)."
    );
}

fn run_architecture_dependencies(root: &Path) -> i32 {
    match Command::new("python3")
        .args(["tools/architecture/check_architecture.py", "dependencies"])
        .current_dir(root)
        .status()
    {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("cannot run architecture dependency check: {error}");
            1
        }
    }
}

fn scope_for(root: &Path, phase: &str) -> Vec<String> {
    read(&root.join("tools/xtask/scope.txt"))
        .lines()
        .find_map(|line| {
            let (id, rest) = line.split_once(':')?;
            (id.trim() == phase).then(|| {
                rest.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
        })
        .unwrap_or_default()
}

fn phase_goal(root: &Path, phase: &str) -> String {
    let text = read(&root.join("docs/architecture/workspace-structure-programme.md"));
    for line in text.lines() {
        let cells: Vec<&str> = line.split('|').map(|c| c.trim()).collect();
        if cells.len() > 3 && cells[1] == phase {
            return cells[2].to_string();
        }
    }
    "(phase not found in the programme tables)".into()
}

fn context(root: &Path, phase: &str) -> i32 {
    let goal = phase_goal(root, phase);
    let paths = scope_for(root, phase);
    if paths.is_empty() {
        println!("context: phase `{phase}` has no declared scope in tools/xtask/scope.txt");
        return 1;
    }
    println!("active phase: {phase}");
    println!("objective: {goal}");
    println!("declared scope:");
    for p in &paths {
        println!("  {p}");
    }
    println!("rules: one phase one commit; `cargo check -p <crate>` is the only loop;");
    println!(
        "       movement and behaviour in separate commits; never raise a ceiling or drop a test;"
    );
    println!("       unforeseen work goes into the programme before it is done.");
    println!("verify:  cargo run -p xtask -- check-scope {phase}");
    println!("         cargo run -p xtask -- check-layers (check-deps is an alias)");
    0
}

fn check_scope(root: &Path, phase: &str) -> i32 {
    let paths = scope_for(root, phase);
    if paths.is_empty() {
        println!("check-scope: phase `{phase}` has no declared scope");
        return 1;
    }
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output();
    let Ok(output) = output else {
        println!("check-scope: cannot run git status");
        return 1;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut outside = Vec::new();
    let mut changed = 0;
    for line in text.lines() {
        let path = line.get(3..).unwrap_or("").trim();
        let path = path.split(" -> ").last().unwrap_or(path);
        if path.is_empty() {
            continue;
        }
        changed += 1;
        let allowed = paths.iter().any(|p| {
            let prefix = p.split('*').next().unwrap_or(p);
            path.starts_with(prefix)
        });
        if !allowed {
            outside.push(path.to_string());
        }
    }
    if outside.is_empty() {
        println!("check-scope: PASS ({changed} changed paths, all inside {phase})");
        0
    } else {
        println!(
            "check-scope: FAIL, {} changed paths outside {phase}:",
            outside.len()
        );
        for p in outside {
            println!("  {p}");
        }
        1
    }
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .expect("workspace root");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("");
    let code = match command {
        "structure-audit" => {
            structure_audit(&root);
            0
        }
        "check-layers" | "check-deps" => run_architecture_dependencies(&root),
        "context" => context(&root, args.get(1).map(String::as_str).unwrap_or("")),
        "check-scope" => check_scope(&root, args.get(1).map(String::as_str).unwrap_or("")),
        other => {
            println!("unknown command `{other}`");
            println!(
                "usage: xtask structure-audit|check-layers|check-deps|context <phase>|check-scope <phase>"
            );
            1
        }
    };
    exit(code);
}

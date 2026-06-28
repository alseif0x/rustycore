//! Named flow registry: each flow pins a C++ golden capture plus a reference
//! Rust capture and an accepted-divergence baseline, so a milestone PR can run
//! one command (`capture-diff diff <flow>`) and get an objective gate.
//!
//! On-disk layout (committed under `crates/capture-diff/flows/<name>/`):
//!
//! ```text
//!   flows/<name>/cpp.pkt                    # C++ PKT 3.1 golden capture
//!   flows/<name>/rust/                      # reference Rust dump (.bin/.meta)
//!   flows/<name>/expected-divergences.json  # accepted-divergence baseline
//!   flows/<name>/flow.json                  # optional: description + directions
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::model::Direction;

/// Root of the committed flow fixtures, resolved at compile time so the tool
/// and tests work regardless of the current working directory.
#[must_use]
pub fn flows_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("flows")
}

/// A resolved flow and the paths of its pinned artifacts.
#[derive(Debug, Clone)]
pub struct Flow {
    pub name: String,
    pub golden_pkt: PathBuf,
    pub reference_rust: PathBuf,
    pub expected: PathBuf,
    pub directions: Vec<Direction>,
    pub description: String,
}

#[derive(Debug, Deserialize, Default)]
struct FlowConfig {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    directions: Option<Vec<Direction>>,
}

/// Load a flow by name from the committed fixtures root.
pub fn load_flow(name: &str) -> Result<Flow> {
    load_flow_from(&flows_root(), name)
}

/// Load a flow from an explicit root (used by tests).
pub fn load_flow_from(root: &Path, name: &str) -> Result<Flow> {
    let dir = root.join(name);
    if !dir.is_dir() {
        bail!(
            "unknown flow '{name}': {} does not exist (known flows: {})",
            dir.display(),
            list_flows_from(root).join(", ")
        );
    }
    let golden_pkt = dir.join("cpp.pkt");
    if !golden_pkt.is_file() {
        bail!(
            "flow '{name}' has no golden C++ capture at {}",
            golden_pkt.display()
        );
    }

    let config_path = dir.join("flow.json");
    let config: FlowConfig = if config_path.is_file() {
        let text = std::fs::read_to_string(&config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", config_path.display()))?
    } else {
        FlowConfig::default()
    };

    Ok(Flow {
        name: name.to_string(),
        golden_pkt,
        reference_rust: dir.join("rust"),
        expected: dir.join("expected-divergences.json"),
        directions: config
            .directions
            .unwrap_or_else(|| vec![Direction::S2C, Direction::C2S]),
        description: config.description.unwrap_or_else(|| name.to_string()),
    })
}

/// List the known flow names under the committed fixtures root.
#[must_use]
pub fn list_flows() -> Vec<String> {
    list_flows_from(&flows_root())
}

fn list_flows_from(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().join("cpp.pkt").is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

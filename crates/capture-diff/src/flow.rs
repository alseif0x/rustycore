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
//!   flows/<name>/requirement.json           # optional: fail-closed capture contract
//!   flows/<name>/capture-lineage.json        # exact RAW-to-derived hash contract
//!   flows/<name>/capture-provenance/         # retained exact RAW manifests
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::model::{Capture, CapturedPacket, Direction, PacketBoundary};
use crate::semantic;

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
#[serde(deny_unknown_fields)]
struct FlowConfig {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    directions: Option<Vec<Direction>>,
}

/// Whether an operator has attested that reviewable artifacts are installed.
///
/// `AwaitingRealCaptures` is deliberately not a soft warning: a required flow
/// in this state must fail its preflight gate. The tool validates the declared
/// pair's bytes and shape; it cannot independently prove how those bytes were
/// recorded, so promotion to `Ready` remains an explicit review action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequirementStatus {
    AwaitingRealCaptures,
    Ready,
}

/// Flow-specific semantic contract applied after the exact packet topology.
///
/// Required captures are acceptance evidence, so an opcode-only contract is
/// insufficient: two equally malformed or unrelated packets would otherwise
/// compare byte-clean. Every required flow must name a reviewed semantic
/// contract explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequirementSemanticContract {
    LootSingleItemClaimV1,
    ChaseAroundObstacleV1,
    CreatureSpellCastingV1,
}

/// One routing/order anchor that must occur in a required capture.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredPacket {
    pub direction: Direction,
    pub connection_id: u32,
    pub opcode: u16,
    pub label: String,
}

impl RequiredPacket {
    fn matches(&self, packet: &CapturedPacket) -> bool {
        self.direction == packet.direction
            && self.connection_id == packet.connection_id
            && self.opcode == packet.opcode
    }

    fn render(&self) -> String {
        format!(
            "{} conn={} 0x{:04X} {}",
            self.direction, self.connection_id, self.opcode, self.label
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequirementConfig {
    issue: String,
    status: RequirementStatus,
    description: String,
    #[serde(default)]
    blocked_reason: Option<String>,
    directions: Vec<Direction>,
    import_selection: RequiredImportSelection,
    require_each_anchor_exactly_once: bool,
    semantic_contract: RequirementSemanticContract,
    required_order: Vec<RequiredPacket>,
}

/// One directional action boundary pinned by a required import contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredImportBoundary {
    pub direction: Direction,
    pub opcode: u16,
}

impl From<RequiredImportBoundary> for PacketBoundary {
    fn from(value: RequiredImportBoundary) -> Self {
        Self {
            direction: Some(value.direction),
            opcode: value.opcode,
        }
    }
}

/// The only filtering contract allowed to derive one required flow.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredImportSelection {
    pub from_opcode: RequiredImportBoundary,
    pub until_opcode: RequiredImportBoundary,
    pub ignored_opcodes: Vec<RequiredImportBoundary>,
}

/// A fail-closed contract for a capture that is required by a milestone PR.
///
/// `required_order` is the exact complete packet sequence after reviewed
/// ambient filtering. It is not a subsequence: an extra packet, including a
/// duplicate on the wrong direction or socket, invalidates the evidence.
#[derive(Debug, Clone)]
pub struct FlowRequirement {
    pub name: String,
    pub issue: String,
    pub status: RequirementStatus,
    pub description: String,
    pub blocked_reason: Option<String>,
    pub directions: Vec<Direction>,
    pub import_selection: RequiredImportSelection,
    pub require_each_anchor_exactly_once: bool,
    pub semantic_contract: RequirementSemanticContract,
    pub required_order: Vec<RequiredPacket>,
    pub directory: PathBuf,
}

impl FlowRequirement {
    /// Refuse to validate a flow until its manifest explicitly says that the
    /// real C++ and Rust artifacts have been installed and reviewed.
    pub fn require_ready(&self) -> Result<()> {
        if self.status == RequirementStatus::Ready {
            return Ok(());
        }

        let reason = self
            .blocked_reason
            .as_deref()
            .unwrap_or("the required real capture pair has not been installed");
        bail!(
            "required flow '{}' for {} is BLOCKED: {reason} See {}",
            self.name,
            self.issue,
            self.directory.join("README.md").display()
        )
    }

    /// Reject any import flags that differ from the reviewed action window.
    pub fn validate_import_selection(
        &self,
        directions: &[Direction],
        from_opcode: Option<PacketBoundary>,
        until_opcode: Option<PacketBoundary>,
        ignored_opcodes: &[PacketBoundary],
        strict: bool,
    ) -> Result<()> {
        let required_from: PacketBoundary = self.import_selection.from_opcode.into();
        let required_until: PacketBoundary = self.import_selection.until_opcode.into();
        let required_ignores = self
            .import_selection
            .ignored_opcodes
            .iter()
            .copied()
            .map(PacketBoundary::from)
            .collect::<Vec<_>>();
        if !strict
            || directions != self.directions
            || from_opcode != Some(required_from)
            || until_opcode != Some(required_until)
            || ignored_opcodes != required_ignores
        {
            bail!(
                "required flow '{}' import selection differs from its reviewed contract: strict=true, directions {:?}, from {}, until {}, ignores {:?}",
                self.name,
                self.directions,
                required_from,
                required_until,
                required_ignores
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            );
        }
        Ok(())
    }

    /// Validate the action boundary, connection topology, and C++-anchored
    /// packet order of one side of a required capture pair.
    pub fn validate_capture_shape(&self, capture: &Capture) -> Result<()> {
        if !self.require_each_anchor_exactly_once {
            bail!(
                "required flow '{}' disables exact packet cardinality",
                self.name
            );
        }
        let Some(first_required) = self.required_order.first() else {
            bail!("required flow '{}' has no packet-order contract", self.name);
        };
        let last_required = self
            .required_order
            .last()
            .expect("first required packet already proved the list is nonempty");
        let Some(first_actual) = capture.packets.first() else {
            bail!("{} is empty", capture.source);
        };
        let last_actual = capture
            .packets
            .last()
            .expect("first packet already proved the capture is nonempty");

        if !first_required.matches(first_actual) {
            bail!(
                "{} starts with {} conn={} 0x{:04X}; required boundary is {}",
                capture.source,
                first_actual.direction,
                first_actual.connection_id,
                first_actual.opcode,
                first_required.render()
            );
        }
        if !last_required.matches(last_actual) {
            bail!(
                "{} ends with {} conn={} 0x{:04X}; required fence is {}",
                capture.source,
                last_actual.direction,
                last_actual.connection_id,
                last_actual.opcode,
                last_required.render()
            );
        }

        if capture.packets.len() != self.required_order.len() {
            bail!(
                "{} contains {} packet(s); required flow '{}' permits exactly {}",
                capture.source,
                capture.packets.len(),
                self.name,
                self.required_order.len()
            );
        }

        for (index, (required, actual)) in
            self.required_order.iter().zip(&capture.packets).enumerate()
        {
            if !required.matches(actual) {
                bail!(
                    "{} packet {index} is {} conn={} 0x{:04X}; required packet is {}",
                    capture.source,
                    actual.direction,
                    actual.connection_id,
                    actual.opcode,
                    required.render()
                );
            }
        }

        Ok(())
    }

    /// Validate both the exact wire topology and the action's correlated
    /// payload semantics.
    pub fn validate_capture(&self, capture: &Capture) -> Result<()> {
        self.validate_capture_for_side(capture, RequiredCaptureSide::Rust)
    }

    pub fn validate_capture_for_side(
        &self,
        capture: &Capture,
        side: RequiredCaptureSide,
    ) -> Result<()> {
        self.validate_capture_shape(capture)?;
        match self.semantic_contract {
            RequirementSemanticContract::LootSingleItemClaimV1 => {
                semantic::validate_loot_single_item_claim_capture(capture)
                    .map_err(anyhow::Error::msg)?;
            }
            RequirementSemanticContract::ChaseAroundObstacleV1 => {
                match side {
                    RequiredCaptureSide::Cpp => {
                        semantic::validate_legacy_cpp_detour_chase_capture(capture)
                    }
                    RequiredCaptureSide::Rust => semantic::validate_detour_chase_capture(capture),
                }
                .map_err(anyhow::Error::msg)?;
            }
            RequirementSemanticContract::CreatureSpellCastingV1 => {
                semantic::validate_creature_spell_casting_capture(capture)
                    .map_err(anyhow::Error::msg)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredCaptureSide {
    Cpp,
    Rust,
}

/// Load a flow by name from the committed fixtures root.
pub fn load_flow(name: &str) -> Result<Flow> {
    load_flow_from(&flows_root(), name)
}

/// Load a required-flow contract from the committed fixtures root.
pub fn load_requirement(name: &str) -> Result<FlowRequirement> {
    load_requirement_from(&flows_root(), name)
}

/// Validate one path-component-safe flow name before joining it under a
/// fixture/capture root. This guards every later write or recursive removal
/// from absolute paths and `..` traversal.
pub fn validate_flow_name(name: &str) -> Result<()> {
    let mut bytes = name.bytes();
    if !bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric())
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!(
            "invalid flow name '{name}' (use only ASCII letters, digits, '.', '_', or '-' in one path component)"
        );
    }
    Ok(())
}

/// Load a flow from an explicit root (used by tests).
pub fn load_flow_from(root: &Path, name: &str) -> Result<Flow> {
    validate_flow_name(name)?;
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

/// Load and structurally validate a required-flow contract from an explicit
/// root (the explicit form is used by unit tests).
pub fn load_requirement_from(root: &Path, name: &str) -> Result<FlowRequirement> {
    validate_flow_name(name)?;
    let directory = root.join(name);
    let path = directory.join("requirement.json");
    if !path.is_file() {
        bail!(
            "flow '{name}' has no required-flow contract at {}",
            path.display()
        );
    }

    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let config: RequirementConfig =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;

    if config.issue.trim().is_empty() {
        bail!("{} has an empty issue identifier", path.display());
    }
    if config.description.trim().is_empty() {
        bail!("{} has an empty description", path.display());
    }
    if config.directions.is_empty() {
        bail!("{} has no comparison directions", path.display());
    }
    if config.import_selection.from_opcode.opcode == 0
        || config.import_selection.until_opcode.opcode == 0
        || config
            .import_selection
            .ignored_opcodes
            .iter()
            .any(|boundary| boundary.opcode == 0)
    {
        bail!("{} contains a zero import-selection opcode", path.display());
    }
    let import_directions = std::iter::once(config.import_selection.from_opcode.direction)
        .chain(std::iter::once(
            config.import_selection.until_opcode.direction,
        ))
        .chain(
            config
                .import_selection
                .ignored_opcodes
                .iter()
                .map(|boundary| boundary.direction),
        );
    if import_directions
        .into_iter()
        .any(|direction| !config.directions.contains(&direction))
    {
        bail!(
            "{} import selection uses a direction outside the comparison contract",
            path.display()
        );
    }
    let mut unique_ignores = Vec::new();
    for ignored in &config.import_selection.ignored_opcodes {
        if unique_ignores.contains(ignored) {
            bail!("{} contains a duplicate ignored opcode", path.display());
        }
        unique_ignores.push(*ignored);
    }
    if config.directions.len() > 2
        || config
            .directions
            .iter()
            .enumerate()
            .any(|(index, direction)| config.directions[..index].contains(direction))
    {
        bail!("{} has duplicate comparison directions", path.display());
    }
    if config.required_order.len() < 2 {
        bail!(
            "{} must pin at least an action boundary and an end fence",
            path.display()
        );
    }
    if !config.require_each_anchor_exactly_once {
        bail!(
            "{} must set require_each_anchor_exactly_once=true",
            path.display()
        );
    }
    for packet in &config.required_order {
        if packet.opcode == 0 {
            bail!("{} contains a zero required opcode", path.display());
        }
        if packet.label.trim().is_empty() {
            bail!("{} contains an unlabeled required packet", path.display());
        }
        if !config.directions.contains(&packet.direction) {
            bail!(
                "{} requires {} but does not compare that direction",
                path.display(),
                packet.render()
            );
        }
    }
    match config.status {
        RequirementStatus::AwaitingRealCaptures => {
            if config
                .blocked_reason
                .as_deref()
                .is_none_or(|reason| reason.trim().is_empty())
            {
                bail!(
                    "{} is awaiting real captures but has no blocked_reason",
                    path.display()
                );
            }
        }
        RequirementStatus::Ready if config.blocked_reason.is_some() => {
            bail!(
                "{} is ready but still carries blocked_reason; remove stale blocked evidence",
                path.display()
            );
        }
        RequirementStatus::Ready => {}
    }

    Ok(FlowRequirement {
        name: name.to_string(),
        issue: config.issue,
        status: config.status,
        description: config.description,
        blocked_reason: config.blocked_reason,
        directions: config.directions,
        import_selection: config.import_selection,
        require_each_anchor_exactly_once: config.require_each_anchor_exactly_once,
        semantic_contract: config.semantic_contract,
        required_order: config.required_order,
        directory,
    })
}

/// List the known flow names under the committed fixtures root.
#[must_use]
pub fn list_flows() -> Vec<String> {
    list_flows_from(&flows_root())
}

/// List committed fail-closed capture contracts, including contracts that are
/// still awaiting real artifacts and therefore are not yet normal flows.
#[must_use]
pub fn list_requirements() -> Vec<String> {
    list_requirements_from(&flows_root())
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

fn list_requirements_from(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().join("requirement.json").is_file()
                && let Some(name) = entry.file_name().to_str()
            {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(direction: Direction, connection_id: u32, opcode: u16) -> CapturedPacket {
        CapturedPacket {
            direction,
            connection_id,
            opcode,
            body: Vec::new(),
        }
    }

    fn requirement() -> FlowRequirement {
        FlowRequirement {
            name: "loot-single-item-claim".to_string(),
            issue: "#106".to_string(),
            status: RequirementStatus::Ready,
            description: "test contract".to_string(),
            blocked_reason: None,
            directions: vec![Direction::S2C, Direction::C2S],
            import_selection: RequiredImportSelection {
                from_opcode: RequiredImportBoundary {
                    direction: Direction::C2S,
                    opcode: 0x3211,
                },
                until_opcode: RequiredImportBoundary {
                    direction: Direction::C2S,
                    opcode: 0x3768,
                },
                ignored_opcodes: vec![
                    RequiredImportBoundary {
                        direction: Direction::S2C,
                        opcode: 0x2DD2,
                    },
                    RequiredImportBoundary {
                        direction: Direction::C2S,
                        opcode: 0x3A3D,
                    },
                    RequiredImportBoundary {
                        direction: Direction::S2C,
                        opcode: 0x2DD4,
                    },
                ],
            },
            require_each_anchor_exactly_once: true,
            semantic_contract: RequirementSemanticContract::LootSingleItemClaimV1,
            required_order: vec![
                RequiredPacket {
                    direction: Direction::C2S,
                    connection_id: 1,
                    opcode: 0x3211,
                    label: "CMSG_LOOT_ITEM".to_string(),
                },
                RequiredPacket {
                    direction: Direction::S2C,
                    connection_id: 1,
                    opcode: 0x27CB,
                    label: "SMSG_UPDATE_OBJECT item CreateObject".to_string(),
                },
                RequiredPacket {
                    direction: Direction::S2C,
                    connection_id: 1,
                    opcode: 0x2615,
                    label: "SMSG_LOOT_REMOVED".to_string(),
                },
                RequiredPacket {
                    direction: Direction::S2C,
                    connection_id: 0,
                    opcode: 0x2623,
                    label: "SMSG_ITEM_PUSH_RESULT".to_string(),
                },
                RequiredPacket {
                    direction: Direction::S2C,
                    connection_id: 1,
                    opcode: 0x27CB,
                    label: "SMSG_UPDATE_OBJECT InvSlots VALUES".to_string(),
                },
                RequiredPacket {
                    direction: Direction::C2S,
                    connection_id: 1,
                    opcode: 0x3768,
                    label: "CMSG_PING".to_string(),
                },
            ],
            directory: PathBuf::from("flows/loot-single-item-claim"),
        }
    }

    #[test]
    fn flow_names_are_single_safe_path_components() {
        for valid in ["stand-state", "login_3.4.3", "flow.01"] {
            validate_flow_name(valid).unwrap();
        }
        for invalid in [
            "",
            ".",
            "..",
            ".hidden",
            "_hidden",
            "-hidden",
            "../outside",
            "/tmp/outside",
            "a/b",
            "a\\b",
        ] {
            assert!(validate_flow_name(invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn required_shape_pins_loot_order_routes_and_boundaries() {
        let requirement = requirement();
        let capture = Capture::new(
            "real capture",
            vec![
                packet(Direction::C2S, 1, 0x3211),
                packet(Direction::S2C, 1, 0x27CB),
                packet(Direction::S2C, 1, 0x2615),
                packet(Direction::S2C, 0, 0x2623),
                packet(Direction::S2C, 1, 0x27CB),
                packet(Direction::C2S, 1, 0x3768),
            ],
        );

        requirement.validate_capture_shape(&capture).unwrap();

        let mut reversed = capture.clone();
        reversed.packets.swap(2, 3);
        let error = requirement
            .validate_capture_shape(&reversed)
            .expect_err("ItemPushResult before LootRemoved must fail C++ order");
        assert!(error.to_string().contains("SMSG_LOOT_REMOVED"));

        let mut wrong_route = capture.clone();
        wrong_route.packets[3].connection_id = 1;
        let error = requirement
            .validate_capture_shape(&wrong_route)
            .expect_err("ItemPushResult on the instance socket must fail");
        assert!(error.to_string().contains("SMSG_ITEM_PUSH_RESULT"));

        let mut duplicate_anchor = capture;
        duplicate_anchor
            .packets
            .insert(4, packet(Direction::S2C, 0, 0x2623));
        let error = requirement
            .validate_capture_shape(&duplicate_anchor)
            .expect_err("a second item-push anchor must fail single-session provenance");
        assert!(error.to_string().contains("contains 7 packet(s)"));

        let mut wrong_route_duplicate = Capture::new(
            "wrong-route duplicate",
            vec![
                packet(Direction::C2S, 1, 0x3211),
                packet(Direction::S2C, 1, 0x27CB),
                packet(Direction::S2C, 1, 0x2615),
                packet(Direction::S2C, 0, 0x2623),
                packet(Direction::S2C, 1, 0x2623),
                packet(Direction::S2C, 1, 0x27CB),
                packet(Direction::C2S, 1, 0x3768),
            ],
        );
        let error = requirement
            .validate_capture_shape(&wrong_route_duplicate)
            .expect_err("an ItemPushResult duplicate on the wrong socket must fail");
        assert!(error.to_string().contains("contains 7 packet(s)"));

        wrong_route_duplicate.packets.remove(4);
        wrong_route_duplicate.packets[1].direction = Direction::C2S;
        let error = requirement
            .validate_capture_shape(&wrong_route_duplicate)
            .expect_err("replacing an exact packet without changing count must fail");
        assert!(error.to_string().contains("packet 1"));
    }

    #[test]
    fn required_shape_rejects_capture_outside_action_boundaries() {
        let requirement = requirement();
        let prefixed = Capture::new(
            "prefixed capture",
            vec![
                packet(Direction::S2C, 0, 0x2DD2),
                packet(Direction::C2S, 1, 0x3211),
                packet(Direction::S2C, 1, 0x27CB),
                packet(Direction::S2C, 1, 0x2615),
                packet(Direction::S2C, 0, 0x2623),
                packet(Direction::S2C, 1, 0x27CB),
                packet(Direction::C2S, 1, 0x3768),
            ],
        );
        assert!(
            requirement
                .validate_capture_shape(&prefixed)
                .unwrap_err()
                .to_string()
                .contains("required boundary")
        );

        let suffixed = Capture::new(
            "suffixed capture",
            vec![
                packet(Direction::C2S, 1, 0x3211),
                packet(Direction::S2C, 1, 0x27CB),
                packet(Direction::S2C, 1, 0x2615),
                packet(Direction::S2C, 0, 0x2623),
                packet(Direction::S2C, 1, 0x27CB),
                packet(Direction::C2S, 1, 0x3768),
                packet(Direction::S2C, 1, 0x304E),
            ],
        );
        assert!(
            requirement
                .validate_capture_shape(&suffixed)
                .unwrap_err()
                .to_string()
                .contains("required fence")
        );
    }

    #[test]
    fn awaiting_required_flow_fails_closed_with_recorded_reason() {
        let root = std::env::temp_dir().join(format!(
            "capture-diff-required-flow-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        let directory = root.join("loot-single-item-claim");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("requirement.json"),
            r##"{
  "issue": "#106",
  "status": "awaiting-real-captures",
  "description": "single-session loot claim",
  "blocked_reason": "no matched real C++ capture exists",
  "directions": ["s2c", "c2s"],
  "import_selection": {
    "from_opcode": {"direction": "c2s", "opcode": 12817},
    "until_opcode": {"direction": "c2s", "opcode": 14184},
    "ignored_opcodes": []
  },
  "require_each_anchor_exactly_once": true,
  "semantic_contract": "loot-single-item-claim-v1",
  "required_order": [
    {"direction": "c2s", "connection_id": 1, "opcode": 12817, "label": "CMSG_LOOT_ITEM"},
    {"direction": "c2s", "connection_id": 1, "opcode": 14184, "label": "CMSG_PING"}
  ]
}"##,
        )
        .unwrap();

        let requirement =
            load_requirement_from(&root, "loot-single-item-claim").expect("valid requirement");
        let error = requirement
            .require_ready()
            .expect_err("missing real pair must block preflight");
        assert!(error.to_string().contains("BLOCKED"));
        assert!(error.to_string().contains("no matched real C++ capture"));
        assert!(error.to_string().contains("README.md"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn required_flow_cannot_omit_or_disable_exact_cardinality() {
        let root = std::env::temp_dir().join(format!(
            "capture-diff-required-cardinality-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        let directory = root.join("loot-single-item-claim");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&directory).unwrap();

        let manifest = |cardinality: Option<bool>| {
            let cardinality = cardinality.map_or_else(String::new, |value| {
                format!("\n  \"require_each_anchor_exactly_once\": {value},")
            });
            format!(
                r##"{{
  "issue": "#106",
  "status": "ready",
  "description": "single-session loot claim",
  "directions": ["s2c", "c2s"],{cardinality}
  "import_selection": {{
    "from_opcode": {{"direction": "c2s", "opcode": 12817}},
    "until_opcode": {{"direction": "c2s", "opcode": 14184}},
    "ignored_opcodes": []
  }},
  "semantic_contract": "loot-single-item-claim-v1",
  "required_order": [
    {{"direction": "c2s", "connection_id": 1, "opcode": 12817, "label": "CMSG_LOOT_ITEM"}},
    {{"direction": "c2s", "connection_id": 1, "opcode": 14184, "label": "CMSG_PING"}}
  ]
}}"##
            )
        };

        std::fs::write(directory.join("requirement.json"), manifest(None)).unwrap();
        let missing = load_requirement_from(&root, "loot-single-item-claim")
            .expect_err("omitting exact cardinality must fail");
        assert!(missing.to_string().contains("parsing"));

        std::fs::write(directory.join("requirement.json"), manifest(Some(false))).unwrap();
        let disabled = load_requirement_from(&root, "loot-single-item-claim")
            .expect_err("disabling exact cardinality must fail");
        assert!(
            disabled
                .to_string()
                .contains("require_each_anchor_exactly_once=true")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn flow_and_requirement_json_reject_unknown_fields_at_every_contract_layer() {
        let root = std::env::temp_dir().join(format!(
            "capture-diff-deny-unknown-flow-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        let directory = root.join("strict-schema");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("cpp.pkt"), b"fixture marker").unwrap();
        std::fs::write(
            directory.join("flow.json"),
            r#"{"description":"schema","directions":["s2c","c2s"],"typo":true}"#,
        )
        .unwrap();
        let error =
            load_flow_from(&root, "strict-schema").expect_err("unknown flow.json field must fail");
        assert!(error.to_string().contains("parsing"));

        let requirement = r##"{
  "issue": "#106",
  "status": "ready",
  "description": "strict schema",
  "directions": ["s2c", "c2s"],
  "import_selection": {
    "from_opcode": {"direction": "c2s", "opcode": 12817, "typo": true},
    "until_opcode": {"direction": "c2s", "opcode": 14184},
    "ignored_opcodes": []
  },
  "require_each_anchor_exactly_once": true,
  "semantic_contract": "loot-single-item-claim-v1",
  "required_order": [
    {"direction": "c2s", "connection_id": 1, "opcode": 12817, "label": "request"},
    {"direction": "c2s", "connection_id": 1, "opcode": 14184, "label": "fence"}
  ]
}"##;
        std::fs::write(directory.join("requirement.json"), requirement).unwrap();
        let error = load_requirement_from(&root, "strict-schema")
            .expect_err("unknown nested requirement field must fail");
        assert!(error.to_string().contains("parsing"));

        std::fs::remove_dir_all(root).unwrap();
    }
}

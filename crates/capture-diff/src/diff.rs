//! Opcode-aware capture diff engine.
//!
//! Aligns two normalized [`Capture`]s by opcode order (per direction) using a
//! longest-common-subsequence alignment, then reports the three divergence
//! classes the world-load audit was tracking by hand:
//!
//! - **count / presence** — an opcode the C++ capture has but Rust does not
//!   (`MissingInRust`), or vice versa (`ExtraInRust`);
//! - **order** — a moved packet falls out of the common subsequence and shows up
//!   as a `MissingInRust` + `ExtraInRust` pair of the same opcode;
//! - **value** — an aligned (matched) packet whose body bytes differ
//!   (`BodyMismatch`);
//! - **routing** — an aligned packet travelled over a different realm/instance
//!   connection (`ConnectionMismatch`).

use serde::{Deserialize, Serialize};

use crate::model::{Capture, CapturedPacket, Direction};

/// Number of body bytes shown in hex previews.
const HEX_PREVIEW_BYTES: usize = 32;

/// Outcome of aligning one packet position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    /// Same opcode present in both captures at this aligned position.
    Match,
    /// Present in the C++ capture, absent from Rust at this position.
    MissingInRust,
    /// Present in the Rust capture, absent from C++ at this position.
    ExtraInRust,
}

/// Class of a divergence recorded in the regression baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergenceKind {
    /// Opcode present in C++ but missing from Rust (presence/count/order).
    MissingInRust,
    /// Opcode present in Rust but absent from C++ (presence/count/order).
    ExtraInRust,
    /// Aligned packet whose body bytes differ (value divergence).
    BodyMismatch,
    /// Aligned packet whose realm/instance connection differs (routing
    /// divergence).
    ConnectionMismatch,
}

/// Connection routing comparison for two opcode-aligned packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionDiff {
    /// C++ `ConnectionId` (`0` realm, `1` instance).
    pub cpp_connection_id: u32,
    /// RustyCore connection id (`0` realm, `1` instance).
    pub rust_connection_id: u32,
}

impl ConnectionDiff {
    /// True when both packets travelled over the same connection type.
    #[must_use]
    pub fn is_identical(&self) -> bool {
        self.cpp_connection_id == self.rust_connection_id
    }
}

/// Byte-level comparison of two matched packet bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyDiff {
    pub cpp_len: usize,
    pub rust_len: usize,
    /// First differing byte offset, or `None` when the bodies are identical.
    pub first_diff_offset: Option<usize>,
    pub cpp_hex: String,
    pub rust_hex: String,
}

impl BodyDiff {
    fn compute(cpp: &[u8], rust: &[u8]) -> BodyDiff {
        let mut first_diff = None;
        let min = cpp.len().min(rust.len());
        for i in 0..min {
            if cpp[i] != rust[i] {
                first_diff = Some(i);
                break;
            }
        }
        if first_diff.is_none() && cpp.len() != rust.len() {
            first_diff = Some(min);
        }
        BodyDiff {
            cpp_len: cpp.len(),
            rust_len: rust.len(),
            first_diff_offset: first_diff,
            cpp_hex: hex_preview(cpp),
            rust_hex: hex_preview(rust),
        }
    }

    /// True when the two bodies are byte-identical.
    #[must_use]
    pub fn is_identical(&self) -> bool {
        self.first_diff_offset.is_none()
    }
}

/// One entry in the aligned diff walk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlignedOp {
    pub kind: OpKind,
    pub direction: Direction,
    pub opcode: u16,
    pub name: String,
    /// Index within the C++ same-direction stream (for `Match`/`MissingInRust`).
    pub cpp_index: Option<usize>,
    /// Index within the Rust same-direction stream (for `Match`/`ExtraInRust`).
    pub rust_index: Option<usize>,
    /// Body comparison, present only on `Match`.
    pub body: Option<BodyDiff>,
    /// Realm/instance routing comparison, present only on `Match`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub connection: Option<ConnectionDiff>,
}

/// Summary tallies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffCounts {
    pub matched: usize,
    pub body_mismatches: usize,
    #[serde(default)]
    pub connection_mismatches: usize,
    pub missing_in_rust: usize,
    pub extra_in_rust: usize,
}

/// Full diff result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffReport {
    pub cpp_source: String,
    pub rust_source: String,
    pub directions: Vec<Direction>,
    pub ops: Vec<AlignedOp>,
    pub counts: DiffCounts,
}

/// A stable, position-independent divergence record used for the committed
/// regression baseline (`expected-divergences.json`). It deliberately omits
/// volatile absolute indices and hex previews so the baseline only trips on a
/// real behavioral change, not on unrelated packet shifts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivergenceSignature {
    pub kind: DivergenceKind,
    pub direction: Direction,
    /// Opcode as a `0xXXXX` string for human-readable baselines.
    pub opcode: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cpp_body_len: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rust_body_len: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub first_diff_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cpp_connection_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rust_connection_id: Option<u32>,
}

impl DiffReport {
    /// Compute the diff of `cpp` (golden) vs `rust` over the given directions.
    #[must_use]
    pub fn compute(cpp: &Capture, rust: &Capture, directions: &[Direction]) -> DiffReport {
        let mut ops = Vec::new();
        let mut counts = DiffCounts::default();

        for &dir in directions {
            let cpp_pkts = cpp.by_direction(dir);
            let rust_pkts = rust.by_direction(dir);
            let cpp_ops: Vec<u16> = cpp_pkts.iter().map(|p| p.opcode).collect();
            let rust_ops: Vec<u16> = rust_pkts.iter().map(|p| p.opcode).collect();

            for step in align(&cpp_ops, &rust_ops) {
                match step {
                    Step::Match(i, j) => {
                        let body = BodyDiff::compute(&cpp_pkts[i].body, &rust_pkts[j].body);
                        let connection = ConnectionDiff {
                            cpp_connection_id: cpp_pkts[i].connection_id,
                            rust_connection_id: rust_pkts[j].connection_id,
                        };
                        if !body.is_identical() {
                            counts.body_mismatches += 1;
                        }
                        if !connection.is_identical() {
                            counts.connection_mismatches += 1;
                        }
                        if body.is_identical() && connection.is_identical() {
                            counts.matched += 1;
                        }
                        ops.push(AlignedOp {
                            kind: OpKind::Match,
                            direction: dir,
                            opcode: cpp_pkts[i].opcode,
                            name: cpp_pkts[i].opcode_name(),
                            cpp_index: Some(i),
                            rust_index: Some(j),
                            body: Some(body),
                            connection: Some(connection),
                        });
                    }
                    Step::Del(i) => {
                        counts.missing_in_rust += 1;
                        ops.push(missing(dir, cpp_pkts[i], i));
                    }
                    Step::Ins(j) => {
                        counts.extra_in_rust += 1;
                        ops.push(extra(dir, rust_pkts[j], j));
                    }
                }
            }
        }

        DiffReport {
            cpp_source: cpp.source.clone(),
            rust_source: rust.source.clone(),
            directions: directions.to_vec(),
            ops,
            counts,
        }
    }

    /// True when there are no divergences of any class.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.counts.body_mismatches == 0
            && self.counts.connection_mismatches == 0
            && self.counts.missing_in_rust == 0
            && self.counts.extra_in_rust == 0
    }

    /// Stable divergence signatures for the regression baseline.
    #[must_use]
    pub fn signatures(&self) -> Vec<DivergenceSignature> {
        let mut sigs = Vec::new();
        for op in &self.ops {
            match op.kind {
                OpKind::MissingInRust => sigs.push(DivergenceSignature {
                    kind: DivergenceKind::MissingInRust,
                    direction: op.direction,
                    opcode: format!("0x{:04X}", op.opcode),
                    name: op.name.clone(),
                    cpp_body_len: None,
                    rust_body_len: None,
                    first_diff_offset: None,
                    cpp_connection_id: None,
                    rust_connection_id: None,
                }),
                OpKind::ExtraInRust => sigs.push(DivergenceSignature {
                    kind: DivergenceKind::ExtraInRust,
                    direction: op.direction,
                    opcode: format!("0x{:04X}", op.opcode),
                    name: op.name.clone(),
                    cpp_body_len: None,
                    rust_body_len: None,
                    first_diff_offset: None,
                    cpp_connection_id: None,
                    rust_connection_id: None,
                }),
                OpKind::Match => {
                    if let Some(connection) = &op.connection {
                        if !connection.is_identical() {
                            sigs.push(DivergenceSignature {
                                kind: DivergenceKind::ConnectionMismatch,
                                direction: op.direction,
                                opcode: format!("0x{:04X}", op.opcode),
                                name: op.name.clone(),
                                cpp_body_len: None,
                                rust_body_len: None,
                                first_diff_offset: None,
                                cpp_connection_id: Some(connection.cpp_connection_id),
                                rust_connection_id: Some(connection.rust_connection_id),
                            });
                        }
                    }
                    if let Some(body) = &op.body {
                        if !body.is_identical() {
                            sigs.push(DivergenceSignature {
                                kind: DivergenceKind::BodyMismatch,
                                direction: op.direction,
                                opcode: format!("0x{:04X}", op.opcode),
                                name: op.name.clone(),
                                cpp_body_len: Some(body.cpp_len),
                                rust_body_len: Some(body.rust_len),
                                first_diff_offset: body.first_diff_offset,
                                cpp_connection_id: None,
                                rust_connection_id: None,
                            });
                        }
                    }
                }
            }
        }
        sigs
    }

    /// Render a human-readable report for the terminal.
    #[must_use]
    pub fn render_text(&self) -> String {
        use std::fmt::Write as _;
        let mut s = String::new();
        let _ = writeln!(s, "capture-diff: C++ (golden) vs Rust");
        let _ = writeln!(s, "  cpp : {}", self.cpp_source);
        let _ = writeln!(s, "  rust: {}", self.rust_source);
        let dirs: Vec<&str> = self.directions.iter().map(|d| d.tag()).collect();
        let _ = writeln!(s, "  directions: {}", dirs.join(", "));
        let _ = writeln!(s);

        for op in &self.ops {
            match op.kind {
                OpKind::Match => {
                    if let Some(connection) = &op.connection {
                        if !connection.is_identical() {
                            let _ = writeln!(
                                s,
                                "~ ROUTE  [{}] 0x{:04X} {} connection cpp={} rust={}",
                                op.direction,
                                op.opcode,
                                op.name,
                                connection.cpp_connection_id,
                                connection.rust_connection_id,
                            );
                        }
                    }
                    if let Some(body) = &op.body {
                        if !body.is_identical() {
                            let _ = writeln!(
                                s,
                                "~ VALUE  [{}] 0x{:04X} {} body cpp={}B rust={}B first_diff=@{}",
                                op.direction,
                                op.opcode,
                                op.name,
                                body.cpp_len,
                                body.rust_len,
                                body.first_diff_offset
                                    .map_or_else(|| "-".to_string(), |o| o.to_string()),
                            );
                            let _ = writeln!(s, "         cpp : {}", body.cpp_hex);
                            let _ = writeln!(s, "         rust: {}", body.rust_hex);
                        }
                    }
                }
                OpKind::MissingInRust => {
                    let _ = writeln!(
                        s,
                        "- MISS   [{}] 0x{:04X} {} present in C++ (#{}), absent in Rust",
                        op.direction,
                        op.opcode,
                        op.name,
                        op.cpp_index.unwrap_or(0),
                    );
                }
                OpKind::ExtraInRust => {
                    let _ = writeln!(
                        s,
                        "+ EXTRA  [{}] 0x{:04X} {} present in Rust (#{}), absent in C++",
                        op.direction,
                        op.opcode,
                        op.name,
                        op.rust_index.unwrap_or(0),
                    );
                }
            }
        }

        let c = &self.counts;
        let _ = writeln!(s);
        let _ = writeln!(
            s,
            "summary: {} matched, {} value-diffs, {} routing-diffs, {} missing-in-rust, {} extra-in-rust",
            c.matched,
            c.body_mismatches,
            c.connection_mismatches,
            c.missing_in_rust,
            c.extra_in_rust
        );
        let _ = writeln!(
            s,
            "result : {}",
            if self.is_clean() {
                "CLEAN ✓ (capture matches C++)"
            } else {
                "DIVERGENT ✗"
            }
        );
        s
    }
}

/// Multiset delta of a fresh diff's signatures against an accepted baseline.
///
/// Comparison is **count-aware** (a multiset diff, not set membership) so a
/// change in the multiplicity of an otherwise-identical signature — e.g. 3 of 4
/// MOTD `ChatServerMessage` lines now sent instead of 1 — surfaces, instead of
/// being hidden because every distinct signature is still present on both sides.
/// It is intentionally **order-insensitive** (signatures omit absolute indices);
/// the committed golden test additionally pins order via `assert_eq`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineDelta {
    /// Signatures present more often in the fresh diff than in the baseline
    /// (regressions or genuinely new divergences).
    pub new: Vec<DivergenceSignature>,
    /// Baseline signatures no longer produced as often (fixed divergences).
    pub fixed: Vec<DivergenceSignature>,
}

impl BaselineDelta {
    /// True when the fresh diff's signature multiset equals the baseline's.
    #[must_use]
    pub fn matches(&self) -> bool {
        self.new.is_empty() && self.fixed.is_empty()
    }
}

/// Compute the count-aware delta of `actual` signatures vs an `expected` baseline.
#[must_use]
pub fn baseline_delta(
    actual: &[DivergenceSignature],
    expected: &[DivergenceSignature],
) -> BaselineDelta {
    // Consume-on-match: each actual signature cancels one equal baseline entry,
    // so surplus actuals become `new` and unmatched baseline entries `fixed`.
    let mut remaining = expected.to_vec();
    let mut new = Vec::new();
    for sig in actual {
        if let Some(pos) = remaining.iter().position(|e| e == sig) {
            remaining.remove(pos);
        } else {
            new.push(sig.clone());
        }
    }
    BaselineDelta {
        new,
        fixed: remaining,
    }
}

fn missing(dir: Direction, pkt: &CapturedPacket, cpp_index: usize) -> AlignedOp {
    AlignedOp {
        kind: OpKind::MissingInRust,
        direction: dir,
        opcode: pkt.opcode,
        name: pkt.opcode_name(),
        cpp_index: Some(cpp_index),
        rust_index: None,
        body: None,
        connection: None,
    }
}

fn extra(dir: Direction, pkt: &CapturedPacket, rust_index: usize) -> AlignedOp {
    AlignedOp {
        kind: OpKind::ExtraInRust,
        direction: dir,
        opcode: pkt.opcode,
        name: pkt.opcode_name(),
        cpp_index: None,
        rust_index: Some(rust_index),
        body: None,
        connection: None,
    }
}

fn hex_preview(body: &[u8]) -> String {
    let shown = body.len().min(HEX_PREVIEW_BYTES);
    let mut s: String = body[..shown]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    if body.len() > shown {
        s.push_str(" …");
    }
    if body.is_empty() {
        s.push_str("(empty)");
    }
    s
}

/// One alignment step over the two opcode sequences.
enum Step {
    /// `a[i]` aligns with `b[j]` (same opcode).
    Match(usize, usize),
    /// `a[i]` has no counterpart in `b` (deletion / missing in Rust).
    Del(usize),
    /// `b[j]` has no counterpart in `a` (insertion / extra in Rust).
    Ins(usize),
}

/// Longest-common-subsequence alignment of two opcode streams.
///
/// Returns the edit script in forward order. `golden` is the C++ stream,
/// `rust` the Rust stream.
fn align(golden: &[u16], rust: &[u16]) -> Vec<Step> {
    let m = golden.len();
    let n = rust.len();
    // dp[i][j] = LCS length of golden[i..] and rust[j..].
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            dp[i][j] = if golden[i] == rust[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut steps = Vec::with_capacity(m + n);
    let (mut i, mut j) = (0, 0);
    while i < m && j < n {
        if golden[i] == rust[j] {
            steps.push(Step::Match(i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            steps.push(Step::Del(i));
            i += 1;
        } else {
            steps.push(Step::Ins(j));
            j += 1;
        }
    }
    while i < m {
        steps.push(Step::Del(i));
        i += 1;
    }
    while j < n {
        steps.push(Step::Ins(j));
        j += 1;
    }
    steps
}

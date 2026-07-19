//! Reader and writer for the RustyCore packet dump directory.
//!
//! When `RUSTYCORE_PACKET_DUMP_DIR` is set, the world server writes one pair of
//! files per packet (`dump_world_packet_like_cpp` in
//! `crates/wow-network/src/world_socket.rs`):
//!
//! - `rust-<dir>-<seq:08>-counter<n>-0x<opcode>-<name>-len<n>.bin` — the raw
//!   packet bytes **including** the 2-byte little-endian opcode prefix.
//! - `...meta` — `key=value` lines: `direction`, `addr`, `seq`, `counter`,
//!   `connection_id`, `opcode=0x....`, `name`, `len`. `len` is the full `.bin`
//!   length (opcode prefix + body). All fields and the canonical filename are
//!   checked; incomplete legacy sidecars are deliberately rejected because a
//!   required capture is acceptance evidence, not a best-effort log import.
//!   `direction` is `c2s`/`s2c`, or `c2s-unencrypted`/`s2c-unencrypted` for the
//!   pre-encryption handshake packets — both normalize via [`Direction::from_tag`].
//!
//! Packets are ordered by the global monotonic `seq` (c2s and s2c counters reset
//! independently, so `seq` — not `counter` — is the cross-direction order).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::model::{Capture, CapturedPacket, Direction};

/// A parsed `.meta` sidecar.
struct Meta {
    direction: Direction,
    connection_id: u32,
    seq: u64,
    len: usize,
    opcode: u16,
    bin_path: PathBuf,
}

const META_KEYS: [&str; 8] = [
    "direction",
    "connection_id",
    "addr",
    "seq",
    "counter",
    "opcode",
    "name",
    "len",
];
const RUST_CAPTURE_MANIFEST_FILE: &str = "rust.capture-manifest.json";
const RACE_BOT_REPORT_FILE: &str = "race.bot-report.json";

fn validate_race_bot_report_sidecar(dir: &Path, report_path: &Path) -> Result<()> {
    let manifest_path = dir.join(RUST_CAPTURE_MANIFEST_FILE);
    let manifest_metadata = std::fs::symlink_metadata(&manifest_path).with_context(|| {
        format!(
            "race bot report requires capture manifest {}",
            manifest_path.display()
        )
    })?;
    if manifest_metadata.file_type().is_symlink() || !manifest_metadata.file_type().is_file() {
        bail!(
            "race bot report capture manifest is not a regular non-symlink file: {}",
            manifest_path.display()
        );
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&manifest_path)
            .with_context(|| format!("reading capture manifest {}", manifest_path.display()))?,
    )
    .with_context(|| format!("parsing capture manifest {}", manifest_path.display()))?;
    let evidence = manifest
        .get("bot_report")
        .and_then(serde_json::Value::as_object)
        .context("race capture manifest is missing bot_report evidence")?;
    let expected_sha = evidence
        .get("report_sha256")
        .and_then(serde_json::Value::as_str)
        .context("race capture manifest bot_report SHA-256 is missing")?;
    let declared_path = evidence
        .get("report_path")
        .and_then(serde_json::Value::as_str)
        .context("race capture manifest bot_report path is missing")?;
    if manifest.get("flow").and_then(serde_json::Value::as_str)
        != Some("loot-two-session-atomic-race")
        || evidence.get("contract").and_then(serde_json::Value::as_str)
            != Some("wow-test-bot-loot-two-session-atomic-race-report-v1")
        || Path::new(declared_path)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(RACE_BOT_REPORT_FILE)
    {
        bail!("race bot report sidecar is not declared by the matching race manifest contract");
    }
    let report = std::fs::read(report_path)
        .with_context(|| format!("reading race bot report {}", report_path.display()))?;
    let actual_sha = format!("{:x}", Sha256::digest(&report));
    if actual_sha != expected_sha {
        bail!("race bot report SHA-256 does not match its capture manifest");
    }
    Ok(())
}

fn parse_decimal<T>(path: &Path, key: &str, value: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value.parse::<T>().map_err(|error| {
        anyhow::anyhow!(
            "meta {} has invalid {key} {value:?}: {error}",
            path.display()
        )
    })
}

fn parse_meta(path: &Path, expected_stem: &str) -> Result<Meta> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading meta {}", path.display()))?;
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            bail!("meta {} has malformed line {line:?}", path.display());
        };
        if !META_KEYS.contains(&key) {
            bail!("meta {} has unknown field {key:?}", path.display());
        }
        if value.trim() != value || value.is_empty() {
            bail!(
                "meta {} has empty or non-canonical value for {key}",
                path.display()
            );
        }
        if fields.insert(key, value).is_some() {
            bail!("meta {} repeats field {key:?}", path.display());
        }
    }

    for key in META_KEYS {
        if !fields.contains_key(key) {
            bail!("meta {} is missing field {key:?}", path.display());
        }
    }

    let direction_tag = fields["direction"];
    if !matches!(
        direction_tag,
        "c2s" | "s2c" | "c2s-unencrypted" | "s2c-unencrypted"
    ) {
        bail!("meta {} has invalid direction", path.display());
    }
    let direction = Direction::from_tag(direction_tag)
        .with_context(|| format!("meta {} has invalid direction", path.display()))?;
    fields["addr"]
        .parse::<std::net::SocketAddr>()
        .with_context(|| {
            format!(
                "meta {} has invalid socket address {:?}",
                path.display(),
                fields["addr"]
            )
        })?;
    let connection_id = parse_decimal::<u32>(path, "connection_id", fields["connection_id"])?;
    if connection_id > 1 {
        bail!(
            "meta {} has unsupported world connection_id {connection_id}",
            path.display()
        );
    }
    let seq = parse_decimal::<u64>(path, "seq", fields["seq"])?;
    let counter = parse_decimal::<u64>(path, "counter", fields["counter"])?;
    let len = parse_decimal::<usize>(path, "len", fields["len"])?;
    if len < 2 {
        bail!(
            "meta {} declares len {len}, smaller than opcode",
            path.display()
        );
    }
    let opcode_text = fields["opcode"];
    let Some(hex) = opcode_text.strip_prefix("0x") else {
        bail!("meta {} has non-canonical opcode", path.display());
    };
    let opcode = u16::from_str_radix(hex, 16)
        .with_context(|| format!("meta {} has invalid opcode", path.display()))?;
    if opcode_text != format!("0x{opcode:04X}") {
        bail!("meta {} has non-canonical opcode", path.display());
    }
    let name = fields["name"];
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        bail!("meta {} has unsafe packet name {name:?}", path.display());
    }
    let canonical_stem =
        format!("rust-{direction_tag}-{seq:08}-counter{counter}-0x{opcode:04X}-{name}-len{len}");
    if expected_stem != canonical_stem {
        bail!(
            "meta {} filename stem disagrees with its metadata; expected {canonical_stem:?}",
            path.display()
        );
    }
    let bin_path = path.with_extension("bin");
    Ok(Meta {
        direction,
        connection_id,
        seq,
        len,
        opcode,
        bin_path,
    })
}

fn require_plain_directory(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("inspecting rust dump path {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        bail!(
            "rust dump path is not a regular directory: {}",
            path.display()
        );
    }
    Ok(())
}

/// Parse a RustyCore packet dump directory into a normalized [`Capture`].
pub fn parse_rust_dump(dir: &Path) -> Result<Capture> {
    require_plain_directory(dir)?;

    let mut meta_paths = BTreeMap::<String, PathBuf>::new();
    let mut bin_paths = BTreeMap::<String, PathBuf>::new();
    let mut race_bot_report_path = None;
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("reading dump dir {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .with_context(|| format!("inspecting dump entry {}", path.display()))?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            bail!(
                "rust dump entry is not a regular non-symlink file: {}",
                path.display()
            );
        }
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("rust dump contains a non-UTF-8 filename"))?;
        if file_name == RUST_CAPTURE_MANIFEST_FILE {
            continue;
        }
        if file_name == RACE_BOT_REPORT_FILE {
            race_bot_report_path = Some(path);
            continue;
        }
        let (stem, paths) = if let Some(stem) = file_name.strip_suffix(".meta") {
            (stem, &mut meta_paths)
        } else if let Some(stem) = file_name.strip_suffix(".bin") {
            (stem, &mut bin_paths)
        } else {
            bail!("unexpected file in rust dump: {}", path.display());
        };
        if stem.is_empty() || paths.insert(stem.to_string(), path.clone()).is_some() {
            bail!("duplicate or empty packet stem in rust dump: {file_name}");
        }
    }

    if let Some(report_path) = race_bot_report_path {
        validate_race_bot_report_sidecar(dir, &report_path)?;
    }

    if meta_paths.is_empty() {
        bail!("no .meta files found in dump dir {}", dir.display());
    }

    let meta_stems = meta_paths.keys().cloned().collect::<BTreeSet<_>>();
    let bin_stems = bin_paths.keys().cloned().collect::<BTreeSet<_>>();
    if meta_stems != bin_stems {
        let orphan_meta = meta_stems
            .difference(&bin_stems)
            .cloned()
            .collect::<Vec<_>>();
        let orphan_bin = bin_stems
            .difference(&meta_stems)
            .cloned()
            .collect::<Vec<_>>();
        bail!(
            "rust dump has unpaired packet files (orphan .meta: {orphan_meta:?}; orphan .bin: {orphan_bin:?})"
        );
    }

    let mut metas = meta_paths
        .iter()
        .map(|(stem, path)| parse_meta(path, stem))
        .collect::<Result<Vec<_>>>()?;

    // Global seq is the canonical cross-direction wire order.
    metas.sort_by_key(|m| m.seq);
    for pair in metas.windows(2) {
        let Some(expected_next) = pair[0].seq.checked_add(1) else {
            bail!("packet dump seq overflow after {}", pair[0].seq);
        };
        if pair[1].seq != expected_next {
            bail!(
                "non-contiguous packet dump seq {} then {} in {} and {}; the world process may have restarted or files are missing",
                pair[0].seq,
                pair[1].seq,
                pair[0].bin_path.display(),
                pair[1].bin_path.display(),
            );
        }
    }

    let mut packets = Vec::with_capacity(metas.len());
    for meta in metas {
        let bin = std::fs::read(&meta.bin_path)
            .with_context(|| format!("reading dump body {}", meta.bin_path.display()))?;
        if bin.len() != meta.len {
            bail!(
                "dump body {} has {} bytes but metadata declares {}",
                meta.bin_path.display(),
                bin.len(),
                meta.len
            );
        }
        if bin.len() < 2 {
            bail!(
                "dump body {} too short ({} bytes) to hold an opcode",
                meta.bin_path.display(),
                bin.len()
            );
        }
        let bin_opcode = u16::from_le_bytes([bin[0], bin[1]]);
        if bin_opcode != meta.opcode {
            bail!(
                "dump body {} opcode 0x{bin_opcode:04X} disagrees with meta opcode 0x{:04X}",
                meta.bin_path.display(),
                meta.opcode
            );
        }
        packets.push(CapturedPacket {
            direction: meta.direction,
            connection_id: meta.connection_id,
            opcode: meta.opcode,
            // Strip the 2-byte opcode prefix so the body matches PKT normalization.
            body: bin[2..].to_vec(),
        });
    }

    Ok(Capture::new(dir.display().to_string(), packets))
}

/// Write a [`Capture`] to a dump directory in RustyCore's native layout.
///
/// Used by the fixture generator. The `.bin` re-prepends the 2-byte opcode
/// prefix so the file is byte-identical to a real live dump.
pub fn write_rust_dump(dir: &Path, capture: &Capture) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating dump dir {}", dir.display()))?;
    for (seq, pkt) in capture.packets.iter().enumerate() {
        let name = pkt.opcode_name();
        let mut bin = Vec::with_capacity(pkt.body.len() + 2);
        bin.extend_from_slice(&pkt.opcode.to_le_bytes());
        bin.extend_from_slice(&pkt.body);
        let stem = format!(
            "rust-{}-{seq:08}-counter{seq}-0x{:04X}-{name}-len{}",
            pkt.direction.tag(),
            pkt.opcode,
            bin.len()
        );
        std::fs::write(dir.join(format!("{stem}.bin")), &bin)?;
        let meta = format!(
            "direction={}\nconnection_id={}\naddr=127.0.0.1:0\nseq={seq}\ncounter={seq}\nopcode=0x{:04X}\nname={name}\nlen={}\n",
            pkt.direction.tag(),
            pkt.connection_id,
            pkt.opcode,
            bin.len()
        );
        std::fs::write(dir.join(format!("{stem}.meta")), meta)?;
    }
    Ok(())
}

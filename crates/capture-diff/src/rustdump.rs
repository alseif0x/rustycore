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
//!   length (opcode prefix + body); the parser ignores it and derives the body
//!   from `bin[2..]`. Legacy metadata without `connection_id` defaults to the
//!   realm connection (`0`).
//!   `direction` is `c2s`/`s2c`, or `c2s-unencrypted`/`s2c-unencrypted` for the
//!   pre-encryption handshake packets — both normalize via [`Direction::from_tag`].
//!
//! Packets are ordered by the global monotonic `seq` (c2s and s2c counters reset
//! independently, so `seq` — not `counter` — is the cross-direction order).

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::model::{Capture, CapturedPacket, Direction};

/// A parsed `.meta` sidecar.
struct Meta {
    direction: Direction,
    connection_id: u32,
    seq: u64,
    opcode: u16,
    bin_path: std::path::PathBuf,
}

fn parse_meta(path: &Path) -> Result<Meta> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading meta {}", path.display()))?;
    let mut direction = None;
    let mut connection_id = None;
    let mut seq = None;
    let mut opcode = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "direction" => direction = Direction::from_tag(value),
            "connection_id" => {
                connection_id = Some(value.trim().parse::<u32>().with_context(|| {
                    format!(
                        "meta {} has invalid connection_id {:?}",
                        path.display(),
                        value.trim()
                    )
                })?);
            }
            "seq" => seq = value.trim().parse::<u64>().ok(),
            "opcode" => {
                let v = value
                    .trim()
                    .trim_start_matches("0x")
                    .trim_start_matches("0X");
                opcode = u16::from_str_radix(v, 16).ok();
            }
            _ => {}
        }
    }
    let direction =
        direction.with_context(|| format!("meta {} missing/invalid direction", path.display()))?;
    let seq = seq.with_context(|| format!("meta {} missing seq", path.display()))?;
    let opcode = opcode.with_context(|| format!("meta {} missing opcode", path.display()))?;
    let bin_path = path.with_extension("bin");
    Ok(Meta {
        direction,
        connection_id: connection_id.unwrap_or(0),
        seq,
        opcode,
        bin_path,
    })
}

/// Parse a RustyCore packet dump directory into a normalized [`Capture`].
pub fn parse_rust_dump(dir: &Path) -> Result<Capture> {
    if !dir.is_dir() {
        bail!("rust dump path is not a directory: {}", dir.display());
    }

    let mut metas = Vec::new();
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("reading dump dir {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("meta") {
            metas.push(parse_meta(&path)?);
        }
    }

    if metas.is_empty() {
        bail!("no .meta files found in dump dir {}", dir.display());
    }

    // Global seq is the canonical cross-direction wire order.
    metas.sort_by_key(|m| m.seq);
    for pair in metas.windows(2) {
        if pair[0].seq == pair[1].seq {
            bail!(
                "duplicate packet dump seq {} in {} and {}; the world process may have restarted during capture",
                pair[0].seq,
                pair[0].bin_path.display(),
                pair[1].bin_path.display(),
            );
        }
    }

    let mut packets = Vec::with_capacity(metas.len());
    for meta in metas {
        let bin = std::fs::read(&meta.bin_path)
            .with_context(|| format!("reading dump body {}", meta.bin_path.display()))?;
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

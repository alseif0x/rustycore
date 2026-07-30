// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Parser for the linked handler-registry snapshot.

use std::collections::BTreeSet;

const CONTRACT_HEADER: &str =
    "opcode_value\topcode_name\thandler_name\tsession_status\tpacket_processing";

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SnapshotContract {
    pub(crate) row_count: usize,
    pub(crate) opcode_names: BTreeSet<String>,
}

pub(crate) fn parse_snapshot_contract(snapshot: &str) -> Result<SnapshotContract, String> {
    let mut lines = snapshot
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#'));
    let Some((header_line, header)) = lines.next() else {
        return Err("handler contract snapshot has no header".to_owned());
    };
    if header != CONTRACT_HEADER {
        return Err(format!(
            "handler contract header at line {}: expected {CONTRACT_HEADER:?}, actual {header:?}",
            header_line + 1
        ));
    }

    let mut opcode_names = BTreeSet::new();
    let mut opcode_values = BTreeSet::new();
    let mut row_count = 0usize;
    for (line_index, line) in lines {
        if line.is_empty() {
            return Err(format!(
                "handler contract contains an empty row at line {}",
                line_index + 1
            ));
        }
        let columns: Vec<_> = line.split('\t').collect();
        let [
            opcode_value,
            opcode_name,
            handler_name,
            session_status,
            packet_processing,
        ] = columns.as_slice()
        else {
            return Err(format!(
                "handler contract line {} has {} columns; expected 5",
                line_index + 1,
                columns.len()
            ));
        };
        if [opcode_name, handler_name, session_status, packet_processing]
            .iter()
            .any(|column| column.is_empty())
        {
            return Err(format!(
                "handler contract line {} contains an empty field",
                line_index + 1
            ));
        }
        let opcode_value = opcode_value
            .strip_prefix("0x")
            .and_then(|value| u32::from_str_radix(value, 16).ok())
            .ok_or_else(|| {
                format!(
                    "handler contract line {} has invalid opcode value {opcode_value:?}",
                    line_index + 1
                )
            })?;
        if !opcode_values.insert(opcode_value) {
            return Err(format!(
                "handler contract line {} duplicates opcode value 0x{opcode_value:04X}",
                line_index + 1
            ));
        }
        if !opcode_names.insert((*opcode_name).to_owned()) {
            return Err(format!(
                "handler contract line {} duplicates opcode name {opcode_name}",
                line_index + 1
            ));
        }
        row_count += 1;
    }
    if row_count == 0 {
        return Err("handler contract snapshot has no data rows".to_owned());
    }

    Ok(SnapshotContract {
        row_count,
        opcode_names,
    })
}

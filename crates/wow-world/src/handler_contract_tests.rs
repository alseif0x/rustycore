// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Regression guard for the complete world-handler registration contract.
//!
//! The dispatcher is assembled through `inventory`, so checking source files or
//! registration counts alone cannot prove which handlers are linked into the
//! final `wow-world` test binary. This module snapshots the effective registry:
//! opcode value/name, handler name, session status, and processing mode.

use wow_handler::PacketHandlerEntry;

const CONTRACT_SNAPSHOT: &str =
    include_str!("../../../tools/architecture/world-handler-contract.tsv");
const CONTRACT_HEADER: &str =
    "opcode_value\topcode_name\thandler_name\tsession_status\tpacket_processing";

#[derive(Clone, Debug, Eq, PartialEq)]
struct HandlerContractRow {
    opcode_value: u32,
    opcode_name: String,
    handler_name: String,
    session_status: String,
    packet_processing: String,
}

impl HandlerContractRow {
    fn from_entry(entry: &PacketHandlerEntry) -> Self {
        Self {
            opcode_value: entry.opcode as u32,
            opcode_name: format!("{:?}", entry.opcode),
            handler_name: entry.handler_name.to_owned(),
            session_status: format!("{:?}", entry.status),
            packet_processing: format!("{:?}", entry.processing),
        }
    }

    fn display(&self) -> String {
        format!(
            "0x{:04X} {} handler={} status={} processing={}",
            self.opcode_value,
            self.opcode_name,
            self.handler_name,
            self.session_status,
            self.packet_processing
        )
    }
}

fn registered_contract() -> Vec<HandlerContractRow> {
    let mut rows: Vec<_> = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .map(HandlerContractRow::from_entry)
        .collect();
    rows.sort_by(|left, right| {
        left.opcode_value
            .cmp(&right.opcode_value)
            .then_with(|| left.opcode_name.cmp(&right.opcode_name))
            .then_with(|| left.handler_name.cmp(&right.handler_name))
    });
    rows
}

fn render_contract(rows: &[HandlerContractRow]) -> String {
    let mut rendered = String::from(
        "# Generated from the linked inventory registry; do not hand-edit rows.\n\
         # Refresh deliberately with: cargo test -p wow-world print_world_handler_contract_snapshot --lib -- --ignored --nocapture\n",
    );
    rendered.push_str(CONTRACT_HEADER);
    rendered.push('\n');
    for row in rows {
        rendered.push_str(&format!(
            "0x{:04X}\t{}\t{}\t{}\t{}\n",
            row.opcode_value,
            row.opcode_name,
            row.handler_name,
            row.session_status,
            row.packet_processing
        ));
    }
    rendered
}

fn parse_contract(snapshot: &str) -> Result<Vec<HandlerContractRow>, String> {
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

    let mut rows = Vec::new();
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
        let opcode_value = opcode_value
            .strip_prefix("0x")
            .and_then(|value| u32::from_str_radix(value, 16).ok())
            .ok_or_else(|| {
                format!(
                    "handler contract line {} has invalid opcode value {opcode_value:?}",
                    line_index + 1
                )
            })?;
        rows.push(HandlerContractRow {
            opcode_value,
            opcode_name: (*opcode_name).to_owned(),
            handler_name: (*handler_name).to_owned(),
            session_status: (*session_status).to_owned(),
            packet_processing: (*packet_processing).to_owned(),
        });
    }
    Ok(rows)
}

fn compare_contract(
    expected: &[HandlerContractRow],
    actual: &[HandlerContractRow],
) -> Result<(), String> {
    for index in 0..expected.len().max(actual.len()) {
        let row_number = index + 1;
        let Some(expected_row) = expected.get(index) else {
            return Err(format!(
                "handler contract row {row_number}: unexpected actual {}",
                actual[index].display()
            ));
        };
        let Some(actual_row) = actual.get(index) else {
            return Err(format!(
                "handler contract row {row_number}: missing actual {}",
                expected_row.display()
            ));
        };

        let fields = [
            (
                "opcode_value",
                format!("0x{:04X}", expected_row.opcode_value),
                format!("0x{:04X}", actual_row.opcode_value),
            ),
            (
                "opcode_name",
                expected_row.opcode_name.clone(),
                actual_row.opcode_name.clone(),
            ),
            (
                "handler_name",
                expected_row.handler_name.clone(),
                actual_row.handler_name.clone(),
            ),
            (
                "session_status",
                expected_row.session_status.clone(),
                actual_row.session_status.clone(),
            ),
            (
                "packet_processing",
                expected_row.packet_processing.clone(),
                actual_row.packet_processing.clone(),
            ),
        ];
        if let Some((field, expected_value, actual_value)) = fields
            .into_iter()
            .find(|(_, expected_value, actual_value)| expected_value != actual_value)
        {
            return Err(format!(
                "handler contract row {row_number} ({}) field {field}: expected {expected_value:?}, actual {actual_value:?}",
                expected_row.opcode_name
            ));
        }
    }
    Ok(())
}

fn contract_row(
    opcode_value: u32,
    opcode_name: &str,
    handler_name: &str,
    session_status: &str,
    packet_processing: &str,
) -> HandlerContractRow {
    HandlerContractRow {
        opcode_value,
        opcode_name: opcode_name.to_owned(),
        handler_name: handler_name.to_owned(),
        session_status: session_status.to_owned(),
        packet_processing: packet_processing.to_owned(),
    }
}

#[test]
fn world_handler_contract_matches_snapshot() {
    let expected = parse_contract(CONTRACT_SNAPSHOT)
        .unwrap_or_else(|error| panic!("invalid world handler contract snapshot: {error}"));
    let actual = registered_contract();

    if let Err(error) = compare_contract(&expected, &actual) {
        panic!(
            "{error} (expected {} rows, actual {})\n\
             update tools/architecture/world-handler-contract.tsv only after auditing the \
             opcode, handler, status, and processing change",
            expected.len(),
            actual.len()
        );
    }
}

#[test]
fn contract_comparison_reports_handler_name_and_metadata_drift() {
    let expected = vec![contract_row(
        0x1234,
        "ExampleOpcode",
        "handle_example",
        "LoggedIn",
        "ThreadUnsafe",
    )];

    for (actual, expected_field) in [
        (
            vec![contract_row(
                0x1234,
                "ExampleOpcode",
                "handle_other",
                "LoggedIn",
                "ThreadUnsafe",
            )],
            "field handler_name",
        ),
        (
            vec![contract_row(
                0x1234,
                "ExampleOpcode",
                "handle_example",
                "Authed",
                "ThreadUnsafe",
            )],
            "field session_status",
        ),
        (
            vec![contract_row(
                0x1234,
                "ExampleOpcode",
                "handle_example",
                "LoggedIn",
                "Inplace",
            )],
            "field packet_processing",
        ),
    ] {
        let error =
            compare_contract(&expected, &actual).expect_err("contract drift must be reported");
        assert!(
            error.contains(expected_field),
            "expected {expected_field:?} in drift report, got {error:?}"
        );
    }
}

/// Prints the canonical snapshot to stdout for a deliberate, reviewable refresh.
///
/// Copy only the text between the markers into
/// `tools/architecture/world-handler-contract.tsv`, audit the diff, and run the
/// non-ignored contract test. The test never overwrites the baseline itself.
#[test]
#[ignore = "manual snapshot refresh helper"]
fn print_world_handler_contract_snapshot() {
    println!(
        "----- BEGIN world-handler-contract.tsv -----\n{}----- END world-handler-contract.tsv -----",
        render_contract(&registered_contract())
    );
}

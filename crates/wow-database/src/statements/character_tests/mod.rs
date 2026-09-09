//! Character-statement C++ contrast regressions.
//!
//! Separated from the character_tests.rs root under #652.

//! Behaviour tests for [`super`].
//!
//! Extracted from `character.rs`, which was 6,684 lines of which
//! 3,289 — 49% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use std::path::PathBuf;

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use crate::{CharStatements, CharacterDatabase, Database};

use super::*;

fn cpp_character_database_cpp() -> PathBuf {
    let root = std::env::var_os("RUSTYCORE_CPP_REFERENCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/server/woltk-trinity-legacy"));
    root.join("src/server/database/Database/Implementation/CharacterDatabase.cpp")
}

fn cpp_string_literals(block: &str) -> String {
    let mut output = String::new();
    let bytes = block.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }

        i += 1;
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                if i + 1 < bytes.len() {
                    output.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
            }
            if bytes[i] == b'"' {
                i += 1;
                break;
            }
            output.push(bytes[i] as char);
            i += 1;
        }
    }
    output
}

fn select_item_instance_content(cpp: &str) -> String {
    let start = cpp
        .find("#define SelectItemInstanceContent")
        .expect("C++ SelectItemInstanceContent macro must exist");
    let end = cpp[start..]
        .find("\n\n")
        .map(|offset| start + offset)
        .expect("C++ SelectItemInstanceContent macro block must end before statements");
    cpp_string_literals(&cpp[start..end])
}

fn cpp_character_sql() -> Vec<String> {
    let contents = std::fs::read_to_string(cpp_character_database_cpp())
        .expect("C++ CharacterDatabase.cpp must be available for parity tests");
    let item_content = select_item_instance_content(&contents);
    let mut sql = Vec::new();
    let mut offset = 0;
    while let Some(relative_start) = contents[offset..].find("PrepareStatement(CHAR_") {
        let start = offset + relative_start;
        let Some(relative_end) = contents[start..].find("CONNECTION_") else {
            break;
        };
        let after_connection = start + relative_end;
        let Some(relative_stmt_end) = contents[after_connection..].find(");") else {
            break;
        };
        let end = after_connection + relative_stmt_end + 2;
        let block = &contents[start..end];
        let mut statement_sql = cpp_string_literals(block);
        if block.contains("SelectItemInstanceContent") {
            statement_sql =
                statement_sql.replacen("SELECT ,", &format!("SELECT {item_content},"), 1);
        }
        sql.push(statement_sql);
        offset = end;
    }
    sql
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
mod scenarios_4;

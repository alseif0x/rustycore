//! Static guard for module SQL (#1444 H6).
//!
//! A module may touch only the `characters` and `world` databases, and there
//! only tables whose name starts with its own prefix (`mod_<id with . as _>_`).
//! Its migrations may create, alter and index those tables and change their
//! rows; its purge scripts may only drop them. Anything else, including any
//! statement that reads another table, is refused before it reaches a
//! database.
//!
//! `tools/modules/module_sql.py` implements the same rules for `compose.py`.
//! Both read the shared vectors in `tools/modules/sql_guard_vectors.json`, so
//! the compose-time and apply-time guards cannot drift.

use super::split_sql;

/// Words that read or reach another table, or rename one away from the prefix.
const BANNED_WORDS: [&str; 7] = [
    "SELECT", "JOIN", "RENAME", "UNION", "OUTFILE", "DUMPFILE", "INFILE",
];

/// The table prefix a module owns: `mod_` + its id with `.` as `_` + `_`.
#[must_use]
pub fn table_prefix(module_id: &str) -> String {
    format!("mod_{}_", module_id.replace('.', "_"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    Quoted(String),
    Str,
    Punct(char),
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

/// Tokens with comments removed and string literals emptied. An executable
/// comment (`/*!` or `/*M!`) is refused: MariaDB runs what it contains.
fn tokens(statement: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = statement.chars().collect();
    let starts = |at: usize, text: &str| {
        text.chars()
            .enumerate()
            .all(|(offset, c)| chars.get(at + offset) == Some(&c))
    };
    let mut out = Vec::new();
    let mut index = 0;
    let n = chars.len();
    while index < n {
        let c = chars[index];
        if c.is_whitespace() {
            index += 1;
        } else if starts(index, "--") || c == '#' {
            while index < n && chars[index] != '\n' {
                index += 1;
            }
        } else if starts(index, "/*") {
            if starts(index, "/*!") || starts(index, "/*M!") {
                return Err("executable comment /*! ... */".into());
            }
            index += 2;
            while index < n && !starts(index, "*/") {
                index += 1;
            }
            index = (index + 2).min(n);
        } else if c == '\'' || c == '"' {
            index += 1;
            while index < n {
                if chars[index] == '\\' {
                    index += 2;
                } else if chars[index] == c {
                    if chars.get(index + 1) == Some(&c) {
                        index += 2;
                        continue;
                    }
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            out.push(Token::Str);
        } else if c == '`' {
            let end = chars[index + 1..]
                .iter()
                .position(|&ch| ch == '`')
                .ok_or("unterminated identifier")?;
            out.push(Token::Quoted(
                chars[index + 1..index + 1 + end].iter().collect(),
            ));
            index += end + 2;
        } else if is_ident_char(c) {
            let start = index;
            while index < n && is_ident_char(chars[index]) {
                index += 1;
            }
            out.push(Token::Word(chars[start..index].iter().collect()));
        } else {
            out.push(Token::Punct(c));
            index += 1;
        }
    }
    Ok(out)
}

struct Cursor<'a> {
    tokens: &'a [Token],
    prefix: &'a str,
    at: usize,
}

impl Cursor<'_> {
    /// Consume the keywords if they come next.
    fn word(&mut self, words: &[&str]) -> bool {
        let mut at = self.at;
        for word in words {
            match self.tokens.get(at) {
                Some(Token::Word(text)) if text.eq_ignore_ascii_case(word) => at += 1,
                _ => return false,
            }
        }
        self.at = at;
        true
    }

    fn punct(&mut self, expected: char) -> bool {
        if self.tokens.get(self.at) == Some(&Token::Punct(expected)) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    /// Consume an unqualified identifier; refuse a qualified one.
    fn ident(&mut self) -> Result<String, String> {
        let name = match self.tokens.get(self.at) {
            Some(Token::Word(text) | Token::Quoted(text)) => text.clone(),
            _ => return Err("expected a table or index name".into()),
        };
        self.at += 1;
        if self.tokens.get(self.at) == Some(&Token::Punct('.')) {
            return Err(format!(
                "qualified name {name}.…: a module names no database"
            ));
        }
        Ok(name)
    }

    fn own_table(&mut self) -> Result<String, String> {
        let name = self.ident()?;
        let rest = name.strip_prefix(self.prefix).unwrap_or("");
        if rest.is_empty()
            || !rest
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(format!(
                "table '{name}' is not the module's own (prefix '{}')",
                self.prefix
            ));
        }
        Ok(name)
    }

    fn done(&self) -> bool {
        self.at >= self.tokens.len()
    }
}

fn check_common(tokens: &[Token], prefix: &str) -> Result<(), String> {
    for (index, token) in tokens.iter().enumerate() {
        let Token::Word(text) = token else { continue };
        let upper = text.to_ascii_uppercase();
        if BANNED_WORDS.contains(&upper.as_str()) {
            return Err(format!("{upper} is not allowed in module SQL"));
        }
        if upper == "REFERENCES" {
            Cursor {
                tokens,
                prefix,
                at: index + 1,
            }
            .own_table()?;
        }
    }
    Ok(())
}

/// `Ok` only when the statement changes nothing but the module's own tables.
pub fn check_migration_statement(statement: &str, prefix: &str) -> Result<(), String> {
    let tokens = tokens(statement)?;
    check_common(&tokens, prefix)?;
    let mut c = Cursor {
        tokens: &tokens,
        prefix,
        at: 0,
    };
    if c.word(&["CREATE", "TABLE"]) {
        c.word(&["IF", "NOT", "EXISTS"]);
        c.own_table()?;
        if !c.punct('(') {
            return Err("CREATE TABLE must define its columns".into());
        }
    } else if c.word(&["ALTER", "TABLE"]) {
        c.own_table()?;
    } else if c.word(&["CREATE", "UNIQUE", "INDEX"])
        || c.word(&["CREATE", "INDEX"])
        || c.word(&["DROP", "INDEX"])
    {
        c.ident()?;
        if !c.word(&["ON"]) {
            return Err("an index statement must name its table with ON".into());
        }
        c.own_table()?;
    } else if c.word(&["INSERT", "IGNORE", "INTO"])
        || c.word(&["INSERT", "INTO"])
        || c.word(&["REPLACE", "INTO"])
    {
        c.own_table()?;
    } else if c.word(&["UPDATE"]) {
        c.own_table()?;
        if !c.word(&["SET"]) {
            return Err("UPDATE may change one module table only".into());
        }
    } else if c.word(&["DELETE", "FROM"]) {
        c.own_table()?;
        if !(c.done() || c.word(&["WHERE"]) || c.word(&["ORDER"]) || c.word(&["LIMIT"])) {
            return Err("DELETE may change one module table only".into());
        }
    } else {
        return Err("only CREATE/ALTER TABLE, CREATE/DROP INDEX, INSERT, REPLACE, UPDATE and DELETE on the module's tables are allowed".into());
    }
    Ok(())
}

/// `Ok` only when the statement drops nothing but the module's own tables.
pub fn check_purge_statement(statement: &str, prefix: &str) -> Result<(), String> {
    let tokens = tokens(statement)?;
    check_common(&tokens, prefix)?;
    let mut c = Cursor {
        tokens: &tokens,
        prefix,
        at: 0,
    };
    if !c.word(&["DROP", "TABLE"]) {
        return Err("a purge script may only DROP TABLE the module's tables".into());
    }
    c.word(&["IF", "EXISTS"]);
    c.own_table()?;
    while c.punct(',') {
        c.own_table()?;
    }
    if !c.done() {
        return Err("unexpected text after DROP TABLE".into());
    }
    Ok(())
}

/// Every problem in one script, as `statement N: reason`; empty when clean.
#[must_use]
pub fn check_script(content: &str, prefix: &str, purge: bool) -> Vec<String> {
    split_sql(content)
        .into_iter()
        .enumerate()
        .filter_map(|(index, statement)| {
            let result = if purge {
                check_purge_statement(statement, prefix)
            } else {
                check_migration_statement(statement, prefix)
            };
            result
                .err()
                .map(|reason| format!("statement {}: {reason}", index + 1))
        })
        .collect()
}

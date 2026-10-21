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

/// Words that read or reach another table, rename one away from the prefix,
/// move data between tables (partitions), reach files or other servers, or
/// call a side-effecting function. Matched as whole words, case-insensitively,
/// outside strings and comments. Mirrors `module_sql.py`'s `BANNED_WORDS`.
const BANNED_WORDS: &[&str] = &[
    "SELECT",
    "JOIN",
    "RENAME",
    "UNION",
    "OUTFILE",
    "DUMPFILE",
    "INFILE",
    "PARTITION",
    "PARTITIONS",
    "SUBPARTITION",
    "EXCHANGE",
    "TABLESPACE",
    "DIRECTORY",
    "CONNECTION",
    "FOR",
    "LOAD_FILE",
    "RELEASE_ALL_LOCKS",
    "RELEASE_LOCK",
    "GET_LOCK",
    "IS_FREE_LOCK",
    "IS_USED_LOCK",
    "SLEEP",
    "BENCHMARK",
    "NEXTVAL",
    "SETVAL",
    "LASTVAL",
    "MASTER_POS_WAIT",
    "MASTER_GTID_WAIT",
    "SYS_EXEC",
    "SYS_EVAL",
];

/// Storage engines a module table may use: none of them reaches another
/// table, file or server (CONNECT, FEDERATED, MERGE, SPIDER and so on would).
const ALLOWED_ENGINES: &[&str] = &["INNODB", "ARIA", "MYISAM", "MEMORY"];

/// Table options allowed after CREATE TABLE's columns and as ALTER clauses.
const TABLE_OPTIONS: &[&str] = &[
    "ENGINE",
    "CHARSET",
    "CHARACTER",
    "COLLATE",
    "COMMENT",
    "AUTO_INCREMENT",
    "ROW_FORMAT",
];

/// MariaDB starts a `--` comment only when the second dash is followed by
/// whitespace, a control character or the end of input; `1--1` is
/// arithmetic. Shared by the tokenizer and `split_sql`.
pub(super) fn dash_comment_at(bytes: &[u8], index: usize) -> bool {
    bytes.get(index) == Some(&b'-')
        && bytes.get(index + 1) == Some(&b'-')
        && bytes
            .get(index + 2)
            .is_none_or(|next| next.is_ascii_whitespace() || *next < 32)
}

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

/// The byte offset of char `index` (statements are mostly ASCII).
fn byte_offset(chars: &[char], index: usize) -> usize {
    chars[..index].iter().map(|c| c.len_utf8()).sum()
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
        } else if (c == '-' && dash_comment_at(statement.as_bytes(), byte_offset(&chars, index)))
            || c == '#'
        {
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

impl Cursor<'_> {
    /// One allowed table option: ENGINE (allowed engines only), [DEFAULT]
    /// CHARSET / CHARACTER SET / COLLATE, COMMENT, AUTO_INCREMENT, ROW_FORMAT.
    fn table_option(&mut self) -> Result<(), String> {
        self.word(&["DEFAULT"]);
        let option = match self.tokens.get(self.at) {
            Some(Token::Word(text))
                if TABLE_OPTIONS.contains(&text.to_ascii_uppercase().as_str()) =>
            {
                text.to_ascii_uppercase()
            }
            Some(other) => return Err(format!("table option {other:?} is not allowed")),
            None => return Err("table option 'end of statement' is not allowed".into()),
        };
        self.at += 1;
        if option == "CHARACTER" && !self.word(&["SET"]) {
            return Err("CHARACTER must be followed by SET".into());
        }
        self.punct('=');
        let value = self
            .tokens
            .get(self.at)
            .cloned()
            .ok_or("a table option needs a value")?;
        self.at += 1;
        if option == "ENGINE" {
            let allowed = matches!(&value, Token::Word(name) | Token::Quoted(name)
                if ALLOWED_ENGINES.contains(&name.to_ascii_uppercase().as_str()));
            if !allowed {
                return Err(format!(
                    "engine {value:?} is not allowed; use one of {ALLOWED_ENGINES:?}"
                ));
            }
        }
        Ok(())
    }

    /// Skip a parenthesised group whose opening `(` was just consumed.
    fn skip_group(&mut self) -> Result<(), String> {
        let mut depth = 1;
        while depth > 0 {
            let token = self.tokens.get(self.at).ok_or("unbalanced parentheses")?;
            self.at += 1;
            match token {
                Token::Punct('(') => depth += 1,
                Token::Punct(')') => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }

    /// A row statement names one module table only.
    fn dml_tail(&self) -> Result<(), String> {
        if self.tokens[self.at..]
            .iter()
            .any(|token| matches!(token, Token::Word(text) if text.eq_ignore_ascii_case("TABLE")))
        {
            return Err("a row statement may name one module table only".into());
        }
        Ok(())
    }
}

/// One ALTER TABLE clause: ADD/DROP/MODIFY/CHANGE/ALTER of columns, indexes,
/// keys and own foreign keys, or a table option. Nothing else.
fn alter_clause(clause: &[Token], prefix: &str) -> Result<(), String> {
    let head = match clause.first() {
        Some(Token::Word(text)) => text.to_ascii_uppercase(),
        other => return Err(format!("ALTER TABLE clause {other:?} is not allowed")),
    };
    if ["ADD", "DROP", "MODIFY", "CHANGE", "ALTER"].contains(&head.as_str()) {
        return Ok(());
    }
    if TABLE_OPTIONS.contains(&head.as_str()) || head == "DEFAULT" {
        let mut c = Cursor {
            tokens: clause,
            prefix,
            at: 0,
        };
        c.table_option()?;
        if !c.done() {
            return Err("unexpected text after a table option".into());
        }
        return Ok(());
    }
    Err(format!("ALTER TABLE clause '{head}' is not allowed"))
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
        c.skip_group()?;
        while !c.done() {
            c.punct(',');
            c.table_option()?;
        }
    } else if c.word(&["ALTER", "TABLE"]) {
        c.own_table()?;
        let mut clause = Vec::new();
        let mut depth = 0i32;
        for token in tokens[c.at..]
            .iter()
            .chain(std::iter::once(&Token::Punct(',')))
        {
            match token {
                Token::Punct('(') => depth += 1,
                Token::Punct(')') => depth -= 1,
                _ => {}
            }
            if *token == Token::Punct(',') && depth == 0 {
                if clause.is_empty() {
                    return Err("empty ALTER TABLE clause".into());
                }
                alter_clause(&clause, prefix)?;
                clause.clear();
            } else {
                clause.push(token.clone());
            }
        }
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
        c.dml_tail()?;
    } else if c.word(&["UPDATE"]) {
        c.own_table()?;
        if !c.word(&["SET"]) {
            return Err("UPDATE may change one module table only".into());
        }
        c.dml_tail()?;
    } else if c.word(&["DELETE", "FROM"]) {
        c.own_table()?;
        if !(c.done() || c.word(&["WHERE"]) || c.word(&["ORDER"]) || c.word(&["LIMIT"])) {
            return Err("DELETE may change one module table only".into());
        }
        c.dml_tail()?;
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
    if !c.word(&["IF", "EXISTS"]) {
        return Err("a purge DROP TABLE needs IF EXISTS, so a retried purge is safe".into());
    }
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

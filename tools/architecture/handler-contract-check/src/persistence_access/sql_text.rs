//! Pinned Rust string expressions and conservative MySQL advisory-lock classification.
//! Symbol resolution is supplied by the caller; this module owns no source graph or flow state.

use proc_macro2::{TokenStream, TokenTree};
use std::collections::BTreeSet;
use syn::parse::Parser;
use syn::{Expr, Item};

use super::{normalized_ident, normalized_tokens, path_names};

const ADVISORY_LOCK_FUNCTIONS: &[&str] = &[
    "GET_LOCK",
    "RELEASE_LOCK",
    "IS_USED_LOCK",
    "IS_FREE_LOCK",
    "RELEASE_ALL_LOCKS",
];

/// Drop the parts of `sql` the database never executes, keeping everything
/// else in upper case.
///
/// Inert text must not be able to name a function: an ordinary comment and a
/// single-quoted literal are removed, while the body of an executable
/// `/*!…*/` or `/*M!…*/` comment is kept because the server runs it. Double
/// quotes are left alone — in this grammar they delimit the Rust literal that
/// carries the statement, not a SQL string.
fn executable_sql_text(
    fingerprint: &str,
    backslash_escapes: bool,
    resolve_path: &dyn Fn(Vec<String>) -> Option<String>,
) -> String {
    // A fingerprint is Rust token text whose statements live inside string
    // literals. Read those out first: the double quotes around them belong to
    // Rust, while double quotes *inside* a statement are SQL — a string under
    // MySQL's default mode and a quoted identifier under `ANSI_QUOTES`, and
    // neither one calls a function.
    // Two literals of one expression are not concatenated at runtime unless
    // the expression says so — `"SELECT GET_" + name.as_str() + "LOCK(…)"`
    // never builds `GET_LOCK`. Separate them so no call can be fabricated
    // across whatever stands between them.
    let sql = match string_literal_values(fingerprint.parse().unwrap_or_default(), resolve_path)
        .as_slice()
    {
        [] => fingerprint.to_owned(),
        values => values.join("\0"),
    };
    let mut executable = String::with_capacity(sql.len());
    let mut index = 0;
    while index < sql.len() {
        let rest = &sql[index..];
        if let Some(body) = rest.strip_prefix("/*") {
            if let Some(payload) = executable_comment_payload(body) {
                index += rest.len() - payload.len();
                continue;
            }
            let end = body.find("*/").map_or(body.len(), |end| end + "*/".len());
            index += "/*".len() + end;
            executable.push(' ');
            continue;
        }
        // MySQL opens a comment on `--` only when whitespace or a control
        // character follows; `SELECT 1--1` is arithmetic, not a remark.
        let opens_line_comment = rest.strip_prefix("--").is_some_and(|tail| {
            tail.is_empty()
                || tail
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_whitespace() || character.is_control())
        }) || rest.starts_with('#');
        if opens_line_comment {
            index += rest.find('\n').map_or(rest.len(), |end| end + 1);
            executable.push(' ');
            continue;
        }
        if let Some(quote) = rest
            .chars()
            .next()
            .filter(|character| matches!(character, '\'' | '`' | '"'))
        {
            index += quoted_span_length(rest, quote, backslash_escapes);
            executable.push(' ');
            continue;
        }
        let character = rest.chars().next().expect("index is a character boundary");
        executable.extend(character.to_uppercase());
        index += character.len_utf8();
    }
    executable
}

/// How far a quoted SQL token extends, including its delimiters.
///
/// MySQL ends the token at the first unescaped delimiter: outside
/// `NO_BACKSLASH_ESCAPES` a backslash escapes the next character, and a doubled
/// delimiter stands for one literal delimiter. Stopping at the first raw match
/// would hand the rest of the statement back to the scanner as if it were
/// executable, and `SELECT 'it\'s', GET_LOCK('x', 0)` would lose its call.
fn quoted_span_length(rest: &str, quote: char, backslash_escapes: bool) -> usize {
    // A backtick identifier escapes its delimiter only by doubling it; a
    // backslash inside one is an ordinary character. Treating it as an escape
    // would swallow the closing backtick and read the statement after it as
    // data.
    let backslash_escapes = backslash_escapes && quote != '`';
    let mut characters = rest.char_indices().skip(1);
    while let Some((offset, character)) = characters.next() {
        if backslash_escapes && character == '\\' {
            characters.next();
            continue;
        }
        if character == quote {
            // A doubled delimiter is one literal delimiter, not the end.
            if rest[offset + character.len_utf8()..].starts_with(quote) {
                characters.next();
                continue;
            }
            return offset + character.len_utf8();
        }
    }
    rest.len()
}

/// Whether `executable` calls `function`: the name must stand as a whole token
/// and be followed by its argument list. A binding called `GET_LOCK_SQL` names
/// no call, and neither does prose that merely contains the word.
fn calls_sql_function(executable: &str, function: &str) -> bool {
    // MySQL's unquoted identifiers admit `$` and characters beyond ASCII, so a
    // routine named `éGET_LOCK` must not read as the built-in.
    let is_name_character = |character: char| {
        character.is_ascii_alphanumeric()
            || character == '_'
            || character == '$'
            || ('\u{80}'..='\u{ffff}').contains(&character)
    };
    // `CALL GET_LOCK(…)` invokes a stored routine and `app.GET_LOCK(…)` names a
    // schema one; neither is the built-in whose connection affinity this
    // classification stands for.
    let is_routine_context = |before: &str| {
        let trimmed = before.trim_end();
        trimmed.ends_with('.')
            || trimmed
                .rsplit(|character: char| !is_name_character(character))
                .find(|word| !word.is_empty())
                .is_some_and(|word| word == "CALL")
    };
    let mut search = executable;
    let mut consumed = 0;
    while let Some(position) = search.find(function) {
        let start = consumed + position;
        let end = start + function.len();
        let before = &executable[..start];
        let follows = executable[end..].trim_start();
        if !before.chars().next_back().is_some_and(is_name_character)
            && follows.starts_with('(')
            && !is_routine_context(before)
        {
            return true;
        }
        consumed = end;
        search = &executable[consumed..];
    }
    false
}

/// Whether the executable SQL carried by `fingerprint` takes a MySQL advisory
/// lock. Valid MySQL spells these functions in any case
/// (`SELECT get_lock(...)`), so the comparison is case-insensitive.
pub(super) fn sql_is_advisory_lock(
    fingerprint: &str,
    resolve_path: &dyn Fn(Vec<String>) -> Option<String>,
) -> bool {
    // The repository pins no `sql_mode`, and `NO_BACKSLASH_ESCAPES` decides
    // where a quoted token ends. Read the statement under both meanings and
    // keep the identity if either one calls the function: a ratchet may record
    // a lock the session does not take, but it must not miss one it does.
    [true, false].into_iter().any(|backslash_escapes| {
        let executable = executable_sql_text(fingerprint, backslash_escapes, resolve_path);
        ADVISORY_LOCK_FUNCTIONS
            .iter()
            .any(|function| calls_sql_function(&executable, function))
    })
}

/// The statements a token stream pins, one entry per statement.
///
/// Two literals of one expression are separate entries: they are not
/// concatenated at run time unless the expression says so, and
/// `"SELECT GET_" + name.as_str() + "LOCK(…)"` never builds a call. The
/// exception is `concat!`, which does join its arguments, so its literals form
/// a single entry.
/// The statement a source pins, when it pins exactly one.
fn string_literal_values(
    tokens: TokenStream,
    resolve_path: &dyn Fn(Vec<String>) -> Option<String>,
) -> Vec<String> {
    let mut values = Vec::new();
    collect_string_literal_values(tokens, resolve_path, &mut values);
    values
}

/// What a literal contributes to a compile-time string.
///
/// `concat!` renders integers, characters, and floats as well as strings, so
/// dropping them would both lose statement text and let neighbouring pieces
/// close over the gap: `concat!("SELECT GET", 1, "_LOCK(…)")` must read as
/// `SELECT GET1_LOCK(…)`, which calls nothing. Booleans arrive as identifiers
/// rather than literals and are rendered where the pieces are collected.
fn rendered_literal(literal: &proc_macro2::Literal) -> Option<String> {
    let text = literal.to_string();
    if let Ok(value) = syn::parse_str::<syn::LitStr>(&text) {
        return Some(value.value());
    }
    if let Ok(value) = syn::parse_str::<syn::LitChar>(&text) {
        return Some(value.value().to_string());
    }
    if let Ok(value) = syn::parse_str::<syn::LitInt>(&text) {
        return Some(value.base10_digits().to_owned());
    }
    if let Ok(value) = syn::parse_str::<syn::LitFloat>(&text) {
        return Some(value.base10_digits().to_owned());
    }
    // A byte string is not part of a compile-time `concat!` string; leaving it
    // unrendered keeps the pieces around it from closing over the gap.
    None
}

fn collect_string_literal_values(
    tokens: TokenStream,
    resolve_path: &dyn Fn(Vec<String>) -> Option<String>,
    values: &mut Vec<String>,
) {
    let trees = tokens.into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while index < trees.len() {
        if let Some((macro_name, group)) = compile_time_string_macro(&trees, index, resolve_path) {
            if macro_name == "stringify" {
                // `stringify!` renders its input tokens, not the literals in
                // them.
                values.push(group.stream().to_string());
            } else {
                let mut joined = Vec::new();
                collect_concat_pieces(group.stream(), resolve_path, &mut joined);
                if !joined.is_empty() {
                    values.push(joined.concat());
                }
            }
            index += 3;
            continue;
        }
        match &trees[index] {
            TokenTree::Literal(literal) => {
                if let Some(value) = rendered_literal(literal) {
                    values.push(value);
                }
            }
            TokenTree::Group(group) => {
                collect_string_literal_values(group.stream(), resolve_path, values)
            }
            _ => {}
        }
        index += 1;
    }
}

/// The name and argument group of a `concat!`/`stringify!` invocation starting
/// at `index`, both of which build a string at compile time.
/// Whether a resolved path names the standard `concat!`/`stringify!` rather
/// than somebody's macro of the same leaf name.
///
/// An unqualified builtin resolves to the enclosing module's path plus its own
/// name, because that is what resolution does with a name it cannot find — that
/// is still the prelude macro. A path leading anywhere else is a namesake.
/// Whether a resolved prefix is the enclosing module itself.
///
/// Resolution may or may not carry a leading `crate`, and the recorded module
/// path may or may not either, so both are compared without it.
fn prefix_is_enclosing_module(prefix: &[String], module_path: &[String]) -> bool {
    let strip = |names: &[String]| -> Vec<String> {
        names
            .iter()
            .skip_while(|name| *name == "crate")
            .cloned()
            .collect()
    };
    strip(prefix) == strip(module_path)
}

/// Whether `path` names the standard `String::from`.
///
/// A type of one's own called `String` may return whatever it likes, so the
/// conversion only preserves a pinned source when it is the real one.
pub(super) fn is_standard_string_conversion(
    path: &syn::Path,
    resolve_path: &dyn Fn(Vec<String>) -> Vec<String>,
    local_types: &BTreeSet<String>,
) -> bool {
    let names = path_names(path);
    if names.last().map(String::as_str) != Some("from") {
        return false;
    }
    let owner = names[..names.len() - 1].to_vec();
    if owner.last().map(String::as_str) != Some("String") {
        return false;
    }
    // Spelled out — `custom::String::from`, `crate::custom::String::from` — only
    // the standard type qualifies, and it has to resolve into `std`/`alloc`.
    if owner.len() > 1 {
        return matches!(
            resolve_path(owner).as_slice(),
            [root, .., name] if matches!(root.as_str(), "std" | "alloc") && name == "String"
        );
    }
    // Written bare it is the prelude type, unless this scope declares one of
    // its own or imports somebody else's under that name. Both are recorded as
    // shadows, because resolution alone reports the enclosing module's path for
    // a prelude name and cannot tell the two apart.
    !local_types.contains("String")
}

fn is_standard_string_macro(
    resolved: &[String],
    macro_name: &str,
    module_path: &[String],
    local_definitions: &BTreeSet<String>,
) -> bool {
    match resolved {
        [name] => name == macro_name && !local_definitions.contains(name),
        [root, name] if matches!(root.as_str(), "std" | "core") => name == macro_name,
        // A module that defines its own `concat!` shadows the prelude one, and
        // both resolve to this shape.
        _ => resolved.split_last().is_some_and(|(name, prefix)| {
            name == macro_name
                && prefix_is_enclosing_module(prefix, module_path)
                && !local_definitions.contains(name)
        }),
    }
}

/// The names a module declares with `macro_rules!` before `index`.
///
/// A `macro_rules!` scope starts at its declaration, so an invocation above it
/// is still the prelude macro. Marking it opaque is not the safe direction for
/// a ratchet that exists to notice changes: an opaque source lets its literal
/// change without moving a row.
pub(super) fn macro_shadows_before(items: &[Item], index: usize) -> BTreeSet<String> {
    items[..index]
        .iter()
        .filter_map(|item| match item {
            Item::Macro(item_macro) => item_macro.ident.as_ref().map(normalized_ident),
            _ => None,
        })
        .collect()
}

/// The standard compile-time string macro `written` names, if it names one.
pub(super) fn standard_string_macro_of(
    written: Vec<String>,
    resolve: &dyn Fn(Vec<String>) -> Vec<String>,
    module_path: &[String],
    local_definitions: &BTreeSet<String>,
) -> Option<String> {
    let resolved = resolve(written);
    ["concat", "stringify"]
        .into_iter()
        .find(|candidate| {
            is_standard_string_macro(&resolved, candidate, module_path, local_definitions)
        })
        .map(str::to_owned)
}

fn compile_time_string_macro(
    trees: &[TokenTree],
    index: usize,
    resolve_path: &dyn Fn(Vec<String>) -> Option<String>,
) -> Option<(String, proc_macro2::Group)> {
    let TokenTree::Ident(ident) = trees.get(index)? else {
        return None;
    };
    // A leaf name does not identify a macro: `other::concat!` is somebody's own
    // macro and `strings::c!` may re-export the standard one. Resolve the whole
    // path and apply compile-time-string semantics only to what it names.
    let mut written = vec![normalized_ident(ident)];
    let mut segment = index;
    while segment >= 2
        && matches!(trees.get(segment - 1), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
        && matches!(trees.get(segment - 2), Some(TokenTree::Punct(punct)) if punct.as_char() == ':')
        && let Some(TokenTree::Ident(owner)) = trees.get(segment - 3)
    {
        written.insert(0, normalized_ident(owner));
        segment -= 3;
    }
    let name = resolve_path(written)?;
    match (trees.get(index + 1), trees.get(index + 2)) {
        (Some(TokenTree::Punct(punct)), Some(TokenTree::Group(group)))
            if punct.as_char() == '!' =>
        {
            Some((name, group.clone()))
        }
        _ => None,
    }
}

/// The pieces a `concat!` renders, in order.
///
/// Every argument contributes: `true` and `false` reach a token stream as
/// identifiers rather than literals, and dropping one would let the strings
/// around it close over the gap — `concat!("SELECT GET", true, "_LOCK(…)")`
/// expands to `SELECT GETtrue_LOCK(…)`, which calls nothing.
fn collect_concat_pieces(
    tokens: TokenStream,
    resolve_path: &dyn Fn(Vec<String>) -> Option<String>,
    pieces: &mut Vec<String>,
) {
    let trees = tokens.into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while index < trees.len() {
        if let Some((macro_name, group)) = compile_time_string_macro(&trees, index, resolve_path) {
            if macro_name == "stringify" {
                pieces.push(group.stream().to_string());
            } else {
                collect_concat_pieces(group.stream(), resolve_path, pieces);
            }
            index += 3;
            continue;
        }
        match &trees[index] {
            TokenTree::Literal(literal) => {
                if let Some(value) = rendered_literal(literal) {
                    pieces.push(value);
                }
            }
            TokenTree::Ident(ident) => {
                let name = normalized_ident(ident);
                if matches!(name.as_str(), "true" | "false") {
                    pieces.push(name);
                }
            }
            TokenTree::Group(group) => collect_concat_pieces(group.stream(), resolve_path, pieces),
            _ => {}
        }
        index += 1;
    }
}

/// The payload of a MySQL (`/*!`) or MariaDB (`/*M!`) executable comment, whose
/// body the server runs as part of the statement. The optional version prefix
/// (`/*!50000 SELECT …`) is not part of the SQL.
///
/// `body` starts immediately after the opening `/*`.
fn executable_comment_payload(body: &str) -> Option<&str> {
    let payload = body
        .strip_prefix('!')
        .or_else(|| body.strip_prefix("M!"))
        .or_else(|| body.strip_prefix("m!"))?;
    Some(payload.trim_start_matches(|character: char| character.is_ascii_digit()))
}

/// The argument of a SQLx query macro that carries the statement.
///
/// `query!` and `query_scalar!` take it first; the `query_as!` family takes the
/// output type first and the statement second. Anything else in the invocation
/// is a bound value, and a value is not executed as SQL.
///
/// `name` must be the canonical SQLx macro name: an import alias
/// (`use sqlx::query_as as q`) hides which position carries the statement.
pub(super) fn query_macro_statement(name: &str, tokens: &TokenStream) -> Option<String> {
    let arguments = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
        .parse2(tokens.clone())
        .ok()?;
    let position = usize::from(name.ends_with("_as") || name.ends_with("_as_unchecked"));
    arguments.iter().nth(position).map(normalized_tokens)
}

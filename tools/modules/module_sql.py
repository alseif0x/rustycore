"""Static guard for module SQL (#1444 H6).

A module may touch only the `characters` and `world` databases, and there only
tables whose name starts with its own prefix (`mod_<id with . as _>_`). Its
migrations may create, alter and index those tables and change their rows; its
purge scripts may only drop them. Anything else, including any statement that
reads another table, is refused before it reaches a database.

The same rules are implemented in Rust (`wow_database::migration::module_sql`)
for `rustycore-db module apply|purge`. Both read the shared vectors in
`tools/modules/sql_guard_vectors.json`, so the two guards cannot drift.
"""
from __future__ import annotations

MODULE_DATABASES = ("characters", "world")
# Words that read or reach another table, or rename one away from the prefix.
BANNED_WORDS = {"SELECT", "JOIN", "RENAME", "UNION", "OUTFILE", "DUMPFILE", "INFILE"}
_IDENT_CHARS = set("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_$")


def table_prefix(module_id: str) -> str:
    return "mod_" + module_id.replace(".", "_") + "_"


def split_statements(content: str) -> list[str]:
    """Mirror `migration::split_sql`: split on `;` outside comments and quotes."""
    statements: list[str] = []
    start = index = 0
    n = len(content)
    while index < n:
        c = content[index]
        if c == "-" and index + 1 < n and content[index + 1] == "-" or c == "#":
            while index < n and content[index] != "\n":
                index += 1
        elif c == "/" and index + 1 < n and content[index + 1] == "*":
            index += 2
            while index + 1 < n and not (content[index] == "*" and content[index + 1] == "/"):
                index += 1
            index = min(index + 2, n)
        elif c in "'\"":
            index += 1
            while index < n:
                if content[index] == "\\":
                    index = min(index + 2, n)
                elif content[index] == c:
                    index += 1
                    break
                else:
                    index += 1
        elif c == ";":
            statement = content[start:index].strip()
            if statement:
                statements.append(statement)
            index += 1
            start = index
        else:
            index += 1
    tail = content[start:].strip()
    if tail:
        statements.append(tail)
    return statements


def _tokens(statement: str) -> list[tuple[str, str]]:
    """Tokens with comments removed and string literals emptied.

    Kinds: `w` word, `q` backtick identifier (content), `s` string, `p` one
    punctuation character. An executable comment (`/*!` or `/*M!`) raises,
    because MariaDB runs what it contains.
    """
    tokens: list[tuple[str, str]] = []
    index, n = 0, len(statement)
    while index < n:
        c = statement[index]
        if c.isspace():
            index += 1
        elif c == "-" and statement.startswith("--", index) or c == "#":
            while index < n and statement[index] != "\n":
                index += 1
        elif c == "/" and statement.startswith("/*", index):
            if statement.startswith("/*!", index) or statement.startswith("/*M!", index):
                raise ValueError("executable comment /*! ... */")
            end = statement.find("*/", index + 2)
            index = n if end < 0 else end + 2
        elif c in "'\"":
            index += 1
            while index < n:
                if statement[index] == "\\":
                    index += 2
                elif statement[index] == c:
                    if index + 1 < n and statement[index + 1] == c:
                        index += 2
                        continue
                    index += 1
                    break
                else:
                    index += 1
            tokens.append(("s", ""))
        elif c == "`":
            end = statement.find("`", index + 1)
            if end < 0:
                raise ValueError("unterminated identifier")
            tokens.append(("q", statement[index + 1 : end]))
            index = end + 1
        elif c in _IDENT_CHARS:
            start = index
            while index < n and statement[index] in _IDENT_CHARS:
                index += 1
            tokens.append(("w", statement[start:index]))
        else:
            tokens.append(("p", c))
            index += 1
    return tokens


class _Cursor:
    def __init__(self, tokens: list[tuple[str, str]], prefix: str):
        self.tokens, self.prefix, self.at = tokens, prefix, 0

    def word(self, *words: str) -> bool:
        """Consume the keywords if they come next."""
        at = self.at
        for word in words:
            if at >= len(self.tokens) or self.tokens[at][0] != "w" or self.tokens[at][1].upper() != word:
                return False
            at += 1
        self.at = at
        return True

    def punct(self, char: str) -> bool:
        if self.at < len(self.tokens) and self.tokens[self.at] == ("p", char):
            self.at += 1
            return True
        return False

    def ident(self) -> str:
        """Consume an unqualified identifier; refuse a qualified one."""
        if self.at >= len(self.tokens) or self.tokens[self.at][0] not in "wq":
            raise ValueError("expected a table or index name")
        name = self.tokens[self.at][1]
        self.at += 1
        if self.at < len(self.tokens) and self.tokens[self.at] == ("p", "."):
            raise ValueError(f"qualified name {name}.…: a module names no database")
        return name

    def own_table(self) -> str:
        name = self.ident()
        rest = name[len(self.prefix) :]
        if not name.startswith(self.prefix) or not rest or not all(
            ch in "abcdefghijklmnopqrstuvwxyz0123456789_" for ch in rest
        ):
            raise ValueError(f"table {name!r} is not the module's own (prefix {self.prefix!r})")
        return name

    def done(self) -> bool:
        return self.at >= len(self.tokens)


def _check_common(tokens: list[tuple[str, str]], prefix: str) -> None:
    for index, (kind, text) in enumerate(tokens):
        if kind == "w" and text.upper() in BANNED_WORDS:
            raise ValueError(f"{text.upper()} is not allowed in module SQL")
        if kind == "w" and text.upper() == "REFERENCES":
            cursor = _Cursor(tokens, prefix)
            cursor.at = index + 1
            cursor.own_table()


def check_migration_statement(statement: str, prefix: str) -> None:
    """Raise `ValueError` unless the statement only changes the module's tables."""
    tokens = _tokens(statement)
    _check_common(tokens, prefix)
    c = _Cursor(tokens, prefix)
    if c.word("CREATE", "TABLE"):
        c.word("IF", "NOT", "EXISTS")
        c.own_table()
        if not c.punct("("):
            raise ValueError("CREATE TABLE must define its columns")
    elif c.word("ALTER", "TABLE"):
        c.own_table()
    elif c.word("CREATE", "UNIQUE", "INDEX") or c.word("CREATE", "INDEX") or c.word("DROP", "INDEX"):
        c.ident()
        if not c.word("ON"):
            raise ValueError("an index statement must name its table with ON")
        c.own_table()
    elif c.word("INSERT", "IGNORE", "INTO") or c.word("INSERT", "INTO") or c.word("REPLACE", "INTO"):
        c.own_table()
    elif c.word("UPDATE"):
        c.own_table()
        if not c.word("SET"):
            raise ValueError("UPDATE may change one module table only")
    elif c.word("DELETE", "FROM"):
        c.own_table()
        if not (c.done() or c.word("WHERE") or c.word("ORDER") or c.word("LIMIT")):
            raise ValueError("DELETE may change one module table only")
    else:
        raise ValueError("only CREATE/ALTER TABLE, CREATE/DROP INDEX, INSERT, REPLACE, UPDATE and DELETE on the module's tables are allowed")


def check_purge_statement(statement: str, prefix: str) -> None:
    """Raise `ValueError` unless the statement only drops module tables."""
    tokens = _tokens(statement)
    _check_common(tokens, prefix)
    c = _Cursor(tokens, prefix)
    if not c.word("DROP", "TABLE"):
        raise ValueError("a purge script may only DROP TABLE the module's tables")
    c.word("IF", "EXISTS")
    c.own_table()
    while c.punct(","):
        c.own_table()
    if not c.done():
        raise ValueError("unexpected text after DROP TABLE")


def check_script(content: str, prefix: str, purge: bool) -> list[str]:
    """Every problem in one script, as `statement N: reason`; empty when clean."""
    problems = []
    check = check_purge_statement if purge else check_migration_statement
    for number, statement in enumerate(split_statements(content), start=1):
        try:
            check(statement, prefix)
        except ValueError as error:
            problems.append(f"statement {number}: {error}")
    return problems

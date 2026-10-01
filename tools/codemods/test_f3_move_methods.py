#!/usr/bin/env python3
"""Self-test for f3_move_methods.py (#1241 F3) on a synthetic mini-tree. Standard library only.

Covers plan/apply idempotence, the thunk rule (resident caller, pub API, stale thunk removal),
shim placement and visibility, and the precondition abort that leaves the tree untouched.
"""
from __future__ import annotations

import hashlib
import io
import pathlib
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import f3_move_methods as F  # noqa: E402

FILES = {
    "crates/wow-world/src/lib.rs": "pub mod session;\n",
    "crates/wow-world/src/session/mod.rs": (
        "mod catalogs;\nmod connection;\nmod state;\npub use state::WorldSession;\n"),
    "crates/wow-world/src/session/state.rs": (
        "mod session_core;\npub(in crate::session) use session_core::SessionCore;\n"
        "mod catalogs;\npub(in crate::session) use catalogs::SessionCatalogs;\n\n"
        "pub struct WorldSession {\n"
        "    pub(crate) core: SessionCore,\n"
        "    pub(crate) catalogs: SessionCatalogs,\n"
        "}\n"),
    "crates/wow-world/src/session/state/session_core.rs": (
        "pub(crate) struct SessionCore {\n    pub(crate) account_id: u32,\n}\n"),
    "crates/wow-world/src/session/state/catalogs.rs": (
        "pub(crate) struct SessionCatalogs {\n    pub(crate) foo_store: Option<u32>,\n"
        "    pub(crate) bar: u32,\n}\n"),
    "crates/wow-world/src/session/catalogs/mod.rs": "mod ops;\n",
    "crates/wow-world/src/session/catalogs/ops.rs": (
        "use super::*;\n\n"
        "impl WorldSession {\n"
        "    /// Read the foo store.\n"
        "    pub(crate) fn foo_store(&self) -> Option<u32> {\n"
        "        self.catalogs.foo_store\n"
        "    }\n"
        "    pub fn set_foo_store(&mut self, value: u32) {\n"
        "        self.catalogs.foo_store = Some(value);\n"
        "    }\n"
        "    fn only_tests(&self) -> u32 {\n"
        "        self.catalogs.bar + 1\n"
        "    }\n"
        "    fn never_called(&self) -> u32 {\n"
        "        self.catalogs.bar\n"
        "    }\n"
        "    pub(crate) fn uses_whole(&self) -> u32 {\n"
        "        helper(self) + self.catalogs.bar\n"
        "    }\n"
        "}\n"),
    "crates/wow-world/src/session/connection.rs": (
        "use super::*;\n\n"
        "impl WorldSession {\n"
        "    pub(crate) fn account_plus_foo(&self) -> u32 {\n"
        "        self.core.account_id + self.foo_store().unwrap_or(0)\n"
        "    }\n"
        "}\n"),
    "crates/wow-world/unit_tests/catalogs_tests.rs": (
        "fn t(session: &mut WorldSession) -> u32 {\n    session.set_foo_store(1);\n"
        "    session.only_tests()\n}\n"),
}


def digest(root: pathlib.Path) -> str:
    h = hashlib.sha256()
    for p in sorted(root.rglob("*.rs")):
        h.update(p.relative_to(root).as_posix().encode() + p.read_bytes())
    return h.hexdigest()


def run(*argv) -> tuple[int, str]:
    out = io.StringIO()
    with redirect_stdout(out), redirect_stderr(out):
        rc = F.main(list(argv))
    return rc, out.getvalue()


class F3MoveMethodsTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name)
        for rel, text in FILES.items():
            path = self.root / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        self.ops = self.root / "crates/wow-world/src/session/catalogs/ops.rs"

    def tearDown(self):
        self.tmp.cleanup()

    def plan(self):
        return F.plan(self.root, {"catalogs"}, {"P", "C-hub"})

    def apply(self, *extra):
        return run("apply", "--group", "catalogs", "--root", str(self.root), "--text-only", *extra)

    def test_plan_thunk_rule_and_preconditions(self):
        P = self.plan()
        rows = {r["name"]: r for r in P["rows"]}
        self.assertEqual(set(rows), {"foo_store", "set_foo_store", "only_tests"})
        self.assertIn("resident", rows["foo_store"]["thunk"])
        self.assertEqual(rows["set_foo_store"]["thunk"], "pub API, no production caller")
        self.assertEqual((rows["only_tests"]["thunk"], bool(rows["only_tests"]["shim"])), ("", True))
        self.assertTrue(P["blocked"]["uses_whole"].startswith("whole-self"))
        self.assertTrue(P["blocked"]["never_called"].startswith("no callers anywhere"))

    def test_precondition_aborts_before_any_write(self):
        before = digest(self.root)
        rc, out = self.apply()
        self.assertEqual(rc, 1)
        self.assertIn("nothing written", out)
        self.assertEqual(digest(self.root), before)

    def test_apply_moves_thunks_shims_and_is_idempotent(self):
        rc, out = self.apply("--demote-blocked")
        self.assertEqual(rc, 0, out)
        text = self.ops.read_text()
        moved = text[text.index("impl crate::session::state::SessionCatalogs {"):]
        self.assertIn("/// Read the foo store.", moved)              # cut byte-for-byte, docs travel
        self.assertIn("self.foo_store\n", moved)                     # self.catalogs. -> self.
        self.assertIn("self.foo_store = Some(value);", moved)
        head = text[:text.index("impl crate::session::state::SessionCatalogs {")]
        self.assertIn("self.catalogs.foo_store()", head)             # resident-caller thunk
        self.assertIn("self.catalogs.set_foo_store(value)", head)    # pub API thunk
        self.assertNotIn("fn only_tests", head)                      # test-only caller: no thunk
        self.assertIn("helper(self)", head)                          # demoted fn stays
        self.assertIn("fn never_called", head)                       # uncalled fn stays
        shim = self.root / "crates/wow-world/unit_tests/session/catalogs/ops/f3_shims.rs"
        self.assertIn("pub(in crate::session::catalogs::ops) fn only_tests(&self) -> u32", shim.read_text())
        self.assertIn('#[path = "../../../unit_tests/session/catalogs/ops/f3_shims.rs"]\nmod f3_shims;', text)
        state = (self.root / "crates/wow-world/src/session/state.rs").read_text()
        self.assertIn("pub(in crate::session) use catalogs::SessionCatalogs;", state)  # no widening needed
        after = digest(self.root)
        rc, out = self.apply("--demote-blocked")
        self.assertEqual((rc, "already applied (no-op)" in out), (0, True), out)
        self.assertEqual(digest(self.root), after)
        self.assertEqual(self.plan()["rows"], [])

    def test_stale_thunk_is_removed_when_last_caller_moves(self):
        self.assertEqual(self.apply("--demote-blocked")[0], 0)
        conn = self.root / "crates/wow-world/src/session/connection.rs"
        conn.write_text(conn.read_text().replace(" + self.foo_store().unwrap_or(0)", ""))
        self.assertEqual(self.plan()["stale"], ["foo_store"])
        rc, out = self.apply("--demote-blocked")
        self.assertEqual(rc, 0, out)
        head = self.ops.read_text().split("impl crate::session::state::SessionCatalogs {")[0]
        self.assertNotIn("fn foo_store", head)
        self.assertIn("fn set_foo_store", head)                      # pub API thunk is never stale
        self.assertEqual(self.plan()["stale"], [])


if __name__ == "__main__":
    unittest.main()

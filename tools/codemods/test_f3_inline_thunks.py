"""Self-test for f3_inline_thunks.py (#1241 F3-I) on a synthetic tree moved by the f3 codemod."""
import io, pathlib, sys, unittest
from contextlib import redirect_stderr, redirect_stdout

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import f3_inline_thunks as I  # noqa: E402
import test_f3_move_methods as T  # noqa: E402


def run(*argv):
    out = io.StringIO()
    with redirect_stdout(out), redirect_stderr(out):
        rc = I.main(list(argv))
    return rc, out.getvalue()


class InlineThunksTest(unittest.TestCase):
    setUp, tearDown, write, cx_tree = (T.F3MoveMethodsTest.setUp, T.F3MoveMethodsTest.tearDown,
                                       T.F3MoveMethodsTest.write, T.F3MoveMethodsTest.cx_tree)

    def moved_tree(self, user="", test_file=None):
        """cx_tree plus a `bump(x)` Cx fn and the given WorldSession callers, moved with the Cx kind."""
        self.cx_tree()
        npc = self.root / "crates/wow-world/src/session/npc_interaction.rs"
        npc.write_text(npc.read_text().replace("impl WorldSession {\n", (
            "impl WorldSession {\n    pub(crate) fn bump(&mut self, x: u32) {\n"
            "        self.interaction.source = x + self.loot.gold;\n    }\n"), 1))
        conn = self.root / "crates/wow-world/src/session/connection.rs"
        conn.write_text(conn.read_text().replace("    pub fn cx_user(&mut self) -> u32 {\n",
                                                 user + "    pub fn cx_user(&mut self) -> u32 {\n"))
        if test_file:
            self.write("crates/wow-world/unit_tests/session/inline_tests.rs", test_file)
        out = io.StringIO()
        with redirect_stdout(out), redirect_stderr(out):
            rc = I.F.main(["apply", "--group", "interaction,loot", "--classes", "P,C-hub,Cx", "--cx-siblings",
                           "interaction=loot", "--root", str(self.root), "--text-only", "--demote-blocked"])
        self.assertEqual(rc, 0, out.getvalue())

    def rows(self, kinds=None):
        return {r["name"]: r for r in I.plan(self.root, kinds)["rows"]}

    def test_kinds_shapes_and_hoisting(self):
        self.moved_tree("    pub fn inline_user(&mut self) -> u32 {\n        self.bump(self.core.account_id);\n"
                        "        let b = self.loot_plus(1);\n        b + self.loot_plus(2) + self.wide()\n    }\n")
        rows = self.rows()
        self.assertEqual((rows["mix"]["kind"], rows["peek"]["kind"], rows["loot_plus"]["kind"]),
                         ("cx", "cx", "state-hub"))
        self.assertEqual([s["new"] for s in rows["mix"]["sites"]], ["crate::session::cx_interaction(self).mix()"])
        self.assertEqual({s["new"] for s in rows["peek"]["sites"]}, {"crate::session::cx_interaction_ref(self).peek()"})
        self.assertEqual([(s["new"], s["hoist"]) for s in rows["bump"]["sites"]],     # the argument re-reads self
                         [("{ let a0 = self.core.account_id; crate::session::cx_interaction(self).bump(a0) }", True)])
        loot = rows["loot_plus"]                                         # split form: statement level only
        self.assertEqual([s["new"] for s in loot["sites"]],
                         ["{ let (s, h) = crate::session::split_loot_ref(self); s.loot_plus(h, 1) }"])
        self.assertEqual([r for *_x, r in loot["skipped"]], ["split form nested beside another receiver use"])
        self.assertEqual(loot["status"], "partial")
        self.assertEqual(self.rows({"state", "cx"})["loot_plus"]["status"], "kind not selected")
        I.SPLIT_MAX = 1                                                  # more than SPLIT_MAX sites: left for F5
        try:
            self.assertEqual(self.rows()["loot_plus"]["status"], "split kind with 2 call sites (F5)")
        finally:
            I.SPLIT_MAX = 2

    def test_unit_test_receivers_and_nesting(self):
        self.moved_tree(
            "    pub fn nest_user(&mut self, session: &mut WorldSession) {\n"
            "        self.bump(self.peek());\n        session.mix();\n    }\n",
            test_file="fn t() {\n    let mut session = make();\n    session.mix();\n"
                      "    let other = make();\n    other.mix();\n    let r = &session;\n    r.peek();\n}\n")
        rows = self.rows()
        self.assertEqual(sorted((s["file"], s["new"]) for s in rows["mix"]["sites"]), [
            ("src/session/connection.rs", "crate::session::cx_interaction(self).mix()"),
            ("src/session/connection.rs", "crate::session::cx_interaction(session).mix()"),   # a `&mut` binding
            ("unit_tests/session/inline_tests.rs", "crate::session::cx_interaction(&mut session).mix()")])
        bump = [s for s in rows["bump"]["sites"] if "peek" in s["old"]]            # `other`/`r`: not sessions
        self.assertEqual([(s["new"], s["nested"]) for s in bump], [(
            "{ let a0 = crate::session::cx_interaction_ref(self).peek(); crate::session::cx_interaction(self).bump(a0) }",
            ["peek"])])                                                             # inner site travels with it

    def test_source_text_pins_and_test_kinds(self):
        self.moved_tree(test_file='const SRC: &str = include_str!("../../src/session/connection.rs");\n'
                                  'fn t() { assert!(SRC.contains("self.mix")); }\n'
                                  'fn u() {\n    let mut session = make();\n    session.peek();\n}\n')
        rows = self.rows()                                               # a paren-less `self.m` pin counts
        self.assertEqual((rows["mix"]["status"], rows["mix"]["sites"]), ("source-text pinned", []))
        self.assertTrue(any(s["test"] for s in rows["peek"]["sites"]))
        rows = {r["name"]: r for r in I.plan(self.root, None, {"state"})["rows"]}
        self.assertFalse(any(s["test"] for s in rows["peek"]["sites"]))  # builder test sites keep a shim

    def test_only_verbatim_forwarding_thunks_inline(self):
        self.moved_tree("    pub fn bump_user(&mut self) {\n        self.bump(1);\n    }\n")
        npc = self.root / "crates/wow-world/src/session/npc_interaction.rs"
        text = npc.read_text()
        self.assertIn("crate::session::cx_interaction(self).bump(x)", text)
        npc.write_text(text.replace("crate::session::cx_interaction(self).bump(x)",
                                    "crate::session::cx_interaction(self).bump(x + 1)"))
        self.assertEqual(self.rows()["bump"]["status"], "body is not a delegation thunk")

    def test_block_replacements_are_parenthesized_where_rust_needs_it(self):
        form = dict(kind="cx", shape="single", recv="crate::session::cx_g(self)", m="m", shared=False)
        text = "fn f(&mut self) {\n    let Some(x) =\n        self.m(self.y)\n    else {\n        return;\n    };\n}\n"
        rstart = text.index("self.m")
        site, why = I.plan_site(form, text, text, rstart, text.index("(", rstart))
        self.assertEqual((site["new"], why), ("({ let a0 = self.y; crate::session::cx_g(self).m(a0) })", None))
        self.assertEqual(I.rewrite(form, "self", "", ["x", " self.f(1)", " 2"], False, [False, True, False]),
                         "{ let a1 = self.f(1); crate::session::cx_g(self).m(x, a1, 2) }")  # plain args stay put
        self.assertTrue(I.TRIVIAL.fullmatch(" &mut item.guid ") and not I.TRIVIAL.fullmatch("f(x)"))

    def test_split_blocks_only_where_they_read_well(self):
        ok = ["    self.m(1);\n", "    let x = self.m(1);\n", "    let Some(x) = self.m(1) else {", "    x = self.m(1);\n",
              "    match k { 1 => self.m(1), _ => 0 }", "    v.iter().map(|x| self.m(x))", "    {\n    self.m(1)\n}"]
        dense = ["    if self.m(1) {", "    a + self.m(1);", "    f(self.m(1));", "    let x = self.m(1)?;",
                 "    let y = x == self.m(1);"]
        for case in ok + dense:
            text = "fn f() {\n" + case
            start = text.index("self.m")
            end = text.index(")", start) + 1
            self.assertEqual(I.block_context_ok(text, start, end), case in ok, case)
        form = dict(kind="state-hub", shape="split", split="split_g_ref", m="m", shared=True)
        text = "fn f(&self) -> u8 {\n    self.m(1)\n}\n"                      # a block's whole tail: spliced
        start = text.index("self.m")
        self.assertEqual(I.plan_site(form, text, text, start, text.index("(", start))[0]["new"],
                         "let (s, h) = crate::session::split_g_ref(self); s.m(h, 1)")
        text = "fn f(&self) {\n    self.m(h);\n}\n"
        start = text.index("self.m")
        self.assertEqual(I.plan_site(form, text, text, start, text.index("(", start)),
                         (None, "argument names collide with the split bindings"))

    def test_apply_retires_inlined_thunks_and_reverts(self):
        self.moved_tree()
        P = I.plan(self.root)
        ledger = I.apply_sites(self.root, [r for r in P["rows"] if r["sites"]])
        crate = self.root / "crates/wow-world"
        for site in ledger:
            self.assertEqual((crate / site["file"]).read_text()[site["start"]:site["end"]], site["new"])
        I.revert(self.root, ledger, list(ledger))                       # the compiler loop's revert path
        self.assertEqual(ledger, [])
        for row in P["rows"]:
            for site in row["sites"]:
                self.assertIn(site["old"], (crate / site["file"]).read_text())
        rc, out = run("apply", "--root", str(self.root), "--text-only")
        self.assertEqual(rc, 0, out)
        npc = (crate / "src/session/npc_interaction.rs").read_text()
        conn = (crate / "src/session/connection.rs").read_text()
        self.assertIn("crate::session::cx_interaction(self).mix();", conn)
        self.assertNotIn("pub(crate) fn mix(&mut self)", npc.split("impl crate::session::InteractionCx")[0])
        self.assertIn("impl crate::session::InteractionCx<'_> {", npc)          # the moved fn stays
        rc, out = run("apply", "--root", str(self.root), "--text-only")         # a second pass is a no-op
        self.assertIn("inlined 0 sites", out)


if __name__ == "__main__":
    unittest.main()

"""#1241 F4a-P2: the f3 codemods recognise and emit the `test-fixtures` gate as well as `cfg(test)`."""
import pathlib, sys, unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import f3_codemod_lib as L  # noqa: E402
import f3_move_methods as F  # noqa: E402
import test_f3_move_methods as T  # noqa: E402

GATE = '#[cfg(any(test, feature = "test-fixtures"))]'


class F3FixturesGateTest(unittest.TestCase):
    def test_strip_blanks_both_forms_raw_and_blanked(self):
        W = F.lib(F.REPO)
        for attr in ("#[cfg(test)]", GATE):
            body = f"{{\n    {attr}\n    self.fixtures.x += 1;\n    self.core.y += 1;\n}}"
            for text in (body, W.blank_noncode(body)):
                out = L.strip_cfg_test(text)
                self.assertNotIn("fixtures", out)
                self.assertIn("self.core.y", out)
                self.assertEqual(len(out), len(text))
        other = '{\n    #[cfg(any(test, feature = "other"))]\n    self.fixtures.x += 1;\n}'
        self.assertIn("fixtures", L.strip_cfg_test(other))                  # only the exact gate

    def test_predicates_match_both_forms_and_nothing_else(self):
        for good in ("cfg(test)", 'cfg(any(test, feature = "test-fixtures"))', 'cfg(any(test,feature="test-fixtures"))',
                     'cfg(any(test, feature = "             "))'):
            self.assertTrue(L.TEST_CFG.fullmatch(good), good)
        for bad in ("cfg(not(test))", 'cfg(any(test, feature = "other"))', "cfg(unix)"):
            self.assertFalse(L.TEST_CFG.fullmatch(bad), bad)
        self.assertTrue(L.CX_IMPL.match(f"{GATE}\nimpl crate::session::PetsCx<'_> {{").group(1))
        self.assertTrue(L.CX_IMPL.match("#[cfg(test)]\nimpl crate::session::PetsCx<'_> {").group(1))

    def test_templates_emit_the_gate_for_the_fixtures_hub_member(self):
        for text in (L.HUB_RS, L.SPLIT_FN.format(g="loot", t="LootState"),
                     L.SPLIT_MUT_FN.format(g="loot", t="LootState")):
            self.assertNotIn("#[cfg(test)] fixtures", text)
            self.assertNotIn("#[cfg(test)]\n    pub(crate) fixtures", text)
            self.assertIn(f"{GATE} fixtures", text) if "split_" in text else self.assertIn(GATE, text)
        cx = L.add_cx_items("", "pets", ("lifecycle",), "cx", False,
                            {"pets": "PetState", "lifecycle": "SessionLifecycleState"}, own_state=False)
        self.assertIn(f"{GATE} fixtures: &mut s.fixtures", cx)
        self.assertNotIn("#[cfg(test)]", cx)


class F3FixturesGateScanTest(unittest.TestCase):
    setUp, tearDown, write = T.F3MoveMethodsTest.setUp, T.F3MoveMethodsTest.tearDown, T.F3MoveMethodsTest.write

    def test_fns_under_either_gate_are_test_only_and_keep_their_raw_attribute(self):
        self.write("crates/wow-world/src/session/gate_probe.rs",
                   f"{GATE}\nimpl WorldSession {{\n    pub(crate) fn gated_impl(&self) -> u32 {{\n"
                   "        self.core.account_id\n    }\n}\n\nimpl WorldSession {\n"
                   f"    {GATE}\n    pub(crate) fn gated_fn(&self) -> u32 {{\n        self.core.account_id\n    }}\n"
                   "    #[cfg(test)]\n    pub(crate) fn test_fn(&self) -> u32 {\n        self.core.account_id\n    }\n"
                   "    pub(crate) fn prod_fn(&self) -> u32 {\n        self.core.account_id\n    }\n}\n")
        fns = {f["name"]: f for f in F.scan(self.root)[5] if f["file"] == "session/gate_probe.rs"}
        self.assertEqual({n: f["cfg_test"] for n, f in fns.items()},
                         {"gated_impl": True, "gated_fn": True, "test_fn": True, "prod_fn": False})
        self.assertEqual(fns["gated_impl"]["impl_attr"], [GATE])          # raw text, not the blanked string


if __name__ == "__main__":
    unittest.main()

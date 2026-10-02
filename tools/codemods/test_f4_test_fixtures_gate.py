"""Self-test for f4_test_fixtures_gate.py (#1241 F4a-P2) on synthetic wow-world trees."""
import io, json, os, pathlib, shutil, sys, tempfile, unittest
from contextlib import redirect_stderr, redirect_stdout

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
os.environ.setdefault("F4_ARCH_DIR", str(HERE.parents[1] / "tools/architecture"))
import f4_test_fixtures_gate as G  # noqa: E402

F = 'feature = "test-fixtures"'
GATE = f"any(test, {F})"

CORE = '''use super::*;

pub(crate) struct SessionCore {
    pub account_id: u32,
    #[cfg(test)]
    pub(in crate::session) trace: Vec<u32>,
}
'''

STATE = '''#[cfg(test)]
mod fixtures;
#[cfg(test)]
pub(in crate::session) use fixtures::SessionFixtures;
mod session_core;
pub(crate) use session_core::SessionCore;
#[cfg(test)]
use super::other::OnlyTests;

pub struct WorldSession {
    pub(crate) core: SessionCore,
    #[cfg(test)]
    pub(crate) fixtures: SessionFixtures,
    #[cfg(test)]
    pub(crate) unrelated: OnlyTests,
}
'''

FIXTURES = '''pub(crate) struct SessionFixtures {
    #[cfg(test)]
    pub(crate) seen: u32,
}
'''

OPS = '''use super::*;

impl crate::session::state::SessionCore {
    #[cfg(test)]
    pub(crate) fn record(&mut self, x: u32) {
        #[cfg(test)]
        self.trace.push(x);
        #[cfg(not(test))]
        let _ = x;
    }
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn both(&self) -> bool {
        cfg!(test) && self.account_id > 0
    }
    #[cfg(all(test, debug_assertions))]
    pub(crate) fn dbg_only(&self) {}
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn any_only(&self) {}
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn already(&self) {}
}

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn untouched(&self) {}
}

#[cfg(test)]
pub(crate) const HELPER_LIKE_CPP: u32 = 1;

#[cfg(test)]
pub(crate) const NOT_LISTED: u32 = 2;

#[cfg(test)]
#[path = "../../unit_tests/session/ops/f3_shims.rs"]
mod f3_shims;

#[cfg(test)]
mod tests {
    #[cfg(test)]
    fn inner() {}
}
'''

CONSTRUCTION = '''use super::*;

impl WorldSession {
    pub fn new() -> Self {
        Self {
            core: SessionCore {
                account_id: 1,
                #[cfg(test)]
                trace: Vec::new(),
            },
            #[cfg(test)]
            fixtures: SessionFixtures {
                #[cfg(test)]
                seen: 0,
            },
            #[cfg(test)]
            unrelated: OnlyTests { v: { #[cfg(test)] let z = 1; z } },
        }
    }
    fn make() -> SessionCore {
        todo!()
    }
}
'''

TARGETS = {"files": ["crates/wow-world/src/session/state/session_core.rs",
                     "crates/wow-world/src/session/state/fixtures.rs"],
           "impl_types": ["SessionCore", "SessionFixtures"],
           "items": [{"file": "crates/wow-world/src/session/ops.rs", "name": "HELPER_LIKE_CPP", "kind": "const"}],
           "impls": []}


def run(*argv):
    out = io.StringIO()
    with redirect_stdout(out), redirect_stderr(out):
        rc = G.main(list(argv))
    return rc, out.getvalue()


class F4GateTest(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="f4gate-"))
        self.root = self.tmp / "repo"
        self.write("crates/wow-world/src/session/state.rs", STATE)
        self.write("crates/wow-world/src/session/state/session_core.rs", CORE)
        self.write("crates/wow-world/src/session/state/fixtures.rs", FIXTURES)
        self.write("crates/wow-world/src/session/ops.rs", OPS)
        self.write("crates/wow-world/src/session/construction.rs", CONSTRUCTION)
        self.targets = self.tmp / "targets.json"
        self.targets.write_text(json.dumps(TARGETS))

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def write(self, rel, text):
        p = self.root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)

    def read(self, rel):
        return (self.root / "crates/wow-world/src" / rel).read_text()

    def apply(self):
        rc, out = run("apply", "--text-only", "--root", str(self.root), "--targets", str(self.targets))
        self.assertEqual(rc, 0, out)
        return out

    # -- predicate mapping -----------------------------------------------------------------------
    def test_every_mapping_form(self):
        cases = {"test": GATE, "not(test)": f"not({GATE})", "all(test, debug_assertions)": f"all({GATE}, debug_assertions)",
                 "any(test, debug_assertions)": f"any(test, {F}, debug_assertions)",
                 "not(all(unix, test))": f"not(all(unix, {GATE}))", GATE: GATE, f"not({GATE})": f"not({GATE})",
                 'feature = "x"': 'feature = "x"', "debug_assertions": "debug_assertions"}
        for old, new in cases.items():
            self.assertEqual(G.show(G.gate(G.parse_pred(old))), new, old)
            self.assertEqual(G.has_bare_test(G.parse_pred(old)), old != new, old)

    def test_attribute_forms_in_a_target_impl(self):
        self.apply()
        ops = self.read("session/ops.rs")
        for want in (f"#[cfg({GATE})]\n    pub(crate) fn record", f"        #[cfg({GATE})]\n        self.trace",
                     f"#[cfg(not({GATE}))]\n        let _ = x;", f"#[cfg_attr({GATE}, allow(dead_code))]",
                     f"cfg!({GATE}) && self.account_id", f"#[cfg(all({GATE}, debug_assertions))]",
                     f"#[cfg(any(test, {F}, debug_assertions))]", f"#[cfg({GATE})]\n    pub(crate) fn already"):
            self.assertIn(want, ops)
        self.assertEqual(ops.count(f"any(test, {F}, {F})"), 0)                 # no double insertion

    # -- what is never rewritten / out of scope ----------------------------------------------------
    def test_never_rewritten_mounts_and_untargeted_code(self):
        self.apply()
        ops = self.read("session/ops.rs")
        self.assertIn('#[cfg(test)]\n#[path = "../../unit_tests/session/ops/f3_shims.rs"]\nmod f3_shims;', ops)
        self.assertIn("#[cfg(test)]\nmod tests {\n    #[cfg(test)]\n    fn inner", ops)
        self.assertIn("impl WorldSession {\n    #[cfg(test)]\n    pub(crate) fn untouched", ops)
        self.assertIn("#[cfg(test)]\npub(crate) const NOT_LISTED", ops)
        self.assertIn(f"#[cfg({GATE})]\npub(crate) const HELPER_LIKE_CPP", ops)    # a listed item

    def test_unit_test_path_mount_is_never_rewritten_even_in_a_target_file(self):
        self.write("crates/wow-world/src/session/state/session_core.rs", CORE +
                   '\n#[cfg(test)]\n#[path = "../../../unit_tests/session/state/core_tests.rs"]\nmod core_tests;\n')
        self.apply()
        core = self.read("session/state/session_core.rs")
        self.assertIn('#[cfg(test)]\n#[path = "../../../unit_tests/session/state/core_tests.rs"]', core)
        self.assertIn(f"    #[cfg({GATE})]\n    pub(in crate::session) trace", core)

    # -- matching sites outside the targets ------------------------------------------------------
    def test_literal_field_mount_and_use_propagation(self):
        self.apply()
        con = self.read("session/construction.rs")
        self.assertIn(f"account_id: 1,\n                #[cfg({GATE})]\n                trace", con)   # literal field
        self.assertIn(f"#[cfg({GATE})]\n                seen: 0", con)                            # nested literal
        self.assertIn(f"            #[cfg({GATE})]\n            fixtures: SessionFixtures", con)  # init of a paired field
        self.assertIn("#[cfg(test)] let z = 1;", con)                                             # depth > 1 untouched
        self.assertIn("            #[cfg(test)]\n            unrelated: OnlyTests", con)
        st = self.read("session/state.rs")
        self.assertIn(f"#[cfg({GATE})]\nmod fixtures;", st)                                       # mount of a target file
        self.assertIn(f"#[cfg({GATE})]\npub(in crate::session) use fixtures::SessionFixtures;", st)
        self.assertIn(f"    #[cfg({GATE})]\n    pub(crate) fixtures: SessionFixtures,", st)       # field naming a target type
        self.assertIn("    #[cfg(test)]\n    pub(crate) unrelated: OnlyTests,", st)
        self.assertIn("#[cfg(test)]\nuse super::other::OnlyTests;", st)

    def test_listed_literal_type_pairs_every_field(self):
        t = dict(TARGETS, literal_types=["WorldSession"])
        self.targets.write_text(json.dumps(t))
        self.apply()
        con = self.read("session/construction.rs")
        self.assertIn(f"            #[cfg({GATE})]\n            unrelated: OnlyTests", con)     # `Self {` in impl WorldSession
        self.assertIn("fn make() -> SessionCore {", con)                                          # return type, not a literal

    # -- idempotence, diff shape, plan ---------------------------------------------------------------
    def test_idempotent_and_shape(self):
        before = {p: p.read_text() for p in self.root.rglob("*.rs")}
        self.apply()
        after = {p: p.read_text() for p in self.root.rglob("*.rs")}
        for p, old in before.items():
            G.assert_diff_shape(old, after[p], str(p))
        out = self.apply()
        self.assertIn("already applied (no-op)", out)
        self.assertEqual(after, {p: p.read_text() for p in self.root.rglob("*.rs")})
        rc, out = run("plan", "--root", str(self.root), "--targets", str(self.targets))
        self.assertEqual(rc, 0, out)
        self.assertTrue(out.startswith("0 cfg predicates"), out)

    def test_plan_counts(self):
        rc, out = run("plan", "--root", str(self.root), "--targets", str(self.targets), "--json")
        self.assertEqual(rc, 0, out)
        cats = {}
        for s in json.loads(out):
            cats[s["category"]] = cats.get(s["category"], 0) + 1
        self.assertEqual(cats, {"target-file": 2, "target-impl": 7, "target-item": 1, "literal-pairing": 2,
                                "mount-pairing": 1, "use-pairing": 1, "field-pairing": 1, "field-init-pairing": 1})

    def test_diff_shape_rejects_anything_else(self):
        old = "#[cfg(test)]\nfn a() {}\n"
        G.assert_diff_shape(old, f"#[cfg({GATE})]\nfn a() {{}}\n")
        for bad in ("#[cfg(test)]\nfn b() {}\n", f"#[cfg({GATE})]\nfn a() {{}}\n\n", "#[cfg(unix)]\nfn a() {}\n",
                    f"#[cfg({GATE})]\nfn a() {{ x }}\n"):
            with self.assertRaises(G.CodemodError):
                G.assert_diff_shape(old, bad)
        with self.assertRaises(G.CodemodError):                                                # removal of a gate
            G.assert_diff_shape(f"#[cfg({GATE})]\n", "#[cfg(test)]\n")
        long = "    a || cfg!(test) && b\n"
        G.assert_diff_shape(long, f"    a\n        || cfg!({GATE})\n            && b\n", formatted=True)
        with self.assertRaises(G.CodemodError):
            G.assert_diff_shape(long, f"    a\n        || cfg!({GATE})\n            && c\n", formatted=True)

    def test_use_names_from_the_compile_loop_gate_matching_use_lines(self):
        self.write("crates/wow-world/src/session/mod.rs", "#[cfg(test)]\npub(crate) use ops::HELPER_LIKE_CPP;\n"
                   "#[cfg(test)]\nuse wow_data::{OtherStore, PetStore};\n#[cfg(test)]\nuse wow_data::Unrelated;\n")
        self.targets.write_text(json.dumps(dict(TARGETS, use_names=["HELPER_LIKE_CPP", "PetStore"])))
        self.apply()
        mod = self.read("session/mod.rs")
        self.assertIn(f"#[cfg({GATE})]\npub(crate) use ops::HELPER_LIKE_CPP;", mod)
        self.assertIn(f"#[cfg({GATE})]\nuse wow_data::{{OtherStore, PetStore}};", mod)
        self.assertIn("#[cfg(test)]\nuse wow_data::Unrelated;", mod)
        self.assertTrue(G.gated_use(self.root, "Unrelated"))
        self.assertFalse(G.gated_use(self.root, "PetStore"))

    def test_compile_loop_needs_a_cargo_slot(self):
        os.environ.pop("F4_ALLOW_CARGO", None)
        rc, out = run("apply", "--root", str(self.root), "--targets", str(self.targets))
        self.assertEqual(rc, 1)
        self.assertIn("scheduled cargo slot", out)


if __name__ == "__main__":
    unittest.main()

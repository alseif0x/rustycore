"""Fixture-tree tests for the #1442 wow-world test census and its ceiling ratchet."""
import copy
import json
import os
import subprocess
import pathlib
import sys
import tempfile
import textwrap
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import wow_world_test_census as census  # noqa: E402
import wow_world_test_scan as scan  # noqa: E402

LOCK = """
version = 4

[[package]]
name = "wow-world"
version = "0.1.0"
dependencies = [
 "wow-world-core",
 "wow-world-social",
 "wow-world-spell",
]

[[package]]
name = "wow-world-core"
version = "0.1.0"

[[package]]
name = "wow-world-social"
version = "0.1.0"
dependencies = [
 "wow-world-core",
]

[[package]]
name = "wow-world-spell"
version = "0.1.0"
dependencies = [
 "wow-world-core",
]
"""

FILES = {
    "crates/wow-world-core/src/lib.rs": """
        pub trait SessionHubHostLikeCpp { fn test_fixtures_ref_like_cpp(&self) -> Option<u32>; }
        pub fn hub_value_like_cpp() -> u32 { 1 }
    """,
    "crates/wow-world-social/src/lib.rs": """
        use wow_world_core::SessionHubHostLikeCpp;
        pub trait SocialHostLikeCpp { fn social_ping_like_cpp(&mut self); }
        pub fn social_rank_like_cpp() -> u32 { 2 }
        pub fn rest_like_cpp<H: SessionHubHostLikeCpp>(host: &H) -> bool {
            // test_fixtures_ref_like_cpp().is_some() in a comment is not a site
            #[cfg(any(test, feature = "test-fixtures"))]
            if host.test_fixtures_ref_like_cpp().is_some() {
                return true;
            }
            host.test_fixtures_ref_like_cpp()
                .is_some()
        }
        pub fn bind_like_cpp<H: SessionHubHostLikeCpp>(host: &H) -> u32 {
            if let Some(fixtures) = host.test_fixtures_ref_like_cpp() {
                return fixtures;
            }
            if host.test_fixtures_ref_like_cpp().is_none() { 0 } else { 1 }
        }
    """,
    "crates/wow-world-spell/src/lib.rs": """
        pub fn spell_power_like_cpp() -> u32 { 3 }
    """,
    "crates/wow-world/src/lib.rs": """
        pub mod session;
        pub fn world_only_like_cpp() -> u32 { 4 }
        #[cfg(test)]
        #[path = "../unit_tests/tests.rs"]
        mod tests;
        #[cfg(test)]
        mod src_tests {
            #[test]
            fn src_side_like_cpp() { assert_eq!(super::world_only_like_cpp(), 4); }
        }
    """,
    "crates/wow-world/src/session.rs": """
        pub struct WorldSession { pub social: u32 }
        impl WorldSession {
            pub fn new() -> Self { WorldSession { social: 0 } }
            pub fn social_rank_like_cpp(&self) -> u32 { wow_world_social::social_rank_like_cpp() }
            pub fn handle_world_logic_like_cpp(&mut self) {
                let value = 1;
                if value > 0 { self.social = 2; }
            }
        }
        impl wow_world_social::SocialHostLikeCpp for WorldSession {
            fn social_ping_like_cpp(&mut self) {}
        }
    """,
    "crates/wow-world/unit_tests/tests.rs": """
        use super::*;
        use wow_world_spell::spell_power_like_cpp;

        fn make_session() -> session::WorldSession { session::WorldSession::new() }
        fn indirect_session_like_cpp() -> u32 { let _session = make_session(); 1 }
        struct Fixture { session: session::WorldSession }
        impl Fixture { fn build_like_cpp() -> Self { Fixture { session: make_session() } } }

        #[test]
        fn p_domain() { assert_eq!(spell_power_like_cpp(), 3); }
        #[test]
        fn p_library() { assert_eq!("#[test] fn fake() {}".len(), 19); }
        #[test]
        fn p_world_items() { assert_eq!(world_only_like_cpp(), 4); }
        #[tokio::test]
        async fn h_host() {
            let mut session = make_session();
            session.social_ping_like_cpp();
            assert_eq!(session.social_rank_like_cpp(), 2);
        }
        #[test]
        fn w_world_logic() { let mut session = make_session(); session.handle_world_logic_like_cpp(); }
        #[test]
        fn w_helper_closure() { assert_eq!(indirect_session_like_cpp(), 1); }
        #[test]
        fn w_fixture_type() { let fixture = Fixture::build_like_cpp(); assert_eq!(fixture.session.social, 0); }
        #[test]
        #[ignore = "fixture"]
        fn ignored_p() {}
        mod nested {
            #[test]
            fn inner_like_cpp() {}
        }
    """,
}

CEILING = {
    "schema_version": 1,
    "scope": {"unit_tests_lines": "x", "unit_tests_tests": "x", "src_tests": "x"},
    "entries": [{
        "id": "baseline", "kind": "baseline", "issue": 1442, "reviewed_at": "2026-10-21",
        "reason": "fixture", "unit_tests_lines": 0, "unit_tests_tests": 9, "src_tests": 1,
    }],
}


def write_tree(root: pathlib.Path) -> None:
    (root / "Cargo.lock").write_text(LOCK, encoding="utf-8")
    for relative, text in FILES.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(textwrap.dedent(text).lstrip(), encoding="utf-8")
    ceiling = copy.deepcopy(CEILING)
    lines = len((root / "crates/wow-world/unit_tests/tests.rs").read_text().splitlines())
    ceiling["entries"][0]["unit_tests_lines"] = lines
    (root / census.CEILING).parent.mkdir(parents=True, exist_ok=True)
    (root / census.CEILING).write_text(json.dumps(ceiling), encoding="utf-8")


class CensusTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name)
        write_tree(self.root)

    def tearDown(self):
        self.tmp.cleanup()

    def rows(self):
        ledger, metrics, lines = census.census(self.root)
        return {row["id"].split("::", 1)[1]: row for row in ledger["tests"]}, ledger, metrics, lines

    def test_classes_destinations_and_reasons(self):
        rows, ledger, metrics, _ = self.rows()
        expected = {
            "p_domain": ("P", "wow-world-spell", "P-domain", False),
            "p_library": ("P", "wow-world", "P-library", False),
            "p_world_items": ("P", "wow-world", "P-world-items", False),
            "h_host": ("H", "wow-world-social", "H-host", True),
            "w_world_logic": ("W", "wow-world", "W-world-logic", True),
            "w_helper_closure": ("W", "wow-world", "W-fields", True),
            "w_fixture_type": ("W", "wow-world", "W-fields", True),
            "ignored_p": ("P", "wow-world", "P-library", False),
            "nested::inner_like_cpp": ("P", "wow-world", "P-library", False),
            "src_tests::src_side_like_cpp": ("P", "wow-world", "P-world-items", False),
        }
        got = {key: (row["class"], row["destination"], row["reason"], row["session"]) for key, row in rows.items()}
        self.assertEqual(got, expected)
        self.assertEqual(rows["p_domain"]["owner"], "wow-world-spell")
        self.assertEqual(rows["h_host"]["owner"], "wow-world-social")
        self.assertTrue(rows["h_host"]["shims"] is False)
        self.assertEqual(rows["w_world_logic"]["blockers"], ["handle_world_logic_like_cpp"])
        self.assertEqual(rows["p_world_items"]["blockers"], ["world_only_like_cpp"])
        self.assertTrue(rows["ignored_p"].get("ignored"))
        self.assertEqual(metrics["unit_tests_tests"], 9)
        self.assertEqual(metrics["src_tests"], 1)
        self.assertEqual(ledger["summary"]["by_class"], {"P": 6, "H": 1, "W": 3})
        self.assertEqual(sum(sum(row.values()) for row in ledger["summary"]["by_destination"].values()), 10)
        self.assertEqual(ledger["host_trait_methods"], {"wow-world-social": 1})

    def test_world_test_branch_inventory_ignores_comments_and_marks_gating(self):
        _, ledger, _, _ = self.rows()
        self.assertEqual(ledger["world_test_branches"], [
            {"crate": "wow-world-social", "file": "crates/wow-world-social/src/lib.rs", "fn": "rest_like_cpp",
             "kind": "fixture-check", "test_fixtures_gated": True, "sites": 1},
            {"crate": "wow-world-social", "file": "crates/wow-world-social/src/lib.rs", "fn": "rest_like_cpp",
             "kind": "fixture-check", "test_fixtures_gated": False, "sites": 1},
            {"crate": "wow-world-social", "file": "crates/wow-world-social/src/lib.rs", "fn": "bind_like_cpp",
             "kind": "fixture-binding", "test_fixtures_gated": False, "sites": 1},
            {"crate": "wow-world-social", "file": "crates/wow-world-social/src/lib.rs", "fn": "bind_like_cpp",
             "kind": "fixture-check", "test_fixtures_gated": False, "sites": 1},
        ])

    def test_fully_qualified_test_attributes_are_counted(self):
        tests = self.root / "crates/wow-world/unit_tests/tests.rs"
        tests.write_text(tests.read_text() + "#[::tokio::test]\nasync fn rooted_like_cpp() {}\n"
                         "#[core::prelude::v1::test]\nfn prelude_like_cpp() {}\n"
                         "#[test_case]\nfn not_a_test_like_cpp() {}\n", encoding="utf-8")
        rows, _, metrics, _ = self.rows()
        self.assertIn("rooted_like_cpp", rows)
        self.assertIn("prelude_like_cpp", rows)
        self.assertNotIn("not_a_test_like_cpp", rows)
        self.assertEqual(metrics["unit_tests_tests"], 11)

    def test_tests_mounted_outside_src_and_unit_tests_fail_closed(self):
        lib = self.root / "crates/wow-world/src/lib.rs"
        lib.write_text(lib.read_text() + '#[cfg(test)]\n#[path = "../elsewhere/extra.rs"]\nmod extra;\n')
        extra = self.root / "crates/wow-world/elsewhere/extra.rs"
        extra.parent.mkdir()
        extra.write_text("#[test]\nfn escaped_like_cpp() {}\n", encoding="utf-8")
        with self.assertRaisesRegex(census.CensusError, "outside src/ and unit_tests/: elsewhere/extra.rs"):
            census.census(self.root)

    def test_unmounted_src_test_fails_the_raw_attribute_count(self):
        ledger, _, _ = census.census(self.root)
        (self.root / census.LEDGER).write_text(census.dump(ledger), encoding="utf-8")
        (self.root / "crates/wow-world/src/stray.rs").write_text("#[test]\nfn stray_like_cpp() {}\n")
        (self.root / "crates/wow-world/tests").mkdir()
        (self.root / "crates/wow-world/tests/it.rs").write_text("#[test]\nfn integration_like_cpp() {}\n")
        errors, metrics, _, _ = census.check_errors(self.root)
        self.assertEqual(metrics["raw_test_attributes"], 11)
        self.assertEqual(errors, ["11 test attributes in crates/wow-world/** (outside tests/) but 10 mounted "
                                  "test functions: a test source is not mounted from src/lib.rs"])

    def test_ledger_round_trip_and_check_passes(self):
        ledger, _, _ = census.census(self.root)
        text = census.dump(ledger)
        self.assertEqual(census.load(text), json.loads(json.dumps(ledger)))
        (self.root / census.LEDGER).write_text(text, encoding="utf-8")
        errors, metrics, ceiling, _ = census.check_errors(self.root)
        self.assertEqual(errors, [])
        self.assertEqual(ceiling["id"], "baseline")

    def test_check_rejects_drift_growth_and_unmounted_sources(self):
        ledger, _, _ = census.census(self.root)
        (self.root / census.LEDGER).write_text(census.dump(ledger), encoding="utf-8")
        tests = self.root / "crates/wow-world/unit_tests/tests.rs"
        tests.write_text(tests.read_text() + "#[test]\nfn added_like_cpp() {}\n", encoding="utf-8")
        (self.root / "crates/wow-world/unit_tests/orphan.rs").write_text("#[test]\nfn lost() {}\n")
        errors, _, _, _ = census.check_errors(self.root)
        self.assertIn("test not in ledger: unit_tests/tests.rs::added_like_cpp", errors)
        self.assertIn("unit_tests source not mounted from src/lib.rs: unit_tests/orphan.rs", errors)
        self.assertTrue(any(e.startswith("unit_tests_tests 10 exceeds the reviewed ceiling 9") for e in errors), errors)
        self.assertTrue(any(e.startswith("unit_tests_lines ") for e in errors), errors)
        self.assertTrue(any(e.startswith("regenerate with:") for e in errors), errors)

    def test_reclassification_is_drift(self):
        ledger, _, _ = census.census(self.root)
        edited = json.loads(json.dumps(ledger))
        edited["tests"][0]["class"] = "W"
        problems = census.ledger_diff(census.load(census.dump(edited)), census.load(census.dump(ledger)))
        self.assertEqual(problems, [f"ledger row changed: {ledger['tests'][0]['id']} (class)"])

    def test_shrinking_suite_passes_and_suggests_tightening(self):
        tests = self.root / "crates/wow-world/unit_tests/tests.rs"
        tests.write_text(tests.read_text().replace("#[test]\nfn p_library()", "fn p_library()"), encoding="utf-8")
        ledger, _, _ = census.census(self.root)
        (self.root / census.LEDGER).write_text(census.dump(ledger), encoding="utf-8")
        errors, metrics, ceiling, _ = census.check_errors(self.root)
        self.assertEqual(errors, [])
        self.assertLess(metrics["unit_tests_tests"], ceiling["unit_tests_tests"])

    def test_ledger_does_not_depend_on_hash_seed(self):
        tool = pathlib.Path(census.__file__).resolve()
        outputs = []
        for seed in ("1", "2"):
            subprocess.run([sys.executable, str(tool), "census", "--repo", str(self.root), "--write"],
                           check=True, capture_output=True, env={**os.environ, "PYTHONHASHSEED": seed})
            outputs.append((self.root / census.LEDGER).read_text(encoding="utf-8"))
        self.assertEqual(outputs[0], outputs[1])

    def test_attribute_count_must_match_test_functions(self):
        tests = self.root / "crates/wow-world/unit_tests/tests.rs"
        tests.write_text(tests.read_text() + "#[test]\nconst NOT_A_FN: u32 = 1;\n", encoding="utf-8")
        with self.assertRaisesRegex(census.CensusError, "test attributes"):
            census.census(self.root)


class CeilingCommandTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name)
        write_tree(self.root)
        ledger, _, _ = census.census(self.root)
        (self.root / census.LEDGER).write_text(census.dump(ledger), encoding="utf-8")
        self.tests = self.root / "crates/wow-world/unit_tests/tests.rs"

    def tearDown(self):
        self.tmp.cleanup()

    def run_tool(self, *args):
        tool = pathlib.Path(census.__file__).resolve()
        return subprocess.run([sys.executable, str(tool), "census", "--repo", str(self.root), *args],
                              capture_output=True, text=True)

    def policy(self):
        return json.loads((self.root / census.CEILING).read_text(encoding="utf-8"))

    def test_one_step_raise_writes_ledger_and_an_exact_entry(self):
        self.tests.write_text(self.tests.read_text() + "#[test]\nfn added_like_cpp() {}\n", encoding="utf-8")
        self.assertNotEqual(census.check_errors(self.root)[0], [])
        result = self.run_tool("--write", "--raise", "--issue", "1500", "--reason", "new regression test")
        self.assertEqual(result.returncode, 0, result.stderr)
        errors, metrics, ceiling, _ = census.check_errors(self.root)
        self.assertEqual(errors, [])
        self.assertEqual(ceiling["kind"], "raise")
        self.assertEqual(ceiling["id"], "raise-1500-1")
        self.assertEqual({k: ceiling[k] for k in census.CEILING_KEYS}, {k: metrics[k] for k in census.CEILING_KEYS})

    def test_raise_needs_growth_an_issue_a_reason_and_write(self):
        before = self.policy()
        self.assertEqual(self.run_tool("--write", "--raise", "--issue", "1500", "--reason", "x").returncode, 1)
        self.tests.write_text(self.tests.read_text() + "#[test]\nfn added_like_cpp() {}\n", encoding="utf-8")
        self.assertEqual(self.run_tool("--write", "--raise", "--issue", "1500").returncode, 1)
        self.assertEqual(self.run_tool("--write", "--raise", "--reason", "x").returncode, 1)
        self.assertEqual(self.run_tool("--raise", "--issue", "1500", "--reason", "x").returncode, 2)
        self.assertEqual(self.policy(), before)

    def test_newest_raise_must_equal_the_live_metrics(self):
        self.tests.write_text(self.tests.read_text() + "#[test]\nfn added_like_cpp() {}\n", encoding="utf-8")
        self.assertEqual(self.run_tool("--write", "--raise", "--issue", "1500", "--reason", "x").returncode, 0)
        self.tests.write_text(self.tests.read_text().replace("#[test]\nfn added_like_cpp() {}\n", ""),
                              encoding="utf-8")
        ledger, _, _ = census.census(self.root)
        (self.root / census.LEDGER).write_text(census.dump(ledger), encoding="utf-8")
        errors = census.check_errors(self.root)[0]
        self.assertTrue(errors and all("a raise must equal the live metrics" in e for e in errors), errors)
        result = self.run_tool("--write", "--tighten", "--issue", "1500", "--reason", "test moved")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(census.check_errors(self.root)[0], [])
        self.assertEqual(self.policy()["entries"][-1]["kind"], "tighten")


class CeilingPolicyTests(unittest.TestCase):
    def entry(self, **changes):
        base = dict(CEILING["entries"][0], unit_tests_lines=100)
        base.update(changes)
        return base

    def policy(self, *entries):
        return {"schema_version": 1, "scope": {}, "entries": list(entries)}

    def test_newest_entry_is_the_ceiling(self):
        tighter = self.entry(id="t1", kind="tighten", unit_tests_tests=8, reviewed_at="2026-10-22")
        self.assertEqual(census.effective_ceiling(self.policy(self.entry(), tighter))["unit_tests_tests"], 8)

    def test_raise_requires_a_reviewed_raise_entry(self):
        grows = self.entry(id="t1", kind="tighten", unit_tests_tests=10)
        with self.assertRaisesRegex(census.CensusError, "without kind raise"):
            census.effective_ceiling(self.policy(self.entry(), grows))
        raised = self.entry(id="t1", kind="raise", unit_tests_tests=10)
        self.assertEqual(census.effective_ceiling(self.policy(self.entry(), raised))["unit_tests_tests"], 10)
        empty_raise = self.entry(id="t1", kind="raise")
        with self.assertRaisesRegex(census.CensusError, "raises nothing"):
            census.effective_ceiling(self.policy(self.entry(), empty_raise))

    def test_malformed_policies_are_rejected(self):
        cases = [
            ({"schema_version": 1, "entries": []}, "exactly"),
            (self.policy(), "baseline entry"),
            (self.policy(self.entry(kind="tighten")), "first ceiling entry"),
            (self.policy(self.entry(), self.entry()), "unique id"),
            (self.policy(self.entry(reason=" ")), "reason"),
            (self.policy(self.entry(unit_tests_tests=-1)), "non-negative"),
            (self.policy(self.entry(reviewed_at="soon")), "ISO date"),
            (self.policy(self.entry(), self.entry(id="t1", kind="tighten", reviewed_at="2026-01-01")), "review order"),
            (self.policy({"id": "x"}), "requires exactly"),
        ]
        for policy, message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(census.CensusError, message):
                census.effective_ceiling(policy)

    def test_ceiling_errors_name_each_breached_metric(self):
        ceiling = self.entry()
        metrics = {"unit_tests_lines": 101, "unit_tests_tests": 9, "src_tests": 2}
        errors = census.ceiling_errors(metrics, ceiling)
        self.assertEqual([e.split(" ")[0] for e in errors], ["unit_tests_lines", "src_tests"])


class LexerTests(unittest.TestCase):
    def test_mask_blanks_literals_and_comments_but_keeps_offsets(self):
        source = ('fn a() { let s = "x{\\"}"; let r = r#"}"#; let c = \'{\'; let l: &\'a str; '
                  'let b = b\'}\'; /* { /* } */ */ // }\n}')
        masked = scan.mask(source)
        self.assertEqual(len(masked), len(source))
        self.assertEqual(scan.brace_pairs(masked), {masked.index("{"): len(source) - 1})
        self.assertIn("'a str", masked)
        self.assertEqual(scan.mask("é\n"), " \n")

    def test_symbol_names_skip_plain_variables(self):
        names = scan.symbol_names(
            "let player = Foo::bar(x); player.go_like_cpp(); baz!(y); qux; Step::Commit; "
            "WorldSession::shim_like_cpp(&mut s); wow_world_x::Item")
        self.assertEqual(names, {"Foo", "go_like_cpp", "baz", "Step", "WorldSession", "shim_like_cpp",
                                 "wow_world_x", "Item"})

    def test_use_trees_map_names_to_crate_roots(self):
        imports = scan.parse_uses(
            "use wow_world_quest::{a::B, c as D, e::{F, G}};\nuse super::*;\nuse std::x::Y;\n",
            lambda root: root.startswith("wow_"),
        )
        self.assertEqual(imports, {"B": "wow_world_quest", "D": "wow_world_quest",
                                   "F": "wow_world_quest", "G": "wow_world_quest"})


if __name__ == "__main__":
    unittest.main()

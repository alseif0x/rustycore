"""Synthetic-tree tests for the #1241 wow-world coupling map tool."""
import contextlib
import io
import json
import pathlib
import sys
import tempfile
import textwrap
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import wow_world_coupling as coupling  # noqa: E402

STATE = """
use std::collections::HashMap;

/// Docs mentioning a brace { and a struct WorldSession { that must not count.
pub struct WorldSession {
    // Identity { comment brace
    pub account_id: u32,
    /// Doc comment: `fake_field: u8,`
    pub(crate) player_handle: Option<u64>,
    #[cfg(test)]
    pub(in crate::session) test_only_flag: bool,
    #[cfg(feature = "x")]
    #[allow(dead_code)]
    gated: u8,
    nested: HashMap<u32, Vec<Option<(u8, u16)>>>,
    callback: Box<dyn Fn(u32, u8) -> Result<u8, ()> + Send>,
    multiline:
        crate::session::Wrapper<
            std::sync::Arc<u8>,
        >,
    array: [u8; 4],
}

impl WorldSession {
    pub fn state_helper(&self) -> u32 { self.account_id }
}
"""

ALPHA = """
impl<'a> WorldSession {
    pub fn alpha_one(&mut self) {
        let brace = "}}}";
        let ch = '{';
        let esc = '\\'';
        // } stray comment brace
        /* nested /* } */ block */
        fn nested_helper() {}
        self.beta_one();
        self.beta_one();
        let _ = self.account_id + self.player_handle.unwrap_or(0) as u32;
        let _ = self.nested.len();
    }

    fn alpha_two<'b>(&'b self) -> &'b str { r#"raw { brace"# }
}
"""

BETA = """
impl crate::session::WorldSession {
    pub(crate) fn beta_one(&mut self) {
        self.alpha_two();
        let _ = self.array;
        let _ = self.nested.len();
        let _ = self.account_id;
    }
}

impl Runtime for WorldSession {
    fn trait_method(&self) {}
}
"""

HANDLER = """
pub fn handle(session: &mut WorldSession) {
    session.alpha_one();
    session.beta_one();
    let _ = session.account_id;
    let _ = session.player_handle;
    let _ = session.state_helper();
}
"""

TEST_FILE = """
impl WorldSession {
    fn only_in_tests(&self) { self.alpha_one(); }
}
"""


class CouplingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="rustycore-coupling-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        self.write("session/state.rs", STATE)
        self.write("session/alpha.rs", ALPHA)
        self.write("session/beta/mod.rs", BETA)
        self.write("handlers/chat/mod.rs", HANDLER)
        self.write("handlers/misc.rs", HANDLER)
        self.write("session/tests/scenario.rs", TEST_FILE)
        self.write("session/alpha_tests.rs", "// test only\n")

    def write(self, rel, text):
        path = self.root / coupling.CRATE_SRC / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(textwrap.dedent(text), encoding="utf-8")

    def report(self, **kwargs):
        return coupling.analyze(self.root, **kwargs)

    def test_field_parsing_handles_cfg_generics_and_docs(self):
        fields = {f["name"]: f["cfg"] for f in coupling.parse_fields(STATE)}
        self.assertEqual(sorted(fields), [
            "account_id", "array", "callback", "gated", "multiline", "nested",
            "player_handle", "test_only_flag",
        ])
        self.assertEqual(fields["test_only_flag"], ["#[cfg(test)]"])
        self.assertEqual(fields["gated"], ['#[cfg(feature = " ")]'])
        self.assertEqual(fields["account_id"], [])

    def test_blanking_keeps_offsets_and_lifetimes(self):
        code = coupling.blank_noncode(ALPHA)
        self.assertEqual(len(code), len(ALPHA))
        self.assertEqual(code.count("\n"), ALPHA.count("\n"))
        self.assertIn("impl<'a> WorldSession", code)
        self.assertEqual(code.count("{"), code.count("}"))

    def test_method_attribution_skips_nested_and_trait_fns(self):
        code = coupling.blank_noncode(textwrap.dedent(ALPHA))
        self.assertEqual(coupling.impl_methods(code), ["alpha_one", "alpha_two"])
        beta = coupling.blank_noncode(textwrap.dedent(BETA))
        self.assertEqual(coupling.impl_methods(beta), ["beta_one"])

    def test_r5_metric_and_path_classification(self):
        r5 = self.report()["r5"]
        self.assertEqual(r5["worldsession_fields"], 8)
        self.assertEqual(r5["worldsession_cfg_fields"], 2)
        self.assertEqual(r5["impl_methods_production"], 4)
        self.assertEqual(r5["impl_methods_test"], 1)
        self.assertEqual(r5["impl_files_production"], 3)
        self.assertEqual(r5["impl_files_test"], 1)
        self.assertEqual(r5["test_lines"], textwrap.dedent(TEST_FILE).count("\n") + 1)
        self.assertTrue(coupling.is_test_path("session/tests/scenario.rs"))
        self.assertTrue(coupling.is_test_path("tests.rs"))
        self.assertTrue(coupling.is_test_path("handlers/x/test_support.rs"))
        self.assertFalse(coupling.is_test_path("session/testament.rs"))
        self.assertEqual(coupling.domain_of("session/beta/mod.rs"), "session/beta")
        self.assertEqual(coupling.domain_of("lib.rs"), "lib")

    def test_cross_domain_edges_and_domain_stats(self):
        report = self.report()
        edges = {(e["from"], e["to"]): e["calls"] for e in report["top_edges"]}
        self.assertEqual(edges[("session/alpha", "session/beta")], 2)
        self.assertEqual(edges[("session/beta", "session/alpha")], 1)
        self.assertEqual(edges[("handlers/chat", "session/alpha")], 1)
        self.assertNotIn(("session/tests", "session/alpha"), edges)
        stats = {s["domain"]: s for s in report["domains"]}
        self.assertEqual(stats["session/alpha"]["methods"], 2)
        self.assertEqual(stats["session/alpha"]["calls_in"], 3)
        self.assertEqual(stats["session/alpha"]["calls_out"], 2)
        self.assertEqual(stats["handlers/chat"]["outbound_domains"], 3)
        self.assertEqual(report["fields"]["owner"]["account_id"], "handlers/chat")
        self.assertEqual(report["fields"]["owner"]["nested"], "session/alpha")
        # Method calls are not field accesses; `state_helper` is a call edge.
        self.assertEqual(stats["handlers/chat"]["own_accesses"]
                         + stats["handlers/chat"]["foreign_accesses"], 2)

    def test_hub_detection_respects_threshold(self):
        hubs = [h["field"] for h in self.report(hub_threshold=4)["hub_fields"]]
        self.assertEqual(hubs, ["account_id"])
        hubs = [h["field"] for h in self.report(hub_threshold=2)["hub_fields"]]
        self.assertEqual(hubs, ["account_id", "player_handle", "nested"])

    def test_cycles_reported_as_dag_violations(self):
        violations = self.report()["dag_violations"]
        self.assertEqual([v["domains"] for v in violations],
                         [["session/alpha", "session/beta"]])
        self.assertEqual(len(violations[0]["edges"]), 2)
        order = [o["domain"] for o in self.report()["extraction_order"]]
        self.assertEqual(order[0], "session/beta")
        self.assertEqual(order[-1], "handlers/misc")
        sccs = coupling.strongly_connected(
            ["a", "b", "c", "d"], {"a": ["b"], "b": ["c"], "c": ["a"], "d": ["a"]})
        self.assertEqual(sccs, [["a", "b", "c"]])

    def test_json_shape_and_text_render(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            status = coupling.main(["report", "--root", str(self.root), "--json", "--top", "2"])
        self.assertEqual(status, 0)
        data = json.loads(out.getvalue())
        self.assertEqual(set(data), {
            "schema_version", "crate_src", "r5", "fields", "methods", "hub_threshold",
            "hub_fields", "domains", "top_edges", "dag_violations", "extraction_order",
            "footnote",
        })
        self.assertEqual(len(data["top_edges"]), 2)
        text = coupling.render(self.report(), 5)
        self.assertIn("DAG violations among top edges: 1", text)
        self.assertIn("Lexical scan limits", text)

    def test_substate_leaves_attribute_nested_accesses(self):
        self.write("session/state.rs", """
            pub struct WorldSession {
                pub core: SessionCore,
                #[cfg(test)]
                pub(crate) fixtures: SessionFixtures,
                pub(crate) plain: u32,
            }
        """)
        self.write("session/state/fixtures.rs", """
            pub(crate) struct SessionFixtures {
                pub(crate) movement: MovementState,
            }
        """)
        self.write("session/state/session_core.rs", """
            pub struct SessionCore {
                pub account_id: u32,
                #[cfg(test)]
                pub(crate) fixture: u8,
            }
        """)
        self.write("session/state/movement.rs", """
            pub(crate) struct MovementState {
                pub(crate) position: u8,
            }
        """)
        self.write("session/alpha.rs", """
            impl WorldSession {
                fn alpha(&self) -> u32 {
                    let _ = self.core.fixture;
                    let _ = self.core.helper();
                    let _ = self.fixtures.movement.position;
                    let _ = self.fixtures.movement;
                    self.core
                        .account_id + self.plain
                }
            }
        """)
        self.write("handlers/chat/mod.rs", """
            pub fn handle(session: &mut WorldSession) {
                let _ = session.core.account_id;
                let _ = session.core;
            }
        """)
        self.write("session/beta/mod.rs", "\n")
        self.write("handlers/misc.rs", "\n")
        report = self.report()
        r5 = report["r5"]
        self.assertEqual(r5["worldsession_fields"], 3)
        self.assertEqual(r5["worldsession_cfg_fields"], 1)
        self.assertEqual(r5["substate_leaf_fields"], 4)
        self.assertEqual(r5["substate_leaf_cfg_fields"], 2)
        owner = report["fields"]["owner"]
        self.assertEqual(sorted(owner), ["account_id", "fixture", "plain", "position"])
        self.assertEqual(owner["position"], "session/alpha")
        self.assertEqual(owner["plain"], "session/alpha")
        # `account_id` is read once in each domain; the tie breaks by name.
        self.assertEqual(owner["account_id"], "handlers/chat")
        self.assertEqual(report["fields"]["unused_in_production"], [])
        text = coupling.render(report, 5)
        self.assertIn("sub-state leaf fields             4  (2 cfg-gated)", text)

    def test_f3_moved_thunks_and_shims_are_counted(self):
        self.write("session/state.rs", """
            pub struct WorldSession {
                pub(crate) catalogs: SessionCatalogs,
            }
        """)
        self.write("session/state/catalogs.rs", """
            pub(crate) struct SessionCatalogs {
                pub(crate) store: u8,
            }
        """)
        self.write("session/alpha.rs", """
            impl WorldSession {
                fn store(&self) -> u8 { self.catalogs.store() }
                fn other(&self) -> u8 { 0 }
            }
            impl crate::session::state::SessionCatalogs {
                fn store(&self) -> u8 { self.store }
                fn only_moved(&self) -> u8 { 1 }
            }
            impl<'a> HubRef<'a> {
                fn hub_fn(&self) {}
            }
        """)
        shim = self.root / coupling.CRATE_SRC.parent / "unit_tests/session/alpha/f3_shims.rs"
        shim.parent.mkdir(parents=True, exist_ok=True)
        shim.write_text("impl crate::session::WorldSession {\n    fn shim_a(&self) {}\n"
                        "    fn shim_b(&self) {}\n}\n", encoding="utf-8")
        r5 = self.report()["r5"]
        self.assertEqual(r5["impl_methods_production"], 3)  # alpha.rs 2 + beta_one
        self.assertEqual(r5["substate_impl_methods_production"], 3)
        self.assertEqual(r5["worldsession_thunks"], 1)
        self.assertEqual(r5["f3_test_shims"], 2)
        self.assertIn("moved onto sub-states             3  fns (1 WorldSession thunks keep their name),"
                      " 2 unit_tests shims", coupling.render(self.report(), 3))

    def test_missing_struct_is_an_error(self):
        self.write("session/state.rs", "pub struct Other {}\n")
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(coupling.main(["report", "--root", str(self.root)]), 2)


if __name__ == "__main__":
    unittest.main()

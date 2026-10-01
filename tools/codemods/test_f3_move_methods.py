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
        "    pub(crate) fn uses_whole(&self) -> u32 {\n"
        "        helper(self) + self.catalogs.bar\n"
        "    }\n"
        "    fn never_called(&self) -> u32 {\n"
        "        self.catalogs.bar\n"
        "    }\n"
        "    fn tests_only_two(&self) -> u32 {\n"
        "        self.catalogs.bar\n"
        "    }\n"
        "}\n\n"
        "impl WorldSession {\n"
        "    fn only_tests(&self) -> u32 {\n"
        "        self.catalogs.bar + 1\n"
        "    }\n"
        "    #[cfg(test)]\n    fn uses_test_support(&self) -> u32 {\n"
        "        self.catalogs.bar + self.fixture_view()\n    }\n"
        "    pub(crate) fn pair(&self) -> [u32; 2] {\n"
        "        [self.catalogs.bar, 0]\n"
        "    }\n"
        "    pub(crate) fn ctx_only(&self) -> u32 {\n"
        "        self.catalogs.bar\n"
        "    }\n"
        "    pub(crate) fn with_mut(&self, mut f: impl FnMut(u32) -> u32) -> u32 {\n"
        "        f(self.catalogs.bar)\n"
        "    }\n"
        "}\n"),
    "crates/wow-world/src/session/connection.rs": (
        "use super::*;\n\n"
        "impl WorldSession {\n"
        "    pub fn account_plus_foo(&self) -> u32 {\n"
        "        self.core.account_id + self.foo_store().unwrap_or(0) + self.pair()[0]\n"
        "            + self.with_mut(|x| x)\n"
        "    }\n"
        "    #[cfg(test)]\n"
        "    fn test_helper(&self) -> u32 {\n"
        "        self.ctx_only()\n"
        "    }\n"
        "}\n"),
    "crates/wow-world/src/session/test_support/ops.rs": (
        "impl WorldSession {\n    pub(crate) fn fixture_view(&self) -> u32 {\n        0\n    }\n}\n"),
    "crates/wow-world/unit_tests/catalogs_tests.rs": (
        "fn t(session: &mut WorldSession) -> u32 {\n    session.set_foo_store(1);\n"
        "    session.only_tests() + session.tests_only_two()\n}\n"),
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
        self.assertEqual(set(rows), {"foo_store", "set_foo_store", "only_tests", "pair", "ctx_only", "with_mut"})
        self.assertEqual((rows["ctx_only"]["thunk"], bool(rows["ctx_only"]["shim"])), ("", True))  # cfg(test) caller
        self.assertIn("resident", rows["foo_store"]["thunk"])
        self.assertEqual(rows["set_foo_store"]["thunk"], "pub API, no production caller")
        self.assertEqual((rows["only_tests"]["thunk"], bool(rows["only_tests"]["shim"])), ("", True))
        self.assertTrue(P["blocked"]["uses_whole"].startswith("whole-self"))
        self.assertIn("never_called", P["blocked"])                                     # uncalled: stays
        self.assertEqual(P["blocked"]["uses_test_support"], "callee fixture_view not moved")  # test-path callee
        self.assertTrue(P["blocked"]["tests_only_two"].startswith("dead in non-test builds"))  # lint group

    def test_upgrade_keeps_hub_preconditions(self):
        f = dict(target="config", store_rehomed=True, store_from="core", recv="&self", acc={"config": 1},
                 file="handlers/stats.rs", ret_borrow=False)
        self.assertEqual(F.upgrade(f, "state", {"P", "C-hub"}), "reads config outside crate::session")
        f.update(target="config", store_rehomed=True, store_from="core", file="session/stats.rs", ret_borrow=True)
        self.assertTrue(F.upgrade(f, "state", {"P", "C-hub"}).startswith("returns a borrow"))
        f.update(target="config", store_rehomed=True, store_from="core", ret_borrow=False, writes_store=True)
        self.assertTrue(F.upgrade(f, "state", {"P", "C-hub"}).startswith("writes catalogs/config"))
        f.update(target="config", store_rehomed=True, store_from="core", writes_store=False)
        self.assertEqual(F.upgrade(f, "state", {"P", "C-hub"}), ("C-hub", "hubref", "HubRef"))
        f.update(target="config", store_rehomed=True, store_from="spell_effects", ret_borrow=False)
        self.assertTrue(F.upgrade(f, "state", {"P", "C-hub"}).startswith("stateless group"))  # no state type
        f.update(target="social", store_rehomed=False, ret_borrow=True)            # group state + hub param
        self.assertTrue(F.upgrade(f, "state", {"P", "C-hub"}).startswith("returns a borrow"))
        self.assertTrue(F.STORE_WRITE.search("self.config.max_level = 3;"))
        self.assertTrue(F.STORE_WRITE.search("take(&mut self.catalogs.x)"))
        self.assertFalse(F.STORE_WRITE.search("if self.config.max_level == 3 {}"))

    def test_config_route_outside_session_is_blocked(self):
        state = self.root / "crates/wow-world/src/session/state.rs"
        state.write_text(state.read_text().replace(
            "    pub(crate) catalogs: SessionCatalogs,\n",
            "    pub(crate) catalogs: SessionCatalogs,\n    pub(in crate::session) config: SessionWorldConfig,\n"))
        self.write("crates/wow-world/src/session/runtime_policy_access.rs",
                   "impl crate::session::state::SessionWorldConfig {\n"
                   "    pub(crate) fn limit(&self) -> u32 {\n        self.limit\n    }\n}\n")
        body = "    pub(crate) fn uses_limit(&self) -> u32 {\n        self.core.account_id + self.limit()\n    }\n"
        self.write("crates/wow-world/src/handlers/chat.rs", "impl WorldSession {\n" + body + "}\n")
        self.write("crates/wow-world/src/session/admission.rs",
                   "impl WorldSession {\n" + body.replace("uses_limit", "inner_limit") + "}\n")
        F.DOMAIN["handlers/chat"] = "core"
        self.addCleanup(F.DOMAIN.pop, "handlers/chat")
        P = F.plan(self.root, {"core"}, {"P", "C-hub"})
        self.assertEqual(P["blocked"].get("uses_limit"), "callee on an unreachable owner")
        self.assertTrue(P["blocked"]["inner_limit"].startswith("no callers"))  # reachable inside session
        self.write("crates/wow-world/src/session/social/ops.rs",
                   "impl WorldSession {\n    pub fn set_level_req(&mut self, v: u32) {\n"
                   "        self.config.level = v;\n    }\n}\n")
        tests = self.root / "crates/wow-world/unit_tests/catalogs_tests.rs"
        tests.write_text(tests.read_text() + "fn s(x: &mut WorldSession) { x.set_level_req(2) }\n")
        P = F.plan(self.root, {"social", "config"}, {"P", "C-hub"})
        self.assertEqual(P["cand"]["set_level_req"], ("P", "state", "SessionWorldConfig"))  # any group's setter

    def test_player_guid_is_a_permanent_inline_thunk(self):
        self.write("crates/wow-world/src/session/player_binding.rs",
                   "impl WorldSession {\n    pub fn player_guid(&self) -> u32 {\n"
                   "        self.core.account_id\n    }\n}\n")
        tests = self.root / "crates/wow-world/unit_tests/catalogs_tests.rs"
        tests.write_text(tests.read_text() + "fn g(s: &WorldSession) -> u32 { s.player_guid() }\n")
        rc, out = run("apply", "--group", "core", "--root", str(self.root), "--text-only", "--demote-blocked")
        self.assertEqual(rc, 0, out)
        text = (self.root / "crates/wow-world/src/session/player_binding.rs").read_text()
        self.assertIn("    #[inline]\n    pub fn player_guid(&self) -> u32 {\n        self.core.player_guid()\n", text)
        self.assertIn("impl crate::session::state::SessionCore {", text)
        self.assertFalse((self.root / "crates/wow-world/unit_tests/session/player_binding").exists())

    def test_fixture_group_and_state_hub_kinds(self):
        state = self.root / "crates/wow-world/src/session/state.rs"
        state.write_text(state.read_text().replace(
            "    pub(crate) catalogs: SessionCatalogs,\n",
            "    pub(crate) catalogs: SessionCatalogs,\n    #[cfg(test)]\n    pub(crate) fixtures: SessionFixtures,\n"
            "    pub(crate) interaction: InteractionState,\n").replace(
            "mod session_core;", "mod interaction;\npub(in crate::session) use interaction::InteractionState;\n"
            "mod session_core;"))
        mod = self.root / "crates/wow-world/src/session/mod.rs"
        mod.write_text("mod battleground_adapter;\nmod npc_interaction;\n" + mod.read_text())
        self.write("crates/wow-world/src/session/battleground_adapter.rs", (
            "impl WorldSession {\n"
            "    #[cfg(test)]\n    pub(crate) fn bg_hellos(&self) -> usize {\n"
            "        self.fixtures.battleground.hellos.len()\n    }\n"
            "    pub(crate) fn bg_status(&self) -> u32 {\n        self.core.account_id\n    }\n}\n"))
        self.write("crates/wow-world/src/session/npc_interaction.rs", (
            "impl WorldSession {\n"
            "    pub(crate) fn source_plus(&self, x: u32) -> u32 {\n"
            "        self.interaction.source + self.core.account_id + x\n    }\n"
            "    pub(crate) fn test_only_body(&mut self) {\n        #[cfg(test)]\n        {\n"
            "            self.interaction.source = self.core.account_id;\n        }\n    }\n"
            "    pub(crate) fn source_from(&mut self, state: u32) {\n"
            "        self.interaction.source = state + self.core.account_id;\n    }\n"
            "    pub(crate) fn reset_source(&mut self) {\n"
            "        self.interaction.source = self.core.account_id;\n    }\n"
            "    pub(crate) fn reset_twice(&mut self) {\n        self.reset_source();\n        self.reset_source();\n    }\n"
            "}\n"))
        self.write("crates/wow-world/unit_tests/pins.rs",              # a source-text test pins the call
                   'const SRC: &str = include_str!("npc.rs");\nfn t() { assert!(SRC.contains("reset_source()")); }\n')
        conn = self.root / "crates/wow-world/src/session/connection.rs"
        conn.write_text(conn.read_text().replace(
            "            + self.with_mut(|x| x)\n",
            "            + self.with_mut(|x| x) + self.bg_status() + self.source_plus(1)\n").replace(
            "        self.ctx_only()\n", "        self.ctx_only() + self.bg_hellos() as u32\n").replace(
            "    pub fn account_plus_foo(&self)", "    pub fn reset(&mut self) {\n        self.reset_source();\n        self.reset_twice();\n        self.source_from(1);\n"
            "        self.test_only_body();\n    }\n"
            "    pub fn account_plus_foo(&self)"))
        self.write("crates/wow-world/src/session/state/interaction.rs",
                   "pub(in crate::session) struct InteractionState {\n    pub(crate) source: u32,\n}\n")
        self.write("crates/wow-world/src/handlers/vendor.rs", "impl WorldSession {\n"
                   "    pub(crate) fn vendor_source(&self) -> u32 {\n        self.interaction.source\n    }\n}\n")
        F.DOMAIN["handlers/vendor"] = "interaction"
        self.addCleanup(F.DOMAIN.pop, "handlers/vendor")
        conn.write_text(conn.read_text().replace("self.source_plus(1)", "self.source_plus(1) + self.vendor_source()"))
        P = F.plan(self.root, {"battleground", "interaction"}, {"P", "C-hub"})
        self.assertEqual(P["blocked"]["reset_twice"], "source-text test pins the call `reset_source(`")
        self.assertTrue(P["blocked"]["test_only_body"].startswith("uses its hub only under cfg(test)"))
        rc, out = run("apply", "--group", "battleground,interaction", "--root", str(self.root), "--text-only",
                      "--demote-blocked")
        self.assertEqual(rc, 0, out)
        bg = (self.root / "crates/wow-world/src/session/battleground_adapter.rs").read_text()
        self.assertIn("#[cfg(test)]\nimpl crate::session::state::BattlegroundState {\n    #[cfg(test)]\n"
                      "    pub(crate) fn bg_hellos(&self) -> usize {\n        self.hellos.len()\n", bg)
        shim = (self.root / "crates/wow-world/unit_tests/session/battleground_adapter/f3_shims.rs").read_text()
        self.assertIn("    #[cfg(test)]\n    pub(crate) fn bg_hellos(&self) -> usize {\n"
                      "        self.fixtures.battleground.bg_hellos()\n", shim)     # cfg(test) caller: a shim
        self.assertIn("impl crate::session::HubRef<'_> {\n    pub(crate) fn bg_status(&self)", bg)
        npc = (self.root / "crates/wow-world/src/session/npc_interaction.rs").read_text()
        self.assertIn("fn source_plus(&self, hub: crate::session::HubRef<'_>, x: u32) -> u32 {\n"
                      "        self.source + hub.core.account_id + x\n", npc)
        self.assertIn("let (state, hub) = crate::session::split_interaction_ref(self);\n"
                      "        state.source_plus(hub, x)\n", npc)
        self.assertIn("let (state, mut hub) = crate::session::split_interaction_mut(self);\n"
                      "        state.reset_source(&mut hub)\n", npc)
        self.assertIn("let (owner, mut hub) = crate::session::split_interaction_mut(self);\n"
                      "        owner.source_from(&mut hub, state)\n", npc)        # a `state` param is never shadowed
        self.assertIn("fn reset_source(&mut self, hub: &mut crate::session::HubMut<'_>) {\n"
                      "        self.source = hub.core.account_id;\n", npc)
        self.assertIn("pub(crate) struct InteractionState {",            # widened: named from handlers/
                      (self.root / "crates/wow-world/src/session/state/interaction.rs").read_text())
        npc_file = self.root / "crates/wow-world/src/session/npc_interaction.rs"   # a later caller of a moved
        npc_file.write_text(npc_file.read_text() + "impl WorldSession {\n"          # group fn passes the hub too
                            "    pub(crate) fn later(&self) -> u32 {\n        self.interaction.source + self.source_plus(2)\n"
                            "    }\n}\n")
        conn.write_text(conn.read_text().replace("self.vendor_source()", "self.vendor_source() + self.later()"))
        self.assertEqual(run("apply", "--group", "interaction", "--root", str(self.root), "--text-only",
                             "--demote-blocked")[0], 0)
        self.assertIn("self.source + self.source_plus(hub, 2)", npc_file.read_text())
        hub = (self.root / "crates/wow-world/src/session/state/hub.rs").read_text()
        self.assertIn("pub(crate) fn split_interaction_mut(s: &mut WorldSession) -> (&mut InteractionState, HubMut<'_>)", hub)
        self.assertIn("pub(crate) fn split_interaction_ref(s: &WorldSession)", hub)
        exports = (self.root / "crates/wow-world/src/session/mod.rs").read_text()
        self.assertIn("split_interaction_mut, split_interaction_ref};", exports)
        mod = self.root / "crates/wow-world/src/session/mod.rs"           # rustfmt-wrapped export line
        mod.write_text(mod.read_text().replace("use state::{HubMut", "use state::{\n    HubMut"))
        F.ensure_prelude(self.root / "crates/wow-world/src", {"hubref"}, set(), set())
        self.assertEqual(mod.read_text().count("use state::{"), 1)   # rewritten, never duplicated

    def write(self, rel, text):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def test_guard_across_self_call_is_a_precondition(self):
        self.assertTrue(F.guard_call("let g = self.core.m.lock().unwrap();\n self.other();\n g.x"))
        self.assertFalse(F.guard_call("let g = self.core.m.lock().unwrap();\n drop(g);\n self.other();"))
        self.assertFalse(F.guard_call("{ let g = m.write().unwrap(); g.x(); }\n self.other();"))
        self.assertFalse(F.guard_call("let r = { let g = m.write().unwrap(); 1 };\n self.other();"))

    def test_shims_avoid_gitignored_dirs(self):
        import subprocess
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        (self.root / ".gitignore").write_text("ops/\n")             # like the repo-wide `skills/` rule
        self.assertEqual(self.apply("--demote-blocked")[0], 0)
        shim = self.root / "crates/wow-world/unit_tests/session/catalogs/ops_f3_shims.rs"
        self.assertIn("fn only_tests", shim.read_text())
        self.assertIn('#[path = "../../../unit_tests/session/catalogs/ops_f3_shims.rs"]', self.ops.read_text())
        old = self.root / "crates/wow-world/unit_tests/session/catalogs/ops/f3_shims.rs"   # relocation
        old.parent.mkdir(parents=True)
        old.write_text(shim.read_text())
        shim.unlink()
        self.ops.write_text(self.ops.read_text().replace("ops_f3_shims.rs", "ops/f3_shims.rs"))
        F.relocate_ignored_shims(self.root, self.root / "crates/wow-world/src")
        self.assertTrue(shim.exists() and not old.exists())
        self.assertIn("ops_f3_shims.rs", self.ops.read_text())
        (self.root / ".gitignore").write_text("/ops/\n")             # the rule is later root-anchored
        self.assertEqual(F.relocate_ignored_shims(self.root, self.root / "crates/wow-world/src"), [])
        self.assertEqual(F.shim_path(self.root, "session/catalogs/ops.rs"), shim)   # the mount wins

    def test_compiler_loop_removes_shims_rustc_reports_unused(self):
        self.assertEqual(self.apply("--demote-blocked")[0], 0)
        shim = self.root / "crates/wow-world/unit_tests/session/catalogs/ops/f3_shims.rs"
        rel = shim.relative_to(self.root).as_posix()
        replies = iter([(0, [{"level": "warning", "message": "method `only_tests` is never used", "code": None,
                              "spans": [{"is_primary": True, "file_name": rel}]}]), (0, [])])
        original = F.cargo_check
        F.cargo_check = lambda root, log: next(replies)
        self.addCleanup(setattr, F, "cargo_check", original)
        manifest = dict(unthunked=[], restored=[])
        with redirect_stderr(io.StringIO()):
            self.assertEqual(F.compile_loop(self.root, "catalogs", manifest, 3, self.root), [1, 0])
        self.assertNotIn("fn only_tests", shim.read_text())
        self.assertEqual(manifest["retired_shims"], ["only_tests"])

    def test_rehome_overrides_the_derived_target(self):
        self.assertNotIn("foo_store", F.plan(self.root, {"core"}, {"P", "C-hub"})["cand"])
        P = F.plan(self.root, {"core"}, {"P", "C-hub"}, {"foo_store": "core"})
        self.assertIn("foo_store", P["cand"])                        # now planned under the core run
        with self.assertRaises(F.CodemodError):
            F.plan(self.root, {"core"}, {"P", "C-hub"}, {"no_such_fn": "core"})
        rc, out = run("plan", "--group", "core", "--root", str(self.root), "--rehome", "foo_store=core")
        self.assertEqual(rc, 0, out)
        self.assertIn("foo_store", out)

    def test_receiver_aware_callers_and_dead_only_callers(self):
        self.write("crates/wow-world/src/session/catalogs/more.rs", (
            "impl WorldSession {\n"
            "    pub(crate) fn collide(&self) -> u32 {\n        self.catalogs.bar\n    }\n"
            "    pub(crate) fn served(&self) -> u32 {\n        self.catalogs.bar\n    }\n"
            "    pub(crate) fn live_user(&self) -> u32 {\n        self.served() + self.catalogs.bar\n    }\n}\n"))
        conn = self.root / "crates/wow-world/src/session/connection.rs"
        conn.write_text(conn.read_text().replace(
            "    pub fn account_plus_foo(&self) -> u32 {\n",
            "    fn dead_user(&self) -> u32 {\n        self.served()\n    }\n"
            "    pub fn other_collide(&self, other: &Other) -> u32 {\n        other.collide() + self.live_user()\n    }\n"
            "    pub fn account_plus_foo(&self) -> u32 {\n"))
        tests = self.root / "crates/wow-world/unit_tests/catalogs_tests.rs"
        tests.write_text(tests.read_text() + "fn c(x: &WorldSession) -> u32 { x.collide() }\n")
        P = self.plan()
        rows = {r["name"]: r for r in P["rows"]}
        self.assertEqual((rows["collide"]["thunk"], bool(rows["collide"]["shim"])), ("", True))  # `other.` ignored
        self.assertEqual(P["blocked"]["served"], "only dead-in-production callers remain; its thunk would be dead")
        self.assertEqual(P["blocked"]["live_user"], "callee served not moved")
        self.write("crates/wow-world/src/session/catalogs/third.rs", (     # a dead lint group with an uncalled fn
            "impl WorldSession {\n    fn t3_used(&self) -> u32 {\n        self.catalogs.bar\n    }\n"
            "    fn t3_idle(&self) -> u32 {\n        self.catalogs.bar\n    }\n}\n"))
        tests.write_text(tests.read_text() + "fn d(x: &WorldSession) -> u32 { x.t3_used() }\n")
        P = self.plan()
        self.assertTrue(P["blocked"]["t3_idle"].startswith("no callers anywhere"))
        self.assertTrue(P["blocked"]["t3_used"].startswith("dead in non-test builds"))  # group stays whole

    def test_keep_holds_a_reviewed_fence_on_worldsession(self):
        P = F.plan(self.root, {"catalogs"}, {"P", "C-hub"}, None, {"foo_store"})
        self.assertTrue(P["blocked"]["foo_store"].startswith("kept on WorldSession by review"))
        self.assertNotIn("foo_store", P["cand"])
        rc, out = run("plan", "--group", "catalogs", "--root", str(self.root), "--keep", "foo_store")
        self.assertEqual(rc, 0, out)
        self.assertIn("kept on WorldSession by review", out)

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
        self.assertIn("fn with_mut(&self, f: impl FnMut(u32) -> u32) -> u32 {", text)  # thunk drops `mut`
        self.assertIn("fn with_mut(&self, mut f: impl FnMut(u32) -> u32) -> u32 {", moved)
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
        tests = self.root / "crates/wow-world/unit_tests/catalogs_tests.rs"     # the last shim caller goes
        tests.write_text(tests.read_text().replace("session.only_tests() + ", ""))
        shim = self.root / "crates/wow-world/unit_tests/session/catalogs/ops/f3_shims.rs"
        self.assertEqual(list(self.plan()["stale_shims"]), ["only_tests"])
        self.assertEqual(self.apply("--demote-blocked")[0], 0)
        self.assertNotIn("fn only_tests", shim.read_text())
        self.assertIn("fn ctx_only", shim.read_text())
        self.assertEqual(self.plan()["stale_shims"], {})


if __name__ == "__main__":
    unittest.main()

"""Fixture-graph tests for the #1363 wow-world closure check."""
import contextlib
import io
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import wow_world_closure as closure  # noqa: E402

LOCK = """
version = 4

[[package]]
name = "wow-world-core"
version = "0.1.0"

[[package]]
name = "wow-world-inventory"
version = "0.1.0"
dependencies = [
 "tracing",
 "wow-world-core",
 "wow-world-inventory",
]

[[package]]
name = "wow-world-social"
version = "0.1.0"
dependencies = [
 "wow-world-core",
 "wow-world-inventory",
]

[[package]]
name = "wow-world-application"
version = "0.1.0"
dependencies = [
 "wow-world-social",
]

[[package]]
name = "wow-data"
version = "0.1.0"
dependencies = [
 "wow-world-core",
]
"""


def metadata_package(name, *deps):
    return {"name": name, "dependencies": [{"name": dep, "kind": kind} for dep, kind in deps]}


class ClosureTests(unittest.TestCase):
    def test_lock_edges_keep_wow_world_packages_and_drop_self_dependency(self):
        edges = closure.lock_edges(LOCK)
        self.assertEqual(set(edges), {"wow-world-core", "wow-world-inventory",
                                      "wow-world-social", "wow-world-application"})
        self.assertEqual(edges["wow-world-inventory"], {"wow-world-core"})
        self.assertEqual(closure.closure(edges, "wow-world-application"),
                         {"wow-world-social", "wow-world-inventory", "wow-world-core"})
        self.assertEqual(closure.violations(edges), [])

    def test_metadata_edges_exclude_dev_dependencies(self):
        edges = closure.metadata_edges({"packages": [
            metadata_package("wow-world-spell", ("wow-world-core", None), ("wow-world-social", "dev"),
                             ("wow-world-spell", None)),
            metadata_package("wow-world-loot", ("wow-world-core", "build"), ("serde", None)),
        ]})
        self.assertEqual(edges, {"wow-world-spell": {"wow-world-core"},
                                 "wow-world-loot": {"wow-world-core"}})

    def test_simulated_edge_closing_a_cycle_is_rejected(self):
        edges = closure.lock_edges(LOCK)
        closure.add_edges(edges, ["wow-world-inventory->wow-world-social"])
        found = closure.violations(edges)
        self.assertIn("cycle: wow-world-inventory reaches itself", found)
        self.assertIn("cycle: wow-world-social reaches itself", found)
        self.assertIn("wow-world-inventory reaches ['wow-world-social']", found)

    def test_domain_crate_reaching_the_top_is_rejected(self):
        edges = {"wow-world-core": set(), "wow-world-lifecycle": {"wow-world-core"},
                 "wow-world-application": {"wow-world-lifecycle"}}
        self.assertEqual(closure.violations(edges), [])
        closure.add_edges(edges, ["wow-world-lifecycle->wow-world"])
        self.assertEqual(closure.violations(edges),
                         ["wow-world-lifecycle reaches ['wow-world']"])

    def test_low_crate_reaching_instances_transitively_is_rejected(self):
        edges = {"wow-world-entities": {"wow-world-interaction"},
                 "wow-world-interaction": {"wow-world-instances"},
                 "wow-world-instances": set()}
        self.assertEqual(closure.violations(edges),
                         ["wow-world-entities reaches ['wow-world-instances']"])

    def test_only_the_top_may_reach_the_character_crate(self):
        edges = {"wow-world-core": set(), "wow-world-inventory": {"wow-world-core"},
                 "wow-world-character": {"wow-world-inventory"},
                 "wow-world-application": {"wow-world-character"},
                 "wow-world": {"wow-world-application", "wow-world-character"}}
        self.assertEqual(closure.violations(edges), [])
        closure.add_edges(edges, ["wow-world-core->wow-world-character"])
        self.assertIn("wow-world-core reaches ['wow-world-character']", closure.violations(edges))
        edges = {"wow-world-character": set(), "wow-world-lifecycle": set(),
                 "wow-world-social": {"wow-world-lifecycle"}}
        closure.add_edges(edges, ["wow-world-lifecycle->wow-world-character"])
        self.assertEqual(closure.violations(edges),
                         ["wow-world-lifecycle reaches ['wow-world-character']",
                          "wow-world-social reaches ['wow-world-character']"])

    def test_only_the_top_and_loot_ops_may_reach_the_quest_crate(self):
        edges = {"wow-world-core": set(), "wow-world-inventory": {"wow-world-core"},
                 "wow-world-quest": {"wow-world-inventory"},
                 "wow-world-loot-ops": {"wow-world-quest"},
                 "wow-world-application": {"wow-world-quest", "wow-world-loot-ops"},
                 "wow-world": {"wow-world-application", "wow-world-quest"}}
        self.assertEqual(closure.violations(edges), [])
        closure.add_edges(edges, ["wow-world-character->wow-world-quest"])
        self.assertEqual(closure.violations(edges),
                         ["wow-world-character reaches ['wow-world-quest']"])
        edges = {"wow-world-quest": set(), "wow-world-loot": set(),
                 "wow-world-instances": {"wow-world-loot"}}
        closure.add_edges(edges, ["wow-world-loot->wow-world-quest"])
        self.assertEqual(closure.violations(edges),
                         ["wow-world-instances reaches ['wow-world-quest']",
                          "wow-world-loot reaches ['wow-world-quest']"])

    def test_only_the_top_may_reach_loot_ops(self):
        edges = {"wow-world-quest": set(), "wow-world-loot": set(),
                 "wow-world-loot-ops": {"wow-world-quest", "wow-world-loot"},
                 "wow-world-application": {"wow-world-loot-ops"},
                 "wow-world": {"wow-world-application", "wow-world-loot-ops"}}
        self.assertEqual(closure.violations(edges), [])
        closure.add_edges(edges, ["wow-world-quest->wow-world-loot-ops"])
        self.assertEqual(closure.violations(edges),
                         ["cycle: wow-world-loot-ops reaches itself",
                          "cycle: wow-world-quest reaches itself",
                          "wow-world-quest reaches ['wow-world-loot-ops']"])
        edges = {"wow-world-loot-ops": set(), "wow-world-loot": set(),
                 "wow-world-instances": {"wow-world-loot"}}
        closure.add_edges(edges, ["wow-world-loot->wow-world-loot-ops"])
        self.assertEqual(closure.violations(edges),
                         ["wow-world-instances reaches ['wow-world-loot-ops']",
                          "wow-world-loot reaches ['wow-world-loot-ops']"])

    def test_only_quest_and_above_may_reach_the_player_conditions_crate(self):
        conditions = "wow-world-player-conditions"
        edges = {"wow-world-core": set(), "wow-world-inventory": {"wow-world-core"},
                 conditions: {"wow-world-inventory"},
                 "wow-world-quest": {conditions},
                 "wow-world-loot-ops": {"wow-world-quest", conditions},
                 "wow-world-application": {"wow-world-quest", conditions},
                 "wow-world": {"wow-world-application", conditions}}
        self.assertEqual(closure.violations(edges), [])
        closure.add_edges(edges, [f"{conditions}->wow-world-quest"])
        self.assertEqual(closure.violations(edges),
                         ["cycle: wow-world-player-conditions reaches itself",
                          "wow-world-player-conditions reaches ['wow-world-quest']",
                          "cycle: wow-world-quest reaches itself"])
        edges = {conditions: set(), "wow-world-lifecycle": set(),
                 "wow-world-character": {"wow-world-lifecycle"}}
        closure.add_edges(edges, [f"wow-world-lifecycle->{conditions}"])
        self.assertEqual(closure.violations(edges),
                         ["wow-world-character reaches ['wow-world-player-conditions']",
                          "wow-world-lifecycle reaches ['wow-world-player-conditions']"])

    def test_malformed_planned_edge_is_an_error(self):
        with self.assertRaises(ValueError):
            closure.add_edges({}, ["wow-world-inventory"])

    def test_check_reads_cargo_lock_and_sets_exit_status(self):
        with tempfile.TemporaryDirectory(prefix="wow-world-closure-") as scratch:
            repo = pathlib.Path(scratch)
            (repo / "Cargo.lock").write_text(LOCK, encoding="utf-8")
            with contextlib.redirect_stdout(io.StringIO()) as out:
                self.assertEqual(closure.main(["check", "--repo", str(repo)]), 0)
            self.assertIn("PASS", out.getvalue())
            with contextlib.redirect_stdout(io.StringIO()) as out:
                self.assertEqual(closure.main(["check", "--repo", str(repo), "--add",
                                               "wow-world-core->wow-world-application"]), 1)
            self.assertIn("cycle: wow-world-core reaches itself", out.getvalue())


if __name__ == "__main__":
    unittest.main()

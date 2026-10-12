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

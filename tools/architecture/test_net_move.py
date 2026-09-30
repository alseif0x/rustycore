"""Tests for the R1 net-move check, against temporary git repositories."""
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import net_move  # noqa: E402

SCRIPT = HERE / "net_move.py"


class NetMoveTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="rustycore-net-move-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        self.git("init", "-q")
        self.git("config", "user.email", "net-move@example.invalid")
        self.git("config", "user.name", "Net Move")
        self.write("crates/wow-world/src/loot.rs", 1000)
        self.write("crates/wow-world/src/lib.rs", 10)
        self.write("crates/wow-map/src/lib.rs", 10)
        self.commit("base")
        self.base = self.git("rev-parse", "HEAD").strip()

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, check=True,
                              capture_output=True, text=True).stdout

    def write(self, path, lines, marker="line"):
        destination = self.root / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text("".join(f"// {marker} {i}\n" for i in range(lines)), encoding="utf-8")

    def remove(self, path):
        (self.root / path).unlink()

    def commit(self, message):
        self.git("add", "-A")
        self.git("commit", "-qm", message)

    def run_check(self, *extra):
        result = subprocess.run([sys.executable, str(SCRIPT), "check", "--base", self.base, "--json", *extra],
                                cwd=self.root, capture_output=True, text=True)
        report = json.loads(result.stdout) if result.returncode in (0, 1) else None
        return result.returncode, report, result.stderr

    def test_pure_move_passes(self):
        self.remove("crates/wow-world/src/loot.rs")
        self.write("crates/wow-world-loot/src/lib.rs", 1000)
        self.commit("move loot")
        code, report, _ = self.run_check()
        self.assertEqual((code, report["verdict"]), (0, "pass"))
        self.assertEqual((report["shrink"], report["growth"]), (1000, 1000))
        self.assertEqual(report["top_growth"], [{"path": "crates/wow-world-loot/src/lib.rs", "delta": 1000}])

    def test_moving_tests_out_of_src_counts_as_growth(self):
        self.remove("crates/wow-world/src/loot.rs")
        self.write("crates/wow-world/tests/loot.rs", 1000)
        self.commit("tests out of src")
        code, report, _ = self.run_check()
        self.assertEqual((code, report["shrink"], report["growth"]), (0, 1000, 1000))

    def test_copy_fails_with_exact_excess(self):
        self.write("crates/wow-world/src/loot.rs", 900)  # only 100 lines deleted
        self.write("crates/wow-map/src/loot.rs", 1000)
        self.commit("copy loot")
        code, report, _ = self.run_check()
        self.assertEqual((code, report["verdict"]), (1, "fail"))
        self.assertEqual((report["shrink"], report["growth"]), (100, 1000))
        self.assertAlmostEqual(report["allowance"], 405.0)
        self.assertIn("growth outside crates/wow-world/src/ is 1000 lines", report["message"])
        self.assertIn("exceeding the allowance 405.0", report["message"])
        self.assertIn("by 595.0 lines", report["message"])
        text = subprocess.run([sys.executable, str(SCRIPT), "check", "--base", self.base],
                              cwd=self.root, capture_output=True, text=True)
        self.assertEqual(text.returncode, 1)
        for needle in ("S (shrink of crates/wow-world/src/): 100", "G (net growth elsewhere): 1000",
                       "allowance: 405.0", "+1000 crates/wow-map/src/loot.rs", "FAIL: R1 net-move"):
            self.assertIn(needle, text.stdout)

    def test_wow_world_growth_is_not_a_move(self):
        self.write("crates/wow-world/src/feature.rs", 500)
        self.write("crates/wow-map/src/feature.rs", 5000)
        self.commit("feature")
        code, report, _ = self.run_check()
        self.assertEqual((code, report["verdict"], report["shrink"]), (0, "not-applicable", -500))
        self.assertIn("not a move", report["message"])

    def test_uncommitted_and_untracked_changes_count(self):
        self.remove("crates/wow-world/src/loot.rs")  # unstaged deletion
        code, report, _ = self.run_check()
        self.assertEqual((code, report["shrink"], report["growth"]), (0, 1000, 0))
        self.write("crates/wow-map/src/loot.rs", 1000)  # untracked
        self.write("crates/wow-entities/src/loot.rs", 1000)
        self.git("add", "crates/wow-entities/src/loot.rs")  # staged
        code, report, _ = self.run_check()
        self.assertEqual((code, report["verdict"], report["growth"]), (1, "fail", 2000))

    def test_ignored_and_non_rust_files_are_not_counted(self):
        (self.root / ".gitignore").write_text("target/\n", encoding="utf-8")
        self.remove("crates/wow-world/src/loot.rs")
        self.write("target/debug/build/out.rs", 5000)
        self.write("docs/loot.md", 5000)
        code, report, _ = self.run_check()
        self.assertEqual((code, report["shrink"], report["growth"]), (0, 1000, 0))

    def test_slack_and_ratio_boundaries(self):
        # S = 1000, default allowance = 1000 * 1.05 + 300 = 1350.
        self.remove("crates/wow-world/src/loot.rs")
        self.write("crates/wow-map/src/loot.rs", 1350)
        self.commit("at boundary")
        code, report, _ = self.run_check()
        self.assertEqual((code, report["growth"], report["allowance"]), (0, 1350, 1350.0))
        self.write("crates/wow-map/src/loot.rs", 1351)
        code, report, _ = self.run_check()
        self.assertEqual((code, report["verdict"]), (1, "fail"))
        self.assertEqual(self.run_check("--slack", "301")[0], 0)
        self.assertEqual(self.run_check("--ratio", "0", "--slack", "351")[0], 0)
        self.assertEqual(self.run_check("--ratio", "0", "--slack", "350")[0], 1)
        self.assertEqual(self.run_check("--ratio", "0.051")[0], 0)

    def test_merge_base_is_used(self):
        self.git("checkout", "-qb", "feature")
        self.remove("crates/wow-world/src/loot.rs")
        self.write("crates/wow-map/src/loot.rs", 1000)
        self.commit("move on feature")
        self.git("checkout", "-q", "-")
        self.write("crates/other/src/lib.rs", 9000)
        self.commit("unrelated upstream growth")
        upstream = self.git("rev-parse", "HEAD").strip()
        self.git("checkout", "-q", "feature")
        fork = self.base
        self.base = upstream
        code, report, _ = self.run_check()
        self.assertEqual((code, report["growth"], report["merge_base"]), (0, 1000, fork))

    def test_generated_policy_paths_are_excluded(self):
        self.assertEqual(net_move.evaluate({"crates/wow-world/src/a.rs": -10, "crates/gen/src/out.rs": 9000},
                                           {"crates/gen/src/out.rs"}, 0.05, 300)["growth"], 0)

    def test_usage_and_git_errors_exit_2(self):
        self.assertEqual(self.run_check("--ratio", "-1")[0], 2)
        self.base = "no-such-revision"
        code, _, stderr = self.run_check()
        self.assertEqual(code, 2)
        self.assertIn("net_move:", stderr)
        missing = subprocess.run([sys.executable, str(SCRIPT), "check"], cwd=self.root, capture_output=True)
        self.assertEqual(missing.returncode, 2)


if __name__ == "__main__":
    unittest.main()

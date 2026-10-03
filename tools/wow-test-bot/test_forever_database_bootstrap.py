"""Pure bootstrap guards: no Docker, database, network, archive or account."""
import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("forever_database_bootstrap.py")
SPEC = importlib.util.spec_from_file_location("forever_bootstrap", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class BootstrapGuards(unittest.TestCase):
    def test_applied_name_is_not_proof_without_the_exact_source_hash(self):
        sql = b"SELECT 1;"
        self.assertTrue(MODULE.update_needed("fixture.sql", sql, {}))
        self.assertFalse(MODULE.update_needed("fixture.sql", sql, {"fixture.sql": MODULE.hashlib.sha1(sql).hexdigest()}))
        with self.assertRaises(RuntimeError):
            MODULE.update_needed("fixture.sql", sql, {"fixture.sql": "0" * 40})

    def test_source_paths_follow_filename_not_directory_order(self):
        paths = ["sql/custom/world/2026_09_27_00_world.sql", "sql/updates/world/master/2026_09_10_00_world.sql",
                 "sql/custom/hotfixes/2026_09_01_00_hotfixes.sql", "sql/custom/world/.gitignore"]
        self.assertEqual(MODULE.update_paths(paths, "world"), [paths[1], paths[0]])

    def test_duplicate_update_filename_is_not_applied_twice(self):
        paths = ["sql/custom/world/same.sql", "sql/updates/world/master/same.sql"]
        with self.assertRaises(RuntimeError):
            MODULE.update_paths(paths, "world")

    def test_explicit_source_schema_is_rebound_only_in_sql_code(self):
        sql = b"DELETE FROM world.conditions; INSERT INTO `world`.gossip_menu VALUES ('world.untouched'); -- world.comment\n"
        adapted = MODULE.adapt_schema(sql, "world")
        self.assertIn(b"DELETE FROM `world_forever_70170_02245`.conditions", adapted)
        self.assertIn(b"INSERT INTO `world_forever_70170_02245`.gossip_menu", adapted)
        self.assertIn(b"'world.untouched'", adapted)
        self.assertIn(b"-- world.comment", adapted)

    def test_comments_and_descriptions_do_not_look_like_schema_switches(self):
        MODULE.validate_sql(b"-- USE auth;\nINSERT INTO notes VALUES ('world. example; USE auth;', 'characters.fake'); /* USE auth; */")

    def test_schema_switches_including_conditional_directives_fail_closed(self):
        for sql in [b"USE auth;", b"SELECT 1; USE world;", b"DROP DATABASE auth;", b"CREATE DATABASE test;",
                    b"SOURCE /tmp/unsafe.sql;", b"/*!40101 USE auth */;", b"DELETE FROM auth.account;",
                    b"INSERT INTO `characters`.characters VALUES (1);"]:
            with self.subTest(sql=sql), self.assertRaises(RuntimeError):
                MODULE.validate_sql(sql)

    def test_rebinding_world_does_not_authorize_auth_changes(self):
        with self.assertRaises(RuntimeError):
            MODULE.adapt_schema(b"DELETE FROM auth.account;", "world")

    def test_child_error_is_metadata_only_even_when_mysql_dumps_secret_sql(self):
        result = subprocess.CompletedProcess([], 1, b"", b"ERROR 1064 (42000) at line 17: password=never-render-this")
        with patch.object(MODULE.subprocess, "run", return_value=result):
            with self.assertRaises(RuntimeError) as caught:
                MODULE.command(["not-executed"])
        self.assertIn("mysql_code=1064", str(caught.exception))
        self.assertIn("line=17", str(caught.exception))
        self.assertNotIn("never-render", str(caught.exception))


if __name__ == "__main__":
    unittest.main()

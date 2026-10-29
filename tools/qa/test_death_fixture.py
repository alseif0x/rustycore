"""Hermetic #43 death fixture policy tests. No MariaDB or service access."""

import contextlib
import io
import json
import os
from pathlib import Path
import re
import tempfile
import unittest

import death_fixture as fixture


class FakeDB:
    def __init__(self):
        self.target_database = {"host": "127.0.0.1", "port": "3306", "schema": "characters"}
        self.seeded = False
        self.fail_transaction = None
        self.wrong_item_owner = False
        self.statements = []
        self.on_transaction = None
        self.verify_calls = 0
        self.snapshot_calls = 0
        self.account_rows = [
            ["9", "TESTBOT2@bot.local"], ["10", "TESTBOT3@bot.local"],
            ["12", "TESTBOT5@bot.local"],
        ]

    def inspect(self):
        return {
            "accounts": [row.copy() for row in self.account_rows],
            "characters": [["15", "9", "OriginalPal", "0", "0"],
                           ["16", "10", "OriginalMage", "0", "0"],
                           ["18", "12", "OriginalHunter", "0", "0"]],
            "online_count": 0, "name_collisions": [], "max_character_id": 30,
            "max_item_id": 90, "unsupported_existing_rows": [],
        }

    def assert_ids_unoccupied(self, char_ids, item_ids):
        self.asserted_ids = (list(char_ids), list(item_ids))

    def transaction(self, statements):
        self.statements.extend(statements)
        if self.on_transaction:
            self.on_transaction()
        if self.fail_transaction:
            if self.fail_transaction == "commit-then-error":
                self.seeded = True
            raise RuntimeError("unknown transaction outcome")
        if any(statement.startswith("INSERT INTO characters ") for statement in statements):
            self.seeded = True
        else:
            self.seeded = False

    def rows_under_ids(self, journal):
        empty = {f"{table}.{owner}": [] for table, owner in fixture.CHAR_OWNER_TABLES}
        empty.update({f"{table}.{owner}": [] for table, owner in fixture.ITEM_CHILD_TABLES})
        empty["item_instance.guid"] = []
        empty["character_inventory.item"] = []
        if self.seeded:
            empty["characters.guid"] = [[str(actor["guid"])] for actor in journal["actors"]]
            empty["item_instance.guid"] = [
                [str(item["guid"]), str(item["owner_guid"] + (1 if self.wrong_item_owner else 0))]
                for item in journal["items"]]
            empty["character_inventory.item"] = [
                [str(item["guid"]), str(item["owner_guid"])] for item in journal["items"]]
        return empty

    def unsupported_rows(self, journal):
        return {}

    def verify_accounts_and_offline(self, journal):
        self.verify_calls += 1
        return None

    def character_identities(self, journal):
        if not self.seeded:
            return []
        return [[str(actor["guid"]), str(actor["account_id"]), actor["name"], "0"]
                for actor in journal["actors"]]

    def snapshot(self, journal):
        self.snapshot_calls += 1
        return {"characters": {"columns": ["guid", "name"],
                               "rows": [[str(actor["guid"]), actor["name"]]
                                        for actor in journal["actors"]] if self.seeded else []}}


class DeathFixtureTests(unittest.TestCase):
    def private_journal_dir(self, root):
        path = Path(root) / "journal"
        path.mkdir(mode=0o700)
        path.chmod(0o700)
        return path

    def bot_config(self):
        return {"bots": [
            {"account": "TESTBOT2@bot.local", "account_id": 9, "character_guid": 15, "password": "secret"},
            {"account": "TESTBOT3@bot.local", "account_id": 10, "character_guid": 16, "password": "secret"},
            {"account": "TESTBOT5@bot.local", "account_id": 12, "character_guid": 18, "password": "secret"},
        ]}

    def prepare_fixture(self, db, directory):
        path = directory / "death-fixture.journal"
        record = fixture.prepare(db, self.bot_config(), path, ack=True, world_stopped=True)
        return path, record

    def maria_db_with_fake_execute(self, account_rows):
        # Bypass MariaDB.__init__; exercise its production verifier without credentials or a DB.
        db = object.__new__(fixture.MariaDB)
        db.statements = []

        def fake_execute(statement):
            db.statements.append(statement)
            if statement.startswith("SELECT a.id,b.email"):
                return "\n".join("\t".join(row) for row in account_rows)
            if statement.startswith("SELECT guid,account,online"):
                return "15\t9\t0\n16\t10\t0\n18\t12\t0"
            if statement == "SELECT COUNT(*) FROM characters WHERE online<>0":
                return "0"
            self.fail(f"unexpected query in account verification: {statement}")

        db.execute = fake_execute
        return db

    def test_rejects_identity_collisions_and_out_of_range_counters(self):
        info = FakeDB().inspect()
        info["name_collisions"] = [["0", "Qadeathfall"]]
        with self.assertRaises(fixture.FixtureError):
            fixture.validate_inspection(info)
        for email in ("wrong@bot.local", "TESTBOT2@bot.local.evil", "TÉSTBOT2@bot.local"):
            info = FakeDB().inspect()
            info["accounts"][0][1] = email
            with self.subTest(email=email), self.assertRaises(fixture.FixtureError):
                fixture.validate_inspection(info)
        info = FakeDB().inspect()
        info["accounts"][0][0] = "90"
        with self.assertRaises(fixture.FixtureError):
            fixture.validate_inspection(info)
        info = FakeDB().inspect()
        info["characters"][0][3] = "1"
        with self.assertRaises(fixture.FixtureError):
            fixture.validate_inspection(info)
        for counter in (0, fixture.MAX_COUNTER + 1, True):
            with self.assertRaises(fixture.FixtureError):
                fixture.checked_counter(counter)

    def test_plan_accepts_ascii_email_case_variants_for_exact_reviewed_ids(self):
        reviewed_rows = FakeDB().account_rows
        for change_case in (str.lower, str.upper):
            with self.subTest(case=change_case.__name__):
                db = FakeDB()
                db.account_rows = [[account_id, change_case(email)]
                                   for account_id, email in reviewed_rows]
                plan = fixture.safe_plan(db)
                self.assertEqual(plan["mode"], "read-only-plan")

    def test_mariadb_restore_verification_accepts_case_but_rejects_wrong_bindings(self):
        reviewed_rows = FakeDB().account_rows
        for change_case in (str.lower, str.upper):
            with self.subTest(case=change_case.__name__):
                rows = [[account_id, change_case(email)]
                        for account_id, email in reviewed_rows]
                db = self.maria_db_with_fake_execute(rows)
                db.verify_accounts_and_offline({"actors": []})
                self.assertEqual(len(db.statements), 3)
                self.assertTrue(all(statement.startswith("SELECT") for statement in db.statements))

        wrong_email_rows = [row.copy() for row in reviewed_rows]
        wrong_email_rows[0][1] = "TESTBOT3@bot.local"
        wrong_id_rows = [row.copy() for row in reviewed_rows]
        wrong_id_rows[0][0] = "90"
        for rows in (wrong_email_rows, wrong_id_rows):
            with self.subTest(rows=rows):
                db = self.maria_db_with_fake_execute(rows)
                with self.assertRaises(fixture.FixtureError):
                    db.verify_accounts_and_offline({"actors": []})

    def test_journal_is_durable_and_private_before_first_sql_write(self):
        with tempfile.TemporaryDirectory() as root:
            directory = self.private_journal_dir(root)
            db = FakeDB()
            journal_path = directory / "death-fixture.journal"

            def check_journal_precedes_transaction():
                self.assertTrue(journal_path.is_file())
                self.assertEqual(journal_path.stat().st_mode & 0o777, 0o600)
                journal = json.loads(journal_path.read_text())
                self.assertEqual(journal["state"], "prepare-attempted")
                self.assertEqual(len(journal["actors"]), 5)
                self.assertEqual(len(journal["items"]), 3)
                self.assertTrue(journal["prior_row_absence"]["character_ids"])
                self.assertEqual(journal["target_database"], db.target_database)
                self.assertNotIn("secret", journal_path.read_text())

            db.on_transaction = check_journal_precedes_transaction
            record = fixture.prepare(db, self.bot_config(), journal_path, ack=True, world_stopped=True)
            self.assertEqual(record["state"], "prepared")

    def test_rollback_and_unknown_commit_keep_recoverable_journal(self):
        for outcome in ("rollback", "commit-then-error"):
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as root:
                directory = self.private_journal_dir(root)
                db = FakeDB()
                db.fail_transaction = outcome
                journal_path = directory / "death-fixture.journal"
                with self.assertRaisesRegex(RuntimeError, "unknown transaction outcome"):
                    fixture.prepare(db, self.bot_config(), journal_path, ack=True, world_stopped=True)
                self.assertTrue(journal_path.is_file())
                self.assertEqual(fixture.read_journal(journal_path)["state"], "prepare-attempted")
                db.fail_transaction = None
                fixture.restore(db, journal_path, ack=True, world_stopped=True)
                self.assertTrue(Path(str(journal_path) + ".cleanup-complete").is_file())

    def test_restore_is_owned_and_idempotent(self):
        with tempfile.TemporaryDirectory() as root:
            directory = self.private_journal_dir(root)
            db = FakeDB()
            journal_path, _ = self.prepare_fixture(db, directory)
            cleanup_start = len(db.statements)
            fixture.restore(db, journal_path, ack=True, world_stopped=True)
            cleanup_members = db.statements[cleanup_start:]
            self.assertTrue(cleanup_members)
            self.assertTrue(all(statement.startswith("DELETE FROM ") for statement in cleanup_members))
            actor_ids = ",".join(str(actor["guid"]) for actor in fixture.read_journal(journal_path)["actors"])
            item_ids = ",".join(str(item["guid"]) for item in fixture.read_journal(journal_path)["items"])
            inventory_delete = (
                f"DELETE FROM character_inventory WHERE item IN ({item_ids}) AND guid IN ({actor_ids})")
            item_delete = (
                f"DELETE FROM item_instance WHERE guid IN ({item_ids}) AND owner_guid IN ({actor_ids})")
            self.assertIn(inventory_delete, cleanup_members)
            self.assertLess(cleanup_members.index(inventory_delete), cleanup_members.index(item_delete))
            marker = Path(str(journal_path) + ".cleanup-complete")
            self.assertTrue(marker.is_file())
            self.assertEqual(marker.stat().st_mode & 0o777, 0o600)
            self.assertEqual(fixture.read_journal(journal_path)["state"], "restored")
            self.assertEqual(len(fixture.read_journal(journal_path)["cleanup_snapshot"]["characters"]["rows"]), 5)
            self.assertEqual(db.snapshot_calls, 1)
            self.assertFalse(any(db.rows_under_ids(fixture.read_journal(journal_path)).values()))
            count = len(db.statements)
            fixture.restore(db, journal_path, ack=True, world_stopped=True)
            self.assertEqual(len(db.statements), count)

    def test_cli_prepare_prints_actor_projection_without_credentials(self):
        with tempfile.TemporaryDirectory() as root:
            directory = self.private_journal_dir(root)
            bot_config = Path(root) / "bot-config.json"
            bot_raw = self.bot_config()
            bot_raw["bots"][0]["password"] = "cli-bot-secret-927"
            bot_config.write_text(json.dumps(bot_raw), encoding="utf-8")
            world_config = Path(root) / "worldserver.conf"
            world_config.write_text(
                'CharacterDatabaseInfo = "127.0.0.1;3306;fixture-user;cli-db-secret-491;characters"\n',
                encoding="utf-8")
            journal_path = directory / "death-fixture.journal"
            db = FakeDB()
            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                status = fixture.main([
                    "prepare", "--bot-config", str(bot_config), "--world-config", str(world_config),
                    "--journal", str(journal_path), fixture.ACK_TEXT, "--world-stopped",
                ], db_factory=lambda _db_info: db)

            self.assertEqual(status, 0)
            result = json.loads(stdout.getvalue())
            self.assertEqual(result["mode"], "prepared")
            self.assertEqual(result["journal"], str(journal_path))
            self.assertEqual([actor["name"] for actor in result["actors"]],
                             [actor["name"] for actor in fixture.ACTORS])
            self.assertEqual([actor["guid"] for actor in result["actors"]], [31, 32, 33, 34, 35])
            self.assertNotIn("cli-bot-secret-927", stdout.getvalue() + stderr.getvalue())
            self.assertNotIn("cli-db-secret-491", stdout.getvalue() + stderr.getvalue())

    def test_wrong_item_owner_holds_restore_and_retains_journal(self):
        with tempfile.TemporaryDirectory() as root:
            directory = self.private_journal_dir(root)
            db = FakeDB()
            journal_path, _ = self.prepare_fixture(db, directory)
            db.wrong_item_owner = True
            prior_count = len(db.statements)
            with self.assertRaisesRegex(fixture.FixtureError, "owner identity drifted"):
                fixture.restore(db, journal_path, ack=True, world_stopped=True)
            self.assertTrue(journal_path.is_file())
            self.assertFalse(Path(str(journal_path) + ".cleanup-complete").exists())
            self.assertEqual(len(db.statements), prior_count)

    def test_seed_sql_has_no_account_npc_or_original_character_writes(self):
        with tempfile.TemporaryDirectory() as root:
            directory = self.private_journal_dir(root)
            db = FakeDB()
            _, journal = self.prepare_fixture(db, directory)
            allowed = {"characters", "character_homebind", "item_instance",
                       "character_inventory", "character_spell", "character_achievement"}
            tables = {statement.split(" INTO ", 1)[1].split(" ", 1)[0]
                      for statement in db.statements if statement.startswith("INSERT INTO ")}
            self.assertLessEqual(tables, allowed)
            self.assertEqual(len(tables), 6)
            self.assertTrue(all("auth." not in statement and "world." not in statement
                                for statement in db.statements))
            self.assertTrue(all(statement.startswith("INSERT INTO ") for statement in db.statements))
            self.assertFalse(any("guid=15" in statement or "guid=16" in statement or "guid=18" in statement
                                 for statement in db.statements))
            self.assertEqual({(item["actor"], item["spell"]) for item in fixture.SPELL_SEEDS}, {
                ("Qadeathcast", 7328), ("Qadeathcast", 7355), ("Qadeathcast", 8690),
                ("Qadeathreinc", 20608), ("Qadeathwisp", 20585),
            })
            actor_guids = {item["name"]: item["guid"] for item in journal["actors"]}
            spell_rows = {statement for statement in db.statements
                          if statement.startswith("INSERT INTO character_spell ")}
            self.assertEqual(spell_rows, {
                f"INSERT INTO character_spell (guid,spell,active,disabled) VALUES "
                f"({actor_guids[seed['actor']]},{seed['spell']},1,0)"
                for seed in fixture.SPELL_SEEDS
            })
            self.assertEqual(fixture.ACHIEVEMENT_SEEDS, (
                {"actor": "Qadeathfall", "achievement": 13},
                {"actor": "Qadeathreinc", "achievement": 13},
                {"actor": "Qadeathcast", "achievement": 13},
            ))
            self.assertEqual({seed["actor"] for seed in fixture.ACHIEVEMENT_SEEDS},
                             {actor["name"] for actor in journal["actors"] if actor["level"] == 80})
            self.assertEqual(journal["seed_facts"]["achievements"], list(fixture.ACHIEVEMENT_SEEDS))
            self.assertEqual(
                {statement for statement in db.statements
                 if statement.startswith("INSERT INTO character_achievement ")},
                {f"INSERT INTO character_achievement (guid,achievement,date) VALUES "
                 f"({actor_guids[seed['actor']]},13,UNIX_TIMESTAMP())"
                 for seed in fixture.ACHIEVEMENT_SEEDS},
            )
            self.assertIn(("character_achievement", "guid"), fixture.CHAR_OWNER_TABLES)

    def test_bad_private_config_error_does_not_echo_credential_text(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "config.json"
            path.write_text('{"password":"top-secret-value", invalid')
            with self.assertRaises(fixture.FixtureError) as caught:
                fixture.validate_private_bot_config(path)
            self.assertNotIn("top-secret-value", str(caught.exception))

    def test_actor_config_is_an_in_memory_copy_with_only_new_guid_selected(self):
        raw = self.bot_config()
        journal = fixture.seed_record([31, 32, 33, 34, 35], [91, 92, 93], [1, 1, 2, 1, 1],
                                      {"host": "127.0.0.1", "port": "3306", "schema": "characters"})
        configured = fixture.bot_config_for_actor(raw, journal, "Qadeathfall")
        self.assertEqual(len(configured["bots"]), 1)
        self.assertEqual(configured["bots"][0]["character_guid"], 31)
        self.assertEqual(configured["bots"][0]["password"], "secret")
        self.assertEqual(raw["bots"][1]["character_guid"], 16)

    def test_target_identity_excludes_credentials_and_mismatch_holds_before_queries(self):
        first = fixture.target_database_identity(("db.test", "3306", "user-a", "secret-a", "characters"))
        rotated = fixture.target_database_identity(("db.test", "3306", "user-b", "secret-b", "characters"))
        self.assertEqual(first, rotated)
        self.assertEqual(set(first), {"host", "port", "schema"})
        with tempfile.TemporaryDirectory() as root:
            directory = self.private_journal_dir(root)
            db = FakeDB()
            journal_path, _ = self.prepare_fixture(db, directory)
            before = len(db.statements)
            db.target_database = {"host": "other.test", "port": "3306", "schema": "characters"}
            with self.assertRaisesRegex(fixture.FixtureError, "target database differs"):
                fixture.restore(db, journal_path, ack=True, world_stopped=True)
            self.assertEqual(len(db.statements), before)
            self.assertEqual(db.verify_calls, 0)
            self.assertEqual(db.snapshot_calls, 0)

    @unittest.skipUnless("RUSTYCORE_REFERENCE_CPP_ROOT" in os.environ,
                         "set RUSTYCORE_REFERENCE_CPP_ROOT to run the pinned source-DDL check")
    def test_owned_table_columns_match_pinned_primary_characters_ddl(self):
        root = Path(os.environ["RUSTYCORE_REFERENCE_CPP_ROOT"])
        ddl_path = root / "sql/base/characters_database.sql"
        self.assertTrue(ddl_path.is_file(), f"primary DDL is missing under {root}")
        ddl = ddl_path.read_text(encoding="utf-8")
        tables = {}
        for match in re.finditer(r"CREATE TABLE `([^`]+)` \((.*?)\) ENGINE=", ddl, re.DOTALL):
            table, definition = match.groups()
            tables[table] = set(re.findall(r"^  `([^`]+)` ", definition, re.MULTILINE))
        owned_columns = set(fixture.CHAR_OWNER_TABLES)
        owned_columns.update(fixture.ITEM_OWNER_TABLES)
        owned_columns.update(fixture.ITEM_ID_TABLES)
        owned_columns.update(fixture.UNSUPPORTED_OWNER_TABLES)
        owned_columns.update(fixture.CHAR_GUID_REFERENCE_TABLES)
        owned_columns.update((table, owner) for table, owner, _ in fixture.SNAPSHOT_TABLES)
        for table, column in sorted(owned_columns):
            with self.subTest(table=table, column=column):
                self.assertIn(table, tables)
                self.assertIn(column, tables[table])


if __name__ == "__main__":
    unittest.main()

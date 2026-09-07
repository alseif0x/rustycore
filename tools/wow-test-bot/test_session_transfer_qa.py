"""Hermetic fixture recovery policy tests; no service or database access."""
import contextlib
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import run_session_transfer_qa as qa


class TransferFixtureTests(unittest.TestCase):
    def test_location_values_reject_sql_and_nonfinite(self):
        for invalid in ("NaN", "Infinity", "0; DELETE FROM characters", "'0'"):
            with self.assertRaises(Exception):
                qa.numeric((*qa.SOURCE[:6], invalid))

    def test_location_values_require_exact_field_count(self):
        with self.assertRaises(RuntimeError):
            qa.numeric(qa.SOURCE[:6])
        self.assertEqual(qa.numeric(qa.SOURCE), qa.SOURCE)

    def test_recovery_restores_only_admitted_fixture_before_start(self):
        for admitted in (True, False):
            with self.subTest(admitted=admitted), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "journal.json"
                qa.persist(path, {"schema": "session-transfer-585-v1", "identity": [14, 8],
                                  "original_location": qa.SOURCE, "original_executable": "hash",
                                  "admitted": admitted, "restored": False})
                calls = []
                with patch.object(qa, "runtime_lock", contextlib.nullcontext), \
                     patch.object(qa, "service", side_effect=lambda action: calls.append(action)), \
                     patch.object(qa, "preflight"), \
                     patch.object(qa, "write_location", side_effect=lambda _: calls.append("restore")), \
                     patch.object(qa, "digest", return_value="hash"), \
                     patch.object(qa, "wait_serving"):
                    qa.recover(path)
                    self.assertEqual(calls, ["stop", "restore", "start"] if admitted else ["stop", "start"])
                    qa.recover(path)
                    self.assertEqual(calls.count("start"), 1)

    def test_wrong_binary_is_never_restarted(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "journal.json"
            qa.persist(path, {"schema": "session-transfer-585-v1", "identity": [14, 8],
                              "original_location": qa.SOURCE, "original_executable": "expected",
                              "admitted": False, "restored": False})
            with patch.object(qa, "runtime_lock", contextlib.nullcontext), \
                 patch.object(qa, "service") as service, \
                 patch.object(qa, "digest", return_value="wrong"):
                with self.assertRaises(RuntimeError):
                    qa.recover(path)
                service.assert_called_once_with("stop")


if __name__ == "__main__":
    unittest.main()

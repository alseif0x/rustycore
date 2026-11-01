"""Hermetic #45 solo instance scenario checks."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

import death_fixture
import death_scenario
import instance_scenario as scenario


def actor() -> dict:
    selected = copy.deepcopy(next(item for item in death_fixture.ACTORS
                                    if item["name"] == scenario.ACTOR_NAME))
    selected["guid"] = 73
    return selected


def identity_row(selected: dict, online: str = "0") -> list[str]:
    return [str(selected["guid"]), str(selected["account_id"]), selected["name"], online]


def position_row(selected: dict, map_id: int, instance_id: int,
                 position: tuple[float, float, float], orientation: float = 0.0) -> str:
    x, y, z = position
    return "\t".join([
        str(selected["guid"]), str(selected["account_id"]), selected["name"], "0",
        str(map_id), str(instance_id), str(x), str(y), str(z), str(orientation),
    ])


def inside_state(selected: dict, instance_id: int = 41) -> dict:
    x, y, z = scenario.INSTANCE_POSITION
    return {
        "guid": selected["guid"], "online": 0, "logout_time": 1,
        "map": scenario.INSTANCE_MAP, "instance_id": instance_id,
        "position_x": x, "position_y": y, "position_z": z, "orientation": 0.0,
    }


def outside_state(selected: dict) -> dict:
    x, y, z = selected["home"]
    return {
        "guid": selected["guid"], "online": 0, "logout_time": 1,
        "map": selected["map"], "instance_id": 0,
        "position_x": x, "position_y": y, "position_z": z, "orientation": 0.0,
    }


def sent_reset(sequence: int = 10) -> dict:
    return {
        "sequence": 1, "observation_sequence": sequence, "action_index": 0,
        "action": "reset_instances", "connection": "instance",
        "opcode": scenario.CMSG_RESET_INSTANCES, "body_hex": "",
        "write_attempted": True, "write_completed": True, "write_error": None,
    }


def reset_event(sequence: int = 11, body: bytes | None = None,
                connection: str = "instance") -> dict:
    if body is None:
        body = scenario.INSTANCE_MAP.to_bytes(4, "little")
    return {
        "sequence": 2, "observation_sequence": sequence, "connection": connection,
        "opcode": scenario.SMSG_INSTANCE_RESET, "body_hex": body.hex(),
    }


class InstancePlanTests(unittest.TestCase):
    def test_observer_and_reset_plans_use_native_logout_and_exact_map_body(self) -> None:
        selected = actor()
        for plan in (scenario.observer_plan(selected), scenario.reset_plan(selected)):
            self.assertEqual(plan["character_guid"], selected["guid"])
            self.assertEqual(plan["finish_policy"], "logout")
        self.assertTrue(scenario.observer_plan(selected)["observe_only"])
        plan = scenario.reset_plan(selected)
        self.assertFalse(plan.get("observe_only", False))
        self.assertEqual(plan["actions"], [
            {"action": "reset_instances"},
            {"action": "wait_for_packet", "connection": "instance",
             "opcode": scenario.SMSG_INSTANCE_RESET, "timeout_ms": 5_000},
        ])
        self.assertEqual(plan["expect"][0], {
            "event": "packet", "connection": "instance",
            "opcode": 0x2686, "body_hex": "21000000",
        })
        self.assertEqual(plan["expect"][1]["connection"], "realm")
        self.assertEqual(plan["expect"][1]["opcode"], scenario.LOGOUT_COMPLETE)
        self.assertEqual(scenario.CMSG_RESET_INSTANCES, 0x366A)
        self.assertEqual(scenario.INSTANCE_POSITION, (-228.191, 2111.41, 76.8904))


class OfflineSqlTests(unittest.TestCase):
    def test_relocate_is_actor_scoped_exact_and_does_not_copy_facing_as_radians(self) -> None:
        selected = actor()
        db = Mock()
        db.character_identities.return_value = [identity_row(selected)]
        db.execute.return_value = position_row(selected, 33, 0, scenario.INSTANCE_POSITION)

        state = scenario.relocate_inside(db, {}, selected)

        statement = db.transaction.call_args.args[0][0]
        self.assertIn("UPDATE characters SET map=33,instance_id=0", statement)
        self.assertIn("position_x=-228.191,position_y=2111.41,position_z=76.8904", statement)
        self.assertIn("orientation=0.0", statement)
        self.assertNotIn("70", statement.split("WHERE", 1)[0])
        self.assertNotIn("zone=", statement)
        self.assertIn(f"guid={selected['guid']}", statement)
        self.assertIn(f"account={selected['account_id']}", statement)
        self.assertIn("online=0", statement)
        self.assertNotIn("group", statement.lower())
        self.assertNotIn("lock", statement.lower())
        self.assertEqual(state["map"], 33)
        self.assertEqual(state["instance_id"], 0)

    def test_restore_uses_only_journal_home_and_zero_instance(self) -> None:
        selected = actor()
        db = Mock()
        db.character_identities.return_value = [identity_row(selected)]
        db.execute.return_value = position_row(selected, selected["map"], 0, selected["home"])

        state = scenario.restore_outside(db, {}, selected)

        statement = db.transaction.call_args.args[0][0]
        self.assertIn(f"map={selected['map']},instance_id=0", statement)
        self.assertIn("orientation=0.0", statement)
        self.assertNotIn("zone=", statement)
        self.assertEqual(state["map"], selected["map"])
        self.assertEqual(state["instance_id"], 0)

    def test_offline_identity_and_exact_row_guards_abort_mutation_or_verification(self) -> None:
        selected = actor()
        for row in (
            identity_row(selected, "1"),
            [str(selected["guid"]), "999", selected["name"], "0"],
            [str(selected["guid"]), str(selected["account_id"]), "Other", "0"],
            [],
        ):
            db = Mock()
            db.character_identities.return_value = [row] if row else []
            with self.subTest(row=row), self.assertRaises(scenario.ScenarioFailure):
                scenario.relocate_inside(db, {}, selected)
            db.transaction.assert_not_called()

        db = Mock()
        db.character_identities.return_value = [identity_row(selected)]
        db.execute.return_value = position_row(selected, 33, 99, scenario.INSTANCE_POSITION)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.relocate_inside(db, {}, selected)


class InstanceStateTests(unittest.TestCase):
    def test_reconnect_requires_same_nonzero_instance_and_native_position(self) -> None:
        selected = actor()
        initial = inside_state(selected, 41)
        reconnect = inside_state(selected, 41)
        self.assertEqual(scenario.validate_reconnect_state(initial, reconnect), 41)
        for defect in ("zero", "different", "map", "position", "orientation"):
            bad = copy.deepcopy(reconnect)
            if defect == "zero":
                bad["instance_id"] = 0
            elif defect == "different":
                bad["instance_id"] = 42
            elif defect == "map":
                bad["map"] = 0
            elif defect == "position":
                bad["position_x"] += 1.0
            else:
                bad["orientation"] = 70.0
            with self.subTest(defect=defect), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_reconnect_state(initial, bad)

    def test_final_position_must_be_outside_journal_home(self) -> None:
        selected = actor()
        scenario.validate_outside_state(selected, outside_state(selected))
        bad = inside_state(selected)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_outside_state(selected, bad)


class ResetEvidenceTests(unittest.TestCase):
    def test_one_typed_reset_and_exact_response_are_required(self) -> None:
        evidence = {
            "sent_packets": [sent_reset()],
            "events": [reset_event()],
        }
        facts = scenario.validate_reset_evidence(evidence)
        self.assertEqual(facts["map_id"], 33)
        self.assertLess(facts["request_sequence"], facts["response_sequence"])

        for defect in ("missing_request", "duplicate_request", "missing_response",
                       "duplicate_response", "wrong_body", "wrong_connection", "early"):
            bad = copy.deepcopy(evidence)
            if defect == "missing_request":
                bad["sent_packets"] = []
            elif defect == "duplicate_request":
                bad["sent_packets"].append(sent_reset(12))
            elif defect == "missing_response":
                bad["events"] = []
            elif defect == "duplicate_response":
                bad["events"].append(reset_event(12))
            elif defect == "wrong_body":
                bad["events"] = [reset_event(body=33 .to_bytes(4, "big"))]
            elif defect == "wrong_connection":
                bad["events"] = [reset_event(connection="realm")]
            else:
                bad["events"] = [reset_event(sequence=9)]
            with self.subTest(defect=defect), self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_reset_evidence(bad)


class RunCaseTests(unittest.TestCase):
    def test_report_records_all_passes_lifecycle_and_position_restoration(self) -> None:
        selected = actor()
        identity = {"capture": Path("/capture")}
        journal = {"actors": [selected]}
        entry = inside_state(selected, 41)
        reconnect = inside_state(selected, 41)
        final = outside_state(selected)
        reset_evidence = {"sent_packets": [sent_reset()], "events": [reset_event()]}
        with tempfile.TemporaryDirectory() as raw:
            paths = []
            for label in ("entry", "relogin", "reset"):
                path = Path(raw) / f"{label}.json"
                path.write_text("{}\n", encoding="utf-8")
                paths.append(path)
            with patch.object(scenario, "relocate_inside"), \
                    patch.object(scenario, "restore_outside") as restore, \
                    patch.object(scenario.logout_scenario, "_run_single",
                                 side_effect=[{"lifecycle": "entry"}, {"lifecycle": "relogin"},
                                              reset_evidence]), \
                    patch.object(scenario, "_save_position_snapshot",
                                 side_effect=[({}, entry, paths[0]), ({}, reconnect, paths[1]),
                                              ({}, final, paths[2])]):
                report = scenario._run_case(identity, journal, 1, Mock())
        self.assertEqual(report["scenario"], "instance-solo-reset")
        self.assertEqual(len(report["observations"]), 3)
        self.assertEqual(report["observations"][0]["instance_id"], 41)
        self.assertTrue(report["observations"][1]["relogin_same_instance"])
        self.assertEqual(report["observations"][2]["map_id"], 33)
        self.assertEqual(report["final_state"], final)
        self.assertTrue(report["position_restoration"]["outside_home"])
        self.assertFalse(report["position_restoration"]["group_or_lock_rows_mutated"])
        restore.assert_called_once()

    def test_failure_stops_owned_bots_and_restores_owned_position(self) -> None:
        selected = actor()
        bot = object()
        with patch.object(scenario, "relocate_inside"), \
                patch.object(scenario, "restore_outside") as restore, \
                patch.object(scenario.logout_scenario, "_run_single",
                             side_effect=RuntimeError("stop")), \
                patch.object(scenario.death_scenario, "ACTIVE_BOTS", [bot]), \
                patch.object(scenario.death_scenario, "_poll_actor_offline"), \
                patch.object(scenario.death_scenario, "_stop_owned") as stop:
            with self.assertRaisesRegex(RuntimeError, "stop"):
                scenario._run_case({"capture": Path("/capture")}, {"actors": [selected]}, 1, Mock())
        stop.assert_called_once_with(bot)
        restore.assert_called_once()

    def test_failure_report_is_private_and_versioned(self) -> None:
        report = scenario._failure_report(RuntimeError("secret-password"), scenario.SCENARIO)
        self.assertEqual(report["schema"], "rustycore-instance-scenario-v1")
        self.assertFalse(report["parity_assessed"])
        self.assertNotIn("secret-password", json.dumps(report))


if __name__ == "__main__":
    unittest.main()

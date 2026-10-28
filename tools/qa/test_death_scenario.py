"""Hermetic plan, state, secrecy, and login-race checks for death_scenario."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import unittest

import death_fixture
import death_scenario as scenario


def actor(name: str, guid: int) -> dict:
    value = copy.deepcopy(next(item for item in death_fixture.ACTORS if item["name"] == name))
    value["guid"] = guid
    return value


def journal_actors() -> dict[str, dict]:
    return {item["name"]: actor(item["name"], 1000 + index)
            for index, item in enumerate(death_fixture.ACTORS, 1)}


class DeathScenarioTests(unittest.TestCase):
    def test_packed_player_guid_and_legal_wisp_seed(self) -> None:
        wisp = actor("Qadeathwisp", 0x123456)
        guid = scenario.player_guid(wisp, 7)
        self.assertEqual(guid["high"], (2 << 58) | (7 << 42))
        self.assertEqual(scenario.packed_guid(guid["low"], guid["high"]),
                         bytes((0x07, 0xA0, 0x56, 0x34, 0x12, 0x1C, 0x08)))
        seeds = {(item["actor"], item["spell"]) for item in death_fixture.SPELL_SEEDS}
        self.assertIn(("Qadeathwisp", 20585), seeds)
        self.assertNotIn(("Qadeathwisp", 20584), seeds)

    def test_expected_packet_order_and_nonrelease_deaths(self) -> None:
        actors = journal_actors()
        fall = scenario.plan_for("fall-release", 1, actors["Qadeathfall"], 1, 10)
        self.assertEqual([rule["opcode"] for rule in fall["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH, scenario.PRE_RESURRECT,
                          scenario.LOGOUT_COMPLETE])
        offer = scenario.plan_for("offer", 1, actors["Qadeathfall"], 1, 10)
        self.assertEqual([rule["opcode"] for rule in offer["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH, scenario.LOGOUT_COMPLETE])
        reinc = scenario.plan_for("reincarnation", 1, actors["Qadeathreinc"], 1, 10)
        self.assertEqual([rule["opcode"] for rule in reinc["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH, scenario.LOGOUT_COMPLETE])
        wisp = scenario.plan_for("wisp", 1, actors["Qadeathwisp"], 1, 10)
        self.assertEqual([rule["opcode"] for rule in wisp["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH, scenario.PRE_RESURRECT,
                          scenario.LOGOUT_COMPLETE])
        recover = scenario.plan_for("recovery-dead", 1, actors["Qadeathfall"], 1, 10)
        self.assertEqual(recover["finish_policy"], "disconnect")
        self.assertEqual([rule["opcode"] for rule in recover["expect"]],
                         [scenario.DURABILITY_DAMAGE_DEATH])

    def test_each_bot_plan_has_exactly_one_normal_logout_or_none_on_disconnect(self) -> None:
        actors = journal_actors()
        plans = [
            scenario.plan_for("fall-release", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("reclaim", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("healer", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("healer", 2, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("wisp", 1, actors["Qadeathwisp"], 1, 10),
            scenario.plan_for("wisp", 2, actors["Qadeathwisp"], 1, 10),
            scenario.plan_for("reincarnation", 1, actors["Qadeathreinc"], 1, 10),
            scenario.plan_for("mob", 1, actors["Qadeathmob"], 1, 10),
            scenario.plan_for("automatic-release", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("offer", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("offer", 2, actors["Qadeathfall"], 1, 10, actors),
            scenario.plan_for("recovery-dead", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("recovery-dead", 2, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("recovery-ghost", 1, actors["Qadeathfall"], 1, 10),
            scenario.plan_for("recovery-ghost", 2, actors["Qadeathfall"], 1, 10),
        ]
        for plan in plans:
            count = sum(rule.get("opcode") == scenario.LOGOUT_COMPLETE for rule in plan["expect"])
            self.assertEqual(count, 0 if plan["finish_policy"] == "disconnect" else 1)

    def test_case_windows_and_wisp_relogin_plan_are_bounded(self) -> None:
        actors = journal_actors()
        self.assertEqual(scenario.plan_for("fall-release", 1, actors["Qadeathfall"], 1, 10)["timeout_ms"], 10_000)
        self.assertEqual(scenario.plan_for("reclaim", 1, actors["Qadeathfall"], 1, 10)["timeout_ms"], 45_000)
        self.assertEqual(scenario.plan_for("healer", 2, actors["Qadeathfall"], 1, 10)["timeout_ms"], 3_000)
        self.assertEqual(scenario.plan_for("mob", 1, actors["Qadeathmob"], 1, 10)["timeout_ms"], 30_000)
        automatic = scenario.plan_for("automatic-release", 1, actors["Qadeathfall"], 1, 10)
        self.assertEqual(automatic["timeout_ms"], 390_000)
        self.assertFalse(any(action["action"] == "release_spirit" for action in automatic["actions"]))
        self.assertEqual(scenario.wisp_expiry_plan(actors["Qadeathwisp"])["actions"],
                         [{"action": "wait", "milliseconds": 75_000}])
        self.assertEqual(scenario.wisp_expiry_plan(actors["Qadeathwisp"])["timeout_ms"], 78_000)

    def test_wisp_epoch_two_heals_then_relogs_without_refalling(self) -> None:
        actors = journal_actors()
        plan = scenario.plan_for("wisp", 2, actors["Qadeathwisp"], 1, 10)
        self.assertEqual([item["action"] for item in plan["actions"]], ["spirit_healer_activate"])
        self.assertEqual(plan["character_guid"], actors["Qadeathwisp"]["guid"])
        observer = scenario.observer_plan(150_000, actors["Qadeathcast"]["guid"])
        self.assertEqual(observer["character_guid"], actors["Qadeathcast"]["guid"])
        self.assertTrue(observer["observe_only"])

    def test_offer_plan_uses_validated_journal_actor_guids(self) -> None:
        actors = journal_actors()
        plan = scenario.plan_for("offer", 2, actors["Qadeathfall"], 3, 10, actors)
        responses = [action for action in plan["actions"] if action["action"] == "resurrect_response"]
        self.assertEqual([item["response"] for item in responses], [0, 1, 0, 0])
        self.assertEqual(responses[0]["offerer"], scenario.player_guid(actors["Qadeathmob"], 3))
        self.assertEqual(responses[2]["offerer"], scenario.player_guid(actors["Qadeathcast"], 3))
        self.assertEqual(plan["actions"][0]["opcode"], scenario.RESURRECT_REQUEST)

    def test_wisp_state_validation_is_epoch_specific_and_checks_corpse_owner(self) -> None:
        selected = actor("Qadeathwisp", 44)
        ghost = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                 "rows": [["44", "1", str(scenario.PLAYER_GHOST)]]},
                 "corpse": {"columns": ["guid"], "rows": [["44"]]}}
        state = scenario.validate_state("wisp", 1, selected, ghost)
        self.assertTrue(state["ghost"])
        bad_health = copy.deepcopy(ghost)
        bad_health["characters"]["rows"][0][1] = "0"
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_state("wisp", 1, selected, bad_health)
        alive = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                 "rows": [["44", "1", "0"]]},
                 "corpse": {"columns": ["guid"], "rows": []}}
        self.assertFalse(scenario.validate_state("wisp", 2, selected, alive)["ghost"])
        wrong_owner = copy.deepcopy(ghost)
        wrong_owner["corpse"]["rows"] = [["45"]]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_state("wisp", 1, selected, wrong_owner)

    def test_unreleased_deaths_require_zero_health_without_ghost_or_corpse(self) -> None:
        selected = actor("Qadeathfall", 55)
        dead = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                "rows": [["55", "0", "0"]]},
                "corpse": {"columns": ["guid"], "rows": []}}
        self.assertEqual(scenario.validate_state("offer", 1, selected, dead)["health"], 0)
        with_corpse = copy.deepcopy(dead)
        with_corpse["corpse"]["rows"] = [["55"]]
        for case, epoch in (("offer", 1), ("mob", 1), ("recovery-dead", 2)):
            with self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_state(case, epoch, selected, with_corpse)

    def test_mob_accepts_damage_and_root_during_login_drain(self) -> None:
        evidence = {
            "trace": [
                {"sequence": 1, "kind": "packet_observed", "opcode": scenario.ATTACKER_STATE_UPDATE,
                 "event_sequence": 0},
                {"sequence": 2, "kind": "packet_redacted", "opcode": scenario.MOVE_ROOT},
                {"sequence": 3, "kind": "action_started", "action": "attack_swing"},
                {"sequence": 4, "kind": "send_attempted", "action": "attack_swing"},
            ],
            "events": [{"sequence": 0, "opcode": scenario.ATTACKER_STATE_UPDATE,
                        "body_bytes": 24, "body_hex": "aa"}],
            "target_selections": [{"action": "attack_swing", "selector": {"entry": 6, "map_id": 0},
                                   "guid": {"low": 90, "high": 1},
                                   "source_event_sha256": "a" * 64}],
        }
        observed: dict = {}
        scenario._validate_mob_evidence(evidence, observed)
        self.assertEqual(observed["mob_evidence"]["damage_phases"],
                         [{"phase": "login_observation", "event": evidence["events"][0]}])
        self.assertEqual(observed["mob_evidence"]["root_phases"], ["login_observation"])

    def test_snapshot_pass_names_do_not_collide_and_wisp_expiry_uses_owner_rows(self) -> None:
        selected = actor("Qadeathwisp", 44)
        output = Path("/private")
        first = scenario._snapshot_path(output, "wisp", 2, selected, "wisp-healer")
        second = scenario._snapshot_path(output, "wisp", 2, selected, "wisp-expiry-relogin")
        self.assertNotEqual(first, second)
        healer = {"auras": [{"spell": 15007, "maxduration": 60_000}]}
        expiry = {"auras": []}
        healer["auras"][0]["remaintime"] = 58_000
        facts = scenario._validate_wisp_sickness(healer, expiry)
        self.assertEqual(facts["expiry_relogin_wait_ms"], 75_000)
        healer["auras"][0]["remaintime"] = 0
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_wisp_sickness(healer, expiry)
        paladin = {"auras": [{"spell": 15007, "maxduration": 60_000}]}
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_wisp_sickness(healer, paladin)

    def test_observer_nonzero_exit_requires_observe_only_evidence_and_failure_is_private(self) -> None:
        self.assertTrue(scenario._observer_exit_allowed(1))
        self.assertFalse(scenario._observer_exit_allowed(2))
        selected = actor("Qadeathcast", 70)
        report = {"results": [{"character_guid": 70, "cast_lifecycle": {
            "failure": None, "observe_only": True, "passed": False,
            "finish_completed": True}}]}
        evidence = scenario._result_for(report, selected, observer=True)
        self.assertFalse(evidence["passed"])
        report["results"][0]["cast_lifecycle"]["observe_only"] = False
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._result_for(report, selected, observer=True)
        payload = scenario._failure_report(RuntimeError("secret-password"), "wisp", 2)
        self.assertNotIn("secret-password", json.dumps(payload))
        self.assertNotIn("traceback", json.dumps(payload).casefold())

    def test_ready_marker_is_explicit(self) -> None:
        self.assertTrue(scenario._has_ready_marker("INFO CAST_LIFECYCLE_READY"))
        self.assertFalse(scenario._has_ready_marker("login stream started"))


if __name__ == "__main__":
    unittest.main()

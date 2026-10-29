"""Hermetic plan, state, secrecy, and login-race checks for death_scenario."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
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

    def test_stuck_cases_keep_original_scenarios_and_bind_to_journal_caster(self) -> None:
        self.assertEqual(len(scenario.SCENARIOS), 12)
        self.assertEqual(sum(scenario.SCENARIOS.values()), 18)
        self.assertEqual({name: count for name, count in scenario.SCENARIOS.items()
                          if name not in {"stuck-death", "stuck-home"}}, {
            "fall-release": 1, "reclaim": 1, "healer": 2, "wisp": 2,
            "reincarnation": 1, "mob": 1, "automatic-release": 1, "offer": 2,
            "recovery-dead": 2, "recovery-ghost": 2,
        })
        self.assertEqual(scenario.SCENARIOS["stuck-death"], 2)
        self.assertEqual(scenario.SCENARIOS["stuck-home"], 1)
        selected = actor("Qadeathcast", 73)
        death_first = scenario.plan_for("stuck-death", 1, selected, 1, 10)
        death_second = scenario.plan_for("stuck-death", 2, selected, 1, 10)
        home = scenario.plan_for("stuck-home", 1, selected, 1, 10)
        for plan in (death_first, death_second, home):
            self.assertEqual(plan["character_guid"], selected["guid"])
            self.assertEqual(plan["finish_policy"], "logout")
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.plan_for("stuck-death", 3, selected, 1, 10)

    def test_stuck_death_plans_cast_hearthstone_then_kill_and_later_repop(self) -> None:
        selected = actor("Qadeathcast", 74)
        first = scenario.plan_for("stuck-death", 1, selected, 2, 10)
        self.assertEqual([action["action"] for action in first["actions"]],
                         ["cast", "wait_for_packet", "wait", "cast", "wait_for_packet"])
        self.assertEqual([action["spell_id"] for action in first["actions"]
                          if action["action"] == "cast"], [8690, 7355])
        self.assertEqual([action["opcode"] for action in first["actions"]
                          if action["action"] == "wait_for_packet"],
                         [scenario.MOVE_TELEPORT, scenario.MOVE_ROOT])
        self.assertEqual([action["milliseconds"] for action in first["actions"]
                          if action["action"] == "wait"], [2000])
        self.assertTrue(all(action.get("target") == scenario.player_guid(selected, 2)
                            for action in first["actions"] if action["action"] == "cast"))
        self.assertEqual([(rule.get("event"), rule.get("spell_id"), rule.get("opcode"))
                          for rule in first["expect"]], [
            ("go", 8690, None), ("go", 7355, None),
            ("packet", None, scenario.LOGOUT_COMPLETE),
        ])
        self.assertNotIn(scenario.DURABILITY_DAMAGE_DEATH,
                         [rule.get("opcode") for rule in first["expect"]])

        second = scenario.plan_for("stuck-death", 2, selected, 2, 10)
        self.assertEqual([action["spell_id"] for action in second["actions"]
                          if action["action"] == "cast"], [7355])
        self.assertFalse(any(action["action"] in {"release_spirit", "spirit_healer_activate", "self_res"}
                             for action in second["actions"]))
        self.assertEqual([rule.get("spell_id") for rule in second["expect"]
                          if rule.get("event") == "go"], [7355])

    def test_stuck_home_plan_has_no_hearthstone_cast_or_cooldown_seed_action(self) -> None:
        selected = actor("Qadeathcast", 75)
        plan = scenario.plan_for("stuck-home", 1, selected, 1, 10)
        self.assertEqual([action["spell_id"] for action in plan["actions"]
                          if action["action"] == "cast"], [7355])
        self.assertEqual([action["opcode"] for action in plan["actions"]
                          if action["action"] == "wait_for_packet"], [scenario.MOVE_TELEPORT])
        self.assertEqual([rule.get("spell_id") for rule in plan["expect"]
                          if rule.get("event") == "go"], [7355])
        self.assertEqual(plan["finish_policy"], "logout")

    def test_stuck_death_requires_the_explicit_pvp_durability_guard(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            config = Path(root) / "worldserver.conf"
            config.write_text("DurabilityLoss.InPvP = 0\n", encoding="utf-8")
            scenario._require_stuck_death_config("stuck-death", config)
            scenario._require_stuck_death_config("stuck-home", config)
            for setting in ("1", "true", ""):
                config.write_text(f"DurabilityLoss.InPvP = {setting}\n", encoding="utf-8")
                with self.subTest(setting=setting), self.assertRaises(scenario.ScenarioFailure):
                    scenario._require_stuck_death_config("stuck-death", config)

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

    def test_stuck_state_requires_dead_no_corpse_or_live_home_result(self) -> None:
        selected = actor("Qadeathcast", 56)
        dead = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                "rows": [["56", "0", "0"]]},
                "corpse": {"columns": ["guid"], "rows": []}}
        for epoch in (1, 2):
            self.assertEqual(scenario.validate_state("stuck-death", epoch, selected, dead)["health"], 0)
        alive = {"characters": {"columns": ["guid", "health", "playerFlags"],
                                 "rows": [["56", "100", "0"]]},
                 "corpse": {"columns": ["guid"], "rows": []}}
        self.assertEqual(scenario.validate_state("stuck-home", 1, selected, alive)["health"], 100)
        ghost = copy.deepcopy(dead)
        ghost["characters"]["rows"][0][2] = str(scenario.PLAYER_GHOST)
        corpse = copy.deepcopy(dead)
        corpse["corpse"]["rows"] = [["56"]]
        living_death = copy.deepcopy(dead)
        living_death["characters"]["rows"][0][1] = "1"
        dead_home = copy.deepcopy(alive)
        dead_home["characters"]["rows"][0][1] = "0"
        for bad in (ghost, corpse, living_death):
            with self.assertRaises(scenario.ScenarioFailure):
                scenario.validate_state("stuck-death", 2, selected, bad)
        with self.assertRaises(scenario.ScenarioFailure):
            scenario.validate_state("stuck-home", 1, selected, dead_home)

    def test_stuck_graveyard_position_uses_quarter_yard_tolerance(self) -> None:
        selected = actor("Qadeathcast", 57)
        snapshot = {"characters": {"columns": ["guid", "map", "position_x", "position_y",
                                                 "position_z"],
                                    "rows": [["57", "0", "-9339.72", "170.2", "61.5622"]]}}
        position = scenario._validate_stuck_graveyard_position(snapshot, selected)
        self.assertEqual(position["map"], 0)
        self.assertLessEqual(position["distance_from_graveyard_106"], 0.25)
        near_home = copy.deepcopy(snapshot)
        near_home["characters"]["rows"][0][2:] = ["-9340.21", "167.61", "61.60"]
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_graveyard_position(near_home, selected)

    def test_stuck_first_epoch_proves_hearthstone_cooldown_and_ack_before_kill(self) -> None:
        selected = actor("Qadeathcast", 58)
        snapshot = {"character_spell_cooldown": {
            "columns": ["guid", "spell", "time", "categoryId", "categoryEnd"],
            "rows": [["58", "8690", "2000000000", "1176", "2000000000"]],
        }}
        evidence = {
            "events": [
                {"opcode": scenario.SPELL_GO, "spell_id": 8690},
                {"opcode": scenario.SPELL_GO, "spell_id": 7355},
                {"opcode": scenario.MOVE_ROOT},
            ],
            "sent_packets": [
                {"action": "cast", "action_index": 0},
                {"action": "ack_move_teleport", "opcode": scenario.MOVE_TELEPORT_ACK,
                 "connection": "instance", "write_attempted": True,
                 "write_completed": True, "write_error": None},
                {"action": "cast", "action_index": 2},
            ],
        }
        observation: dict = {}
        scenario._validate_stuck_evidence("stuck-death", 1, selected, snapshot,
                                          evidence, observation)
        self.assertEqual(observation["stuck_spell_cooldowns"][0]["categoryid"], 1176)
        durability = copy.deepcopy(evidence)
        durability["events"].append({"opcode": scenario.DURABILITY_DAMAGE_DEATH})
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_evidence("stuck-death", 1, selected, snapshot,
                                              durability, {})

    def test_stuck_home_rejects_any_saved_or_emitted_hearthstone_cooldown(self) -> None:
        selected = actor("Qadeathcast", 59)
        empty_snapshot = {"character_spell_cooldown": {
            "columns": ["guid", "spell", "time", "categoryId", "categoryEnd"], "rows": []}}
        evidence = {
            "events": [{"opcode": scenario.SPELL_GO, "spell_id": 7355}],
            "sent_packets": [
                {"action": "cast", "action_index": 0},
                {"action": "ack_move_teleport", "opcode": scenario.MOVE_TELEPORT_ACK,
                 "connection": "instance", "write_attempted": True,
                 "write_completed": True, "write_error": None},
            ],
        }
        scenario._validate_stuck_evidence("stuck-home", 1, selected, empty_snapshot,
                                          evidence, {})
        emitted = copy.deepcopy(evidence)
        emitted["events"].append({"opcode": scenario.COOLDOWN_EVENT,
                                  "body_hex": (8690).to_bytes(4, "little", signed=True).hex() + "00"})
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_evidence("stuck-home", 1, selected, empty_snapshot,
                                              emitted, {})
        saved = {"character_spell_cooldown": {
            "columns": ["guid", "spell", "time", "categoryId", "categoryEnd"],
            "rows": [["59", "8690", "2000000000", "1176", "2000000000"]],
        }}
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_evidence("stuck-home", 1, selected, saved,
                                              evidence, {})

    def test_dead_stuck_repop_requires_packet_evidence_and_exact_graveyard(self) -> None:
        selected = actor("Qadeathcast", 60)
        snapshot = {"characters": {"columns": ["guid", "map", "position_x", "position_y",
                                                 "position_z"],
                                    "rows": [["60", "0", "-9339.72", "170.2", "61.5622"]]},
                    "character_spell_cooldown": {
                        "columns": ["guid", "spell", "time", "categoryId", "categoryEnd"],
                        "rows": [["60", "8690", "2000000000", "1176", "2000000000"]]}}
        evidence = {"events": [{"opcode": scenario.SPELL_GO, "spell_id": 7355}],
                    "sent_packets": []}
        observation: dict = {}
        with self.assertRaises(scenario.ScenarioFailure):
            scenario._validate_stuck_evidence("stuck-death", 2, selected, snapshot,
                                              evidence, observation)
        evidence["events"].append({"opcode": scenario.DEATH_RELEASE_LOC})
        scenario._validate_stuck_evidence("stuck-death", 2, selected, snapshot,
                                          evidence, observation)
        self.assertEqual(observation["graveyard_position"]["map"], 0)
        self.assertEqual(observation["stuck_spell_cooldowns"], [{
            "spell": 8690, "time": 2_000_000_000,
            "categoryid": 1176, "categoryend": 2_000_000_000,
        }])
        absent = copy.deepcopy(snapshot)
        absent["character_spell_cooldown"]["rows"] = []
        invalid_category = copy.deepcopy(snapshot)
        invalid_category["character_spell_cooldown"]["rows"][0][3] = "0"
        invalid_time = copy.deepcopy(snapshot)
        invalid_time["character_spell_cooldown"]["rows"][0][2] = "0"
        invalid_category_end = copy.deepcopy(snapshot)
        invalid_category_end["character_spell_cooldown"]["rows"][0][4] = "0"
        for bad_snapshot in (absent, invalid_category, invalid_time, invalid_category_end):
            with self.assertRaises(scenario.ScenarioFailure):
                scenario._validate_stuck_evidence("stuck-death", 2, selected, bad_snapshot,
                                                  evidence, {})

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

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map scripts and zone weather.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn represented_script_schedule_count_like_cpp(&self) -> usize {
        self.script_schedule_like_cpp.values().map(Vec::len).sum()
    }

    pub fn represented_executed_script_actions_like_cpp(
        &self,
    ) -> &[RepresentedScriptScheduleActionLikeCpp] {
        &self.represented_executed_script_actions_like_cpp
    }

    pub const fn is_script_schedule_locked_like_cpp(&self) -> bool {
        self.script_schedule_lock_like_cpp
    }

    /// Bounded represented seam for C++ `Map::ScriptCommandStart` scheduling.
    ///
    /// C++ anchors:
    /// - `MapScripts.cpp:72-98` schedules one action at
    ///   `GameTime::GetGameTime() + delay`, increments the global scheduled count,
    ///   and immediately processes zero-delay actions when `!i_scriptLock`.
    /// - `MapScripts.cpp:386-893` real commands are intentionally not executed by
    ///   this Rust seam; due actions are only recorded as represented evidence.
    pub fn schedule_represented_script_action_like_cpp(
        &mut self,
        now_secs: i64,
        delay_secs: u32,
        source_guid: ObjectGuid,
        target_guid: ObjectGuid,
        owner_guid: ObjectGuid,
        command_id: u32,
    ) -> ScriptScheduleStartOutcomeLikeCpp {
        let due_time_secs = now_secs.saturating_add(i64::from(delay_secs));
        let scheduled = RepresentedScriptScheduleActionLikeCpp {
            source_guid,
            target_guid,
            owner_guid,
            command_id,
            due_time_secs,
        };
        self.script_schedule_like_cpp
            .entry(due_time_secs)
            .or_default()
            .push(scheduled);

        let immediate_process = if delay_secs == 0 && !self.script_schedule_lock_like_cpp {
            Some(self.process_script_schedule_update_order_like_cpp(now_secs))
        } else {
            None
        };

        ScriptScheduleStartOutcomeLikeCpp {
            scheduled,
            represented_increase_count: 1,
            remaining_after_schedule: self.represented_script_schedule_count_like_cpp(),
            immediate_process,
        }
    }

    /// Bounded represented C++ `Map::ScriptsProcess()` drain.
    ///
    /// Empty schedules are no-ops. Otherwise only sorted entries whose due time is
    /// `<= GameTime::GetGameTime()` are erased and recorded as represented-executed
    /// evidence; future entries remain queued and stop the drain. This does not
    /// execute talk/emote/move/teleport/quest/gossip/item/weather/script-manager
    /// commands or any DB/session/ObjectAccessor side effects.
    pub fn process_due_script_schedule_like_cpp(
        &mut self,
        now_secs: i64,
    ) -> ScriptScheduleProcessSummaryLikeCpp {
        let queued_before = self.represented_script_schedule_count_like_cpp();
        if queued_before == 0 {
            return ScriptScheduleProcessSummaryLikeCpp {
                queued_before,
                remaining: 0,
                empty_noop: true,
                ..Default::default()
            };
        }

        let mut processed_actions = Vec::new();
        loop {
            let Some((&due_time_secs, _)) = self.script_schedule_like_cpp.first_key_value() else {
                break;
            };
            if due_time_secs > now_secs {
                break;
            }
            if let Some(mut actions) = self.script_schedule_like_cpp.remove(&due_time_secs) {
                processed_actions.append(&mut actions);
            }
        }

        self.represented_executed_script_actions_like_cpp
            .extend(processed_actions.iter().copied());
        let remaining = self.represented_script_schedule_count_like_cpp();
        ScriptScheduleProcessSummaryLikeCpp {
            queued_before,
            processed: processed_actions.len(),
            remaining,
            represented_decrease_count: processed_actions.len(),
            lock_entered: false,
            empty_noop: false,
            processed_actions,
        }
    }

    /// C++ `Map::Update` order helper for the script seam.
    ///
    /// Mirrors `if (!m_scriptSchedule.empty()) { i_scriptLock = true;
    /// ScriptsProcess(); i_scriptLock = false; }` between `SendObjectUpdates()`
    /// and weather/personal phase (`Map.cpp:777-798`).
    pub fn process_script_schedule_update_order_like_cpp(
        &mut self,
        now_secs: i64,
    ) -> ScriptScheduleProcessSummaryLikeCpp {
        if self.script_schedule_like_cpp.is_empty() {
            return ScriptScheduleProcessSummaryLikeCpp {
                empty_noop: true,
                ..Default::default()
            };
        }

        self.script_schedule_lock_like_cpp = true;
        let mut summary = self.process_due_script_schedule_like_cpp(now_secs);
        self.script_schedule_lock_like_cpp = false;
        summary.lock_entered = true;
        summary
    }

    #[cfg(test)]
    pub(super) fn set_script_schedule_lock_for_test(&mut self, locked: bool) {
        self.script_schedule_lock_like_cpp = locked;
    }

    pub const fn weather_update_timer_current_ms_like_cpp(&self) -> u32 {
        self.weather_update_timer_current_ms_like_cpp
    }

    pub const fn weather_update_timer_interval_ms_like_cpp(&self) -> u32 {
        self.weather_update_timer_interval_ms_like_cpp
    }

    pub fn represented_zone_default_weather_update_diffs_like_cpp(
        &self,
        zone_id: u32,
    ) -> Option<&[u32]> {
        self.zone_dynamic_info_like_cpp
            .get(&zone_id)?
            .default_weather
            .as_ref()
            .map(RepresentedZoneDefaultWeatherLikeCpp::update_call_diffs_ms)
    }

    #[cfg(test)]
    pub(crate) fn register_represented_zone_default_weather_for_test(&mut self, zone_id: u32) {
        self.zone_dynamic_info_like_cpp
            .entry(zone_id)
            .or_default()
            .default_weather = Some(RepresentedZoneDefaultWeatherLikeCpp::new());
    }

    #[cfg(test)]
    pub(crate) fn set_represented_zone_default_weather_next_update_alive_for_test(
        &mut self,
        zone_id: u32,
        alive: bool,
    ) -> bool {
        let Some(weather) = self
            .zone_dynamic_info_like_cpp
            .get_mut(&zone_id)
            .and_then(|zone| zone.default_weather.as_mut())
        else {
            return false;
        };
        weather.set_next_update_returns_alive(alive);
        true
    }

    /// Represented C++ `_weatherUpdateTimer` / `_zoneDynamicInfo.DefaultWeather`
    /// step from `Map::Update` (`Map.cpp:777-798`).
    ///
    /// Timer semantics mirror `IntervalTimer` (`Timer.h:62-87`): update adds the
    /// diff, `Passed()` is `current >= interval`, and `Reset()` keeps overshoot via
    /// modulo. When passed, existing represented zones are iterated and only zones
    /// with `DefaultWeather` call represented `Weather::Update(interval)`. A false
    /// represented return removes only that optional weather pointer like C++
    /// `DefaultWeather.reset()`. Weather regeneration/RNG, `UpdateWeather`, player
    /// discovery/fanout, `sWorld->SendZoneMessage`, `sScriptMgr` hooks, DB and
    /// WeatherMgr runtime are explicit gaps surfaced in the summary flag.
    pub fn update_weather_like_cpp(&mut self, diff_ms: u32) -> WeatherUpdateSummaryLikeCpp {
        let interval_ms = self.weather_update_timer_interval_ms_like_cpp;
        let timer_current_before = self.weather_update_timer_current_ms_like_cpp;
        self.weather_update_timer_current_ms_like_cpp = self
            .weather_update_timer_current_ms_like_cpp
            .saturating_add(diff_ms);
        let timer_current_after_update = self.weather_update_timer_current_ms_like_cpp;
        let timer_passed = timer_current_after_update >= interval_ms;
        let mut summary = WeatherUpdateSummaryLikeCpp {
            interval_ms,
            timer_current_before,
            timer_current_after_update,
            timer_current_after_reset: timer_current_after_update,
            timer_passed,
            script_update_regeneration_fanout_not_represented: true,
            ..Default::default()
        };

        if !timer_passed {
            return summary;
        }

        summary.zones_seen = self.zone_dynamic_info_like_cpp.len();
        for zone_info in self.zone_dynamic_info_like_cpp.values_mut() {
            let Some(default_weather) = zone_info.default_weather.as_mut() else {
                summary.zones_without_default_weather += 1;
                continue;
            };
            summary.default_weather_updated += 1;
            summary.weather_update_call_diff_ms = Some(interval_ms);
            if !default_weather.update_like_cpp(interval_ms) {
                zone_info.default_weather = None;
                summary.default_weather_removed += 1;
            }
        }

        self.weather_update_timer_current_ms_like_cpp %= interval_ms;
        summary.timer_current_after_reset = self.weather_update_timer_current_ms_like_cpp;
        summary
    }

    /// Count exact typed in-world Creature/GameObject candidates for represented
    /// C++ `GameEventMgr::RunSmartAIScripts` evidence.
    ///
    /// This intentionally reads only canonical `Map::entity_world`. Generic
    /// fallback records are ignored because C++ uses typed object stores. Transport
    /// records are also ignored even though they can expose a GameObject view; the
    /// C++ hook worker's switch has no transport branch in this slice.
    pub fn game_event_smart_ai_script_candidates_like_cpp(
        &self,
    ) -> GameEventSmartAiScriptCandidateSummaryLikeCpp {
        let mut summary = GameEventSmartAiScriptCandidateSummaryLikeCpp {
            maps_visited: 1,
            ..GameEventSmartAiScriptCandidateSummaryLikeCpp::default()
        };

        for record in self.entity_world.values() {
            match record.kind() {
                AccessorObjectKind::Creature => {
                    if record
                        .creature()
                        .is_some_and(|creature| creature.unit().world().object().is_in_world())
                    {
                        summary.in_world_creature_candidates += 1;
                        summary.creature_ai_enabled_unrepresented += 1;
                        summary.script_dispatch_unrepresented += 1;
                    }
                }
                AccessorObjectKind::GameObject => {
                    if record
                        .game_object()
                        .is_some_and(|game_object| game_object.world().object().is_in_world())
                    {
                        summary.in_world_gameobject_candidates += 1;
                        summary.script_dispatch_unrepresented += 1;
                    }
                }
                _ => {}
            }
        }

        summary
    }
}

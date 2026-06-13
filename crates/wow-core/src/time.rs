use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Server-side time tracking. Wraps `Instant` for monotonic elapsed time.
#[derive(Debug, Clone, Copy)]
pub struct ServerTime {
    start: Instant,
}

impl ServerTime {
    pub fn now() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Milliseconds elapsed since this `ServerTime` was created.
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// Duration elapsed since this `ServerTime` was created.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Default for ServerTime {
    fn default() -> Self {
        Self::now()
    }
}

/// Game time (Unix timestamp based). Used for calendar, mail, auctions, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameTime(u64);

impl GameTime {
    /// Current game time (Unix timestamp in seconds).
    pub fn now() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self(secs)
    }

    /// Create from a Unix timestamp (seconds).
    pub fn from_unix(secs: u64) -> Self {
        Self(secs)
    }

    /// Get the Unix timestamp in seconds.
    pub fn as_secs(&self) -> u64 {
        self.0
    }

    /// Get the packed WoW time format.
    /// Bits: [minute:6][hour:5][weekday:3][monthDay:6][month:4][year:5][unused:3]
    pub fn to_packed(&self) -> u32 {
        let secs = i64::try_from(self.0).unwrap_or(i64::MAX);
        local_wow_time_fields_like_cpp(secs)
            .map(pack_wow_time_fields_like_cpp)
            .unwrap_or_else(|| pack_wow_time_fields_like_cpp(utc_wow_time_fields_like_cpp(0)))
    }

    /// Check if this time has passed (is before now).
    pub fn has_passed(&self) -> bool {
        *self <= Self::now()
    }

    /// Duration until this time from now (0 if already passed).
    pub fn time_until(&self) -> Duration {
        let now = Self::now();
        if self.0 > now.0 {
            Duration::from_secs(self.0 - now.0)
        } else {
            Duration::ZERO
        }
    }

    /// Add seconds to this time.
    pub fn add_secs(&self, secs: u64) -> Self {
        Self(self.0 + secs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WowTimeFieldsLikeCpp {
    year: u32,
    month: u32,
    month_day: u32,
    week_day: u32,
    hour: u32,
    minute: u32,
    flags: u32,
}

fn local_wow_time_fields_like_cpp(secs: i64) -> Option<WowTimeFieldsLikeCpp> {
    #[cfg(unix)]
    {
        unix_local_wow_time_fields_like_cpp(secs)
    }

    #[cfg(not(unix))]
    {
        Some(utc_wow_time_fields_like_cpp(secs))
    }
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn unix_local_wow_time_fields_like_cpp(secs: i64) -> Option<WowTimeFieldsLikeCpp> {
    let unix_time = libc::time_t::try_from(secs).ok()?;
    let mut time_local = std::mem::MaybeUninit::<libc::tm>::uninit();

    // SAFETY: `time_local` points to valid writable storage for `libc::tm`,
    // and `unix_time` lives long enough for this synchronous libc call.
    let result = unsafe { libc::localtime_r(&unix_time, time_local.as_mut_ptr()) };
    if result.is_null() {
        return None;
    }

    // SAFETY: `localtime_r` returned non-null, so it initialized `time_local`.
    let time_local = unsafe { time_local.assume_init() };
    Some(wow_time_fields_from_tm_like_cpp(
        time_local.tm_year,
        time_local.tm_mon,
        time_local.tm_mday,
        time_local.tm_wday,
        time_local.tm_hour,
        time_local.tm_min,
    ))
}

fn wow_time_fields_from_tm_like_cpp(
    tm_year: i32,
    tm_mon: i32,
    tm_mday: i32,
    tm_wday: i32,
    tm_hour: i32,
    tm_min: i32,
) -> WowTimeFieldsLikeCpp {
    WowTimeFieldsLikeCpp {
        year: (tm_year - 100).rem_euclid(100) as u32,
        month: tm_mon.max(0) as u32,
        month_day: tm_mday.saturating_sub(1).max(0) as u32,
        week_day: tm_wday.max(0) as u32,
        hour: tm_hour.max(0) as u32,
        minute: tm_min.max(0) as u32,
        flags: 0,
    }
}

fn pack_wow_time_fields_like_cpp(fields: WowTimeFieldsLikeCpp) -> u32 {
    ((fields.year & 0x1F) << 24)
        | ((fields.month & 0x0F) << 20)
        | ((fields.month_day & 0x3F) << 14)
        | ((fields.week_day & 0x07) << 11)
        | ((fields.hour & 0x1F) << 6)
        | (fields.minute & 0x3F)
        | ((fields.flags & 0x03) << 29)
}

fn utc_wow_time_fields_like_cpp(secs: i64) -> WowTimeFieldsLikeCpp {
    let days = secs.div_euclid(86_400);
    let seconds_of_day = secs.rem_euclid(86_400);
    let (year, month, month_day) = civil_from_days_like_cpp(days);

    WowTimeFieldsLikeCpp {
        year: (year - 2000).rem_euclid(100) as u32,
        month,
        month_day,
        week_day: (days + 4).rem_euclid(7) as u32,
        hour: (seconds_of_day / 3_600) as u32,
        minute: ((seconds_of_day % 3_600) / 60) as u32,
        flags: 0,
    }
}

fn civil_from_days_like_cpp(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };

    (year as i32, (month - 1) as u32, (day - 1) as u32)
}

impl Default for GameTime {
    fn default() -> Self {
        Self(0)
    }
}

/// Diff time — milliseconds elapsed since last update tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diff(pub u32);

impl Diff {
    pub fn from_ms(ms: u32) -> Self {
        Self(ms)
    }

    pub fn as_ms(&self) -> u32 {
        self.0
    }

    pub fn as_secs_f32(&self) -> f32 {
        self.0 as f32 / 1000.0
    }
}

/// C++ `IntervalTimer` port from `src/common/Time/Timer.h`.
///
/// It accumulates signed millisecond diffs, reports passed once
/// `current >= interval`, and `reset` preserves overshoot with modulo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalTimer {
    interval: i64,
    current: i64,
}

impl IntervalTimer {
    pub const fn new() -> Self {
        Self {
            interval: 0,
            current: 0,
        }
    }

    pub fn update(&mut self, diff: i64) {
        self.current = self.current.saturating_add(diff);
        if self.current < 0 {
            self.current = 0;
        }
    }

    pub const fn passed(&self) -> bool {
        self.current >= self.interval
    }

    pub fn reset(&mut self) {
        if self.interval > 0 && self.current >= self.interval {
            self.current %= self.interval;
        }
    }

    pub const fn set_current(&mut self, current: i64) {
        self.current = current;
    }

    pub const fn set_interval(&mut self, interval: i64) {
        self.interval = interval;
    }

    pub const fn interval(&self) -> i64 {
        self.interval
    }

    pub const fn current(&self) -> i64 {
        self.current
    }
}

impl Default for IntervalTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_time_elapsed() {
        let t = ServerTime::now();
        std::thread::sleep(Duration::from_millis(10));
        assert!(t.elapsed_ms() >= 10);
    }

    #[test]
    fn test_game_time_now() {
        let t = GameTime::now();
        assert!(t.as_secs() > 0);
    }

    #[test]
    fn test_game_time_add() {
        let t = GameTime::from_unix(1000);
        let t2 = t.add_secs(500);
        assert_eq!(t2.as_secs(), 1500);
    }

    #[test]
    fn packed_wow_time_matches_cpp_bit_layout() {
        let packed =
            pack_wow_time_fields_like_cpp(wow_time_fields_from_tm_like_cpp(124, 4, 23, 4, 13, 7));

        assert_eq!(packed & 0x3F, 7);
        assert_eq!((packed >> 6) & 0x1F, 13);
        assert_eq!((packed >> 11) & 0x07, 4);
        assert_eq!((packed >> 14) & 0x3F, 22);
        assert_eq!((packed >> 20) & 0x0F, 4);
        assert_eq!((packed >> 24) & 0x1F, 24);
        assert_eq!((packed >> 29) & 0x03, 0);
    }

    #[test]
    fn packed_wow_time_uses_real_calendar_math_at_year_boundary() {
        let packed = pack_wow_time_fields_like_cpp(utc_wow_time_fields_like_cpp(1_735_689_540));

        assert_eq!(packed & 0x3F, 59);
        assert_eq!((packed >> 6) & 0x1F, 23);
        assert_eq!((packed >> 11) & 0x07, 2);
        assert_eq!((packed >> 14) & 0x3F, 30);
        assert_eq!((packed >> 20) & 0x0F, 11);
        assert_eq!((packed >> 24) & 0x1F, 24);
    }

    #[test]
    fn test_game_time_has_passed() {
        let past = GameTime::from_unix(0);
        assert!(past.has_passed());

        let future = GameTime::from_unix(u64::MAX / 2);
        assert!(!future.has_passed());
    }

    #[test]
    fn test_diff() {
        let d = Diff::from_ms(100);
        assert_eq!(d.as_ms(), 100);
        assert!((d.as_secs_f32() - 0.1).abs() < 0.001);
    }

    #[test]
    fn interval_timer_defaults_match_cpp() {
        let timer = IntervalTimer::new();
        assert_eq!(timer.interval(), 0);
        assert_eq!(timer.current(), 0);
        assert!(timer.passed());
    }

    #[test]
    fn interval_timer_update_clamps_negative_current_like_cpp() {
        let mut timer = IntervalTimer::new();
        timer.set_current(5);
        timer.update(-20);
        assert_eq!(timer.current(), 0);
    }

    #[test]
    fn interval_timer_passed_uses_current_greater_or_equal_interval_like_cpp() {
        let mut timer = IntervalTimer::new();
        timer.set_interval(100);
        timer.update(99);
        assert!(!timer.passed());
        timer.update(1);
        assert!(timer.passed());
    }

    #[test]
    fn interval_timer_reset_preserves_overshoot_like_cpp() {
        let mut timer = IntervalTimer::new();
        timer.set_interval(100);
        timer.update(250);
        assert!(timer.passed());
        timer.reset();
        assert_eq!(timer.current(), 50);
        assert!(!timer.passed());
    }

    #[test]
    fn interval_timer_reset_before_passed_keeps_current_like_cpp() {
        let mut timer = IntervalTimer::new();
        timer.set_interval(100);
        timer.update(40);
        timer.reset();
        assert_eq!(timer.current(), 40);
    }
}

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
        let secs = self.0 as i64;
        // Calculate local time components from Unix timestamp
        let days = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;

        // Approximate date calculation (not accounting for all edge cases)
        let year = ((days as f64) / 365.25) as u32;
        let remaining_days = days - (year as i64 * 365 + year as i64 / 4);
        let month = (remaining_days / 30).clamp(0, 11) as u32;
        let day = (remaining_days % 30).clamp(0, 30) as u32;
        let weekday = ((days + 4) % 7) as u32; // Jan 1 1970 was Thursday (4)

        (minutes as u32 & 0x3F)
            | ((hours as u32 & 0x1F) << 6)
            | ((weekday & 0x07) << 11)
            | ((day & 0x3F) << 14)
            | ((month & 0x0F) << 20)
            | (((year.wrapping_sub(100)) & 0x1F) << 24)
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
}

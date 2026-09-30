//! Player stat calculation and published projection cases.

use super::super::*;

    #[test]
    fn stats_limits_cap_percentages_only_when_enabled_like_cpp() {
        let disabled = StatsLimitsLikeCpp::default();
        assert!(!disabled.enabled);
        assert_eq!(disabled.dodge, 95.0);
        assert_eq!(disabled.parry, 95.0);
        assert_eq!(disabled.block, 95.0);
        assert_eq!(disabled.crit, 95.0);
        assert_eq!(disabled.clamp_dodge_like_cpp(120.0), 120.0);
        assert_eq!(disabled.clamp_crit_like_cpp(120.0), 120.0);

        let enabled = StatsLimitsLikeCpp {
            enabled: true,
            dodge: 40.0,
            parry: 50.0,
            block: 60.0,
            crit: 70.0,
        };
        assert_eq!(enabled.clamp_dodge_like_cpp(120.0), 40.0);
        assert_eq!(enabled.clamp_parry_like_cpp(120.0), 50.0);
        assert_eq!(enabled.clamp_block_like_cpp(120.0), 60.0);
        assert_eq!(enabled.clamp_crit_like_cpp(120.0), 70.0);
        // C++ is `value > limit`, so the boundary and negatives are untouched.
        assert_eq!(enabled.clamp_dodge_like_cpp(40.0), 40.0);
        assert_eq!(enabled.clamp_block_like_cpp(-5.0), -5.0);
    }

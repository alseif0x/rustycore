//! Immutable name policy. Native regex implementation stays in composition;
//! domain owns locale/profane/global-reserved/SQL-reserved ordering.
use super::{
    CheckedName, NamePolicy, NameRejection, NameRuleError, lower_units, normalize_and_check,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub trait Pattern: Send + Sync {
    fn matches(&self, units: &[u16]) -> Result<bool, NameRuleError>;
}
pub type Patterns = Vec<Arc<dyn Pattern>>;

pub enum Validation {
    Accepted(CheckedName),
    Rejected(NameRejection),
}

pub struct NameRules {
    locale_patterns: [Patterns; 12],
    global_reserved: Patterns,
    sql_reserved: BTreeSet<Vec<u16>>,
    creation_charsets: BTreeMap<u32, u8>,
}

impl NameRules {
    /// Locale patterns must contain profanity followed by locale-reserved
    /// entries, exactly as DB2Stores.cpp:1416-1458. Both yield PROFANE.
    pub fn new(
        locale_patterns: [Patterns; 12],
        global_reserved: Patterns,
        sql_reserved: impl IntoIterator<Item = String>,
        creation_charsets: impl IntoIterator<Item = (u32, u8)>,
    ) -> Self {
        Self {
            locale_patterns,
            global_reserved,
            sql_reserved: sql_reserved
                .into_iter()
                .map(|name| lower_units(&name))
                .collect(),
            creation_charsets: creation_charsets.into_iter().collect(),
        }
    }

    pub fn creation_charset(&self, realm_timezone: u32) -> u32 {
        // ObjectMgr::GetRealmLanguageType: English fallback when absent.
        u32::from(
            self.creation_charsets
                .get(&realm_timezone)
                .copied()
                .unwrap_or(2),
        )
    }

    pub fn check(
        &self,
        name: &str,
        policy: NamePolicy,
        locale: u8,
        skip_sql_reserved: bool,
    ) -> Result<Validation, NameRuleError> {
        let patterns = self
            .locale_patterns
            .get(locale as usize)
            .ok_or(NameRuleError::InvalidLocale)?;
        let checked = match normalize_and_check(name, policy) {
            Ok(checked) => checked,
            Err(rejection) => return Ok(Validation::Rejected(rejection)),
        };
        for pattern in patterns {
            if pattern.matches(checked.lower_units())? {
                return Ok(Validation::Rejected(NameRejection::Profane));
            }
        }
        for pattern in &self.global_reserved {
            if pattern.matches(checked.lower_units())? {
                return Ok(Validation::Rejected(NameRejection::Reserved));
            }
        }
        // Permission 17 bypasses only the SQL reserved-name set, not DB2.
        if !skip_sql_reserved && self.sql_reserved.contains(checked.lower_units()) {
            return Ok(Validation::Rejected(NameRejection::Reserved));
        }
        Ok(Validation::Accepted(checked))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Match(bool, Arc<AtomicUsize>);
    impl Pattern for Match {
        fn matches(&self, units: &[u16]) -> Result<bool, NameRuleError> {
            assert_eq!(units, lower_units("Elune"));
            self.1.fetch_add(1, Ordering::SeqCst);
            Ok(self.0)
        }
    }
    fn policy() -> NamePolicy {
        NamePolicy {
            minimum_units: 2,
            strict_mask: 0,
            creation_charset: 2,
        }
    }
    fn rejected(result: Result<Validation, NameRuleError>, rejection: NameRejection) {
        assert!(matches!(result, Ok(Validation::Rejected(value)) if value == rejection));
    }

    #[test]
    fn prefix_and_locale_patterns_precede_global_and_sql_reserved() {
        let local_calls = Arc::new(AtomicUsize::new(0));
        let global_calls = Arc::new(AtomicUsize::new(0));
        let mut locales: [Patterns; 12] = std::array::from_fn(|_| vec![]);
        locales[6].push(Arc::new(Match(true, local_calls.clone())));
        let rules = NameRules::new(
            locales,
            vec![Arc::new(Match(true, global_calls.clone()))],
            ["Elune".into()],
            [],
        );
        rejected(
            rules.check("A", policy(), 6, false),
            NameRejection::TooShort,
        );
        assert_eq!(local_calls.load(Ordering::SeqCst), 0);
        rejected(
            rules.check("ELUNE", policy(), 6, true),
            NameRejection::Profane,
        );
        assert_eq!(global_calls.load(Ordering::SeqCst), 0);
        rejected(
            rules.check("ELUNE", policy(), 5, true),
            NameRejection::Reserved,
        );
        assert_eq!(global_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn sql_reserved_uses_finite_case_and_permission_only_bypasses_sql() {
        let rules = NameRules::new(
            std::array::from_fn(|_| vec![]),
            vec![],
            ["ẞa".into(), "İab".into()],
            [(1, 4)],
        );
        assert_eq!(rules.creation_charset(1), 4);
        assert_eq!(rules.creation_charset(2), 2);
        rejected(
            rules.check("ßA", policy(), 6, false),
            NameRejection::Reserved,
        );
        assert!(matches!(
            rules.check("ßA", policy(), 6, true),
            Ok(Validation::Accepted(_))
        ));
        assert!(matches!(
            rules.check("Iab", policy(), 6, false),
            Ok(Validation::Accepted(_))
        ));
        assert!(matches!(
            rules.check("Elune", policy(), 12, false),
            Err(NameRuleError::InvalidLocale)
        ));
    }

    #[test]
    fn engine_failure_is_not_a_rejection_or_an_accepted_name() {
        struct Broken;
        impl Pattern for Broken {
            fn matches(&self, _: &[u16]) -> Result<bool, NameRuleError> {
                Err(NameRuleError::Engine)
            }
        }
        let mut locales: [Patterns; 12] = std::array::from_fn(|_| vec![]);
        locales[6].push(Arc::new(Broken));
        let rules = NameRules::new(locales, vec![], Vec::new(), []);
        assert!(matches!(
            rules.check("Elune", policy(), 6, false),
            Err(NameRuleError::Engine)
        ));
    }
}

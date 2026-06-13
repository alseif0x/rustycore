/// Mirrors TrinityCore's `Utf8ToUpperOnlyLatin`.
///
/// The C++ helper name is broader than its implementation: it only uppercases
/// ASCII Basic Latin letters because `wcharToUpperOnlyLatin` is gated by
/// `isBasicLatinCharacter`.
pub fn utf8_to_upper_only_latin_like_cpp(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        output.push(match ch {
            'a'..='z' => ((ch as u8) - b'a' + b'A') as char,
            _ => ch,
        });
    }
    output
}

/// Symmetric Rust helper for C++'s Basic-Latin-only casing boundary.
///
/// TrinityCore has `wcharToLower`, which also lowercases Latin-1 supplement
/// letters. This helper is intentionally narrower: it mirrors the
/// `isBasicLatinCharacter` gate used by `Utf8ToUpperOnlyLatin`, preventing
/// future call sites from accidentally applying Unicode lowercase expansion.
pub fn utf8_to_lower_only_latin_like_cpp(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        output.push(match ch {
            'A'..='Z' => ((ch as u8) - b'A' + b'a') as char,
            _ => ch,
        });
    }
    output
}

/// Byte-wise ASCII case-insensitive substring search matching
/// `StringContainsStringI`.
///
/// C++ calls `std::search` with `std::tolower(char)` on both sides. Rust keeps
/// this deliberately byte-oriented and ASCII-only; it does not perform Unicode
/// case folding.
pub fn string_contains_string_i_like_cpp(haystack: &str, needle: &str) -> bool {
    let needle = needle.as_bytes();
    if needle.is_empty() {
        return true;
    }

    haystack.as_bytes().windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuzzyFindMatch<'a, T> {
    pub score: usize,
    pub value: &'a T,
}

/// Port of `Trinity::Containers::FuzzyFindIn` with the default
/// `StringContainsStringI` predicate.
///
/// Despite the name, the C++ helper is not Levenshtein/Jaro fuzzy matching. It
/// scores each candidate by the number of provided needles that appear as
/// case-insensitive substrings, applies an optional caller bonus, discards zero
/// scores, and returns matches ordered by descending score.
pub fn fuzzy_find_in_like_cpp<'a, T, I, N, S>(
    container: I,
    needles: N,
) -> Vec<FuzzyFindMatch<'a, T>>
where
    T: AsRef<str> + 'a,
    I: IntoIterator<Item = &'a T>,
    N: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fuzzy_find_in_with_bonus_like_cpp(container, needles, |_| 0)
}

pub fn fuzzy_find_in_with_bonus_like_cpp<'a, T, I, N, S, B>(
    container: I,
    needles: N,
    bonus: B,
) -> Vec<FuzzyFindMatch<'a, T>>
where
    T: AsRef<str> + 'a,
    I: IntoIterator<Item = &'a T>,
    N: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: Fn(&T) -> usize,
{
    let needles = needles
        .into_iter()
        .map(|needle| needle.as_ref().to_owned())
        .collect::<Vec<_>>();
    let mut results = Vec::new();

    for value in container {
        let mut score = needles
            .iter()
            .filter(|needle| string_contains_string_i_like_cpp(value.as_ref(), needle))
            .count();

        if score == 0 {
            continue;
        }

        score += bonus(value);
        results.push(FuzzyFindMatch { score, value });
    }

    results.sort_by(|left, right| right.score.cmp(&left.score));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_to_upper_only_latin_uppercases_ascii_like_cpp() {
        assert_eq!(
            utf8_to_upper_only_latin_like_cpp("account:name-123"),
            "ACCOUNT:NAME-123"
        );
    }

    #[test]
    fn utf8_to_upper_only_latin_preserves_non_basic_latin_like_cpp() {
        assert_eq!(
            utf8_to_upper_only_latin_like_cpp("cafe cafe cafe"),
            "CAFE CAFE CAFE"
        );
        assert_eq!(utf8_to_upper_only_latin_like_cpp("caféÀßÿ"), "CAFéÀßÿ");
    }

    #[test]
    fn utf8_to_upper_only_latin_does_not_apply_unicode_expansions_like_cpp() {
        assert_eq!(utf8_to_upper_only_latin_like_cpp("straße"), "STRAßE");
        assert_eq!(utf8_to_upper_only_latin_like_cpp("κόσμος"), "κόσμος");
    }

    #[test]
    fn utf8_to_lower_only_latin_lowercases_ascii_like_cpp_boundary() {
        assert_eq!(
            utf8_to_lower_only_latin_like_cpp("ACCOUNT:NAME-123"),
            "account:name-123"
        );
    }

    #[test]
    fn utf8_to_lower_only_latin_preserves_non_basic_latin_like_cpp_boundary() {
        assert_eq!(utf8_to_lower_only_latin_like_cpp("CAFÉÀẞŸ"), "cafÉÀẞŸ");
        assert_eq!(utf8_to_lower_only_latin_like_cpp("ΚΌΣΜΟΣ"), "ΚΌΣΜΟΣ");
    }

    #[test]
    fn string_contains_string_i_uses_ascii_case_insensitive_substring_like_cpp() {
        assert!(string_contains_string_i_like_cpp(
            "Stormwind Harbor",
            "WIND"
        ));
        assert!(string_contains_string_i_like_cpp("abc", ""));
        assert!(!string_contains_string_i_like_cpp("Darnassus", "iron"));
    }

    #[test]
    fn string_contains_string_i_does_not_unicode_fold_like_cpp_byte_search() {
        assert!(string_contains_string_i_like_cpp("straße", "STRA"));
        assert!(!string_contains_string_i_like_cpp("straße", "STRASSE"));
    }

    #[test]
    fn fuzzy_find_in_scores_by_number_of_matching_needles_like_cpp() {
        let values = vec![
            "Stormwind Harbor".to_owned(),
            "Stormwind Keep".to_owned(),
            "Ironforge".to_owned(),
        ];

        let matches = fuzzy_find_in_like_cpp(&values, ["storm", "harbor"]);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].score, 2);
        assert_eq!(matches[0].value, "Stormwind Harbor");
        assert_eq!(matches[1].score, 1);
        assert_eq!(matches[1].value, "Stormwind Keep");
    }

    #[test]
    fn fuzzy_find_in_applies_bonus_after_positive_match_like_cpp() {
        let values = vec![
            "tele stormwind".to_owned(),
            "tele stormwind harbor".to_owned(),
            "tele ironforge".to_owned(),
        ];

        let matches = fuzzy_find_in_with_bonus_like_cpp(&values, ["storm"], |value| {
            usize::from(value.contains("harbor"))
        });

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].score, 2);
        assert_eq!(matches[0].value, "tele stormwind harbor");
        assert_eq!(matches[1].score, 1);
        assert_eq!(matches[1].value, "tele stormwind");
    }
}

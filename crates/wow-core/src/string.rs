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
}

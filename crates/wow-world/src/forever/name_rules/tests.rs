use super::*;
fn policy(strict_mask: u32, creation_charset: u32) -> NamePolicy {
    NamePolicy {
        minimum_units: 2,
        strict_mask,
        creation_charset,
    }
}
fn reject(name: &str, policy: NamePolicy, expected: NameRejection) {
    assert_eq!(normalize_and_check(name, policy).err(), Some(expected));
}

#[test]
fn finite_source_casing_does_not_expand_sharp_s_or_follow_unicode_tables() {
    assert_eq!(
        normalize_and_check("ßA", policy(0, 0)).unwrap().spelling(),
        "ẞa"
    );
    assert_eq!(
        normalize_and_check("ÉLuNE", policy(0, 0))
            .unwrap()
            .spelling(),
        "Élune"
    );
    assert_eq!(
        normalize_and_check("ЁЛКА", policy(0, 0))
            .unwrap()
            .spelling(),
        "Ёлка"
    );
    // Dotted I and Greek Sigma are outside the source's finite mapping,
    // unlike Unicode casing which expands/modifies them.
    assert_eq!(lower_units("İΣẞŒŸ"), vec![0x130, 0x3A3, 0xDF, 0x153, 0xFF]);
    assert_eq!(lower_units("😀"), vec![0xD83D, 0xDE00]);
}

#[test]
fn ordering_uses_utf16_length_before_charset_and_triples() {
    reject("", policy(1, 2), NameRejection::NoName);
    reject("A", policy(1, 2), NameRejection::TooShort);
    reject(
        "A".repeat(13).as_str(),
        policy(1, 2),
        NameRejection::TooLong,
    );
    reject(
        "😀".repeat(7).as_str(),
        policy(1, 2),
        NameRejection::TooLong,
    );
    assert_eq!(
        normalize_and_check(&"😀".repeat(6), policy(2, 0))
            .unwrap()
            .lower_units()
            .len(),
        12
    );
    reject("AAA1", policy(1, 2), NameRejection::MixedLanguages);
    reject("aAa", policy(1, 2), NameRejection::ThreeConsecutive);
    assert!(normalize_and_check("Aa", policy(1, 2)).is_ok());
}

#[test]
fn strict_modes_keep_whole_families_and_source_any_realm_bypass() {
    for strict in [0, 1, 2, 3] {
        assert!(normalize_and_check("Thrall", policy(strict, 2)).is_ok());
    }
    assert!(normalize_and_check("Málaga", policy(0, 2)).is_ok());
    reject("Málaga", policy(2, 2), NameRejection::MixedLanguages);
    assert!(normalize_and_check("Málaga", policy(2, 1)).is_ok());
    assert!(normalize_and_check("Ёлка", policy(2, 4)).is_ok());
    reject("Aя", policy(0, 0), NameRejection::MixedLanguages);
    reject("Aя", policy(2, 1 | 4), NameRejection::MixedLanguages);
    assert!(normalize_and_check("A1 !", policy(2, 0)).is_ok());
    reject("A1", policy(0, 0), NameRejection::MixedLanguages);
    reject("Aa", policy(4, 2), NameRejection::MixedLanguages);
    assert!(normalize_and_check("Aa", policy(3, 4)).is_ok());
}

#[test]
fn character_family_boundaries_match_source_not_unicode_categories() {
    for unit in [0xD7, 0xF7, 0xFF, 0x130, 0x178, 0x3A3] {
        assert!(!extended(unit));
    }
    for unit in [0xDF, 0xFE, 0x12F, 0x1E9E] {
        assert!(extended(unit));
    }
    assert!(korean(0xFF01));
    assert!(!korean(0xFF00));
    assert!(chinese(0x9FFF));
    assert!(!chinese(0xA000));
    assert!(cyrillic(0x401));
    assert!(!cyrillic(0x402));
    assert_eq!(NameRejection::ThreeConsecutive.wire_result(), 107);
}

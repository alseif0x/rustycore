//! Narrow Forever name-pattern capability backed by target-compatible
//! Boost.Regex.  The composition layer owns the native engine; wow-data and
//! wow-world remain independent of C++ and of process-global locale state.
//!
//! The target source (`02245dcd`, `Regex.h:21-32` and
//! `DB2Stores.cpp:1416-1458`) uses `boost::wregex` with Perl, icase and
//! optimize flags.  UTF-8 DB2 strings become UTF-16 units before widening to
//! `wchar_t` on Linux.  The C++ bridge keeps that representation and never
//! crosses `std::wstring`, `std::locale`, or a pattern value over the ABI.

use std::{ffi::c_void, ptr::NonNull, sync::Arc};

use anyhow::anyhow;
use wow_world::forever::name_rules::{NameRuleError, Pattern};

const RESULT_OK: i32 = 0;
const RESULT_INVALID_LOCALE: i32 = 1;

// This binary intentionally does not consume the package's legacy library.
// Cargo attaches build-script link libraries to that library target, so retain
// the target-only archive explicitly at the ABI consumer as well.
#[link(name = "rustycore_forever_name_regex", kind = "static")]
unsafe extern "C" {
    fn rustycore_forever_name_pattern_compile(
        pattern: *const u16,
        pattern_length: usize,
        output: *mut *mut c_void,
    ) -> i32;
    fn rustycore_forever_name_pattern_matches(
        pattern: *const c_void,
        candidate: *const u16,
        candidate_length: usize,
        matched: *mut i32,
    ) -> i32;
    fn rustycore_forever_name_pattern_destroy(pattern: *mut c_void);
}

/// Compile one DB2 pattern without retaining its spelling in a diagnostic.
pub(super) fn compile(pattern: &str) -> anyhow::Result<Arc<dyn Pattern>> {
    let units: Vec<u16> = pattern.encode_utf16().collect();
    let mut output = std::ptr::null_mut();
    // The bridge accepts a null pointer for an empty input, but Rust's slice
    // pointer is also valid for the zero-length case.  No C++ dereference is
    // permitted unless the length is nonzero.
    let result =
        unsafe { rustycore_forever_name_pattern_compile(units.as_ptr(), units.len(), &mut output) };
    if result != RESULT_OK {
        return Err(match result {
            RESULT_INVALID_LOCALE => anyhow!("name regex locale unavailable"),
            _ => anyhow!("name regex engine rejected pattern"),
        });
    }

    let handle = NonNull::new(output).ok_or_else(|| anyhow!("name regex returned no handle"))?;
    Ok(Arc::new(CompiledPattern { handle }))
}

struct CompiledPattern {
    handle: NonNull<c_void>,
}

impl Pattern for CompiledPattern {
    fn matches(&self, units: &[u16]) -> std::result::Result<bool, NameRuleError> {
        let mut matched = 0;
        let result = unsafe {
            rustycore_forever_name_pattern_matches(
                self.handle.as_ptr(),
                units.as_ptr(),
                units.len(),
                &mut matched,
            )
        };
        match result {
            RESULT_OK => Ok(matched != 0),
            RESULT_INVALID_LOCALE => Err(NameRuleError::InvalidLocale),
            _ => Err(NameRuleError::Engine),
        }
    }
}

impl Drop for CompiledPattern {
    fn drop(&mut self) {
        unsafe { rustycore_forever_name_pattern_destroy(self.handle.as_ptr()) }
    }
}

// The C++ object is immutable after construction.  Its match operation uses
// only the immutable compiled expression and a caller-owned candidate; the
// bridge creates/imbues a locale during compilation and never mutates the
// process-global locale.  This is the narrow, reviewed unsafe boundary that
// permits Pattern: Send + Sync. The standalone bridge explicitly enables
// BOOST_HAS_THREADS so the vendor's process-wide caches remain synchronized.
unsafe impl Send for CompiledPattern {}
unsafe impl Sync for CompiledPattern {}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn assert_match(pattern: &dyn Pattern, candidate: &[u16], expected: bool) {
        match pattern.matches(candidate) {
            Ok(actual) if actual == expected => {}
            Ok(_) => panic!("name regex returned an unexpected match result"),
            Err(_) => panic!("name regex matching failed"),
        }
    }

    #[test]
    fn perl_case_and_character_classes_match() {
        let pattern = compile(r"^a+b$").expect("valid regex should compile");
        assert_match(&*pattern, &units("AAb"), true);
        assert_match(&*pattern, &units("Aac"), false);
    }

    #[test]
    fn word_boundaries_and_perl_backreferences_are_preserved() {
        let boundary = compile(r"\bcat\b").expect("word-boundary regex should compile");
        assert_match(&*boundary, &units("the CAT!"), true);
        assert_match(&*boundary, &units("scatter"), false);
        assert_match(&*boundary, &units("catfish"), false);

        let backreference = compile(r"(ab)\1").expect("backreference should compile");
        assert_match(&*backreference, &units("abab"), true);
        assert_match(&*backreference, &units("abac"), false);
    }

    #[test]
    fn surrogate_units_are_not_decoded_as_a_single_scalar() {
        let pattern = compile("😀").expect("supplementary pattern should compile");
        assert_match(&*pattern, &units("😀"), true);
        let mut different = units("😀");
        different[1] = 0xDE00u16 - 1;
        assert_match(&*pattern, &different, false);
    }

    #[test]
    fn unicode_candidate_outside_the_pattern_is_rejected() {
        let pattern = compile("^ä$").expect("valid Unicode regex should compile");
        assert_match(&*pattern, &units("a"), false);
    }

    #[test]
    fn compiled_pattern_can_be_shared_for_concurrent_matches() {
        let pattern = compile(r"^a+b$").expect("valid regex should compile");
        std::thread::scope(|scope| {
            for (candidate, expected) in [("AAb", true), ("Aac", false)] {
                let pattern = std::sync::Arc::clone(&pattern);
                scope.spawn(move || assert_match(&*pattern, &units(candidate), expected));
            }
        });
    }

    #[test]
    fn independent_concurrent_compilation_and_matching_keep_vendor_caches_safe() {
        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    for _ in 0..32 {
                        let pattern = compile(r"^a+b$").expect("threaded regex compilation");
                        assert_match(&*pattern, &units("AAb"), true);
                        assert_match(&*pattern, &units("Aac"), false);
                    }
                });
            }
        });
    }

    #[test]
    fn invalid_pattern_is_rejected_without_fallback() {
        assert!(compile("(").is_err());
    }
}

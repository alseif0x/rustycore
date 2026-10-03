// Single-threaded Boost.Regex parity oracle for the Forever adapter.
//
// This is intentionally a standalone diagnostic source.  It declares the
// bridge ABI instead of including bridge.cpp, builds source-style default
// wregex values under the target global locale, and restores that locale after
// each case.  It prints case counts and error metadata only; patterns and
// candidate names are never rendered.

#ifndef BOOST_REGEX_STANDALONE
#define BOOST_REGEX_STANDALONE 1
#endif
#ifndef BOOST_HAS_THREADS
#define BOOST_HAS_THREADS 1
#endif

#include <boost/regex.hpp>

#include <cstddef>
#include <cstdint>
#include <iostream>
#include <locale>
#include <string>
#include <vector>

extern "C" int rustycore_forever_name_pattern_compile(
    std::uint16_t const*,
    std::size_t,
    void**) noexcept;
extern "C" int rustycore_forever_name_pattern_matches(
    void const*,
    std::uint16_t const*,
    std::size_t,
    int*) noexcept;
extern "C" void rustycore_forever_name_pattern_destroy(void*) noexcept;

namespace
{
static_assert(sizeof(wchar_t) == 4,
    "Forever name regex oracle currently supports Linux wchar_t32 only");
static_assert(sizeof(std::uint16_t) == 2);

constexpr int RESULT_OK = 0;
constexpr int RESULT_INVALID_LOCALE = 1;

using Units = std::vector<std::uint16_t>;

Units units(char16_t const* text)
{
    Units result;
    for (std::size_t i = 0; text[i] != u'\0'; ++i)
        result.push_back(static_cast<std::uint16_t>(text[i]));
    return result;
}

Units surrogate_pair()
{
    return {0xD83D, 0xDE00};
}

bool widen(Units const& input, std::wstring& output)
{
    output.clear();
    if (input.empty())
        return true;
    output.reserve(input.size());
    for (std::uint16_t unit : input)
        output.push_back(static_cast<wchar_t>(unit));
    return true;
}

int target_locale(std::locale& output) noexcept
{
    try
    {
        output = std::locale(std::locale(""), std::locale("C"), std::locale::numeric);
        return RESULT_OK;
    }
    catch (...)
    {
        return RESULT_INVALID_LOCALE;
    }
}

class GlobalLocaleGuard
{
public:
    bool install() noexcept
    {
        try
        {
            previous_ = std::locale();
            std::locale wanted;
            if (target_locale(wanted) != RESULT_OK)
                return false;
            std::locale::global(wanted);
            active_ = true;
            return true;
        }
        catch (...)
        {
            return false;
        }
    }

    bool restore_now() noexcept
    {
        if (!active_)
            return true;
        try
        {
            std::locale::global(previous_);
            active_ = false;
            return true;
        }
        catch (...)
        {
            return false;
        }
    }

    ~GlobalLocaleGuard() noexcept
    {
        // A second attempt keeps the oracle from leaving the process-global
        // locale changed even when the explicit restoration reports failure.
        if (active_)
        {
            try
            {
                std::locale::global(previous_);
            }
            catch (...)
            {
            }
        }
    }

private:
    std::locale previous_;
    bool active_ = false;
};

struct Evaluation
{
    bool locale_ok = false;
    bool restored = false;
    bool valid = false;
    bool matched = false;
    int error = 0;
};

Evaluation source_evaluate(Units const& pattern, Units const& candidate)
{
    Evaluation result;
    GlobalLocaleGuard locale;
    if (!locale.install())
    {
        result.error = RESULT_INVALID_LOCALE;
        return result;
    }
    result.locale_ok = true;

    try
    {
        std::wstring wide_pattern;
        std::wstring wide_candidate;
        widen(pattern, wide_pattern);
        widen(candidate, wide_candidate);
        boost::wregex expression(
            wide_pattern,
            boost::regex::perl | boost::regex::icase | boost::regex::optimize);
        result.valid = true;
        result.matched = boost::regex_search(wide_candidate, expression);
    }
    catch (...)
    {
        result.error = 2;
    }
    result.restored = locale.restore_now();
    return result;
}

Evaluation bridge_evaluate(Units const& pattern, Units const& candidate)
{
    Evaluation result;
    void* handle = nullptr;
    result.error = rustycore_forever_name_pattern_compile(
        pattern.data(), pattern.size(), &handle);
    if (result.error != RESULT_OK || handle == nullptr)
    {
        // RESULT_ENGINE is also the expected invalid-pattern result.  Locale
        // failure is the only compile error that prevents us from classifying
        // the bridge's construction attempt as locale-ready.
        result.locale_ok = result.error != RESULT_INVALID_LOCALE;
        result.restored = true;
        return result;
    }

    int matched = 0;
    result.error = rustycore_forever_name_pattern_matches(
        handle, candidate.data(), candidate.size(), &matched);
    rustycore_forever_name_pattern_destroy(handle);
    if (result.error == RESULT_OK)
    {
        result.valid = true;
        result.locale_ok = true;
        result.matched = matched != 0;
        result.restored = true;
    }
    return result;
}

struct Case
{
    Units pattern;
    Units candidate;
    bool expected_valid;
    bool expected_match;
};
}

int main()
{
    // The process is deliberately single-threaded: this temporary global
    // locale guard mirrors the source default-regex construction safely only
    // when no other thread can observe the transition.
    std::vector<Case> cases = {
        {units(u"^a+b$"), units(u"AAb"), true, true},
        {units(u"\\bcat\\b"), units(u"the CAT!"), true, true},
        {units(u"\\bcat\\b"), units(u"catfish"), true, false},
        {units(u"\\<cat\\>"), units(u"cat"), true, true},
        {units(u"cat(?=dog)"), units(u"catdog"), true, true},
        {units(u"(ab)\\1"), units(u"abab"), true, true},
        {units(u"^cat$"), units(u"dog"), true, false},
        {units(u"^\u00E4$"), units(u"\u00C4"), true, true},
        {units(u"^\u00E4$"), units(u"a"), true, false},
        {surrogate_pair(), surrogate_pair(), true, true},
        {units(u"("), units(u"irrelevant"), false, false},
    };

    std::size_t failed = 0;
    std::size_t bridge_errors = 0;
    std::size_t source_errors = 0;
    std::size_t locale_restore_errors = 0;
    for (Case const& test : cases)
    {
        Evaluation source = source_evaluate(test.pattern, test.candidate);
        Evaluation bridge = bridge_evaluate(test.pattern, test.candidate);
        if (bridge.error != RESULT_OK)
            ++bridge_errors;
        if (source.error != 0)
            ++source_errors;
        if (!source.restored)
            ++locale_restore_errors;

        bool pass = source.locale_ok && source.restored && bridge.locale_ok
            && source.error == (test.expected_valid ? RESULT_OK : 2)
            && bridge.error == (test.expected_valid ? RESULT_OK : 2)
            && source.valid == test.expected_valid
            && bridge.valid == test.expected_valid
            && (!test.expected_valid || (source.matched == test.expected_match
                && bridge.matched == test.expected_match
                && source.matched == bridge.matched));
        if (!pass)
            ++failed;
    }

    std::cout << "name_regex_oracle " << (failed == 0 ? "PASS" : "FAIL")
              << " cases=" << cases.size()
              << " failed=" << failed
              << " bridge_errors=" << bridge_errors
              << " source_errors=" << source_errors
              << " locale_restore_errors=" << locale_restore_errors << '\n';
    return failed == 0 ? 0 : 1;
}

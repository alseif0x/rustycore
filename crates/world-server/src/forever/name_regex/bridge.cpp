// Boost.Regex bridge for the build-70170 name-rule engine.
//
// The target constructs boost::wregex values from UTF-16 units widened to
// wchar_t.  This file intentionally keeps that conversion on the C++ side:
// std::wstring and std::locale never cross the Rust ABI boundary.

#ifndef BOOST_REGEX_STANDALONE
#define BOOST_REGEX_STANDALONE 1
#endif
// Standalone omits Boost.Config, which normally defines this threading
// capability. C++17 mutexes are available on the verified Linux host; retain
// Boost's own cache synchronization, not an extra Rust/session lock.
#ifndef BOOST_HAS_THREADS
#define BOOST_HAS_THREADS 1
#endif

#include <boost/regex.hpp>

#include <cstddef>
#include <cstdint>
#include <locale>
#include <memory>
#include <new>
#include <string>

namespace
{
static_assert(sizeof(wchar_t) == 4,
    "Forever name regex bridge currently supports Linux wchar_t32 only");
static_assert(sizeof(std::uint16_t) == 2);

constexpr int RESULT_OK = 0;
constexpr int RESULT_INVALID_LOCALE = 1;
constexpr int RESULT_ENGINE = 2;

struct PatternHandle
{
    boost::wregex expression;
};

bool widen_utf16(std::uint16_t const* units, std::size_t length, std::wstring& result)
{
    result.clear();
    if (length == 0)
        return true;
    if (units == nullptr)
        return false;

    result.reserve(length);
    for (std::size_t i = 0; i < length; ++i)
        result.push_back(static_cast<wchar_t>(units[i]));
    return true;
}

int make_regex_locale(std::locale& result) noexcept
{
    try
    {
        // This is the target's UTF-8 process locale with numeric punctuation
        // retained from the classic C locale (Locales.cpp:28-41).  Unlike the
        // target process bootstrap, the adapter never mutates the global
        // locale.
        result = std::locale(std::locale(""), std::locale("C"), std::locale::numeric);
        return RESULT_OK;
    }
    catch (...)
    {
        return RESULT_INVALID_LOCALE;
    }
}
}

extern "C" int rustycore_forever_name_pattern_compile(
    std::uint16_t const* pattern,
    std::size_t pattern_length,
    void** output) noexcept
{
    if (output == nullptr)
        return RESULT_ENGINE;
    *output = nullptr;

    try
    {
        std::wstring wide_pattern;
        if (!widen_utf16(pattern, pattern_length, wide_pattern))
            return RESULT_ENGINE;

        std::locale regex_locale;
        int locale_result = make_regex_locale(regex_locale);
        if (locale_result != RESULT_OK)
            return locale_result;

        auto handle = std::make_unique<PatternHandle>();
        // The target uses Perl syntax, locale-sensitive case-insensitive
        // matching, and the optimization hint for every DB2 name pattern.
        handle->expression.imbue(regex_locale);
        handle->expression.assign(
            wide_pattern,
            boost::regex::perl | boost::regex::icase | boost::regex::optimize);
        *output = handle.release();
        return RESULT_OK;
    }
    catch (...)
    {
        return RESULT_ENGINE;
    }
}

extern "C" int rustycore_forever_name_pattern_matches(
    void const* pattern,
    std::uint16_t const* candidate,
    std::size_t candidate_length,
    int* matched) noexcept
{
    if (pattern == nullptr || matched == nullptr)
        return RESULT_ENGINE;
    *matched = 0;

    try
    {
        std::wstring wide_candidate;
        if (!widen_utf16(candidate, candidate_length, wide_candidate))
            return RESULT_ENGINE;

        auto const* handle = static_cast<PatternHandle const*>(pattern);
        *matched = boost::regex_search(wide_candidate, handle->expression) ? 1 : 0;
        return RESULT_OK;
    }
    catch (...)
    {
        *matched = 0;
        return RESULT_ENGINE;
    }
}

extern "C" void rustycore_forever_name_pattern_destroy(void* pattern) noexcept
{
    delete static_cast<PatternHandle*>(pattern);
}

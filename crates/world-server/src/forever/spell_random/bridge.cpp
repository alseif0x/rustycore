// Numerical capability only; spell/domain rules remain in Rust.
// Pristine 02245dcd headers/SFMT and extracted frand/urand/urandweighted share TLS.
#include "abi.hpp"
#include "Random.h"
#include "SFMT.h"
#include <cmath>
#include <limits>
#include <random>
#include <vector>

// This freezes reference-server numerical behavior, not the client's toolchain.
static_assert(__GNUC__ == 15 && __GNUC_MINOR__ == 2 && __GNUC_PATCHLEVEL__ == 0);
static_assert(__GLIBCXX__ == 20260321 && _GLIBCXX_RELEASE == 15);
static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559);
#if defined(__clang__) || defined(__FAST_MATH__)
#error Forever source RNG requires the pinned GNU non-fast-math contract
#endif

namespace
{
bool valid_range(float minimum, float maximum)
{
    return std::isfinite(minimum) && std::isfinite(maximum) && maximum >= minimum;
}
struct SeededEngine
{
    using result_type = RandomEngine::result_type;
    sfmt_t* state;
    static constexpr result_type min() { return RandomEngine::min(); }
    static constexpr result_type max() { return RandomEngine::max(); }
    result_type operator()() const { return sfmt_genrand_uint32(state); }
};

bool valid_weights(double const* weights, std::size_t count, bool& weighted)
{
    if (!weights || count == 0 || count - 1 > std::numeric_limits<std::uint32_t>::max())
        return false;
    double sum = 0.0;
    for (std::size_t index = 0; index < count; ++index)
        sum += weights[index];
    // Containers.h:143-159: zero/negative/NaN totals select uniformly.
    weighted = sum > 0.0;
    if (!weighted)
        return true;
    // Reject only undefined discrete_distribution inputs; do not normalize
    // negative/NaN weights in the source's uniform fallback branch.
    if (!std::isfinite(sum))
        return false;
    for (std::size_t index = 0; index < count; ++index)
        if (!std::isfinite(weights[index]) || weights[index] < 0.0)
            return false;
    return true;
}
}

extern "C" int rustycore_forever_spell_select(double const* weights,
    std::size_t count, std::uint32_t* selection) noexcept
{
    bool weighted;
    if (!selection || !valid_weights(weights, count, weighted))
        return 1;
    try
    {
        // Both pristine source functions use the SAME TLS SFMT as frand.
        *selection = weighted ? urandweighted(count, weights) : urand(0, std::uint32_t(count - 1));
        return 0;
    }
    catch (...) { return 2; }
}

extern "C" int rustycore_forever_spell_select_seeded(std::uint32_t seed,
    double const* weights, std::size_t count, std::uint32_t* selections,
    std::size_t draws) noexcept
{
    bool weighted;
    if ((draws != 0 && !selections) || !valid_weights(weights, count, weighted))
        return 1;
    try
    {
        alignas(16) sfmt_t state;
        sfmt_init_gen_rand(&state, seed);
        SeededEngine engine {&state};
        // Source wrappers construct a fresh distribution for EACH call.
        for (std::size_t index = 0; index < draws; ++index)
            selections[index] = weighted
                ? std::discrete_distribution<std::uint32_t>(weights, weights + count)(engine)
                : std::uniform_int_distribution<std::uint32_t>(0, std::uint32_t(count - 1))(engine);
        return 0;
    }
    catch (...) { return 2; }
}

extern "C" int rustycore_forever_spell_random(float minimum, float maximum,
    float* value) noexcept
{
    if (value == nullptr || !valid_range(minimum, maximum))
        return 1;
    try
    {
        *value = frand(minimum, maximum);
        return 0;
    }
    catch (...) { return 2; }
}

extern "C" int rustycore_forever_spell_random_seeded(std::uint32_t seed,
    std::uint32_t const* seed_words, std::size_t seed_length,
    float const* minima, float const* maxima, float* values,
    std::size_t count) noexcept
{
    if (seed_length > std::size_t(std::numeric_limits<int>::max())
        || (seed_length != 0 && seed_words == nullptr)
        || (count != 0 && (minima == nullptr || maxima == nullptr || values == nullptr)))
        return 1;
    // Validate the entire batch before drawing/writing anything.
    for (std::size_t index = 0; index < count; ++index)
        if (!valid_range(minima[index], maxima[index]))
            return 1;
    try
    {
        alignas(16) sfmt_t state;
        if (seed_length == 0)
            sfmt_init_gen_rand(&state, seed);
        else
        {
            // SFMT's API is mutable but never modifies its seed array. Keep a
            // transient copy rather than casting away the caller's constness.
            std::vector<std::uint32_t> words(seed_words, seed_words + seed_length);
            sfmt_init_by_array(&state, words.data(), int(seed_length));
        }
        SeededEngine engine {&state};
        for (std::size_t index = 0; index < count; ++index)
            values[index] = std::uniform_real_distribution<float>(minima[index], maxima[index])(engine);
        return 0;
    }
    catch (...) { return 2; }
}

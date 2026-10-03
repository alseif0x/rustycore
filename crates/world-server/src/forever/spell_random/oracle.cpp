// Source-extracted RandomEngine/frand in a separate seeded namespace, compared
// bit-for-bit to the native ABI. No assets, DB, sessions, passwords or network.
#include "abi.hpp"
#include "SFMT.h"
#include "Define.h"
#include <array>
#include <bit>
#include <cassert>
#include <iostream>
#include <limits>
#include <random>
#include <vector>

namespace reference
{
alignas(16) sfmt_t state;
uint32 rand32() { return sfmt_genrand_uint32(&state); }
// Exact class/function text obtained from immutable target Git objects.
#include "SourceSeededDraw.hpp"
}

int main()
{
    try
    {
        constexpr std::array<std::pair<float, float>, 8> ranges = {{{-0.5f, 0.5f},
            {0.0f, 0.0f}, {-8.0f, -1.0f}, {1.0f, 8.0f},
            {-0.0f, 0.0f}, {4.0f, 4.0f}, {-9999.0f, 9999.0f},
            {-std::numeric_limits<float>::denorm_min(), std::numeric_limits<float>::denorm_min()}}};
        std::vector<float> minima(2048), maxima(2048), actual(2048);
        for (std::size_t index = 0; index < actual.size(); ++index)
        {
            minima[index] = ranges[index % ranges.size()].first;
            maxima[index] = ranges[index % ranges.size()].second;
        }
        std::size_t draws = 0;
        for (std::uint32_t seed : {0u, 1u, 1234u, 0x80000000u, 0xffffffffu})
            for (std::size_t word_count : {0u, 1u, 7u, 624u})
            {
                std::vector<std::uint32_t> words(word_count);
                for (std::size_t index = 0; index < word_count; ++index)
                    words[index] = seed + std::uint32_t(index) * 0x9e3779b9;
                if (word_count == 0)
                    sfmt_init_gen_rand(&reference::state, seed);
                else
                    sfmt_init_by_array(&reference::state, words.data(), int(words.size()));
                if (rustycore_forever_spell_random_seeded(seed, words.data(), words.size(),
                    minima.data(), maxima.data(), actual.data(), actual.size()) != 0)
                    return 1;
                for (std::size_t index = 0; index < actual.size(); ++index)
                {
                    auto expected = reference::frand(minima[index], maxima[index]);
                    if (std::bit_cast<std::uint32_t>(expected) != std::bit_cast<std::uint32_t>(actual[index]))
                        return 1;
                    ++draws;
                }
            }
        float sentinel = 123.5f;
        std::size_t selections = 0;
        std::array<std::array<double, 3>, 6> weight_sets = {{{0.0, 0.0, 0.0},
            {-3.0, 0.0, -1.0}, {std::numeric_limits<double>::quiet_NaN(), 1.0, 2.0},
            {0.1, 0.2, 0.7}, {0.0, 0.0, 1.0}, {1e-300, 1e100, 1.0}}};
        for (auto const& weights : weight_sets)
            for (std::uint32_t seed : {0u, 1u, 1234u, 0x80000000u, 0xffffffffu})
            {
                std::array<std::uint32_t, 2048> actualChoices;
                if (rustycore_forever_spell_select_seeded(seed, weights.data(), weights.size(),
                    actualChoices.data(), actualChoices.size()) != 0)
                    return 1;
                sfmt_init_gen_rand(&reference::state, seed);
                double sum = 0.0;
                for (double weight : weights)
                    sum += weight;
                for (auto choice : actualChoices)
                {
                    auto expected = sum > 0.0
                        ? reference::urandweighted(weights.size(), weights.data())
                        : reference::urand(0, std::uint32_t(weights.size() - 1));
                    if (choice != expected)
                        return 1;
                    ++selections;
                }
            }
        if (rustycore_forever_spell_random(1.0f, -1.0f, &sentinel) != 1 || sentinel != 123.5f
            || rustycore_forever_spell_random(0.0f, 1.0f, nullptr) != 1)
            return 1;
        // Production uses pristine source entropy/TLS. Its sequences are not
        // asserted equal to a fixed seed, only its callable/error ABI here.
        for (auto [minimum, maximum] : ranges)
        {
            float value;
            if (rustycore_forever_spell_random(minimum, maximum, &value) != 0
                || !(value >= minimum && value <= maximum))
                return 1;
        }
        std::cout << "PASS source-random-oracle draws=" << draws << " selections=" << selections
            << " production-ranges=" << ranges.size() << '\n';
        return 0;
    }
    catch (...) { return 1; }
}

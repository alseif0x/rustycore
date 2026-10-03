#pragma once
#include <cstddef>
#include <cstdint>

extern "C" int rustycore_forever_spell_random(float minimum, float maximum,
    float* value) noexcept;
extern "C" int rustycore_forever_spell_select(double const* weights,
    std::size_t count, std::uint32_t* selection) noexcept;
extern "C" int rustycore_forever_spell_select_seeded(std::uint32_t seed,
    double const* weights, std::size_t count, std::uint32_t* selections,
    std::size_t draws) noexcept;

// Deterministic QA only. Never changes or seeds the production thread-local RNG.
extern "C" int rustycore_forever_spell_random_seeded(std::uint32_t seed,
    std::uint32_t const* seed_words, std::size_t seed_length,
    float const* minima, float const* maxima, float* values,
    std::size_t count) noexcept;

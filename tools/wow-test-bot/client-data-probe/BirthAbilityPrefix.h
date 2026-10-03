#ifndef RUSTYCORE_FOREVER_BIRTH_ABILITY_PREFIX_H
#define RUSTYCORE_FOREVER_BIRTH_ABILITY_PREFIX_H

#include <cstdint>
#include <stdexcept>
#include <vector>

namespace BirthAbilityPrefix
{
// Bounded 70170/esES observation, not arbitrary encrypted-table recovery.
// DB2FileLoader.cpp:1915-1926 at 02245dcd skips an unavailable section.
constexpr uint32_t FileDataId = 1266278;
constexpr uint32_t AvailableBytes = 346918;
constexpr uint32_t FullBytes = 347118;
constexpr uint32_t AvailableRecords = 7833;
constexpr uint32_t UnknownRecords = 5;
constexpr uint32_t TableHash = 4282664694;

inline uint32_t Word(std::vector<unsigned char> const& data, size_t offset)
{
    if (offset > data.size() || data.size() - offset < 4)
        throw std::runtime_error("Truncated available birth-ability header");
    return uint32_t(data[offset]) | (uint32_t(data[offset + 1]) << 8) |
        (uint32_t(data[offset + 2]) << 16) | (uint32_t(data[offset + 3]) << 24);
}

inline void Validate(std::vector<unsigned char> const& data)
{
    if (data.size() != AvailableBytes || Word(data, 0) != 0x35434457 || Word(data, 4) != 5 ||
        Word(data, 136) != AvailableRecords + UnknownRecords || Word(data, 140) != 18 ||
        Word(data, 144) != 20 || Word(data, 152) != TableHash || Word(data, 156) != 0x224F7EA0 ||
        Word(data, 172) != (2u << 16) || Word(data, 176) != 18 ||
        Word(data, 184) != 1 || Word(data, 200) != 6)
        throw std::runtime_error("Unexpected available birth-ability schema/extent");

    for (uint32_t i = 0; i < 6; ++i)
    {
        size_t at = 204 + i * 40;
        bool plaintext = Word(data, at) == 0 && Word(data, at + 4) == 0;
        uint32_t records = i == 0 ? AvailableRecords : 1;
        uint32_t offset = i == 0 ? 127580 : AvailableBytes + (i - 1) * 40;
        if (plaintext != (i == 0) || Word(data, at + 8) != offset ||
            Word(data, at + 12) != records || Word(data, at + 16) != (i == 0 ? 2u : 0u) ||
            Word(data, at + 24) != 0 || Word(data, at + 28) != 12 + records * 8 ||
            Word(data, at + 32) != 0 || Word(data, at + 36) != 0)
            throw std::runtime_error("Unexpected available birth-ability section boundary");
    }
    // Records + strings + parent lookup, no external IDs or copy table.
    if (Word(data, 212) + uint64_t(AvailableRecords) * 20 + 2 +
        Word(data, 232) != AvailableBytes)
        throw std::runtime_error("Inconsistent available birth-ability plaintext boundary");
}

inline void SelfTest()
{
    // Synthetic header and zero body only; no client rows or TACT identifiers.
    std::vector<unsigned char> valid(AvailableBytes, 0);
    auto put = [](auto& data, size_t at, uint32_t word)
    {
        for (size_t i = 0; i < 4; ++i) data[at + i] = static_cast<unsigned char>(word >> (i * 8));
    };
    put(valid, 0, 0x35434457); put(valid, 4, 5);
    put(valid, 136, AvailableRecords + UnknownRecords); put(valid, 140, 18);
    put(valid, 144, 20); put(valid, 152, TableHash); put(valid, 156, 0x224F7EA0);
    put(valid, 172, 2u << 16); put(valid, 176, 18); put(valid, 184, 1); put(valid, 200, 6);
    for (uint32_t i = 0; i < 6; ++i)
    {
        size_t at = 204 + i * 40;
        uint32_t records = i == 0 ? AvailableRecords : 1;
        put(valid, at, i == 0 ? 0 : 1);
        put(valid, at + 8, i == 0 ? 127580 : AvailableBytes + (i - 1) * 40);
        put(valid, at + 12, records); put(valid, at + 16, i == 0 ? 2 : 0);
        put(valid, at + 28, 12 + records * 8);
    }
    Validate(valid);
    auto reject = [](auto const& invalid)
    {
        try { Validate(invalid); }
        catch (std::runtime_error const&) { return; }
        throw std::runtime_error("Available birth-ability self-test accepted drift");
    };
    for (size_t at : {size_t(136), size_t(140), size_t(144), size_t(152), size_t(156),
        size_t(172), size_t(176), size_t(184), size_t(200), size_t(212), size_t(220),
        size_t(232), size_t(244), size_t(252), size_t(256), size_t(268), size_t(272)})
    {
        auto invalid = valid; invalid[at] ^= 1; reject(invalid);
    }
    auto invalid = valid; invalid.pop_back(); reject(invalid);
    invalid = valid; invalid.push_back(0); reject(invalid);
}
}
#endif

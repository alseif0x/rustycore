#ifndef RUSTYCORE_FOREVER_SPELL_PREFIXES_H
#define RUSTYCORE_FOREVER_SPELL_PREFIXES_H

#include "SpellInfoSchemas.h"
#include <cstdint>
#include <stdexcept>
#include <vector>

namespace SpellPrefixes
{
struct Contract
{
    size_t schemaIndex;
    uint32_t tableHash, fullBytes, prefixBytes, firstOffset;
    uint32_t records, knownRecords, recordBytes, sections, strings, copies;
};

// Fresh 70170/esES header/section observation, 2026-10-03 07:08 UTC.
// Existing twenty contracts are unchanged. Four custom-attribute dependencies
// were independently observed at 09:46 UTC; other stores stay complete reads.
// DB2FileLoader.cpp:1858-1968 (02245dcd): encrypted Skip, normal records,
// string/ID/copy/parent extents. No encrypted body or TACT value is retained here.
constexpr std::array<Contract, 24> Contracts = {{
    { 0, 1187407512, 711994, 696455, 4260, 18134, 17565, 4, 9, 438291, 14173 },
    { 1, 4030871717, 1749502, 1701488, 89932, 43670, 42409, 26, 9, 2, 0 },
    { 2, 3322146344, 3016982, 2933534, 15280, 32626, 31720, 80, 9, 2, 0 },
    { 3, 4096770149, 209698, 208578, 2388, 12953, 12886, 4, 5, 2, 0 },
    { 4, 3130494798, 9624, 9134, 904, 333, 316, 14, 5, 2, 0 },
    { 5, 1627543382, 37512, 37028, 1188, 3302, 3258, 7, 5, 2, 0 },
    { 6, 3689412649, 189161, 183217, 3224, 10931, 10587, 5, 9, 2, 0 },
    { 7, 680438657, 59695, 59623, 5180, 6057, 6049, 5, 3, 2, 0 },
    { 8, 4193483863, 68837, 67802, 1248, 4501, 4436, 3, 6, 2, 0 },
    { 11, 3830102996, 21625, 21438, 668, 1905, 1888, 7, 4, 2, 0 },
    { 12, 1720692227, 156592, 151816, 3932, 10170, 9858, 3, 9, 2, 0 },
    { 13, 813076512, 61484, 61045, 828, 4662, 4631, 1, 4, 2, 0 },
    { 14, 501138918, 196940, 194513, 2064, 12986, 12829, 3, 7, 2, 0 },
    { 15, 2712461791, 55536, 54957, 3508, 3466, 3429, 7, 3, 2, 0 },
    { 17, 2875640223, 131214, 130834, 97832, 3338, 3300, 6, 5, 2, 0 },
    { 21, 3764692828, 74862, 72638, 1552, 4578, 4442, 4, 5, 2, 0 },
    { 22, 2769259057, 10559, 10541, 828, 1081, 1079, 5, 2, 2, 0 },
    { 23, 666345498, 319037, 309086, 5352, 20905, 20248, 7, 9, 2, 0 },
    { 33, 868868520, 1970, 1962, 976, 90, 89, 4, 2, 2, 34 },
    { 34, 1821637041, 3786, 3746, 1484, 115, 113, 20, 3, 2, 0 },
    { 37, 3764045193, 330036, 329881, 964, 2200, 2199, 124, 2, 47309, 17 },
    { 38, 4146370265, 125674, 124866, 3656, 2292, 2273, 36, 7, 2, 3786 },
    { 39, 1369604944, 35442, 35390, 6496, 723, 722, 32, 2, 2, 0 },
    { 40, 48336690, 49315, 49213, 1680, 2689, 2683, 13, 2, 2, 240 },
}};

inline Contract const* Find(size_t schemaIndex)
{
    for (auto const& contract : Contracts)
        if (contract.schemaIndex == schemaIndex) return &contract;
    return nullptr;
}

inline uint32_t LocaleMask(Contract const& c)
{
    // Observed native header, not the storage-selection locale argument:
    // SpellName/BattlePetSpecies carry esES; numeric files declare all locales.
    return c.schemaIndex == 0 || c.schemaIndex == 34 || c.schemaIndex == 37 ? 64 : 0xFFFFFFFF;
}

inline uint32_t Word(std::vector<unsigned char> const& data, size_t offset)
{
    if (offset > data.size() || data.size() - offset < 4)
        throw std::runtime_error("Truncated spell prefix header");
    return uint32_t(data[offset]) | (uint32_t(data[offset+1]) << 8) |
        (uint32_t(data[offset+2]) << 16) | (uint32_t(data[offset+3]) << 24);
}

inline void Validate(std::vector<unsigned char> const& data, Contract const& c)
{
    auto const& schema = AcquisitionSchemas::SpellInfoSchemas.at(c.schemaIndex);
    uint32_t flags = schema.indexField == -1 ? 4 : 0;
    uint32_t nativeIndex = schema.indexField == -1 ? 0 : schema.indexField;
    bool parent = schema.parentIndexField >= int(schema.fileFieldCount);
    if (data.size() != c.prefixBytes || c.prefixBytes >= c.fullBytes ||
        c.fullBytes > 4 * 1024 * 1024 || Word(data, 0) != 0x35434457 || Word(data, 4) != 5 ||
        Word(data, 136) != c.records || Word(data, 140) != schema.fileFieldCount ||
        Word(data, 144) != c.recordBytes || Word(data, 152) != c.tableHash ||
        Word(data, 156) != schema.layoutHash || Word(data, 168) != LocaleMask(c) ||
        Word(data, 172) != (flags | (nativeIndex << 16)) ||
        Word(data, 176) != schema.fileFieldCount || Word(data, 184) != uint32_t(parent) ||
        Word(data, 200) != c.sections || c.sections < 2)
        throw std::runtime_error("Wrong bounded 70170/esES spell prefix contract");

    uint64_t count = 0, previousOffset = 0;
    for (uint32_t i = 0; i < c.sections; ++i)
    {
        size_t at = 204 + size_t(i) * 40;
        bool plain = Word(data, at) == 0 && Word(data, at + 4) == 0;
        uint32_t records = Word(data, at + 12), offset = Word(data, at + 8);
        uint64_t ids = schema.indexField == -1 ? uint64_t(records) * 4 : 0;
        uint64_t relation = parent ? 12 + uint64_t(records) * 8 : 0;
        uint64_t extent = uint64_t(offset) + uint64_t(records) * c.recordBytes +
            Word(data, at + 16) + ids + uint64_t(Word(data, at + 36)) * 8 + relation;
        if (plain != (i == 0) || offset <= previousOffset || extent > c.fullBytes ||
            Word(data, at + 24) != ids || Word(data, at + 28) != relation ||
            Word(data, at + 32) != 0 ||
            (i == 0 && (offset != c.firstOffset || records != c.knownRecords ||
                Word(data, at + 16) != c.strings || Word(data, at + 36) != c.copies ||
                extent != c.prefixBytes)) ||
            (i == 1 && offset != c.prefixBytes) || (i > 0 && offset < c.prefixBytes))
            throw std::runtime_error("Wrong spell plaintext/unknown section boundary");
        previousOffset = offset;
        count += records;
    }
    if (count != c.records) throw std::runtime_error("Wrong spell section record total");
}

inline void SelfTest()
{
    // Synthetic metadata only: no actual record, string, or TACT identifier.
    auto put = [](auto& data, size_t at, uint32_t value)
    { for (size_t i = 0; i < 4; ++i) data.at(at + i) = static_cast<unsigned char>(value >> (i * 8)); };
    for (auto const& c : Contracts)
    {
        auto const& s = AcquisitionSchemas::SpellInfoSchemas.at(c.schemaIndex);
        bool parent = s.parentIndexField >= int(s.fileFieldCount);
        std::vector<unsigned char> data(c.prefixBytes, 0);
        put(data, 0, 0x35434457); put(data, 4, 5); put(data, 136, c.records);
        put(data, 140, s.fileFieldCount); put(data, 144, c.recordBytes);
        put(data, 152, c.tableHash); put(data, 156, s.layoutHash); put(data, 168, LocaleMask(c));
        put(data, 172, s.indexField == -1 ? 4 : uint32_t(s.indexField) << 16);
        put(data, 176, s.fileFieldCount); put(data, 184, parent); put(data, 200, c.sections);
        uint32_t unknown = c.records - c.knownRecords;
        for (uint32_t i = 0; i < c.sections; ++i)
        {
            size_t at = 204 + size_t(i) * 40;
            uint32_t records = i == 0 ? c.knownRecords : i == 1 ? unknown : 0;
            put(data, at, i == 0 ? 0 : 1);
            put(data, at + 8, i == 0 ? c.firstOffset : c.prefixBytes + i - 1);
            put(data, at + 12, records); put(data, at + 16, i == 0 ? c.strings : 0);
            put(data, at + 24, s.indexField == -1 ? records * 4 : 0);
            put(data, at + 28, parent ? 12 + records * 8 : 0);
            put(data, at + 36, i == 0 ? c.copies : 0);
        }
        Validate(data, c);
        auto reject = [&](auto const& invalid)
        {
            try { Validate(invalid, c); }
            catch (std::runtime_error const&) { return; }
            throw std::runtime_error("Spell prefix self-test accepted drift");
        };
        for (size_t at : {size_t(136), size_t(140), size_t(144), size_t(152), size_t(156),
            size_t(168), size_t(172), size_t(176), size_t(184), size_t(200), size_t(204),
            size_t(212), size_t(216), size_t(220), size_t(228), size_t(232), size_t(236),
            size_t(240), size_t(244), size_t(252), size_t(256)})
        {
            auto invalid = data; invalid[at] ^= 1; reject(invalid);
        }
        auto invalid = data; invalid.pop_back(); reject(invalid);
        invalid = data; invalid.push_back(0); reject(invalid);
    }
}
}
#endif

#ifndef RUSTYCORE_FOREVER_ITEM_PREFIXES_H
#define RUSTYCORE_FOREVER_ITEM_PREFIXES_H
#include <array>
#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <vector>

// Exact 70170/esES observations; source DB2FileLoader.cpp:990-1050,1858-1968
// at 02245dcd. No normal-reader or generic encrypted/sparse relaxation.
namespace ItemPrefixes
{
struct Contract
{
    char const* output;
    uint32_t fileId, hash, layout, fullBytes, prefixBytes, fields, recordSize;
    uint16_t flags;
    uint32_t parentLookups, sections;
    std::array<uint32_t, 8> counts, offsets, copies;
};
constexpr std::array<Contract, 4> Contracts = {{
    { "Item.available.db2", 841626, 0x50238EC2, 0x9A2A4834, 302290, 301411, 16, 9, 4, 0, 8,
      {9033,1,1,1,1,53,1,1}, {1676,301411,301424,301437,301450,301463,302264,302277},
      {22788,0,0,0,0,14,0,0} },
    { "ItemSparse.available.db2", 1572924, 0x919BE54E, 0x6FCC3191, 6999130, 6971318, 68, 356, 5, 0, 8,
      {19167,1,1,1,1,63,1,1}, {2732,6971318,6971702,6972058,6972438,6972806,6998406,6998758},
      {57,0,0,0,0,0,0,0} },
    { "ItemEffect.available.db2", 969941, 0x4002A5B1, 0x4CA77678, 125562, 125074, 9, 7, 4, 0, 7,
      {7580,7,2,2,2,25,2,0}, {1572,125074,125151,125173,125195,125217,125540,0},
      {5015,0,0,0,0,6,0,0} },
    { "ItemXItemEffect.available.db2", 3177687, 0x00CB674F, 0x96F083AD, 190146, 189486, 1, 3, 4, 1, 6,
      {12588,2,2,2,32,2,0,0}, {652,189486,189528,189570,189612,190104,0,0},
      {} },
}};

inline uint32_t Word(std::vector<unsigned char> const& bytes, size_t at)
{
    if (at > bytes.size() || bytes.size() - at < 4)
        throw std::runtime_error("Truncated item-prefix metadata");
    return uint32_t(bytes[at]) | (uint32_t(bytes[at+1]) << 8) |
        (uint32_t(bytes[at+2]) << 16) | (uint32_t(bytes[at+3]) << 24);
}

inline void Validate(std::vector<unsigned char> const& bytes, Contract const& c)
{
    if (bytes.size() != c.prefixBytes || Word(bytes, 0) != 0x35434457 || Word(bytes, 4) != 5 ||
        Word(bytes, 140) != c.fields || Word(bytes, 144) != c.recordSize ||
        Word(bytes, 152) != c.hash || Word(bytes, 156) != c.layout ||
        Word(bytes, 172) != c.flags || Word(bytes, 176) != c.fields ||
        Word(bytes, 184) != c.parentLookups || Word(bytes, 188) != c.fields * 24 ||
        Word(bytes, 200) != c.sections)
        throw std::runtime_error("Wrong bounded item-prefix schema/extent");
    uint64_t total = 0;
    for (uint32_t i = 0; i < c.sections; ++i)
    {
        size_t at = 204 + size_t(i) * 40;
        bool plain = Word(bytes, at) == 0 && Word(bytes, at+4) == 0;
        uint32_t strings = i == 0 && c.flags == 4 ? 2 : 0;
        uint32_t parentBytes = c.parentLookups ? 12 + c.counts[i] * 8 : 0;
        if (plain != (i == 0) || Word(bytes, at+8) != c.offsets[i] ||
            Word(bytes, at+12) != c.counts[i] || Word(bytes, at+16) != strings ||
            Word(bytes, at+24) != c.counts[i] * 4 || Word(bytes, at+28) != parentBytes ||
            Word(bytes, at+32) != (c.flags == 5 ? c.counts[i] : 0) ||
            Word(bytes, at+36) != c.copies[i])
            throw std::runtime_error("Wrong bounded item-prefix section");
        total += c.counts[i];
        // Sparse source reads IDs, copies, then six-byte catalog entries
        // from CatalogDataOffset, followed by the additional IdTableSize
        // bytes read by Load at 1954-1968. Normal uses fixed record extents.
        uint64_t end = c.flags == 5 ? Word(bytes, at+20) :
            uint64_t(c.offsets[i]) + uint64_t(c.counts[i]) * c.recordSize + strings;
        end += uint64_t(c.counts[i]) * 4 + uint64_t(c.copies[i]) * 8 + parentBytes;
        if (c.flags == 5) end += uint64_t(c.counts[i]) * 10;
        uint32_t expectedEnd = i+1 < c.sections ? c.offsets[i+1] : c.fullBytes;
        if (end != expectedEnd)
            throw std::runtime_error("Wrong bounded item-prefix source extent");
    }
    if (Word(bytes, 136) != total || c.offsets[1] != bytes.size())
        throw std::runtime_error("Wrong bounded item-prefix record coverage");
}

inline void Put(std::vector<unsigned char>& bytes, size_t at, uint32_t value)
{
    for (size_t i = 0; i < 4; ++i) bytes.at(at+i) = static_cast<unsigned char>(value >> (i*8));
}

inline void SelfTest()
{
    for (auto const& c : Contracts)
    {
        std::vector<unsigned char> bytes(c.prefixBytes, 0);
        Put(bytes, 0, 0x35434457); Put(bytes, 4, 5);
        Put(bytes, 140, c.fields); Put(bytes, 144, c.recordSize);
        Put(bytes, 152, c.hash); Put(bytes, 156, c.layout); Put(bytes, 172, c.flags);
        Put(bytes, 176, c.fields); Put(bytes, 184, c.parentLookups);
        Put(bytes, 188, c.fields*24); Put(bytes, 200, c.sections);
        uint32_t count = 0;
        for (uint32_t i = 0; i < c.sections; ++i)
        {
            size_t at = 204 + size_t(i) * 40;
            Put(bytes, at, i == 0 ? 0 : 1); Put(bytes, at+8, c.offsets[i]);
            Put(bytes, at+12, c.counts[i]); Put(bytes, at+16, i == 0 && c.flags == 4 ? 2 : 0);
            Put(bytes, at+24, c.counts[i]*4);
            Put(bytes, at+28, c.parentLookups ? 12 + c.counts[i]*8 : 0);
            Put(bytes, at+32, c.flags == 5 ? c.counts[i] : 0);
            Put(bytes, at+36, c.copies[i]);
            if (c.flags == 5)
            {
                uint32_t end = i+1 < c.sections ? c.offsets[i+1] : c.fullBytes;
                Put(bytes, at+20, end - c.counts[i]*14 - c.copies[i]*8);
            }
            count += c.counts[i];
        }
        Put(bytes, 136, count);
        Validate(bytes, c);
        auto reject = [&](std::vector<unsigned char> const& invalid)
        {
            try { Validate(invalid, c); }
            catch (std::runtime_error const&) { return; }
            throw std::runtime_error("Item-prefix self-test admitted drift");
        };
        for (size_t at : {size_t(0), size_t(136), size_t(140), size_t(152), size_t(156),
            size_t(172), size_t(184), size_t(200), size_t(228), size_t(244), size_t(252), size_t(256)})
        {
            auto invalid = bytes; invalid[at] ^= 1; reject(invalid);
        }
        if (c.flags == 5) { auto invalid = bytes; invalid[224] ^= 1; reject(invalid); }
        auto shortBytes = bytes; shortBytes.pop_back(); reject(shortBytes);
        bytes.push_back(0); reject(bytes);
    }
}
}
#endif

// Independent pointer-payload source container, not production replay internals.
// Driver extracts the exact typedef from pinned DB2Stores.cpp Git objects.
#include "abi.hpp"
#include "SourceSkillRaceClassContainer.hpp"
#include <algorithm>
#include <limits>
#include <set>
#include <vector>

namespace
{
bool compare(std::size_t length, unsigned mode)
{
    std::vector<SkillRaceClassInfoEntry> rows;
    rows.reserve(length); // fixture pointer stability; no reserve on source map
    std::vector<ForeverSkillKey> keys;
    for (std::size_t i = 0; i < length; ++i)
    {
        std::uint32_t skill = mode == 0 ? 10 : mode == 1 ? i % 3
            : mode == 2 ? (i * 131u) % 59u : (i % 11 == 0 ? 0 : (i * 97u) % 65536u);
        auto id = i + 1 == length && mode == 3
            ? std::numeric_limits<std::uint32_t>::max() : static_cast<std::uint32_t>(i);
        rows.push_back({id, static_cast<std::uint16_t>(skill)});
        keys.push_back({skill, id});
    }
    SkillRaceClassInfoContainer source;
    std::set<std::uint32_t> skills;
    // Exact source default container and value_type insert; values are stable
    // pointers, not integer payloads as in the production identifiers-only ABI.
    for (SkillRaceClassInfoEntry const& row : rows)
    {
        source.insert(SkillRaceClassInfoContainer::value_type(row.SkillID, &row));
        skills.insert(row.SkillID);
    }
    std::vector<std::uint32_t> expected;
    for (auto skill : skills)
    {
        auto [first, last] = source.equal_range(skill);
        for (; first != last; ++first)
            expected.push_back(first->second->ID);
    }
    std::vector<std::uint32_t> output(length, 99);
    std::size_t written = 77;
    return rustycore_forever_birth_skill_lookup(keys.data(), keys.size(), output.data(), output.size(), &written) == 0
        && written == length && output == expected;
}
}

// Main owns reporting/catch boundary. Negative ABI guards live in Rust cases.
bool birth_lookup_source_oracle(std::size_t& cases)
{
    cases = 0;
    for (std::size_t length : {0, 1, 2, 12, 13, 14, 28, 29, 30, 58, 59, 60,
        126, 127, 128, 256, 257, 258, 1000, 4096, 12000})
        for (unsigned mode = 0; mode != 4; ++mode)
        {
            if (!compare(length, mode))
                return false;
            ++cases;
        }
    return true;
}

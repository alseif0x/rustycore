// 02245dcd DB2Stores.cpp:445,1546-1548,3038-3059; source container only.
// Built in the existing pinned GNU/Boost replay capability. No raw records,
// Player state, matching rules or persistent native container are retained.
#include "abi.hpp"
#include <set>
#include <unordered_map>

extern "C" int rustycore_forever_birth_skill_lookup(
    ForeverSkillKey const* records, std::size_t length,
    std::uint32_t* order, std::size_t capacity, std::size_t* written) noexcept
{
    if (!written)
        return 1;
    *written = 0;
    if (length != capacity || (length && (!records || !order)))
        return 1;
    try
    {
        // The source hashes/equality use uint32 SkillID, not pointer payloads.
        // Keep default hash/container/insertion without reserve/rehash hints.
        std::unordered_multimap<std::uint32_t, std::uint32_t> entries;
        std::set<std::uint32_t> skills;
        for (std::size_t i = 0; i < length; ++i)
        {
            if (i && records[i - 1].record >= records[i].record)
                return 1;
            entries.insert({records[i].skill, records[i].record});
            skills.insert(records[i].skill);
        }
        // No allocations or fallible operations below: all validation precedes
        // any write. Group-key sorting is ABI organization, not a guessed
        // unordered_map global traversal; within each group use equal_range.
        std::size_t at = 0;
        for (std::uint32_t skill : skills)
        {
            auto [first, last] = entries.equal_range(skill);
            for (; first != last; ++first)
                order[at++] = first->second;
        }
        *written = at;
        return 0;
    }
    catch (...)
    {
        return 2;
    }
}

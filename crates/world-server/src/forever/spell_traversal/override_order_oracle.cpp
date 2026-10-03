// Exact m_overrideSpells declaration extracted from immutable Player.h;
// source outer group lifetime plus inner insert/erase(key), independently of
// the production replay. No book-map typedef reused as a set proof.
#include "abi.hpp"
#include "SourcePlayerOverrides.hpp"
#include <algorithm>
#include <limits>
#include <vector>

namespace
{
bool compare(std::size_t length, unsigned mode, unsigned historyMode)
{
    ReferenceOverrideOwner source;
    constexpr std::uint32_t original = 0;
    std::vector<ForeverSpellBookMutation> history;
    auto insert = [&](std::uint32_t id)
    {
        if (source.m_overrideSpells[original].insert(id).second)
            history.push_back({id, 1});
    };
    auto erase = [&](std::uint32_t id)
    {
        auto group = source.m_overrideSpells.find(original);
        if (group == source.m_overrideSpells.end())
            return;
        if (group->second.erase(id))
            history.push_back({id, 2});
        if (group->second.empty())
        {
            source.m_overrideSpells.erase(group);
            history.clear(); // source set and its bucket/link history retired
        }
    };
    std::vector<std::uint32_t> ids;
    for (std::size_t i = 0; i < length; ++i)
        ids.push_back(mode == 0 ? i : mode == 1 ? i * 131u
            : mode == 2 ? i * 257u : std::numeric_limits<std::uint32_t>::max() - i);
    if (mode % 2)
        std::reverse(ids.begin(), ids.end());
    for (auto id : ids)
    {
        insert(id);
        insert(id); // source duplicate calls must not change the witness
    }
    std::vector<std::uint32_t> erased;
    if (historyMode >= 1 && historyMode <= 3)
        for (std::size_t i = 0; i < ids.size(); ++i)
            if (historyMode == 3 || i % 3 == 0)
            {
                erase(ids[i]);
                erase(ids[i]); // absent removal, no extra witness/node
                erased.push_back(ids[i]);
            }
    if (historyMode == 2 || historyMode == 3)
        for (auto i = erased.rbegin(); i != erased.rend(); ++i)
            insert(*i);
    if (historyMode == 4)
    {
        // Group remains live for nonzero lengths: erasing transient members
        // does not shrink its buckets. Rebuilding only final IDs is wrong.
        std::vector<std::uint32_t> transient;
        std::uint32_t id = 1000000000u;
        for (unsigned i = 0; i != 1024; ++i)
        {
            while (source.m_overrideSpells.contains(original)
                && source.m_overrideSpells.at(original).contains(id))
                ++id;
            insert(id);
            transient.push_back(id++);
        }
        for (auto id : transient)
            erase(id);
    }
    std::vector<std::uint32_t> expected;
    if (source.m_overrideSpells.contains(original))
        for (auto id : source.m_overrideSpells.at(original))
            expected.push_back(id);
    std::vector<std::uint32_t> output(expected.size(), 99);
    std::size_t written = 77;
    return rustycore_forever_spell_override_order(history.data(), history.size(),
        output.data(), output.size(), &written) == 0
        && written == output.size() && output == expected;
}
}

bool override_set_source_oracle(std::size_t& cases)
{
    cases = 0;
    for (std::size_t length : {0, 1, 2, 12, 13, 14, 28, 29, 30, 58, 59, 60,
        126, 127, 128, 256, 257, 258, 1000, 4096, 12000})
        for (unsigned mode = 0; mode != 4; ++mode)
            for (unsigned historyMode = 0; historyMode != 5; ++historyMode)
            {
                if (!compare(length, mode, historyMode))
                    return false;
                ++cases;
            }
    return true;
}

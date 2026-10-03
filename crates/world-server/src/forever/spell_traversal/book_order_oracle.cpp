// Independent actual PlayerSpell payload and map typedef, extracted from the
// pinned immutable Player.h. No production container/state implementation reuse.
#include "abi.hpp"
#include "SourcePlayerSpellMap.hpp"
#include <algorithm>
#include <limits>
#include <vector>

namespace
{
bool compare(std::size_t length, unsigned mode, unsigned historyMode)
{
    PlayerSpellMap source;
    std::vector<ForeverSpellBookMutation> history;
    std::vector<std::uint32_t> ids;
    for (std::size_t i = 0; i < length; ++i)
        ids.push_back(mode == 0 ? i : mode == 1 ? i * 131u
            : mode == 2 ? i * 257u : std::numeric_limits<std::uint32_t>::max() - i);
    if (mode % 2)
        std::reverse(ids.begin(), ids.end());
    auto insert = [&](std::uint32_t id)
    {
        // Both source node-creating operations: default construction and
        // actual PlayerSpell payload, not the identifiers-only ABI payload.
        if (id % 2)
        {
            if (!source.try_emplace(id).second)
                return false;
        }
        else
        {
            if (source.contains(id))
                return false;
            source[id];
        }
        auto& payload = source.at(id);
        payload.state = id % 3 ? PLAYERSPELL_TEMPORARY : PLAYERSPELL_REMOVED;
        payload.active = id % 5 != 0;
        payload.dependent = id % 7 != 0;
        payload.disabled = id % 11 != 0;
        payload.favorite = id % 13 != 0;
        payload.Trait = PlayerSpellTrait{};
        history.push_back({id, 1});
        return true;
    };
    auto erase = [&](std::uint32_t id)
    {
        auto entry = source.find(id);
        if (entry == source.end())
            return false;
        source.erase(entry);
        history.push_back({id, 2});
        return true;
    };
    for (auto id : ids)
        if (!insert(id))
            return false;
    std::vector<std::uint32_t> erased;
    if (historyMode >= 1 && historyMode <= 3)
        for (std::size_t i = 0; i < ids.size(); ++i)
            if (historyMode == 3 || i % 3 == 0)
            {
                if (!erase(ids[i]))
                    return false;
                erased.push_back(ids[i]);
            }
    if (historyMode == 2 || historyMode == 3)
        for (auto i = erased.rbegin(); i != erased.rend(); ++i)
            if (!insert(*i))
                return false;
    if (historyMode == 4)
    {
        // A source map does not shrink after erasure. Membership-equivalent
        // reconstruction alone loses the bucket history caused by these nodes.
        std::vector<std::uint32_t> transient;
        std::uint32_t id = 1000000000u;
        for (unsigned i = 0; i != 1024; ++i)
        {
            while (source.contains(id))
                ++id;
            if (!insert(id))
                return false;
            transient.push_back(id++);
        }
        for (auto id : transient)
            if (!erase(id))
                return false;
    }
    std::vector<std::uint32_t> expected;
    for (auto const& [id, payload] : source)
        expected.push_back(id); // Removed/disabled nodes remain physical members
    std::vector<std::uint32_t> output(source.size(), 99);
    std::size_t written = 77;
    return rustycore_forever_spell_book_order(history.data(), history.size(),
        output.data(), output.size(), &written) == 0
        && written == output.size() && output == expected;
}

bool trait_narrowing()
{
    for (auto [input, expected] : std::vector<std::pair<std::int32_t, std::int32_t>>{
        {0, 0}, {0x7fffff, 0x7fffff}, {0x800000, -0x800000}, {0xffffff, -1},
        {0x1000000, 0}, {std::numeric_limits<std::int32_t>::max(), -1},
        {std::numeric_limits<std::int32_t>::min(), 0}, {-1, -1}})
    {
        PlayerSpellTrait trait{};
        trait.DefinitionId = input;
        if (trait.DefinitionId != expected)
            return false;
    }
    for (auto [input, expected] : std::vector<std::pair<std::int32_t, std::int32_t>>{
        {127, 127}, {128, -128}, {255, -1}, {256, 0}, {-129, 127}, {-1, -1}})
    {
        PlayerSpellTrait trait{};
        trait.Rank = input;
        if (trait.Rank != expected)
            return false;
    }
    return true;
}
}

bool spell_book_source_oracle(std::size_t& cases)
{
    cases = 0;
    if (!trait_narrowing())
        return false;
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

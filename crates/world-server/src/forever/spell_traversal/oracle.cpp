// Independent source-shaped container construction. The QA driver obtains
// Hash.h and the exact Difficulty declaration from pinned Git OBJECTS, not the
// potentially sparse/modified reference worktree. No production replay helper
// or production PairHash implementation is imported here.
#include "abi.hpp"
#include <type_traits>
#include "SourceDifficulty.hpp"
#include "ForeverReferenceHash.hpp"
#include <boost/multi_index/composite_key.hpp>
#include <boost/multi_index/hashed_index.hpp>
#include <boost/multi_index/member.hpp>
#include <boost/multi_index_container.hpp>
#include <algorithm>
#include <iostream>
#include <limits>
#include <map>
#include <set>
#include <unordered_map>
#include <vector>

bool birth_lookup_source_oracle(std::size_t& cases);
bool spell_book_source_oracle(std::size_t& cases);
bool override_set_source_oracle(std::size_t& cases);
bool source_cast_resolver_oracle(std::size_t& cases);

namespace
{
using Pair = std::pair<std::uint32_t, Difficulty>;
struct SourceSpell
{
    std::uint32_t const Id;
    Difficulty const DifficultyID;
    SourceSpell(std::uint32_t id, Difficulty difficulty) : Id(id), DifficultyID(difficulty) { }
};
struct SourcePrimary;
struct SourceId;
using SourceMap = boost::multi_index::multi_index_container<
    SourceSpell,
    boost::multi_index::indexed_by<
        boost::multi_index::hashed_unique<
            boost::multi_index::tag<SourcePrimary>,
            boost::multi_index::composite_key<
                SourceSpell,
                boost::multi_index::member<SourceSpell, std::uint32_t const, &SourceSpell::Id>,
                boost::multi_index::member<SourceSpell, Difficulty const, &SourceSpell::DifficultyID>>>,
        boost::multi_index::hashed_non_unique<
            boost::multi_index::tag<SourceId>,
            boost::multi_index::member<SourceSpell, std::uint32_t const, &SourceSpell::Id>>>>;

struct Input
{
    std::vector<ForeverSpellKey> helpers, clients, server;
};
Pair pair(ForeverSpellKey key)
{
    return {key.id, static_cast<Difficulty>(key.difficulty)};
}
ForeverSpellKey key(SourceSpell const& spell)
{
    return {spell.Id, static_cast<std::int16_t>(spell.DifficultyID)};
}
bool equal(std::vector<ForeverSpellKey> const& lhs, std::vector<ForeverSpellKey> const& rhs)
{
    return lhs.size() == rhs.size() && std::equal(lhs.begin(), lhs.end(), rhs.begin(),
        [](auto a, auto b) { return a.id == b.id && a.difficulty == b.difficulty; });
}

// SpellMgr.cpp:2496,2708-2735,2891-2993 with values reduced to const key
// members. Actual target Hash.h supplies std::hash<pair<uint32,Difficulty>>.
SourceMap reference(Input const& input)
{
    std::unordered_map<Pair, std::uint8_t> loadData;
    for (auto helper : input.helpers)
        loadData[pair(helper)] = 0;
    std::set<Pair> named;
    for (auto client : input.clients)
        named.insert(pair(client));
    SourceMap spells;
    for (auto const& [current, unused] : loadData)
        if (named.contains(current))
            spells.emplace(current.first, current.second);
    for (auto server : input.server)
        spells.emplace(server.id, static_cast<Difficulty>(server.difficulty));
    return spells;
}

bool valid_case(Input const& input)
{
    SourceMap spells = reference(input);
    std::vector<ForeverSpellKey> primary(spells.size()), secondary(spells.size());
    std::size_t written = 99;
    if (rustycore_forever_spell_traversal(input.helpers.data(), input.helpers.size(),
        input.clients.data(), input.clients.size(), input.server.data(), input.server.size(),
        primary.data(), secondary.data(), primary.size(), &written) != 0 || written != spells.size())
        return false;
    std::vector<ForeverSpellKey> expected;
    for (SourceSpell const& spell : spells)
        expected.push_back(key(spell));
    if (!equal(primary, expected))
        return false;
    expected.clear();
    auto const& byId = spells.get<SourceId>();
    std::set<std::uint32_t> ids;
    for (SourceSpell const& spell : byId)
    {
        expected.push_back(key(spell));
        ids.insert(spell.Id);
    }
    if (!equal(secondary, expected))
        return false;
    // Crucially, global second-index iteration must retain each equal_range's
    // relative order, not merely contain the same difficulties.
    for (auto id : ids)
    {
        expected.clear();
        auto [begin, end] = byId.equal_range(id);
        for (auto it = begin; it != end; ++it)
            expected.push_back(key(*it));
        std::vector<ForeverSpellKey> actual;
        for (auto current : secondary)
            if (current.id == id)
                actual.push_back(current);
        if (!equal(actual, expected))
            return false;
    }
    return true;
}

Input sized(std::size_t length, bool reverse)
{
    Input input;
    constexpr std::int16_t difficulties[] = {0, 1, -1, std::numeric_limits<std::int16_t>::min()};
    for (std::size_t i = 0; i < length; ++i)
    {
        ForeverSpellKey current{static_cast<std::uint32_t>(i / 4 + 1), difficulties[i % 4]};
        input.helpers.push_back(current);
        if (i % 7 != 3) // unnamed keys still affect helper-map buckets/rehash
            input.clients.push_back(current);
    }
    if (reverse)
        std::reverse(input.helpers.begin(), input.helpers.end());
    if (length != 0)
        input.server = {{50000, -1}, {50000, 0}, {50001, 1}, {50000, 0},
            {std::numeric_limits<std::uint32_t>::max(), std::numeric_limits<std::int16_t>::max()}};
    return input;
}

bool invalid_case(Input const& input, std::size_t capacity)
{
    constexpr ForeverSpellKey sentinel{0xdeadbeef, -7};
    std::vector<ForeverSpellKey> primary(capacity, sentinel), secondary(capacity, sentinel);
    auto before = primary;
    std::size_t written = 99;
    int result = rustycore_forever_spell_traversal(input.helpers.data(), input.helpers.size(),
        input.clients.data(), input.clients.size(), input.server.data(), input.server.size(),
        primary.data(), secondary.data(), capacity, &written);
    return result == 1 && written == 0 && equal(primary, before) && equal(secondary, before);
}
}

int main()
{
    try
    {
        int valid = 0, invalid = 0;
        for (std::size_t length : {0, 1, 2, 12, 13, 14, 28, 29, 30, 52, 53, 54,
            96, 97, 98, 192, 193, 194, 1000, 4096})
            for (bool reverse : {false, true})
            {
                if (!valid_case(sized(length, reverse)))
                    return 1;
                ++valid;
            }
        // More named difficulties than the usual group and nonconsecutive
        // duplicate requests exercise non-unique group links independently.
        Input group;
        for (std::int16_t difficulty = -100; difficulty <= 100; ++difficulty)
            group.helpers.push_back({7, difficulty});
        group.clients = group.helpers;
        group.server = {{9, 0}, {11, -1}, {9, 1}, {9, 0}, {11, 1}, {9, -1}};
        if (!valid_case(group))
            return 1;
        ++valid;

        for (auto const& [input, capacity] : std::vector<std::pair<Input, std::size_t>>{
            {Input{{{1, 0}, {1, 0}}, {{1, 0}}, {}}, 1},
            {Input{{{1, 0}}, {{2, 0}}, {}}, 1},
            {Input{{{1, 0}}, {{1, 0}, {1, 0}}, {}}, 1},
            {Input{{{1, 0}}, {{1, 0}}, {{1, 9}}}, 2},
            {Input{{{1, 0}}, {{1, 0}}, {{2, 0}}}, 1},
            {Input{{{1, 0}}, {{1, 0}}, {{2, 0}}}, 3}})
        {
            if (!invalid_case(input, capacity))
                return 1;
            ++invalid;
        }
        std::size_t written = 99;
        if (rustycore_forever_spell_traversal(nullptr, 0, nullptr, 0, nullptr, 0,
            nullptr, nullptr, 0, &written) != 0 || written != 0)
            return 1;
        ++valid;
        if (rustycore_forever_spell_traversal(nullptr, 1, nullptr, 0, nullptr, 0,
            nullptr, nullptr, 0, &written) != 1 || written != 0)
            return 1;
        ++invalid;
        if (rustycore_forever_spell_traversal(nullptr, 0, nullptr, 0, nullptr, 0,
            nullptr, nullptr, 0, nullptr) != 1)
            return 1;
        ++invalid;
        std::size_t birthCases = 0;
        if (!birth_lookup_source_oracle(birthCases))
            return 1;
        std::size_t bookCases = 0;
        if (!spell_book_source_oracle(bookCases))
            return 1;
        std::size_t overrideCases = 0;
        if (!override_set_source_oracle(overrideCases))
            return 1;
        std::size_t castCases = 0;
        if (!source_cast_resolver_oracle(castCases))
            return 1;
        std::cout << "PASS valid=" << valid << " invalid=" << invalid
            << " birth-equal-range=" << birthCases
            << " player-book-history=" << bookCases
            << " player-override-history=" << overrideCases
            << " source-cast-resolution=" << castCases
            << " primary+secondary+equal-range+book+trait-bits+override-set+cast; synthetic-source-only\n";
        return 0;
    }
    catch (...)
    {
        return 2;
    }
}

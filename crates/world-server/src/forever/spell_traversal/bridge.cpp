// Target 02245dcd SpellMgr.cpp:40-64,2496-2735,2891-2993.
// Replay containers and keys only. Rust retains all correction/gameplay rules.
#include "abi.hpp"
#include <boost/multi_index/composite_key.hpp>
#include <boost/multi_index/hashed_index.hpp>
#include <boost/multi_index/member.hpp>
#include <boost/multi_index_container.hpp>
#include <boost/version.hpp>
#include <functional>
#include <set>
#include <unordered_map>
#include <utility>

// This is an explicit reference-server toolchain contract, not a claim that
// the client mandates Boost/GCC, or that other C++ hosts have this traversal.
static_assert(BOOST_VERSION == 108300);
static_assert(__GNUC__ == 15 && __GNUC_MINOR__ == 2 && __GNUC_PATCHLEVEL__ == 0);
static_assert(__GLIBCXX__ == 20260321 && _GLIBCXX_RELEASE == 15);
static_assert(sizeof(std::size_t) == 8);
#if defined(__clang__) || defined(_GLIBCXX_DEBUG)
#error Forever spell traversal requires the pinned GNU non-debug containers
#endif

namespace
{
enum DifficultyKind : std::int16_t { None = 0 };
using Key = std::pair<std::uint32_t, DifficultyKind>;

struct PairHash
{
    // Exact 64-bit Hash.h::hash_combine/std::hash<pair> arithmetic. Do NOT add
    // noexcept: source hash is not noexcept, so GNU caches node hash codes.
    std::size_t operator()(Key const& key) const
    {
        std::size_t seed = std::hash<std::uint32_t>()(key.first);
        seed = seed + 0x9E3779B9 + std::hash<DifficultyKind>()(key.second);
        constexpr std::size_t m = 0xE9846AF9B1A615D;
        seed ^= seed >> 32;
        seed *= m;
        seed ^= seed >> 32;
        seed *= m;
        seed ^= seed >> 28;
        return seed;
    }
};

struct Entry
{
    std::uint32_t const Id;
    DifficultyKind const Difficulty;
    Entry(std::uint32_t id, DifficultyKind difficulty) : Id(id), Difficulty(difficulty) { }
};
struct SpellIdDifficultyIndex;
struct SpellIdIndex;
using Entries = boost::multi_index::multi_index_container<
    Entry,
    boost::multi_index::indexed_by<
        boost::multi_index::hashed_unique<
            boost::multi_index::tag<SpellIdDifficultyIndex>,
            boost::multi_index::composite_key<
                Entry,
                boost::multi_index::member<Entry, std::uint32_t const, &Entry::Id>,
                boost::multi_index::member<Entry, DifficultyKind const, &Entry::Difficulty>>>,
        boost::multi_index::hashed_non_unique<
            boost::multi_index::tag<SpellIdIndex>,
            boost::multi_index::member<Entry, std::uint32_t const, &Entry::Id>>>>;

Key key(ForeverSpellKey value)
{
    return {value.id, static_cast<DifficultyKind>(value.difficulty)};
}
ForeverSpellKey abi_key(Entry const& entry)
{
    return {entry.Id, static_cast<std::int16_t>(entry.Difficulty)};
}
bool valid_array(void const* pointer, std::size_t length)
{
    return length == 0 || pointer != nullptr;
}
}

extern "C" int rustycore_forever_spell_traversal(
    ForeverSpellKey const* helper_keys, std::size_t helpers_length,
    ForeverSpellKey const* client_keys, std::size_t clients_length,
    ForeverSpellKey const* server_keys, std::size_t server_length,
    ForeverSpellKey* primary, ForeverSpellKey* secondary,
    std::size_t capacity, std::size_t* written) noexcept
{
    if (written == nullptr)
        return 1;
    *written = 0;
    if (!valid_array(helper_keys, helpers_length) || !valid_array(client_keys, clients_length)
        || !valid_array(server_keys, server_length) || !valid_array(primary, capacity)
        || !valid_array(secondary, capacity))
        return 1;
    try
    {
        // Dummy values cannot change iteration: source hashes/equality use
        // keys only and never object addresses. Keep operator[] and defaults,
        // without reserve/rehash hints or a different noexcept hash trait.
        std::unordered_map<Key, std::uint8_t, PairHash> helpers;
        for (std::size_t i = 0; i < helpers_length; ++i)
        {
            auto size = helpers.size();
            helpers[key(helper_keys[i])] = 0;
            if (helpers.size() == size) // history must be first insertions only
                return 1;
        }
        std::set<Key> clients;
        std::set<std::uint32_t> client_ids;
        for (std::size_t i = 0; i < clients_length; ++i)
        {
            Key current = key(client_keys[i]);
            if (helpers.find(current) == helpers.end() || !clients.insert(current).second)
                return 1;
            client_ids.insert(current.first);
        }
        Entries entries;
        for (auto const& [current, unused] : helpers)
            if (clients.contains(current))
                entries.emplace(current.first, current.second);
        // Preserve ALL admitted SQL emplace attempts, including duplicates.
        for (std::size_t i = 0; i < server_length; ++i)
        {
            Key current = key(server_keys[i]);
            if (client_ids.contains(current.first))
                return 1;
            entries.emplace(current.first, current.second);
        }
        if (entries.size() != capacity)
            return 1;

        // No fallible operation after this point; don't publish partial output.
        std::size_t index = 0;
        for (Entry const& entry : entries)
            primary[index++] = abi_key(entry);
        index = 0;
        for (Entry const& entry : entries.get<SpellIdIndex>())
            secondary[index++] = abi_key(entry);
        *written = entries.size();
        return 0;
    }
    catch (...)
    {
        return 2; // never disclose IDs, source payloads or exception text
    }
}

// IDs-only startup ABI. No C++ object, string, raw record or ownership crosses it.
#ifndef RUSTYCORE_FOREVER_SPELL_TRAVERSAL_ABI_HPP
#define RUSTYCORE_FOREVER_SPELL_TRAVERSAL_ABI_HPP
#include <cstddef>
#include <cstdint>

struct ForeverSpellKey
{
    std::uint32_t id;
    std::int16_t difficulty;
};
static_assert(sizeof(ForeverSpellKey) == 8);
static_assert(alignof(ForeverSpellKey) == 4);
static_assert(offsetof(ForeverSpellKey, difficulty) == 4);

// Caller supplies valid arrays with the stated lengths, and disjoint output
// buffers. Null is allowed only for zero-length inputs/output capacity.
// 0=success, 1=invalid input/capacity, 2=engine failure. On failure written=0;
// outputs stay unchanged. All temporary containers retire before return.
extern "C" int rustycore_forever_spell_traversal(
    ForeverSpellKey const* helpers, std::size_t helpers_length,
    ForeverSpellKey const* clients, std::size_t clients_length,
    ForeverSpellKey const* server, std::size_t server_length,
    ForeverSpellKey* primary, ForeverSpellKey* secondary,
    std::size_t capacity, std::size_t* written) noexcept;

struct ForeverSkillKey
{
    std::uint32_t skill;
    std::uint32_t record;
};
static_assert(sizeof(ForeverSkillKey) == 8);
static_assert(alignof(ForeverSkillKey) == 4);
static_assert(offsetof(ForeverSkillKey, record) == 4);

// Same output/failure contract. Inputs are admitted RC records in ascending
// storage-ID order; output groups ascending skill, preserving native equal_range.
extern "C" int rustycore_forever_birth_skill_lookup(
    ForeverSkillKey const* records, std::size_t length,
    std::uint32_t* order, std::size_t capacity, std::size_t* written) noexcept;

struct ForeverSpellBookMutation
{
    std::uint32_t spell;
    std::uint32_t action; // 1=successful node insert, 2=successful node erase
};
static_assert(sizeof(ForeverSpellBookMutation) == 8);
static_assert(alignof(ForeverSpellBookMutation) == 4);
static_assert(offsetof(ForeverSpellBookMutation, action) == 4);

// Same output/failure contract. Capacity must equal final physical node count.
// Duplicate insertion/missing erasure are invalid histories. No payload/state
// crosses the ABI, and the default source container is retired before return.
extern "C" int rustycore_forever_spell_book_order(
    ForeverSpellBookMutation const* mutations, std::size_t length,
    std::uint32_t* order, std::size_t capacity, std::size_t* written) noexcept;

// Same contract, but actual source unordered_set insert / erase(key) replay.
// History is for ONE existing override group. Deleting an empty outer group
// resets that history; map replay is not an unordered_set order substitute.
extern "C" int rustycore_forever_spell_override_order(
    ForeverSpellBookMutation const* mutations, std::size_t length,
    std::uint32_t* order, std::size_t capacity, std::size_t* written) noexcept;
#endif

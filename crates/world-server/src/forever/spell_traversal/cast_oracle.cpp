// Independent resolver QA: the driver extracts actual Player/Unit functions,
// context and enums from pinned Git objects. These minimal collaborators are
// synthetic numeric Unit/SpellMgr/Aura fixtures, NOT live effect executors.
#include "SourceCastEnums.hpp"
#include "SourcePlayerOverrides.hpp"
#include <array>
#include <forward_list>
#include <map>
#include <unordered_map>
#include <unordered_set>
#include <vector>

struct flag128
{
    std::array<uint32, 4> words{};
    explicit operator bool() const
    {
        for (auto word : words) if (word) return true;
        return false;
    }
    flag128 operator&(flag128 const& rhs) const
    {
        flag128 result;
        for (unsigned i = 0; i != 4; ++i) result.words[i] = words[i] & rhs.words[i];
        return result;
    }
};
struct SpellInfo
{
    uint32 Id = 0, SpellFamilyName = 0;
    flag128 SpellFamilyFlags;
    uint32 Attr8 = 0, Attr11 = 0;
    bool IsAffected(uint32 familyName, flag128 const& familyFlags) const;
    bool HasAttribute(SpellAttr8 flag) const { return (Attr8 & flag) != 0; }
    bool HasAttribute(SpellAttr11 flag) const { return (Attr11 & flag) != 0; }
};
struct AuraEffect
{
    struct Effect { flag128 SpellClassMask; int32 MiscValue = 0; } effect;
    SpellInfo const* m_spellInfo = nullptr;
    float amount = 0;
    int32 GetMiscValue() const { return effect.MiscValue; }
    int32 GetAmountAsInt() const { return static_cast<int32>(amount); }
    SpellInfo const* GetSpellInfo() const { return m_spellInfo; }
    Effect const& GetSpellEffectInfo() const { return effect; }
    bool IsAffectingSpell(SpellInfo const* spell) const;
};
struct NumericSpellMgr
{
    std::map<std::pair<uint32, int16>, SpellInfo> spells;
    std::vector<std::pair<uint32, int16>> lookups;
    SpellInfo const* GetSpellInfo(uint32 spell, int16 difficulty)
    {
        lookups.emplace_back(spell, difficulty);
        auto entry = spells.find({spell, difficulty});
        return entry == spells.end() ? nullptr : &entry->second;
    }
};
static NumericSpellMgr* sSpellMgr = nullptr;
struct NumericMap { int16 difficulty = 0; int16 GetDifficultyID() const { return difficulty; } };
struct Unit
{
    #include "SourceCastContext.hpp"
    NumericMap map;
    std::forward_list<AuraEffect*> regular, triggered;
    NumericMap const* GetMap() const { return &map; }
    std::forward_list<AuraEffect*> const& GetAuraEffectsByType(AuraType kind) const
    {
        return kind == SPELL_AURA_OVERRIDE_ACTIONBAR_SPELLS ? regular : triggered;
    }
    virtual SpellInfo const* GetCastSpellInfo(SpellInfo const* info,
        TriggerCastFlags& flags, GetCastSpellInfoContext* context) const;
};
struct Player : Unit, ReferenceOverrideOwner
{
    SpellInfo const* GetCastSpellInfo(SpellInfo const* info,
        TriggerCastFlags& flags, GetCastSpellInfoContext* context) const override;
};
#include "SourceCastFunctions.hpp"

namespace
{
struct Fixture
{
    NumericSpellMgr mgr;
    Player player;
    Fixture()
    {
        sSpellMgr = &mgr;
        for (uint32 id = 1; id != 8; ++id) mgr.spells[{id, 0}].Id = id;
    }
    bool cast(uint32 initial, uint32 expected, uint32 flags = 0)
    {
        Unit::GetCastSpellInfoContext context;
        TriggerCastFlags triggered = TRIGGERED_NONE;
        auto result = player.GetCastSpellInfo(&mgr.spells.at({initial, 0}), triggered, &context);
        return result && result->Id == expected && uint32(triggered) == flags;
    }
};
}

bool source_cast_resolver_oracle(std::size_t& cases)
{
    cases = 0;
    {
        Unit::GetCastSpellInfoContext context;
        if (context.AddSpell(0)) return false;
        for (uint32 id : {1u, 2u, 3u, 4u, 0xffffffffu}) if (!context.AddSpell(id)) return false;
        if (context.AddSpell(5) || context.AddSpell(1) || context.AddSpell(0)) return false;
        ++cases;
    }
    {
        Fixture f;
        if (!f.cast(1, 1) || !f.mgr.lookups.empty()) return false;
        ++cases;
        f.player.m_overrideSpells[1].insert(1);
        if (!f.cast(1, 1) || f.mgr.lookups != std::vector<std::pair<uint32, int16>>{{1, 0}}) return false;
        ++cases;
    }
    {
        Fixture f;
        f.player.m_overrideSpells[1].insert(2);
        f.player.m_overrideSpells[2].insert(1);
        if (!f.cast(1, 1) || f.mgr.lookups != std::vector<std::pair<uint32, int16>>{{2, 0}, {1, 0}}) return false;
        ++cases;
    }
    {
        Fixture f;
        for (uint32 id = 1; id != 7; ++id) f.player.m_overrideSpells[id].insert(id + 1);
        if (!f.cast(1, 6) || f.mgr.lookups.size() != 5) return false;
        ++cases;
    }
    {
        Fixture f;
        auto& group = f.player.m_overrideSpells[1];
        for (uint32 id = 100; id != 106; ++id) group.insert(id);
        // Only the final source-order candidate exists: five failed lookups
        // consume the complete context. Native order is not guessed/sorted.
        uint32 last = 0; for (auto id : group) last = id;
        f.mgr.spells[{last, 0}].Id = last;
        if (!f.cast(1, 1) || f.mgr.lookups.size() != 5) return false;
        ++cases;
    }
    for (uint32 retained : {0u, 4u, 0x400u, 0x404u})
    {
        Fixture f;
        SpellInfo first{20, 0, {}, 0x40000, 0x200};
        SpellInfo second{21, 0, {}, (retained & 4) ? 0x40000u : 0,
            (retained & 0x400) ? 0x200u : 0};
        AuraEffect a{{{}, 1}, &first, 2.0f}, b{{{}, 2}, &second, 3.0f};
        f.player.triggered.push_front(&a);
        f.player.regular.push_front(&b);
        if (!f.cast(1, 3, retained)) return false;
        ++cases;
    }
    {
        Fixture f;
        SpellInfo auraInfo{20, 0, {}, 0x40000, 0x200};
        AuraEffect a{{{}, 1}, &auraInfo, 2.0f};
        f.player.triggered.push_front(&a);
        f.player.m_overrideSpells[1].insert(4);
        if (!f.cast(1, 4)) return false;
        ++cases;
        f.player.m_overrideSpells.erase(1);
        f.player.m_overrideSpells[2].insert(3);
        if (!f.cast(1, 3, 0x444)) return false;
        ++cases;
    }
    {
        Fixture f;
        SpellInfo first{20, 0, {}, 0x40000, 0x200}, second{21};
        AuraEffect a{{{}, 1}, &first, 2.0f}, b{{{}, 2}, &second, 999.0f};
        f.player.triggered.push_front(&a); f.player.regular.push_front(&b);
        if (!f.cast(1, 2, 0x444)) return false;
        ++cases;
    }
    for (unsigned mode = 0; mode != 5; ++mode)
    {
        Fixture f;
        f.mgr.spells.at({1, 0}).SpellFamilyName = 7;
        f.mgr.spells.at({1, 0}).SpellFamilyFlags.words[3] = 4;
        SpellInfo auraInfo{20, mode == 0 ? 0u : mode == 4 ? 8u : 7u};
        AuraEffect a{{{}, 0}, &auraInfo, 2.0f};
        if (mode == 0 || mode == 2) a.effect.SpellClassMask.words[3] = 4;
        if (mode == 3) a.effect.SpellClassMask.words[3] = 8;
        f.player.regular.push_front(&a);
        if (!f.cast(1, mode < 3 ? 2 : 1)) return false;
        ++cases;
    }
    {
        Fixture f;
        f.mgr.spells[{0xffffffffu, 0}].Id = 0xffffffffu;
        SpellInfo info{20, 99};
        AuraEffect a{{{}, -1}, &info, 2.9f};
        f.player.regular.push_front(&a);
        if (!f.cast(0xffffffffu, 2)) return false;
        ++cases;
        a.effect.MiscValue = 1; a.amount = -1.9f;
        if (!f.cast(1, 0xffffffffu)) return false;
        ++cases;
    }
    {
        Fixture f;
        f.player.map.difficulty = 9;
        f.mgr.spells[{2, 9}].Id = 2;
        f.player.m_overrideSpells[1].insert(2);
        if (!f.cast(1, 2) || f.mgr.lookups != std::vector<std::pair<uint32, int16>>{{2, 9}}) return false;
        ++cases;
        f.player.m_overrideSpells[1].clear(); f.player.m_overrideSpells[1].insert(3);
        if (!f.cast(1, 1)) return false; // fixture lookup has NO implicit regular fallback
        ++cases;
    }
    return true;
}

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Module identity, descriptors and the explicit registrar.

use std::collections::BTreeMap;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::effect::{
    EffectBatch, HookOutcome, ModuleEffectError, NotificationEffect, PlayerLoginEffects,
    ScopedEffects, validated,
};
use crate::hook::{PlayerLoginModule, PlayerLoginSnapshot};
use crate::player::{
    CreatureKillSnapshot, PlayerLevelChangedSnapshot, PlayerLogoutSnapshot, PlayerModule,
    PlayerMoneyChangedSnapshot, QuestAcceptedSnapshot, QuestRewardedSnapshot, SpellCastSnapshot,
};
use crate::world::{
    ConfigLoadedSnapshot, WorldEffect, WorldEffects, WorldModule, WorldShutdownSnapshot,
    WorldStartupSnapshot, WorldUpdateSnapshot,
};

/// A validated module identifier.
///
/// Deliberately narrow so an id can be logged, ordered and compared without
/// escaping: lowercase ASCII letters, digits, `_` and `.`, starting with a
/// letter. Rejecting at construction means the registry never holds one that
/// cannot be rendered.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleId(String);

impl ModuleId {
    pub fn new(raw: &str) -> Result<Self, ModuleRegistrationError> {
        if raw.is_empty() || raw.len() > 64 {
            return Err(ModuleRegistrationError::InvalidId {
                id: raw.to_owned(),
                reason: "length must be 1..=64",
            });
        }
        let mut chars = raw.chars();
        let first = chars.next().expect("non-empty");
        if !first.is_ascii_lowercase() {
            return Err(ModuleRegistrationError::InvalidId {
                id: raw.to_owned(),
                reason: "must start with a lowercase ASCII letter",
            });
        }
        if !raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
        {
            return Err(ModuleRegistrationError::InvalidId {
                id: raw.to_owned(),
                reason: "only lowercase ASCII letters, digits, '_' and '.' are allowed",
            });
        }
        Ok(Self(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Semantic-looking module version. No compatibility rule is implied yet.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl fmt::Display for ModuleVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// What a module declares about itself at registration time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleDescriptor {
    pub id: ModuleId,
    pub version: ModuleVersion,
    pub display_name: String,
}

impl ModuleDescriptor {
    pub fn new(
        id: &str,
        version: ModuleVersion,
        display_name: &str,
    ) -> Result<Self, ModuleRegistrationError> {
        let id = ModuleId::new(id)?;
        if display_name.trim().is_empty() || display_name.len() > 128 {
            return Err(ModuleRegistrationError::InvalidDescriptor {
                id: id.to_string(),
                reason: "display name must be non-blank and at most 128 bytes",
            });
        }
        Ok(Self {
            id,
            version,
            display_name: display_name.to_owned(),
        })
    }
}

/// Why a registration was refused.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ModuleRegistrationError {
    InvalidId {
        id: String,
        reason: &'static str,
    },
    InvalidDescriptor {
        id: String,
        reason: &'static str,
    },
    DuplicateId {
        id: String,
    },
    /// A module offered no hook family, so it could never be called.
    NoHooks {
        id: String,
    },
    /// A module refused its configuration, so it never registered.
    Configuration(crate::ModuleConfigError),
}

impl From<crate::ModuleConfigError> for ModuleRegistrationError {
    fn from(error: crate::ModuleConfigError) -> Self {
        Self::Configuration(error)
    }
}

impl fmt::Display for ModuleRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId { id, reason } => write!(f, "invalid module id {id:?}: {reason}"),
            Self::InvalidDescriptor { id, reason } => {
                write!(f, "invalid descriptor for module {id}: {reason}")
            }
            Self::DuplicateId { id } => write!(f, "module {id} is already registered"),
            Self::NoHooks { id } => write!(f, "module {id} registered no hook family"),
            Self::Configuration(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ModuleRegistrationError {}

thread_local! {
    /// Set while this thread runs a module callback under its `catch_unwind`.
    static IN_MODULE_CALLBACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the current thread is inside a linked-module callback.
///
/// A panic there is caught at the module boundary and only disables that
/// module, so the process panic hook can report it as contained instead of
/// fatal (`world-server` wires this into `wow-logging`'s hook). Host plumbing,
/// not part of the module-facing contract.
#[doc(hidden)]
#[must_use]
pub fn module_callback_in_progress() -> bool {
    IN_MODULE_CALLBACK.with(std::cell::Cell::get)
}

/// Marks the callback scope; cleared on return and on unwind alike.
struct CallbackScope;

impl CallbackScope {
    fn enter() -> Self {
        IN_MODULE_CALLBACK.with(|flag| flag.set(true));
        Self
    }
}

impl Drop for CallbackScope {
    fn drop(&mut self) {
        IN_MODULE_CALLBACK.with(|flag| flag.set(false));
    }
}

/// A hook family: one capability a module may offer.
///
/// The registry is indexed by capability, not by a single trait: a module
/// declares exactly the families it implements through [`ModuleHooks`], and a
/// dispatch only visits the modules that offered that family. Families mirror
/// AzerothCore's script classes (`ScriptDefines/*.h`) one group at a time.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum HookFamily {
    /// `player.login`, source API 1. AzerothCore `PlayerScript::OnPlayerLogin`
    /// (`PlayerScript.h:332`); TrinityCore `PlayerScript::OnLogin`.
    PlayerLogin,
    /// The `player.*` notifications of [`PlayerModule`] (#1444): logout,
    /// level, money and quest progress. AzerothCore `PlayerScript`.
    Player,
    /// The `world.*` notifications of [`WorldModule`] (#1444): configuration,
    /// startup, shutdown and the world tick. AzerothCore `WorldScript`.
    World,
}

/// The hook families one module offers (source API 2).
///
/// Each entry is an `Arc`, so one module value can serve several families and
/// keep a single state between them. A family left empty is never called.
#[derive(Clone, Default)]
pub struct ModuleHooks {
    player_login: Option<Arc<dyn PlayerLoginModule>>,
    player: Option<Arc<dyn PlayerModule>>,
    world: Option<Arc<dyn WorldModule>>,
}

impl ModuleHooks {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Offer the `player.login` family.
    #[must_use]
    pub fn player_login(mut self, module: Arc<dyn PlayerLoginModule>) -> Self {
        self.player_login = Some(module);
        self
    }

    /// Offer the `player.*` family.
    #[must_use]
    pub fn player(mut self, module: Arc<dyn PlayerModule>) -> Self {
        self.player = Some(module);
        self
    }

    /// Offer the `world.*` family.
    #[must_use]
    pub fn world(mut self, module: Arc<dyn WorldModule>) -> Self {
        self.world = Some(module);
        self
    }

    /// Whether this set offers `family`.
    #[must_use]
    pub fn offers(&self, family: HookFamily) -> bool {
        match family {
            HookFamily::PlayerLogin => self.player_login.is_some(),
            HookFamily::Player => self.player.is_some(),
            HookFamily::World => self.world.is_some(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.player_login.is_none() && self.player.is_none() && self.world.is_none()
    }
}

/// One registered module and its process-lifetime health.
struct RegisteredModule {
    descriptor: ModuleDescriptor,
    hooks: ModuleHooks,
    /// Set once, when a callback panics. A disabled module is skipped by every
    /// later dispatch of every family for the rest of the process.
    disabled: AtomicBool,
}

/// The registered trusted modules.
///
/// Ordering is deterministic: modules run in `ModuleId` order, never in
/// registration order, so a build that links the same set always dispatches
/// the same sequence. This is distinct from the compositor's registration
/// order (`order`, then id, recorded in `modules.lock.toml`), which only
/// decides the order registrars run in.
///
/// The registry is built once by the compositor, then shared immutably for
/// the life of the process. The only state that changes afterwards is the
/// per-module disabled flag a panicking callback sets.
#[derive(Default)]
pub struct ModuleRegistry {
    modules: BTreeMap<ModuleId, RegisteredModule>,
}

impl ModuleRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a module with the hook families it offers (source API 2).
    pub fn register_module(
        &mut self,
        descriptor: ModuleDescriptor,
        hooks: ModuleHooks,
    ) -> Result<(), ModuleRegistrationError> {
        if self.modules.contains_key(&descriptor.id) {
            return Err(ModuleRegistrationError::DuplicateId {
                id: descriptor.id.to_string(),
            });
        }
        if hooks.is_empty() {
            return Err(ModuleRegistrationError::NoHooks {
                id: descriptor.id.to_string(),
            });
        }
        self.modules.insert(
            descriptor.id.clone(),
            RegisteredModule {
                descriptor,
                hooks,
                disabled: AtomicBool::new(false),
            },
        );
        Ok(())
    }

    /// Register a login-only module. Source API 1's registrar, kept unchanged:
    /// it is [`Self::register_module`] with only the `player.login` family.
    pub fn register_player_login_module(
        &mut self,
        descriptor: ModuleDescriptor,
        module: Box<dyn PlayerLoginModule>,
    ) -> Result<(), ModuleRegistrationError> {
        self.register_module(
            descriptor,
            ModuleHooks::new().player_login(Arc::from(module)),
        )
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Whether any enabled module offers `family`. Owners check this before
    /// building a snapshot, so a family nobody uses costs nothing.
    #[must_use]
    pub fn handles(&self, family: HookFamily) -> bool {
        self.modules
            .values()
            .any(|m| !m.disabled.load(Ordering::Acquire) && m.hooks.offers(family))
    }

    /// Whether a panic disabled `id`. Unknown ids are not disabled.
    #[must_use]
    pub fn is_disabled(&self, id: &ModuleId) -> bool {
        self.modules
            .get(id)
            .is_some_and(|m| m.disabled.load(Ordering::Acquire))
    }

    #[must_use]
    pub fn descriptors(&self) -> Vec<&ModuleDescriptor> {
        self.modules.values().map(|m| &m.descriptor).collect()
    }

    /// Run every registered login module and return the validated batch.
    ///
    /// The batch is validated as a whole before the caller applies anything,
    /// so an invalid effect from one module discards the entire login batch
    /// rather than leaving a partial result. With no modules registered this
    /// allocates nothing observable and returns an empty batch, which is the
    /// zero-module no-op path. Source API 1's form of
    /// [`Self::dispatch_player_login_outcome`].
    pub fn dispatch_player_login(
        &self,
        snapshot: &PlayerLoginSnapshot,
    ) -> Result<PlayerLoginEffects, ModuleEffectError> {
        self.dispatch_player_login_outcome(snapshot).effects
    }

    /// `player.login`, reporting which modules a panic disabled.
    pub fn dispatch_player_login_outcome(
        &self,
        snapshot: &PlayerLoginSnapshot,
    ) -> HookOutcome<NotificationEffect> {
        self.dispatch_notifications(
            |hooks| hooks.player_login.as_ref(),
            |module, effects| module.on_player_login(snapshot, effects),
        )
    }

    /// `player.logout`. Observation only: the outcome never carries effects.
    pub fn dispatch_player_logout(
        &self,
        snapshot: &PlayerLogoutSnapshot,
    ) -> HookOutcome<NotificationEffect> {
        self.dispatch_notifications(
            |hooks| hooks.player.as_ref(),
            |module, _effects| module.on_logout(snapshot),
        )
    }

    /// `player.level_changed`.
    pub fn dispatch_player_level_changed(
        &self,
        snapshot: &PlayerLevelChangedSnapshot,
    ) -> HookOutcome<NotificationEffect> {
        self.dispatch_notifications(
            |hooks| hooks.player.as_ref(),
            |module, effects| module.on_level_changed(snapshot, effects),
        )
    }

    /// `player.money_changed`.
    pub fn dispatch_player_money_changed(
        &self,
        snapshot: &PlayerMoneyChangedSnapshot,
    ) -> HookOutcome<NotificationEffect> {
        self.dispatch_notifications(
            |hooks| hooks.player.as_ref(),
            |module, effects| module.on_money_changed(snapshot, effects),
        )
    }

    /// `player.quest_accepted`.
    pub fn dispatch_quest_accepted(
        &self,
        snapshot: &QuestAcceptedSnapshot,
    ) -> HookOutcome<NotificationEffect> {
        self.dispatch_notifications(
            |hooks| hooks.player.as_ref(),
            |module, effects| module.on_quest_accepted(snapshot, effects),
        )
    }

    /// `player.quest_rewarded`.
    pub fn dispatch_quest_rewarded(
        &self,
        snapshot: &QuestRewardedSnapshot,
    ) -> HookOutcome<NotificationEffect> {
        self.dispatch_notifications(
            |hooks| hooks.player.as_ref(),
            |module, effects| module.on_quest_rewarded(snapshot, effects),
        )
    }

    /// `world.config_loaded`.
    pub fn dispatch_config_loaded(
        &self,
        snapshot: &ConfigLoadedSnapshot,
    ) -> HookOutcome<WorldEffect> {
        self.dispatch_world(|module, effects| module.on_config_loaded(snapshot, effects))
    }

    /// `world.startup`.
    pub fn dispatch_world_startup(
        &self,
        snapshot: &WorldStartupSnapshot,
    ) -> HookOutcome<WorldEffect> {
        self.dispatch_world(|module, effects| module.on_startup(snapshot, effects))
    }

    /// `world.shutdown`.
    pub fn dispatch_world_shutdown(
        &self,
        snapshot: &WorldShutdownSnapshot,
    ) -> HookOutcome<WorldEffect> {
        self.dispatch_world(|module, effects| module.on_shutdown(snapshot, effects))
    }

    /// `world.update`.
    pub fn dispatch_world_update(
        &self,
        snapshot: &WorldUpdateSnapshot,
    ) -> HookOutcome<WorldEffect> {
        self.dispatch_world(|module, effects| module.on_update(snapshot, effects))
    }

    /// `world.update`, also reporting how long each module's callback ran,
    /// in dispatch order. The host uses it for the per-module tick budget and
    /// metric; effect application is not included.
    pub fn dispatch_world_update_timed(
        &self,
        snapshot: &WorldUpdateSnapshot,
    ) -> (HookOutcome<WorldEffect>, Vec<(ModuleId, Duration)>) {
        let mut timings = Vec::new();
        let outcome = self.dispatch_family(
            |hooks| hooks.world.as_ref(),
            |module, batch, id| module.on_update(snapshot, &mut WorldEffects::new(batch, id)),
            Some(&mut timings),
        );
        (outcome, timings)
    }

    /// `player.creature_kill`.
    pub fn dispatch_creature_kill(
        &self,
        snapshot: &CreatureKillSnapshot,
    ) -> HookOutcome<NotificationEffect> {
        self.dispatch_notifications(
            |hooks| hooks.player.as_ref(),
            |module, effects| module.on_creature_kill(snapshot, effects),
        )
    }

    /// `player.spell_cast`.
    pub fn dispatch_spell_cast(
        &self,
        snapshot: &SpellCastSnapshot,
    ) -> HookOutcome<NotificationEffect> {
        self.dispatch_notifications(
            |hooks| hooks.player.as_ref(),
            |module, effects| module.on_spell_cast(snapshot, effects),
        )
    }

    /// Visit, in `ModuleId` order, every enabled module offering one
    /// notification family, then validate the batch as a whole.
    ///
    /// Each callback runs under its own `catch_unwind`. `AssertUnwindSafe` is
    /// sound here because nothing a panicking callback could leave half-done
    /// is observed again: the effects it queued are truncated away, and the
    /// module is disabled so its own state is never read again. The other
    /// modules and the owner only see the batch.
    fn dispatch_notifications<M: ?Sized>(
        &self,
        family: impl Fn(&ModuleHooks) -> Option<&Arc<M>>,
        call: impl Fn(&M, &mut ScopedEffects<'_>),
    ) -> HookOutcome<NotificationEffect> {
        self.dispatch_family(
            family,
            |module, batch, id| {
                call(module, &mut ScopedEffects::new(batch, id));
            },
            None,
        )
    }

    /// [`Self::dispatch_notifications`] for the `world.*` effects.
    fn dispatch_world(
        &self,
        call: impl Fn(&dyn WorldModule, &mut WorldEffects<'_>),
    ) -> HookOutcome<WorldEffect> {
        self.dispatch_family(
            |hooks| hooks.world.as_ref(),
            |module, batch, id| call(module, &mut WorldEffects::new(batch, id)),
            None,
        )
    }

    /// The one dispatch loop every family shares: `ModuleId` order, one
    /// `catch_unwind` per callback, then whole-batch validation. With
    /// `timings`, each callback's duration is recorded under its module.
    fn dispatch_family<M: ?Sized, E: crate::effect::ValidatedEffect>(
        &self,
        family: impl Fn(&ModuleHooks) -> Option<&Arc<M>>,
        call: impl Fn(&M, &mut EffectBatch<E>, ModuleId),
        mut timings: Option<&mut Vec<(ModuleId, Duration)>>,
    ) -> HookOutcome<E> {
        let mut batch = EffectBatch::default();
        let mut disabled = Vec::new();
        for (id, entry) in &self.modules {
            if entry.disabled.load(Ordering::Acquire) {
                continue;
            }
            let Some(module) = family(&entry.hooks) else {
                continue;
            };
            let mark = batch.len();
            let started = timings.is_some().then(Instant::now);
            let completed = catch_unwind(AssertUnwindSafe(|| {
                let _scope = CallbackScope::enter();
                call(module, &mut batch, id.clone());
            }));
            if let (Some(timings), Some(started)) = (timings.as_deref_mut(), started) {
                timings.push((id.clone(), started.elapsed()));
            }
            if completed.is_err() {
                batch.truncate(mark);
                entry.disabled.store(true, Ordering::Release);
                disabled.push(id.clone());
            }
        }
        HookOutcome {
            effects: validated(batch),
            disabled,
        }
    }
}

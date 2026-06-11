//! Database startup sequencer mirroring TrinityCore `DatabaseLoader`.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;

type LoaderFuture<'a> = Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
type LoaderStep<'a> = Box<dyn FnMut() -> LoaderFuture<'a> + Send + 'a>;

/// TrinityCore `DatabaseLoader::DatabaseTypeFlags`.
pub const DATABASE_NONE_LIKE_CPP: u32 = 0;
pub const DATABASE_LOGIN_LIKE_CPP: u32 = 1;
pub const DATABASE_CHARACTER_LIKE_CPP: u32 = 2;
pub const DATABASE_WORLD_LIKE_CPP: u32 = 4;
pub const DATABASE_HOTFIX_LIKE_CPP: u32 = 8;
pub const DATABASE_MASK_ALL_LIKE_CPP: u32 = DATABASE_LOGIN_LIKE_CPP
    | DATABASE_CHARACTER_LIKE_CPP
    | DATABASE_WORLD_LIKE_CPP
    | DATABASE_HOTFIX_LIKE_CPP;

/// Async-friendly equivalent of TC's `DatabaseLoader`.
///
/// The C++ type queues open/populate/update/prepare predicates and pushes a
/// close action when each database opens. If any queued predicate fails, all
/// registered closers are executed in reverse registration order. RustyCore's
/// actual DB operations are async, so each step is an async closure returning
/// `true` on success and `false` on failure.
pub struct DatabaseLoaderLikeCpp<'a> {
    auto_setup: bool,
    update_flags: u32,
    open: VecDeque<LoaderStep<'a>>,
    populate: VecDeque<LoaderStep<'a>>,
    update: VecDeque<LoaderStep<'a>>,
    prepare: VecDeque<LoaderStep<'a>>,
    close: Vec<LoaderStep<'a>>,
}

impl<'a> DatabaseLoaderLikeCpp<'a> {
    /// Create a loader with TC's `Updates.AutoSetup` and enabled DB mask.
    pub fn new(auto_setup: bool, update_flags: u32) -> Self {
        Self {
            auto_setup,
            update_flags,
            open: VecDeque::new(),
            populate: VecDeque::new(),
            update: VecDeque::new(),
            prepare: VecDeque::new(),
            close: Vec::new(),
        }
    }

    pub fn auto_setup_like_cpp(&self) -> bool {
        self.auto_setup
    }

    pub fn update_flags_like_cpp(&self) -> u32 {
        self.update_flags
    }

    pub fn updates_enabled_for_like_cpp(&self, database_flag: u32) -> bool {
        self.update_flags & database_flag != 0
    }

    pub fn add_open_step<F, Fut>(&mut self, step: F) -> &mut Self
    where
        F: FnMut() -> Fut + Send + 'a,
        Fut: Future<Output = bool> + Send + 'a,
    {
        self.open.push_back(box_step(step));
        self
    }

    pub fn add_populate_step<F, Fut>(&mut self, step: F) -> &mut Self
    where
        F: FnMut() -> Fut + Send + 'a,
        Fut: Future<Output = bool> + Send + 'a,
    {
        self.populate.push_back(box_step(step));
        self
    }

    pub fn add_update_step<F, Fut>(&mut self, step: F) -> &mut Self
    where
        F: FnMut() -> Fut + Send + 'a,
        Fut: Future<Output = bool> + Send + 'a,
    {
        self.update.push_back(box_step(step));
        self
    }

    pub fn add_prepare_step<F, Fut>(&mut self, step: F) -> &mut Self
    where
        F: FnMut() -> Fut + Send + 'a,
        Fut: Future<Output = bool> + Send + 'a,
    {
        self.prepare.push_back(box_step(step));
        self
    }

    /// Push a close action onto the rollback stack.
    pub fn add_close_step<F, Fut>(&mut self, step: F) -> &mut Self
    where
        F: FnMut() -> Fut + Send + 'a,
        Fut: Future<Output = bool> + Send + 'a,
    {
        self.close.push(box_step(step));
        self
    }

    /// Run open, populate, update and prepare queues in TC order.
    pub async fn load_like_cpp(&mut self) -> bool {
        if !process_like_cpp(&mut self.open, &mut self.close).await {
            return false;
        }
        if !process_like_cpp(&mut self.populate, &mut self.close).await {
            return false;
        }
        if !process_like_cpp(&mut self.update, &mut self.close).await {
            return false;
        }
        process_like_cpp(&mut self.prepare, &mut self.close).await
    }
}

fn box_step<'a, F, Fut>(mut step: F) -> LoaderStep<'a>
where
    F: FnMut() -> Fut + Send + 'a,
    Fut: Future<Output = bool> + Send + 'a,
{
    Box::new(move || Box::pin(step()))
}

async fn process_like_cpp<'a>(
    queue: &mut VecDeque<LoaderStep<'a>>,
    close: &mut Vec<LoaderStep<'a>>,
) -> bool {
    while let Some(mut step) = queue.pop_front() {
        if !step().await {
            while let Some(mut closer) = close.pop() {
                closer().await;
            }
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{
        DATABASE_CHARACTER_LIKE_CPP, DATABASE_LOGIN_LIKE_CPP, DATABASE_MASK_ALL_LIKE_CPP,
        DATABASE_WORLD_LIKE_CPP, DatabaseLoaderLikeCpp,
    };
    use std::sync::{Arc, Mutex};

    fn record(events: &Arc<Mutex<Vec<&'static str>>>, value: &'static str) {
        events.lock().unwrap().push(value);
    }

    #[tokio::test]
    async fn loader_runs_phases_in_cpp_order() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut loader = DatabaseLoaderLikeCpp::new(true, DATABASE_MASK_ALL_LIKE_CPP);

        {
            let events = Arc::clone(&events);
            loader.add_open_step(move || {
                let events = Arc::clone(&events);
                async move {
                    record(&events, "open");
                    true
                }
            });
        }
        {
            let events = Arc::clone(&events);
            loader.add_populate_step(move || {
                let events = Arc::clone(&events);
                async move {
                    record(&events, "populate");
                    true
                }
            });
        }
        {
            let events = Arc::clone(&events);
            loader.add_update_step(move || {
                let events = Arc::clone(&events);
                async move {
                    record(&events, "update");
                    true
                }
            });
        }
        {
            let events = Arc::clone(&events);
            loader.add_prepare_step(move || {
                let events = Arc::clone(&events);
                async move {
                    record(&events, "prepare");
                    true
                }
            });
        }

        assert!(loader.load_like_cpp().await);
        assert_eq!(
            events.lock().unwrap().as_slice(),
            ["open", "populate", "update", "prepare"]
        );
    }

    #[tokio::test]
    async fn loader_rolls_back_closers_lifo_on_failure_like_cpp() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut loader = DatabaseLoaderLikeCpp::new(true, DATABASE_MASK_ALL_LIKE_CPP);

        {
            let events = Arc::clone(&events);
            loader.add_open_step(move || {
                let events = Arc::clone(&events);
                async move {
                    record(&events, "open-login");
                    true
                }
            });
        }
        {
            let events = Arc::clone(&events);
            loader.add_close_step(move || {
                let events = Arc::clone(&events);
                async move {
                    record(&events, "close-login");
                    true
                }
            });
        }
        {
            let events = Arc::clone(&events);
            loader.add_open_step(move || {
                let events = Arc::clone(&events);
                async move {
                    record(&events, "open-world");
                    true
                }
            });
        }
        {
            let events = Arc::clone(&events);
            loader.add_close_step(move || {
                let events = Arc::clone(&events);
                async move {
                    record(&events, "close-world");
                    true
                }
            });
        }
        {
            let events = Arc::clone(&events);
            loader.add_prepare_step(move || {
                let events = Arc::clone(&events);
                async move {
                    record(&events, "prepare-fail");
                    false
                }
            });
        }

        assert!(!loader.load_like_cpp().await);
        assert_eq!(
            events.lock().unwrap().as_slice(),
            [
                "open-login",
                "open-world",
                "prepare-fail",
                "close-world",
                "close-login"
            ]
        );
    }

    #[test]
    fn loader_update_mask_matches_cpp_flags() {
        let loader =
            DatabaseLoaderLikeCpp::new(true, DATABASE_LOGIN_LIKE_CPP | DATABASE_WORLD_LIKE_CPP);

        assert!(loader.auto_setup_like_cpp());
        assert_eq!(
            loader.update_flags_like_cpp(),
            DATABASE_LOGIN_LIKE_CPP | DATABASE_WORLD_LIKE_CPP
        );
        assert!(loader.updates_enabled_for_like_cpp(DATABASE_LOGIN_LIKE_CPP));
        assert!(!loader.updates_enabled_for_like_cpp(DATABASE_CHARACTER_LIKE_CPP));
        assert!(loader.updates_enabled_for_like_cpp(DATABASE_WORLD_LIKE_CPP));
    }
}

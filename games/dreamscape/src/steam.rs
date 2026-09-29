//! The Steam bridge. With the `steam` feature it starts Steamworks, mirrors
//! achievement unlocks (including any earned offline) and pumps callbacks;
//! without it every call is a no-op, so the rest of the game never cares.
//! The App ID is Spacewar (480) until Dreamscape has its own.

#[cfg(feature = "steam")]
mod imp {
    use std::cell::RefCell;

    /// Placeholder until the store page exists.
    pub const APP_ID: u32 = 480;

    thread_local! {
        static CLIENT: RefCell<Option<steamworks::Client>> = const { RefCell::new(None) };
    }

    pub fn init(unlocked: &[String]) {
        match steamworks::Client::init_app(APP_ID) {
            Ok(client) => {
                log::info!("Steam: signed in (app {APP_ID})");
                CLIENT.with(|c| *c.borrow_mut() = Some(client));
                // Unlocks earned offline or before Steam was connected.
                for id in unlocked {
                    unlock(id);
                }
            }
            Err(e) => log::warn!("Steam: not available ({e:?}); achievements stay local"),
        }
    }

    pub fn unlock(id: &str) {
        CLIENT.with(|c| {
            if let Some(client) = c.borrow().as_ref() {
                let stats = client.user_stats();
                if stats.achievement(id).set().is_err() {
                    log::warn!("Steam: couldn't set achievement {id}");
                }
                let _ = stats.store_stats();
            }
        });
    }

    pub fn tick() {
        CLIENT.with(|c| {
            if let Some(client) = c.borrow().as_ref() {
                client.run_callbacks();
            }
        });
    }
}

#[cfg(not(feature = "steam"))]
mod imp {
    pub fn init(_unlocked: &[String]) {}
    pub fn unlock(_id: &str) {}
    pub fn tick() {}
}

pub use imp::{init, tick, unlock};

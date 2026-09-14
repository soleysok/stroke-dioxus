//! What the browser's session history will tell us, which is less than it looks.
//!
//! [`Navigator::can_go_back`] is always true on the web. The browser will not say
//! whether the entry behind the current one is a page of ours or whatever the tab
//! was showing before somebody followed a link into the app, so a Back control
//! that trusted it would sometimes leave the app altogether.
//!
//! The one thing the browser does expose is how many entries the session has.
//! Recording that as the document loads — before anything can navigate — is
//! enough to tell a back that stays inside the app from one that walks out of it.

use dioxus::prelude::*;

/// How long the session history was when this document loaded.
#[derive(Clone, Copy, PartialEq)]
pub struct AtEntry(u32);

impl AtEntry {
    /// Whether the app has since pushed an entry of its own, so there is a page
    /// of ours behind this one.
    pub fn app_has_pushed(self) -> bool {
        length() > self.0
    }
}

fn length() -> u32 {
    web_sys::window()
        .and_then(|window| window.history().ok())
        .and_then(|history| history.length().ok())
        .unwrap_or_default()
}

/// Called once by `App`, so every route can ask how it was arrived at.
pub fn provide_entry() {
    use_context_provider(|| AtEntry(length()));
}

/// The reading [`provide_entry`] took, for a route that wants to know how it was
/// arrived at.
pub fn use_entry() -> AtEntry {
    use_context()
}

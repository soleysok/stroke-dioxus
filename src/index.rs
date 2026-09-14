//! The search index, loaded once and shared through context.
//!
//! Every page needs it, it is ~518 KB, and it never changes, so it is fetched at
//! the app root and handed down rather than being re-fetched per route.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::data::Index;

/// `None` while the fetch is in flight, then the result. Pages render a loading
/// state for `None` and an error for `Err`, so a failed fetch is visible rather
/// than an empty page.
pub type IndexState = Option<Result<Rc<Index>, String>>;

/// Kick off the load and publish it to descendants. Call once, at the root.
pub fn provide_index() -> Signal<IndexState> {
    let state = use_context_provider(|| Signal::new(None as IndexState));

    use_future(move || async move {
        let mut state = state;
        let loaded = Index::load().await.map(Rc::new);
        state.set(Some(loaded));
    });

    state
}

pub fn use_index() -> Signal<IndexState> {
    use_context()
}

/// Read the loaded index, if it is ready. Returns `None` while loading or on
/// error, which is what most call sites want.
pub fn ready(state: &Signal<IndexState>) -> Option<Rc<Index>> {
    state.read().as_ref().and_then(|r| r.as_ref().ok()).cloned()
}

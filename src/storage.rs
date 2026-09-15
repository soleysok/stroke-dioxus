//! What the browser remembers for us: recently viewed and saved characters in
//! `localStorage`, which character the handwriting pad opened in `sessionStorage`,
//! and the raw `localStorage` accessors [`crate::lists`] keeps its named lists
//! through.
//!
//! There is no account system, so everything here is per-device. Characters are
//! stored as a plain string rather than JSON, which keeps them small and readable
//! in devtools; the named lists need structure and are JSON.

const RECENT_KEY: &str = "stroke:recent";
const SAVED_KEY: &str = "stroke:saved";

/// Enough history to be useful without turning the Lists tab into a wall.
const MAX_RECENT: usize = 36;

fn storage() -> Option<web_sys::Storage> {
    // Both calls fail rather than panic when storage is blocked, which is what
    // Safari does in private browsing.
    web_sys::window()?.local_storage().ok().flatten()
}

/// Read one key. `None` when it is unset or storage is unavailable, which are the
/// same thing as far as any caller is concerned.
pub(crate) fn read_text(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

/// Write one key, silently doing nothing when storage is blocked or full. A
/// failed write costs the visitor their saved words, not the page they are on.
pub(crate) fn write_text(key: &str, value: &str) {
    if let Some(store) = storage() {
        let _ = store.set_item(key, value);
    }
}

fn read(key: &str) -> Vec<char> {
    read_text(key)
        .map(|raw| raw.chars().collect())
        .unwrap_or_default()
}

fn write(key: &str, chars: &[char]) {
    let value: String = chars.iter().collect();
    write_text(key, &value);
}

pub fn recent() -> Vec<char> {
    read(RECENT_KEY)
}

pub fn saved() -> Vec<char> {
    read(SAVED_KEY)
}

/// Record a visit, moving the character to the front if it was already there.
pub fn push_recent(glyph: char) {
    let mut list = recent();
    list.retain(|&c| c != glyph);
    list.insert(0, glyph);
    list.truncate(MAX_RECENT);
    write(RECENT_KEY, &list);
}

// ── Where a character was opened from ───────────────────────────────────────

/// The character the handwriting pad last opened, kept for the tab it happened
/// in.
///
/// The character page needs this to know that the pad is where somebody came
/// from, so its Back control can lead there rather than guessing. Nothing else
/// can say: `/character/你` is deliberately the same URL however it was reached,
/// and a referrer is the page the tab was opened with, since navigating inside
/// the app never leaves the document. So the pad leaves a note instead — one
/// that survives a reload of the character page, and goes with the tab.
const FROM_DRAW_KEY: &str = "stroke:from-draw";

fn session() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok().flatten()
}

/// Note that `glyph` is being opened from the pad.
pub fn mark_from_draw(glyph: char) {
    if let Some(store) = session() {
        let _ = store.set_item(FROM_DRAW_KEY, &glyph.to_string());
    }
}

/// Whether `glyph` is the character the pad opened. A note about any other
/// character was left by an earlier visit, and is dropped rather than left to
/// claim that a later arrival came from the pad.
pub fn came_from_draw(glyph: char) -> bool {
    let Some(store) = session() else {
        return false;
    };
    let Some(marked) = store.get_item(FROM_DRAW_KEY).ok().flatten() else {
        return false;
    };
    if marked.starts_with(glyph) {
        return true;
    }
    let _ = store.remove_item(FROM_DRAW_KEY);
    false
}

/// Drop the note. The pad calls this as it mounts: whatever it opened has been
/// come back from, and the next character has not been written yet.
pub fn forget_from_draw() {
    if let Some(store) = session() {
        let _ = store.remove_item(FROM_DRAW_KEY);
    }
}

pub fn is_saved(glyph: char) -> bool {
    saved().contains(&glyph)
}

/// Add or remove a character from the saved list. Returns its new state.
pub fn toggle_saved(glyph: char) -> bool {
    let mut list = saved();
    let now_saved = if let Some(at) = list.iter().position(|&c| c == glyph) {
        list.remove(at);
        false
    } else {
        list.insert(0, glyph);
        true
    };
    write(SAVED_KEY, &list);
    now_saved
}

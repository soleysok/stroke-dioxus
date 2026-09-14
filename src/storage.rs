//! Recently viewed and saved characters, persisted in `localStorage`, plus the
//! raw accessors [`crate::lists`] stores its named lists through.
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

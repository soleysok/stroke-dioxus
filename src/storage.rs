//! Recently viewed and saved characters, persisted in `localStorage`.
//!
//! There is no account system yet, so this is the whole of "My Lists" for now.
//! Values are stored as a plain string of characters rather than JSON, which
//! keeps them small and readable in devtools.

const RECENT_KEY: &str = "stroke:recent";
const SAVED_KEY: &str = "stroke:saved";

/// Enough history to be useful without turning the Lists tab into a wall.
const MAX_RECENT: usize = 36;

fn storage() -> Option<web_sys::Storage> {
    // Both calls fail rather than panic when storage is blocked, which is what
    // Safari does in private browsing.
    web_sys::window()?.local_storage().ok().flatten()
}

fn read(key: &str) -> Vec<char> {
    storage()
        .and_then(|s| s.get_item(key).ok().flatten())
        .map(|raw| raw.chars().collect())
        .unwrap_or_default()
}

fn write(key: &str, chars: &[char]) {
    if let Some(store) = storage() {
        let value: String = chars.iter().collect();
        let _ = store.set_item(key, &value);
    }
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

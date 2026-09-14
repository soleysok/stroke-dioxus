//! My Lists: named vocabulary lists, persisted in `localStorage`.
//!
//! A list entry carries the Chinese plus the two readings a learner drills
//! against — pinyin and a short English gloss — copied from the course at the
//! moment the word is added. That is what lets `/lists` render without fetching a
//! 100 KB level file or the 518 KB character index first.
//!
//! The whole document lives in one signal, provided at the app root. Pages read
//! it directly, so importing a unit on `/hsk/1/01` shows up on `/lists` without a
//! reload; every mutation writes through to `localStorage` immediately, because
//! there is no other copy of it anywhere.
//!
//! The stored shape matches the React app's `stroke:lists:v1` document field for
//! field, so the two could one day exchange lists as plain JSON. There is no
//! account and nothing is synced.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::hsk;
use crate::storage;

const KEY: &str = "stroke:lists:v1";

/// Guard rails, so one runaway paste cannot fill the storage quota.
pub const MAX_LISTS: usize = 120;
pub const MAX_WORDS: usize = 600;
pub const MAX_TITLE: usize = 80;
const MAX_GLOSS: usize = 140;
const MAX_PINYIN: usize = 60;
/// Headwords are words, not sentences. Anything longer is a mistake.
const MAX_HEADWORD: usize = 12;

// ── The document ────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWord {
    /// Simplified headword. Identifies the entry within its list.
    pub word: String,
    #[serde(default)]
    pub pinyin: String,
    #[serde(default)]
    pub gloss: String,
    /// HSK band, when the word came from the course.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(default)]
    pub added_at: f64,
}

impl ListWord {
    /// A course word, copied into a list exactly as the unit teaches it.
    pub fn from_course(word: &hsk::Word) -> Self {
        Self {
            word: word.word.clone(),
            pinyin: word.pinyin.clone(),
            gloss: word.gloss.clone(),
            level: Some(word.level),
            added_at: now(),
        }
    }
}

/// Where a list came from, so a card can say more than its title.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum ListSource {
    HskUnit {
        level: u8,
        unit_id: String,
        unit_title: String,
    },
    /// Built by hand. Also where a source this version does not understand lands,
    /// so one unknown value cannot discard every list. Has to be declared last:
    /// serde's catch-all variant is the final one.
    #[default]
    #[serde(other)]
    Custom,
}

impl ListSource {
    pub fn from_unit(level: u8, unit: &hsk::Unit) -> Self {
        Self::HskUnit {
            level,
            unit_id: unit.id.clone(),
            unit_title: unit.title.clone(),
        }
    }

    /// Human label for a list's origin, shown under its title.
    pub fn label(&self) -> String {
        match self {
            Self::Custom => "Your own list".to_string(),
            Self::HskUnit {
                level, unit_id, ..
            } => format!("HSK {level} · Unit {}", position_of(unit_id)),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordList {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub source: ListSource,
    #[serde(default)]
    pub words: Vec<ListWord>,
    #[serde(default)]
    pub created_at: f64,
    #[serde(default)]
    pub updated_at: f64,
}

impl WordList {
    /// True when this list was imported from the given course unit. Accepts the
    /// unit's position as well as its full id, since a route carries the former.
    pub fn is_from_unit(&self, level: u8, unit: &str) -> bool {
        match &self.source {
            ListSource::HskUnit {
                level: from,
                unit_id,
                ..
            } => *from == level && (unit_id == unit || position_of(unit_id) == unit),
            ListSource::Custom => false,
        }
    }

    /// The characters the list's words are written with, for stroke practice.
    pub fn characters(&self) -> Vec<char> {
        hsk::characters(self.words.iter().map(|w| w.word.as_str()))
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListsDoc {
    pub version: u8,
    #[serde(default)]
    pub lists: Vec<WordList>,
    #[serde(default)]
    pub updated_at: f64,
}

impl Default for ListsDoc {
    fn default() -> Self {
        Self {
            version: 1,
            lists: Vec::new(),
            updated_at: 0.0,
        }
    }
}

impl ListsDoc {
    pub fn get(&self, id: &str) -> Option<&WordList> {
        self.lists.iter().find(|list| list.id == id)
    }

    /// The list already imported from a unit, if there is one. This is what makes
    /// "add this lesson" idempotent.
    pub fn list_for_unit(&self, level: u8, unit: &str) -> Option<&WordList> {
        self.lists.iter().find(|list| list.is_from_unit(level, unit))
    }

    /// Which of a level's units have already been imported, for the "Added"
    /// markers on a level page.
    pub fn imported_units(&self, level: u8) -> Vec<String> {
        self.lists
            .iter()
            .filter_map(|list| match &list.source {
                ListSource::HskUnit {
                    level: at,
                    unit_id,
                    ..
                } if *at == level => Some(unit_id.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn total_words(&self) -> usize {
        self.lists.iter().map(|list| list.words.len()).sum()
    }

    pub fn is_full(&self) -> bool {
        self.lists.len() >= MAX_LISTS
    }
}

// ── Reading and writing ─────────────────────────────────────────────────────

/// Load the document and publish it to descendants. Call once, at the root.
pub fn provide() -> Signal<ListsDoc> {
    use_context_provider(|| Signal::new(load()))
}

pub fn use_lists() -> Signal<ListsDoc> {
    use_context()
}

fn load() -> ListsDoc {
    storage::read_text(KEY)
        .as_deref()
        .map(parse)
        .unwrap_or_default()
}

/// Parse a stored document, keeping whatever is intelligible.
///
/// Anything here may have been hand-edited in devtools or written by a newer
/// version of the app, and losing every list to one bad field would be the worst
/// possible outcome — so a document that does not parse at all becomes an empty
/// one, and a list that does parse is clamped into shape.
fn parse(raw: &str) -> ListsDoc {
    let mut doc: ListsDoc = serde_json::from_str(raw).unwrap_or_default();
    doc.version = 1;
    doc.lists.truncate(MAX_LISTS);
    doc.lists.retain_mut(|list| {
        if list.id.is_empty() {
            return false;
        }
        list.title = clean_title(&list.title);
        list.words.truncate(MAX_WORDS);
        list.words.retain_mut(|word| {
            word.word = clean_headword(&word.word);
            word.pinyin = clamp(&word.pinyin, MAX_PINYIN);
            word.gloss = clamp(&word.gloss, MAX_GLOSS);
            !word.word.is_empty()
        });
        let mut seen: Vec<String> = Vec::new();
        list.words.retain(|word| {
            let fresh = !seen.contains(&word.word);
            if fresh {
                seen.push(word.word.clone());
            }
            fresh
        });
        true
    });
    doc
}

fn save(doc: &ListsDoc) {
    if let Ok(json) = serde_json::to_string(doc) {
        storage::write_text(KEY, &json);
    }
}

/// Apply an edit, stamp the document, and persist it in one go. Every mutation
/// goes through here so nothing can change the lists without saving them.
fn update<T>(mut lists: Signal<ListsDoc>, edit: impl FnOnce(&mut ListsDoc) -> T) -> T {
    let (result, snapshot) = {
        let mut doc = lists.write();
        let result = edit(&mut doc);
        doc.updated_at = now();
        (result, doc.clone())
    };
    save(&snapshot);
    result
}

// ── Mutations ───────────────────────────────────────────────────────────────

/// Create a list and return its id, or `None` when there is no room for another.
pub fn create(
    lists: Signal<ListsDoc>,
    title: &str,
    source: ListSource,
    words: Vec<ListWord>,
) -> Option<String> {
    if lists.read().is_full() {
        return None;
    }
    let now = now();
    let mut list = WordList {
        id: new_id(),
        title: clean_title(title),
        source,
        words: Vec::new(),
        created_at: now,
        updated_at: now,
    };
    merge(&mut list.words, words);
    let id = list.id.clone();
    update(lists, move |doc| doc.lists.insert(0, list));
    Some(id)
}

pub fn rename(lists: Signal<ListsDoc>, id: &str, title: &str) {
    let title = clean_title(title);
    update(lists, |doc| {
        if let Some(list) = doc.lists.iter_mut().find(|list| list.id == id) {
            if list.title != title {
                list.title = title;
                list.updated_at = now();
            }
        }
    });
}

pub fn delete(lists: Signal<ListsDoc>, id: &str) {
    update(lists, |doc| doc.lists.retain(|list| list.id != id));
}

pub fn remove_word(lists: Signal<ListsDoc>, id: &str, word: &str) {
    update(lists, |doc| {
        if let Some(list) = doc.lists.iter_mut().find(|list| list.id == id) {
            let before = list.words.len();
            list.words.retain(|entry| entry.word != word);
            if list.words.len() != before {
                list.updated_at = now();
            }
        }
    });
}

/// Append the words a list does not already have. The dedupe is what makes
/// re-importing a unit a no-op rather than a second copy of it.
fn merge(existing: &mut Vec<ListWord>, incoming: Vec<ListWord>) -> usize {
    let mut added = 0;
    for word in incoming {
        if word.word.is_empty() || existing.iter().any(|have| have.word == word.word) {
            continue;
        }
        if existing.len() >= MAX_WORDS {
            break;
        }
        existing.push(word);
        added += 1;
    }
    added
}

// ── Course imports ──────────────────────────────────────────────────────────

/// The title an imported unit gets: `"HSK 1 · Greetings & courtesy"`.
pub fn unit_list_title(level: u8, unit: &hsk::Unit) -> String {
    clean_title(&format!("HSK {level} · {}", unit.title))
}

/// Import a unit, or return the list it was already imported into.
///
/// One tap on a unit page ends up here, and tapping it again has to be safe: the
/// second tap opens the list rather than making another one. A list the learner
/// has since pruned is left alone — the words that are missing were removed on
/// purpose.
pub fn import_unit(
    lists: Signal<ListsDoc>,
    level: u8,
    unit: &hsk::Unit,
    words: &[hsk::Word],
) -> Option<String> {
    if let Some(existing) = lists.read().list_for_unit(level, &unit.id) {
        return Some(existing.id.clone());
    }
    create(
        lists,
        &unit_list_title(level, unit),
        ListSource::from_unit(level, unit),
        words.iter().map(ListWord::from_course).collect(),
    )
}

// ── Cleaning ────────────────────────────────────────────────────────────────

fn clamp(value: &str, max: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(max).collect()
}

pub fn clean_title(value: &str) -> String {
    let title = clamp(value, MAX_TITLE);
    if title.is_empty() {
        "Untitled list".to_string()
    } else {
        title
    }
}

/// The Han characters in a headword, dropping spaces, latin and punctuation.
fn clean_headword(value: &str) -> String {
    value
        .chars()
        .filter(|c| is_han(*c))
        .take(MAX_HEADWORD)
        .collect()
}

fn is_han(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{9fff}' | '\u{3005}' | '\u{f900}'..='\u{faff}')
}

/// `"3-07"` -> `"07"`, for a source label.
fn position_of(unit_id: &str) -> &str {
    unit_id.split_once('-').map_or(unit_id, |(_, n)| n)
}

// ── Ids and time ────────────────────────────────────────────────────────────

fn now() -> f64 {
    js_sys::Date::now()
}

/// Unique enough: the millisecond it was made plus 16 random bits. Two lists made
/// in the same millisecond are a double tap, which the callers already prevent.
fn new_id() -> String {
    let millis = now() as u64;
    let salt = (js_sys::Math::random() * 65_536.0) as u32;
    format!("l{millis:x}{salt:04x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(headword: &str) -> ListWord {
        ListWord {
            word: headword.to_string(),
            pinyin: String::new(),
            gloss: String::new(),
            level: Some(1),
            added_at: 1.0,
        }
    }

    fn unit(id: &str) -> hsk::Unit {
        hsk::Unit {
            id: id.to_string(),
            title: "Greetings & courtesy".to_string(),
            topic: "greetings".to_string(),
            words: vec!["你好".to_string()],
        }
    }

    #[test]
    fn a_unit_source_survives_a_round_trip_through_json() {
        let source = ListSource::from_unit(1, &unit("1-01"));
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"hsk-unit","level":1,"unitId":"1-01","unitTitle":"Greetings & courtesy"}"#
        );
        assert_eq!(serde_json::from_str::<ListSource>(&json).unwrap(), source);

        let custom = serde_json::to_string(&ListSource::Custom).unwrap();
        assert_eq!(custom, r#"{"kind":"custom"}"#);
        assert_eq!(
            serde_json::from_str::<ListSource>(&custom).unwrap(),
            ListSource::Custom
        );
    }

    #[test]
    fn an_unknown_source_reads_as_a_custom_list_rather_than_losing_it() {
        let doc = parse(r#"{"version":1,"lists":[{"id":"l1","title":"Kept","source":{"kind":"from-the-future"},"words":[{"word":"好"}]}]}"#);
        assert_eq!(doc.lists.len(), 1);
        assert_eq!(doc.lists[0].source, ListSource::Custom);
        assert_eq!(doc.lists[0].words[0].word, "好");
    }

    #[test]
    fn nonsense_in_storage_parses_as_an_empty_document() {
        assert!(parse("not json at all").lists.is_empty());
        assert!(parse("{}").lists.is_empty());
    }

    #[test]
    fn parsing_clamps_a_hand_edited_document_into_shape() {
        let doc = parse(
            r#"{"version":9,"lists":[
                {"id":"l1","title":"   ","words":[
                    {"word":"你好 (hello)"},{"word":"你好"},{"word":"!"}
                ]},
                {"id":"","title":"No id","words":[]}
            ]}"#,
        );
        assert_eq!(doc.version, 1);
        assert_eq!(doc.lists.len(), 1, "the list without an id is dropped");
        let list = &doc.lists[0];
        assert_eq!(list.title, "Untitled list");
        assert_eq!(
            list.words.iter().map(|w| w.word.as_str()).collect::<Vec<_>>(),
            ["你好"],
            "latin is stripped, the repeat and the empty word are dropped"
        );
    }

    #[test]
    fn merging_skips_words_a_list_already_has() {
        let mut words = vec![word("你好")];
        assert_eq!(merge(&mut words, vec![word("你好"), word("谢谢")]), 1);
        assert_eq!(words.len(), 2);
        assert_eq!(merge(&mut words, vec![word("谢谢")]), 0);
    }

    #[test]
    fn merging_stops_at_the_word_ceiling() {
        let mut words: Vec<ListWord> = (0..MAX_WORDS - 1)
            .map(|i| word(&format!("已{i}")))
            .collect();
        let incoming: Vec<ListWord> = (0..5).map(|i| word(&format!("新{i}"))).collect();
        assert_eq!(merge(&mut words, incoming), 1);
        assert_eq!(words.len(), MAX_WORDS);
    }

    #[test]
    fn a_list_knows_which_unit_it_came_from() {
        let list = WordList {
            id: "l1".to_string(),
            title: "HSK 1 · Greetings & courtesy".to_string(),
            source: ListSource::from_unit(1, &unit("1-01")),
            words: vec![word("你好")],
            created_at: 0.0,
            updated_at: 0.0,
        };
        assert!(list.is_from_unit(1, "1-01"), "by full id");
        assert!(list.is_from_unit(1, "01"), "by position, as a route carries it");
        assert!(!list.is_from_unit(2, "01"), "a different level");
        assert!(!list.is_from_unit(1, "02"), "a different unit");
        assert_eq!(list.source.label(), "HSK 1 · Unit 01");
        assert_eq!(list.characters(), vec!['你', '好']);
    }

    #[test]
    fn a_title_is_collapsed_and_capped() {
        assert_eq!(clean_title("  Food   and drink "), "Food and drink");
        assert_eq!(clean_title(""), "Untitled list");
        assert_eq!(clean_title(&"x".repeat(200)).chars().count(), MAX_TITLE);
    }
}

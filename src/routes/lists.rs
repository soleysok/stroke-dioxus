//! `/lists` and `/lists/:id` — My Lists.
//!
//! Two kinds of thing live here, because they answer two different questions.
//! Named word lists are what somebody is deliberately working on, built by hand
//! or taken whole from an HSK unit. Saved and recently viewed *characters* are the
//! trail left by the dictionary. Both are in `localStorage`, so there is no
//! account and nothing to sign in to.

use dioxus::prelude::*;

use crate::components::char_list::CharGrid;
use crate::components::icons;
use crate::components::shell::{Empty, RoadmapItem, Section};
use crate::components::word_list::{WordRow, WordRows};
use crate::index::{self, use_index};
use crate::lists::{self, ListSource};
use crate::{hsk, storage, Route};

/// Headwords shown on a picker row before it would start wrapping on a phone.
const PREVIEW_WORDS: usize = 5;

/// Which of the two "add a list" panels is open, if either. Both start closed:
/// the lists themselves are the point of the page.
#[derive(Clone, Copy, PartialEq)]
enum Panel {
    None,
    New,
    Unit,
}

#[component]
pub fn Lists() -> Element {
    let state = use_index();
    let lists = lists::use_lists();
    let navigator = use_navigator();

    let mut panel = use_signal(|| Panel::None);
    let mut title = use_signal(String::new);
    // Deleting is two taps rather than a browser dialog: the first arms the row.
    let mut arming = use_signal(|| None as Option<String>);

    // Re-read on mount rather than caching: the character lists change elsewhere.
    let saved = use_memo(move || {
        index::ready(&state)
            .map(|idx| idx.lookup_all(&storage::saved()))
            .unwrap_or_default()
    });
    let recent = use_memo(move || {
        index::ready(&state)
            .map(|idx| idx.lookup_all(&storage::recent()))
            .unwrap_or_default()
    });

    let doc = lists.read().clone();
    let full = doc.is_full();

    let mut create = move |_| {
        let Some(id) = lists::create(lists, &title(), ListSource::Custom, Vec::new()) else {
            return;
        };
        title.set(String::new());
        panel.set(Panel::None);
        navigator.push(Route::ListDetail { id });
    };

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            header { class: "section",
                h1 { "My Lists" }
                p { class: "lede",
                    "The words you are working on, and the characters you have looked at. "
                    "Kept on this device — there is no account to create."
                }
                if !doc.lists.is_empty() {
                    p { class: "action-note num",
                        "{doc.lists.len()} lists · {doc.total_words()} words"
                    }
                }

                div { class: "actions",
                    button {
                        class: if panel() == Panel::New { "btn btn-accent" } else { "btn" },
                        r#type: "button",
                        disabled: full,
                        aria_expanded: (panel() == Panel::New).to_string(),
                        onclick: move |_| {
                            arming.set(None);
                            panel.set(if panel() == Panel::New { Panel::None } else { Panel::New });
                        },
                        icons::Plus {}
                        "New List"
                    }
                    button {
                        class: if panel() == Panel::Unit { "btn btn-accent" } else { "btn" },
                        r#type: "button",
                        disabled: full,
                        aria_expanded: (panel() == Panel::Unit).to_string(),
                        onclick: move |_| {
                            arming.set(None);
                            panel.set(if panel() == Panel::Unit { Panel::None } else { Panel::Unit });
                        },
                        icons::TabLevels {}
                        "From An HSK Unit"
                    }
                }
                if full {
                    p { class: "action-note",
                        "That is {lists::MAX_LISTS} lists, which is the ceiling. Delete one to make room."
                    }
                }
            }

            if panel() == Panel::New {
                Section { title: "Name Your List".to_string(),
                    div { class: "card card-pad stack",
                        form {
                            class: "field",
                            onsubmit: move |event| {
                                // Dioxus 0.7 allows native submission, which would
                                // reload the page.
                                event.prevent_default();
                                create(());
                            },
                            input {
                                class: "field-input",
                                r#type: "text",
                                value: "{title}",
                                placeholder: "Words for Friday",
                                aria_label: "List name",
                                autofocus: true,
                                autocomplete: "off",
                                enterkeyhint: "done",
                                oninput: move |event| title.set(event.value()),
                            }
                            button { class: "btn btn-accent", r#type: "submit", "Create" }
                        }
                        p { class: "action-note",
                            "Words go in next — a whole HSK unit, or one at a time from the dictionary."
                        }
                    }
                }
            }

            if panel() == Panel::Unit {
                Section { title: "Take A Unit From The Course".to_string(),
                    UnitPicker {}
                }
            }

            if doc.lists.is_empty() {
                Empty { glyph: "空".to_string(), title: "No Lists Yet".to_string(),
                    p { "Open an HSK unit and add the lesson, or start an empty list and fill it as you go." }
                    Link { class: "btn btn-accent", to: Route::Hsk {}, "Browse The Course" }
                }
            } else {
                Section { title: "Lists".to_string(), note: doc.lists.len().to_string(),
                    div { class: "rows",
                        for list in doc.lists.iter() {
                            ListRow {
                                key: "{list.id}",
                                id: list.id.clone(),
                                title: list.title.clone(),
                                detail: format!("{} · {} words", list.source.label(), list.words.len()),
                                armed: arming() == Some(list.id.clone()),
                                arming,
                            }
                        }
                    }
                }
            }

            if !saved().is_empty() {
                Section { title: "Saved Characters".to_string(), note: saved().len().to_string(),
                    CharGrid { entries: saved() }
                }
            }

            if !recent().is_empty() {
                Section { title: "Recently Viewed".to_string(), note: recent().len().to_string(),
                    CharGrid { entries: recent() }
                }
            }

            Section { title: "Coming To This Section".to_string(),
                div { class: "card card-pad",
                    ul { class: "roadmap",
                        RoadmapItem {
                            done: true,
                            title: "Named Lists".to_string(),
                            detail: "Several lists, each with a name, stored in this browser.".to_string(),
                        }
                        RoadmapItem {
                            done: true,
                            title: "Straight From A Lesson".to_string(),
                            detail: "An HSK unit becomes a list in one tap, and adding it twice opens the list you already have.".to_string(),
                        }
                        RoadmapItem {
                            done: true,
                            title: "Saved Characters".to_string(),
                            detail: "Anything you tapped Save on, from a character page.".to_string(),
                        }
                        RoadmapItem {
                            title: "Study A List".to_string(),
                            detail: "Hide the pinyin, the meaning, or both, and drill what is left.".to_string(),
                        }
                        RoadmapItem {
                            title: "Import And Export".to_string(),
                            detail: "Move a list in or out as plain text.".to_string(),
                        }
                        RoadmapItem {
                            title: "Accounts And Sync".to_string(),
                            detail: "Sign in to carry lists and progress between devices.".to_string(),
                        }
                    }
                }
            }
        }
    }
}

/// One row on `/lists`: open it, or arm and confirm a delete.
#[component]
fn ListRow(
    id: String,
    title: String,
    detail: String,
    armed: bool,
    arming: Signal<Option<String>>,
) -> Element {
    let lists = lists::use_lists();
    let mut arming = arming;

    rsx! {
        div { class: "row-split",
            Link { class: "row", to: Route::ListDetail { id: id.clone() },
                div { class: "row-body",
                    div { class: "row-title",
                        span { class: "row-pinyin", "{title}" }
                    }
                    div { class: "row-gloss", "{detail}" }
                }
                span { class: "row-chevron", icons::Chevron {} }
            }
            if armed {
                button {
                    class: "btn btn-accent row-confirm",
                    r#type: "button",
                    aria_label: "Delete {title} for good",
                    onclick: {
                        let id = id.clone();
                        move |_| {
                            lists::delete(lists, &id);
                            arming.set(None);
                        }
                    },
                    "Delete"
                }
            } else {
                button {
                    class: "row-action",
                    r#type: "button",
                    aria_label: "Delete {title}",
                    onclick: {
                        let id = id.clone();
                        move |_| arming.set(Some(id.clone()))
                    },
                    icons::Trash {}
                }
            }
        }
    }
}

/// Pick a unit from the course and import it, without leaving `/lists`.
///
/// The words are the course's own, so an imported list matches what the unit
/// teaches rather than a second, slightly different copy of the vocabulary.
#[component]
fn UnitPicker() -> Element {
    let lists = lists::use_lists();
    let mut level = use_signal(|| 1u8);
    let mut query = use_signal(String::new);

    let course = use_resource(move || async move { hsk::load_level(level()).await });
    let imported = use_memo(move || lists.read().imported_units(level()));

    rsx! {
        div { class: "card card-pad stack",
            div { class: "segmented", role: "group", aria_label: "HSK level",
                for option in hsk::LEVELS {
                    button {
                        key: "{option}",
                        class: "segment",
                        r#type: "button",
                        aria_pressed: (option == level()).to_string(),
                        onclick: move |_| level.set(option),
                        "HSK {option}"
                    }
                }
            }

            div { class: "searchfield",
                span { icons::Search {} }
                input {
                    r#type: "search",
                    value: "{query}",
                    placeholder: "Find a unit — food, 你好, 01…",
                    aria_label: "Find a unit",
                    autocomplete: "off",
                    oninput: move |event| query.set(event.value()),
                }
            }

            match &*course.read_unchecked() {
                Some(Ok(loaded)) => {
                    let needle = query().trim().to_lowercase();
                    let matched: Vec<&hsk::Unit> = loaded
                        .units
                        .iter()
                        .filter(|unit| {
                            needle.is_empty()
                                || unit.title.to_lowercase().contains(&needle)
                                || unit.id.contains(&needle)
                                || unit.words.iter().any(|word| word.contains(&needle))
                        })
                        .collect();
                    let already = imported();

                    if matched.is_empty() {
                        rsx! {
                            p { class: "action-note", "No unit in HSK {level} matches “{query}”." }
                        }
                    } else {
                        rsx! {
                            div { class: "rows rows-scroll",
                                for unit in matched {
                                    PickerRow {
                                        key: "{unit.id}",
                                        level: level(),
                                        unit: unit.clone(),
                                        words: loaded.unit_words(unit),
                                        added: already.iter().any(|id| id == &unit.id),
                                    }
                                }
                            }
                        }
                    }
                },
                Some(Err(message)) => rsx! {
                    p { class: "action-note", "{message}" }
                },
                None => rsx! {
                    p { class: "action-note", "Loading HSK {level}…" }
                },
            }
        }
    }
}

#[component]
fn PickerRow(level: u8, unit: hsk::Unit, words: Vec<hsk::Word>, added: bool) -> Element {
    let lists = lists::use_lists();
    let navigator = use_navigator();

    let count = words.len();
    let number = unit.number().to_string();
    let preview = unit
        .words
        .iter()
        .take(PREVIEW_WORDS)
        .cloned()
        .collect::<Vec<_>>()
        .join("  ");

    let pick = {
        let unit = unit.clone();
        let words = words.clone();
        move |_| {
            let Some(id) = lists::import_unit(lists, level, &unit, &words) else {
                return;
            };
            navigator.push(Route::ListDetail { id });
        }
    };

    rsx! {
        div { class: "row-split",
            div { class: "row",
                span { class: "row-index num", aria_hidden: "true", "{number}" }
                div { class: "row-body",
                    div { class: "row-title",
                        span { class: "row-pinyin", "{unit.title}" }
                        if added {
                            span { class: "badge badge-accent", "Added" }
                        }
                    }
                    div { class: "row-preview han", "{preview}" }
                }
            }
            button {
                class: "btn row-confirm",
                r#type: "button",
                aria_label: if added { format!("Open the list for unit {number}") } else { format!("Add unit {number} to My Lists") },
                onclick: pick,
                if added { "Open" } else { "Add" }
                span { class: "num", "{count}" }
            }
        }
    }
}

/// `/lists/:id` — one list: what is on it, and what to do with it.
#[component]
pub fn ListDetail(id: String) -> Element {
    let state = use_index();
    let lists = lists::use_lists();
    let navigator = use_navigator();

    let mut renaming = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let mut arming = use_signal(|| false);

    let found = use_memo(use_reactive!(|(id,)| lists.read().get(&id).cloned()));
    let entries = use_memo(move || {
        let glyphs = found().map(|list| list.characters()).unwrap_or_default();
        index::ready(&state)
            .map(|idx| idx.lookup_all(&glyphs))
            .unwrap_or_default()
    });

    let Some(list) = found() else {
        return rsx! {
            Empty { glyph: "？".to_string(), title: "No Such List".to_string(),
                p { "That list is not on this device. It may have been deleted." }
                Link { class: "btn", to: Route::Lists {}, "My Lists" }
            }
        };
    };

    let rows: Vec<WordRow> = list.words.iter().map(WordRow::from).collect();
    let count = rows.len();
    let list_id = list.id.clone();
    let list_title = list.title.clone();

    let remove = {
        let list_id = list_id.clone();
        move |word: String| lists::remove_word(lists, &list_id, &word)
    };
    let mut save_title = {
        let list_id = list_id.clone();
        move |_| {
            lists::rename(lists, &list_id, &draft());
            renaming.set(false);
        }
    };
    let delete = move |_| {
        lists::delete(lists, &list_id);
        navigator.push(Route::Lists {});
    };

    // A list taken from a unit can go back to the lesson it came from.
    let lesson = match &list.source {
        ListSource::HskUnit { level, unit_id, .. } => Some(Route::HskUnitPage {
            level: *level,
            unit: unit_id
                .split_once('-')
                .map_or_else(|| unit_id.clone(), |(_, position)| position.to_string()),
        }),
        ListSource::Custom => None,
    };

    rsx! {
        div { class: "stack", style: "padding-top: 1.5rem",
            nav { class: "crumbs", aria_label: "Breadcrumb",
                Link { to: Route::Lists {}, "My Lists" }
            }

            header { class: "section",
                if renaming() {
                    form {
                        class: "field",
                        onsubmit: move |event| {
                            event.prevent_default();
                            save_title(());
                        },
                        input {
                            class: "field-input",
                            r#type: "text",
                            value: "{draft}",
                            aria_label: "List name",
                            autofocus: true,
                            autocomplete: "off",
                            enterkeyhint: "done",
                            oninput: move |event| draft.set(event.value()),
                        }
                        button { class: "btn btn-accent", r#type: "submit", "Save" }
                    }
                } else {
                    h1 { "{list.title}" }
                }

                div { class: "badges",
                    span { class: "badge", "{list.source.label()}" }
                    span { class: "badge num", "{count} words" }
                }

                div { class: "actions",
                    if let Some(lesson) = lesson {
                        Link { class: "btn", to: lesson, "Open The Lesson" }
                    }
                    button {
                        class: "btn",
                        r#type: "button",
                        onclick: move |_| {
                            draft.set(list_title.clone());
                            renaming.set(!renaming());
                        },
                        if renaming() { "Cancel" } else { "Rename" }
                    }
                    if arming() {
                        button { class: "btn btn-accent", r#type: "button", onclick: delete,
                            icons::Trash {}
                            "Delete For Good"
                        }
                    } else {
                        button {
                            class: "btn",
                            r#type: "button",
                            onclick: move |_| arming.set(true),
                            icons::Trash {}
                            "Delete"
                        }
                    }
                }
            }

            if rows.is_empty() {
                Empty { glyph: "空".to_string(), title: "Nothing On It Yet".to_string(),
                    p { "Take a lesson's words from the course, or look a word up and save its characters." }
                    Link { class: "btn btn-accent", to: Route::Hsk {}, "Browse The Course" }
                }
            } else {
                Section { title: "Words".to_string(), note: count.to_string(),
                    WordRows { rows, on_remove: EventHandler::new(remove) }
                }
            }

            if !entries().is_empty() {
                Section { title: "Characters".to_string(), note: entries().len().to_string(),
                    CharGrid { entries: entries() }
                }
            }
        }
    }
}

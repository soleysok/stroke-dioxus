//! Stroke Order — learn Chinese characters stroke by stroke.
//!
//! A Dioxus 0.7 web app. See README.md for how to run it and where it is going.

mod components;
mod data;
mod history;
mod hsk;
mod index;
mod lists;
mod recognize;
mod routes;
mod speech;
mod storage;
mod text;
mod url;

use dioxus::prelude::*;

use components::shell::Shell;
use routes::{
    character::CharacterPage,
    draw::Draw,
    home::Home,
    hsk::{Hsk, HskBand, HskLevel, HskUnitPage},
    lists::{ListDetail, Lists},
    not_found::NotFound,
    search::Search,
};

/// Every route in the app.
///
/// `Shell` wraps all of them, so navigation chrome persists across transitions
/// instead of being torn down and rebuilt.
#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Shell)]
        #[route("/")]
        Home {},

        #[route("/search?:q")]
        Search { q: String },

        #[route("/draw")]
        Draw {},

        // The segment holds the character itself, percent-encoded. See `crate::url`.
        #[route("/character/:glyph")]
        CharacterPage { glyph: String },

        #[route("/hsk")]
        Hsk {},

        // Ahead of the course routes on purpose: "band" is not a number, so a
        // level cannot swallow it, and this keeps the two halves of the section
        // — words and characters — from needing two different top-level paths.
        #[route("/hsk/band/:band")]
        HskBand { band: u8 },

        #[route("/hsk/:level")]
        HskLevel { level: u8 },

        // The unit segment is its position within the level: /hsk/1/07.
        #[route("/hsk/:level/:unit")]
        HskUnitPage { level: u8, unit: String },

        #[route("/lists")]
        Lists {},

        #[route("/lists/:id")]
        ListDetail { id: String },

        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Loads the dictionary index once and publishes it to every route.
    index::provide_index();
    // Noted before anything can navigate, so a page can tell whether it was
    // arrived at from inside the app or opened cold.
    history::provide_entry();
    // Reads the saved lists once, so a list imported on an HSK unit page is on
    // /lists without a reload.
    lists::provide();

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1, viewport-fit=cover",
        }
        document::Meta { name: "theme-color", content: "#f9f9fb" }
        Router::<Route> {}
    }
}

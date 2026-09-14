//! Stroke Order — learn Chinese characters stroke by stroke.
//!
//! A Dioxus 0.7 web app. See README.md for how to run it and where it is going.

mod components;
mod data;
mod index;
mod recognize;
mod routes;
mod speech;
mod storage;
mod url;

use dioxus::prelude::*;

use components::shell::Shell;
use routes::{
    character::CharacterPage, draw::Draw, home::Home, hsk::Hsk, hsk::HskBand, lists::Lists,
    not_found::NotFound, search::Search,
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

        #[route("/hsk/:band")]
        HskBand { band: u8 },

        #[route("/lists")]
        Lists {},

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

//! Percent-encoding for route segments carrying Han characters.
//!
//! `/character/好` is the URL we want people to share, but a browser normalises
//! it to `/character/%E5%A5%BD` in the address bar and on reload. Links are built
//! encoded so both spellings round-trip, and [`decode_char`] accepts either.

use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};

/// Everything a path segment must not contain verbatim. Han characters are
/// non-ASCII and so are escaped by `utf8_percent_encode` regardless.
const SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'/')
    .add(b'{')
    .add(b'}');

pub fn encode(text: &str) -> String {
    utf8_percent_encode(text, SEGMENT).to_string()
}

pub fn decode(text: &str) -> String {
    percent_decode_str(text)
        .decode_utf8()
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| text.to_string())
}

/// Read a character out of a route segment, whether or not it arrived encoded.
pub fn decode_char(segment: &str) -> Option<char> {
    decode(segment).chars().next()
}

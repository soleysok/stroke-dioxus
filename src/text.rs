//! Two small text helpers, shared by the pages that count things.
//!
//! Both exist because the alternative reads badly: "5369 words" is a number to
//! decode rather than a figure to take in, and "1 lists" is the tell that nobody
//! looked at the page with one list on it.

/// `5369` -> `"5,369"`.
pub fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// `(1, "word")` -> `"1 word"`, `(11, "word")` -> `"11 words"`. Only the plurals
/// that take an -s, which is all this app needs.
pub fn counted(n: usize, singular: &str) -> String {
    if n == 1 {
        format!("1 {singular}")
    } else {
        format!("{} {singular}s", grouped(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_are_grouped() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(512), "512");
        assert_eq!(grouped(5369), "5,369");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }

    #[test]
    fn one_of_something_is_singular() {
        assert_eq!(counted(0, "list"), "0 lists");
        assert_eq!(counted(1, "list"), "1 list");
        assert_eq!(counted(2, "list"), "2 lists");
        assert_eq!(counted(5369, "word"), "5,369 words");
    }
}

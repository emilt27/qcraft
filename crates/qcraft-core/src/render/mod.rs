pub mod ctx;
pub mod policy;
pub mod renderer;

/// Escape LIKE special characters (`%`, `_`, `\`) using backslash as escape char.
///
/// Use this to build safe LIKE patterns from user input.
pub fn escape_like_value(val: &str) -> String {
    let mut out = String::with_capacity(val.len());
    for ch in val.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '%' => out.push_str("\\%"),
            '_' => out.push_str("\\_"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_plain_text() {
        assert_eq!(escape_like_value("hello"), "hello");
    }

    #[test]
    fn escape_percent() {
        assert_eq!(escape_like_value("50%"), "50\\%");
    }

    #[test]
    fn escape_underscore() {
        assert_eq!(escape_like_value("user_name"), "user\\_name");
    }

    #[test]
    fn escape_backslash() {
        assert_eq!(escape_like_value("C:\\path"), "C:\\\\path");
    }

    #[test]
    fn escape_all_special() {
        assert_eq!(escape_like_value("50%_\\"), "50\\%\\_\\\\");
    }
}

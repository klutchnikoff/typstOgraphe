//! Encoding literal strings and content for generated Typst.
pub fn typst_string(text: &str) -> String {
    // Les chaînes JSON partagent les échappements usuels avec les chaînes Typst.
    // Les contrôles moins courants sont écrits avec la syntaxe Unicode de Typst.
    let mut result = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => result.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

pub fn escape_markup(text: &str) -> String {
    let mut result = String::new();
    for c in text.chars() {
        if "\\#*_[]$`<>@=+-/~{}\"':".contains(c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

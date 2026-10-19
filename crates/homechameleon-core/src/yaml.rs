//! Hand-written theme YAML. Home Assistant wants quoted color strings.
//! `serde_yaml` is deprecated and `serde_yml` is not used.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeValue {
    Scalar(String),
    /// Literal block (`|`). Used for optional card-mod CSS.
    Block(String),
}

pub fn emit(
    theme_name: &str,
    style_id: &str,
    background_on: bool,
    modes: &[(&str, BTreeMap<String, ThemeValue>)],
) -> String {
    let background = if background_on { "on" } else { "off" };
    let mut out = String::new();
    out.push_str("# HomeChameleon theme\n");
    out.push_str(&format!("# style: {style_id}\n"));
    out.push_str(&format!("# background: {background}\n"));
    // `!include_dir_merge_named` uses this top-level key as the theme name.
    write_key(&mut out, theme_name);
    out.push_str(":\n");
    out.push_str("  modes:\n");
    for (name, vars) in modes {
        out.push_str(&format!("    {name}:\n"));
        emit_map(&mut out, vars, 6);
    }
    out
}

fn write_key(out: &mut String, name: &str) {
    if is_plain_key(name) {
        out.push_str(name);
    } else {
        write_quoted(out, name);
    }
}

fn is_plain_key(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-')
        && !name.ends_with(' ')
}

fn emit_map(out: &mut String, vars: &BTreeMap<String, ThemeValue>, indent: usize) {
    for (key, value) in vars {
        pad(out, indent);
        out.push_str(key);
        out.push_str(": ");
        match value {
            ThemeValue::Scalar(text) => {
                write_quoted(out, text);
                out.push('\n');
            }
            ThemeValue::Block(text) => {
                out.push_str("|\n");
                for line in text.split('\n') {
                    if line.is_empty() {
                        out.push('\n');
                        continue;
                    }
                    pad(out, indent + 2);
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
    }
}

fn write_quoted(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn pad(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push(' ');
    }
}

pub fn emit_css(modes: &[(&str, BTreeMap<String, ThemeValue>)]) -> String {
    let mut out = String::new();
    for (name, vars) in modes {
        out.push_str(&format!("/* {name} */\n"));
        for (key, value) in vars {
            if let ThemeValue::Scalar(text) = value {
                out.push_str(&format!("--{key}: {text};\n"));
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_hex_and_keeps_block() {
        let mut dark = BTreeMap::new();
        dark.insert("primary-color".into(), ThemeValue::Scalar("#7aa2f7".into()));
        dark.insert(
            "card-mod-card".into(),
            ThemeValue::Block("ha-card {\n  opacity: 1;\n}".into()),
        );
        let yaml = emit("Desk", "sheer", false, &[("dark", dark)]);
        assert!(yaml.contains("Desk:\n  modes:\n"));
        assert!(yaml.contains("primary-color: \"#7aa2f7\""));
        assert!(yaml.contains("card-mod-card: |"));
        assert!(yaml.contains("        ha-card {"));
        assert!(!yaml.contains("lovelace-background"));
    }
}

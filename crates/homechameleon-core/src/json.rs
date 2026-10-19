//! A small JSON reader and writer for the request, the result, and palette files.
//! It replaces `serde_json` in the library so the wasm build stays small.

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn object(pairs: impl IntoIterator<Item = (&'static str, Json)>) -> Self {
        Self::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Self::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Self::Object(pairs) => Some(pairs),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn stringify(&self) -> String {
        let mut out = String::new();
        write_compact(&mut out, self);
        out
    }
}

impl From<&str> for Json {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for Json {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<bool> for Json {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

pub fn parse(input: &str) -> Result<Json, String> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        index: 0,
    };
    parser.skip();
    let value = parser.value()?;
    parser.skip();
    if parser.index != parser.bytes.len() {
        return Err("unexpected trailing JSON".into());
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while matches!(
            self.bytes.get(self.index),
            Some(b' ' | b'\n' | b'\r' | b'\t')
        ) {
            self.index += 1;
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip();
        match self.bytes.get(self.index).copied() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b't') => self.literal(b"true", Json::Bool(true)),
            Some(b'f') => self.literal(b"false", Json::Bool(false)),
            Some(b'n') => self.literal(b"null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err("expected a JSON value".into()),
        }
    }

    fn literal(&mut self, text: &[u8], value: Json) -> Result<Json, String> {
        if self.bytes.get(self.index..self.index + text.len()) == Some(text) {
            self.index += text.len();
            Ok(value)
        } else {
            Err("invalid JSON literal".into())
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.index += 1;
        let mut pairs = Vec::new();
        self.skip();
        if self.bytes.get(self.index) == Some(&b'}') {
            self.index += 1;
            return Ok(Json::Object(pairs));
        }
        loop {
            self.skip();
            if self.bytes.get(self.index) != Some(&b'"') {
                return Err("JSON object key must be a string".into());
            }
            let key = self.string()?;
            self.skip();
            if self.bytes.get(self.index) != Some(&b':') {
                return Err("expected ':' after a JSON key".into());
            }
            self.index += 1;
            pairs.push((key, self.value()?));
            self.skip();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b'}') => {
                    self.index += 1;
                    break;
                }
                _ => return Err("expected ',' or '}'".into()),
            }
        }
        Ok(Json::Object(pairs))
    }

    fn array(&mut self) -> Result<Json, String> {
        self.index += 1;
        let mut items = Vec::new();
        self.skip();
        if self.bytes.get(self.index) == Some(&b']') {
            self.index += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b']') => {
                    self.index += 1;
                    break;
                }
                _ => return Err("expected ',' or ']'".into()),
            }
        }
        Ok(Json::Array(items))
    }

    fn string(&mut self) -> Result<String, String> {
        self.index += 1;
        let mut out = Vec::new();
        while let Some(byte) = self.bytes.get(self.index).copied() {
            self.index += 1;
            match byte {
                b'"' => {
                    return String::from_utf8(out).map_err(|_| "JSON string is not utf-8".into());
                }
                b'\\' => {
                    let esc = self
                        .bytes
                        .get(self.index)
                        .copied()
                        .ok_or("bad JSON escape")?;
                    self.index += 1;
                    let ch = match esc {
                        b'"' | b'\\' | b'/' => esc as char,
                        b'b' => '\u{0008}',
                        b'f' => '\u{000c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hex = self
                                .bytes
                                .get(self.index..self.index + 4)
                                .ok_or("short \\u escape")?;
                            self.index += 4;
                            let text = std::str::from_utf8(hex).map_err(|_| "bad \\u escape")?;
                            let code =
                                u32::from_str_radix(text, 16).map_err(|_| "bad \\u escape")?;
                            char::from_u32(code).ok_or("bad \\u escape")?
                        }
                        _ => return Err("bad JSON escape".into()),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                0x00..=0x1f => return Err("raw control character in a JSON string".into()),
                _ => out.push(byte),
            }
        }
        Err("unterminated JSON string".into())
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.index;
        if self.bytes.get(self.index) == Some(&b'-') {
            self.index += 1;
        }
        let digits = self.index;
        while matches!(self.bytes.get(self.index), Some(b'0'..=b'9')) {
            self.index += 1;
        }
        if self.index == digits {
            return Err("invalid JSON number".into());
        }
        if self.bytes.get(self.index) == Some(&b'.') {
            self.index += 1;
            let frac = self.index;
            while matches!(self.bytes.get(self.index), Some(b'0'..=b'9')) {
                self.index += 1;
            }
            if self.index == frac {
                return Err("invalid JSON number".into());
            }
        }
        if matches!(self.bytes.get(self.index), Some(b'e' | b'E')) {
            self.index += 1;
            if matches!(self.bytes.get(self.index), Some(b'+' | b'-')) {
                self.index += 1;
            }
            let exp = self.index;
            while matches!(self.bytes.get(self.index), Some(b'0'..=b'9')) {
                self.index += 1;
            }
            if self.index == exp {
                return Err("invalid JSON number".into());
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.index])
            .map_err(|_| "invalid JSON number")?;
        let value: f64 = text.parse().map_err(|_| "invalid JSON number")?;
        if !value.is_finite() {
            return Err("JSON number is out of range".into());
        }
        Ok(Json::Number(value))
    }
}

fn write_compact(out: &mut String, value: &Json) {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Number(number) => push_number(out, *number),
        Json::String(text) => push_string(out, text),
        Json::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_compact(out, item);
            }
            out.push(']');
        }
        Json::Object(pairs) => {
            out.push('{');
            for (i, (key, item)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                push_string(out, key);
                out.push(':');
                write_compact(out, item);
            }
            out.push('}');
        }
    }
}

pub fn push_string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn push_number(out: &mut String, number: f64) {
    if number.fract() == 0.0 && (-1.0e15..1.0e15).contains(&number) {
        out.push_str(&format!("{}", number as i64));
        return;
    }
    let text = format!("{number:.6}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    out.push_str(text);
}

pub fn expect_u32(value: &Json, name: &str) -> Result<u32, String> {
    let number = value
        .as_f64()
        .ok_or_else(|| format!("{name} must be a number"))?;
    if number < 0.0 || number > f64::from(u32::MAX) || number.fract() != 0.0 {
        return Err(format!("{name} must be a whole number"));
    }
    Ok(number as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_object() {
        let raw = r#"{"a":1,"b":"hi","c":[true,null],"d":{"e":-2.5}}"#;
        let value = parse(raw).unwrap();
        assert_eq!(value.stringify(), raw);
        assert_eq!(value.get("b").and_then(Json::as_str), Some("hi"));
    }
}

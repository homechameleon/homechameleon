//! JS entry point. The same `generate` function runs underneath.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn generate_json(request: &str) -> String {
    match crate::generate_json(request) {
        Ok(body) => body,
        Err(err) => {
            let mut out = String::from("{\"error\":");
            crate::json::push_string(&mut out, &err.to_string());
            out.push('}');
            out
        }
    }
}

#[wasm_bindgen]
pub fn styles_json() -> String {
    let items: Vec<crate::json::Json> = crate::styles()
        .into_iter()
        .map(|style| {
            crate::json::Json::object([
                ("id", crate::json::Json::from(style.id)),
                ("name", crate::json::Json::from(style.name)),
                ("summary", crate::json::Json::from(style.summary)),
            ])
        })
        .collect();
    crate::json::Json::Array(items).stringify()
}

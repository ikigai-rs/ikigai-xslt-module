//! `ikigai-xslt-module` — the ikigai-xslt transform as a standalone, dynamically-loadable
//! WASM artifact. A wasm-bindgen wrapper over [`ikigai_xslt::transform_xml`], built to its
//! own `.wasm` and lazy-loaded by a host — so xrust (~2.2 MB) lives in *this* artifact,
//! not in the host's binary, and is fetched only when an `urn:xslt:*` resource is first
//! resolved.
//!
//! Two entry points:
//! - [`transform`] — **by value**: the host resolves the `src`/`stylesheet` references and
//!   passes the bytes in.
//! - [`transform_refs`] — **by reference**: the module is handed the *IRIs* and resolves
//!   them itself by calling back to the host ([`host_resolve`]), the bidirectional
//!   callback the dynamic-module format is really about.

use wasm_bindgen::prelude::*;

/// Install a panic hook so a Rust panic surfaces in the browser console (once).
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Apply `stylesheet` (XSLT) to `src` (XML). `text` selects string-value serialization
/// (a `method="text"` stylesheet) vs XML/markup. Returns the transformed string, or a
/// JS error with the failure detail. (By value: the host already resolved the refs.)
#[wasm_bindgen]
pub fn transform(src: &str, stylesheet: &str, text: bool) -> Result<String, JsValue> {
    ikigai_xslt::transform_xml(src, stylesheet, text).map_err(|e| JsValue::from_str(&e))
}

// The host's resolver, imported as a global: resolve a resource IRI to its text against
// the *host* kernel. This is the callback channel — the module reaches back across the
// wasm boundary for resources the host owns (the catalog, the stylesheet).
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_name = "hostResolve")]
    async fn host_resolve(uri: &str) -> Result<JsValue, JsValue>;
}

/// **By reference.** The module is handed the `src`/`stylesheet` *IRIs* and resolves them
/// itself by calling back to the host (`hostResolve`), then transforms — the module
/// pulling the resources it needs from the host mid-invocation, across the wasm boundary.
#[wasm_bindgen]
pub async fn transform_refs(src_uri: &str, style_uri: &str, text: bool) -> Result<String, JsValue> {
    let src = host_resolve(src_uri)
        .await?
        .as_string()
        .ok_or_else(|| JsValue::from_str("host returned a non-string for src"))?;
    let stylesheet = host_resolve(style_uri)
        .await?
        .as_string()
        .ok_or_else(|| JsValue::from_str("host returned a non-string for stylesheet"))?;
    ikigai_xslt::transform_xml(&src, &stylesheet, text).map_err(|e| JsValue::from_str(&e))
}
